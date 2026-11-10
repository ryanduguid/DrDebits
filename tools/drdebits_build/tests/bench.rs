// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: LicenseRef-Duguid-Public-Attribution-1.0

//! The published benchmark's keys, and a scorer that keeps a wrong citation
//! apart from a wrong figure.

mod common;

use std::path::Path;

use common::{py, repo_root, run_in};
use drdebits::bench::{self, Cited, Scored, Value};
use drdebits::pyjson::{self, Json};

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
    bench::load_keys(&py(&repo_root())).unwrap()
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
    let report = bench::check_keys(&py(&repo_root())).unwrap();
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
    let report = bench::check_keys(&py(tree.path())).unwrap();
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
    let report = bench::check_keys(&py(tree.path())).unwrap();
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
    let report = bench::check_keys(&py(tree.path())).unwrap();
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
    let error = bench::check_keys(&py(tree.path())).unwrap_err();
    assert!(error.contains("past the calendar"), "{error}");
}

#[test]
fn keys_that_do_not_answer_the_questions_one_for_one_are_refused() {
    let tree = copy();
    edit(tree.path(), "questions.json", "PDS-001", "PDS-000");
    let error = bench::load_keys(&py(tree.path())).unwrap_err();
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
        bench::load_keys(&py(tree.path())).unwrap_err(),
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
        let error = bench::load_keys(&py(tree.path())).unwrap_err();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn a_changed_canary_is_refused() {
    let tree = copy();
    edit(tree.path(), "keys.json", "DRDEBITS-BENCHMARK", "BENCHMARK");
    let error = bench::load_keys(&py(tree.path())).unwrap_err();
    assert!(error.contains("benchmark_canary"), "{error}");
}

#[test]
fn a_key_needs_its_source_and_derivation() {
    let tree = copy();
    edit(tree.path(), "keys.json", "\"derivation\"", "\"working\"");
    let error = bench::load_keys(&py(tree.path())).unwrap_err();
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
    let error = bench::load_keys(&py(tree.path())).unwrap_err();
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
        let report = bench::check_keys(&py(tree.path())).unwrap();
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
    let report = bench::check_keys(&py(tree.path())).unwrap();
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
    let error = bench::check_keys(&py(tree.path())).unwrap_err();
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
    let error = bench::check_keys(&py(tree.path())).unwrap_err();
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
        let found = bench::check_keys(&py(tree.path()));
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
    let error = bench::check_keys(&py(tree.path())).unwrap_err();
    assert!(
        error.contains("is before the calendar's first complete day"),
        "{error}"
    );
}
