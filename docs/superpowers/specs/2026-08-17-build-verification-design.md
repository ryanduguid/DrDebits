# DrDebits build-and-verification module: design

- Date: 2026-08-17
- Status: approved 2026-08-17
- Origin: account architecture survey (16 August 2026), candidate 6

## Problem

DrDebits v0.2.0-draft is five hand-edited runtime files plus a manual maintenance protocol. MAINTENANCE.md steps 6 and 8 require testing every link and behaviour case, updating the version string in six-plus locations, regenerating SHA256SUMS over seven files and confirming the end marker, all by hand. The 20 behaviour tests are a prose table no harness can execute. The repository has no CI, so nothing catches a missed version stamp, a stale digest, a truncated guide or a count that no longer matches the frontmatter.

## Decision

Full build-from-sources. Structured sources compile deterministically into the committed runtime files; an executable verify step proves the committed outputs match the sources byte for byte and enforces the structural rules. The runtime interface (raw files fetched from GitHub) does not change.

## Sources of truth (`src/`)

- `src/guide/` : the guide's prose as ordered markdown fragments, one per H2 section, named `NNN-<slug>.md` (`010-deployment.md`, `020-how-to-use.md`, ...). Authored order is the build order.
- `src/data/metadata.yaml` : every frontmatter field (version, status, jurisdiction, owner, canonical repository, release tag, source-check timestamp, review-due date, compilation identifiers, APES filenames and digest, statement counts, end-marker template).
- `src/data/tpb-catalogue.yaml` : 55 entries with `id`, `title`, `url`, `trigger`.
- `src/data/apes-110-map.yaml` : the two reference tables (context starting points; paragraph-level retrieval points).
- `src/data/behaviour-tests.yaml` : 20 entries with `id`, `scenario`, `expected_status`, `required_behaviour`, `side_effect_check`.
- `src/data/changelog.yaml` : the change-log rows.

Rule of assignment: anything counted, tabular, versioned or repeated is data; only genuine prose is a fragment.

## Committed outputs

Two classes, both committed so diffs stay reviewable and consumers keep fetching plain files:

- **Fully generated (byte-exact):** `drdebits.md` (frontmatter + fragments + rendered inline tables + end marker), `reference/tpb-catalogue.md`, `reference/apes-110-map.md`, `tests/behaviour-tests.md`, `SHA256SUMS`. Every byte comes from `src/`; hand edits are build failures.
- **Stamped (hand-maintained, version-token rewritten):** `README.md` and `MAINTENANCE.md`. The build rewrites only their version strings in place; the surrounding prose stays hand-edited. Verify checks the stamped version matches `metadata.yaml`, not whole-file parity.

## Build package (`tools/drdebits_build/`)

Python 3.12, uv-locked, `pyyaml` pinned, pytest dev extra. Modules:

- `model.py` : load and schema-validate all YAML sources (unique ids, fixed status enum, non-empty required fields, well-formed https URLs, count fields consistent with row counts).
- `render.py` : frontmatter block, markdown tables, satellite headers, end marker.
- `build.py` : assemble and write every output with LF endings and no timestamps or other run-varying content.
- `verify.py` : the verification pass (below).
- `__main__.py` : `uv run python -m drdebits_build build` and `... verify`.

Determinism contract: two builds from the same sources produce byte-identical outputs; the build embeds nothing derived from the clock, the environment or randomness.

## Verification pass

1. Rebuild the fully generated outputs to a temporary directory and byte-compare each against the committed file. Any divergence fails with the offending path and the instruction to edit `src/` instead. For the stamped files, verify only that the version token equals `metadata.yaml`.
2. End marker: literal last line of `drdebits.md` equals the rendered `guide_end_marker`.
3. Version: the version string is identical in every declared location (frontmatter fields, guide header, change log, end marker, README, MAINTENANCE.md, satellite headers).
4. Counts: catalogue rows == `tpb_guidance_statement_count` == `tpb_library_index_count`.
5. Behaviour tests: ids unique, `expected_status` from the fixed enum, no empty fields.
6. Digests: every `SHA256SUMS` entry matches the actual file digest.
7. URLs: well-formed https in every data file.

Explicitly not machine-checked, by design: source currency, legal substance and APES-text licence boundaries. Those remain human steps in MAINTENANCE.md, consistent with the guide's rule that automation never declares source currency.

## CI

- `verify.yml` (push and pull_request): pytest over the build package, then the verify pass. `contents: read`.
- `link-check.yml` (weekly cron plus manual dispatch): live-checks every URL in the data files; on failures opens or updates a single issue listing the dead links with `SOURCE CURRENCY NOT CONFIRMED` wording. Report-only: no workflow edits content, closes the loop or claims currency.
- Conventions follow release-policy: actions pinned by full commit SHA with version comments, third-party binaries digest-verified, `persist-credentials: false`, fail closed.

## Migration invariant

Phase one extracts the current v0.2.0-draft content into `src/` and must reproduce every committed output byte-identically, proven by the verify pass passing against the untouched files before any other change lands. This project changes no guide content; the text is frozen. The renderer copies the existing files' formatting quirks.

## Maintenance protocol update

MAINTENANCE.md step 6 becomes "run `verify`" plus the link-check workflow reference; step 8 becomes "edit `src/`, run `build`, run `verify`". Human steps (source checking, substantive review, signing, tagging) stay as written.

## Non-goals

- No wheel building or publishing; the build package is repo-internal tooling.
- No automated source-currency claims anywhere.
- No guide content changes in this project.
- No signing automation; tags and signatures remain Ryan's manual acts.

## Acceptance criteria

1. `uv run python -m drdebits_build verify` exits 0 on the migrated repo with the pre-existing runtime files untouched.
2. Hand-editing any generated output makes `verify` fail, naming the file and the correct source location.
3. Changing the version in `metadata.yaml` and running `build` updates every declared location in one command, and `verify` passes.
4. CI runs the verify pass on every push and pull request; the link-check workflow exists, is scheduled, and only reports.
5. The build package's own tests pass under `uv run --locked pytest`.
