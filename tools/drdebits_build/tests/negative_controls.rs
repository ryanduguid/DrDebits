// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Deterministic 6-mutation negative-control self-check for `gate` and `bench score`.
//!
//! Starts from verified passing reference inputs, injects 6 controlled classes of
//! corruption (arithmetic perturbation, reversed sign, fabricated or swapped
//! citation, fact mismatch, wrong-route or unexecuted tool claim, and
//! self-contradiction) and asserts a 0% false-pass rate with zero API spend.

mod common;

use common::{py, repo_root};
use drdebits::bench::{self, Cited, Value};
use drdebits::builder;
use drdebits::gate::{
    self, EXIT_BLOCK, EXIT_PASS, EXIT_REVIEW, ErrorClass, check, check_authorities, check_facts,
    load_authorities, load_facts,
};
use drdebits::model::Conclusion;
use drdebits::pyjson::{self, Json};

const REFERENCE_DRAFT: &str = "\
DRAFT — PROFESSIONAL REVIEW REQUIRED

Primary decision status: PROCEED_DRAFT_ONLY
action_authority: NONE
state_changing_tool_action: NONE

Scope and period
- GST calculation for a taxable supply in the 2026-27 income year.

Source-version status
- Historical authority: CONFIRMED
- Current duties: CONFIRMED

Facts and evidence
- Invoice total: $1,100.
- Prior credit: $100.

Applicable authority
- s 9-5 of the A New Tax System (Goods and Services Tax) Act 1999.

Analysis
- GST component: $1,100 / 11 = $100.00.
- Balance after credit: $1,100 - $100 = $1,000.

Assumptions
- The supply is wholly taxable.

Human actions before use
- Confirm the tax invoice before lodging the BAS.

External-action record
- No external action taken.
";

const REFERENCE_FACTS: &str = r#"{"facts": {"invoice total": "$1,100", "prior credit": "$100"}}"#;
const REFERENCE_AUTHORITIES: &str = r#"{"authorities": [
  "s 9-5 of the A New Tax System (Goods and Services Tax) Act 1999"
]}"#;

fn conclusions() -> Vec<Conclusion> {
    let sources = builder::load_sources(&py(&repo_root())).unwrap();
    sources.whole_response.prohibited_conclusions
}

fn gate_with_all(draft: &str) -> gate::Report {
    let facts = load_facts(REFERENCE_FACTS).unwrap();
    let authorities = load_authorities(REFERENCE_AUTHORITIES).unwrap();
    let mut report = check(draft, &conclusions());
    check_facts(draft, &facts, &mut report);
    check_authorities(draft, &authorities, &mut report);
    report
}

fn passing_answers(topic: &str) -> (Vec<bench::Key>, Json) {
    let root = repo_root();
    let keys = bench::load_keys(&py(&root), topic).unwrap();
    let body = std::fs::read_to_string(root.join(format!("benchmarks/{topic}/keys.json"))).unwrap();
    let raw = pyjson::loads(&body).unwrap();
    let Some(Json::Array(rows)) = raw.get("keys") else {
        panic!("keys array missing for {topic}");
    };
    let entries: Vec<(String, Json)> = rows
        .iter()
        .map(|k| {
            (
                k.get("id").unwrap().as_str().unwrap().to_string(),
                Json::Object(vec![
                    ("value".into(), k.get("value").unwrap().clone()),
                    ("authorities".into(), k.get("authorities").unwrap().clone()),
                ]),
            )
        })
        .collect();
    (
        keys,
        Json::Object(vec![("answers".into(), Json::Object(entries))]),
    )
}

