"""Acceptance: the real repo verifies clean; hand edits fail; version bump propagates."""
import shutil
from pathlib import Path
from drdebits_build.__main__ import main
from drdebits_build.build import find_root

REAL = find_root(Path(__file__).resolve())
COPY_ITEMS = ["src", "drdebits.md", "reference", "tests", "SHA256SUMS",
              "README.md", "MAINTENANCE.md", "LICENSE"]


def clone_repo(tmp_path):
    for item in COPY_ITEMS:
        src = REAL / item
        dst = tmp_path / item
        if src.is_dir():
            shutil.copytree(src, dst)
        else:
            shutil.copy2(src, dst)
    return tmp_path


def test_real_repo_verifies(tmp_path):
    assert main(["verify", "--root", str(clone_repo(tmp_path))]) == 0


def test_hand_edit_fails(tmp_path):
    root = clone_repo(tmp_path)
    p = root / "reference" / "tpb-catalogue.md"
    p.write_text(p.read_text(encoding="utf-8").replace("TPB", "TBP", 1), encoding="utf-8", newline="\n")
    assert main(["verify", "--root", str(root)]) == 1


def test_version_bump_propagates(tmp_path):
    """The true bump procedure (MAINTENANCE.md step 8): metadata.yaml, the guide's
    own header version line, and the changelog's newest entry all move together."""
    root = clone_repo(tmp_path)
    for rel in ("src/data/metadata.yaml", "src/guide/000-header.md", "src/data/changelog.yaml"):
        p = root / rel
        text = p.read_text(encoding="utf-8").replace("0.2.0-draft", "0.2.1-draft")
        p.write_text(text, encoding="utf-8", newline="\n")
    assert main(["build", "--root", str(root)]) == 0
    assert main(["verify", "--root", str(root)]) == 0
    assert "0.2.1-draft" in (root / "README.md").read_text(encoding="utf-8")
    assert (root / "drdebits.md").read_text(encoding="utf-8").rstrip("\n").endswith("0.2.1-draft")
