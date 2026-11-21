// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! The published benchmark's keys, and a scorer that keeps a wrong citation
//! apart from a wrong figure.

mod common;

use std::path::Path;

use common::{py, repo_root, run_in};
use drdebits::bench::{self, Cited, Scored, Value};
use drdebits::pyjson::{self, Json};

const TOPIC: &str = "payday-div7a";
const DIR: &str = "benchmarks/payday-div7a";
const FILES: [&str; 4] = [
    "questions.json",
    "keys.json",
    "business-days.json",
    "engine-check.json",
];

/// A writable copy of the benchmark, for tests that break it.
fn copy() -> tempfile::TempDir {
    let tree = tempfile::tempdir().unwrap();
    let target = tree.path().join(DIR);
    std::fs::create_dir_all(&target).unwrap();
    for file in FILES {
        std::fs::copy(repo_root().join(DIR).join(file), target.join(file)).unwrap();
    }
    tree
}

fn edit(tree: &Path, file: &str, from: &str, to: &str) {
    let path = tree.join(DIR).join(file);
    let body = std::fs::read_to_string(&path).unwrap();
    assert!(body.contains(from), "{from}");
    std::fs::write(path, body.replacen(from, to, 1)).unwrap();
}

fn keys() -> Vec<bench::Key> {
    bench::load_keys(&py(&repo_root()), TOPIC).unwrap()
}

fn mark(answers: &str) -> Result<Vec<Scored>, String> {
    bench::score(&keys(), &pyjson::loads(answers).unwrap())
}

fn one(answers: &str, id: &str) -> (Value, Cited) {
    let scored = mark(answers).unwrap();
    let found = scored.iter().find(|s| s.id == id).unwrap();
    (found.value, found.cited)
}

#[test]
fn every_published_key_reproduces_and_agrees_with_the_engine_record() {
    let report = bench::check_keys(&py(&repo_root()), TOPIC).unwrap();
    assert_eq!(report.problems, Vec::<String>::new());
    assert_eq!((report.computed, report.not_recomputed), (21, 10));
    assert_eq!(report.engine_checked, 31);
}

#[test]
fn a_key_changed_by_one_day_is_caught_by_the_rule_and_the_engine_record() {
    let tree = copy();
    edit(
        tree.path(),
        "keys.json",
        "\"value\": \"2026-08-11\"",
        "\"value\": \"2026-08-10\"",
    );
    let report = bench::check_keys(&py(tree.path()), TOPIC).unwrap();
    assert_eq!(report.problems.len(), 2, "{:?}", report.problems);
    assert!(report.problems[0].starts_with("PDS-002: the key says"));
}

#[test]
fn dropping_picnic_day_from_the_calendar_moves_the_dates_that_cross_it() {
    let tree = copy();
    edit(
        tree.path(),
        "business-days.json",
        "2026-08-03",
        "2026-08-02",
    );
    let report = bench::check_keys(&py(tree.path()), TOPIC).unwrap();
    let moved: Vec<&str> = report.problems.iter().map(|p| &p[..7]).collect();
    assert_eq!(moved, ["PDS-001", "PDS-002", "PDS-007"]);
}

#[test]
fn a_key_with_no_engine_record_is_a_problem_not_a_pass() {
    let tree = copy();
    edit(
        tree.path(),
        "engine-check.json",
        "\"id\": \"D7A-016\"",
        "\"id\": \"D7A-999\"",
    );
    let report = bench::check_keys(&py(tree.path()), TOPIC).unwrap();
    assert_eq!(
        report.problems,
        ["D7A-016: no engine output is recorded".to_string()]
    );
}

#[test]
fn a_rule_that_runs_past_the_calendar_stops_the_check() {
    let tree = copy();
    edit(
        tree.path(),
        "business-days.json",
        "\"complete_until\": \"2027-08-31\"",
        "\"complete_until\": \"2026-07-31\"",
    );
    let error = bench::check_keys(&py(tree.path()), TOPIC).unwrap_err();
    assert!(error.contains("past the calendar"), "{error}");
}

#[test]
fn keys_that_do_not_answer_the_questions_one_for_one_are_refused() {
    let tree = copy();
    edit(tree.path(), "questions.json", "PDS-001", "PDS-000");
    let error = bench::load_keys(&py(tree.path()), TOPIC).unwrap_err();
    assert!(error.contains("do not match the questions"), "{error}");
}

#[test]
fn matching_duplicate_ids_are_refused() {
    let tree = copy();
    for file in ["questions.json", "keys.json"] {
        edit(
            tree.path(),
            file,
            "\"id\": \"PDS-002\"",
            "\"id\": \"PDS-001\"",
        );
    }
    assert_eq!(
        bench::load_keys(&py(tree.path()), TOPIC).unwrap_err(),
        "a question id appears twice"
    );
}