#[test]
fn gate_six_mutation_negative_controls_have_zero_false_passes() {
    let baseline = gate_with_all(REFERENCE_DRAFT);
    assert_eq!(
        baseline.verdict(),
        ("PASS", EXIT_PASS),
        "baseline failed: blocks={:?} reviews={:?}",
        baseline.blocks,
        baseline.reviews
    );
    assert!(baseline.error_classes.is_empty());

    let mutations: [(&str, String, u8, &[ErrorClass]); 6] = [
        // 1. Arithmetic perturbation ($0.01 error): operands match --facts, result wrong.
        (
            "arithmetic_perturbation_0_01",
            REFERENCE_DRAFT.replace("= $100.00.", "= $100.01."),
            EXIT_BLOCK,
            &[ErrorClass::ArithmeticError],
        ),
        // 2. Reversed sign (explicit minus and accounting parenthetical negative).
        (
            "reversed_sign",
            REFERENCE_DRAFT.replace("= $1,000.", "= -$1,000."),
            EXIT_BLOCK,
            &[ErrorClass::ArithmeticError],
        ),
        // 3. Fabricated / unlisted authority citation.
        (
            "fabricated_citation",
            REFERENCE_DRAFT.replace(
                "s 9-5 of the A New Tax System (Goods and Services Tax) Act 1999",
                "s 999-99 of the Income Tax Assessment Act 1997",
            ),
            EXIT_REVIEW,
            &[ErrorClass::RouteError],
        ),
        // 4. Fact mismatch: arithmetic is internally valid, operand contradicts --facts.
        (
            "fact_mismatch",
            REFERENCE_DRAFT
                .replace("Invoice total: $1,100.", "Invoice total: $1,200.")
                .replace("$1,100 / 11 = $100.00.", "$1,200 / 11 = $109.09.")
                .replace("$1,100 - $100 = $1,000.", "$1,200 - $100 = $1,100."),
            EXIT_REVIEW,
            &[ErrorClass::FactMismatch],
        ),
        // 5. Wrong-route / unexecuted tool claim + prohibited conclusion.
        (
            "wrong_route_and_tool_claim",
            REFERENCE_DRAFT
                .replace(
                    "state_changing_tool_action: NONE",
                    "state_changing_tool_action: LODGE_BAS",
                )
                .replace(
                    "Confirm the tax invoice before lodging the BAS.",
                    "The safe harbour applies to your position.",
                ),
            EXIT_REVIEW,
            &[ErrorClass::RouteError],
        ),
        // 6. Self-contradiction: conflicting primary decision statuses and chained sum.
        (
            "self_contradiction",
            REFERENCE_DRAFT
                .replace(
                    "Primary decision status: PROCEED_DRAFT_ONLY",
                    "Primary decision status: PROCEED_DRAFT_ONLY\nPrimary decision status: HARD_STOP",
                )
                .replace("= $100.00.", "= $100.00 = $105.00."),
            EXIT_BLOCK,
            &[ErrorClass::RouteError, ErrorClass::ArithmeticError],
        ),
    ];

    let mut false_passes = 0usize;
    for (name, mutated, expected_code, expected_classes) in &mutations {
        let report = gate_with_all(mutated);
        let (_, code) = report.verdict();
        if code == EXIT_PASS {
            false_passes += 1;
        }
        assert_eq!(
            code, *expected_code,
            "{name}: blocks={:?} reviews={:?}",
            report.blocks, report.reviews
        );
        assert_eq!(report.error_classes.as_slice(), *expected_classes, "{name}");
    }
    assert_eq!(
        false_passes, 0,
        "negative controls must have 0% false-pass rate"
    );

    // Also verify parenthetical sign flip and unauthorised action_authority never pass.
    for (name, mutated) in [
        (
            "parenthetical_sign_flip",
            REFERENCE_DRAFT.replace("= $1,000.", "= ($1,000)."),
        ),
        (
            "unauthorised_action_authority",
            REFERENCE_DRAFT.replace("action_authority: NONE", "action_authority: EXECUTE"),
        ),
        (
            "missing_statutory_section",
            REFERENCE_DRAFT.replace("Assumptions\n- The supply is wholly taxable.\n", ""),
        ),
    ] {
        let report = gate_with_all(&mutated);
        assert_eq!(report.verdict().1, EXIT_BLOCK, "{name}");
    }
}

