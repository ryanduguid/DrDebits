"""Tests for markdown rendering primitives."""
from drdebits_build.render import render_frontmatter, render_table, render_link, render_end_marker


def test_frontmatter_verbatim_values_in_order():
    meta = {"title": "DrDebits", "apes_110_pdf_sha256": "B6937B93"}
    assert render_frontmatter(meta) == "---\ntitle: DrDebits\napes_110_pdf_sha256: B6937B93\n---\n"


def test_table_single_space_padding_and_alignment_tokens():
    out = render_table(["ID", "Scenario"], ["---", "---"],
                       [["AUTH-001", "says x"], ["INJ-001", "says y"]])
    assert out == (
        "| ID | Scenario |\n"
        "|---|---|\n"
        "| AUTH-001 | says x |\n"
        "| INJ-001 | says y |\n"
    )


def test_link_and_marker():
    assert render_link("T", "https://x.invalid/a") == "[T](https://x.invalid/a)"
    assert render_end_marker({"guide_end_marker": "DRDEBITS-END-v0.2.0-draft"}) == "DRDEBITS-END-v0.2.0-draft"
