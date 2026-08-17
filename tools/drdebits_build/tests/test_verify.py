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


# Regression tests for fix 1: stamped-file presence and version coverage
def test_missing_stamped_file_detected(tmp_path):
    """Fix 1 (CRITICAL): missing stamped file should be detected."""
    root = make_repo(tmp_path)
    sync(root)
    (root / "MAINTENANCE.md").unlink()
    failures = run_verify(root)
    assert any("MAINTENANCE.md: missing" in f for f in failures)


def test_stripped_version_stamp_detected(tmp_path):
    """Fix 1 (CRITICAL): stripped version stamp (zero matches) should be detected."""
    root = make_repo(tmp_path)
    sync(root)
    (root / "MAINTENANCE.md").write_text("Protocol for maintenance.\n", encoding="utf-8", newline="\n")
    failures = run_verify(root)
    assert any("MAINTENANCE.md: no version stamp found" in f for f in failures)


# Regression test for fix 2: end-marker check on committed file
def test_committed_end_marker_drift_detected(tmp_path):
    """Fix 2 (IMPORTANT): appended line to committed drdebits.md should be detected as end-marker drift."""
    root = make_repo(tmp_path)
    sync(root)
    guide = root / "drdebits.md"
    guide.write_text(guide.read_text(encoding="utf-8") + "extra line\n", encoding="utf-8", newline="\n")
    failures = run_verify(root)
    assert any("end marker:" in f for f in failures)


# Regression test for fix 3: checksum rebuild crash
def test_missing_checksum_file_becomes_message_not_crash(tmp_path):
    """Fix 3 (IMPORTANT): missing checksum_files member should produce message, not crash."""
    root = make_repo(tmp_path)
    sync(root)
    (root / "README.md").unlink()  # README is in checksum_files
    failures = run_verify(root)
    # Should have messages, not raise; check that we got failures without exception
    assert isinstance(failures, list)
    assert any("README.md: missing" in f for f in failures)


# Regression tests for the final-review fix: the guide version lives in four
# more places than the stamped files, and a version bump that misses one of
# them produced a build that verified clean but disagreed with itself.

def test_header_version_line_mismatch_detected(tmp_path):
    """Final-review fix (a): drdebits.md's own header version line must match guide_version."""
    root = make_repo(tmp_path)
    header = root / "src" / "guide" / "000-header.md"
    header.write_text(
        header.read_text(encoding="utf-8").replace("0.9.9-test", "0.9.8-test"),
        encoding="utf-8", newline="\n")
    sync(root)  # rebuild so the committed guide matches the (stale) src output
    failures = run_verify(root)
    assert any("drdebits.md: header version line does not match guide_version" in f for f in failures)


def test_release_tag_mismatch_detected(tmp_path):
    """Final-review fix (b): metadata release_tag must equal 'v' + guide_version."""
    root = make_repo(tmp_path)
    meta = root / "src" / "data" / "metadata.yaml"
    meta.write_text(
        meta.read_text(encoding="utf-8").replace('value: "v0.9.9-test"', 'value: "v9.9.9-test"'),
        encoding="utf-8", newline="\n")
    sync(root)
    failures = run_verify(root)
    assert any("metadata: release_tag v9.9.9-test != v0.9.9-test" in f for f in failures)


def test_guide_end_marker_metadata_mismatch_detected(tmp_path):
    """Final-review fix (c): metadata guide_end_marker must equal 'DRDEBITS-END-' + release_tag."""
    root = make_repo(tmp_path)
    meta = root / "src" / "data" / "metadata.yaml"
    meta.write_text(
        meta.read_text(encoding="utf-8").replace(
            'value: "DRDEBITS-END-v0.9.9-test"', 'value: "DRDEBITS-END-BOGUS"'),
        encoding="utf-8", newline="\n")
    sync(root)
    failures = run_verify(root)
    assert any(
        "metadata: guide_end_marker DRDEBITS-END-BOGUS != DRDEBITS-END-v0.9.9-test" in f
        for f in failures)


def test_changelog_newest_entry_mismatch_detected(tmp_path):
    """Final-review fix (d): the newest (first) changelog entry must match guide_version."""
    root = make_repo(tmp_path)
    changelog = root / "src" / "data" / "changelog.yaml"
    changelog.write_text(
        changelog.read_text(encoding="utf-8").replace('version: "0.9.9-test"', 'version: "0.9.8-test"'),
        encoding="utf-8", newline="\n")
    sync(root)
    failures = run_verify(root)
    assert any("changelog: newest entry 0.9.8-test != 0.9.9-test" in f for f in failures)
