# DrDebits Build-and-Verification Module Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compile DrDebits from structured sources deterministically and verify the committed runtime files byte-for-byte, with CI enforcement and a report-only link checker.

**Architecture:** A uv-locked Python package under `tools/drdebits_build/` loads YAML data plus ordered prose fragments from `src/`, renders the five fully generated runtime files, stamps the two hand-maintained files, and verifies by rebuilding to a temp dir and byte-comparing. A one-off, self-checking extractor migrates the frozen v0.2.0-draft content into `src/`; the migration gate is `verify` exiting 0 against the untouched runtime files.

**Tech Stack:** Python 3.12, PyYAML 6.0.2, pytest 8.3.3, uv 0.12.0, GitHub Actions (pins copied from the release-policy conventions).

## Global Constraints

- Guide content is FROZEN: no byte of `drdebits.md`, `reference/*.md`, `tests/behaviour-tests.md`, `README.md`, `MAINTENANCE.md`, `LICENSE`, `SHA256SUMS` changes until Task 11 (maintenance-protocol rewrite), and only MAINTENANCE.md/README.md/SHA256SUMS change there.
- Migration gate: after Task 10, `uv run --project tools/drdebits_build python -m drdebits_build verify` exits 0 with the pre-existing runtime files untouched. Nothing after Task 10 lands until this holds.
- Determinism: no output byte derives from the clock, environment or randomness. All writes use LF and UTF-8 (no BOM).
- Repo root discovery: walk up from cwd until a directory containing both `drdebits.md` and `src/` is found; `--root PATH` overrides.
- Behaviour-test `expected_status` allowed set, exactly: `HARD_STOP`, `ESCALATE`, `NEEDS_FACTS`, `PROCEED_DRAFT_ONLY`, and the literal string `Low impact — proportionate answer` (frozen content keeps its em dash; the no-em-dash rule binds new prose only).
- YAML sources quote every scalar that YAML would otherwise coerce (timestamps like `2026-08-16T00:00:00+10:00`, digests, version strings); loaders use `yaml.safe_load` and treat all leaf values as strings.
- Render frontmatter and tables by explicit line construction; `yaml.dump` loses round-trip fidelity.
- Git identity: `Ryan Duguid <ryan@duguid.com.au>` (already configured repo-locally). - No em dashes in newly written prose (docs, comments, commit messages). Frozen content and its YAML copies are exempt.
- Work happens in `C:/Users/-/AppData/Local/Temp/claude/C--/d73ac32a-b268-474f-a1ba-c058181baa3f/scratchpad/DrDebits` on `main`. Nothing is pushed until Task 14's confirm gate.
- Windows note: run tests via `uv run --project tools/drdebits_build --locked --extra dev pytest tools/drdebits_build/tests`; `python` not `python3`.

---

### Task 1: Package scaffold

**Files:**
- Create: `tools/drdebits_build/pyproject.toml`
- Create: `tools/drdebits_build/drdebits_build/__init__.py`
- Create: `tools/drdebits_build/tests/__init__.py`
- Create: `tools/drdebits_build/uv.lock` (generated)
- Modify: `.gitattributes` (add `*.py text eol=lf` and `*.yaml text eol=lf` lines)

**Interfaces:**
- Produces: the project layout every later task assumes; `uv run --project tools/drdebits_build --locked --extra dev pytest tools/drdebits_build/tests` as the test command.

- [ ] **Step 1: Write pyproject.toml**

```toml
[project]
name = "drdebits-build"
version = "0.1.0"
description = "Deterministic build and verification tooling for the DrDebits guide"
requires-python = ">=3.12"
dependencies = ["pyyaml==6.0.2"]

[project.optional-dependencies]
dev = ["pytest==8.3.3"]

[build-system]
requires = ["setuptools==84.0.0"]
build-backend = "setuptools.build_meta"

[tool.setuptools.packages.find]
include = ["drdebits_build*"]

[tool.pytest.ini_options]
testpaths = ["tests"]
```

- [ ] **Step 2: Create the two `__init__.py` files**, each containing a single module docstring line (e.g. `"""DrDebits build tooling."""`).

- [ ] **Step 3: Lock and smoke-run**

```bash
cd "C:/Users/-/AppData/Local/Temp/claude/C--/d73ac32a-b268-474f-a1ba-c058181baa3f/scratchpad/DrDebits" && uv lock --project tools/drdebits_build && uv run --project tools/drdebits_build --locked --extra dev pytest tools/drdebits_build/tests
```

Expected: lockfile written; pytest reports "no tests ran" and exits 5, which is fine at this step.

- [ ] **Step 4: Append to .gitattributes** the two lines `*.py text eol=lf` and `*.yaml text eol=lf`.

- [ ] **Step 5: Commit**

```bash
git add tools/ .gitattributes
git commit -m "Scaffold the drdebits-build package (uv-locked, pyyaml, pytest)"
```

---

### Task 2: Data loading and validation (model.py)

**Files:**
- Create: `tools/drdebits_build/drdebits_build/model.py`
- Test: `tools/drdebits_build/tests/test_model.py`

**Interfaces:**
- Produces (exact signatures, all raising `ModelError(str)` on violation):
  - `load_metadata(path) -> dict[str, str]` (ordered; every value a string)
  - `load_catalogue(path) -> list[dict]` (keys `id`, `title`, `url`, `trigger`)
  - `load_behaviour_tests(path) -> list[dict]` (keys `id`, `scenario`, `expected_status`, `required_behaviour`, `side_effect_check`)
  - `load_apes_map(path) -> dict` (keys `contexts`, `retrieval_points`, each a list of 2-item dicts `label`, `value`)
  - `load_changelog(path) -> list[dict]` (keys `version`, `date`, `status`, `change`)
  - `ALLOWED_STATUSES` frozenset per Global Constraints.
  - `validate_counts(metadata, catalogue)` : both metadata count fields equal the catalogue row count.

