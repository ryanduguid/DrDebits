// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! `inspect-export` writes every behaviour case as an Inspect sample, with
//! the same text as `evals/cases.json`.

mod common;

use common::{fixture, repo_root, run_in};
use drdebits::pyjson::{self, Json};

fn text<'a>(item: &'a Json, field: &str) -> &'a str {
    item.get(field).and_then(Json::as_str).unwrap_or_default()
}

#[test]
fn every_case_becomes_one_sample_with_its_prompt_rubric_and_digests() {
    let root = repo_root();
    let cases =
        pyjson::loads(&std::fs::read_to_string(root.join("evals/cases.json")).unwrap()).unwrap();
    let Some(Json::Array(cases)) = cases.get("cases").cloned() else {
        panic!("cases.json has no cases")
    };
    let digests = run_in(&root, &["digests"], true).stdout;
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("drdebits.jsonl");
    let ran = run_in(
        &root,
        &["inspect-export", "--out", out.to_str().unwrap()],
        true,
    );
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let body = std::fs::read_to_string(&out).unwrap();
    let samples: Vec<Json> = body.lines().map(|l| pyjson::loads(l).unwrap()).collect();
    assert!(body.ends_with('\n'));
    assert_eq!(samples.len(), cases.len());

    let mut frozen = 0;
    for (sample, case) in samples.iter().zip(&cases) {
        let id = text(case, "id");
        assert_eq!(text(sample, "id"), id);
        let metadata = sample.get("metadata").unwrap();
        let input = sample.get("input").unwrap();
        match (case.get("prompt"), case.get("system_instruction")) {
            (Some(prompt), Some(system)) => {
                let Json::Array(messages) = input else {
                    panic!("{id}: a system instruction needs a message list")
                };
                assert_eq!(messages.len(), 2, "{id}");
                assert_eq!(text(&messages[0], "role"), "system");
                assert_eq!(messages[0].get("content"), Some(system));
                assert_eq!(text(&messages[1], "role"), "user");
                assert_eq!(messages[1].get("content"), Some(prompt));
            }
            (Some(prompt), None) => assert_eq!(input, prompt, "{id}"),
            (None, _) => assert_eq!(input.as_str(), Some(text(case, "scenario")), "{id}"),
        }
        let source = if case.get("prompt").is_some() {
            frozen += 1;
            "prompt"
        } else {
            "scenario"
        };
        assert_eq!(text(metadata, "input"), source, "{id}");
        assert_eq!(
            text(sample, "target"),
            format!(
                "expected_status: {}\nrequired_behaviour: {}\nside_effect_check: {}",
                text(case, "expected_status"),
                text(case, "required_behaviour"),
                text(case, "side_effect_check")
            )
        );
        for field in [
            "scenario",
            "expected_status",
            "required_behaviour",
            "side_effect_check",
            "prohibited_anywhere",
        ] {
            assert_eq!(metadata.get(field), case.get(field), "{id} {field}");
        }
        for line in digests.lines() {
            let (key, value) = line.split_once(": ").unwrap();
            assert_eq!(text(metadata, key), value, "{id} {key}");
        }
        assert!(text(metadata, "benchmark_canary").starts_with("DRDEBITS-BENCHMARK-CANARY "));
        assert!(!text(metadata, "guide_version").is_empty());
    }
    assert!(frozen > 0 && frozen < cases.len());
    assert!(cases.iter().any(|c| c.get("system_instruction").is_some()));
    assert_eq!(
        ran.stdout,
        format!(
            "inspect-export: wrote {} samples to {}: {frozen} with a frozen prompt, {} with only a scenario\n",
            cases.len(),
            out.display(),
            cases.len() - frozen
        )
    );
}

#[test]
fn an_unwritable_file_or_a_missing_tree_stops_the_export() {
    let root = repo_root();
    let dir = tempfile::tempdir().unwrap();
    let nowhere = dir.path().join("missing").join("drdebits.jsonl");
    let refused = run_in(
        &root,
        &["inspect-export", "--out", nowhere.to_str().unwrap()],
        true,
    );
    assert_eq!(refused.code, 2);
    assert!(
        refused.stderr.starts_with("inspect-export: "),
        "{}",
        refused.stderr
    );
    assert!(!nowhere.exists());

    let empty = tempfile::tempdir().unwrap();
    let out = dir.path().join("drdebits.jsonl");
    let lost = run_in(
        empty.path(),
        &["inspect-export", "--out", out.to_str().unwrap()],
        true,
    );
    assert_eq!(lost.code, 2);
    assert!(
        lost.stderr.starts_with("inspect-export: "),
        "{}",
        lost.stderr
    );
    assert!(!out.exists());
}

/// No published case breaks a line, so a fixture does: the export keeps one
/// sample a line and gives the prompt back exactly, breaks and indents too.
#[test]
fn a_prompt_with_line_breaks_and_indents_stays_one_line_and_exact() {
    let (dir, _) = fixture();
    let source = dir.path().join("src/data/behaviour-tests.yaml");
    let yaml = std::fs::read_to_string(&source).unwrap();
    let scenario = "    scenario: \"s\"\n";
    assert!(yaml.contains(scenario), "{yaml}");
    let with_prompt = format!("{scenario}    prompt: \"  line one\\n  line two\\r\\n\\tend\"\n");
    std::fs::write(&source, yaml.replacen(scenario, &with_prompt, 1)).unwrap();
    let out = dir.path().join("out.jsonl");
    let ran = run_in(
        dir.path(),
        &["inspect-export", "--out", out.to_str().unwrap()],
        true,
    );
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let body = std::fs::read_to_string(&out).unwrap();
    assert_eq!(body.matches('\n').count(), 1, "{body}");
    assert!(body.ends_with('\n'));
    let sample = pyjson::loads(body.trim_end()).unwrap();
    assert_eq!(text(&sample, "input"), "  line one\n  line two\r\n\tend");
    assert_eq!(text(sample.get("metadata").unwrap(), "input"), "prompt");
}
