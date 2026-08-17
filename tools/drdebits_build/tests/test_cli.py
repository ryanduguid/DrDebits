"""CLI behaviour via main() return codes."""
from drdebits_build.__main__ import main
from drdebits_build.verify import run_verify
from tests.test_build import make_repo


def test_build_then_verify_roundtrip(tmp_path, capsys):
    root = make_repo(tmp_path)
    assert main(["build", "--root", str(root)]) == 0
    assert main(["verify", "--root", str(root)]) == 0
    assert "verify: OK" in capsys.readouterr().out


def test_verify_failure_exit_code(tmp_path):
    root = make_repo(tmp_path)
    main(["build", "--root", str(root)])
    (root / "drdebits.md").write_text("tampered", encoding="utf-8", newline="\n")
    assert main(["verify", "--root", str(root)]) == 1


def test_build_stamps_readme_and_maintenance_before_checksums(tmp_path):
    root = make_repo(tmp_path)
    (root / "README.md").write_text("DrDebits `0.0.1` stub\n", encoding="utf-8", newline="\n")
    (root / "MAINTENANCE.md").write_text("Protocol `0.0.1`.\n", encoding="utf-8", newline="\n")
    assert main(["build", "--root", str(root)]) == 0
    assert "0.9.9-test" in (root / "README.md").read_text(encoding="utf-8")
    assert "0.9.9-test" in (root / "MAINTENANCE.md").read_text(encoding="utf-8")
    assert run_verify(root) == []
