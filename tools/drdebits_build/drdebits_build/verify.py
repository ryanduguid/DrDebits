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