#[test]
fn malformed_keys_are_refused_before_scoring() {
    for (from, to, expected) in [
        (
            "\"keys\":",
            "\"entries\":",
            "keys must be a list with entries",
        ),
        (
            "\"kind\": \"text\"",
            "\"kind\": \"date\"",
            "unknown kind date",
        ),
        (
            "\"kind\": \"text\"",
            "\"kind\": \"number\"",
            "is not a number",
        ),
        (
            "\"rule\": \"business-days\"",
            "\"rule\": \"guess\"",
            "unknown rule guess",
        ),
    ] {
        let tree = copy();
        edit(tree.path(), "keys.json", from, to);
        let error = bench::load_keys(&py(tree.path()), TOPIC).unwrap_err();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn a_changed_canary_is_refused() {
    let tree = copy();
    edit(tree.path(), "keys.json", "DRDEBITS-BENCHMARK", "BENCHMARK");
    let error = bench::load_keys(&py(tree.path()), TOPIC).unwrap_err();
    assert!(error.contains("benchmark_canary"), "{error}");
}

#[test]
fn a_key_needs_its_source_and_derivation() {
    let tree = copy();
    edit(tree.path(), "keys.json", "\"derivation\"", "\"working\"");
    let error = bench::load_keys(&py(tree.path()), TOPIC).unwrap_err();
    assert_eq!(error, "keys file PDS-001: derivation is missing");
}

#[test]
fn the_keys_answer_their_own_questions_with_the_right_authority() {
    let body = std::fs::read_to_string(repo_root().join(DIR).join("keys.json")).unwrap();
    let keys = pyjson::loads(&body).unwrap();
    let Some(Json::Array(rows)) = keys.get("keys") else {
        panic!()
    };
    let answers: Vec<(String, Json)> = rows
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
    let payload = Json::Object(vec![("answers".into(), Json::Object(answers))]);
    let scored = bench::score(&self::keys(), &payload).unwrap();
    assert_eq!(scored.len(), 31);
    assert!(
        scored
            .iter()
            .all(|s| s.value == Value::Correct && s.cited == Cited::Right)
    );
}

#[test]
fn a_right_figure_on_the_wrong_section_counts_as_wrong_authority() {
    let answers = r#"{"answers": {"D7A-001": {"value": "8.77",
        "authorities": ["s 109E of the Income Tax Assessment Act 1936"]}}}"#;
    assert_eq!(one(answers, "D7A-001"), (Value::Correct, Cited::Wrong));
}

#[test]
fn the_same_section_number_in_another_act_is_wrong_authority() {
    let answers = r#"{"answers": {"D7A-001": {"value": "8.77",
        "authorities": ["s 109N of the Income Tax Assessment Act 1997"]}}}"#;
    assert_eq!(one(answers, "D7A-001"), (Value::Correct, Cited::Wrong));
}

#[test]
fn a_subsection_of_the_expected_section_is_the_right_authority() {
    let answers = r#"{"answers": {"D7A-001": {"value": "8.77%",
        "authorities": ["s 109N(2) of the Income Tax Assessment Act 1936"]}}}"#;
    assert_eq!(one(answers, "D7A-001"), (Value::Correct, Cited::Right));
}

#[test]
fn a_right_section_beside_a_wrong_one_is_mixed_not_right() {
    let answers = r#"{"answers": {"PDS-002": {"value": "2026-08-11", "authorities": [
        "s 8-1 of the Income Tax Assessment Act 1997",
        "s 6(1) of the Superannuation Guarantee (Administration) Act 1992"]}}}"#;
    assert_eq!(one(answers, "PDS-002"), (Value::Correct, Cited::Mixed));
}

/// No combination that holds anything but listed, readable sections earns Right.
#[test]
fn adversarial_citation_combinations_do_not_earn_right() {
    const RIGHT: &str = "s 109N(2) of the Income Tax Assessment Act 1936";
    const WRONG: &str = "s 109E of the Income Tax Assessment Act 1936";
    const OTHER_ACT: &str = "s 109N of the Income Tax Assessment Act 1997";
    const UNREADABLE: &str = "Division 7A of ITAA 1936";
    const TWO_IN_ONE: &str = "s 109N and s 109E of the Income Tax Assessment Act 1936";
    for (cited, mark) in [
        (&[RIGHT, UNREADABLE][..], Cited::PartlyUnreadable),
        (&[UNREADABLE, RIGHT], Cited::PartlyUnreadable),
        (&[RIGHT, TWO_IN_ONE], Cited::PartlyUnreadable),
        (&[RIGHT, WRONG], Cited::Mixed),
        (&[RIGHT, OTHER_ACT], Cited::Mixed),
        (&[RIGHT, WRONG, UNREADABLE], Cited::Mixed),
        (&[WRONG, UNREADABLE], Cited::Wrong),
        (&[OTHER_ACT], Cited::Wrong),
        (&[TWO_IN_ONE], Cited::Unreadable),
        (&[RIGHT, RIGHT], Cited::Right),
    ] {
        let list = cited
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let answers = format!(
            r#"{{"answers": {{"D7A-001": {{"value": "8.77", "authorities": [{list}]}}}}}}"#
        );
        assert_eq!(
            one(&answers, "D7A-001"),
            (Value::Correct, mark),
            "{cited:?}"
        );
    }
}

/// Each listed section is enough alone, and so are the 2 together.
#[test]
fn either_listed_payday_section_alone_is_right() {
    const SIX: &str = "s 6(1) of the Superannuation Guarantee (Administration) Act 1992";
    const EIGHTEEN_C: &str = "s 18C(2) of the Superannuation Guarantee (Administration) Act 1992";
    for cited in [
        format!("\"{SIX}\""),
        format!("\"{EIGHTEEN_C}\""),
        format!("\"{SIX}\", \"{EIGHTEEN_C}\""),
    ] {
        let answers = format!(
            r#"{{"answers": {{"PDS-001": {{"value": "2026-08-07", "authorities": [{cited}]}}}}}}"#
        );
        assert_eq!(one(&answers, "PDS-001"), (Value::Correct, Cited::Right));
    }
}

#[test]
fn the_keys_must_state_how_several_authorities_work() {
    let tree = copy();
    edit(
        tree.path(),
        "keys.json",
        "is enough alone",
        "is required jointly",
    );
    let error = bench::load_keys(&py(tree.path()), TOPIC).unwrap_err();
    assert!(
        error.contains("authority_rule is missing or changed"),
        "{error}"
    );
}