#[test]
fn bench_score_six_mutation_negative_controls_have_zero_false_passes() {
    let corpus = load_authorities(
        r#"{"authorities": [
            "s 6 of the Superannuation Guarantee (Administration) Act 1992",
            "s 18C of the Superannuation Guarantee (Administration) Act 1992",
            "s 109E of the Income Tax Assessment Act 1936",
            "s 109N of the Income Tax Assessment Act 1936",
            "s 15-10 of the Taxation Administration Act 1953",
            "s 15-25 of the Taxation Administration Act 1953",
            "s 6 of the Medicare Levy Act 1986",
            "s 7 of the Medicare Levy Act 1986",
            "s 8 of the Medicare Levy Act 1986"
        ]}"#,
    )
    .unwrap();

    // Baseline reference answers pass 100% with 0% fabricated citation rate.
    for topic in ["payday-div7a", "payg-medicare"] {
        let (keys, answers) = passing_answers(topic);
        let scored = bench::score(&keys, &answers).unwrap();
        assert!(
            scored
                .iter()
                .all(|s| s.value == Value::Correct && s.cited == Cited::Right),
            "{topic} baseline"
        );
        let rate = bench::fabricated_citation_rate(&scored, &corpus);
        assert_eq!(rate.unlisted, 0, "{topic}");
        assert_eq!(rate.rate, Some(0.0), "{topic}");
    }

    let (keys, _) = passing_answers("payday-div7a");
    let check_case = |json: &str, id: &str| {
        let scored = bench::score(&keys, &pyjson::loads(json).unwrap()).unwrap();
        let item = scored.iter().find(|s| s.id == id).unwrap();
        (
            item.value,
            item.cited,
            bench::fabricated_citation_rate(&scored, &corpus),
        )
    };

    // 1. Arithmetic perturbation (+$0.01 on D7A-007: 19715.97 -> 19715.98).
    let (v, c, _) = check_case(
        r#"{"answers": {"D7A-007": {"value": "19715.98", "authorities": ["s 109E of the Income Tax Assessment Act 1936"]}}}"#,
        "D7A-007",
    );
    assert_eq!((v, c), (Value::Wrong, Cited::Right));

    // 2. Reversed sign (explicit minus and accounting parenthetical negatives).
    for flipped in ["-19715.97", "(19,715.97)", "($19,715.97)", "$(19,715.97)"] {
        let payload = format!(
            r#"{{"answers": {{"D7A-007": {{"value": "{flipped}", "authorities": ["s 109E of the Income Tax Assessment Act 1936"]}}}}}}"#
        );
        let (v, _, _) = check_case(&payload, "D7A-007");
        assert_eq!(v, Value::Wrong, "flipped sign {flipped} must not pass");
    }

    // 3. Fabricated citation (not in corpus) vs swapped real citation (in corpus, wrong for question).
    let (v, c, rate) = check_case(
        r#"{"answers": {"D7A-001": {"value": "8.77", "authorities": ["s 999Z of the Income Tax Assessment Act 1936"]}}}"#,
        "D7A-001",
    );
    assert_eq!((v, c), (Value::Correct, Cited::Wrong));
    assert_eq!((rate.unlisted, rate.checked), (1, 1));
    assert_eq!(rate.rate, Some(1.0));
    assert!(rate.ci95_low.unwrap() > 0.0);

    // 4. Swapped authority section (real section, wrong rule route for D7A-001).
    let (v, c, rate) = check_case(
        r#"{"answers": {"D7A-001": {"value": "8.77", "authorities": ["s 109E of the Income Tax Assessment Act 1936"]}}}"#,
        "D7A-001",
    );
    assert_eq!((v, c), (Value::Correct, Cited::Wrong));
    assert_eq!(
        rate.unlisted, 0,
        "real wrong section is not mislabelled fabricated"
    );

    // 5. Wrong-route date answer (off-by-1 day on Payday Super due date).
    let (v, c, _) = check_case(
        r#"{"answers": {"PDS-002": {"value": "2026-08-10", "authorities": ["s 6 of the Superannuation Guarantee (Administration) Act 1992"]}}}"#,
        "PDS-002",
    );
    assert_eq!((v, c), (Value::Wrong, Cited::Right));

    // 6. Self-contradiction (citing both right and wrong sections simultaneously).
    let (v, c, _) = check_case(
        r#"{"answers": {"D7A-001": {"value": "8.77", "authorities": [
            "s 109N of the Income Tax Assessment Act 1936",
            "s 109E of the Income Tax Assessment Act 1936"
        ]}}}"#,
        "D7A-001",
    );
    assert_eq!((v, c), (Value::Correct, Cited::Mixed));
}
