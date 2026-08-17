"""Verification pass tests: tamper each way, expect the named failure."""
from drdebits_build.build import write_outputs
from drdebits_build.verify import run_verify
from tests.test_build import make_repo


def sync(root):
    write_outputs(root, root)


def test_clean_repo_verifies(tmp_path):
    root = make_repo(tmp_path)
    sync(root)
    assert run_verify(root) == []


def test_tampered_output_names_file_and_src(tmp_path):
    root = make_repo(tmp_path)
    sync(root)
    guide = root / "drdebits.md"
    guide.write_text(guide.read_text(encoding="utf-8") + "tamper\n", encoding="utf-8", newline="\n")
    failures = run_verify(root)
    assert any("drdebits.md" in f and "edit src/" in f for f in failures)


def test_stamped_version_mismatch_detected(tmp_path):
    root = make_repo(tmp_path)
    sync(root)
    (root / "README.md").write_text("DrDebits `0.0.1` stub\n", encoding="utf-8", newline="\n")
    failures = run_verify(root)
    assert any("README.md" in f and "0.0.1" in f for f in failures)


def test_bad_yaml_becomes_message_not_traceback(tmp_path):
    root = make_repo(tmp_path)
    sync(root)
    bt = root / "src" / "data" / "behaviour-tests.yaml"
    bt.write_text(bt.read_text(encoding="utf-8").replace("HARD_STOP", "NOT_A_STATUS"), encoding="utf-8", newline="\n")
    failures = run_verify(root)
    assert any("NOT_A_STATUS" in f for f in failures)
