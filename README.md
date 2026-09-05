# DrDebits

```
+----------------------------------------------------------------------+
|                               DrDebits                               |
+----------------------------------------------------------------------+
|          APES 110 and TPB Code guardrails for LLM tax work           |
+----------------------------------+-----------------------------------+
| DR  what it gives you            | CR  what it needs                 |
+----------------------------------+-----------------------------------+
| APES 110 mapped guardrails       | drdebits.md as project context    |
| TPB Code GS01 to GS55 map        | Claude Code, Antigravity, Cursor  |
| risk classified draft or stop    | -                                 |
+----------------------------------+-----------------------------------+
```

[![APES 110 aligned](https://img.shields.io/badge/APES%20110-Aligned%20Guardrails-5C2D91.svg?labelColor=04001F)](reference/apes-110-map.md)
[![TPB Code of Conduct](https://img.shields.io/badge/TPB%20Code-GS01--GS55%20Mapped-5C2D91.svg?labelColor=04001F)](reference/tpb-catalogue.md)
[![Jurisdiction](https://img.shields.io/badge/Jurisdiction-Australia%20%F0%9F%87%A6%F0%9F%87%BA-4F485E.svg?labelColor=04001F)](#drdebits)
[![Claude Code](https://img.shields.io/badge/Claude%20Code-Skill%20%26%20Rule%20Ready-5C2D91?logo=anthropic&logoColor=white&labelColor=04001F)](#quick-start-for-ai-agents)
[![License: CC BY 4.0](https://img.shields.io/badge/License-CC%20BY%204.0-4F485E.svg?labelColor=04001F)](LICENSE)

> Australian tax-practice and accounting-ethics guardrails for LLM-assisted work
>
> Version: `0.3.2` · Jurisdiction: Australia · Sources last checked: 2026-08-16

DrDebits is an independent, source-linked operating guide for large language models assisting with Australian accounting, tax and BAS work. It converts the Tax Practitioners Board framework, APES 110, APES 220 and the sector's AML/CTF obligations into practical controls for drafting, research, calculations and review.

DrDebits does not reproduce APES 110, certify compliance, replace the source documents or replace a registered tax practitioner's or professional accountant's judgement. It is not legal, tax or financial advice. A competent, appropriately authorised human remains responsible for every professional service, judgement and consequential action.

"Dr" is part of the project name only. It does not claim a qualification, professional designation, registration, regulatory status or endorsement.

---

## Quick start for AI Agents

Supply `drdebits.md` as persistent project context at the highest configurable instruction tier beneath immutable platform controls, then instruct the model per the "How to use this file" section of the guide. Retrieve the reference files when a routing decision needs them.

### Load into Claude Code / Antigravity
```bash
# Add as persistent project context in your repository
mkdir -p .claude/rules
cp drdebits.md .claude/rules/drdebits.md
```

### Configure for Cursor (`.cursorrules`)
```markdown
# Include in your .cursorrules or project prompt:
Follow Australian accounting ethics and statutory boundaries defined in @drdebits.md.
All tax positions must be cited against primary ATO/Commonwealth sources.
```

---

## Ethical Routing & Boundary Architecture

```mermaid
%%{init: {"themeVariables": {"lineColor": "#B1AFAD"}}}%%
flowchart TD
    Prompt["Accounting or tax prompt"] --> Gate["Intake gate<br/><i>scope, facts, sources, authority, confidentiality, interests, consequences</i>"]
    Gate --> Risk{"Risk classification"}

    Risk -->|"Low impact"| Prop["Proportionate answer<br/><i>general information, operative source cited</i>"]
    Risk -->|"High impact or uncertain"| Flow["Mandatory workflow<br/><i>primary-source grounding, reperformed calculations, human review</i>"]

    Flow --> Status{"Decision status"}
    Status -->|"PROCEED_DRAFT_ONLY"| Draft["Draft for human review<br/><i>banner, citations, assumptions recorded</i>"]
    Status -->|"NEEDS_FACTS"| Facts["Focused questions back to the human"]
    Status -->|"ESCALATE or HARD_STOP"| Stop["No autonomous resolution<br/><i>responsibility stays with the authorised human</i>"]

    style Prop fill:#140E24,stroke:#4F485E,stroke-width:2px,color:#FFFFFF
    style Flow fill:#1E1236,stroke:#5C2D91,stroke-width:2px,color:#FFFFFF
    style Draft fill:#2D184E,stroke:#8A4AC7,stroke-width:2px,color:#FFFFFF
    style Stop fill:#2A0A12,stroke:#C02B0A,stroke-width:2px,color:#FFFFFF
```

The diagram summarises the guide's own gate, classification, workflow and decision statuses; the guide text controls.

---

## Files & Governance

| File | Purpose |
|---|---|
| [drdebits.md](./drdebits.md) | The guide: core operating controls. Load this as persistent context. |
| [tests/behaviour-tests.md](./tests/behaviour-tests.md) | Adverse-case tests an implementation must pass |
| [evals/RESULTS.md](./evals/RESULTS.md) | Recorded manual evaluation runs: model, date and a pass or fail per behaviour test. Not in the bundle. |
| [reference/tpb-catalogue.md](./reference/tpb-catalogue.md) | Complete live TPB Guidance Statement catalogue, GS01 to GS55 |
| [reference/apes-110-map.md](./reference/apes-110-map.md) | Primary APES 110 reference map |
| [AGENTS.md](./AGENTS.md) | Routing instructions for autonomous coding agents |
| [DISCLAIMER.md](./DISCLAIMER.md) | General disclaimer: no advice, no agent-client relationship, human responsibility |
| [MAINTENANCE.md](./MAINTENANCE.md) | Release and source-check protocol |
| [SHA256SUMS](./SHA256SUMS) | Digests of the eight files in the verified guide bundle |

---

## Integrity

Pin the approved release tag, retrieve `SHA256SUMS` at that tag and verify each file's digest before use. The `DRDEBITS-END-…` marker on the guide's final line is a truncation check only, not tamper evidence.

## Licence

Copyright © 2026 Ryan Duguid. Original DrDebits material is licensed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/); see [LICENSE](./LICENSE). The build tooling under `tools/` is separately licensed under the [MIT License](./tools/drdebits_build/LICENSE). The licences do not extend to third-party standards, quotations, logos, trade marks or source documents. DrDebits is not endorsed by the TPB, APESB, AUSTRAC, IFAC, CA ANZ, CPA Australia or IPA.

## Contributing and maintenance

The guide, reference, test and evaluation files are generated. Edit the sources under `src/` and run `uv run --project tools/drdebits_build python -m drdebits_build build`; CI rejects hand edits to generated files. See `MAINTENANCE.md` for the release protocol.
