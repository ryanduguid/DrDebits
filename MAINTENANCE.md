# DrDebits maintenance protocol

Part of [DrDebits](./drdebits.md) `0.3.2`.

A source-check date does not guarantee continuing currency. Before professional reliance, retrieve the authority operative for the relevant historical event or period and the current duties governing action today.

For each DrDebits release:

1. Check the latest TASA, TASR and Code Determination compilations on the Federal Register of Legislation, including uncommenced amendments and transitional provisions.
2. Recount and compare every results page in the filtered live TPB Guidance Statement catalogue; inspect issue and last-modified dates, especially statements changed since the previous release.
3. Check the APESB APES 110 landing page, compiled standard, compilation details, technical alerts and effective/transitional dates, and the current APES 220 issue and effective dates.
4. Check the operative AML/CTF Act text and the current AUSTRAC accountant guidance, transitional rules and compliance-officer requirements.
5. Reassess each DrDebits rule against the changed source. Do not update a date without reviewing the substantive effect.
6. Run `uv run --project tools/drdebits_build python -m drdebits_build verify` and review the latest weekly link-check workflow run; investigate any links it reported.
7. Confirm that no client data, credentials, proprietary prompt content or unauthorised APESB text entered the repository.
8. Edit the sources under `src/` (never the generated files), update `guide_version`, `release_tag`, the end marker and `sources_checked_at` in `src/data/metadata.yaml`, update the header version and source-check date lines in `src/guide/000-header.md`, the dates in `src/guide/040-source-status.md`, the README header date, and the `version` and `date-released` lines in `CITATION.cff`, and add the release row in `src/data/changelog.yaml`. Run `uv run --project tools/drdebits_build python -m drdebits_build build`, then re-run `verify` (it cross-checks the version and date copies it knows about).
9. Tag the release, sign the tag and the release commit, and record the material changes and release date in the change-log row. The signed tag itself records the approver and the approved revision.

If freshness cannot be confirmed, the LLM must label the affected material `SOURCE CURRENCY NOT CONFIRMED`, avoid calling it “latest” or “current”, and restrict the output to a draft requiring primary-source verification.

## GitHub release checklist

For each future GitHub release:

1. Start from a clean worktree. Run the complete pytest suite, builder `verify`, a deterministic second build and the reviewed link-check result. Resolve every failure or classify every unreachable link before staging a release.
2. Confirm the verified guide bundle contains `LICENSE`, `README.md`, `CITATION.cff`, `drdebits.md`, `MAINTENANCE.md`, `reference/tpb-catalogue.md`, `reference/apes-110-map.md`, `tests/behaviour-tests.md` and the generated `SHA256SUMS` manifest. The manifest covers the first eight files and travels with them; it does not hash itself.
3. Create a signed release commit and a signed annotated tag that identify the approved revision.
4. Confirm immutable releases are enabled for the repository before creating the release. Create a GitHub draft release, stage every asset, and include the checksum and verification instructions.
5. Download every staged asset into a clean location while the release is still a draft. Independently verify the signed commit and tag, bundle membership, every `SHA256SUMS` entry and any available asset attestations.
6. Publish the release only after all verification succeeds. With immutable releases enabled, publication locks the tag and assets only after that pre-publication verification.
7. After publication, run `gh release verify TAG` and `gh release verify-asset TAG PATH` for each downloaded asset when immutable releases are enabled, and retain the verification record.
8. Do not overwrite or delete a published immutable release, and do not reuse its tag. Correct a released defect with a new version, new tag and a new release.

The existing `v0.1.0-draft`, `v0.2.0-draft`, `v0.3.0-draft` and `v0.3.1-draft` releases are historical published prereleases, not GitHub draft releases. They have no attached distribution assets. `v0.3.1` (published 21 August 2026) reused the `v0.3.1-draft` commit with no assets and no version bump, and rode a lightweight unsigned tag; `v0.3.2` supersedes it through the checklist above. No published tag before `v0.3.2` carries a signature, whatever earlier notes implied. Do not silently rewrite them; any superseding distribution must use a new tag and a new release.

Versioning convention:

- **Major:** a change to the control model, authority handling or human-decision boundaries.
- **Minor:** a new or materially revised source, rule, context module or behaviour test.
- **Patch:** a link repair, citation correction or wording change with no control effect.
