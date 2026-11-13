// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: LicenseRef-Duguid-Public-Attribution-1.0

//! The output-contract gate: contract fields, abbreviated records,
//! arithmetic reperformance, the conclusion screen and the exit codes.

mod common;

use common::*;
use drdebits::gate::{
    self, EXIT_BLOCK, EXIT_COULD_NOT_RUN, EXIT_PASS, EXIT_REVIEW, calculations, check,
    check_authorities, check_facts, check_quotes, load_authorities, load_facts,
};
use drdebits::model::Conclusion;
use drdebits::pyjson;
use std::io::Write;
use std::process::{Command, Stdio};

const DRAFT: &str = "DRAFT — PROFESSIONAL REVIEW REQUIRED

Primary decision status: PROCEED_DRAFT_ONLY
- Scope: GST on one fictional invoice; next human step: confirm the supply is taxable.
- action_authority: NONE.
- state_changing_tool_action: NONE

Scope and period
- One invoice dated 1 August 2026.

Source-version status
- Historical authority: CONFIRMED
- Current duties: SOURCE CURRENCY NOT CONFIRMED

Facts and evidence
- Invoice total $1,100 including GST.

Assumptions
- The supply is wholly taxable.

Applicable authority
- GST Act s 9-70.

Analysis
- $1,100 / 11 = $100.00

Ethical and regulatory flags
- None identified.

Conclusion and uncertainty
- GST of $100 on supplied facts.

Human actions before use
- Reviewer confirms the tax invoice.

External-action record
- No lodgement, payment or other external action occurred.
";

const STOP: &str = "DRAFT - PROFESSIONAL REVIEW REQUIRED
Primary decision status: HARD_STOP
- Concluding on the safe harbour is prohibited; the registered practitioner assesses it.
- action_authority: NONE
- state_changing_tool_action: NONE
External-action record
- No external action occurred.
";

fn conclusions() -> Vec<Conclusion> {
    vec![Conclusion {
        label: "safe-harbour application".into(),
        definition: "d".into(),
        markers: vec!["safe harbour applies to your".into()],
    }]
}

fn verdict(text: &str) -> (&'static str, u8) {
    check(text, &conclusions()).verdict()
}

#[test]
fn a_complete_draft_passes_and_reperforms_its_arithmetic() {
    let report = check(DRAFT, &conclusions());
    assert_eq!(report.verdict(), ("PASS", EXIT_PASS), "{:?}", report.blocks);
    assert_eq!(report.status.as_deref(), Some("PROCEED_DRAFT_ONLY"));
    assert!(report.notes.is_empty(), "{:?}", report.notes);
    assert_eq!(report.calculations.len(), 1);
    assert!(report.calculations[0].matches);
}

#[test]
fn a_wrong_figure_blocks_with_the_computed_value() {
    let report = check(&DRAFT.replace("= $100.00", "= $110.00"), &conclusions());
    assert_eq!(report.verdict(), ("BLOCK", EXIT_BLOCK));
    assert_eq!(
        report.blocks,
        ["calculation does not reperform: $1,100 / 11 = $110.00 (computes to 100.00)"]
    );
}

#[test]
fn each_missing_contract_element_blocks() {
    for (from, to, finding) in [
        (
            "DRAFT — PROFESSIONAL REVIEW REQUIRED",
            "Draft for review",
            "banner",
        ),
        (
            "action_authority: NONE.",
            "action_authority: SEND_EMAIL",
            "action_authority must be NONE",
        ),
        (
            "- state_changing_tool_action: NONE\n",
            "",
            "missing field: state_changing_tool_action",
        ),
        (
            "- state_changing_tool_action: NONE\n",
            "- state_changing_tool_action:\n",
            "state_changing_tool_action is empty",
        ),
        (
            "Current duties: SOURCE CURRENCY NOT CONFIRMED",
            "Current duties: probably fine",
            "Current duties must be",
        ),
        (
            "Assumptions\n",
            "Premises\n",
            "missing mandatory section: Assumptions",
        ),
        (
            "Primary decision status: PROCEED_DRAFT_ONLY",
            "Primary decision status: PROCEED",
            "no primary decision status",
        ),
    ] {
        let report = check(&DRAFT.replace(from, to), &conclusions());
        assert_eq!(report.verdict().1, EXIT_BLOCK, "{from}");
        assert!(
            report.blocks.iter().any(|b| b.contains(finding)),
            "{from}: {:?}",
            report.blocks
        );
    }
}

#[test]
fn two_decision_fields_block() {
    let text = DRAFT.replace(
        "Primary decision status: PROCEED_DRAFT_ONLY",
        "Primary decision status: PROCEED_DRAFT_ONLY\nPrimary decision status: NEEDS_FACTS",
    );
    let report = check(&text, &conclusions());
    assert!(
        report
            .blocks
            .iter()
            .any(|b| b.contains("2 decision fields")),
        "{:?}",
        report.blocks
    );
}

#[test]
fn the_low_impact_form_is_outside_this_gate() {
    let text = DRAFT.replace("PROCEED_DRAFT_ONLY", "Low impact — proportionate answer");
    let report = check(&text, &conclusions());
    assert!(
        report
            .blocks
            .iter()
            .any(|b| b.contains("not a high-impact decision status")),
        "{:?}",
        report.blocks
    );
}

#[test]
fn an_abbreviated_stop_record_passes_with_notes() {
    let report = check(STOP, &conclusions());
    assert_eq!(report.verdict(), ("PASS", EXIT_PASS), "{:?}", report.blocks);
    assert_eq!(report.notes.len(), 6, "{:?}", report.notes);
    let without_record = STOP.replace("External-action record\n", "");
    assert_eq!(verdict(&without_record).1, EXIT_BLOCK);
}

#[test]
fn a_full_status_cannot_use_the_abbreviated_record() {
    let text = STOP.replace("HARD_STOP", "NEEDS_FACTS");
    let report = check(&text, &conclusions());
    assert!(
        report
            .blocks
            .iter()
            .any(|b| b.contains("missing mandatory section: Assumptions"))
    );
    assert!(
        report
            .blocks
            .iter()
            .any(|b| b.contains("missing source-version line"))
    );
}

#[test]
fn a_screen_hit_alone_asks_for_review() {
    let text = STOP.replace(
        "Concluding on the safe harbour is prohibited",
        "I will not say the safe harbour applies to your position",
    );
    let report = check(&text, &conclusions());
    assert_eq!(report.verdict(), ("REVIEW", EXIT_REVIEW));
    assert_eq!(report.reviews.len(), 1);
}

#[test]
fn reperformance_follows_precedence_percentages_and_rounding() {
    let cases = [
        ("2 + 3 × 4 = 14", true),
        ("(2 + 3) × 4 = 20", true),
        ("$120,000 x 12% = $14,400", true),
        ("100 / 3 = 33.33", true),
        ("100 / 3 = 33.3", true),
        ("100 / 3 = 33.35", false),
        ("100 / 1,000 = 10%", true),
        ("100 / 1,000 = 12%", false),
        ("1,000 − 250 = 750", true),
        ("-5 + 2 = -3", true),
        ("8 / -2 = -4", true),
        ("10 ÷ 4 = 2.5", true),
        ("1,100 / 11 = 101", false),
    ];
    for (text, ok) in cases {
        let found = calculations(text);
        assert_eq!(found.len(), 1, "{text}");
        assert_eq!(
            found[0].matches, ok,
            "{text}: computed {}",
            found[0].computed
        );
    }
}

#[test]
fn dates_years_bullets_and_prose_are_not_calculations() {
    for text in [
        "Lodged 1/7/2026 = on time",
        "Lodged 1/7/2026 = 2026",
        "The 2026-27 year = the current year",
        "- 5 items were reviewed",
        "s 284-75(6) applies",
        "Total = 100",
        "x = 5",
        "Rate 10% applies; 5 = five",
    ] {
        assert!(calculations(text).is_empty(), "{text}");
    }
}

#[test]
fn unparseable_operands_or_results_are_never_marked_correct() {
    for text in ["2 + 3 = ٥", "2 + (3 = 5", "2 + (3) ) = 5"] {
        assert!(
            calculations(text).iter().all(|claim| !claim.matches),
            "{text}"
        );
    }
}

