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
