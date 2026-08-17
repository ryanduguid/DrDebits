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
