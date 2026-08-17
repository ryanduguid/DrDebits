"""Tests for YAML source loading and validation."""
import textwrap
import pytest
from drdebits_build.model import (
    ModelError, ALLOWED_STATUSES, load_metadata, load_catalogue,
    load_behaviour_tests, load_changelog, load_apes_map, validate_counts,
)


def write(tmp_path, name, content):
    p = tmp_path / name
    p.write_text(textwrap.dedent(content), encoding="utf-8", newline="\n")
    return p


def test_metadata_preserves_order_and_strings(tmp_path):
    p = write(tmp_path, "metadata.yaml", """\
        fields:
          - key: title
            value: "DrDebits"
          - key: sources_checked_at
            value: "2026-08-16T00:00:00+10:00"
    """)
    meta = load_metadata(p)
    assert list(meta.keys()) == ["title", "sources_checked_at"]
    assert meta["sources_checked_at"] == "2026-08-16T00:00:00+10:00"
    assert all(isinstance(v, str) for v in meta.values())


def test_catalogue_rejects_duplicate_ids_and_http(tmp_path):
    good = """\
        entries:
          - id: "GS01"
            title: "T1"
            url: "https://example.invalid/a"
            trigger: "x"
    """
    rows = load_catalogue(write(tmp_path, "c1.yaml", good))
    assert rows[0]["id"] == "GS01"
    dup = good + """\
          - id: "GS01"
            title: "T2"
            url: "https://example.invalid/b"
            trigger: "y"
    """
    with pytest.raises(ModelError):
        load_catalogue(write(tmp_path, "c2.yaml", dup))
    with pytest.raises(ModelError):
        load_catalogue(write(tmp_path, "c3.yaml", good.replace("https://", "http://")))


def test_behaviour_status_enum_and_empty_fields(tmp_path):
    tmpl = """\
        entries:
          - id: "AUTH-001"
            scenario: "s"
            expected_status: "{status}"
            required_behaviour: "r"
            side_effect_check: "c"
    """
    ok = load_behaviour_tests(write(tmp_path, "b1.yaml", tmpl.format(status="HARD_STOP")))
    assert ok[0]["expected_status"] == "HARD_STOP"
    assert "Low impact — proportionate answer" in ALLOWED_STATUSES
    with pytest.raises(ModelError):
        load_behaviour_tests(write(tmp_path, "b2.yaml", tmpl.format(status="MAYBE")))
    with pytest.raises(ModelError):
        load_behaviour_tests(write(tmp_path, "b3.yaml", tmpl.format(status="HARD_STOP").replace('"s"', '""')))


def test_counts_must_match(tmp_path):
    meta = {"tpb_guidance_statement_count": "2", "tpb_library_index_count": "2"}
    cat = [{"id": "a", "title": "t", "url": "https://x.invalid", "trigger": "g"}]
    with pytest.raises(ModelError):
        validate_counts(meta, cat)
    validate_counts(meta, cat + [{"id": "b", "title": "t", "url": "https://y.invalid", "trigger": "g"}])


def test_changelog_and_apes_map_shapes(tmp_path):
    cl = load_changelog(write(tmp_path, "cl.yaml", """\
        entries:
          - version: "0.2.0-draft"
            date: "2026-08-16"
            status: "Published draft"
            change: "words"
    """))
    assert cl[0]["version"] == "0.2.0-draft"
    am = load_apes_map(write(tmp_path, "am.yaml", """\
        contexts:
          - label: "All members"
            value: "Part 1"
        retrieval_points:
          - label: "Scope"
            value: "R1.2"
    """))
    assert am["contexts"][0]["label"] == "All members"