- [ ] **Step 1: Write the failing tests**

```python
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
```

- [ ] **Step 2: Run to verify failure**

```bash
uv run --project tools/drdebits_build --locked --extra dev pytest tools/drdebits_build/tests/test_model.py -q
```

Expected: collection error, `drdebits_build.model` missing.

- [ ] **Step 3: Implement model.py**

```python
"""Load and validate the YAML sources of truth. All leaf values are strings."""
from __future__ import annotations

import yaml

ALLOWED_STATUSES = frozenset({
    "HARD_STOP", "ESCALATE", "NEEDS_FACTS", "PROCEED_DRAFT_ONLY",
    "Low impact — proportionate answer",
})


class ModelError(Exception):
    pass


def _read(path):
    with open(path, encoding="utf-8") as fh:
        data = yaml.safe_load(fh)
    if not isinstance(data, dict):
        raise ModelError(f"{path}: top level must be a mapping")
    return data


def _require_str(path, where, value):
    if not isinstance(value, str) or value == "":
        raise ModelError(f"{path}: {where} must be a non-empty string, got {value!r}")
    return value


def _rows(path, data, key, fields):
    entries = data.get(key)
    if not isinstance(entries, list) or not entries:
        raise ModelError(f"{path}: '{key}' must be a non-empty list")
    out = []
    for i, row in enumerate(entries):
        if not isinstance(row, dict) or set(row) != set(fields):
            raise ModelError(f"{path}: entry {i} must have exactly fields {fields}")
        out.append({f: _require_str(path, f"entry {i} field '{f}'", row[f]) for f in fields})
    return out


def _unique_ids(path, rows):
    ids = [r["id"] for r in rows]
    if len(ids) != len(set(ids)):
        raise ModelError(f"{path}: duplicate ids")


def load_metadata(path):
    rows = _rows(path, _read(path), "fields", ("key", "value"))
    keys = [r["key"] for r in rows]
    if len(keys) != len(set(keys)):
        raise ModelError(f"{path}: duplicate metadata keys")
    return {r["key"]: r["value"] for r in rows}


def load_catalogue(path):
    rows = _rows(path, _read(path), "entries", ("id", "title", "url", "trigger"))
    _unique_ids(path, rows)
    for r in rows:
        if not r["url"].startswith("https://"):
            raise ModelError(f"{path}: url for {r['id']} must be https")
    return rows


def load_behaviour_tests(path):
    rows = _rows(path, _read(path), "entries",
                 ("id", "scenario", "expected_status", "required_behaviour", "side_effect_check"))
    _unique_ids(path, rows)
    for r in rows:
        if r["expected_status"] not in ALLOWED_STATUSES:
            raise ModelError(f"{path}: {r['id']} has unknown status {r['expected_status']!r}")
    return rows


def load_changelog(path):
    return _rows(path, _read(path), "entries", ("version", "date", "status", "change"))


def load_apes_map(path):
    data = _read(path)
    out = {}
    for key in ("contexts", "retrieval_points"):
        out[key] = _rows(path, {"entries": data.get(key)}, "entries", ("label", "value"))
    return out


def validate_counts(metadata, catalogue):
    n = str(len(catalogue))
    for field in ("tpb_guidance_statement_count", "tpb_library_index_count"):
        if metadata.get(field) != n:
            raise ModelError(f"metadata {field}={metadata.get(field)!r} but catalogue has {n} rows")
```

- [ ] **Step 4: Run to verify pass** (same command as Step 2)

- [ ] **Step 5: Commit** (`git add tools/`, message `Add YAML source loading and validation`)

---

### Task 3: Rendering (render.py)

**Files:**
- Create: `tools/drdebits_build/drdebits_build/render.py`
- Test: `tools/drdebits_build/tests/test_render.py`

**Interfaces:**
- Produces:
  - `render_frontmatter(meta: dict[str, str]) -> str` : `---\n` + one `key: value\n` per entry in order + `---\n`. Values written verbatim (the stored string IS the file text after `key: `).
  - `render_table(headers: list[str], aligns: list[str], rows: list[list[str]]) -> str` : pipe table, one space padding, alignment row tokens passed verbatim (e.g. `---`, `---:`).
  - `render_link(title: str, url: str) -> str` : `[title](url)`.
  - `render_end_marker(meta) -> str` : the value of `meta["guide_end_marker"]`.

- [ ] **Step 1: Write the failing tests**

```python
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
```

- [ ] **Step 2: Run to verify failure** (same pytest command, `test_render.py`)

- [ ] **Step 3: Implement render.py**

```python
"""Markdown rendering primitives. Explicit line construction only."""
from __future__ import annotations


def render_frontmatter(meta):
    lines = ["---"]
    lines += [f"{k}: {v}" for k, v in meta.items()]
    lines.append("---")
    return "\n".join(lines) + "\n"


def render_table(headers, aligns, rows):
    out = ["| " + " | ".join(headers) + " |", "|" + "|".join(aligns) + "|"]
    out += ["| " + " | ".join(r) + " |" for r in rows]
    return "\n".join(out) + "\n"


def render_link(title, url):
    return f"[{title}]({url})"


def render_end_marker(meta):
    return meta["guide_end_marker"]
```

- [ ] **Step 4: Run to verify pass**

- [ ] **Step 5: Commit** (message `Add markdown rendering primitives`)

