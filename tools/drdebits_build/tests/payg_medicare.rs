// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: LicenseRef-Duguid-Public-Attribution-1.0

//! The PAYG withholding and Medicare levy benchmark: every key is the figure
//! its cited source row prints, and `bench keys` recomputes each one from the
//! Schedule 1 coefficients or the Medicare Levy Act thresholds the key records.

mod common;

use std::path::Path;

use common::{py, repo_root, run_in};
use drdebits::bench;
use drdebits::pyjson::{self, Json};

const TOPIC: &str = "payg-medicare";
const DIR: &str = "benchmarks/payg-medicare";
const FILES: [&str; 3] = ["questions.json", "keys.json", "schedule-1.json"];

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

fn problems(tree: &Path) -> Vec<String> {
    bench::check_keys(&py(tree), TOPIC).unwrap().problems
}

fn keys_json() -> Vec<Json> {
    let body = std::fs::read_to_string(repo_root().join(DIR).join("keys.json")).unwrap();
    match pyjson::loads(&body).unwrap().get("keys") {
        Some(Json::Array(keys)) => keys.clone(),
        _ => panic!("keys.json has no keys"),
    }
}

#[test]
fn every_key_is_recomputed_from_its_cited_source_row() {
    let report = bench::check_keys(&py(&repo_root()), TOPIC).unwrap();
    assert_eq!(report.problems, Vec::<String>::new());
    assert_eq!((report.computed, report.not_recomputed), (20, 0));
    assert_eq!(report.engine_checked, 0);
}

/// Each key names the instrument or ATO page it comes from, the row or example
/// within it, and the day it was read.
#[test]
fn every_key_cites_a_source_row_read_on_the_day() {
    for key in keys_json() {
        let source = key.get("source").unwrap();
        let field = |name: &str| source.get(name).and_then(Json::as_str).unwrap_or_default();
        let id = key.get("id").and_then(Json::as_str).unwrap();
        assert_eq!(field("read_on"), "2026-11-15", "{id}");
        assert!(!field("where").is_empty(), "{id}");
        let url = field("url");
        if id.starts_with("PAYG-") {
            assert_eq!(
                url,
                "https://www.legislation.gov.au/F2026L00716/asmade/text"
            );
            assert!(
                field("title")
                    .starts_with("Taxation Administration (Withholding Schedules) Instrument 2026")
            );
        } else {
            assert!(url.starts_with("https://www.ato.gov.au/individuals-and-families/medicare-and-private-health-insurance/medicare-levy/"), "{id}");
        }
    }
}

/// A wrong coefficient, a wrong key and a wrong threshold are each caught by
/// the keys whose rows use them, and only by those keys. PAYG-001 and PAYG-010
/// (whose x is 538.99) share the scale 2 row below $673.
#[test]
fn a_changed_row_moves_only_the_keys_that_use_it() {
    let tree = copy();
    edit(
        tree.path(),
        "schedule-1.json",
        "\"b\": \"108.2135\"",
        "\"b\": \"109.2135\"",
    );
    assert_eq!(
        problems(tree.path()),
        [
            "PAYG-001: the key says 27.00 and the rule gives 26.00",
            "PAYG-010: the key says 117.00 and the rule gives 113.00",
        ]
    );

    let tree = copy();
    edit(
        tree.path(),
        "keys.json",
        "\"value\": \"98.90\"",
        "\"value\": \"98.00\"",
    );
    assert_eq!(
        problems(tree.path()),
        ["ML-002: the key says 98.00 and the rule gives 98.90"]
    );

    let tree = copy();
    edit(
        tree.path(),
        "schedule-1.json",
        "\"additional_child\": \"4338\"",
        "\"additional_child\": \"4000\"",
    );
    let moved: Vec<String> = problems(tree.path())
        .iter()
        .map(|p| p[..8].to_string())
        .collect();
    assert!(moved.contains(&"PAYG-014".to_string()), "{moved:?}");
    assert!(
        moved
            .iter()
            .all(|id| ["PAYG-013", "PAYG-014", "PAYG-015", "PAYG-016"].contains(&id.as_str())),
        "{moved:?}"
    );
}

