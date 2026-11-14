# PAYG withholding and Medicare levy benchmark

Twenty questions on PAYG withholding from salary and wages, the Medicare levy adjustment to withholding, and the Medicare levy, with published keys, derivations and the same scorer as the [Payday Super and Division 7A benchmark](../payday-div7a/README.md). Each key is the figure a cited row of a primary source prints, and `bench keys` recomputes it. No model has been run on these questions. The claim is that the keys are reproducible, not that the benchmark is independent.

**Publisher conflict.** Ryan Duguid publishes this benchmark, and also publishes the `australian-tax-calculators` package, whose PAYG withholding and Medicare levy calculators it could score. The keys were not taken from or checked against that package. Read any score with that in mind.

## What each case holds

| File | What it holds |
| --- | --- |
| [questions.json](questions.json) | The instructions and questions. Give a model this file and nothing else. |
| [keys.json](keys.json) | Each key's value, the sections it accepts, its source and row, a derivation and the inputs to recompute it. |
| [schedule-1.json](schedule-1.json) | The coefficients, tax offset rates and Medicare levy parameters of Schedule 1 to the *Taxation Administration (Withholding Schedules) Instrument 2026* (F2026L00716), for payments made from 1 July 2026, copied from the instrument as made. |

Each key comes from one of these, named in the key's `source`:

| Source | Cases |
| --- | --- |
| Sample data in Schedule 1 of the instrument | 11 |
| A worked example in Schedule 1 of the instrument | 6 |
| A worked example on an ATO page | 3 |

The instrument and the ATO pages were read on 15 November 2026. The ATO publishes the same Schedule 1, dated 17 June 2026, on its [website](https://www.ato.gov.au/tax-rates-and-codes/payg-withholding-schedule-1-statement-of-formulas-for-calculating-amounts-to-be-withheld). The Medicare levy keys are for the 2025-26 income year and use the thresholds in s 3(1) and s 8 of the *Medicare Levy Act 1986* as compiled on 1 July 2026. A key dated then may go out of date.

## Reproduce the keys offline

```bash
cargo run --manifest-path tools/drdebits_build/Cargo.toml --locked --release -- bench keys --topic payg-medicare --root .
```

The command recomputes all 20 keys and compares each with the published figure. A withholding key works Schedule 1's formula `y = ax − b` from the scale's row for `x`, the weekly earnings in whole dollars plus 99 cents, rounds to the dollar and converts to the pay period, then takes off any tax offsets and Medicare levy adjustment the key records. A Medicare levy adjustment key works the weekly family threshold, the shading out point and the adjustment formula from the scale's parameters. A Medicare levy key applies ss 6, 7 and 8 of the Act to the taxable incomes and thresholds the key records, in exact arithmetic. The command exits 0 when all agree, 1 on a mismatch and 2 if it could not run. There is no engine record for this topic, so no engine check is reported.

## Score a set of answers

Send a model `questions.json` with web access off, record its answers as the [Payday Super and Division 7A benchmark](../payday-div7a/README.md#score-a-set-of-answers) describes, and run:

```bash
cargo run --manifest-path tools/drdebits_build/Cargo.toml --locked --release -- bench score answers.json --topic payg-medicare --root .
```

Value and authority are marked separately, as in that benchmark. Amounts compare by value, so `27`, `27.00` and `$27` are the same. Each section listed under a key is enough alone: s 15-10 or s 15-25 of the *Taxation Administration Act 1953* for withholding, and for the levy the one section of the *Medicare Levy Act 1986* each key lists: s 6, s 7 or s 8.

## Limits

- 20 cases is small, and the questions give the facts a payroll officer or practitioner would have to find, such as the payee's declarations.
- Most withholding keys come from the sample data the instrument publishes for testing payroll software. A model that has seen that table can repeat it without working the formula.
- The gate reads `s <section> of the <Act>` only, so the instructions ask for a section of Schedule 1 to the *Taxation Administration Act 1953* to be written as `s 15-10 of the Taxation Administration Act 1953`. A right citation in the full form `s 15-10 of Schedule 1 to the Taxation Administration Act 1953` is marked `authority_unreadable`, not wrong.
- The ATO's [family income example](https://www.ato.gov.au/individuals-and-families/medicare-and-private-health-insurance/medicare-levy/medicare-levy-reduction/medicare-levy-reduction-family-income) gives $92.90 without the working. The key recomputes it from s 7(2) and the s 8(2) formula, which the Federal Register shows as an image, read from the compilation's PDF.
- Quarterly payments, 53-pay years, study loan components, the Medicare levy surcharge and exemptions by days are not tested.
- Nothing tests when a model should refuse, which is the job of the [behaviour tests](../../tests/behaviour-tests.md).
- `questions.json` and `keys.json` carry the same canary as the behaviour cases. A model that repeats it may have seen these files. Its absence shows nothing.
- A key is what its source and derivation give at the dates shown. It is not advice on anyone's affairs.
