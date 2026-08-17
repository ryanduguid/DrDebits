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
