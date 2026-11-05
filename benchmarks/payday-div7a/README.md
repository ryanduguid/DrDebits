# Payday Super and Division 7A benchmark

Thirty-one questions on Payday Super due dates and Division 7A, with published keys, derivations and a scorer. No model has been scored. The claim is that the keys are reproducible, not that the benchmark is independent.

**Publisher conflict.** Ryan Duguid publishes this benchmark, and also publishes `payday-super-checker`, `div7a-loan-review` and `aus-accounting-mcp`, which it could score. The keys were checked against 2 of those products, and `business-days.json` is copied from one of them. Its table's sources were checked on 15 August 2026. The rule check and the engine check therefore share one calendar for all 14 date keys, and an ATO example stands apart from it for PDS-001 to PDS-007 alone. Read any score with that in mind.

## What each case holds

| File | What it holds |
| --- | --- |
| [questions.json](questions.json) | The instructions and questions. Give a model this file and nothing else. |
| [keys.json](keys.json) | Each key's value, the sections it accepts, its source, a derivation and a rule to recompute it. Its `authority_rule` says how several listed sections work. |
| [business-days.json](business-days.json) | The weekdays from 1 July 2026 to 31 August 2027 that are public holidays for the whole of a State or Territory, copied from the `payday-super-checker` table at the recorded commit. That table's sources were checked on 15 August 2026, not for this benchmark. |
| [engine-check.json](engine-check.json) | Each case's request to an engine, the output, where the value sits in that output and the value read from it. |

Each key comes from one of these, named in the key's `source`:

| Source | Cases |
| --- | --- |
| A worked example on an ATO page | 9 |
| The ATO's benchmark rate table | 4 |
| The ATO's Division 7A calculator, as recorded on 7 October 2026 | 3 |
| The statute, worked in the derivation | 15 |

The 3 calculator figures come from the [recorded comparison](https://github.com/ryanduguid/australian-accounting/blob/2384216acea5b77484f2ac1aaee4c97a08cd2593/packages/div7a-loan-review/docs/ato-calculator-comparison-2026-10-07.md). The calculator was not rerun for this benchmark. All ATO pages and compilations were read on 6 November 2026, and a key dated then may go out of date.

## Reproduce the keys offline

```bash
cargo run --manifest-path tools/drdebits_build/Cargo.toml --locked --release -- bench keys --root .
```

The command recomputes each key that follows from a rule: a business-day count from the published calendar, or the s 109E(6) formula in exact arithmetic, rounded half up. The other 10 are source-derived keys not independently recomputed: the command does not read their sources. It takes each engine value from the recorded `output` by that entry's `read` field and compares all 31 with the keys. It refuses a duplicate record and reports an entry whose `value` disagrees with its `output`. It exits 0 when all agree, 1 on a mismatch and 2 if it could not run. Each `engine-check.json` entry is the JSON to send to that command of the engine at the recorded commit, so a line can be replayed by hand.

Three `read` forms change what the engine said, which `output` keeps. `percent` shows a rate as a percentage. `whole-dollars` rounds the cents half up to the whole dollars the ATO example shows, for D7A-005 and D7A-006. `none-for-year-of-loan` records `NONE` for D7A-016, where the engine gives no repayment and the code `REFUSED_YEAR_IS_YEAR_OF_LOAN`.

## Score a set of answers

Send a model `questions.json` with web access off and record its answers in a file outside this repository:

```json
{"answers": {"D7A-001": {"value": "8.77", "authorities": ["s 109N(2) of the Income Tax Assessment Act 1936"]}}}
```

```bash
cargo run --manifest-path tools/drdebits_build/Cargo.toml --locked --release -- bench score answers.json --root .
```

The scorer marks 2 things separately, because a right figure can rest on the wrong section:

- **Value:** correct, wrong or unanswered, and a blank value is unanswered. Dates and tokens compare as text. Amounts compare by value, ignoring `$`, commas and a trailing `%`.
- **Authority:** right when every citation is readable and names a section the key lists, at section level, so `s 109N(2)` meets `s 109N`. Each listed section is enough alone: for the Payday Super keys either s 6 or s 18C counts as right, because each states the period relied on. `authority_mixed` counts answers with a right section beside a readable section outside the key, so citing every section at once earns no `authority_right`. `authority_partly_unreadable` counts answers whose readable citations are all right beside one the gate cannot read, because it could be wrong. `wrong_authority` counts answers with no right section among their readable citations. `authority_unreadable` counts answers citing only forms the gate cannot read, such as `Division 7A`. `authority_missing` counts answers with no citation.

`correct_value_wrong_authority` counts the right figure on the wrong section. Add `--json` for each question's marks.

## Limits

- The scope is 2 narrow topics. No state tax, employment or payroll tax case is here.
- 31 cases is small, and the questions name most of the facts a practitioner would have to find.
- Every question has a keyed answer. Nothing tests when a model should refuse, which is the job of the [behaviour tests](../../tests/behaviour-tests.md).
- Earnings paid before 1 July 2026 are out of scope, because the engine refuses them. So are exceptional circumstances determinations, which it does not model, and holidays that cover only part of a day. A date range across those was avoided.
- Melbourne Cup Day, Tasmania's Easter Tuesday and Western Australia's King's Birthday on 28 September 2026 are avoided too: each has a local variation that the keys would have to decide.
- Engine agreement is narrow. For D7A-005 to D7A-010 the engine received the key's own principal, rate and term, so agreement shows the arithmetic only. For D7A-008 the engine gives 19715.97 on 100000, and the 80000 was typed into its request.
- Authority marks are mechanical. A cited section can be right and still be misapplied.
- `questions.json` and `keys.json` carry the same canary as the behaviour cases. A model that repeats it may have seen these files. Its absence shows nothing.
- A key is what its source and derivation give at the dates shown. It is not advice on anyone's affairs.