Note for the implementer: any style the real frozen files use that these primitives cannot express gets discovered in Task 8's extraction round-trip; extend the primitives there, with a unit test per extension, rather than guessing now.

---

### Task 4: Output assembly (build.py)

**Files:**
- Create: `tools/drdebits_build/drdebits_build/build.py`
- Test: `tools/drdebits_build/tests/test_build.py`

**Interfaces:**
- Consumes: model loaders, render primitives.
- Produces:
  - `BuildError(Exception)`
  - `find_root(start) -> Path` : walk up until dir contains `drdebits.md` and `src/`; raise `BuildError` otherwise.
  - `load_sources(root) -> Sources` : dataclass holding `meta`, `fragments` (ordered `(name, text)` list read from `src/guide/*.md` sorted by filename), `catalogue`, `behaviour`, `apes`, `changelog`.
  - `build_guide(s) -> str`, `build_catalogue_md(s) -> str`, `build_behaviour_md(s) -> str`, `build_apes_md(s) -> str`
  - `GENERATED: dict[str, callable]` mapping relpath to builder for the four generated markdown files.
  - `build_sha256sums(root, s) -> str` : lines `<hex> *<relpath>` for the files in `meta["checksum_files"]` (`|`-separated ordered list), hashing generated content for generated files and on-disk bytes otherwise.
  - `write_outputs(root, outdir) -> dict[relpath, str]`
  - `stamp_version(text: str, version: str) -> str` and `VERSION_RE` : regex `[0-9]+\.[0-9]+\.[0-9]+(?:-[A-Za-z0-9.]+)?`.

- [ ] **Step 1: Write the failing tests** (synthetic mini-repo fixture; later tasks import `make_repo` from this module)

```python
"""Tests for output assembly against a synthetic source tree."""
import textwrap
from pathlib import Path
import pytest
from drdebits_build.build import (
    BuildError, find_root, load_sources, build_guide, build_sha256sums,
    write_outputs, stamp_version,
)


def make_repo(tmp_path: Path) -> Path:
    (tmp_path / "src" / "guide").mkdir(parents=True)
    (tmp_path / "src" / "data").mkdir()
    (tmp_path / "drdebits.md").write_text("placeholder", encoding="utf-8")
    (tmp_path / "LICENSE").write_text("MIT-ish\n", encoding="utf-8", newline="\n")
    (tmp_path / "README.md").write_text("DrDebits `0.9.9-test` stub\n", encoding="utf-8", newline="\n")
    (tmp_path / "MAINTENANCE.md").write_text("Protocol for `0.9.9-test`.\n", encoding="utf-8", newline="\n")
    (tmp_path / "src" / "guide" / "000-header.md").write_text("# G\n\nIntro.\n", encoding="utf-8", newline="\n")
    (tmp_path / "src" / "guide" / "010-rules.md").write_text("## Rules\n\nBe good.\n", encoding="utf-8", newline="\n")
    (tmp_path / "src" / "data" / "metadata.yaml").write_text(textwrap.dedent("""\
        fields:
          - key: guide_version
            value: "0.9.9-test"
          - key: guide_end_marker
            value: "END-v0.9.9-test"
          - key: tpb_guidance_statement_count
            value: "1"
          - key: tpb_library_index_count
            value: "1"
          - key: checksum_files
            value: "LICENSE|README.md|drdebits.md"
    """), encoding="utf-8", newline="\n")
    (tmp_path / "src" / "data" / "tpb-catalogue.yaml").write_text(textwrap.dedent("""\
        entries:
          - id: "GS01"
            title: "T"
            url: "https://x.invalid/a"
            trigger: "g"
    """), encoding="utf-8", newline="\n")
    (tmp_path / "src" / "data" / "behaviour-tests.yaml").write_text(textwrap.dedent("""\
        entries:
          - id: "A-001"
            scenario: "s"
            expected_status: "HARD_STOP"
            required_behaviour: "r"
            side_effect_check: "c"
    """), encoding="utf-8", newline="\n")
    (tmp_path / "src" / "data" / "apes-110-map.yaml").write_text(textwrap.dedent("""\
        contexts:
          - label: "All"
            value: "Part 1"
        retrieval_points:
          - label: "Scope"
            value: "R1.2"
    """), encoding="utf-8", newline="\n")
    (tmp_path / "src" / "data" / "changelog.yaml").write_text(textwrap.dedent("""\
        entries:
          - version: "0.9.9-test"
            date: "2026-01-01"
            status: "Draft"
            change: "init"
    """), encoding="utf-8", newline="\n")
    return tmp_path


def test_find_root(tmp_path):
    root = make_repo(tmp_path)
    nested = root / "src" / "guide"
    assert find_root(nested) == root
    with pytest.raises(BuildError):
        find_root(tmp_path.parent)


def test_guide_shape_and_determinism(tmp_path):
    s = load_sources(make_repo(tmp_path))
    g1, g2 = build_guide(s), build_guide(s)
    assert g1 == g2
    assert g1.startswith("---\nguide_version: 0.9.9-test\n")
    assert g1.rstrip("\n").endswith("END-v0.9.9-test")
    assert "## Rules" in g1 and "| 0.9.9-test | 2026-01-01 | Draft | init |" in g1


def test_sha256sums_covers_generated_and_static(tmp_path):
    root = make_repo(tmp_path)
    s = load_sources(root)
    out = build_sha256sums(root, s)
    lines = out.rstrip("\n").split("\n")
    assert lines[0].endswith(" *LICENSE") and lines[2].endswith(" *drdebits.md")
    assert all(len(l.split(" *")[0]) == 64 for l in lines)


def test_write_outputs_lf_only(tmp_path):
    root = make_repo(tmp_path)
    outdir = tmp_path / "out"
    written = write_outputs(root, outdir)
    assert "drdebits.md" in written and "SHA256SUMS" in written
    raw = (outdir / "drdebits.md").read_bytes()
    assert b"\r" not in raw


def test_stamp_version():
    assert stamp_version("DrDebits `0.2.0-draft` here", "0.3.0-draft") == "DrDebits `0.3.0-draft` here"
    assert stamp_version("v1.2.3 and 9.9.9-x.1", "2.0.0") == "v2.0.0 and 2.0.0"
```