/// Each branch of the formulas, checked against another row of the same
/// published sample data: the nil row of scale 2, and the weekly and monthly
/// Medicare levy adjustment below the threshold, in formulas (1) and (2), and
/// at the shading out point.
#[test]
fn other_published_sample_rows_reproduce_through_every_branch() {
    for (file, from, to, expected) in [
        // Weekly withholding, $361, scale 2: 0.00.
        (
            "keys.json",
            "\"earnings\": \"538\"",
            "\"earnings\": \"361\"",
            "PAYG-001: the key says 27.00 and the rule gives 0.00",
        ),
        // Weekly adjustment, $537, any column: 0.00.
        (
            "keys.json",
            "\"earnings\": \"1319.21\"",
            "\"earnings\": \"537\"",
            "PAYG-014: the key says 19.00 and the rule gives 0.00",
        ),
        // Weekly adjustment, $604, any column: 7.00 (formula (1)).
        (
            "keys.json",
            "\"earnings\": \"1319.21\"",
            "\"earnings\": \"604\"",
            "PAYG-014: the key says 19.00 and the rule gives 7.00",
        ),
        // Weekly adjustment, $706, any column: 14.00 (formula (2)).
        (
            "keys.json",
            "\"earnings\": \"1319.21\"",
            "\"earnings\": \"706\"",
            "PAYG-014: the key says 19.00 and the rule gives 14.00",
        ),
        // Weekly adjustment, $1,551, 4 children: 0.00 (at or past the SOP).
        (
            "keys.json",
            "\"earnings\": \"1319.21\"",
            "\"earnings\": \"1551\"",
            "PAYG-014: the key says 19.00 and the rule gives 0.00",
        ),
        // Monthly adjustment, $2,617.33, spouse only: 30.00 (formula (1) from monthly earnings).
        (
            "keys.json",
            "\"earnings\": \"4238.00\"",
            "\"earnings\": \"2617.33\"",
            "PAYG-017: the key says 56.00 and the rule gives 30.00",
        ),
    ] {
        let tree = copy();
        edit(tree.path(), file, from, to);
        assert_eq!(problems(tree.path()), [expected], "{to}");
    }
}