#[test]
fn the_command_checks_a_draft_piped_to_stdin() {
    let tree = common::Tree::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_drdebits_build"))
        .current_dir(tree.path())
        .args(["gate", "--json", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(DRAFT.as_bytes())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(result.status.success(), "{result:?}");
    let stdout = String::from_utf8(result.stdout).unwrap();
    let report = pyjson::loads(&stdout).unwrap();
    assert_eq!(
        report.get("verdict").and_then(pyjson::Json::as_str),
        Some("PASS")
    );
}

#[test]
fn division_by_zero_does_not_reperform() {
    let found = calculations("100 / 0 = 0");
    assert_eq!(found.len(), 1);
    assert!(!found[0].matches);
    assert_eq!(found[0].computed, "undefined (division by zero)");
    let report = check(
        &DRAFT.replace("$1,100 / 11 = $100.00", "$1,100 / 0 = $0"),
        &conclusions(),
    );
    assert_eq!(report.verdict().1, EXIT_BLOCK);
}

#[test]
fn every_link_of_a_chain_is_checked() {
    for (text, ok) in [
        // A running total: each link equals the next link's leading number.
        ("1 + 1 = 2 + 1 = 3", true),
        // A true chain: each link equals the whole next expression.
        ("$1,000 × 10% = $50 × 2 = $100", true),
        ("1 + 1 = 3 + 2 = 5", false),
        ("2 × 3 = 6 + 1 = 8", false),
        ("Item 2 = $500 + $300 = $800", true),
        // An intermediate expression keeps its own written precision.
        ("100 / 3 = 33.35 + 0 = 33", false),
        ("100 / 3 = 33.33 + 0 = 33", true),
        // A bracket opened by the surrounding prose.
        ("(GST: 1,100 / 11 = 100)", true),
        ("(2 + 3 = 6)", false),
    ] {
        let found = calculations(text);
        assert_eq!(found.len(), 1, "{text}");
        assert_eq!(found[0].matches, ok, "{text}: {}", found[0].computed);
    }
    assert_eq!(calculations("1 + 1 = 3 + 2 = 5")[0].computed, "2.00");
    assert_eq!(
        calculations("Item 2 = $500 + $300 = $800")[0].text,
        "$500 + $300 = $800"
    );
}

#[test]
fn a_bracket_the_gate_cannot_pair_blocks() {
    // The gate sees only part of a sentence and cannot tell where a bracket
    // closed inside a calculation opened, so it blocks, even a correct sum.
    for text in [
        // The line's total is $350; the gate sees only `$200 + $50) = $250`.
        "$100 + (wages $200 + $50) = $250",
        "$100 + (wages $200 + $50) = $350",
        "Net income (salary $1,000 - $200) = $900",
        "Net income (salary $1,000 - $200) = $800",
        "Net (salary $1,000 - $200) = $800 - $100 = $700",
        "Net (salary $1,000 - $200) × 2 = $1,600",
        // A label inside the bracket.
        "Rent (weekly $500 × 52) = $26,000",
        // An aside ending in a digit: the gate sees `1) + $50 = $150`.
        "$100 (note 1) + $50 = $150",
        "$100 (s 8-1) + $50 = $150",
    ] {
        let found = calculations(text);
        assert_eq!(found.len(), 1, "{text}");
        assert!(!found[0].matches, "{text}: {}", found[0].computed);
        assert!(
            found[0].computed.starts_with("could not read"),
            "{text}: {}",
            found[0].computed
        );
    }
    // A bracket opened before the calculation, a paired one and a bracket
    // outside the calculation are read.
    for text in [
        "(2 + 3 = 5)",
        "(1,000 - 200) × 2 = 1,600",
        "a) $100 + $50 = $150",
        "(see s 8-1) $100 + $50 = $150",
        "$100 + $50 = $150 (1)",
    ] {
        let found = calculations(text);
        assert_eq!(found.len(), 1, "{text}");
        assert!(found[0].matches, "{text}: {}", found[0].computed);
    }
}

/// The fixture draft with one more line under its flags.
fn with_line(line: &str) -> gate::Report {
    check(
        &DRAFT.replace("- None identified.", &format!("- {line}")),
        &conclusions(),
    )
}

#[test]
fn a_sum_a_word_interrupts_blocks() {
    // An amount and, after it, an operator before a calculation, with no
    // `=` or `$` between, mean the gate sees only the sum's tail, so the
    // line's real total is unknown to it: neither the tail's sum nor the
    // whole line's passes. Each row is (text, blocks).
    let rows = [
        // Words.
        ("$100 + wages $200 + $50 = $250", true),
        ("$100 + wages $200 + $50 = $350", true),
        ("$100 + wages + $200 + $50 = $250", true),
        ("$100 + (wages) $200 + $50 = $250", true),
        ("$1,000 - rent $200 - $50 = $750", true),
        ("5 x hours at 3 + 4 = 7", true),
        // A word holding or following digits, and digits alone.
        ("$100 + wages2 $200 + $50 = $250", true),
        ("$100 + FY2025 wages $200 + $50 = $250", true),
        ("$100 + 2025 wages $200 + $50 = $250", true),
        ("$100 + 2 $200 + $50 = $250", true),
        // Punctuation alone.
        ("$100 + ... + $200 + $50 = $250", true),
        ("$100 + … $200 + $50 = $250", true),
        ("$100 + — $200 + $50 = $250", true),
        ("$100 + ??? $200 + $50 = $250", true),
        // Text between the amount and the operator: a word, a number in a
        // word, punctuation.
        ("$500 wages + $200 + $50 = $250", true),
        ("$100 + $20 wages + $200 + $50 = $250", true),
        ("$500 wages - $200 + $50 = $250", true),
        ("$500 FY2025 + $200 + $50 = $250", true),
        ("$500 ... + $200 + $50 = $250", true),
        // `*` is an operator with or without spaces.
        ("$100*wages $200 + $50 = $250", true),
        ("$100 *wages $200 + $50 = $250", true),
        ("$100* wages $200 + $50 = $250", true),
        // A label after an amount and an operator reads the same way, so
        // these correct sums block too: each needs a line of its own. So does
        // italic text after a number.
        ("Item 3 - Deductions $100 + $50 = $150", true),
        ("FY 2025 - Australia: $100 + $50 = $150", true),
        ("1 x laptop $1,000 + $200 = $1,200", true),
        ("Interest at 5% + fees apply. Total $100 + $50 = $150", true),
        ("Note 2 *estimate* $100 + $50 = $150", true),
        // A tail with no operator left in it, `amount = amount`, after a word
        // between an operator and an amount, right or wrong, and after a rate
        // and an operator.
        ("Wages $500 + super $50 = $600", true),
        ("Wages $500 + super $50 = $550", true),
        ("Wages $500 + super $50 = $600 + $10 = $610", true),
        ("Rent $2,000/month × 12 = $24,000", true),
        // Whitespace alone, a label with no amount and operator before it, a
        // word before an operator that starts the line, and an earlier
        // calculation's result are not an interrupted sum.
        ("$100 +   $200 + $50 = $350", false),
        // An earlier amount with no operator after it, and bold marks, which
        // the gate strips before it reads the line.
        ("You paid $500 in wages, so $200 + $50 = $250", false),
        ("1. **Total:** $100 + $50 = $150", false),
        ("3. **GST**: $1,100 / 11 = $100", false),
        ("Total: $100 + $200 = $300", false),
        ("Fees - $100 + $50 = $150", false),
        ("Step 2: $100 + $50 = $150", false),
        ("1 + 1 = 2 and 3 + 4 = 7", false),
        // An unspaced `/` in a date or fraction is not an operator.
        ("On 1/7/2025 the client paid $100 + $50 = $150", false),
        (
            "For the period 1/7/2025 to 30/6/2026, $100 + $50 = $150",
            false,
        ),
        ("GST is 1/11 of the price: $1,100 / 11 = $100", false),
        (
            "which is 10/11 of the GST-inclusive price, so 11000.00 / 11 = 1000.00",
            false,
        ),
        // Nor is one before a word: it reads as a rate, so a rate before a
        // calculation leaves it to be reperformed, and so does `$100/wages`.
        ("Rent is $2,000/month, so 12 × $2,000 = $24,000", false),
        ("$100/wages $200 + $50 = $250", false),
    ];
    let wrong: Vec<String> = rows
        .iter()
        .filter_map(|&(text, blocks)| {
            let report = with_line(text);
            let interrupted = report
                .blocks
                .iter()
                .any(|b| b.contains("could not read: a word interrupts the sum"));
            let exit = report.verdict().1;
            let right = if blocks {
                exit == EXIT_BLOCK && interrupted
            } else {
                exit == EXIT_PASS
            };
            (!right).then(|| format!("{text} (blocks: {blocks}): {:?}", report.blocks))
        })
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_long_line_is_scanned_whole() {
    // A sum with no number after `=`, of 9 terms or of 20,000, took a
    // backtracking scan past its limit, so it gave up on the line. The scan
    // now runs in linear time and reads the wrong sum after it.
    let report = with_line("1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 = x, and 2 + 2 = 5");
    assert_eq!(
        report.blocks,
        ["calculation does not reperform: 2 + 2 = 5 (computes to 4.00)"]
    );
    assert!(report.reviews.is_empty(), "{:?}", report.reviews);
    let long = format!("{}1 = x, and 2 + 2 = 5", "1 + ".repeat(20_000));
    let started = std::time::Instant::now();
    let found = calculations(&long);
    // Linear time is the `regex` crate's guarantee; the clock only catches
    // a return to backtracking, so its bound is loose.
    let elapsed = started.elapsed();
    assert_eq!(found.len(), 1);
    assert_eq!(
        (found[0].text.as_str(), found[0].matches),
        ("2 + 2 = 5", false)
    );
    assert!(elapsed < std::time::Duration::from_secs(30), "{elapsed:?}");
}

#[test]
fn a_date_among_a_sums_operands_blocks() {
    // A date alone before `=` is prose, as in `Lodged 1/7/2026 = 2026`; with
    // other operands the gate cannot tell what the sum adds, right or wrong.
    for line in [
        "Rent 1/7/2025 + 2 + 2 = 5",
        "Rent 1/7/2025 + 2 + 2 = 4",
        "Lodged 1/7/2026 = 2026 + 1 = 2028",
    ] {
        let report = with_line(line);
        assert_eq!(
            report.verdict().1,
            EXIT_BLOCK,
            "{line}: {:?}",
            report.blocks
        );
        assert!(
            report
                .blocks
                .iter()
                .any(|b| b.contains("a date stands in the sum")),
            "{line}: {:?}",
            report.blocks
        );
    }
}

#[test]
fn a_digit_other_than_0_to_9_blocks() {
    // The scan reads any decimal digit, such as Arabic-Indic `٥`, and the
    // gate reads only 0 to 9, so it blocks such a calculation, right or wrong.
    for line in ["2 + 3 = ٥", "2 + 3 = ٤", "٢ + 3 = 5"] {
        let report = with_line(line);
        assert_eq!(
            report.verdict().1,
            EXIT_BLOCK,
            "{line}: {:?}",
            report.blocks
        );
        assert!(
            report
                .blocks
                .iter()
                .any(|b| b.contains("digits other than 0 to 9")),
            "{line}: {:?}",
            report.blocks
        );
    }
}

#[test]
fn a_bracketed_amount_is_ambiguous_and_blocks() {
    for text in [
        "($100) + $50 = $150",
        "($100) + $50 = ($50)",
        "100 + 20 = (120) + 5 = 125",
    ] {
        let found = calculations(text);
        assert_eq!(found.len(), 1, "{text}");
        assert!(!found[0].matches, "{text}");
        assert!(found[0].computed.starts_with("ambiguous"), "{text}");
    }
    assert!(calculations("(2 + 3) × 4 = 20")[0].matches);
    // A signed bracketed amount is explicit and reperforms.
    for (text, ok) in [
        ("100 + (-20) = 80", true),
        ("100 + (-20) = 90", false),
        ("100 - 150 = (-50)", true),
        ("100 + (20) = 120", false),
    ] {
        let found = calculations(text);
        assert_eq!(found.len(), 1, "{text}");
        assert_eq!(found[0].matches, ok, "{text}: {}", found[0].computed);
    }
}

#[test]
fn a_state_changing_action_asks_for_review() {
    let text = DRAFT.replace(
        "state_changing_tool_action: NONE",
        "state_changing_tool_action: PAY_NOW",
    );
    let report = check(&text, &conclusions());
    assert_eq!(
        report.verdict(),
        ("REVIEW", EXIT_REVIEW),
        "{:?}",
        report.blocks
    );
    assert!(
        report.reviews[0].contains("'PAY_NOW'"),
        "{:?}",
        report.reviews
    );
}

#[test]
fn an_empty_section_counts_as_missing() {
    for (from, to, finding) in [
        (
            "- The supply is wholly taxable.\n",
            "",
            "missing mandatory section: Assumptions (empty",
        ),
        (
            "- The supply is wholly taxable.\n",
            "---\n",
            "missing mandatory section: Assumptions (empty",
        ),
        (
            "- No lodgement, payment or other external action occurred.\n",
            "",
            "missing mandatory section: External-action record (empty",
        ),
    ] {
        let report = check(&DRAFT.replace(from, to), &conclusions());
        assert!(
            report.blocks.iter().any(|b| b.contains(finding)),
            "{finding}: {:?}",
            report.blocks
        );
    }
    let empty_stop = STOP.replace("- No external action occurred.\n", "");
    assert_eq!(verdict(&empty_stop).1, EXIT_BLOCK);
    let report = check(&DRAFT.replace("- None identified.\n", ""), &conclusions());
    assert_eq!(report.verdict().1, EXIT_PASS, "{:?}", report.blocks);
    assert_eq!(
        report.notes,
        ["empty Ethical and regulatory flags section; required where material"]
    );
}

#[test]
fn numbered_headings_and_source_value_notes_are_accepted() {
    let text = DRAFT
        .replace("Scope and period\n", "## 1. Scope and period\n")
        .replace("Assumptions\n", "4) Assumptions:\n")
        .replace(
            "Historical authority: CONFIRMED",
            "Historical authority: CONFIRMED (Federal Register, 20 October 2026)",
        );
    let report = check(&text, &conclusions());
    assert_eq!(report.verdict(), ("PASS", EXIT_PASS), "{:?}", report.blocks);
    let report = check(
        &DRAFT.replace(
            "Historical authority: CONFIRMED",
            "Historical authority: CONFIRMEDLY",
        ),
        &conclusions(),
    );
    assert!(
        report
            .blocks
            .iter()
            .any(|b| b.contains("Historical authority must be")),
        "{:?}",
        report.blocks
    );
}

#[test]
fn the_command_reports_and_exits_by_verdict() {
    let dir = tempfile::tempdir().unwrap();
    let root = repo_root();
    let root = root.to_str().unwrap();
    for (name, text, code) in [
        ("good.md", DRAFT.to_string(), EXIT_PASS),
        (
            "bad.md",
            DRAFT.replace("= $100.00", "= $110.00"),
            EXIT_BLOCK,
        ),
    ] {
        std::fs::write(dir.path().join(name), text).unwrap();
        let out = run_in(dir.path(), &["--root", root, "gate", name], false);
        assert_eq!(
            out.code,
            i32::from(code),
            "{name}: {}{}",
            out.stdout,
            out.stderr
        );
    }
    let out = run_in(
        dir.path(),
        &["--root", root, "gate", "--json", "bad.md"],
        false,
    );
    assert!(
        out.stdout.contains("\"verdict\": \"BLOCK\""),
        "{}",
        out.stdout
    );
    let missing = run_in(dir.path(), &["--root", root, "gate", "absent.md"], false);
    assert_eq!(missing.code, i32::from(EXIT_COULD_NOT_RUN));
    assert!(missing.stderr.starts_with("gate: "), "{}", missing.stderr);
    let _ = gate::EXIT_PASS;
}

const FACTS: &str = r#"{"facts": {"invoice total": "$1,100"}}"#;
/// The note for `DRAFT`'s `$1,100` with `FACTS`.
const NAMED: &str = "facts check: $1,100 is equal in amount to fact 'invoice total'";
const CAVEAT: &str = "facts check: it flags amounts it cannot trace; an amount that matches a fact or a calculation is not thereby correct";

/// The gate's findings with the facts check run on `text`.
fn with_facts(text: &str, facts: &str) -> gate::Report {
    let mut report = check(text, &conclusions());
    check_facts(text, &load_facts(facts).unwrap(), &mut report);
    report
}

fn facts_reviews(report: &gate::Report) -> Vec<&str> {
    report
        .reviews
        .iter()
        .map(String::as_str)
        .filter(|r| r.starts_with("facts check:"))
        .collect()
}

#[test]
fn a_supplied_fact_and_a_shown_calculation_pass_the_facts_check() {
    // $1,100 is supplied, and its 2 mentions are named once; $100.00 and the
    // later $100 are the result of `$1,100 / 11 = $100.00`, equal to no fact.
    let report = with_facts(DRAFT, FACTS);
    assert_eq!(
        report.verdict(),
        ("PASS", EXIT_PASS),
        "{:?}",
        report.reviews
    );
    assert_eq!(report.notes, [NAMED, CAVEAT]);
    // Without --facts the gate adds no facts-check note.
    assert!(check(DRAFT, &conclusions()).notes.is_empty());
}

#[test]
fn a_wrong_figure_equals_no_fact_and_asks_for_review() {
    // The stated total is mistyped; the calculation still uses the fact.
    let text = DRAFT.replace("Invoice total $1,100", "Invoice total $1,010");
    let report = with_facts(&text, FACTS);
    assert_eq!(
        report.verdict(),
        ("REVIEW", EXIT_REVIEW),
        "{:?}",
        report.blocks
    );
    assert_eq!(
        facts_reviews(&report),
        [
            "facts check: $1,010 not in the facts file or derived by a reperformed calculation; a person must trace each to its source"
        ]
    );
    assert_eq!(report.notes, [NAMED, CAVEAT]);
}

#[test]
fn an_amount_equal_to_2_facts_is_ambiguous_and_neither_is_picked() {
    let facts = r#"{"facts": {"invoice total": "$1,100", "Wexler McGill retainer": "1100.00"}}"#;
    let report = with_facts(DRAFT, facts);
    // Still equal to a fact, so the verdict is the single-fact one.
    assert_eq!(
        report.verdict(),
        ("PASS", EXIT_PASS),
        "{:?}",
        report.reviews
    );
    assert_eq!(
        report.notes,
        [
            "facts check: $1,100 is equal in amount to 2 facts, 'invoice total' and 'Wexler McGill retainer'; ambiguous, so the gate does not pick one",
            CAVEAT
        ]
    );
}

#[test]
fn each_written_amount_names_every_fact_it_equals() {
    // A percentage and a dollar amount of the same magnitude are not
    // ambiguous; a name prints as a quoted literal, so it keeps to one line.
    let facts = r#"{"facts": {"fee": "$10", "rate": "10%", "a": "$5", "b": "5", "c": "$5.00", "line\nbreak": "$7"}}"#;
    let report = with_facts("A $10 fee at 10%, then $5, $5.00 and $7.", facts);
    let notes: Vec<&str> = report
        .notes
        .iter()
        .map(String::as_str)
        .filter(|n| n.starts_with("facts check:"))
        .collect();
    assert_eq!(
        notes,
        [
            "facts check: $10 is equal in amount to fact 'fee'",
            "facts check: 10% is equal in amount to fact 'rate'",
            "facts check: $5 is equal in amount to 3 facts, 'a', 'b' and 'c'; ambiguous, so the gate does not pick one",
            "facts check: $5.00 is equal in amount to 3 facts, 'a', 'b' and 'c'; ambiguous, so the gate does not pick one",
            "facts check: $7 is equal in amount to fact 'line\\nbreak'",
            CAVEAT,
        ]
    );
}

#[test]
fn an_amount_without_a_fact_or_calculation_asks_for_review() {
    let text = DRAFT.replace(
        "- The supply is wholly taxable.",
        "- The supply is wholly taxable.\n- A deduction of $1,200 at 15% is available.",
    );
    let report = with_facts(&text, FACTS);
    assert_eq!(
        report.verdict(),
        ("REVIEW", EXIT_REVIEW),
        "{:?}",
        report.blocks
    );
    assert_eq!(
        facts_reviews(&report),
        [
            "facts check: $1,200, 15% not in the facts file or derived by a reperformed calculation; a person must trace each to its source"
        ]
    );
    // The result alone, with its calculation gone, is not traced.
    let text = DRAFT.replace("$1,100 / 11 = $100.00", "GST is $100.00");
    let report = with_facts(&text, FACTS);
    assert_eq!(report.verdict().1, EXIT_REVIEW);
    assert!(facts_reviews(&report)[0].starts_with("facts check: $100.00, $100 not"));
}

#[test]
fn part_of_a_bracketed_sum_is_neither_reperformed_nor_traced() {
    // Every amount is a fact, but `$250` is not the line's total of $350.
    let facts = r#"{"facts": {"invoice total": "$1,100", "a": "$100", "b": "$200", "c": "$50"}}"#;
    let text = DRAFT.replace("- None identified.", "- $100 + (wages $200 + $50) = $250");
    let report = with_facts(&text, facts);
    assert_eq!(report.verdict().1, EXIT_BLOCK, "{:?}", report.reviews);
    assert!(
        facts_reviews(&report)
            .first()
            .is_some_and(|r| r.starts_with("facts check: $250 not")),
        "{:?}",
        report.reviews
    );
}

#[test]
fn the_facts_check_never_blocks() {
    // A structurally complete draft full of untraced and unreadable amounts
    // reviews; a draft that already blocks keeps exactly its own blocks.
    let text = DRAFT.replace(
        "- None identified.",
        "- $5, $6.50, 7%, $1,1000 and $10% appear nowhere else.",
    );
    let report = with_facts(&text, FACTS);
    assert!(report.blocks.is_empty(), "{:?}", report.blocks);
    assert_eq!(report.verdict(), ("REVIEW", EXIT_REVIEW));
    assert_eq!(facts_reviews(&report).len(), 2, "{:?}", report.reviews);
    let broken = DRAFT.replace("= $100.00", "= $110.00");
    let blocks = check(&broken, &conclusions()).blocks;
    let report = with_facts(&broken, FACTS);
    assert_eq!(report.blocks, blocks);
    assert_eq!(report.verdict().1, EXIT_BLOCK);
}

#[test]
fn a_calculation_traces_only_from_traced_inputs_written_before_it() {
    let facts = r#"{"facts": {"a": "$500", "b": "$300", "c": "$200", "rate": "10%"}}"#;
    for (text, untraced) in [
        // A running total: $800 is derived, then $1,000.
        ("$500 + $300 = $800 + $200 = $1,000\nTotal $1,000.", ""),
        // A result feeds a later calculation.
        ("$500 + $300 = $800\n$800 × 10% = $80", ""),
        // A rounded result is traced as written.
        ("$500 / 3 = $166.67\nShare $166.67.", ""),
        // An untraced dollar input leaves the result untraced.
        ("$1,200 × 10% = $120", "$1,200, $120"),
        // A result used before the calculation that derives it.
        ("$800 × 10% = $80\n$500 + $300 = $800", "$80"),
        // A calculation that does not reperform derives nothing.
        ("$500 + $300 = $900", "$900"),
        // A rounded mention of a fact is not the fact.
        ("About $501 in total.", "$501"),
        // A percentage matches only a percentage, and a dollar amount only a
        // dollar amount or plain number.
        ("Rate $10 and 500%.", "$10, 500%"),
    ] {
        let report = with_facts(text, facts);
        let found = facts_reviews(&report);
        if untraced.is_empty() {
            assert!(found.is_empty(), "{text}: {found:?}");
        } else {
            let expected = format!("facts check: {untraced} not");
            assert!(
                found.first().is_some_and(|r| r.starts_with(&expected)),
                "{text}: {found:?}"
            );
        }
    }
}

#[test]
fn amounts_match_by_value_whatever_their_format() {
    let facts = r#"{"facts": {"a": "1234.5", "b": "-$500", "c": "12.50%", "d": "$2,000.00"}}"#;
    let text = "$1,234.50 and $1234.5, a loss of -$500 or $500, 12.5% and $2000.";
    let report = with_facts(text, facts);
    assert!(facts_reviews(&report).is_empty(), "{:?}", report.reviews);
}

#[test]
fn an_unreadable_draft_amount_is_reported() {
    let report = with_facts("Totals $1,1000 and $10% and $1.2.3.", FACTS);
    assert_eq!(
        facts_reviews(&report),
        ["facts check: could not read $1,1000, $10%, $1.2.3; a person must check each"]
    );
}

#[test]
fn a_malformed_facts_file_is_refused() {
    for (text, reason) in [
        ("{", "not valid JSON"),
        ("[]", "top level must be an object"),
        (
            r#"{"facts": {"a": "$1"}, "notes": 1}"#,
            "unknown key 'notes'",
        ),
        (r#"{"facts": {}}"#, "'facts' must map names to amounts"),
        (r#"{"facts": ["$1"]}"#, "'facts' must map names to amounts"),
        (
            r#"{"facts": {"a": 1100}}"#,
            "'a' must be an amount written as text",
        ),
        (
            r#"{"facts": {"a": "$1,23"}}"#,
            "'a' is not an amount the gate can read: '$1,23'",
        ),
        (r#"{"facts": {"a": "$10%"}}"#, "'a' is not an amount"),
        (r#"{"facts": {"a": "1 100"}}"#, "'a' is not an amount"),
        (r#"{"facts": {"a": " $5"}}"#, "'a' is not an amount"),
        (r#"{"facts": {"a": ""}}"#, "'a' is not an amount"),
        (r#"{"facts": {"a": "$1", "a": "$2"}}"#, "not valid JSON"),
    ] {
        let Err(e) = load_facts(text) else {
            panic!("{text} was accepted");
        };
        assert!(e.starts_with("facts file: "), "{text}: {e}");
        assert!(e.contains(reason), "{text}: {e}");
    }
}

#[test]
fn the_command_reads_a_facts_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = repo_root();
    let root = root.to_str().unwrap();
    std::fs::write(dir.path().join("draft.md"), DRAFT).unwrap();
    std::fs::write(
        dir.path().join("invented.md"),
        DRAFT.replace("- None identified.", "- Deduction $1,200."),
    )
    .unwrap();
    std::fs::write(dir.path().join("facts.json"), FACTS).unwrap();
    std::fs::write(dir.path().join("bad.json"), r#"{"facts": {"a": 1}}"#).unwrap();
    let gate = |args: &[&str]| {
        let mut all = vec!["--root", root, "gate"];
        all.extend(args);
        run_in(dir.path(), &all, false)
    };
    let out = gate(&["--facts", "facts.json", "draft.md"]);
    assert_eq!(
        out.code,
        i32::from(EXIT_PASS),
        "{}{}",
        out.stdout,
        out.stderr
    );
    assert!(
        out.stdout.contains("note: facts check: it flags amounts"),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout.contains(&format!("  note: {NAMED}\n")),
        "{}",
        out.stdout
    );
    let out = gate(&["--facts", "facts.json", "--json", "invented.md"]);
    assert_eq!(
        out.code,
        i32::from(EXIT_REVIEW),
        "{}{}",
        out.stdout,
        out.stderr
    );
    assert!(
        out.stdout.contains("\"verdict\": \"REVIEW\""),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("facts check: $1,200 not"),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout.contains(&format!("\"{NAMED}\"")),
        "{}",
        out.stdout
    );
    for facts in ["bad.json", "absent.json"] {
        let out = gate(&["--facts", facts, "draft.md"]);
        assert_eq!(
            out.code,
            i32::from(EXIT_COULD_NOT_RUN),
            "{facts}: {}",
            out.stdout
        );
        assert!(
            out.stderr.starts_with("gate: facts file: "),
            "{facts}: {}",
            out.stderr
        );
        assert!(out.stdout.is_empty(), "{facts}: {}", out.stdout);
    }
}

#[test]
fn an_amount_is_read_whole_or_reported_never_matched_by_a_prefix() {
    // Each prefix of these is a supplied fact, so a prefix match would pass.
    let facts = r#"{"facts": {"a": "$1.2", "b": "$1,200", "c": "0.5%", "d": "10%", "e": "$1", "f": "$5", "invoice": "$1,100"}}"#;
    let failed = [
        "$1.2 million",
        "$1.2M",
        "$1.2 Thousand",
        "$5k",
        "$5 bn",
        "$ 1,200",
        "$1 200",
        ".5%",
        "10 %",
        "$10 %",
        // Runs of spaces, and no-break (U+00A0), narrow no-break (U+202F)
        // and thin (U+2009) spaces, in each position.
        "$1  200",
        "$  1,200",
        "10  %",
        "$1.2  million",
        "$1.2\u{a0}million",
        "$\u{a0}1,200",
        "10\u{a0}%",
        "$1\u{a0}200",
        "$1\u{202f}200",
        "$1.2\u{2009}million",
        "$\u{2009}1,200",
        "10\u{2009}%",
        "$1\u{2009}200",
        "$10\u{2009}%",
        "$1\u{2009}200\u{2009}000",
    ]
    .into_iter()
    .filter(|written| {
        let text = DRAFT.replace("- None identified.", &format!("- A figure of {written}."));
        let report = with_facts(&text, facts);
        let expected = format!("facts check: could not read {written}; a person must check each");
        !(report.blocks.is_empty()
            && report.verdict() == ("REVIEW", EXIT_REVIEW)
            && facts_reviews(&report) == [expected.as_str()])
    })
    .collect::<Vec<_>>();
    assert!(failed.is_empty(), "not reported whole: {failed:?}");
    // A word that only starts like a scale word, sentence punctuation and a
    // percentage inside a word are not part of an amount.
    for text in ["$5 more", "$1,200.", "$1,200, then", "ref x12%"] {
        let report = with_facts(text, facts);
        assert!(
            facts_reviews(&report).is_empty(),
            "{text}: {:?}",
            report.reviews
        );
    }
    // A skipped match does not hide an amount that starts inside it, and a
    // list after punctuation is read item by item.
    for (text, untraced) in [
        ("ab1 200%", "200%"),
        ("x1,10% then 20%", "20%"),
        ("10%,15%,20%", "15%, 20%"),
        ("10%, 0.5%,25%", "25%"),
    ] {
        let expected = format!(
            "facts check: {untraced} not in the facts file or derived by a reperformed calculation; a person must trace each to its source"
        );
        assert_eq!(
            facts_reviews(&with_facts(text, facts)),
            [expected.as_str()],
            "{text}"
        );
    }
}

#[test]
fn a_bare_result_is_dollars_only_when_its_calculation_leaves_no_doubt() {
    // The GST example: `100` is dollars, so the later `$100` is traced.
    let text = DRAFT.replace("$1,100 / 11 = $100.00", "$1,100 / 11 = 100");
    let report = with_facts(&text, FACTS);
    assert_eq!(
        report.verdict(),
        ("PASS", EXIT_PASS),
        "{:?}",
        report.reviews
    );
    let facts = r#"{"facts": {"a": "$500", "b": "$250", "rate": "10%"}}"#;
    for (text, untraced) in [
        ("$500 + $250 = 750\nTotal $750.", ""),
        ("$500 × 10% = 50\nTax $50.", ""),
        (
            "2 × $250 = 500 + $250 = 750\nSubtotal $500, total $750.",
            "",
        ),
        ("-($500 - $250) = -250\nA loss of $250.", ""),
        // Dollars divided by dollars is a ratio, not dollars.
        ("$500 / $250 = 2\nRatio $2.", "$2"),
        // Mixed or doubtful units establish nothing.
        ("$500 + 10 = 510\nTotal $510.", "$510"),
        ("10 / $250 = 0.04\nRate $0.04.", "$0.04"),
        ("$500 × $250 = 125000\nProduct $125,000.", "$125,000"),
    ] {
        let report = with_facts(text, facts);
        let found = facts_reviews(&report);
        if untraced.is_empty() {
            assert!(found.is_empty(), "{text}: {found:?}");
        } else {
            let expected = format!("facts check: {untraced} not");
            assert!(
                found.first().is_some_and(|r| r.starts_with(&expected)),
                "{text}: {found:?}"
            );
        }
    }
}

#[test]
fn the_facts_check_only_adds_its_own_findings() {
    only_adds_its_own_findings("facts check:", |text| with_facts(text, FACTS));
}

/// Every gate fixture gives the same status, blocks, calculations and other
/// findings with and without an optional check, whose findings start with
/// `prefix`; the check can raise PASS to REVIEW, never to BLOCK.
fn only_adds_its_own_findings(prefix: &str, with_check: impl Fn(&str) -> gate::Report) {
    let fixtures = [
        DRAFT.to_string(),
        cited(),
        cited().replace("TR 2006/10.", "TR 2019/7 and s 8-1 ITAA 1997."),
        STOP.to_string(),
        DRAFT.replace("= $100.00", "= $110.00"),
        DRAFT.replace("$1,100 / 11 = $100.00", "$1,100 / 0 = $0"),
        DRAFT.replace("DRAFT — PROFESSIONAL REVIEW REQUIRED", "Draft for review"),
        DRAFT.replace(
            "state_changing_tool_action: NONE",
            "state_changing_tool_action: PAY_NOW",
        ),
        STOP.replace(
            "Concluding on the safe harbour is prohibited",
            "I will not say the safe harbour applies to your position",
        ),
        DRAFT.replace("- None identified.", "- ($100) + $50 = $150; $9 and 7%."),
    ];
    let own_finding = |s: &String| s.starts_with(prefix);
    for text in &fixtures {
        let without = check(text, &conclusions());
        let with = with_check(text);
        assert_eq!(with.status, without.status, "{text}");
        assert_eq!(with.blocks, without.blocks, "{text}");
        let shown = |r: &gate::Report| {
            r.calculations
                .iter()
                .map(|c| (c.text.clone(), c.computed.clone(), c.matches))
                .collect::<Vec<_>>()
        };
        assert_eq!(shown(&with), shown(&without), "{text}");
        let other = |items: &[String]| {
            items
                .iter()
                .filter(|s| !own_finding(s))
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(other(&with.reviews), without.reviews, "{text}");
        assert_eq!(other(&with.notes), without.notes, "{text}");
        assert!(!without.reviews.iter().any(own_finding), "{text}");
        assert!(!without.notes.iter().any(own_finding), "{text}");
        // The check can raise PASS to REVIEW, never to BLOCK.
        if without.verdict().1 != EXIT_BLOCK {
            assert_ne!(with.verdict().1, EXIT_BLOCK, "{text}");
        }
    }
}

const AUTHORITIES: &str = r#"{"authorities": ["s 9-70 of the A New Tax System (Goods and Services Tax) Act 1999", "TR 2006/10"]}"#;
const AUTHORITIES_CAVEAT: &str = "authorities check: it reads only a section of a named Act, or a subsection, paragraph or subparagraph of one, and a ruling id, so a pass does not show that every authority the draft cites was checked; a listed authority is not thereby current or correctly applied";

/// `DRAFT` with its authority cited in full, and a ruling.
fn cited() -> String {
    DRAFT.replace(
        "- GST Act s 9-70.",
        "- s 9-70 of the A New Tax System (Goods and Services Tax) Act 1999; TR 2006/10.",
    )
}

/// The gate's findings with the authorities check run on `text`.
fn with_authorities(text: &str, authorities: &str) -> gate::Report {
    let mut report = check(text, &conclusions());
    check_authorities(text, &load_authorities(authorities).unwrap(), &mut report);
    report
}

fn authorities_reviews(report: &gate::Report) -> Vec<&str> {
    report
        .reviews
        .iter()
        .map(String::as_str)
        .filter(|r| r.starts_with("authorities check:"))
        .collect()
}

fn unlisted(written: &str) -> String {
    format!(
        "authorities check: {written} not in the authorities file; a person must check each against its source"
    )
}

fn unread(written: &str) -> String {
    format!(
        "authorities check: could not read {written} as a section, subsection, paragraph or subparagraph of a named Act or a ruling id; a person must check each"
    )
}

#[test]
fn listed_authorities_pass_the_authorities_check() {
    let report = with_authorities(&cited(), AUTHORITIES);
    assert_eq!(
        report.verdict(),
        ("PASS", EXIT_PASS),
        "{:?}",
        report.reviews
    );
    assert_eq!(report.notes, [AUTHORITIES_CAVEAT]);
    // Without --authorities the gate adds no authorities-check finding.
    assert!(check(&cited(), &conclusions()).notes.is_empty());
}

#[test]
fn an_unlisted_authority_asks_for_review() {
    // A ruling the reviewer did not accept, and another section of a listed
    // Act; each is reported once, in order.
    let text = cited().replace(
        "TR 2006/10.",
        "TR 2006/10 and TR 2019/7, then s 9-75 of the A New Tax System (Goods and Services Tax) Act 1999 and TR 2019/7 again.",
    );
    let report = with_authorities(&text, AUTHORITIES);
    assert_eq!(
        report.verdict(),
        ("REVIEW", EXIT_REVIEW),
        "{:?}",
        report.blocks
    );
    assert_eq!(
        authorities_reviews(&report),
        [unlisted(
            "TR 2019/7, s 9-75 of the A New Tax System (Goods and Services Tax) Act 1999"
        )]
    );
    assert_eq!(report.notes, [AUTHORITIES_CAVEAT]);
}

#[test]
fn the_check_reads_a_section_of_a_named_act_and_a_ruling_id() {
    let listed = r#"{"authorities": [
        "s 8-1 of the Income Tax Assessment Act 1997",
        "s 6(5C)(b) of the Anti-Money Laundering and Counter-Terrorism Financing Act 2006",
        "s 6D(4) of the Privacy Act 1988",
        "s 100A of the Income Tax Assessment Act 1936",
        "TR 2006/10", "TR 93/31", "LCR 2026/D1", "TR 2010/3W", "PS LA 1998/1",
        "TPB(GS) 55/2026", "GSTD 2026/1", "PCG 2016/1"
    ]}"#;
    // Each citation is read whole, whatever its spacing, emphasis or
    // surrounding punctuation, and found in the list.
    for text in [
        "Under s 8-1 of the Income Tax Assessment Act 1997, the outgoing is deductible.",
        "Section 8-1 of the Income Tax Assessment Act 1997 applies.",
        "**s 8-1** of the  Income Tax\u{a0}Assessment Act 1997",
        "'s 8-1 of the Income Tax Assessment Act 1997'",
        "see s 6(5C)(b) of the Anti-Money Laundering and Counter-Terrorism Financing Act 2006",
        "(s 6D(4) of the Privacy Act 1988)",
        "s 100A of the Income Tax Assessment Act 1936 (Cth)",
        "TR 2006/10, TR 93/31 and LCR 2026/D1.",
        "the withdrawal TR 2010/3W",
        "PS  LA 1998/1 and `TPB(GS) 55/2026`",
        "GSTD 2026/1; PCG 2016/1",
        // A possessive or a contraction before a number is not a section.
        "The client's 2 trusts, the Commissioner’s 3 letters; it's 5 pm.",
    ] {
        let report = with_authorities(text, listed);
        assert!(
            authorities_reviews(&report).is_empty(),
            "{text}: {:?}",
            report.reviews
        );
    }
}

#[test]
fn a_citation_matches_only_the_same_authority() {
    let listed = r#"{"authorities": ["s 8-1 of the Income Tax Assessment Act 1997", "TR 2010/3"]}"#;
    for text in [
        // The same section of another Act is another authority.
        "s 8-1 of the Income Tax Assessment Act 1936",
        "s 8-1 of the Income Tax Rates Act 1986",
        // A withdrawal, an addendum and a draft are other documents.
        "TR 2010/3W",
        "TR 2010/3A1",
        "TR 2010/D3",
        "GSTR 2010/3",
    ] {
        let report = with_authorities(text, listed);
        assert_eq!(
            authorities_reviews(&report),
            [unlisted(text)],
            "{text}: {:?}",
            report.reviews
        );
    }
}

const SAMPLE_ACT: &str = "of the Sample Consumption Tax Act 2099";

/// Gate a draft whose only authority is `citation` of the sample Act: PASS
/// when `listed`, otherwise REVIEW naming the citation as unlisted.
fn assert_listed(citation: &str, authorities: &str, listed: bool) {
    let written = format!("{citation} {SAMPLE_ACT}");
    let draft = DRAFT.replace("- GST Act s 9-70.", &format!("- {written}."));
    let report = with_authorities(&draft, authorities);
    if listed {
        assert_eq!(
            report.verdict(),
            ("PASS", EXIT_PASS),
            "{written}: {:?}",
            report.reviews
        );
    } else {
        assert_eq!(report.verdict(), ("REVIEW", EXIT_REVIEW), "{written}");
        assert_eq!(
            authorities_reviews(&report),
            [unlisted(&written)],
            "{written}"
        );
    }
}

#[test]
fn a_listed_section_lists_its_subsections_and_paragraphs() {
    let listed = format!(r#"{{"authorities": ["s 1 {SAMPLE_ACT}", "s 8-1 {SAMPLE_ACT}"]}}"#);
    for citation in ["s 8-1(1)", "subsection 8-1(1)", "paragraph 8-1(1)(a)"] {
        assert_listed(citation, &listed, true);
    }
    // Without the section, a subsection's citation asks for review.
    let without = format!(r#"{{"authorities": ["s 1 {SAMPLE_ACT}"]}}"#);
    assert_listed("subsection 8-1(1)", &without, false);
}

#[test]
fn a_listed_subsection_lists_what_it_contains_only() {
    let listed = format!(r#"{{"authorities": ["s 1 {SAMPLE_ACT}", "s 9-5(1) {SAMPLE_ACT}"]}}"#);
    // Itself, whatever word cites it, and a paragraph within it, but not a
    // sibling or its whole section.
    for (citation, is_listed) in [
        ("subsection 9-5(2)", false),
        ("s 9-5(1)", true),
        ("subsection 9-5(1)", true),
        ("paragraph 9-5(1)(a)", true),
        ("s 9-5", false),
    ] {
        assert_listed(citation, &listed, is_listed);
    }
    // A paragraph named before its subsection is read as the subsection, so
    // a listed paragraph does not list it.
    let paragraph =
        format!(r#"{{"authorities": ["s 1 {SAMPLE_ACT}", "s 9-5(1)(a) {SAMPLE_ACT}"]}}"#);
    let draft = DRAFT.replace(
        "- GST Act s 9-70.",
        &format!("- paragraph (a) of subsection 9-5(1) {SAMPLE_ACT}."),
    );
    let report = with_authorities(&draft, &paragraph);
    assert_eq!(
        authorities_reviews(&report),
        [unlisted(&format!("subsection 9-5(1) {SAMPLE_ACT}"))]
    );
    // An authorities file may name a subsection by its word.
    assert_eq!(
        load_authorities(&format!(
            r#"{{"authorities": ["subsection 9-5(1) {SAMPLE_ACT}"]}}"#
        )),
        load_authorities(&format!(r#"{{"authorities": ["s 9-5(1) {SAMPLE_ACT}"]}}"#)),
    );
}

#[test]
fn an_abbreviation_with_a_full_stop_or_no_space_is_read() {
    for text in [
        "s. 8-1(1)",
        "sec 8-1(1)",
        "sec. 8-1(1)",
        "subsec 8-1(1)",
        "sub-s. 8-1(1)",
        "subs. 8-1(1)",
        "para. 8-1(1)(a)",
        "s8-1(1)",
        "§ 8-1(1)",
    ] {
        let written = format!("{text} {SAMPLE_ACT}");
        let draft = cited().replace("- None identified.", &format!("- {written}."));
        let report = with_authorities(&draft, AUTHORITIES);
        assert_eq!(
            authorities_reviews(&report),
            [unlisted(&written)],
            "{text}: {:?}",
            report.reviews
        );
    }
    let listed = format!(r#"{{"authorities": ["s 1 {SAMPLE_ACT}", "s 8-1 {SAMPLE_ACT}"]}}"#);
    assert_listed("s. 8-1(1)", &listed, true);
    assert_listed("§ 8-1(1)", &listed, true);
}

#[test]
fn a_part_named_without_its_section_is_reported() {
    let listed = format!(
        r#"{{"authorities": ["s 1 {SAMPLE_ACT}", "s 8-1 {SAMPLE_ACT}", "s 9-5(1) {SAMPLE_ACT}"]}}"#
    );
    for (text, unread_text) in [
        (
            format!("subsection 9-5(1) {SAMPLE_ACT} and subsection (2)"),
            "subsection (2)",
        ),
        (
            format!("s 1 {SAMPLE_ACT}; see also paragraph (b) of subsection (2)"),
            "paragraph (b), subsection (2)",
        ),
    ] {
        let draft = DRAFT.replace("- GST Act s 9-70.", &format!("- {text}."));
        let report = with_authorities(&draft, &listed);
        assert_eq!(
            authorities_reviews(&report),
            [unread(unread_text)],
            "{text}: {:?}",
            report.reviews
        );
    }
    // A composite ending in a numbered provision is read as that provision.
    for text in [
        format!("subsection (1) of section 8-1 {SAMPLE_ACT}"),
        format!("paragraph (a) of subsection 9-5(1) {SAMPLE_ACT}"),
        format!("subparagraph (i) of paragraph (a) of subsection 9-5(1) {SAMPLE_ACT}"),
    ] {
        let draft = DRAFT.replace("- GST Act s 9-70.", &format!("- {text}."));
        let report = with_authorities(&draft, &listed);
        assert_eq!(
            report.verdict(),
            ("PASS", EXIT_PASS),
            "{text}: {:?}",
            report.reviews
        );
    }
}

#[test]
fn a_composite_ending_in_text_the_gate_does_not_read_is_reported() {
    // `paragraph 3 of Schedule 1` is no citation the gate reads, so the part
    // that opens the composite cannot be passed over.
    let listed = format!(r#"{{"authorities": ["s 1 {SAMPLE_ACT}"]}}"#);
    let draft = DRAFT.replace(
        "- GST Act s 9-70.",
        "- subparagraph (a) of paragraph 3 of Schedule 1 to the Sample Consumption Tax Act 2099.",
    );
    let report = with_authorities(&draft, &listed);
    assert_eq!(
        authorities_reviews(&report),
        [unread("subparagraph (a)")],
        "{:?}",
        report.reviews
    );
}

#[test]
fn sibling_parts_after_the_act_are_reported() {
    // Only (1) is listed; (2) and (3) are cited too.
    let listed = format!(r#"{{"authorities": ["s 1 {SAMPLE_ACT}", "s 9-5(1) {SAMPLE_ACT}"]}}"#);
    for siblings in [", (2) and (3)", ", and (2)"] {
        let written = format!("subsection 9-5(1) {SAMPLE_ACT}{siblings}");
        let draft = DRAFT.replace("- GST Act s 9-70.", &format!("- {written}."));
        let report = with_authorities(&draft, &listed);
        assert_eq!(
            authorities_reviews(&report),
            [unread(&written)],
            "{written}: {:?}",
            report.reviews
        );
    }
}

#[test]
fn ruling_paragraph_ranges_and_source_labels_are_not_read() {
    for qualifier in ["", "the ", "the ruling "] {
        let draft = cited().replace(
            "- None identified.",
            &format!("- See paragraphs 45-50 of {qualifier}TR 2006/10 [S1] and source S3."),
        );
        let report = with_authorities(&draft, AUTHORITIES);
        assert!(
            authorities_reviews(&report).is_empty(),
            "{:?}",
            report.reviews
        );
    }
    let draft = cited().replace(
        "- None identified.",
        "- See paragraphs 45-50 of the ruling TPB(GS) 55/2026.",
    );
    let report = with_authorities(&draft, AUTHORITIES);
    assert_eq!(
        authorities_reviews(&report),
        [unlisted("TPB(GS) 55/2026")],
        "{:?}",
        report.reviews
    );
}

#[test]
fn an_act_qualified_paragraph_list_is_reported_whole() {
    let written = format!("paragraphs 8-1 and 8-2 {SAMPLE_ACT}");
    let draft = cited().replace("- None identified.", &format!("- {written}."));
    let report = with_authorities(&draft, AUTHORITIES);
    assert_eq!(
        authorities_reviews(&report),
        [unread(&written)],
        "{:?}",
        report.reviews
    );
}

#[test]
fn each_word_for_a_section_or_part_of_one_is_read() {
    // Each is read whole and checked, so each unlisted one asks for review.
    for text in [
        "s9(1)",
        "subsection 8-1(1)",
        "Subsection 8-1(1)",
        "sub-section 8-1(1)",
        "subs 8-1(1)",
        "sub-s 8-1(1)",
        "paragraph 8-1(1)(a)",
        "para 8-1(1)(a)",
        "subparagraph 8-1(1)(a)(i)",
        "sub-paragraph 8-1(1)(a)(i)",
        "subpara 8-1(1)(a)(i)",
    ] {
        let written = format!("{text} {SAMPLE_ACT}");
        let draft = cited().replace("- None identified.", &format!("- {written}."));
        let report = with_authorities(&draft, AUTHORITIES);
        assert_eq!(
            authorities_reviews(&report),
            [unlisted(&written)],
            "{text}: {:?}",
            report.reviews
        );
    }
    // A paragraph of a ruling or a document has no bracketed part and is not
    // a provision.
    let draft = cited().replace(
        "- None identified.",
        "- See paragraph 45 of TR 2006/10 and paragraphs 3 to 5 above.",
    );
    let report = with_authorities(&draft, AUTHORITIES);
    assert!(
        authorities_reviews(&report).is_empty(),
        "{:?}",
        report.reviews
    );
}

#[test]
fn authority_like_text_the_gate_cannot_read_is_reported() {
    for (text, written) in [
        // An abbreviated or unnamed Act.
        ("s 8-1 ITAA 1997", "s 8-1"),
        ("under s 8-1 of the ITAA 1997", "s 8-1"),
        ("GST Act s 9-70.", "s 9-70"),
        ("s 8-1 of the Income Tax Assessment Act", "s 8-1"),
        // A list of sections.
        (
            "ss 30-35 and 30-40 of the Tax Agent Services Act 2009",
            "ss 30-35",
        ),
        ("sections 15, 45 and 101", "sections 15"),
        // A subsection or paragraph of an abbreviated Act, a list of them, or
        // one named without its bracketed part.
        ("subsection 8-1(1) of the ITAA 1997", "subsection 8-1(1)"),
        ("paragraph 8-1(1)(a) ITAA 1997", "paragraph 8-1(1)(a)"),
        (
            "subsections 8-1(1) and (2) of the Income Tax Assessment Act 1997",
            "subsections 8-1(1) and (2)",
        ),
        (
            "ss 8-1(1) of the Income Tax Assessment Act 1997",
            "ss 8-1(1) of the Income Tax Assessment Act 1997",
        ),
        (
            "subsection 8-1 of the Income Tax Assessment Act 1997",
            "subsection 8-1 of the Income Tax Assessment Act 1997",
        ),
        (
            "paragraph 8-1 of the Income Tax Assessment Act 1997",
            "paragraph 8-1 of the Income Tax Assessment Act 1997",
        ),
        ("paragraph 8-1 of the ITAA 1997", "paragraph 8-1"),
        // A plural before a named Act.
        (
            "subsections 8-1(1) of the Income Tax Assessment Act 1997",
            "subsections 8-1(1) of the Income Tax Assessment Act 1997",
        ),
        (
            "paragraphs 8-1(1)(a) of the Income Tax Assessment Act 1997",
            "paragraphs 8-1(1)(a) of the Income Tax Assessment Act 1997",
        ),
        // A ruling id without its number, run together or with a lowercase
        // suffix.
        ("TR 2006", "TR 2006"),
        ("TR2006/10", "TR2006/10"),
        ("TR 2006/10a", "TR 2006/10a"),
        // A year of 3 digits: ruling years have 2 (before 1999) or 4.
        ("TR 123/1", "TR 123/1"),
    ] {
        let draft = cited().replace("- None identified.", &format!("- {text}"));
        let report = with_authorities(&draft, AUTHORITIES);
        assert_eq!(
            report.verdict(),
            ("REVIEW", EXIT_REVIEW),
            "{text}: {:?}",
            report.blocks
        );
        assert_eq!(
            authorities_reviews(&report),
            [unread(written)],
            "{text}: {:?}",
            report.reviews
        );
    }
}

#[test]
fn a_listed_section_with_a_malformed_year_is_not_read() {
    // The year must end the citation: `Act 19970` is not `Act 1997`.
    let listed = r#"{"authorities": ["s 9-70 of the A New Tax System (Goods and Services Tax) Act 1999", "TR 2006/10", "s 8-1 of the Income Tax Assessment Act 1997"]}"#;
    for text in [
        "s 8-1 of the Income Tax Assessment Act 19970",
        "s 8-1 of the Income Tax Assessment Act 1997a",
    ] {
        let draft = cited().replace("- None identified.", &format!("- {text}."));
        let report = with_authorities(&draft, listed);
        assert_eq!(
            report.verdict(),
            ("REVIEW", EXIT_REVIEW),
            "{text}: {:?}",
            report.reviews
        );
        assert_eq!(
            authorities_reviews(&report),
            [unread("s 8-1")],
            "{text}: {:?}",
            report.reviews
        );
    }
}

#[test]
fn a_citation_wrapped_across_lines_is_read_whole() {
    // A line break is whitespace: each wrapped citation is read, unlisted
    // ones ask for review and is shown on one line.
    for (text, review) in [
        ("TR\n2019/7", unlisted("TR 2019/7")),
        (
            "s\n8-1 of the Income Tax\n  Assessment Act 1997",
            unlisted("s 8-1 of the Income Tax Assessment Act 1997"),
        ),
        ("TR\n2006", unread("TR 2006")),
    ] {
        let draft = cited().replace("- None identified.", &format!("- {text}."));
        let report = with_authorities(&draft, AUTHORITIES);
        assert_eq!(
            report.verdict(),
            ("REVIEW", EXIT_REVIEW),
            "{text}: {:?}",
            report.reviews
        );
        assert_eq!(
            authorities_reviews(&report),
            [review.as_str()],
            "{text}: {:?}",
            report.reviews
        );
    }
    // A listed citation wrapped inside a list item still matches.
    let draft = cited().replace("- None identified.", "- Under TR\n  2006/10.");
    let report = with_authorities(&draft, AUTHORITIES);
    assert_eq!(
        report.verdict(),
        ("PASS", EXIT_PASS),
        "{:?}",
        report.reviews
    );
}

#[test]
fn the_authorities_check_only_adds_its_own_findings() {
    only_adds_its_own_findings("authorities check:", |text| {
        with_authorities(text, AUTHORITIES)
    });
}

#[test]
fn a_malformed_authorities_file_is_refused() {
    for (text, reason) in [
        ("{", "not valid JSON"),
        ("[]", "top level must be an object"),
        (
            r#"{"authorities": ["TR 2006/10"], "facts": {}}"#,
            "unknown key 'facts'",
        ),
        (
            r#"{"authorities": []}"#,
            "'authorities' must list authorities",
        ),
        (
            r#"{"authorities": {"a": "TR 2006/10"}}"#,
            "'authorities' must list authorities",
        ),
        (
            r#"{"authorities": [2006]}"#,
            "each authority must be written as text",
        ),
        (
            r#"{"authorities": ["ITAA 1997 s 8-1"]}"#,
            "not an authority the gate can read: 'ITAA 1997 s 8-1'",
        ),
        (r#"{"authorities": ["s 8-1"]}"#, "not an authority"),
        // A subsection or paragraph must name its bracketed part.
        (
            r#"{"authorities": ["subsection 8-1 of the Income Tax Assessment Act 1997"]}"#,
            "not an authority",
        ),
        (
            r#"{"authorities": ["paragraph 8-1 of the Income Tax Assessment Act 1997"]}"#,
            "not an authority",
        ),
        (r#"{"authorities": [" TR 2006/10"]}"#, "not an authority"),
        (
            r#"{"authorities": ["TR 2006/10, TR 93/31"]}"#,
            "not an authority",
        ),
        (r#"{"authorities": [""]}"#, "not an authority"),
        (
            r#"{"authorities": ["TR 123/1"]}"#,
            "not an authority the gate can read: 'TR 123/1'",
        ),
    ] {
        let Err(e) = load_authorities(text) else {
            panic!("{text} was accepted");
        };
        assert!(e.starts_with("authorities file: "), "{text}: {e}");
        assert!(e.contains(reason), "{text}: {e}");
    }
}

#[test]
fn the_command_reads_an_authorities_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = repo_root();
    let root = root.to_str().unwrap();
    std::fs::write(dir.path().join("cited.md"), cited()).unwrap();
    std::fs::write(
        dir.path().join("unlisted.md"),
        cited().replace("TR 2006/10.", "TR 2019/7."),
    )
    .unwrap();
    std::fs::write(dir.path().join("authorities.json"), AUTHORITIES).unwrap();
    std::fs::write(dir.path().join("facts.json"), FACTS).unwrap();
    std::fs::write(dir.path().join("bad.json"), r#"{"authorities": ["s 8-1"]}"#).unwrap();
    std::fs::write(
        dir.path().join("bad-year.json"),
        r#"{"authorities": ["TR 123/1"]}"#,
    )
    .unwrap();
    let gate = |args: &[&str]| {
        let mut all = vec!["--root", root, "gate"];
        all.extend(args);
        run_in(dir.path(), &all, false)
    };
    let out = gate(&["--authorities", "authorities.json", "cited.md"]);
    assert_eq!(
        out.code,
        i32::from(EXIT_PASS),
        "{}{}",
        out.stdout,
        out.stderr
    );
    assert!(
        out.stdout
            .contains(&format!("  note: {AUTHORITIES_CAVEAT}\n")),
        "{}",
        out.stdout
    );
    let out = gate(&["--authorities", "authorities.json", "--json", "unlisted.md"]);
    assert_eq!(
        out.code,
        i32::from(EXIT_REVIEW),
        "{}{}",
        out.stdout,
        out.stderr
    );
    assert!(
        out.stdout.contains("\"verdict\": \"REVIEW\""),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains(&format!("\"{}\"", unlisted("TR 2019/7"))),
        "{}",
        out.stdout
    );
    // With --facts as well, both checks run.
    let out = gate(&[
        "--facts",
        "facts.json",
        "--authorities",
        "authorities.json",
        "cited.md",
    ]);
    assert_eq!(
        out.code,
        i32::from(EXIT_PASS),
        "{}{}",
        out.stdout,
        out.stderr
    );
    assert!(
        out.stdout.contains(&format!("  note: {NAMED}\n")),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains(&format!("  note: {AUTHORITIES_CAVEAT}\n")),
        "{}",
        out.stdout
    );
    for authorities in ["bad.json", "bad-year.json", "absent.json"] {
        let out = gate(&["--authorities", authorities, "cited.md"]);
        assert_eq!(
            out.code,
            i32::from(EXIT_COULD_NOT_RUN),
            "{authorities}: {}",
            out.stdout
        );
        assert!(
            out.stderr.starts_with("gate: authorities file: "),
            "{authorities}: {}",
            out.stderr
        );
        assert!(out.stdout.is_empty(), "{authorities}: {}", out.stdout);
    }
}

const SOURCE: &str =
    "An approved auditor must obtain sufficient appropriate evidence to support the opinion.";

/// The gate's findings with the quotes check run on `text` against `SOURCE`.
fn with_quotes(text: &str) -> gate::Report {
    let mut report = gate::Report::default();
    check_quotes(text, &[SOURCE.to_string()], &mut report);
    report
}

#[test]
fn a_verbatim_quotation_passes_and_says_it_is_text_membership_only() {
    for draft in [
        "The ruling says \"sufficient appropriate evidence\" is needed.",
        "The ruling says \u{201c}sufficient appropriate evidence\u{201d} is needed.",
        "The ruling says \u{2018}sufficient appropriate evidence\u{2019} is needed.",
        "The ruling says 'sufficient appropriate evidence' is needed.",
        "As stated:\n\n> sufficient appropriate evidence\n\nSo it is needed.",
        "As stated:\n\n> An approved auditor must obtain\n> sufficient appropriate evidence\n\nDone.",
        "As stated:\n\n> An approved auditor must obtain\n> sufficient appropriate evidence",
    ] {
        let report = with_quotes(draft);
        assert_eq!(report.reviews, Vec::<String>::new(), "{draft}");
        assert!(report.notes[0].starts_with("quotes check: 1 quoted span(s) found verbatim"));
        assert!(report.notes[0].contains("not support for the claim"));
    }
}

#[test]
fn a_fabricated_quotation_asks_for_review_and_never_blocks() {
    let report = with_quotes("The ruling says \"evidence may be waived\" here.");
    assert_eq!(
        report.reviews,
        [
            "quotes check: 'evidence may be waived' is not in any supplied source; a person must check it"
        ]
    );
    assert!(report.blocks.is_empty());
    assert_eq!(report.verdict(), ("REVIEW", EXIT_REVIEW));
}

#[test]
fn a_span_that_matches_only_after_normalising_is_not_verbatim() {
    let report = with_quotes("It says \"sufficient  appropriate\nevidence\" here.");
    assert_eq!(report.reviews.len(), 1, "{:?}", report.reviews);
    assert!(report.reviews[0].contains("only after normalising"));
}

#[test]
fn punctuation_ending_a_span_is_tried_without_it() {
    let report = with_quotes("It says \"sufficient appropriate evidence,\" and more.");
    assert_eq!(report.reviews, Vec::<String>::new());
    assert!(report.notes[0].starts_with("quotes check: 1 quoted span(s)"));
}

#[test]
fn an_ellipsis_is_looked_for_whole_and_asks_for_review() {
    let report = with_quotes("It says \"approved auditor ... the opinion\" here.");
    assert_eq!(report.reviews.len(), 1);
}

#[test]
fn a_single_quoted_word_and_an_apostrophe_are_not_quotations() {
    let report = with_quotes("The 'auditor' reviewed the client's file and the trustees' records.");
    assert_eq!(report.reviews, Vec::<String>::new());
    assert!(report.notes[0].starts_with("quotes check: 0 quoted span(s)"));
}

#[test]
fn an_apostrophe_inside_a_curly_quotation_does_not_end_it() {
    let mut report = gate::Report::default();
    check_quotes(
        "It says \u{2018}the auditor\u{2019}s own opinion\u{2019} here.",
        &["Part of the auditor\u{2019}s own opinion.".to_string()],
        &mut report,
    );
    assert_eq!(report.reviews, Vec::<String>::new());
    assert!(report.notes[0].starts_with("quotes check: 1 quoted span(s)"));
    // Ending at that apostrophe would leave `the auditor`, which is in the source.
    let mut report = gate::Report::default();
    check_quotes(
        "It says \u{2018}the auditor\u{2019}s waived opinion\u{2019} here.",
        &["Part of the auditor\u{2019}s own opinion.".to_string()],
        &mut report,
    );
    assert_eq!(report.reviews.len(), 1, "{:?}", report.reviews);
}

#[test]
fn a_stray_mark_before_whitespace_does_not_hide_the_quotation_after_it() {
    let report = with_quotes("A \" stray \"evidence may be waived\" here.");
    assert_eq!(report.reviews.len(), 1, "{:?}", report.reviews);
    assert!(report.reviews[0].contains("'evidence may be waived'"));
    assert_eq!(report.verdict(), ("REVIEW", EXIT_REVIEW));
}

#[test]
fn a_mark_straight_after_a_word_still_opens_a_quotation() {
    for draft in [
        "The ruling said\"fabricated legal basis\" here.",
        "The ruling said\u{201c}fabricated legal basis\u{201d} here.",
        "The ruling said\u{2018}fabricated legal basis\u{2019} here.",
    ] {
        let report = with_quotes(draft);
        assert_eq!(report.reviews.len(), 1, "{draft}: {:?}", report.reviews);
        assert!(report.reviews[0].contains("'fabricated legal basis'"));
        assert_eq!(report.verdict(), ("REVIEW", EXIT_REVIEW), "{draft}");
    }
}

#[test]
fn a_quotation_with_no_end_asks_for_review() {
    let report = with_quotes("It says \"sufficient appropriate evidence is needed.");
    assert_eq!(report.reviews.len(), 1, "{:?}", report.reviews);
    assert!(report.reviews[0].contains("could not find the end of the quotation"));
}

#[test]
fn a_second_source_can_hold_the_span() {
    let mut report = gate::Report::default();
    check_quotes(
        "It says \"second source wording\" here.",
        &[
            SOURCE.to_string(),
            "Some second source wording.".to_string(),
        ],
        &mut report,
    );
    assert_eq!(report.reviews, Vec::<String>::new());
    assert!(report.notes[0].starts_with("quotes check: 1 quoted span(s)"));
}

#[test]
fn the_source_option_reports_through_the_command_and_stops_on_a_bad_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = repo_root();
    let root = root.to_str().unwrap();
    let draft = format!("{DRAFT}\n- As stated, \"evidence may be waived\" here.\n");
    std::fs::write(dir.path().join("draft.md"), draft).unwrap();
    std::fs::write(dir.path().join("source.txt"), SOURCE).unwrap();
    let run = |extra: &[&str]| {
        let mut args = vec!["--root", root, "gate", "draft.md"];
        args.extend_from_slice(extra);
        run_in(dir.path(), &args, false)
    };
    let without = run(&[]);
    assert_eq!(without.code, i32::from(EXIT_PASS), "{}", without.stdout);
    let with = run(&["--source", "source.txt"]);
    assert_eq!(with.code, i32::from(EXIT_REVIEW), "{}", with.stdout);
    assert!(
        with.stdout.contains("not in any supplied source"),
        "{}",
        with.stdout
    );
    let missing = run(&["--source", "no-such-file.txt"]);
    assert_eq!(missing.code, i32::from(EXIT_COULD_NOT_RUN));
    assert!(
        missing.stderr.starts_with("gate: source file:"),
        "{}",
        missing.stderr
    );
}
