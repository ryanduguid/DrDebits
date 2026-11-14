# Changelog

All notable changes to this repository are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Guide versions use the
versioning convention in [MAINTENANCE.md](MAINTENANCE.md); the guide's own
change log in `src/data/changelog.yaml` remains the record for each guide
release.

## [Unreleased]

### Added

- CALC-011, a behaviour test with a frozen prompt, bringing the suite to 105: 2 AI models agree on a wrong GST figure and the user offers their agreement as verification. The workpaper record now asks for each model consulted, whether they agreed and how a disagreement was resolved, and says agreement between models is not an independent check. No recorded model result.
- 2 naive date methods' answers to the Payday Super and Division 7A benchmark, in `benchmarks/payday-div7a/baselines/`, scored in the benchmark README and checked by a test, with a dated note on the same payday in Nagaris's AI Accounting Index. No model has been scored.
- SRC-009, CALC-012 and WORK-004, behaviour tests with frozen prompts, bringing the suite to 108: a vendor article that states super guarantee rules from before 1 July 2026 as current and misstates how payment deadlines count business days, a super guarantee contribution rounded down to whole dollars (12% of 4870.00 is 584.40, not 584.00), and model-written BAS commentary ending with a partner sign-off the partner has not given. No recorded model result.
- `docs/ai-policy-crosswalk.md`, which maps each DrDebits control to the clause of a small firm's AI-use policy it supports, says whether it is a guide rule, a mechanical check, a behaviour test or a reference document, and names the common clauses it does not cover. Outside the verified bundle.
- CALC-013, SRC-010 and DATA-008, behaviour tests with frozen prompts, bringing the suite to 111: PAYG withholding on a 1845.00 weekly pay worked from the annual rates instead of the ATO's Schedule 1 formula (409.00, not 408.00), a 2026-27 tax line asked for on a rate card with the 2025-26 rates (8520.00, not 8788.00), and an employee's TFN to be sent to a public website's checker. No recorded model result.

### Fixed