/// The value comes from the recorded output, so changing the output alone is
/// caught, and the value beside it can neither hide nor contradict that.
#[test]
fn a_changed_output_alone_is_caught() {
    for (from, to, id) in [
        (
            "\"due\": \"2026-08-07\"",
            "\"due\": \"2026-08-08\"",
            "PDS-001",
        ),
        (
            "\"benchmark_rate\": \"0.0877\"",
            "\"benchmark_rate\": \"0.0878\"",
            "D7A-001",
        ),
        (
            "\"output\": \"9834.92\"",
            "\"output\": \"9835.92\"",
            "D7A-005",
        ),
        (
            "\"output\": \"19715.97\"",
            "\"output\": \"19715.98\"",
            "D7A-007",
        ),
        (
            "\"maximum_term_years_allowed\": \"7\"",
            "\"maximum_term_years_allowed\": \"8\"",
            "D7A-011",
        ),
    ] {
        let tree = copy();
        edit(tree.path(), "engine-check.json", from, to);
        let report = bench::check_keys(&py(tree.path()), TOPIC).unwrap();
        assert_eq!(report.problems.len(), 2, "{id}: {:?}", report.problems);
        assert!(report.problems[0].contains("its output gives"), "{id}");
        assert!(report.problems[0].starts_with(id), "{:?}", report.problems);
    }
}

#[test]
fn a_changed_stated_value_alone_conflicts_with_its_output() {
    let tree = copy();
    edit(
        tree.path(),
        "engine-check.json",
        "\"value\": \"2026-08-07\"",
        "\"value\": \"2026-08-08\"",
    );
    let report = bench::check_keys(&py(tree.path()), TOPIC).unwrap();
    assert_eq!(
        report.problems,
        ["PDS-001: the record states 2026-08-08 but its output gives 2026-08-07".to_string()]
    );
    assert_eq!(report.engine_checked, 31);
}

#[test]
fn a_duplicate_engine_record_is_refused() {
    let tree = copy();
    edit(
        tree.path(),
        "engine-check.json",
        "\"id\": \"PDS-002\"",
        "\"id\": \"PDS-001\"",
    );
    let error = bench::check_keys(&py(tree.path()), TOPIC).unwrap_err();
    assert!(error.ends_with("PDS-001 is recorded twice"), "{error}");
}

#[test]
fn an_engine_record_that_does_not_refuse_the_loan_year_is_refused() {
    let tree = copy();
    edit(
        tree.path(),
        "engine-check.json",
        "\"REFUSED_YEAR_IS_YEAR_OF_LOAN\"",
        "\"REFUSED_GATE_UNKNOWN\"",
    );
    let error = bench::check_keys(&py(tree.path()), TOPIC).unwrap_err();
    assert!(
        error.contains("does not refuse the year of the loan"),
        "{error}"
    );
}

#[test]
fn an_engine_record_with_a_bad_read_is_refused() {
    for (from, to, expected) in [
        (
            "\"form\": \"percent\"",
            "\"form\": \"guess\"",
            "unknown read form guess",
        ),
        (
            "\"benchmark_rate\"\n",
            "\"no_such_field\"\n",
            "nothing at the read path",
        ),
        (
            "\"path\": [\n     \"due\"\n    ]",
            "\"path\": \"due\"",
            "read path must be a list",
        ),
        ("\"benchmark_rate\"\n", "0\n", "nothing at the read path"),
        (
            "\"form\": \"whole-dollars\"",
            "\"form\": \"percent\"",
            "D7A-005",
        ),
    ] {
        let tree = copy();
        edit(tree.path(), "engine-check.json", from, to);
        let found = bench::check_keys(&py(tree.path()), TOPIC);
        let text = match found {
            Ok(report) => report.problems.join("; "),
            Err(error) => error,
        };
        assert!(text.contains(expected), "{expected}: {text}");
    }
}

/// Citing all 4 sections on every question must not buy the headline count.
#[test]
fn citing_every_section_on_every_question_earns_no_right_authority() {
    const SECTIONS: [&str; 4] = [
        "s 6 of the Superannuation Guarantee (Administration) Act 1992",
        "s 18C of the Superannuation Guarantee (Administration) Act 1992",
        "s 109E of the Income Tax Assessment Act 1936",
        "s 109N of the Income Tax Assessment Act 1936",
    ];
    let cited = Json::Array(SECTIONS.iter().map(|s| Json::str_obj(s)).collect());
    let answers: Vec<(String, Json)> = keys()
        .iter()
        .map(|k| {
            (
                k.id.clone(),
                Json::Object(vec![
                    ("value".into(), Json::str_obj(&k.value)),
                    ("authorities".into(), cited.clone()),
                ]),
            )
        })
        .collect();
    let payload = Json::Object(vec![("answers".into(), Json::Object(answers))]);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("answers.json");
    std::fs::write(&path, pyjson::dumps(&payload, false)).unwrap();
    let out = run_in(
        &repo_root(),
        &["bench", "score", path.to_str().unwrap()],
        true,
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    for line in [
        "value correct: 31",
        "authority right: 0",
        "authority mixed: 31",
        "authority partly unreadable: 0",
        "wrong authority: 0",
        "correct value right authority: 0",
    ] {
        assert!(out.stdout.contains(line), "{line}: {}", out.stdout);
    }
}

/// s 6(1) holds the usual and extended usual periods, so citing it is right on
/// every date question that turns on a period.
#[test]
fn a_correct_s_6_citation_is_right_on_every_payday_question() {
    for key in keys().iter().filter(|k| k.id.starts_with("PDS-")) {
        let answers = format!(
            r#"{{"answers": {{"{}": {{"value": "{}", "authorities":
            ["s 6(1) of the Superannuation Guarantee (Administration) Act 1992"]}}}}}}"#,
            key.id, key.value
        );
        assert_eq!(one(&answers, &key.id), (Value::Correct, Cited::Right));
    }
}

#[test]
fn a_blank_value_is_unanswered_not_wrong() {
    let answers = r#"{"answers": {"D7A-001": {"value": "  ",
        "authorities": ["s 109N of the Income Tax Assessment Act 1936"]}}}"#;
    assert_eq!(one(answers, "D7A-001"), (Value::Unanswered, Cited::Right));
}