/// The levy's branches under the Act: no levy at the threshold amount
/// (s 7(1)); no levy where family income does not exceed the family income
/// threshold (s 8(1)); no reduction where the s 8(2) formula is negative; and
/// a liable spouse refused, because ss 8(3) and 8(4) need facts the key does
/// not hold.
#[test]
fn the_levy_follows_each_limb_of_ss_7_and_8() {
    let tree = copy();
    edit(
        tree.path(),
        "keys.json",
        "\"taxable_income\": \"29000\"",
        "\"taxable_income\": \"28011\"",
    );
    assert_eq!(
        problems(tree.path()),
        ["ML-002: the key says 98.90 and the rule gives 0.00"]
    );

    // Family income 49,700 + 11,000 = 60,700 does not exceed 61,623.
    let tree = copy();
    edit(
        tree.path(),
        "keys.json",
        "\"taxable_income\": \"21700\"",
        "\"taxable_income\": \"11000\"",
    );
    assert_eq!(
        problems(tree.path()),
        ["ML-003: the key says 92.90 and the rule gives 0.00"]
    );

    // Family income 89,700, the spouse's 40,000 still within 44,268: 2% of
    // 61,623 less 0.08 of 28,077 is negative, so the s 7(2) levy stands:
    // 10% of (49,700 − 44,268) = 543.20.
    let tree = copy();
    edit(
        tree.path(),
        "keys.json",
        "\"taxable_income\": \"21700\"",
        "\"taxable_income\": \"40000\"",
    );
    assert_eq!(
        problems(tree.path()),
        ["ML-003: the key says 92.90 and the rule gives 543.20"]
    );

    // Taxable income 30,000 and a spouse's 29,000, both over 28,011, against a
    // family income threshold of 47,238. The spouse would be liable, so
    // s 8(3) shares the reduction and s 8(4) can pass the spouse's unused
    // share back, which needs the spouse's own levy and family income
    // threshold. The key does not hold them, so the rule refuses the case.
    // With 28,012 for the spouse, s 8(3) alone would give 156.06 and s 8(4)
    // takes it to 116.16: working s 8(3) without s 8(4) is wrong.
    let tree = copy();
    for (from, to) in [
        (
            "\"taxable_income\": \"49700\"",
            "\"taxable_income\": \"30000\"",
        ),
        (
            "\"threshold_amount\": \"44268\"",
            "\"threshold_amount\": \"28011\"",
        ),
        (
            "\"phase_in_limit\": \"55335\"",
            "\"phase_in_limit\": \"35013\"",
        ),
        (
            "\"family_income_threshold\": \"61623\"",
            "\"family_income_threshold\": \"47238\"",
        ),
        (
            "\"taxable_income\": \"21700\"",
            "\"taxable_income\": \"29000\"",
        ),
        (
            "\"threshold_amount\": \"44268\"",
            "\"threshold_amount\": \"28011\"",
        ),
    ] {
        edit(tree.path(), "keys.json", from, to);
    }
    let error = bench::check_keys(&py(tree.path()), TOPIC).unwrap_err();
    assert_eq!(
        error,
        "ML-003: the spouse would be liable, so ss 8(3) and 8(4) apply, which this rule does not work"
    );
}

#[test]
fn a_rule_that_cannot_be_worked_stops_the_check() {
    for (file, from, to, expected) in [
        // Offsets larger than the withholding leave a negative amount.
        (
            "keys.json",
            "\"offsets\": \"1365\"",
            "\"offsets\": \"136500\"",
            "PAYG-011: the rule cannot be worked",
        ),
        // A row whose b exceeds ax gives a negative y.
        (
            "schedule-1.json",
            "\"b\": \"108.2135\"",
            "\"b\": \"999\"",
            "PAYG-001: y is negative",
        ),
        (
            "keys.json",
            "\"period\": \"monthly\"",
            "\"period\": \"quarterly\"",
            "unknown period quarterly",
        ),
        (
            "schedule-1.json",
            "\"6\": {",
            "\"7\": {",
            "PAYG-016: Schedule 1 has no medicare_levy_parameters for 6",
        ),
        (
            "keys.json",
            "\"scale\": \"2\"",
            "\"scale\": \"9\"",
            "Schedule 1 has no scales for 9",
        ),
        // Scale 1 (no tax-free threshold) allows no tax offsets.
        (
            "keys.json",
            "\"earnings\": \"721\"",
            "\"earnings\": \"721\", \"offsets\": \"1000\"",
            "PAYG-004: Schedule 1 allows tax offsets only on scales 2, 5 and 6",
        ),
    ] {
        let tree = copy();
        edit(tree.path(), file, from, to);
        let error = bench::check_keys(&py(tree.path()), TOPIC).unwrap_err();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn scale_rows_must_be_a_list_that_covers_x() {
    let tree = copy();
    edit(
        tree.path(),
        "schedule-1.json",
        "{\"below\": null, \"a\": \"0.4700\", \"b\": \"655.7704\"}",
        "{\"below\": \"3653\", \"a\": \"0.4700\", \"b\": \"655.7704\"}",
    );
    let error = bench::check_keys(&py(tree.path()), TOPIC).unwrap_err();
    assert!(
        error.contains("no row of scale 2 covers 3653.99"),
        "{error}"
    );

    let tree = copy();
    edit(
        tree.path(),
        "schedule-1.json",
        "\"3\": [",
        "\"3\": \"none\", \"x\": [",
    );
    let error = bench::check_keys(&py(tree.path()), TOPIC).unwrap_err();
    assert!(error.contains("scale 3 must be a list"), "{error}");
}

#[test]
fn the_commands_take_the_topic_and_refuse_an_unknown_one() {
    let root = repo_root();
    let keys = run_in(&root, &["bench", "keys", "--topic", TOPIC], true);
    assert_eq!(keys.code, 0, "{}", keys.stderr);
    assert_eq!(
        keys.stdout.trim(),
        "bench keys: 20 recomputed, 0 source-derived keys not independently recomputed"
    );

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("answers.json");
    std::fs::write(
        &path,
        r#"{"answers": {"PAYG-001": {"value": "27",
        "authorities": ["s 15-10 of the Taxation Administration Act 1953"]},
        "ML-002": {"value": "$98.90",
        "authorities": ["s 15-10(1) of Schedule 1 to the Taxation Administration Act 1953"]}}}"#,
    )
    .unwrap();
    let scored = run_in(
        &root,
        &["bench", "--topic", TOPIC, "score", path.to_str().unwrap()],
        true,
    );
    assert_eq!(scored.code, 0, "{}", scored.stderr);
    for line in [
        "value correct: 2",
        "authority right: 1",
        "authority unreadable: 1",
        "unanswered: 18",
    ] {
        assert!(scored.stdout.contains(line), "{line}: {}", scored.stdout);
    }

    let unknown = run_in(&root, &["bench", "keys", "--topic", "nope"], true);
    assert_eq!(unknown.code, 2);
    assert_eq!(unknown.stderr.trim(), "bench: unknown topic nope");
}