- [ ] **Step 2: Run to verify failure**

- [ ] **Step 3: Implement build.py**

```python
"""Assemble the runtime files from src/. Deterministic, LF, UTF-8."""
from __future__ import annotations

import hashlib
import re
from dataclasses import dataclass
from pathlib import Path

from . import model, render


class BuildError(Exception):
    pass


VERSION_RE = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[A-Za-z0-9.]+)?")

CHANGELOG_HEADERS = ["Version", "Date", "Status", "Change"]
CHANGELOG_ALIGNS = ["---", "---", "---", "---"]


def find_root(start):
    p = Path(start).resolve()
    for candidate in (p, *p.parents):
        if (candidate / "drdebits.md").is_file() and (candidate / "src").is_dir():
            return candidate
    raise BuildError(f"no DrDebits root found above {start}")


@dataclass
class Sources:
    meta: dict
    fragments: list
    catalogue: list
    behaviour: list
    apes: dict
    changelog: list


def load_sources(root):
    root = Path(root)
    data = root / "src" / "data"
    meta = model.load_metadata(data / "metadata.yaml")
    catalogue = model.load_catalogue(data / "tpb-catalogue.yaml")
    model.validate_counts(meta, catalogue)
    fragments = []
    for f in sorted((root / "src" / "guide").glob("*.md")):
        fragments.append((f.name, f.read_text(encoding="utf-8")))
    if not fragments:
        raise BuildError("src/guide/ has no fragments")
    return Sources(
        meta=meta,
        fragments=fragments,
        catalogue=catalogue,
        behaviour=model.load_behaviour_tests(data / "behaviour-tests.yaml"),
        apes=model.load_apes_map(data / "apes-110-map.yaml"),
        changelog=model.load_changelog(data / "changelog.yaml"),
    )


def build_changelog_section(s):
    rows = [[r["version"], r["date"], r["status"], r["change"]] for r in s.changelog]
    return "## Change log\n\n" + render.render_table(CHANGELOG_HEADERS, CHANGELOG_ALIGNS, rows)


def build_guide(s):
    parts = [render.render_frontmatter({k: v for k, v in s.meta.items() if k != "checksum_files"})]
    parts += [text for _, text in s.fragments]
    parts.append(build_changelog_section(s))
    parts.append("\n" + render.render_end_marker(s.meta) + "\n")
    return "".join(parts)


def build_catalogue_md(s):
    rows = [[render.render_link(r["title"], r["url"]), r["trigger"]] for r in s.catalogue]
    header = f"# DrDebits TPB Guidance Statement catalogue\n\nPart of DrDebits `{s.meta['guide_version']}`.\n\n"
    return header + render.render_table(
        ["Statement (concise title and official link)", "LLM trigger"], ["---", "---"], rows)


def build_behaviour_md(s):
    rows = [[r["id"], r["scenario"], r["expected_status"], r["required_behaviour"], r["side_effect_check"]]
            for r in s.behaviour]
    header = f"# DrDebits behaviour tests\n\nPart of DrDebits `{s.meta['guide_version']}`.\n\n"
    return header + render.render_table(
        ["ID", "Scenario", "Expected status", "Required behaviour and human step", "Side-effect check"],
        ["---", "---", "---", "---", "---"], rows)


def build_apes_md(s):
    header = f"# DrDebits APES 110 reference map\n\nPart of DrDebits `{s.meta['guide_version']}`.\n\n"
    a = render.render_table(["Context", "APES 110 starting points"], ["---", "---"],
                            [[r["label"], r["value"]] for r in s.apes["contexts"]])
    b = render.render_table(["Control", "APES 110 retrieval points"], ["---", "---"],
                            [[r["label"], r["value"]] for r in s.apes["retrieval_points"]])
    return header + a + "\n" + b


GENERATED = {
    "drdebits.md": build_guide,
    "reference/tpb-catalogue.md": build_catalogue_md,
    "tests/behaviour-tests.md": build_behaviour_md,
    "reference/apes-110-map.md": build_apes_md,
}


def build_sha256sums(root, s):
    generated = {rel: fn(s) for rel, fn in GENERATED.items()}
    lines = []
    for rel in s.meta["checksum_files"].split("|"):
        if rel in generated:
            digest = hashlib.sha256(generated[rel].encode("utf-8")).hexdigest()
        else:
            digest = hashlib.sha256((Path(root) / rel).read_bytes()).hexdigest()
        lines.append(f"{digest} *{rel}")
    return "\n".join(lines) + "\n"


def stamp_version(text, version):
    return VERSION_RE.sub(version, text)


def write_outputs(root, outdir):
    s = load_sources(root)
    outdir = Path(outdir)
    written = {rel: fn(s) for rel, fn in GENERATED.items()}
    written["SHA256SUMS"] = build_sha256sums(root, s)
    for rel, content in written.items():
        target = outdir / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(content.encode("utf-8"))
    return written
```