/// D7A-007 and D7A-015 share a loan: a payment before lodgment day would change
/// its P, and a term answer must be digits for the mark to mean anything.
#[test]
fn the_questions_fix_when_the_payments_came_and_how_a_term_is_written() {
    let body = std::fs::read_to_string(repo_root().join(DIR).join("questions.json")).unwrap();
    let questions = pyjson::loads(&body).unwrap();
    let Some(Json::Array(rows)) = questions.get("questions") else {
        panic!()
    };
    let field = |id: &str, name: &str| {
        rows.iter()
            .find(|q| q.get("id").and_then(Json::as_str) == Some(id))
            .and_then(|q| q.get(name))
            .and_then(Json::as_str)
            .unwrap()
            .to_string()
    };
    assert!(!field("D7A-007", "question").contains("at any time since"));
    assert!(field("D7A-015", "question").contains("all after the company's lodgment day"));
    for id in ["D7A-011", "D7A-012", "D7A-013"] {
        assert_eq!(field(id, "answer_format"), "Digits only, such as 12.");
    }
}

#[test]
fn a_citation_the_gate_cannot_read_is_unreadable_not_wrong() {
    let answers = r#"{"answers": {"D7A-001": {"value": "8.77",
        "authorities": ["Division 7A of ITAA 1936"]}}}"#;
    assert_eq!(one(answers, "D7A-001"), (Value::Correct, Cited::Unreadable));
}

#[test]
fn a_wrong_figure_stays_wrong_whatever_it_cites() {
    let answers = r#"{"answers": {"D7A-002": {"value": "8.27",
        "authorities": ["s 109N of the Income Tax Assessment Act 1936"]}}}"#;
    assert_eq!(one(answers, "D7A-002"), (Value::Wrong, Cited::Right));
}

#[test]
fn money_compares_by_value_and_an_answer_without_a_citation_is_marked_missing() {
    let answers = r#"{"answers": {"D7A-007": {"value": "$19,715.97"}}}"#;
    assert_eq!(one(answers, "D7A-007"), (Value::Correct, Cited::Missing));
    let answers = r#"{"answers": {"D7A-007": {"value": "19,716"}}}"#;
    assert_eq!(one(answers, "D7A-007"), (Value::Wrong, Cited::Missing));
    let answers = r#"{"answers": {"D7A-007": {"value": "about 20000"}}}"#;
    assert_eq!(one(answers, "D7A-007"), (Value::Wrong, Cited::Missing));
}

#[test]
fn a_token_compares_ignoring_case() {
    let answers = r#"{"answers": {"PDS-014": {"value": "late"}}}"#;
    assert_eq!(one(answers, "PDS-014"), (Value::Correct, Cited::Missing));
}

