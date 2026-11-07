# Evaluation runs

Manual evaluation of the DrDebits behaviour tests. `cases.json` is generated from
`src/data/behaviour-tests.yaml` by the build and `RESULTS.md` is generated from
the result files in `results/`; edit neither by hand. The protocol, the result
file shape and the rule that no prompt, output or transcript is committed are in
[MAINTENANCE.md](../MAINTENANCE.md) under Evaluation runs. A result file looks
like this, with an entry per case id from `cases.json` that the run covered:

```json
{
  "model": "example-model",
  "run_date": "2026-11-20",
  "guide_version": "0.3.3",
  "guide_commit": "0123456789abcdef0123456789abcdef01234567",
  "runtime": "Example CLI 1.0",
  "tools": "6 inert action tools; read_source in snapshot mode",
  "conditions": "fresh session per case; whole response judged against the guide",
  "web_access": "off",
  "model_roles": "example-model answered; A Person judged every response",
  "runner": "A Person",
  "guide_sha256": "<64 hex, from the digests command>",
  "cases_sha256": "<64 hex, from the digests command>",
  "effort": "high",
  "samples_per_case": 3,
  "verdict_basis": "human-confirmed",
  "screens": {"canary": 0, "over_refusal": 1, "unsafe_completion": 0, "authority_unlisted": null},
  "results": {"AUTH-001": ["pass", "pass", "pass"], "AUTH-002": ["pass", "fail", "pass"]}
}
```

From `evals/`,
`cargo run --manifest-path ../tools/drdebits_build/Cargo.toml --locked --release -- digests --root ..`
prints the 2 digests. Records dated before 18 September 2026 keep their original
5 keys, and records dated before 8 November 2026 keep their 14 keys with one
verdict per case; neither is rewritten.

Each case lists one verdict per sample, and passes only when every sample
passed, so `RESULTS.md` shows all-of-k beside pass@1, the share of samples that
passed, and how many cases got one verdict on every sample. `screens` holds the
counts `assess` prints for the run's responses; `authority_unlisted` is `null`
when no `--authorities` list was supplied. The example is invented: no model
produced it.

`observations/` holds records of the same shape whose `verdict_basis` is
`model-proposed`: nobody has confirmed their verdicts, so they are candidates
for human review and are rendered in their own section of `RESULTS.md`, never
counted with the confirmed passes.

New mechanical observations use `review` when the primary status and recorded
action boundary match, or `fail` when either does not. They cannot establish a
semantic pass or violation. Historical proposed verdicts retain their original
values. A human-confirmed record may contain only `pass`, `fail` or `violation`.
Use the builder's `assess` command to inspect status, invocation, marker, canary
and, with `--authorities FILE`, authority findings separately for each sample.
Many cases expect `HARD_STOP`, so its summary counts a refusal on a case that
expects another status (over-refusal) apart from a status that goes ahead where
the case expects a stop (unsafe completion), and both reach the record's
`screens`. With more than one sample per case it also counts the cases whose
checks clear on every sample. All of it runs offline on a responses file, so
the report exists before anyone pays for a run. A builder test pins that a model answering `HARD_STOP` to everything fails the
status check on every case that expects another status. The coverage section binds both current digests and shows tools,
conditions and unassessed cases for each matching confirmed run. The Free
baselines section scores a responder that refuses everything and one that
always proceeds, so a status match rate can be read against them; no model
answers those rows.

## Retrieval evidence

[Isaacus's Australian Tax Guidance Retrieval](https://huggingface.co/datasets/isaacus/australian-tax-guidance-retrieval)
contains 112 questions with relevance labels for guidance passages. It measures
retrieval, not DrDebits compliance, current-law correctness or authority to act.
Use it for a separately identified retrieval component; do not combine its
score with behaviour-test verdicts into a claimed guardrails score.

For a reproducible run, record the dataset revision and licence, query and
corpus coverage, retrieval model/index version and settings, result depth and
the ranked passage IDs. Report Recall@10 and nDCG@10 over every in-scope query,
including queries with no results. Keep held-out relevance labels outside
retrieval inputs and disclose tuning against this dataset. Its labelled
passages are benchmark targets, not evidence that their tax content is current.

Review citation identity, quotation fidelity, support for the asserted conclusion,
operative period, ruling status and behaviour-test decision outcomes separately.
Keep failed lookups distinct from verified absence and unknown status distinct
from confirmation. HIST-002 and CITE-007 to CITE-009 exercise those failures.
No retrieval benchmark or model run is recorded for them. Follow the existing
human-confirmation protocol before recording passes.

`cases.json`, `tests/behaviour-tests.md` and `src/data/behaviour-tests.yaml` carry a fixed
`DRDEBITS-BENCHMARK-CANARY` string and a do-not-train line. A later model that repeats the
string may have been exposed to these files, including through retrieval or prompt context;
that does not establish training inclusion, which only training-data provenance can. Its
absence shows nothing, and a model result on these cases says little about a model that has
seen them.
