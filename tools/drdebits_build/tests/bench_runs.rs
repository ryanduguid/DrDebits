// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! `bench runs` and the strict, partial and topic counts `bench score` gained
//! with it. Every answers file here is made up, and so is every model name
//! and price: no model wrote any of it.

mod common;

use common::{py, repo_root, run_in};
use drdebits::bench::{self, Slice, Unstable};
use drdebits::pyjson::{self, Json};

const TOPIC: &str = "payday-div7a";

fn keys() -> Vec<bench::Key> {
    bench::load_keys(&py(&repo_root()), TOPIC).unwrap()
}

/// Each listed key's value and first listed section, from `keys.json`.
fn key_rows() -> Vec<(String, String, String)> {
    let path = repo_root().join("benchmarks").join(TOPIC).join("keys.json");
    let payload = pyjson::loads(&std::fs::read_to_string(path).unwrap()).unwrap();
    let Some(Json::Array(rows)) = payload.get("keys") else {
        panic!("keys")
    };
    rows.iter()
        .map(|k| {
            let field = |f: &str| k.get(f).and_then(Json::as_str).unwrap().to_string();
            let Some(Json::Array(cited)) = k.get("authorities") else {
                panic!("authorities")
            };
            (
                field("id"),
                field("value"),
                cited[0].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

/// An answers file: `right` ids answered with the key's value and section,
/// `value_only` ids with the value and no section, and an optional `run`.
fn answers(right: &[&str], value_only: &[&str], run: Option<&str>) -> String {
    let rows = key_rows();
    let find = |id: &str| rows.iter().find(|r| r.0 == id).unwrap();
    let mut entries: Vec<String> = right
        .iter()
        .map(|id| {
            let (_, value, cited) = find(id);
            format!("\"{id}\": {{\"value\": \"{value}\", \"authorities\": [\"{cited}\"]}}")
        })
        .collect();
    entries.extend(value_only.iter().map(|id| {
        let (_, value, _) = find(id);
        format!("\"{id}\": {{\"value\": \"{value}\"}}")
    }));
    let run = run.map_or(String::new(), |r| format!("\"run\": {r}, "));
    format!("{{{run}\"answers\": {{{}}}}}", entries.join(", "))
}

fn parsed(body: &str) -> Json {
    pyjson::loads(body).unwrap()
}

fn summarise(files: &[(&str, String)]) -> bench::RunsReport {
    let files: Vec<(String, Json)> = files
        .iter()
        .map(|(p, b)| (p.to_string(), parsed(b)))
        .collect();
    bench::summarise(&keys(), &files).unwrap()
}

fn run_error(run: &str) -> String {
    let body = answers(&[], &[], Some(run));
    match bench::read_run(&parsed(&body)) {
        Err(e) => e,
        Ok(r) => panic!("{run} was read as {r:?}"),
    }
}

#[test]
fn a_run_record_prices_a_run_only_from_its_own_figures() {
    let read = |run: &str| {
        bench::read_run(&parsed(&answers(&[], &[], Some(run))))
            .unwrap()
            .unwrap()
    };
    let rated = read(
        r#"{"model": "model-a", "input_tokens": 1000000, "output_tokens": 200000,
        "input_per_million": "3.00", "output_per_million": "15.00"}"#,
    );
    assert_eq!(rated.model.as_deref(), Some("model-a"));
    assert_eq!(rated.cost_usd().as_deref(), Some("6.0000"));
    let recorded = read(r#"{"cost_usd": "0.4200"}"#);
    assert_eq!(
        (recorded.model.as_deref(), recorded.cost_usd().as_deref()),
        (None, Some("0.4200"))
    );
    let tokens_only = read(r#"{"model": "model-b", "input_tokens": 10, "output_tokens": 5}"#);
    assert_eq!(tokens_only.cost_usd(), None);
    assert_eq!(
        (tokens_only.input_tokens, tokens_only.output_tokens),
        (Some(10), Some(5))
    );
    assert!(
        bench::read_run(&parsed(&answers(&[], &[], None)))
            .unwrap()
            .is_none()
    );
}

#[test]
fn a_run_record_refuses_what_it_cannot_price_exactly() {
    for (run, expected) in [
        ("[]", "run must be an object"),
        (r#"{"price": "1"}"#, "unknown field price"),
        (r#"{"model": " "}"#, "model must not be empty"),
        (r#"{"model": 3}"#, "model"),
        (
            r#"{"input_tokens": -1}"#,
            "input_tokens must be a whole number",
        ),
        (
            r#"{"output_tokens": "9"}"#,
            "output_tokens must be a whole number",
        ),
        (r#"{"cost_usd": "-0.01"}"#, "cost_usd must not be negative"),
        (r#"{"cost_usd": "lots"}"#, "cost_usd lots is not a number"),
        (
            r#"{"cost_usd": "1", "input_per_million": "1", "output_per_million": "1"}"#,
            "not both",
        ),
        (
            r#"{"input_per_million": "1", "output_per_million": "1", "input_tokens": 5}"#,
            "need input_tokens and output_tokens",
        ),
        (
            r#"{"input_per_million": "1", "input_tokens": 5, "output_tokens": 5}"#,
            "give both input_per_million and output_per_million",
        ),
    ] {
        let error = run_error(run);
        assert!(error.contains(expected), "{run}: {error}");
    }
    // The scorer refuses the same files, so `bench score` and `bench compare`
    // never pass over a malformed record.
    let body = answers(&[], &[], Some(r#"{"price": "1"}"#));
    let error = bench::score(&keys(), &parsed(&body)).unwrap_err();
    assert!(error.contains("unknown field price"), "{error}");
    let fine = answers(&["D7A-001"], &[], Some(r#"{"model": "model-a"}"#));
    assert_eq!(bench::score(&keys(), &parsed(&fine)).unwrap().len(), 31);
}

#[test]
fn strict_pass_needs_every_criterion_and_partial_credit_counts_each() {
    let report = summarise(&[(
        "a.json",
        answers(&["D7A-001", "PDS-001"], &["D7A-002"], None),
    )]);
    let c = &report.conditions[0];
    assert_eq!(c.label, "a.json");
    assert_eq!((c.strict_passed, c.criteria_met), (2, 5));
    assert_eq!(c.strict_rate, "0.0645");
    assert_eq!(c.partial_credit, "0.0806");
    assert_eq!(c.strict_per_repeat, [2]);
    assert!(c.unstable.is_empty());
    assert_eq!(report.saturation, None);
}

#[test]
fn slices_follow_each_question_topic_and_name_a_question_without_one() {
    let report = summarise(&[("a.json", answers(&["D7A-001", "PDS-001"], &[], None))]);
    assert_eq!(
        report.conditions[0].families,
        [
            Slice {
                name: "Payday Super dates".into(),
                questions: 15,
                answers: 15,
                strict_passed: 1,
                criteria_met: 2,
            },
            Slice {
                name: "Division 7A".into(),
                questions: 16,
                answers: 16,
                strict_passed: 1,
                criteria_met: 2,
            },
        ]
    );
    let tree = tempfile::tempdir().unwrap();
    let dir = tree.path().join("benchmarks").join(TOPIC);
    std::fs::create_dir_all(&dir).unwrap();
    for file in ["questions.json", "keys.json"] {
        std::fs::copy(
            repo_root().join("benchmarks").join(TOPIC).join(file),
            dir.join(file),
        )
        .unwrap();
    }
    let questions = dir.join("questions.json");
    let body = std::fs::read_to_string(&questions).unwrap();
    let dropped = body.replacen("\"topic\": \"Division 7A\", ", "", 1);
    assert_ne!(dropped, body);
    std::fs::write(&questions, dropped).unwrap();
    let keys = bench::load_keys(&py(tree.path()), TOPIC).unwrap();
    let scored = bench::score(&keys, &parsed(&answers(&[], &[], None))).unwrap();
    let names: Vec<(String, usize)> = bench::families(&keys, &[&scored])
        .into_iter()
        .map(|f| (f.name, f.questions))
        .collect();
    assert_eq!(
        names,
        [
            ("Payday Super dates".to_string(), 15),
            (bench::NO_FAMILY.to_string(), 1),
            ("Division 7A".to_string(), 15),
        ]
    );
    let broken = body.replacen("\"topic\": \"Division 7A\"", "\"topic\": 7", 1);
    std::fs::write(&questions, broken).unwrap();
    let error = bench::load_keys(&py(tree.path()), TOPIC).unwrap_err();
    assert!(error.contains("topic"), "{error}");
}

#[test]
fn repeats_group_by_model_and_a_question_that_passes_on_some_is_unstable() {
    let a = r#"{"model": "model-a"}"#;
    let report = summarise(&[
        ("a1.json", answers(&["D7A-001", "D7A-002"], &[], Some(a))),
        ("b1.json", answers(&["D7A-001"], &[], None)),
        ("a2.json", answers(&["D7A-001"], &["D7A-002"], Some(a))),
        ("a3.json", answers(&["D7A-001", "D7A-002"], &[], Some(a))),
    ]);
    let labels: Vec<&str> = report.conditions.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, ["model-a", "b1.json"]);
    let a = &report.conditions[0];
    assert_eq!(a.files, ["a1.json", "a2.json", "a3.json"]);
    assert_eq!(a.strict_per_repeat, [2, 1, 2]);
    assert_eq!(
        a.unstable,
        [Unstable {
            id: "D7A-002".into(),
            strict_passed: 2,
            repeats: 3,
        }]
    );
    let saturation = report.saturation.unwrap();
    assert_eq!(saturation.every_condition_passes, ["D7A-001"]);
    assert_eq!(saturation.every_condition_fails.len(), 29);
    assert!(
        !saturation
            .every_condition_fails
            .iter()
            .any(|id| id == "D7A-001" || id == "D7A-002")
    );
}

#[test]
fn the_frontier_keeps_runs_no_other_priced_run_beats_on_both_axes() {
    let priced =
        |model: &str, cost: &str| format!(r#"{{"model": "{model}", "cost_usd": "{cost}"}}"#);
    let report = summarise(&[
        (
            "a.json",
            answers(
                &["D7A-001", "D7A-002"],
                &[],
                Some(&priced("model-a", "1.00")),
            ),
        ),
        (
            "b.json",
            answers(&["D7A-001"], &[], Some(&priced("model-b", "2.00"))),
        ),
        (
            "c.json",
            answers(
                &["D7A-001", "D7A-002", "D7A-003"],
                &[],
                Some(r#"{"model": "model-c"}"#),
            ),
        ),
        (
            "d.json",
            answers(
                &["D7A-001", "D7A-002", "D7A-003"],
                &[],
                Some(&priced("model-d", "3.00")),
            ),
        ),
        (
            "e.json",
            answers(&[], &[], Some(&priced("model-e", "0.10"))),
        ),
        (
            "f.json",
            answers(
                &["D7A-001", "D7A-002"],
                &[],
                Some(&priced("model-f", "1.00")),
            ),
        ),
    ]);
    let row = |label: &str| {
        let c = report.conditions.iter().find(|c| c.label == label).unwrap();
        (
            c.on_frontier,
            c.cost_usd.as_deref(),
            c.cost_per_passed_task_usd.as_deref(),
        )
    };
    assert_eq!(row("model-a"), (Some(true), Some("1.0000"), Some("0.5000")));
    assert_eq!(
        row("model-b"),
        (Some(false), Some("2.0000"), Some("2.0000"))
    );
    assert_eq!(row("model-c"), (None, None, None));
    assert_eq!(row("model-d"), (Some(true), Some("3.0000"), Some("1.0000")));
    // Cheapest and passing nothing: on the frontier, with no cost per pass.
    assert_eq!(row("model-e"), (Some(true), Some("0.1000"), None));
    // A tie on both axes leaves both on the frontier.
    assert_eq!(row("model-f"), (Some(true), Some("1.0000"), Some("0.5000")));
}

#[test]
fn a_condition_is_priced_only_when_every_repeat_is() {
    let report = summarise(&[
        (
            "a1.json",
            answers(
                &["D7A-001"],
                &[],
                Some(
                    r#"{"model": "model-a", "input_tokens": 2000000, "output_tokens": 100000,
                    "input_per_million": "1.50", "output_per_million": "10"}"#,
                ),
            ),
        ),
        (
            "a2.json",
            answers(
                &["D7A-001"],
                &[],
                Some(r#"{"model": "model-a", "cost_usd": "5"}"#),
            ),
        ),
        (
            "b1.json",
            answers(
                &["D7A-001"],
                &[],
                Some(r#"{"model": "model-b", "cost_usd": "1"}"#),
            ),
        ),
        (
            "b2.json",
            answers(&[], &[], Some(r#"{"model": "model-b"}"#)),
        ),
    ]);
    let a = &report.conditions[0];
    assert_eq!(a.priced_files, 2);
    assert_eq!(a.cost_usd.as_deref(), Some("9.0000"));
    assert_eq!(a.mean_cost_per_run_usd.as_deref(), Some("4.5000"));
    assert_eq!(a.cost_per_passed_task_usd.as_deref(), Some("4.5000"));
    assert_eq!(a.tokens, None);
    assert_eq!(a.on_frontier, Some(true));
    let b = &report.conditions[1];
    assert_eq!(
        (b.priced_files, b.cost_usd.as_deref(), b.on_frontier),
        (1, None, None)
    );
}

#[test]
fn bench_runs_prints_the_same_report_every_time_and_refuses_a_bad_file() {
    let dir = tempfile::tempdir().unwrap();
    let write = |name: &str, body: String| {
        let path = dir.path().join(name);
        std::fs::write(&path, body).unwrap();
        path.to_string_lossy().into_owned()
    };
    let run = r#"{"model": "model-a", "input_tokens": 1000, "output_tokens": 500,
        "input_per_million": "2", "output_per_million": "8"}"#;
    let a1 = write("a1.json", answers(&["D7A-001", "PDS-001"], &[], Some(run)));
    let a2 = write("a2.json", answers(&["D7A-001"], &["PDS-001"], Some(run)));
    let b = write(
        "b.json",
        answers(&["D7A-001"], &[], Some(r#"{"model": "model-b"}"#)),
    );
    let root = repo_root();
    let args = ["bench", "runs", &a1, &a2, &b];
    let first = run_in(&root, &args, true);
    assert_eq!(first.code, 0, "{}", first.stderr);
    assert_eq!(first.stdout, run_in(&root, &args, true).stdout);
    for line in [
        "bench runs: 3 answers file(s), 2 condition(s), 31 questions (topic payday-div7a)",
        "condition model-a: 2 answers file(s)",
        "strict pass: 3/62 (4.84%) answers met every criterion",
        "partial credit: 7/124 (5.65%) criteria met",
        "family Payday Super dates: 1/30 (3.33%) strict, 3/60 (5.00%) criteria",
        "strict passes per repeat: 2, 1",
        "unstable: PDS-001 1/2",
        "tokens: 2000 input, 1000 output",
        "cost: $0.0120 USD, $0.0060 USD a run; per passed task: $0.0040 USD; on the frontier",
        "condition model-b: 1 answers file(s)",
        "unstable: needs 2 or more repeats",
        "tokens: not recorded on every file",
        "cost: not priced (0 of 1 file(s) record a cost)",
        "every condition passes: D7A-001",
    ] {
        assert!(
            first.stdout.lines().any(|l| l == line),
            "{line}\n{}",
            first.stdout
        );
    }

    let json = run_in(&root, &["bench", "runs", &a1, &a2, &b, "--json"], true);
    let payload = parsed(&json.stdout);
    let Some(Json::Array(conditions)) = payload.get("conditions") else {
        panic!("{}", json.stdout)
    };
    let a = &conditions[0];
    assert_eq!(
        a.get("cost_per_passed_task_usd").and_then(Json::as_str),
        Some("0.0040")
    );
    assert_eq!(a.get("on_frontier"), Some(&Json::Bool(true)));
    assert_eq!(
        a.get("strict_pass")
            .and_then(|s| s.get("rate"))
            .and_then(Json::as_str),
        Some("0.0484")
    );
    assert_eq!(conditions[1].get("cost_usd"), Some(&Json::Null));
    assert_eq!(
        payload
            .get("saturation")
            .and_then(|s| s.get("every_condition_passes")),
        Some(&Json::Array(vec![Json::str_obj("D7A-001")]))
    );

    let one = run_in(&root, &["bench", "runs", &b], true);
    assert!(
        one.stdout
            .lines()
            .any(|l| l == "saturation: needs 2 or more conditions"),
        "{}",
        one.stdout
    );
    let one_json = parsed(&run_in(&root, &["bench", "runs", &b, "--json"], true).stdout);
    assert_eq!(one_json.get("saturation"), Some(&Json::Null));

    let costly = r#"{"model": "model-c", "cost_usd": "1"}"#;
    let c1 = write("c1.json", answers(&[], &[], Some(costly)));
    let c2 = write("c2.json", answers(&[], &[], Some(costly)));
    let worse = run_in(&root, &["bench", "runs", &c1, &c2, &a1], true);
    for line in [
        "cost: $2.0000 USD, $1.0000 USD a run; per passed task: none passed; below the frontier",
        "unstable: none",
        "every condition passes: none",
    ] {
        assert!(
            worse.stdout.lines().any(|l| l == line),
            "{line}\n{}",
            worse.stdout
        );
    }
    let twice = run_in(&root, &["bench", "runs", &b, &b], true);
    assert_eq!(twice.code, 2);
    assert!(twice.stderr.contains("more than once"), "{}", twice.stderr);
    let bad = write("bad.json", answers(&[], &[], Some(r#"{"cost_usd": "-1"}"#)));
    let refused = run_in(&root, &["bench", "runs", &bad], true);
    assert_eq!(refused.code, 2);
    assert!(
        refused.stderr.contains("cost_usd must not be negative") && refused.stderr.contains(&bad),
        "{}",
        refused.stderr
    );
    let missing = run_in(&root, &["bench", "runs", "no-such-file.json"], true);
    assert_eq!(missing.code, 2);
    assert!(
        missing.stderr.contains("no-such-file.json"),
        "{}",
        missing.stderr
    );
}

#[test]
fn bench_score_reports_strict_pass_partial_credit_and_topic_slices() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("answers.json");
    std::fs::write(&path, answers(&["D7A-001"], &["PDS-001"], None)).unwrap();
    let path = path.to_str().unwrap();
    let root = repo_root();
    let text = run_in(&root, &["bench", "score", path], true).stdout;
    for line in [
        "strict pass: 1/31 (3.23%) answers met every criterion",
        "partial credit: 3/62 (4.84%) criteria met",
        "family Payday Super dates: 0/15 (0.00%) strict, 1/30 (3.33%) criteria",
        "family Division 7A: 1/16 (6.25%) strict, 2/32 (6.25%) criteria",
    ] {
        assert!(text.lines().any(|l| l == line), "{line}\n{text}");
    }
    let json = run_in(&root, &["bench", "score", path, "--json"], true).stdout;
    let payload = parsed(&json);
    assert_eq!(
        payload
            .get("partial_credit")
            .and_then(|p| p.get("criteria_met")),
        Some(&Json::Int(3.into()))
    );
    let Some(Json::Array(families)) = payload.get("families") else {
        panic!("{json}")
    };
    assert_eq!(
        families[1].get("family").and_then(Json::as_str),
        Some("Division 7A")
    );
}