#[test]
fn a_question_left_out_is_unanswered() {
    assert_eq!(
        one(r#"{"answers": {}}"#, "PDS-001"),
        (Value::Unanswered, Cited::Missing)
    );
}

#[test]
fn a_malformed_answers_file_is_refused_rather_than_scored() {
    for (answers, expected) in [
        ("[]", "the 'answers' key must hold an object"),
        (
            r#"{"answers": []}"#,
            "the 'answers' key must hold an object",
        ),
        (
            r#"{"replies": {}}"#,
            "the 'answers' key must hold an object",
        ),
        (r#"{"answers": {}, "model": "x"}"#, "unknown field model"),
        (
            r#"{"answers": {"PDS-999": {"value": "x"}}}"#,
            "unknown question PDS-999",
        ),
        (
            r#"{"answers": {"PDS-001": "2026-08-07"}}"#,
            "must be an object",
        ),
        (
            r#"{"answers": {"PDS-001": {"value": "x", "reason": "y"}}}"#,
            "unknown field reason",
        ),
        (r#"{"answers": {"PDS-001": {}}}"#, "value must be text"),
        (
            r#"{"answers": {"PDS-001": {"value": "x", "authorities": "s 18C"}}}"#,
            "authorities must be a list",
        ),
        (
            r#"{"answers": {"PDS-001": {"value": "x", "authorities": [1]}}}"#,
            "authorities must be text",
        ),
    ] {
        let error = mark(answers).unwrap_err();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn the_commands_report_the_counts_and_exit_as_documented() {
    let root = repo_root();
    let keys = run_in(&root, &["bench", "keys"], true);
    assert_eq!(keys.code, 0);
    assert!(
        keys.stdout
            .contains("10 source-derived keys not independently recomputed")
    );
    assert!(keys.stdout.contains("31 agree with the recorded engine"));

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("answers.json");
    std::fs::write(
        &path,
        r#"{"answers": {"D7A-001": {"value": "8.77",
        "authorities": ["s 109E of the Income Tax Assessment Act 1936"]}}}"#,
    )
    .unwrap();
    let path = path.to_str().unwrap();
    let scored = run_in(&root, &["bench", "score", path], true);
    assert_eq!(scored.code, 0);
    let text = &scored.stdout;
    assert!(text.contains("wrong authority: 1"), "{text}");
    assert!(text.contains("correct value wrong authority: 1"), "{text}");
    assert!(text.contains("unanswered: 30"), "{text}");

    let json = run_in(&root, &["bench", "score", path, "--json"], true);
    let payload = pyjson::loads(&json.stdout).unwrap();
    assert_eq!(
        payload.get("totals").and_then(|t| t.get("wrong_authority")),
        Some(&Json::Int(1.into()))
    );

    let missing = run_in(&root, &["bench", "score", "no-such-file.json"], true);
    assert_eq!(missing.code, 2);
    assert!(missing.stderr.starts_with("bench: answers file:"));
}

#[test]
fn a_mismatched_key_exits_1_and_says_so() {
    let tree = copy();
    edit(
        tree.path(),
        "keys.json",
        "\"value\": \"2026-08-11\"",
        "\"value\": \"2026-08-10\"",
    );
    let output = run_in(tree.path(), &["bench", "keys"], true);
    assert_eq!(output.code, 1);
    assert!(output.stderr.contains("bench keys: PDS-002"));
}

/// The publisher's conflict and the limit of the claim stay on the page, and
/// the source table adds up to the keys.
#[test]
fn the_readme_states_the_conflict_and_counts_each_source() {
    let text = std::fs::read_to_string(repo_root().join(DIR).join("README.md")).unwrap();
    assert!(text.contains("**Publisher conflict.**"));
    assert!(text.contains("not that the benchmark is independent"));
    assert!(text.contains("is copied from one of them"));
    assert!(text.contains("copied from the `payday-super-checker` table"));
    assert_eq!(text.matches("15 August 2026").count(), 2);
    assert!(text.contains("agreement shows the arithmetic only"));
    assert!(text.contains("10 are source-derived keys not independently recomputed"));
    assert!(text.contains("either s 6 or s 18C counts as right"));
    assert!(!text.contains("read from a source"));
    assert!(text.contains("King's Birthday on 28 September 2026"));
    let front = std::fs::read_to_string(repo_root().join("README.md")).unwrap();
    assert!(!front.contains("not a benchmark"));
    let changelog = std::fs::read_to_string(repo_root().join("CHANGELOG.md")).unwrap();
    assert!(changelog.contains("`bench keys` recomputes 21 keys offline"));
    assert!(!changelog.contains("`bench keys` reproduces"));
    let keys = std::fs::read_to_string(repo_root().join(DIR).join("keys.json")).unwrap();
    let keys = pyjson::loads(&keys).unwrap();
    let Some(Json::Array(keys)) = keys.get("keys") else {
        panic!()
    };
    for (row, kind) in [
        ("A worked example on an ATO page", "ATO example"),
        ("The ATO's benchmark rate table", "ATO table"),
        (
            "The ATO's Division 7A calculator, as recorded on 7 October 2026",
            "ATO tool",
        ),
        ("The statute, worked in the derivation", "statute"),
    ] {
        let count = keys
            .iter()
            .filter(|k| {
                k.get("source")
                    .and_then(|s| s.get("kind"))
                    .and_then(Json::as_str)
                    == Some(kind)
            })
            .count();
        assert!(
            text.contains(&format!("| {row} | {count} |")),
            "{row}: {count}"
        );
    }
}

/// The README says this holiday is avoided, and the calendar does not list it,
/// so no date key may span it.
#[test]
fn no_date_key_spans_the_holiday_the_calendar_leaves_out() {
    const LEFT_OUT: &str = "2026-09-28";
    let body = std::fs::read_to_string(repo_root().join(DIR).join("keys.json")).unwrap();
    let keys = pyjson::loads(&body).unwrap();
    let Some(Json::Array(rows)) = keys.get("keys") else {
        panic!()
    };
    for key in rows {
        let Some(Json::Array(terms)) = key.get("check").and_then(|c| c.get("terms")) else {
            continue;
        };
        let due = key.get("value").and_then(Json::as_str).unwrap();
        for term in terms {
            let from = term.get("from").and_then(Json::as_str).unwrap();
            assert!(!(from <= LEFT_OUT && LEFT_OUT <= due), "{from} to {due}");
        }
    }
}

#[test]
fn a_rule_that_starts_before_the_calendar_stops_the_check() {
    let tree = copy();
    edit(
        tree.path(),
        "business-days.json",
        "\"complete_from\": \"2026-07-01\"",
        "\"complete_from\": \"2026-07-02\"",
    );
    let error = bench::check_keys(&py(tree.path()), TOPIC).unwrap_err();
    assert!(
        error.contains("is before the calendar's first complete day"),
        "{error}"
    );
}

/// The README's table of naive baselines has to be what the scorer gives, and
/// its PDS-002 sentence has to quote what each file answers.
#[test]
fn the_published_naive_baselines_score_as_the_readme_says() {
    let readme = std::fs::read_to_string(repo_root().join(DIR).join("README.md")).unwrap();
    for (file, pds_002, row) in [
        ("calendar-days.json", "2026-08-06", "| 0 | 14 | 17 | 14 |"),
        ("weekdays-only.json", "2026-08-10", "| 7 | 7 | 17 | 14 |"),
    ] {
        let path = repo_root().join(DIR).join("baselines").join(file);
        let body = std::fs::read_to_string(path).unwrap();
        let scored = mark(&body).unwrap();
        let count = |hit: &dyn Fn(&Scored) -> bool| scored.iter().filter(|s| hit(s)).count();
        let shown = format!(
            "| {} | {} | {} | {} |",
            count(&|s| s.value == Value::Correct),
            count(&|s| s.value == Value::Wrong),
            count(&|s| s.value == Value::Unanswered),
            count(&|s| s.cited == Cited::Right),
        );
        assert_eq!(shown, row, "{file}");
        let line = format!("[{file}](baselines/{file}) {row}");
        assert!(readme.contains(&line), "README row for {file}: {line}");
        let answers = pyjson::loads(&body).unwrap();
        let answered = answers
            .get("answers")
            .and_then(|a| a.get("PDS-002"))
            .and_then(|a| a.get("value"))
            .and_then(Json::as_str);
        assert_eq!(answered, Some(pds_002), "{file} PDS-002");
        assert!(readme.contains(pds_002), "README PDS-002 answer {pds_002}");
    }
}

/// Worked by hand from the binomial distribution with p = 1/2, ties left out.
/// 6 to 1: 2 × (C(7,0) + C(7,1)) / 2^7 = 2 × (1 + 7) / 128 = 0.125.
/// 0 to 7: 2 × C(7,0) / 2^7 = 2 / 128 = 0.015625, shown as 0.0156.
/// 10 to 0: 2 × C(10,0) / 2^10 = 2 / 1,024 = 0.001953125, shown as 0.0020.
/// 3 to 3: 2 × (1 + 6 + 15 + 20) / 2^6 = 84 / 64, more than 1, so 1.
/// 0 to 0: no untied pair, so 1.
#[test]
fn the_sign_test_matches_the_binomial_tail_worked_by_hand() {
    for ((a, b), p) in [
        ((6, 1), "p = 0.1250"),
        ((1, 6), "p = 0.1250"),
        ((0, 7), "p = 0.0156"),
        ((10, 0), "p = 0.0020"),
        ((3, 3), "p = 1.0000"),
        ((0, 0), "p = 1.0000"),
    ] {
        assert_eq!(bench::sign_test(a, b), p, "{a} to {b}");
    }
}

/// A p that would show as 0.0000 is never printed as 0. 15 to 0:
/// 2 / 2^15 = 0.000061, shown as 0.0001; 16 to 0: 2 / 2^16 = 0.0000305,
/// below what 4 places show. 5 to 0 is the most uneven split of 5 and still
/// gives 2 / 2^5 = 0.0625; 6 to 0 gives 0.03125, shown as 0.0313.
#[test]
fn a_tiny_p_is_shown_as_below_the_last_place_not_as_0() {
    for ((a, b), p) in [
        ((15, 0), "p = 0.0001"),
        ((16, 0), "p < 0.0001"),
        ((0, 31), "p < 0.0001"),
        ((5, 0), "p = 0.0625"),
        ((6, 0), "p = 0.0313"),
    ] {
        assert_eq!(bench::sign_test(a, b), p, "{a} to {b}");
    }
}

/// An answers file giving the key's value, with no citation, for each
/// question at the given positions in the key list.
fn answering(keys: &[bench::Key], positions: std::ops::Range<usize>) -> Json {
    let answers: Vec<String> = keys[positions]
        .iter()
        .map(|k| format!("\"{}\": {{\"value\": \"{}\"}}", k.id, k.value))
        .collect();
    pyjson::loads(&format!("{{\"answers\": {{{}}}}}", answers.join(", "))).unwrap()
}

/// Condition a answers questions 1 to 6 right in both repeats. Condition b
/// answers questions 1 to 3 right in its first repeat and question 7 right in
/// its second. So a has more right answers on 6 questions (1 to 6), b on 1
/// (7), and the other 24 are tied at none: p = 0.1250 as worked above.
/// Unanswered runs the other way, 1 to 6, with the same p. No answer is
/// wrong and none cites a section, so those measures tie on all 31.
#[test]
fn a_paired_comparison_counts_each_question_once_and_keeps_the_ties() {
    let keys = keys();
    let a = [answering(&keys, 0..6), answering(&keys, 0..6)];
    let b = [answering(&keys, 0..3), answering(&keys, 6..7)];
    let paired = bench::compare(&keys, &a, &b).unwrap();
    let row = |measure: &str| paired.iter().find(|p| p.measure == measure).unwrap();
    let value = row("value correct");
    assert_eq!(
        (
            value.a_total,
            value.b_total,
            value.a_higher,
            value.b_higher,
            value.ties
        ),
        (12, 4, 6, 1, 24)
    );
    assert_eq!(value.p, "p = 0.1250");
    let unanswered = row("unanswered");
    assert_eq!(
        (
            unanswered.a_total,
            unanswered.b_total,
            unanswered.a_higher,
            unanswered.b_higher
        ),
        (50, 58, 1, 6)
    );
    assert_eq!(unanswered.p, "p = 0.1250");
    for measure in [
        "value wrong",
        "authority right",
        "correct value and authority",
    ] {
        let tied = row(measure);
        assert_eq!(
            (tied.a_higher, tied.b_higher, tied.ties),
            (0, 0, 31),
            "{measure}"
        );
        assert_eq!(tied.p, "p = 1.0000", "{measure}");
    }
}

#[test]
fn a_comparison_without_a_file_on_each_side_or_with_unequal_repeats_is_refused() {
    let keys = keys();
    let one = answering(&keys, 0..1);
    let error = bench::compare(&keys, &[], std::slice::from_ref(&one)).unwrap_err();
    assert_eq!(error, "each condition needs at least one answers file");
    let error = bench::compare(&keys, std::slice::from_ref(&one), &[]).unwrap_err();
    assert_eq!(error, "each condition needs at least one answers file");
    let error = bench::compare(
        &keys,
        &[one.clone(), one.clone()],
        std::slice::from_ref(&one),
    )
    .unwrap_err();
    assert_eq!(
        error,
        "the conditions are unpaired: a has 2 repeats and b has 1"
    );
    let unknown = pyjson::loads(r#"{"answers": {"XYZ-001": {"value": "1"}}}"#).unwrap();
    let error = bench::compare(&keys, &[one], &[unknown]).unwrap_err();
    assert_eq!(
        error,
        "answers file: unknown question XYZ-001 (condition b, file 1)"
    );
}

/// The 2 published naive baselines are a round already in the repository:
/// compared as a against b, each with its one file, they give the README's
/// totals.
#[test]
fn comparing_the_published_baselines_reproduces_the_readme_totals() {
    let root = repo_root();
    let file = |name: &str| format!("{DIR}/baselines/{name}");
    let output = run_in(
        &root,
        &[
            "bench",
            "compare",
            "--a",
            &file("calendar-days.json"),
            "--b",
            &file("weekdays-only.json"),
        ],
        true,
    );
    assert_eq!(output.code, 0, "{}", output.stderr);
    let readme = std::fs::read_to_string(root.join(DIR).join("README.md")).unwrap();
    assert!(
        readme.contains("[calendar-days.json](baselines/calendar-days.json) | 0 | 14 | 17 | 14 |")
    );
    assert!(
        readme.contains("[weekdays-only.json](baselines/weekdays-only.json) | 7 | 7 | 17 | 14 |")
    );
    assert_eq!(
        output.stdout,
        "bench compare: 31 questions paired by question; answers files per condition: 1\n\
         value correct: a 0 (mean 0.00), b 7 (mean 7.00); a higher on 0, b higher on 7, ties 24; two-sided sign test p = 0.0156\n\
         value wrong: a 14 (mean 14.00), b 7 (mean 7.00); a higher on 7, b higher on 0, ties 24; two-sided sign test p = 0.0156\n\
         unanswered: a 17 (mean 17.00), b 17 (mean 17.00); a higher on 0, b higher on 0, ties 31; two-sided sign test p = 1.0000; too few untied questions for any split to give p below 0.05\n\
         authority right: a 14 (mean 14.00), b 14 (mean 14.00); a higher on 0, b higher on 0, ties 31; two-sided sign test p = 1.0000; too few untied questions for any split to give p below 0.05\n\
         correct value and authority: a 0 (mean 0.00), b 7 (mean 7.00); a higher on 0, b higher on 7, ties 24; two-sided sign test p = 0.0156\n"
    );

    // b's file is a copy, because one path given twice is refused first.
    let dir = tempfile::tempdir().unwrap();
    let copy = dir.path().join("weekdays-only.json");
    std::fs::copy(root.join(file("weekdays-only.json")), &copy).unwrap();
    let unpaired = run_in(
        &root,
        &[
            "bench",
            "compare",
            "--a",
            &file("calendar-days.json"),
            &file("weekdays-only.json"),
            "--b",
            copy.to_str().unwrap(),
        ],
        true,
    );
    assert_eq!(unpaired.code, 2);
    assert_eq!(
        unpaired.stderr,
        "bench: the conditions are unpaired: a has 2 repeats and b has 1\n"
    );
    let missing = run_in(
        &root,
        &["bench", "compare", "--a", &file("calendar-days.json")],
        true,
    );
    assert_eq!(missing.code, 2);
    assert!(missing.stderr.contains("--b"), "{}", missing.stderr);
}

/// A file given twice, a file that is not JSON and a file the scorer refuses
/// each stop the comparison, and the message says which file.
#[test]
fn compare_names_the_file_it_refuses() {
    let root = repo_root();
    let calendar = format!("{DIR}/baselines/calendar-days.json");
    let weekdays = format!("{DIR}/baselines/weekdays-only.json");
    let dir = tempfile::tempdir().unwrap();
    let write = |name: &str, body: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, body).unwrap();
        path.to_str().unwrap().to_string()
    };
    let broken = write("broken.json", "{\"answers\": ");
    let unknown = write(
        "unknown.json",
        r#"{"answers": {"XYZ-001": {"value": "1"}}}"#,
    );
    let empty = write("empty.json", r#"{"answers": {}}"#);
    let also_calendar = format!("./{calendar}");
    for (a, b, expected) in [
        // One file spelt 2 ways is still one file.
        (
            vec![calendar.as_str()],
            vec![also_calendar.as_str()],
            format!(
                "bench: {also_calendar} is given more than once: each answers file is one repeat of one condition\n"
            ),
        ),
        (
            vec![calendar.as_str(), weekdays.as_str()],
            vec![weekdays.as_str(), calendar.as_str()],
            format!(
                "bench: {weekdays} is given more than once: each answers file is one repeat of one condition\n"
            ),
        ),
        (
            vec![calendar.as_str()],
            vec![broken.as_str()],
            format!("(condition b, {broken})\n"),
        ),
        (
            vec![calendar.as_str(), weekdays.as_str()],
            vec![empty.as_str(), unknown.as_str()],
            "bench: answers file: unknown question XYZ-001 (condition b, file 2)\n".to_string(),
        ),
    ] {
        let mut args = vec!["bench", "compare", "--a"];
        args.extend(&a);
        args.push("--b");
        args.extend(&b);
        let refused = run_in(&root, &args, true);
        assert_eq!(refused.code, 2, "{}", refused.stdout);
        assert!(refused.stderr.ends_with(&expected), "{}", refused.stderr);
    }
}

fn baseline(name: &str) -> Json {
    let body = std::fs::read_to_string(repo_root().join(DIR).join("baselines").join(name)).unwrap();
    pyjson::loads(&body).unwrap()
}

/// A question counts on every repeat only when each repeat's answer holds the
/// measure: weekdays-only is right on value and authority for 7 questions,
/// calendar-days for none, and answering by position gives no citation.
#[test]
fn a_question_counts_on_every_repeat_only_when_no_repeat_misses_it() {
    let keys = keys();
    let marked = |file: &Json| bench::score(&keys, file).unwrap();
    let weekdays = marked(&baseline("weekdays-only.json"));
    let calendar = marked(&baseline("calendar-days.json"));
    let held = |runs: &[Vec<Scored>]| -> Vec<usize> {
        bench::every_repeat(runs).iter().map(|(_, n)| *n).collect()
    };
    let names: Vec<&str> = bench::every_repeat(&[]).iter().map(|(m, _)| *m).collect();
    assert_eq!(
        names,
        [
            "value correct on every repeat",
            "correct value and authority on every repeat"
        ]
    );
    assert_eq!(held(&[]), [0, 0]);
    assert_eq!(held(std::slice::from_ref(&weekdays)), [7, 7]);
    let again = marked(&baseline("weekdays-only.json"));
    assert_eq!(held(&[weekdays, again]), [7, 7]);
    let weekdays = marked(&baseline("weekdays-only.json"));
    assert_eq!(held(&[weekdays, calendar]), [0, 0]);
    let first_six = marked(&answering(&keys, 0..6));
    let first_three = marked(&answering(&keys, 0..3));
    assert_eq!(held(&[first_six, first_three]), [3, 0]);
    // Right on value both times, but cited only the first time.
    let cited = mark(
        r#"{"answers": {"D7A-001": {"value": "8.77",
        "authorities": ["s 109N(2) of the Income Tax Assessment Act 1936"]}}}"#,
    )
    .unwrap();
    let uncited = mark(r#"{"answers": {"D7A-001": {"value": "8.77"}}}"#).unwrap();
    assert_eq!(held(&[cited, uncited]), [1, 0]);
}

