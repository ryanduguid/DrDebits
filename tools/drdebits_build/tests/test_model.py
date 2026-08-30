"""Tests for YAML source loading and validation."""
import textwrap
import pytest
from drdebits_build.model import (
    ModelError, ALLOWED_STATUSES, load_metadata, load_catalogue,
    load_behaviour_tests, load_changelog, load_apes_map, validate_counts,
    validate_required_metadata,
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


def test_catalogue_ids_must_be_gs_numbered_and_ascending(tmp_path):
    """The catalogue header and verify both publish the first and last rows as
    the authoritative "GSxx to GSyy" range. Uniqueness alone does not make that
    claim true: a malformed or reordered catalogue would let both agree on an
    inverted or nonsensical range, so the ids must be GS-numbered and in
    ascending order at load."""
    def catalogue(name, ids):
        rows = "".join(
            f'  - id: "{i}"\n    title: "T"\n'
            f'    url: "https://example.invalid/{n}"\n    trigger: "x"\n'
            for n, i in enumerate(ids))
        return write(tmp_path, name, "entries:\n" + rows)

    assert [r["id"] for r in load_catalogue(catalogue("ok.yaml", ["GS01", "GS02"]))] \
        == ["GS01", "GS02"]
    # Withdrawn statements leave gaps; a gap is still ascending.
    assert len(load_catalogue(catalogue("gap.yaml", ["GS01", "GS09", "GS54"]))) == 3

    for name, ids, match in (
        ("desc.yaml", ["GS02", "GS01"], "does not come after"),
        ("swap.yaml", ["GS55", "GS02", "GS01"], "does not come after"),
        ("dupnum.yaml", ["GS01", "GS1"], "does not come after"),
        ("bad.yaml", ["GS01", "GS02a"], "Guidance Statement id"),
        ("nope.yaml", ["GS01", "TPB03"], "Guidance Statement id"),
        ("bare.yaml", ["01", "02"], "Guidance Statement id"),
    ):
        with pytest.raises(ModelError, match=match):
            load_catalogue(catalogue(name, ids))


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


def test_missing_source_file_becomes_model_error(tmp_path):
    with pytest.raises(ModelError, match="cannot read source file"):
        load_metadata(tmp_path / "does-not-exist.yaml")


def test_apes_map_missing_key_names_the_key(tmp_path):
    p = write(tmp_path, "am.yaml", """\
        retrieval_points:
          - label: "Scope"
            value: "R1.2"
    """)
    with pytest.raises(ModelError, match="'contexts'"):
        load_apes_map(p)


def test_table_bound_values_reject_pipes_and_newlines(tmp_path):
    tmpl = """\
        entries:
          - id: "GS01"
            title: "T"
            url: "https://x.invalid/a"
            trigger: {trigger}
    """
    with pytest.raises(ModelError, match="break the rendered Markdown table"):
        load_catalogue(write(tmp_path, "p.yaml", tmpl.format(trigger='"either A | B"')))
    with pytest.raises(ModelError, match="contains a newline"):
        load_catalogue(write(tmp_path, "n.yaml", tmpl.format(trigger='"a\\nb"')))
    with pytest.raises(ModelError, match="contains a newline"):
        load_catalogue(write(tmp_path, "r.yaml", tmpl.format(trigger='"a\\rb"')))
    # Metadata keeps the '|' exemption (checksum_files uses it as a separator)
    # but newlines stay banned everywhere - one would inject an extra
    # frontmatter line into the guide.
    m = write(tmp_path, "m.yaml", """\
        fields:
          - key: checksum_files
            value: "LICENSE|README.md"
    """)
    assert load_metadata(m)["checksum_files"] == "LICENSE|README.md"
    with pytest.raises(ModelError, match="contains a newline"):
        load_metadata(write(tmp_path, "mi.yaml", """\
            fields:
              - key: jurisdiction
                value: "AU\\nstatus: injected"
        """))


def test_required_metadata_keys_enforced():
    meta = {k: "x" for k in (
        "guide_version", "release_tag", "guide_end_marker", "sources_checked_at",
        "review_due", "tpb_guidance_statement_count", "tpb_library_index_count",
        "checksum_files")}
    validate_required_metadata(meta)
    del meta["guide_version"]
    with pytest.raises(ModelError, match="guide_version"):
        validate_required_metadata(meta)


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