- [ ] **Step 4: Run to verify pass** (`test_build.py`, then the whole suite)

- [ ] **Step 5: Commit** (message `Assemble runtime outputs from src with deterministic bytes`)

Note: the exact satellite header text and guide assembly details WILL change in Task 8 to match the frozen files byte-for-byte; that is expected, and the structural tests here get updated alongside.

---

### Task 5: Verification pass (verify.py)

**Files:**
- Create: `tools/drdebits_build/drdebits_build/verify.py`
- Test: `tools/drdebits_build/tests/test_verify.py`

**Interfaces:**
- Consumes: `GENERATED`, `build_sha256sums`, `load_sources`, `VERSION_RE`, `ModelError`.
- Produces: `run_verify(root) -> list[str]` (empty list means pass). Checks in order: byte-parity for generated files + SHA256SUMS with message `"{rel}: committed file differs from build output; edit src/ and run build"`; end marker is the last guide line and equals `meta["guide_end_marker"]`; every `VERSION_RE` match in README.md and MAINTENANCE.md equals `meta["guide_version"]`; loader violations (schema, statuses, https URLs, counts) become messages not tracebacks.

- [ ] **Step 1: Write the failing tests**

```python
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
```

Note: `test_stamped_version_mismatch_detected` rewrites README.md AFTER `sync`, so the byte-parity failure for SHA256SUMS also appears (README is in `checksum_files`); the assertion only requires the version message to be among the failures.

- [ ] **Step 2: Run to verify failure**

- [ ] **Step 3: Implement verify.py**

```python
"""Rebuild-and-compare verification. Returns messages instead of raising."""
from __future__ import annotations

from pathlib import Path

from .build import GENERATED, VERSION_RE, build_sha256sums, load_sources


def run_verify(root):
    root = Path(root)
    try:
        s = load_sources(root)
    except Exception as exc:  # loader problems are findings, not crashes
        return [f"sources: {exc}"]

    failures: list[str] = []
    built = {rel: fn(s) for rel, fn in GENERATED.items()}
    built["SHA256SUMS"] = build_sha256sums(root, s)
    for rel, content in built.items():
        committed = root / rel
        if not committed.is_file():
            failures.append(f"{rel}: missing committed file")
        elif committed.read_bytes() != content.encode("utf-8"):
            failures.append(f"{rel}: committed file differs from build output; edit src/ and run build")

    guide = built["drdebits.md"]
    last = guide.rstrip("\n").rsplit("\n", 1)[-1]
    if last != s.meta["guide_end_marker"]:
        failures.append(f"end marker: last line {last!r} != {s.meta['guide_end_marker']!r}")

    for rel in ("README.md", "MAINTENANCE.md"):
        p = root / rel
        if p.is_file():
            for m in VERSION_RE.finditer(p.read_text(encoding="utf-8")):
                if m.group(0) != s.meta["guide_version"]:
                    failures.append(f"{rel}: version {m.group(0)} != {s.meta['guide_version']}")

    return failures
```

- [ ] **Step 4: Run to verify pass** (full suite)

- [ ] **Step 5: Commit** (message `Add rebuild-and-compare verification pass`)

---

### Task 6: CLI (__main__.py)

**Files:**
- Create: `tools/drdebits_build/drdebits_build/__main__.py`
- Test: `tools/drdebits_build/tests/test_cli.py`

**Interfaces:**
- Produces: `main(argv) -> int`; `python -m drdebits_build build [--root PATH]` writes outputs into the repo root and prints `wrote {rel}` lines; `... verify [--root PATH]` prints failures to stderr, returns 1 if any, else prints `verify: OK` and returns 0.

- [ ] **Step 1: Write the failing tests**

```python
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
```

- [ ] **Step 2: Run to verify failure**

- [ ] **Step 3: Implement __main__.py**

```python
"""CLI: build regenerates committed outputs in place; verify checks them."""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

from .build import find_root, load_sources, stamp_version, write_outputs
from .verify import run_verify


def main(argv=None):
    parser = argparse.ArgumentParser(prog="drdebits_build")
    parser.add_argument("command", choices=["build", "verify"])
    parser.add_argument("--root", default=None)
    args = parser.parse_args(argv)
    root = Path(args.root) if args.root else find_root(Path.cwd())
    if args.command == "build":
        for rel in write_outputs(root, root):
            print(f"wrote {rel}")
        return 0
    failures = run_verify(root)
    for f in failures:
        print(f, file=sys.stderr)
    if failures:
        return 1
    print("verify: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 4: Run to verify pass**

- [ ] **Step 5: Commit** (message `Add build/verify CLI`)

---

### Task 7: Stamping integration

**Files:**
- Modify: `tools/drdebits_build/drdebits_build/__main__.py` (build branch)
- Test: extend `tools/drdebits_build/tests/test_cli.py`

**Interfaces:**
- Produces: `build` additionally rewrites `README.md` and `MAINTENANCE.md` in place through `stamp_version(text, meta["guide_version"])` when the file exists, printing `stamped {rel}` lines. IMPORTANT ORDERING: stamping happens BEFORE `write_outputs`, because `SHA256SUMS` hashes the stamped on-disk bytes of README.md and MAINTENANCE.md.

- [ ] **Step 1: Add failing test**

```python
def test_build_stamps_readme_and_maintenance_before_checksums(tmp_path):
    root = make_repo(tmp_path)
    (root / "README.md").write_text("DrDebits `0.0.1` stub\n", encoding="utf-8", newline="\n")
    (root / "MAINTENANCE.md").write_text("Protocol `0.0.1`.\n", encoding="utf-8", newline="\n")
    assert main(["build", "--root", str(root)]) == 0
    assert "0.9.9-test" in (root / "README.md").read_text(encoding="utf-8")
    assert "0.9.9-test" in (root / "MAINTENANCE.md").read_text(encoding="utf-8")
    from drdebits_build.verify import run_verify
    assert run_verify(root) == []