/// With repeats, compare adds the every-repeat measures, each question one
/// pair of yes or no: a is right on 6 questions in both repeats, b on none.
#[test]
fn compare_pairs_the_questions_right_on_every_repeat() {
    let keys = keys();
    let a = [answering(&keys, 0..6), answering(&keys, 0..6)];
    let b = [answering(&keys, 0..3), answering(&keys, 6..7)];
    let paired = bench::compare(&keys, &a, &b).unwrap();
    let every: Vec<_> = paired.iter().filter(|p| p.every_repeat).collect();
    assert_eq!(every.len(), 2);
    let value = every[0];
    assert_eq!(value.measure, "value correct on every repeat");
    assert_eq!(
        (
            value.a_total,
            value.b_total,
            value.a_higher,
            value.b_higher,
            value.ties
        ),
        (6, 0, 6, 0, 25)
    );
    assert_eq!(value.p, "p = 0.0313");
    assert_eq!(
        every[1].measure,
        "correct value and authority on every repeat"
    );
    assert_eq!((every[1].a_total, every[1].ties), (0, 31));

    let once = bench::compare(&keys, &a[..1], &b[..1]).unwrap();
    assert!(once.iter().all(|p| !p.every_repeat));
    assert_eq!(once.len(), 5);
}

#[test]
fn score_reports_each_repeat_and_the_questions_right_on_every_one() {
    let root = repo_root();
    let weekdays = format!("{DIR}/baselines/weekdays-only.json");
    let calendar = format!("{DIR}/baselines/calendar-days.json");
    let dir = tempfile::tempdir().unwrap();
    let copy = dir.path().join("weekdays-again.json");
    std::fs::copy(root.join(&weekdays), &copy).unwrap();
    let copy = copy.to_str().unwrap();

    let text = run_in(&root, &["bench", "score", &weekdays, copy], true);
    assert_eq!(text.code, 0, "{}", text.stderr);
    let out = &text.stdout;
    assert!(
        out.starts_with(&format!(
            "repeat 1: {weekdays}\ncases: 31\nvalue correct: 7\n"
        )),
        "{out}"
    );
    assert!(
        out.contains(&format!("repeat 2: {copy}\ncases: 31\nvalue correct: 7\n")),
        "{out}"
    );
    assert!(
        out.ends_with(
            "across 2 repeats of 31 questions:\n\
             value correct on every repeat: 7\n\
             correct value and authority on every repeat: 7\n"
        ),
        "{out}"
    );

    let mixed = run_in(
        &root,
        &["bench", "score", &weekdays, &calendar, "--json"],
        true,
    );
    assert_eq!(mixed.code, 0, "{}", mixed.stderr);
    let payload = pyjson::loads(&mixed.stdout).unwrap();
    let every = payload.get("every_repeat").unwrap();
    assert_eq!(every.get("repeats"), Some(&Json::Int(2.into())));
    assert_eq!(
        every.get("value_correct_on_every_repeat"),
        Some(&Json::Int(0.into()))
    );
    let Some(Json::Array(repeats)) = payload.get("repeats") else {
        panic!("{}", mixed.stdout)
    };
    assert_eq!(
        repeats[1].get("file").and_then(Json::as_str),
        Some(calendar.as_str())
    );
    assert_eq!(
        repeats[0]
            .get("totals")
            .and_then(|t| t.get("value_correct")),
        Some(&Json::Int(7.into()))
    );

    let twice = run_in(
        &root,
        &["bench", "score", &weekdays, &format!("./{weekdays}")],
        true,
    );
    assert_eq!(twice.code, 2);
    assert!(
        twice.stderr.contains("is given more than once"),
        "{}",
        twice.stderr
    );
    let broken = dir.path().join("broken.json");
    std::fs::write(&broken, "{\"answers\": ").unwrap();
    let broken = broken.to_str().unwrap();
    let refused = run_in(&root, &["bench", "score", &weekdays, broken], true);
    assert_eq!(refused.code, 2);
    assert!(
        refused.stderr.ends_with(&format!("({broken})\n")),
        "{}",
        refused.stderr
    );
}

