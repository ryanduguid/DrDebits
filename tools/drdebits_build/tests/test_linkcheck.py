"""URL collection is tested offline; liveness is CI-only."""
from drdebits_build.build import write_outputs
from drdebits_build.linkcheck import collect_urls
from tests.test_build import make_repo


def test_collect_urls_unique_ordered(tmp_path):
    root = make_repo(tmp_path)
    (root / "README.md").write_text(
        "See https://x.invalid/a and https://x.invalid/a again\n", encoding="utf-8", newline="\n")
    (root / "MAINTENANCE.md").write_text("None here\n", encoding="utf-8", newline="\n")
    write_outputs(root, root)
    urls = collect_urls(root)
    assert urls.count("https://x.invalid/a") == 1
