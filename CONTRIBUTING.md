# Contributing

Send changes as pull requests against `main`, one purpose per pull request, and
complete the pull request template. Report security vulnerabilities privately as
[SECURITY.md](SECURITY.md) describes, never in an issue or pull request. Keep
client data, credentials and proprietary prompt content out of the repository,
commit messages and pull requests; reproduce a problem with fabricated facts.

## Where to make a change

The guide, the reference files, the behaviour tests, `evals/cases.json` and
`evals/RESULTS.md` are generated. Edit their sources under `src/`, then rebuild:

```bash
cargo run --manifest-path tools/drdebits_build/Cargo.toml --locked --release -- build --root .
```

The build rewrites the generated files and `SHA256SUMS`. Commit them with the
source change; CI rejects a hand edit to a generated file. The README's
[Contributing and maintenance](README.md#contributing-and-maintenance) section
covers which files are generated and how evaluation records are kept.
[MAINTENANCE.md](MAINTENANCE.md) holds the source-check and release protocol,
including the hand-written copies a release must update.

## Checks a change must pass

[verify.yml](.github/workflows/verify.yml) runs these on Ubuntu and Windows
with the Rust toolchain pinned in [rust-toolchain.toml](rust-toolchain.toml).
Run them from `tools/drdebits_build` before you push:

```bash
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo llvm-cov --locked --fail-under-lines 92
cargo run --locked --release -- verify --root ../..
python3 -m unittest discover --start-directory tests --pattern 'test_*.py'
```

`cargo llvm-cov` runs the tests with line coverage, as both jobs do; `cargo
test --locked` runs them without it. The Ubuntu job also lints the workflows
with actionlint and audits them with zizmor. CI also scans the commits each push or
pull request adds, and the full history weekly, for secrets and client
identifiers with the shared secrets scan from `ryanduguid/release-policy`
(betterleaks) and [.gitleaks.toml](.gitleaks.toml). It scans a pull request a
second time with the base branch's `.gitleaks.toml`, so add an allowlist entry
in its own pull request, before the change that needs it.
`pre-commit install` runs formatting, lint, tests, `verify` and betterleaks before
each commit, through the same pinned toolchain.

## Tests for the build tooling

New or changed behaviour in `tools/drdebits_build` needs a test in
`tools/drdebits_build/tests` or beside the code it covers. CI holds the builder
at 92% line coverage, so a large untested change fails the coverage
report. A test for a fix should fail on the base commit and pass with the
change.

Codecov's `codecov/project` check, which main requires, holds coverage at
100% (see [codecov.yml](codecov.yml)). If it stays pending after Codecov shows
the commit as processed, rerun the 'upload coverage to Codecov' job in
[verify.yml](.github/workflows/verify.yml); the new upload posts the status.