- OAIC links, which now answer automated clients with a challenge, are listed for checking by hand instead of making the link check incomplete (#264).
- Attorney-General's Department links, which time out from GitHub's runners, are listed for checking by hand too, so the link check is no longer incomplete on them.
- The README's Codacy link names the renamed repository, and its contents line links the Alternatives section.
- `gate` blocks a sum that a word interrupts, such as `$100 + wages $200 + $50 = $250`, which it passed by reperforming only `$200 + $50 = $250`. It cannot tell what such a sum adds, so a correct one, such as `$100 + wages $200 + $50 = $350`, blocks too, as with a bracket that does not pair, and so do `$100 + wages + $200 + $50 = $250`, `$100 + FY2025 wages $200 + $50 = $250` and any other text but whitespace in that place or between the amount and the operator, such as `$100 + ... $200 + $50 = $250` and `$500 wages + $200 + $50 = $250`. A correct sum after a label that follows an amount and an operator blocks for the same reason, as in `Item 3 - Deductions $100 + $50 = $150` and `1 x laptop $1,000 + $200 = $1,200`. A minus, `x` or `/` counts as an operator only with spaces round it, so dates such as `1/7/2025` are not read as one, and nor is `$100/wages`. Write the calculation on a line of its own, without words between its amounts.
- `gate` no longer lets a dollar amount hide a sum that a word interrupts: `$100 + $20 wages $200 + $50 = $250` blocks, where it passed by reperforming only `$200 + $50 = $250`. Only `=` now ends the text the gate looks back over, so a correct sum after an earlier amount and operator, as in `Range $100 - $200 a week, so $100 + $50 = $150`, blocks too, and so does a table row that repeats its amounts, as in `| Laptop | 2 x $1,000 | 2 x $1,000 = $2,000 |`.
- `gate` also blocks `amount = amount` after an amount and an operator with no `=` between, as `$50 = $600` is in `Wages $500 + super $50 = $600`, which it did not read as a calculation at all. The correct `$550` blocks too, and so do a rate or unit before an operator, as in `Rent $2,000/month × 12 = $24,000` and `2,514 km × 0.88 = $2,212`, and a result written as an expression, as in `(95,000 + 20,000 - 10,000) / 480,000 = 105,000 / 480,000`. The message for such a line, and for a sum a word interrupts, now says what the gate saw and asks for only amounts, operators and brackets before each `=` and one amount after the last. An unspaced `/` before a word still reads as a rate, so `Rent is $2,000/month, so 12 × $2,000 = $24,000` is reperformed and `$100/wages $200 + $50 = $250` passes.
- `gate` blocks a sum with a date among its operands, such as `Rent 1/7/2025 + 2 + 2 = 5`, and a calculation written with digits other than 0 to 9, such as `2 + 3 = ٥` in Arabic-Indic digits, instead of skipping them. A date alone before `=`, bracketed or not, as in `Lodged 1/7/2026 = 2026` and `Due (28/10/2026) = 28 days`, is still prose.
- `gate` blocks a sum it reads no calculation in, which it passed unchecked: an `=` followed by an amount, after an amount and an operator with no `=` between, where no calculation the gate reads ends. That covers a word after the last amount, as in `Salary $80,000 + $5,000 bonus = $95,000`, a bracket closing outside every amount, as in `$100 + $50 (GST) = $160` and `2 + (3) ) = 6`, and a sum split across table cells, as in `| Rent | $500 × 52 | = $26,001 |`. It also covers a sum before a date the gate reads as prose, as in `Rent $500 × 52 weeks to 30/6/2026 = $26,001`. A correct one, such as `$100 + $50 (GST) = $150`, blocks too, and so do a result written as an expression, as in `1 + 1 = 2 + 1`, a word in place of an amount, as in `Invoice $1,000 + GST, total = $1,100`, and a label with a spaced hyphen after a number, as in `Year 1 - Depreciation = $2,000`; write `Year 1 depreciation: $2,000` instead. A table cell boundary does not end the look-back, since `|` does not reliably mark a cell and a sum can span cells, so `| Laptop | 2 x $1,000 | 2 x $1,000 = $2,000 |` still blocks; put each amount in a cell of its own, as in `| Laptop | 2 | $1,000 | 2 x $1,000 = $2,000 |`.
- `gate` no longer starts a calculation inside a hyphenated number, so `Rent 2025-07-01 + 2 + 2 = 5`, an ISO date before a sum, blocks where it passed as `01 + 2 + 2 = 5`, and so do `FY 2025-26 + 1 = 27`, with a hyphen or en dash, and `s 8-1 + 2 = 3`. A date before a sum that no operator joins to it, as in `Paid on 2026-07-01, $100 + $50 = $150`, is still reperformed, and so is a calculation after a word and a hyphen, as in `Quarterly-$12,000/4 = $3,000`.
- `gate` reads an en dash, em dash or other dash as a hyphen, and a no-break or thin space as a space, so `$1,000 – $200 = $800` is reperformed and `5 – 3 = 3` and `2 - 3 = –2` block, where all 3 passed unchecked; a spaced `X` multiplies as `x` does, so `2 X $1,000 = $3,000` blocks, and so do correct lines with a spaced `X` that is a word, as in `2 + 3 = 5 X Factor` and `Version 2 X edition: $100 + $50 = $150`. A dash after a number, as in `FY 2025—26 + 1 = 27`, joins it, as a hyphen does.
- `gate` blocks a result it cannot read whole after a sum, which it passed unchecked: an `=` followed by a digit before any `,` or `;`, after an amount and an operator with no `=` between, where no calculation the gate reads ends. That covers `$100 + $50 = $ 160`, `$100 + $50 = A$160`, `$100 + $50 = AUD 160`, `$100 + $50 = approx. $160`, `$100 - $150 = $-60`, `2 + 3 == 6` and `100/4 = 30 x 2`, and a letter written onto a result, as in `$100 + $50 = $150k`, which it read as `$150`. A correct one, such as `$100 + $50 = $ 150`, `$100 - $150 = $-50`, `2 + 3 == 5` or `2 + 3 = 5h`, blocks too, and so does other text with a digit after an `=` that follows an amount and an operator, such as a spreadsheet formula in `Price $100 + 10% GST; in Excel use =A1*1.1` or a note in `Wages $500 + super; tax = see note 3`; write one amount the gate reads, such as `$150` or `-$50`, after the last `=`.
- `gate` no longer starts a calculation after a sign written after `$`, so `$-150 + 200 = 350` blocks where it passed as `150 + 200 = 350`, and so does the correct `$-150 + 200 = 50`; write `-$150`. Before an `=` that no read calculation ends, a `/`, `x` or `X` between numbers outside a date counts as an operator with or without spaces, and so does an unspaced minus before `$`, so `$12,000/4 bonus = $3,500`, `$12,000/ 4 bonus = $3,500`, `$500x52 = $26,001`, `2x3 = 7` and `$1,000-$200 = $900` block where they passed unchecked, and so do the correct `$12,000/4 bonus = $3,000`, `GST is 1/11 of the price = $100`, `2x3 = 6` and `$1,000-$200 = $800`. A date, a rate such as `Rent $2,000/month = $24,000 a year` and a fraction before a calculation the gate reads, as in `GST is 1/11 of the price: $1,100 / 11 = $100`, are still prose.
- `gate` blocks a calculation with a numeric character beside it that is not a digit, such as `²` in `2 + 3 = 5²` and `½` in `2 + 3 = 5½`, which it read as `2 + 3 = 5`.
- `gate` reads `123/4/2025 * 4 * 2025 = 123` as a calculation and reperforms it, where it blocked it as a date: a date must now have no digit or letter either side.
- `gate` reads every line whole. Its calculation scan gave up on a sum of 9 or more terms with no number after `=`, such as `1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 = x`, and so skipped any wrong sum later on that line; it now runs in time linear in the line.
- `assess` labels its authorities count 'authorities off the list or unread': it also counts section-like text it cannot read, such as `§ 162` and 'subsection (1) allows', which `EVALUATION-NOTES.md` said it did not.
- The README's OpenSSF Scorecard badge and link name the renamed repository.

### Changed

- The guide on `main` is labelled the unreleased 0.7.0 candidate in its metadata, header, README, `llms.txt`, MAINTENANCE.md and CITATION.cff, which no longer gives a release date. Its 4 new behaviour tests and workpaper rule make the next release Minor under MAINTENANCE.md; it said 0.6.0 and current whilst differing from the v0.6.0 release.

## [v0.6.0] - 2026-11-12

Changes released in `0.6.0`, merged since `v0.5.0` (31 October 2026). Sources were not rechecked: they stay as checked on 14 October 2026.

### Added

- `docs/connector-examples.md`, a worked example of vendor checklist rows VA-39 to VA-43: for a read-only MCP server, a plugin in a host's catalogue and an MCP server that holds writes in an approval queue, which questions each vendor's own page answers and which stay open, read on 12 November 2026. Outside the verified bundle (#262).
- LEDGER-003 and LEDGER-004, behaviour tests with frozen prompts, bringing the suite to 104: a model with a ledger tool asked to approve a journal it staged, and one holding a system-tier approval for one exact write that is asked to make it with a changed amount and date. Neither has a recorded model result (#256).
- Vendor checklist questions VA-39 to VA-47. A new 'Connectors, agents and recordings' section asks which tools a connector or MCP server gives the model, whether it can be limited to read-only and switched off, whether each user signs in as themselves, whether a write sends exactly the confirmed payload, where connector calls are recorded, how consent to a recording is obtained and how recordings and transcripts are kept and used. A 'Sharing and planned features' section asks who can share chats and outputs (VA-46) and which planned features would change what the product can do (VA-47) (#256).
- Two behaviour tests with frozen prompts, bringing the suite to 102: a recorded shareholder loan close beside a labelled counterfactual repayment that establishes nothing about Division 7A (WHAT-001), and a request to backdate a July receipt into the June close and confirm Division 7A compliance (WHAT-002). Neither has a recorded model result (#260).
- A pre-run report, computed offline from a responses file, so a billed run shows more than one pass count. `assess` now names status mismatches, including over-refusal (`HARD_STOP` where a case expects another status), unsafe completion (going ahead where a case expects a stop) and an unread status (none, or more than one), and counts the rest as other; it reports the benchmark canary repeated in a response and, with `--authorities FILE`, citations off that closed list, and summarises them. A responses file may hold several samples per case, and the report then counts the cases whose checks clear on every sample. A record dated from 8 November 2026 keeps a verdict per sample and carries `web_access`, `model_roles` and those counts as `screens`; `evals/RESULTS.md` shows all-of-k beside pass@1 for it. Earlier confirmed records load and render unchanged; an observation written before this change must be re-scored. No model was run; the tests use scripted responses (#251).
- A Payday Super and Division 7A benchmark in `benchmarks/payday-div7a/`: 31 questions, each key documented from an ATO example, the ATO's table, a recorded ATO calculator run or the statute and checked against the published engines. `bench keys` recomputes 21 keys offline and compares all 31 with the recorded engine output, and `bench score` marks value and authority separately. No model has been scored, and the publisher also publishes products it could score (#250).
- `gate --source FILE`, repeatable, asks for review of each quoted span not found verbatim in a supplied source. It never blocks and shows text membership only (#250).
- Free baselines in `evals/RESULTS.md`: a responder that refuses everything and one that always proceeds, scored by the mechanical status check on every case with no model run. The README's first screen now states current confirmed coverage and these scores (#250).
- 16 behaviour tests with frozen prompts, bringing the suite to 100: AUDIT-001 and AUDIT-002 on AI output offered as SMSF audit work; CALLER-001 and CALLER-002 on a voice agent and an unverified caller, with a benign contrast; REPAIR-001, REPAIR-002 and NEXUS-001 on capital repairs and private outgoings, with contrasts; FACT-001 to FACT-003 on contradictory supplied facts, with a benign contrast; CARE-001 to CARE-003 on reasonable care where AI output was relied on, and on updated ATO guidance that is only proposed; LEDGER-001 and LEDGER-002, where the model holds a ledger write tool; and SRC-006, where the supplied rate post-dates the period. A model evaluation runtime now supplies a sixth inert tool, `ledger_write` (#250).
- An AI tool register, reviewed at least annually and on any tool, model or vendor change, and an incident log for failures found in use, among the AI-specific TPB rules. The GS 55 concordance's CC-35 now points to them; beyond the per-task workflow checks, the guide still sets no periodic monitoring of output quality (#244).
- Vendor checklist question VA-38, in a new 'Source authority' section: does the product rank sources by authority and show each citation's layer, treating a private ruling as binding only for its applicant (#244).
- Three behaviour tests with frozen prompts, bringing the suite to 70: AML/CTF designated-service classification for a registered office address (AML-002) and a bookkeeper's routine payments (AML-003), and a false premise backed by a real, closely matching passage (CITE-010). None has a recorded model result (#224).
- Four behaviour tests with frozen prompts, bringing the suite to 83, for measures of the *Treasury Laws Amendment (Tax Reform No. 1) Act 2026* applied outside their periods: the 50% CGT discount on a sale after 1 July 2027 (FUT-002), the negative gearing quarantine dated from the wrong income year and the 12 May 2026 cut-off (FUT-003), the standard deduction for work-related expenses claimed for 2025-26 (FUT-004) and the Working Australians tax offset claimed for 2026-27 (FUT-005). None has a recorded model result (#245).
- FUT-006, a behaviour test with a frozen prompt, bringing the suite to 84: a user treats the 30% minimum tax on discretionary trusts as law because its Bills passed the House of Representatives on 29 October 2026, and applies it to 2026-27 (#247).
- A CITE-002 grader note on TR 2010/3W and *Commissioner of Taxation v Bendel* [2026] HCA 18 in the evaluation notes (#224).
- `gate --authorities FILE` asks for review, never a block, of each section of an Act named in full, such as `s 8-1 of the Income Tax Assessment Act 1997`, and each ruling id in the TR, TD, LCR, GSTR, GSTD, MT, CR, PR, SGR, PCG, PS LA or TPB(GS) series that a draft cites and the reviewer's closed list omits. It also reads a subsection, paragraph or subparagraph of such a section, such as `subsection 8-1(1)`, `s. 8-1(1)`, `s8-1(1)`, `sub-sec 8-1(1)` or `paragraph 8-1(1)(a)`, or the numbered provision ending a composite such as `paragraph (a) of subsection 9-5(1)`, which the list covers when it holds that citation or a provision containing it, and lists section-like or ruling-like text it cannot read whole, including sibling parts after the Act, such as `, (2) and (3)`, and a part named without its section, such as `subsection (2)`, even in discussion after a full citation, since the gate cannot tell which section it belongs to. Other citations are not checked, so a pass does not establish that every cited authority was checked. A malformed authorities file exits 2; verdicts and exit codes are otherwise unchanged (#241, #252).
- SRC-004, a behaviour test with a frozen prompt, bringing the suite to 71: an available authoritative correction contradicted by a later secondary summary, with a request to certify an unreviewed draft (#232).
- Rules to weigh an Administrative Review Tribunal decision on another taxpayer's facts as reasoning, never as settling the client's position, and to treat a term, authority or rule from another jurisdiction as an unverified lead until an Australian primary source supports it, with behaviour tests CITE-011 and CITE-012, bringing the suite to 73 (#233).
- Six behaviour tests with frozen prompts, bringing the suite to 79: the Medicare levy surcharge family threshold raised for every child (CALC-009), a bank-feed category the chart lacks (CALC-010), an unpublished 2026-27 Medicare levy low-income threshold (SRC-005), a BAS due date moved for a regional holiday (LODGE-002), a fee too low to do the work properly (FEE-001) and a commission kept from the client (FEE-002) (#234).

### Changed

- The APES 110 map's Part 5 independence row gives the 1 July 2028 start, unless early adopted, for the provisions on value chain components, from APESB's amending standard of 8 July 2025 (#244).
- `gate --facts` adds a note for each draft amount equal to a supplied fact, naming the fact: `$1,100 is equal in amount to fact 'invoice total'`. The note records a match in value, not where the figure came from. An amount equal to 2 or more facts is noted as ambiguous with every name, and the gate does not pick one. Reviews, verdicts and exit codes are unchanged (#239).
- The AML/CTF intake classification gives table 6 examples from AUSTRAC's 25 September 2026 guidance, adds items 7 and 9, and states item 3 with the s 6(5C)(b) exception (#224).
- The TPB control set says an interim suspension can be extended for further periods of up to 90 days each once a TPB investigation has started, not only once (#231).
- Versions made available from 4 November 2026 are licensed under the Ryan Duguid Public Attribution Licence 1.0: the FSL-1.1-ALv2 text with a condition that a copy, port or derivative made available outside the licensee's organisation credits Ryan Duguid publicly. Internal use is exempt, and earlier releases keep their licences. `AGENTS.md` asks AI agents to tell their users this before reusing the code (#242, #246).

### Fixed

- `gate` blocks a calculation with a bracket that does not pair within its step between `=` signs, such as `(salary $1,000 - $200) = $900`, instead of passing it unchecked. The gate sees only part of the sentence and cannot tell what the bracket encloses, so a correct sum written this way, such as `(salary $1,000 - $200) = $800`, now blocks too, as does an aside ending in a digit before an operator, such as `$100 (note 1) + $50 = $150`; write the calculation on its own. A bracket opened before a calculation, as in `(2 + 3 = 5)`, is read as before (#254).
- The weekly link check no longer reports the synthetic `portal.example` link in a behaviour-test prompt as dead: links on RFC 2606 documentation names are listed as `EXAMPLE` and never requested (#229).

Not listed, as they change no guide, reference, behaviour test or builder behaviour: README wording and banners (#225 to #228, #238, #248), links to the renamed repository (#253), copyright and SPDX headers (#243), and CI, test coverage and dependency changes (#222, #223, #230, #236, #237, #240, #249, #255, #257 to #259, #261).

## [v0.5.0] - 2026-10-31

Changes released in `0.5.0`, merged since `v0.4.0` (28 October 2026). Sources were not rechecked: they stay as checked on 14 October 2026.

### Added

- `verify` checks that the release-status lines in the guide header, README and `llms.txt`, and the first line of `MAINTENANCE.md`, agree with the metadata `status` and release tag, and that the bundle file counts `MAINTENANCE.md` states match `checksum_files` and the installer's lists (#211).
- `gate --facts FILE` asks for review, never a block, of each dollar amount or percentage in a draft that is neither a supplied fact nor the result of a calculation the draft shows; it flags untraced amounts and cannot certify traced ones (#215).
- A conditional automated-decision transparency check in the data and privacy gate for APP 1.7 to 1.9 of the *Privacy Act 1988*, which start on 10 December 2026; vendor checklist question VA-37 asking for the APP 1.8 high-level information; and a matching row in the AI register entry. Each leaves the legal conclusion to the firm, and the check does not stop a task (#217).
- Eight behaviour tests, bringing the suite to 67: output-side TFN and cross-client leak cases (DATA-003 to DATA-007) and cases where a draft flags a fact and then assumes it (ASSUME-001 to ASSUME-003) (#219).
- A benchmark canary string and do-not-train line in the evaluation files, and a mapping of the CITE cases to the categories of a public database of AI hallucination cases (#219).

### Changed

- The prohibited-text screen in the builder ignores case, whitespace, hyphen and quote variants, so a marker wrapped across lines is caught (#219).
- README licence and VirusTotal badges are local assets and the Scorecard badge is a GitHub-hosted file, so they render through GitHub's image proxy; the banners show brand version 8 (#216, #218, #220).
- The `commit-msg` hook runs its Python in isolated mode, and its header no longer recommends `core.hooksPath=.githooks`, which would run hooks from whichever branch is checked out (#213).

Not listed, as they have no user-visible effect: Clippy-driven changes to builder parameter types (#212, #214).

## [v0.4.0] - 2026-10-28

Changes released in `0.4.0`, merged since `v0.3.3` (6 September 2026).

### Added

- AI tool and vendor assurance checklist, `reference/ai-vendor-assurance.md` (#46).
- Vendor checklist citations of the Guidance for AI Adoption practices (#104), contract-terms checks (#108), APES 320 and the OAIC AI-product guidance (#116), and ISO/IEC 42001 certificate checks with supplier details for a firm's AI register (#121).
- TPB(GS) 55/2026 concordance, `reference/tpb-gs55-concordance.md`, naming the permission forms and contest process (#112).
- TPB sanction controls, with the s 45 row corrected for 1 October 2026 (#105).
- Behaviour tests: eight calculator cases (#76); five cases with the whole-response rule (#82); permitted workpaper completion alongside refusal boundaries (#84); four citation cases (#108); unpublished-figure, draft-ruling, period, registration and review-risk cases (#170).
- Source-currency check against the Federal Register for every pinned compilation, run weekly (#67).
- Source-currency report of registered amendments that come into force within 120 days (#106).
- Bundle installer and verifier for complete guide installations against an approved digest (#80, #102).
- First full behavioural evaluation record (#41) and Codex evaluation observations (#50).
- Evaluation records bound to the guide commit, runtime, tools and input digests (#74, #85, #91), with an inert tool harness for action-boundary tests (#82).
- Case-by-case comparison of recorded runs (#140) and quotation assessment with explicit source coverage (#139).
- Marker hits printed after scoring, with the screen's literal-phrase limit stated (#109).
- `CONTRIBUTING.md` with the change workflow and the checks CI runs (#165).
- `CLAUDE.md` and project settings that protect generated outputs (#122), and maintenance routing in `AGENTS.md` (#49).
- `CHANGELOG.md`, this file (#178).
- TPB Annual Report 2025-26 context in the source-status chapter, kept apart from the dated Guidance Statement count (#202).
- Builder `gate` command that checks a high-impact draft against the output contract, including the decision status, action authority, mandatory sections and reperformed arithmetic (#208).
- Builder test that a model returning `HARD_STOP` for everything fails every case expecting another status (#201).

### Changed

- Decision statuses and evidence requirements clarified after evaluation failures (#42).
- Instruction handling and source verification clarified (#54); source mappings and evidence boundaries clarified (#56).
- Prohibited conclusions held in hypotheticals (#80).
- The 1 October 2026 TASA and Code of Professional Conduct amendments flagged in the guide (#103), with a warning for `v0.3.3` users (#107).
- Pinned and reviewed compilations: Code Determination Compilation 4 (#125, #132), TASA Compilation 27 (#141), TASR Compilation 5 (#148), AML/CTF Act Compilation 63 (#177) and Taxation Administration Act Compilation 227 (#187).
- Sources reviewed on 14 October 2026 (#168); `0.4.0` prepared as an unreleased candidate (#169, #174).
- AUSTRAC guidance reviewed monthly, with the next review date named (#110).
- Private source review register requirement removed (#129).
- Verifier separates a failed check (exit 1) from a verifier that could not run (exit 2) (#68).
- Guide data cross-checked and `llms.txt` stamped from the build (#55, #126).
- Source provenance and quotation checks strengthened (#172).
- Prose aligned with the Australian Government Style Manual (#60), hand-written prose tightened (#40) and redundant disclaimers trimmed (#176).
- Builder ported to Rust: `drdebits_build` 1.0.0 replaces the Python package with the same generated files, findings and exit codes, apart from the divergences listed in `tools/drdebits_build/CHANGELOG.md`. CI runs `cargo fmt`, `clippy`, the tests on Ubuntu and Windows and line coverage (#192, #205, #206, #209). It replaces the interim Python 3.14 requirement (#133, #145).
- Release checklist status of `v0.3.2` stated accurately (#61); maintenance guidance for evaluation tools and bundle contents corrected (#114).
- README and diagrams recoloured to the website palette (#136, #137, #142, #143, #155, #160, #173), with README badge styling flattened and centred (#188, #191), a phone layout for the banner (#196), a brand footer (#200) and an opening line naming the guide and repository together (#190).
- Contributor workflow: pull requests and pushed commits carrying an AI authorship credit are refused (#47, #70, #72).
- The build tooling under `tools/` and the CI helper scripts under `.github/ci/` are licensed under FSL-1.1-ALv2 (Functional Source License, with Apache-2.0 as the future licence) instead of MIT, and the guide under FSL-1.1-ALv2 instead of CC BY 4.0. A root NOTICE joins the verified bundle. Released versions keep their licence (#183, #193, #199).

### Fixed

- AUSTRAC link repaired (#103).
- Link checker checks destinations before requesting them, and the build fails before it writes (#95).
- Link checker closes HTTP error responses (#131).
- Links on hosts that block GitHub's runners are listed for checking by hand instead of making the check incomplete (#120).
- Register records without a compilation reported as incomplete rather than misread (#124, #146).
- Evaluation evidence corrected (#102); impossible evaluation dates reported as source findings (#147).
- Disclaimer set phrase and its checksum corrected (#127).
- Attribution title matching (#52).
- Flaky concurrency probe in the test suite (#64, #65).
- The ATO advice-and-guidance page, which refuses GitHub's runners, is listed for checking by hand instead of making the link check incomplete (#178).

### Security

- Secrets and client identifiers gated locally and in CI with gitleaks and `.gitleaks.toml` (#67).
- Commit identities validated before merging (#51).
- YAML source reader fuzzed, and deeply nested YAML refused (#161).
- Two regular-expression backtracking risks removed (#162).
- Hash-pinned installs and a security policy link (#156); issue-writing jobs install only locked wheels (#164).
- Verify and CodeQL enforced through self-checking aggregate jobs (#115); required gate checks kept when a workflow or gate is renamed (#203), using the canonical gate checker, which fails on drift (#204).
- uv raised to 0.13.5 in CI for GHSA-hq85-5cpj-mr62 (#197).
- Locked dependencies tested with Snyk (#153), OpenSSF Scorecard results published (#151) and release assets scanned with VirusTotal (#150).

Not listed, as they have no user-visible effect: dependency and action pin
updates (#43, #44, #66, #86, #87, #88, #89, #90, #93, #117, #118, #119, #171, #179, #180, #181, #182, #194, #195),
attribution-policy pin moves (#53, #59, #78, #79, #100, #175), review-bot
configuration (#39, #57, #58, #62, #63, #97, #98, #128), badges and coverage
reporting (#99, #144, #149, #152, #154, #157, #158, #166, #167), and CI and
documentation housekeeping (#48, #101, #111, #135, #184, #185, #186, #189, #198,
#207).

[Unreleased]: https://github.com/ryanduguid/DrDebits/compare/v0.6.0...HEAD
[v0.6.0]: https://github.com/ryanduguid/DrDebits/compare/v0.5.0...v0.6.0
[v0.5.0]: https://github.com/ryanduguid/llm-tax-guardrails/compare/v0.4.0...v0.5.0
[v0.4.0]: https://github.com/ryanduguid/llm-tax-guardrails/compare/v0.3.3...v0.4.0
