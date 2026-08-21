"""Rebuild-and-compare verification. Returns messages instead of raising."""
from __future__ import annotations

import re
from pathlib import Path

from .build import GENERATED, STAMP_RE, BuildError, build_sha256sums, load_sources
from .model import ModelError


def run_verify(root):
    root = Path(root)
    try:
        s = load_sources(root)
    except (ModelError, BuildError, OSError) as exc:
        return [f"sources: {exc}"]

    failures: list[str] = []
    built = {rel: fn(s) for rel, fn in GENERATED.items()}

    # A missing checksum_files member must surface as a finding, not a crash.
    try:
        built["SHA256SUMS"] = build_sha256sums(root, s)
    except (ModelError, BuildError, OSError) as exc:
        failures.append(f"SHA256SUMS: cannot rebuild ({exc})")
        # SHA256SUMS drops out of the parity check below; all other entries still checked

    for rel, content in built.items():
        committed = root / rel
        if not committed.is_file():
            failures.append(f"{rel}: missing committed file")
        elif committed.read_bytes() != content.encode("utf-8"):
            failures.append(f"{rel}: committed file differs from build output; edit src/ and run build")

    # The end marker is checked on the committed guide, not the rebuild, so a
    # truncated commit is caught; skip if the file was already reported missing.
    if not any("drdebits.md: missing committed file" in f for f in failures):
        committed_guide = root / "drdebits.md"
        if committed_guide.is_file():
            committed_text = committed_guide.read_text(encoding="utf-8")
            last = committed_text.rstrip("\n").rsplit("\n", 1)[-1]
            if last != s.meta["guide_end_marker"]:
                failures.append(f"end marker: last line {last!r} != {s.meta['guide_end_marker']!r}")

    # Stamped files must exist and carry at least one version token each;
    # zero matches would let a stripped stamp verify vacuously.
    for rel in ("README.md", "MAINTENANCE.md"):
        p = root / rel
        if not p.is_file():
            failures.append(f"{rel}: missing")
        else:
            text = p.read_text(encoding="utf-8")
            matches = list(STAMP_RE.finditer(text))
            if not matches:
                failures.append(f"{rel}: no version stamp found")
            else:
                for m in matches:
                    if m.group(0) != s.meta["guide_version"]:
                        failures.append(f"{rel}: version {m.group(0)} != {s.meta['guide_version']}")

    # The guide version lives in several places beyond the stamped files
    # above. Each must agree with meta["guide_version"] (the single source of
    # truth) or with each other, so a bump that misses one spot is caught here
    # instead of shipping a self-contradictory release.

    # (a) the committed drdebits.md header must carry the exact version line
    if not any("drdebits.md: missing committed file" in f for f in failures):
        committed_guide = root / "drdebits.md"
        if committed_guide.is_file():
            committed_text = committed_guide.read_text(encoding="utf-8")
            version_line = "> Version: `" + s.meta["guide_version"] + "`"
            if version_line not in committed_text.splitlines():
                failures.append("drdebits.md: header version line does not match guide_version")

    # (b) release_tag must be "v" + guide_version
    if s.meta["release_tag"] != "v" + s.meta["guide_version"]:
        failures.append(
            f"metadata: release_tag {s.meta['release_tag']} != v{s.meta['guide_version']}")

    # (c) guide_end_marker must be "DRDEBITS-END-" + release_tag
    if s.meta["guide_end_marker"] != "DRDEBITS-END-" + s.meta["release_tag"]:
        failures.append(
            f"metadata: guide_end_marker {s.meta['guide_end_marker']} != "
            f"DRDEBITS-END-{s.meta['release_tag']}")

    # (d) the newest (first) changelog entry must match guide_version
    newest_changelog_version = s.changelog[0]["version"]
    if newest_changelog_version != s.meta["guide_version"]:
        failures.append(
            f"changelog: newest entry {newest_changelog_version} != {s.meta['guide_version']}")

    # (e) the source-check date is derived into the catalogue header and the
    # guide frontmatter from metadata, but the guide's header line and README
    # carry hand-written copies. A metadata bump that misses either would ship
    # one release carrying two different check dates, so cross-check both.
    # (src/guide/040-source-status.md carries two further prose copies that
    # this check does not cover; MAINTENANCE step 8 owns those.)
    checked_date = s.meta.get("sources_checked_at", "")[:10]
    if checked_date:
        # Substring, not whole-line: the real header line carries a timezone
        # suffix after the backticked date.
        guide_line = f"> Sources last checked: `{checked_date}`"
        committed_guide = root / "drdebits.md"
        if committed_guide.is_file():
            if guide_line not in committed_guide.read_text(encoding="utf-8"):
                failures.append(
                    "drdebits.md: header source-check date line does not match "
                    f"sources_checked_at ({checked_date})")
        readme = root / "README.md"
        if readme.is_file():
            if f"Sources last checked: {checked_date}" not in readme.read_text(encoding="utf-8"):
                failures.append(
                    f"README.md: source-check date does not match sources_checked_at ({checked_date})")

    # (f) CITATION.cff carries its own copies of the version and release date.
    # When the file exists, both must agree with the sources - version with
    # guide_version, date-released with the newest changelog row - so a bump
    # cannot ship a stale citation record. Quoted and unquoted YAML scalars
    # are both accepted. A deliberately absent file is tolerated.
    citation = root / "CITATION.cff"
    if citation.is_file():
        text = citation.read_text(encoding="utf-8")
        version_m = re.search(r'^version:\s*"?([^"\r\n]+?)"?\s*$', text, re.MULTILINE)
        if not version_m or version_m.group(1) != s.meta["guide_version"]:
            failures.append(
                f"CITATION.cff: version does not match guide_version "
                f"{s.meta['guide_version']}")
        release_date = s.changelog[0]["date"]
        date_m = re.search(r'^date-released:\s*"?([0-9-]+)"?\s*$', text, re.MULTILINE)
        if not date_m or date_m.group(1) != release_date:
            failures.append(
                f"CITATION.cff: date-released does not match newest changelog "
                f"date {release_date}")

    return failures
