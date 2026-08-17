"""Rebuild-and-compare verification. Returns messages instead of raising."""
from __future__ import annotations

from pathlib import Path

from .build import GENERATED, VERSION_RE, build_sha256sums, load_sources


def run_verify(root):
    root = Path(root)
    try:
        s = load_sources(root)
    except Exception as exc:  # loader problems are findings, not crashes; catches ModelError and BuildError alike
        return [f"sources: {exc}"]

    failures: list[str] = []
    built = {rel: fn(s) for rel, fn in GENERATED.items()}

    # Fix 3 (IMPORTANT): wrap SHA256SUMS rebuild in try/except to catch missing checksum_files members
    try:
        built["SHA256SUMS"] = build_sha256sums(root, s)
    except Exception as exc:
        failures.append(f"SHA256SUMS: cannot rebuild ({exc})")
        # Skip SHA256SUMS from parity check below; all other entries still checked

    for rel, content in built.items():
        committed = root / rel
        if not committed.is_file():
            failures.append(f"{rel}: missing committed file")
        elif committed.read_bytes() != content.encode("utf-8"):
            failures.append(f"{rel}: committed file differs from build output; edit src/ and run build")

    # Fix 2 (IMPORTANT): check committed drdebits.md, not rebuilt; skip if already reported missing
    if not any("drdebits.md: missing committed file" in f for f in failures):
        committed_guide = root / "drdebits.md"
        if committed_guide.is_file():
            committed_text = committed_guide.read_text(encoding="utf-8")
            last = committed_text.rstrip("\n").rsplit("\n", 1)[-1]
            if last != s.meta["guide_end_marker"]:
                failures.append(f"end marker: last line {last!r} != {s.meta['guide_end_marker']!r}")

    # Fix 1 (CRITICAL): check presence of stamped files; fail if missing or have zero version matches
    for rel in ("README.md", "MAINTENANCE.md"):
        p = root / rel
        if not p.is_file():
            failures.append(f"{rel}: missing")
        else:
            text = p.read_text(encoding="utf-8")
            matches = list(VERSION_RE.finditer(text))
            if not matches:
                failures.append(f"{rel}: no version stamp found")
            else:
                for m in matches:
                    if m.group(0) != s.meta["guide_version"]:
                        failures.append(f"{rel}: version {m.group(0)} != {s.meta['guide_version']}")

    return failures