```

- [ ] **Step 2: Run to verify failure** (the `run_verify == []` assertion fails if checksums were computed pre-stamp)

- [ ] **Step 3: Implement** (in `main`'s build branch, replacing the current body)

```python
    if args.command == "build":
        s = load_sources(root)
        for rel in ("README.md", "MAINTENANCE.md"):
            p = root / rel
            if p.is_file():
                p.write_bytes(stamp_version(p.read_text(encoding="utf-8"), s.meta["guide_version"]).encode("utf-8"))
                print(f"stamped {rel}")
        for rel in write_outputs(root, root):
            print(f"wrote {rel}")
        return 0
```

- [ ] **Step 4: Run full suite to verify pass**

- [ ] **Step 5: Commit** (message `Stamp hand-maintained files before checksum generation`)

---

### Task 8: Extraction of the frozen v0.2.0-draft content (the migration)

**Files:**
- Create: `tools/drdebits_build/scripts/extract_v020.py` (one-off, committed for provenance)
- Create: `src/guide/*.md`, `src/data/*.yaml` (extraction output)
- Modify: `tools/drdebits_build/drdebits_build/build.py` / `render.py` ONLY as needed to reach byte parity, each change with a unit test

**Interfaces:**
- Produces: a `src/` tree from which `python -m drdebits_build verify --root .` exits 0 with all runtime files untouched. THE MIGRATION GATE.

- [ ] **Step 1: Write the extractor.** Core helpers, verbatim:

```python
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
```

CAUTION for the implementer: cell text containing a literal `|` inside a markdown link or code span would break the naive split; the round-trip assertion catches this immediately. If it fires, extend `parse_table` to re-join split cells whose reassembly makes the round-trip pass (join adjacent cells with `|` until the rerender matches), and add a regression unit test for the exact affected line in `tests/test_render.py`.

Then `main()` in the same script, in order (each block asserts before writing):

1. Read `drdebits.md`. Split frontmatter; write `src/data/metadata.yaml` as the ordered `fields:` list, every value through `yaml_quote`, plus a final field `checksum_files` whose value is the `|`-joined relpaths in the committed `SHA256SUMS` order (parse `SHA256SUMS` lines as `<hex> *<relpath>`).
2. Split the body at every line starting `## `. The chunk before the first `## ` is `000-header.md`. Each later chunk becomes `NNN-<slug>.md` (NNN = 010, 020, ... in file order; slug = heading text lowercased, runs of non-alphanumerics collapsed to `-`), EXCEPT the `## Change log` chunk and the trailing end-marker line, which are dropped (rendered from data at build time). Assert BEFORE writing anything: frontmatter + all fragments + change-log chunk + marker line reassembles the original file byte-for-byte.
3. Parse the change-log chunk's table with `parse_table(_, 4)`; write `src/data/changelog.yaml` (`entries:` list; keys `version`, `date`, `status`, `change`; values through `yaml_quote`).
4. Read `reference/tpb-catalogue.md`. Everything above its table's header line is the file's header text; PRINT it verbatim between `CATALOGUE-HEADER-BEGIN/END` markers (Step 3 uses it). Parse the 2-column table; for each row split the first cell with `LINK_RE` (assert exactly one match); write `src/data/tpb-catalogue.yaml` with `id` = `GS` + the two digits from the title's `TPB(GS) NN/` pattern (assert the pattern matches), `title`, `url`, `trigger`.
5. Same pattern for `tests/behaviour-tests.md` (5-column table, header printed between `BEHAVIOUR-HEADER-BEGIN/END`) into `behaviour-tests.yaml`, and `reference/apes-110-map.md` (two 2-column tables, header and the between-tables text printed between markers) into `apes-110-map.yaml`.
6. Print `EXTRACTION OK` and the count of files written.

- [ ] **Step 2: Run the extractor**

```bash
cd "C:/Users/-/AppData/Local/Temp/claude/C--/d73ac32a-b268-474f-a1ba-c058181baa3f/scratchpad/DrDebits" && uv run --project tools/drdebits_build python tools/drdebits_build/scripts/extract_v020.py
```

Expected: `EXTRACTION OK`. On any assertion failure, fix the extractor or extend `render.py` (unit test per extension). NEVER edit the frozen files.

- [ ] **Step 3: Align the builders to the frozen bytes.** Using the printed header texts, replace the guessed header constants in `build_catalogue_md`, `build_behaviour_md`, `build_apes_md`, and adjust `build_guide` joining/spacing until:

```bash
uv run --project tools/drdebits_build python -m drdebits_build verify --root .
```

prints `verify: OK`. Every builder change updates its structural unit test in the same edit. Parameterise the version inside header text via `s.meta["guide_version"]` so Task 9's bump test passes (assert the printed header contains the current version string in exactly one place; if it appears elsewhere, parameterise those too).

- [ ] **Step 4: Full suite + verify + clean status**

```bash
uv run --project tools/drdebits_build --locked --extra dev pytest tools/drdebits_build/tests -q && uv run --project tools/drdebits_build python -m drdebits_build verify --root . && git status --porcelain
```

Expected: tests pass, `verify: OK`, and `git status` shows ONLY new files under `src/` and `tools/` (no modified runtime files).

- [ ] **Step 5: Commit** (message `Extract v0.2.0-draft content into src; builds reproduce runtime files byte-identically`)

---

### Task 9: Acceptance tests on the real repo

**Files:**
- Test: `tools/drdebits_build/tests/test_acceptance.py`

- [ ] **Step 1: Write the tests** (clone the real repo into tmp so nothing frozen is touched)

```python
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
    root = clone_repo(tmp_path)
    meta = root / "src" / "data" / "metadata.yaml"
    text = meta.read_text(encoding="utf-8").replace("0.2.0-draft", "0.2.1-draft")
    meta.write_text(text, encoding="utf-8", newline="\n")
    assert main(["build", "--root", str(root)]) == 0
    assert main(["verify", "--root", str(root)]) == 0
    assert "0.2.1-draft" in (root / "README.md").read_text(encoding="utf-8")
    assert (root / "drdebits.md").read_text(encoding="utf-8").rstrip("\n").endswith("0.2.1-draft")
```

Caveat: `REAL / "tests"` is the repo's `tests/behaviour-tests.md` directory, distinct from the package's own `tests/`; `find_root` from the test file's location resolves the repo root correctly because the package sits under `tools/` inside the repo.

- [ ] **Step 2: Run to verify pass** (these must pass immediately; a failure here is a Task 8 defect to fix under Task 8's rules)

- [ ] **Step 3: Commit** (message `Add acceptance tests over the migrated repo`)

---

### Task 10: Migration gate check

**Files:** none

- [ ] **Step 1: Confirm the gate**

```bash
cd "C:/Users/-/AppData/Local/Temp/claude/C--/d73ac32a-b268-474f-a1ba-c058181baa3f/scratchpad/DrDebits" && git diff 4c14621 --stat -- drdebits.md reference tests/behaviour-tests.md LICENSE SHA256SUMS README.md MAINTENANCE.md && uv run --project tools/drdebits_build python -m drdebits_build verify --root .
```

Expected: EMPTY diff stat (every frozen file identical to the pre-project commit `4c14621`) and `verify: OK`. Record both outputs in the task report. If anything shows, STOP; nothing later lands until the gate holds.

---

### Task 11: Maintenance protocol and README update

**Files:**
- Modify: `MAINTENANCE.md` (steps 6 and 8 only)
- Modify: `README.md` (append a contributor section)
- Regenerated: `SHA256SUMS` (via build)

- [ ] **Step 1: Edit MAINTENANCE.md.** Replace the step-6 line (`6. Test every direct link and all behaviour tests in \`tests/behaviour-tests.md\`.`) with:

```markdown
6. Run `uv run --project tools/drdebits_build python -m drdebits_build verify` and review the latest weekly link-check workflow run; investigate any links it reported.
```

Replace the step-8 line (the version-string/SHA256SUMS/end-marker step) with:

```markdown
8. Edit the sources under `src/` (never the generated files), update `guide_version`, `release_tag` and the end marker in `src/data/metadata.yaml`, run `uv run --project tools/drdebits_build python -m drdebits_build build`, then re-run `verify`.
```

- [ ] **Step 2: Append to README.md:**

```markdown

## Contributing and maintenance

The guide, reference and test files are generated. Edit the sources under
`src/` and run `uv run --project tools/drdebits_build python -m
drdebits_build build`; CI rejects hand edits to generated files. See
`MAINTENANCE.md` for the release protocol.
```

- [ ] **Step 3: Rebuild and verify**

```bash
uv run --project tools/drdebits_build python -m drdebits_build build --root . && uv run --project tools/drdebits_build python -m drdebits_build verify --root . && git diff --stat
```

Expected: `verify: OK`; the diff shows exactly `MAINTENANCE.md`, `README.md` and `SHA256SUMS`.

- [ ] **Step 4: Commit** (message `Route maintenance steps 6 and 8 through the build tooling`)

---

### Task 12: CI verify workflow

**Files:**
- Create: `.github/workflows/verify.yml`

- [ ] **Step 1: Write the workflow**

```yaml
name: Verify

on:
  push:
    branches: [main]
  pull_request:

permissions:
  contents: read

jobs:
  verify:
    name: build parity and structural checks
    runs-on: ubuntu-latest
    steps:
      - name: Check out source
        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false

      - name: Set up Python
        uses: actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97 # v7.0.0
        with:
          python-version: "3.12"

      - name: Install the locked toolchain
        run: python -m pip install "uv==0.12.0"

      - name: Run the build package tests
        run: uv run --project tools/drdebits_build --locked --extra dev pytest tools/drdebits_build/tests -q

      - name: Verify committed outputs against sources
        run: uv run --project tools/drdebits_build --locked python -m drdebits_build verify --root .
```

- [ ] **Step 2: Validate with the local actionlint**

```bash
cd "C:/Users/-/AppData/Local/Temp/claude/C--/d73ac32a-b268-474f-a1ba-c058181baa3f/scratchpad/DrDebits" && "C:/Users/-/AppData/Local/Temp/claude/C--/d73ac32a-b268-474f-a1ba-c058181baa3f/scratchpad/actionlint.exe" .github/workflows/verify.yml
```

Expected: no findings.

- [ ] **Step 3: Commit** (message `Add CI verify workflow`)

---

### Task 13: Report-only link-check workflow

**Files:**
- Create: `tools/drdebits_build/drdebits_build/linkcheck.py`
- Create: `.github/workflows/link-check.yml`
- Test: `tools/drdebits_build/tests/test_linkcheck.py`

**Interfaces:**
- Produces: `collect_urls(root) -> list[str]` (unique, first-seen order: every `https://` URL in the built generated outputs plus README.md and MAINTENANCE.md) and `python -m drdebits_build.linkcheck --root . [--timeout 20]` printing `DEAD <status-or-error> <url>` lines, exit 1 if any. The checker never writes anything; the workflow maintains a single report issue.

- [ ] **Step 1: Write the failing test** (offline only)

```python
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
```

- [ ] **Step 2: Run to verify failure**

- [ ] **Step 3: Implement linkcheck.py**

```python
"""Live link checking. Report-only: prints findings, never edits anything."""
from __future__ import annotations

import argparse
import re
import urllib.request
from pathlib import Path

from .build import GENERATED, find_root, load_sources

URL_RE = re.compile(r"https://[^\s)\"<>]+")
UA = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) drdebits-linkcheck"


def collect_urls(root):
    root = Path(root)
    s = load_sources(root)
    texts = [fn(s) for fn in GENERATED.values()]
    for rel in ("README.md", "MAINTENANCE.md"):
        p = root / rel
        if p.is_file():
            texts.append(p.read_text(encoding="utf-8"))
    seen, out = set(), []
    for text in texts:
        for m in URL_RE.finditer(text):
            url = m.group(0).rstrip(".,;")
            if url not in seen:
                seen.add(url)
                out.append(url)
    return out


def check(url, timeout):
    req = urllib.request.Request(url, headers={"User-Agent": UA}, method="GET")
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return resp.status
    except Exception as exc:  # any failure is a finding, described not raised
        return exc


def main(argv=None):
    parser = argparse.ArgumentParser(prog="drdebits_build.linkcheck")
    parser.add_argument("--root", default=None)
    parser.add_argument("--timeout", type=int, default=20)
    args = parser.parse_args(argv)
    root = Path(args.root) if args.root else find_root(Path.cwd())
    dead = 0
    for url in collect_urls(root):
        result = check(url, args.timeout)
        if not (isinstance(result, int) and 200 <= result < 400):
            print(f"DEAD {result} {url}")
            dead += 1
    return 1 if dead else 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 4: Offline test to pass, then one live local smoke run**

```bash
uv run --project tools/drdebits_build --locked python -m drdebits_build.linkcheck --root . --timeout 25
```

Expected: exit 0 (all links live as of 2026-08-16). Record actual output; transient DEAD lines are findings to note, not code failures.

- [ ] **Step 5: Write link-check.yml**

```yaml
name: Link check

on:
  schedule:
    - cron: "0 21 * * 0"
  workflow_dispatch:

permissions:
  contents: read
  issues: write

jobs:
  links:
    name: live link check, report only
    runs-on: ubuntu-latest
    steps:
      - name: Check out source
        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false

      - name: Set up Python
        uses: actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97 # v7.0.0
        with:
          python-version: "3.12"

      - name: Install the locked toolchain
        run: python -m pip install "uv==0.12.0"

      - name: Check links
        id: links
        shell: bash
        run: |
          set -euo pipefail
          uv run --project tools/drdebits_build --locked python -m drdebits_build.linkcheck --root . | tee dead-links.txt || echo "failed=true" >> "$GITHUB_OUTPUT"

      - name: Open or update the report issue
        if: steps.links.outputs.failed == 'true'
        env:
          GH_TOKEN: ${{ github.token }}
        shell: bash
        run: |
          set -euo pipefail
          title="Link check: SOURCE CURRENCY NOT CONFIRMED for listed links"
          body="The weekly live link check could not confirm these links. Treat the affected material as SOURCE CURRENCY NOT CONFIRMED until a human re-verifies the sources. This workflow only reports; it changes nothing.

          $(cat dead-links.txt)"
          existing="$(gh issue list --repo "$GITHUB_REPOSITORY" --state open --search "$title in:title" --json number --jq '.[0].number // empty')"
          if [ -n "$existing" ]; then
            gh issue comment "$existing" --repo "$GITHUB_REPOSITORY" --body "$body"
          else
            gh issue create --repo "$GITHUB_REPOSITORY" --title "$title" --body "$body"
          fi
```

- [ ] **Step 6: actionlint both workflows** (Task 12 Step 2 command against both files). Expected: no findings.

- [ ] **Step 7: Commit** (message `Add report-only weekly link check`)

---

### Task 14: Push and CI confirmation

**Files:** none (remote operations)

- [ ] **Step 1: Pre-push review.** `git log --oneline 4c14621..HEAD` and `git diff 4c14621 --stat`; re-run Task 10's gate commands one final time (expect: only MAINTENANCE.md, README.md, SHA256SUMS differ among runtime files, per Task 11).

- [ ] **Step 2: CONFIRM WITH RYAN before pushing** (outward action on the public guide repo). Present: the commit list, the runtime-file delta (exactly three files, wording and stamps only), and that two workflows activate on push.

- [ ] **Step 3: Push and watch**

```bash
git push origin main && gh run watch --repo ryanduguid/DrDebits --exit-status "$(gh run list --repo ryanduguid/DrDebits --limit 1 --json databaseId --jq '.[0].databaseId')"
```

Expected: Verify workflow green. If red, fix minimally (trailer on every commit), push, repeat.

- [ ] **Step 4: Dispatch the link check once**

```bash
gh workflow run link-check.yml --repo ryanduguid/DrDebits && sleep 10 && gh run watch --repo ryanduguid/DrDebits --exit-status "$(gh run list --repo ryanduguid/DrDebits --workflow link-check.yml --limit 1 --json databaseId --jq '.[0].databaseId')"
```

Expected: green with no issue created. If an issue appears: transient failures are acceptable and get noted; systematic failure (every URL dead) is a checker bug to fix before closing.
