"""One-off extractor: v0.2.0-draft committed files -> src/ tree.

Every parse asserts its own inverse: after parsing a table row or the
frontmatter, re-render with drdebits_build.render and assert equality with
the original text. A lossy parse dies immediately naming the offending
line, never producing wrong sources.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from drdebits_build import render  # noqa: E402

LINK_RE = re.compile(r"^\[(?P<title>.+)\]\((?P<url>https://[^)]+)\)$")


def yaml_quote(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def split_frontmatter(text: str):
    assert text.startswith("---\n"), "guide must start with frontmatter"
    end = text.index("\n---\n", 4)
    fm_lines = text[4:end].split("\n")
    body = text[end + len("\n---\n"):]
    fields = []
    for line in fm_lines:
        key, sep, value = line.partition(": ")
        assert sep, f"unparseable frontmatter line: {line!r}"
        fields.append((key, value))
    rendered = render.render_frontmatter(dict(fields))
    assert rendered == text[:end + len("\n---\n")], "frontmatter round-trip failed"
    return fields, body


def parse_table(lines, n_cols):
    """lines: header line, align line, then rows. Asserts render_table
    reproduces the exact original text."""
    headers = [c.strip() for c in lines[0].strip().strip("|").split("|")]
    aligns = lines[1].strip().strip("|").split("|")
    rows = []
    for line in lines[2:]:
        cells = line.strip().strip("|").split("|")
        assert len(cells) == n_cols, f"row has {len(cells)} cells, expected {n_cols}: {line!r}"
        rows.append([c.strip() for c in cells])
    rerendered = render.render_table(headers, aligns, rows)
    original = "\n".join(lines) + "\n"
    assert rerendered == original, (
        "table round-trip failed; first divergence:\n"
        + next((f"got:  {a!r}\nwant: {b!r}" for a, b in
                zip(rerendered.split("\n"), original.split("\n")) if a != b), "?"))
    return headers, aligns, rows


# --- extraction helpers (not part of the brief's verbatim core, but needed
# to drive main()) -----------------------------------------------------

def read_text_raw(path: Path) -> str:
    """Read a file as text with zero newline translation (Path.read_text's
    `newline` kwarg needs 3.13; this project is pinned to 3.12)."""
    with open(path, encoding="utf-8", newline="") as fh:
        return fh.read()


def slugify(heading: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", heading.lower()).strip("-")


def split_sections(body: str):
    """Split body into chunks at every line starting '## '. chunks[0] is
    everything before the first such line; each later chunk starts with its
    '## ' line and runs to (but not including) the next one, or EOF."""
    lines = body.splitlines(keepends=True)
    chunks: list[str] = []
    current: list[str] = []
    for line in lines:
        if line.startswith("## ") and current:
            chunks.append("".join(current))
            current = [line]
        else:
            current.append(line)
    if current:
        chunks.append("".join(current))
    return chunks


def find_table_start(lines, start=0):
    """lines: list of raw lines (with trailing \\n). Returns the index of
    the header row of the first table at or after `start`."""
    for i in range(start, len(lines) - 1):
        if lines[i].startswith("|") and lines[i].rstrip("\n").endswith("|") \
                and re.match(r"^\|[\s:|-]+\|\s*$", lines[i + 1]):
            return i
    raise AssertionError(f"no table found at or after line {start}")


def find_table_end(lines, start):
    """start: index of the header row. Returns the exclusive end index of
    the table (first line after the align row that doesn't start with '|')."""
    i = start + 2
    while i < len(lines) and lines[i].startswith("|"):
        i += 1
    return i


def table_span_to_lines(lines, start, end):
    span = lines[start:end]
    assert all(ln.endswith("\n") for ln in span), "table span must be newline-terminated"
    return [ln[:-1] for ln in span]


def print_between(marker: str, text: str):
    print(f"{marker}-BEGIN")
    print(text, end="")
    if not text.endswith("\n"):
        print()
    print(f"{marker}-END")


def write_yaml_rows(path: Path, top_key: str, rows: list, fields: tuple):
    lines = [f"{top_key}:"]
    for row in rows:
        for i, f in enumerate(fields):
            prefix = "  - " if i == 0 else "    "
            lines.append(f"{prefix}{f}: {yaml_quote(row[f])}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")


def main():
    root = Path(__file__).resolve().parents[3]
    written = 0

    # --- 1. metadata.yaml -------------------------------------------------
    guide_path = root / "drdebits.md"
    guide_text = read_text_raw(guide_path)
    fields, body = split_frontmatter(guide_text)
    meta = dict(fields)

    sha_path = root / "SHA256SUMS"
    sha_text = read_text_raw(sha_path)
    relpaths = []
    for line in sha_text.splitlines():
        if not line.strip():
            continue
        _hex, sep, relpath = line.partition(" *")
        assert sep, f"unparseable SHA256SUMS line: {line!r}"
        relpaths.append(relpath)
    checksum_files = "|".join(relpaths)

    metadata_fields = list(fields) + [("checksum_files", checksum_files)]
    metadata_yaml = root / "src" / "data" / "metadata.yaml"
    metadata_yaml.parent.mkdir(parents=True, exist_ok=True)
    lines = ["fields:"]
    for key, value in metadata_fields:
        lines.append(f"  - key: {key}")
        lines.append(f"    value: {yaml_quote(value)}")
    metadata_yaml.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    written += 1

    # --- 2. body -> fragments, holding out the change-log chunk -----------
    sections = split_sections(body)
    assert len(sections) >= 2, "expected a header chunk plus at least one '## ' section"
    changelog_chunk = sections[-1]
    assert changelog_chunk.startswith("## Change log\n\n"), \
        f"last section is not the change log: {changelog_chunk[:40]!r}"
    marker = meta["guide_end_marker"]
    suffix = "\n" + marker + "\n"
    assert changelog_chunk.endswith(suffix), \
        f"change-log chunk does not end with end marker: {changelog_chunk[-60:]!r}"
    table_text = changelog_chunk[len("## Change log\n\n"):-len(suffix)]
    assert table_text.endswith("\n"), "change-log table text must be newline-terminated"
    changelog_table_lines = table_text[:-1].split("\n")
    cl_headers, cl_aligns, cl_rows = parse_table(changelog_table_lines, 4)
    assert cl_headers == ["Version", "Date", "Status", "Change"], cl_headers

    fragment_chunks = sections[:-1]
    guide_dir = root / "src" / "guide"
    guide_dir.mkdir(parents=True, exist_ok=True)
    fragment_names = []
    for i, chunk in enumerate(fragment_chunks):
        if i == 0:
            name = "000-header.md"
        else:
            first_line = chunk[:chunk.index("\n")]
            assert first_line.startswith("## "), f"fragment {i} does not start with '## ': {first_line!r}"
            heading = first_line[3:]
            slug = slugify(heading)
            assert slug, f"empty slug for heading {heading!r}"
            name = f"{i * 10:03d}-{slug}.md"
        fragment_names.append(name)

    # Assert BEFORE writing anything: frontmatter + all fragments +
    # change-log chunk + marker line reassembles the original file
    # byte-for-byte.
    frontmatter_text = guide_text[:len(guide_text) - len(body)]
    rendered_changelog_chunk = "## Change log\n\n" + render.render_table(cl_headers, cl_aligns, cl_rows) + suffix
    reconstructed = frontmatter_text + "".join(fragment_chunks) + rendered_changelog_chunk
    assert reconstructed == guide_text, "full guide round-trip failed before any write"

    for name, chunk in zip(fragment_names, fragment_chunks):
        (guide_dir / name).write_text(chunk, encoding="utf-8", newline="\n")
        written += 1

    # --- 3. changelog.yaml --------------------------------------------------
    changelog_rows = [
        {"version": r[0], "date": r[1], "status": r[2], "change": r[3]} for r in cl_rows
    ]
    write_yaml_rows(root / "src" / "data" / "changelog.yaml", "entries", changelog_rows,
                     ("version", "date", "status", "change"))
    written += 1

    # --- 4. tpb-catalogue.md -> tpb-catalogue.yaml --------------------------
    cat_path = root / "reference" / "tpb-catalogue.md"
    cat_text = read_text_raw(cat_path)
    cat_lines = cat_text.splitlines(keepends=True)
    cat_start = find_table_start(cat_lines)
    cat_header_text = "".join(cat_lines[:cat_start])
    print_between("CATALOGUE-HEADER", cat_header_text)
    cat_end = find_table_end(cat_lines, cat_start)
    cat_table_lines = table_span_to_lines(cat_lines, cat_start, cat_end)
    cat_headers, cat_aligns, cat_rows = parse_table(cat_table_lines, 2)
    assert cat_end == len(cat_lines), "unexpected content after tpb-catalogue table"

    id_re = re.compile(r"TPB\(GS\)\s*(\d{2})/")
    catalogue_rows = []
    for title_cell, trigger in cat_rows:
        m = LINK_RE.match(title_cell)
        assert m, f"catalogue link parse failed: {title_cell!r}"
        title = m.group("title")
        url = m.group("url")
        idm = id_re.search(title)
        assert idm, f"could not derive id from title: {title!r}"
        catalogue_rows.append({"id": "GS" + idm.group(1), "title": title, "url": url, "trigger": trigger})
    write_yaml_rows(root / "src" / "data" / "tpb-catalogue.yaml", "entries", catalogue_rows,
                     ("id", "title", "url", "trigger"))
    written += 1

    # --- 5a. behaviour-tests.md -> behaviour-tests.yaml ---------------------
    bt_path = root / "tests" / "behaviour-tests.md"
    bt_text = read_text_raw(bt_path)
    bt_lines = bt_text.splitlines(keepends=True)
    bt_start = find_table_start(bt_lines)
    bt_header_text = "".join(bt_lines[:bt_start])
    print_between("BEHAVIOUR-HEADER", bt_header_text)
    bt_end = find_table_end(bt_lines, bt_start)
    bt_table_lines = table_span_to_lines(bt_lines, bt_start, bt_end)
    bt_headers, bt_aligns, bt_rows = parse_table(bt_table_lines, 5)
    assert bt_end == len(bt_lines), "unexpected content after behaviour-tests table"

    behaviour_rows = [
        {"id": r[0], "scenario": r[1], "expected_status": r[2], "required_behaviour": r[3],
         "side_effect_check": r[4]} for r in bt_rows
    ]
    write_yaml_rows(root / "src" / "data" / "behaviour-tests.yaml", "entries", behaviour_rows,
                     ("id", "scenario", "expected_status", "required_behaviour", "side_effect_check"))
    written += 1

    # --- 5b. apes-110-map.md -> apes-110-map.yaml ---------------------------
    apes_path = root / "reference" / "apes-110-map.md"
    apes_text = read_text_raw(apes_path)
    apes_lines = apes_text.splitlines(keepends=True)
    apes_start1 = find_table_start(apes_lines)
    apes_header_text = "".join(apes_lines[:apes_start1])
    print_between("APES-HEADER", apes_header_text)
    apes_end1 = find_table_end(apes_lines, apes_start1)
    apes_table1_lines = table_span_to_lines(apes_lines, apes_start1, apes_end1)
    apes_headers1, apes_aligns1, apes_rows1 = parse_table(apes_table1_lines, 2)

    apes_start2 = find_table_start(apes_lines, apes_end1)
    apes_between_text = "".join(apes_lines[apes_end1:apes_start2])
    print_between("APES-BETWEEN", apes_between_text)
    apes_end2 = find_table_end(apes_lines, apes_start2)
    apes_table2_lines = table_span_to_lines(apes_lines, apes_start2, apes_end2)
    apes_headers2, apes_aligns2, apes_rows2 = parse_table(apes_table2_lines, 2)
    assert apes_end2 == len(apes_lines), "unexpected content after apes-110-map second table"

    contexts_rows = [{"label": r[0], "value": r[1]} for r in apes_rows1]
    retrieval_rows = [{"label": r[0], "value": r[1]} for r in apes_rows2]
    apes_yaml = root / "src" / "data" / "apes-110-map.yaml"
    apes_yaml.parent.mkdir(parents=True, exist_ok=True)
    lines = ["contexts:"]
    for row in contexts_rows:
        lines.append(f"  - label: {yaml_quote(row['label'])}")
        lines.append(f"    value: {yaml_quote(row['value'])}")
    lines.append("retrieval_points:")
    for row in retrieval_rows:
        lines.append(f"  - label: {yaml_quote(row['label'])}")
        lines.append(f"    value: {yaml_quote(row['value'])}")
    apes_yaml.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    written += 1

    print(f"EXTRACTION OK: {written} files written")


if __name__ == "__main__":
    main()