/// `bench compare` pairs the named topic's questions, and refuses a file
/// that answers another topic's.
#[test]
fn compare_pairs_the_named_topics_questions() {
    let dir = tempfile::tempdir().unwrap();
    let file = |name: &str, body: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, body).unwrap();
        path.to_str().unwrap().to_string()
    };
    let right = file(
        "right.json",
        r#"{"answers": {"PAYG-001": {"value": "27"}}}"#,
    );
    let wrong = file(
        "wrong.json",
        r#"{"answers": {"PAYG-001": {"value": "26"}}}"#,
    );
    let payday = file(
        "payday.json",
        r#"{"answers": {"PDS-001": {"value": "2026-08-07"}}}"#,
    );
    let root = repo_root();

    let paired = run_in(
        &root,
        &[
            "bench", "compare", "--topic", TOPIC, "--a", &right, "--b", &wrong,
        ],
        true,
    );
    assert_eq!(paired.code, 0, "{}", paired.stderr);
    let lines: Vec<&str> = paired.stdout.lines().collect();
    assert_eq!(
        lines[0],
        "bench compare: 20 questions paired by question; answers files per condition: 1"
    );
    assert!(
        lines[1].starts_with(
            "value correct: a 1 (mean 1.00), b 0 (mean 0.00); a higher on 1, b higher on 0, ties 19;"
        ),
        "{}",
        lines[1]
    );

    let refused = run_in(
        &root,
        &[
            "bench", "--topic", TOPIC, "compare", "--a", &right, "--b", &payday,
        ],
        true,
    );
    assert_eq!(refused.code, 2);
    assert!(
        refused.stderr.contains("unknown question PDS-001"),
        "{}",
        refused.stderr
    );
}

/// The topic README counts what the keys hold and says no model has run.
#[test]
fn the_readme_counts_the_sources_and_says_no_model_has_run() {
    let text = std::fs::read_to_string(repo_root().join(DIR).join("README.md")).unwrap();
    assert!(text.contains("No model has been run on these questions"));
    assert!(text.contains("**Publisher conflict.**"));
    let keys = keys_json();
    for (row, kind) in [
        (
            "Sample data in Schedule 1 of the instrument",
            "instrument sample data",
        ),
        (
            "A worked example in Schedule 1 of the instrument",
            "instrument example",
        ),
        ("A worked example on an ATO page", "ATO example"),
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
