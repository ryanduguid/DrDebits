# DrDebits maintenance protocol

Part of [DrDebits](./drdebits.md) `0.2.0-draft`.

A source-check date does not guarantee continuing currency. Before professional reliance, retrieve the authority operative for the relevant historical event or period and the current duties governing action today.

For each DrDebits release:

1. Check the latest TASA, TASR and Code Determination compilations on the Federal Register of Legislation, including uncommenced amendments and transitional provisions.
2. Recount and compare every results page in the filtered live TPB Guidance Statement catalogue; inspect issue and last-modified dates, especially statements changed since the previous release.
3. Check the APESB APES 110 landing page, compiled standard, compilation details, technical alerts and effective/transitional dates, and the current APES 220 issue and effective dates.
4. Check the operative AML/CTF Act text and the current AUSTRAC accountant guidance, transitional rules and compliance-officer requirements.
5. Reassess each DrDebits rule against the changed source. Do not update a date without reviewing the substantive effect.
6. Test every direct link and all behaviour tests in `tests/behaviour-tests.md`.
7. Confirm that no client data, credentials, proprietary prompt content or unauthorised APESB text entered the repository.
8. Update the version string everywhere it appears (frontmatter `guide_version` and `release_tag`, the header block, the `README.md` header block with its source-check date, the change log, the end marker and the satellite-file headers), regenerate `SHA256SUMS` over every DrDebits file, and confirm the declared end marker.
9. Tag the release, sign the tag and the release commit, and record the source check, material changes, reviewer, approved tag and release date in the change log.

If freshness cannot be confirmed, the LLM must label the affected material `SOURCE CURRENCY NOT CONFIRMED`, avoid calling it “latest” or “current”, and restrict the output to a draft requiring primary-source verification.

Versioning convention:

- **Major:** a change to the control model, authority handling or human-decision boundaries.
- **Minor:** a new or materially revised source, rule, context module or behaviour test.
- **Patch:** a link repair, citation correction or wording change with no control effect.