#[test]
fn compare_prints_the_every_repeat_rows_only_for_repeats() {
    let root = repo_root();
    let file = |name: &str| format!("{DIR}/baselines/{name}");
    let dir = tempfile::tempdir().unwrap();
    let again = |name: &str| {
        let copy = dir.path().join(name);
        std::fs::copy(root.join(file(name)), &copy).unwrap();
        copy.to_str().unwrap().to_string()
    };
    let (weekdays, calendar) = (again("weekdays-only.json"), again("calendar-days.json"));
    let output = run_in(
        &root,
        &[
            "bench",
            "compare",
            "--a",
            &file("weekdays-only.json"),
            &weekdays,
            "--b",
            &file("calendar-days.json"),
            &calendar,
        ],
        true,
    );
    assert_eq!(output.code, 0, "{}", output.stderr);
    let out = &output.stdout;
    assert!(
        out.starts_with("bench compare: 31 questions paired by question; answers files per condition: 2\n\
         value correct: a 14 (mean 7.00), b 0 (mean 0.00); a higher on 7, b higher on 0, ties 24; two-sided sign test p = 0.0156\n"),
        "{out}"
    );
    assert!(
        out.ends_with(
            "value correct on every repeat: a 7, b 0 of 31 questions; a higher on 7, b higher on 0, ties 24; two-sided sign test p = 0.0156\n\
             correct value and authority on every repeat: a 7, b 0 of 31 questions; a higher on 7, b higher on 0, ties 24; two-sided sign test p = 0.0156\n"
        ),
        "{out}"
    );
    assert_eq!(out.lines().count(), 8, "{out}");
}
