"""The evaluation harness: case export, result validation and the results table."""
import json

import pytest

from drdebits_build import evals
from drdebits_build.build import load_sources, write_outputs
from drdebits_build.model import ModelError
from drdebits_build.verify import run_verify
from tests.test_build import make_repo
from tests.test_cli import TODAY

GOOD = {"model": "example-model", "run_date": "2026-02-01", "guide_version": "0.9.9-test",
        "runner": "A Person", "results": {"A-001": "pass"}}


def write_result(root, name="2026-02-01-example-model.json", **overrides):
    data = {**GOOD, **overrides}
    directory = root / "evals" / "results"
    directory.mkdir(parents=True, exist_ok=True)
    (directory / name).write_text(json.dumps(data), encoding="utf-8")


def test_cases_export_carries_every_behaviour_test(tmp_path):
    s = load_sources(make_repo(tmp_path))
    payload = json.loads(evals.build_cases(s))
    assert payload["guide_version"] == "0.9.9-test"
    assert [c["id"] for c in payload["cases"]] == ["A-001"]
    assert set(payload["cases"][0]) == {
        "id", "scenario", "expected_status", "required_behaviour", "side_effect_check"}


def test_results_table_with_no_runs_lists_the_cases(tmp_path):
    root = make_repo(tmp_path)
    out = evals.build_results_md(root, load_sources(root))
    assert "No runs recorded yet" in out
    assert "| A-001 | HARD_STOP |" in out
    assert "| Passed | of 1 |" in out


def test_results_table_has_one_column_per_run_and_a_total(tmp_path):
    root = make_repo(tmp_path)
    write_result(root)
    write_result(root, name="2026-02-02-other-model.json", model="other-model",
                 run_date="2026-02-02", results={"A-001": "fail"})
    out = evals.build_results_md(root, load_sources(root))
    assert "| ID | Expected status | example-model (2026-02-01) | other-model (2026-02-02) |" in out
    assert "| A-001 | HARD_STOP | pass | fail |" in out
    assert "| Passed | of 1 | 1 | 0 |" in out
    assert "No runs recorded" not in out


@pytest.mark.parametrize("name, overrides, message", [
    ("2026-02-01-example-model.json", {"transcript": "..."}, "transcripts do not belong"),
    ("2026-02-01-example-model.json", {"results": {}}, "missing=\\['A-001'\\]"),
    ("2026-02-01-example-model.json", {"results": {"A-001": "pass", "Z-9": "pass"}},
     "unexpected=\\['Z-9'\\]"),
    ("2026-02-01-example-model.json", {"results": {"A-001": "PASS"}}, "must be pass or fail"),
    ("2026-02-01-example-model.json", {"model": " "}, "model must be a non-empty string"),
    ("2026-02-01-example-model.json", {"run_date": "1 Feb 2026"}, "ISO date"),
    ("2026-02-02-example-model.json", {}, "file name date 2026-02-02 != run_date"),
    ("notes.json", {}, "YYYY-MM-DD-<slug>.json"),
])
def test_malformed_results_are_rejected(tmp_path, name, overrides, message):
    root = make_repo(tmp_path)
    write_result(root, name=name, **overrides)
    with pytest.raises(ModelError, match=message):
        evals.load_results(root, load_sources(root))


def test_build_writes_and_verify_checks_the_eval_outputs(tmp_path, monkeypatch):
    from drdebits_build import verify as verify_module
    monkeypatch.setattr(verify_module, "_verification_date", lambda: TODAY)
    root = make_repo(tmp_path)
    write_result(root)
    written = write_outputs(root, root)
    assert evals.CASES_FILE in written and evals.RESULTS_FILE in written
    assert run_verify(root) == []
    (root / evals.RESULTS_FILE).write_text("stale\n", encoding="utf-8")
    assert any(evals.RESULTS_FILE in f and "differs" in f for f in run_verify(root))
    write_result(root, results={"A-001": "maybe"})
    assert any("cannot rebuild" in f for f in run_verify(root))
