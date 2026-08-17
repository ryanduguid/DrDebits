"""CLI behaviour via main() return codes."""
from drdebits_build.__main__ import main
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
