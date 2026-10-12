# work/ — external-project investigation workspace

**The rule: nothing important goes in `/tmp`.** This directory exists because
three sets of artifacts were lost that way in one session — the `hllMerge`
patch and the Redis tree, and then the entire six-application opportunity audit
(sources, binaries, workloads, callgrind profiles). Each time the result
survived only as prose in `docs/CAPABILITY_LEDGER.md`, so the finding could be
read but not acted on.

## Layout

| path | what |
|---|---|
| `audit/` | the six-application opportunity audit (`docs/AUDIT_PREREGISTRATION.md`) |
| `pr-targets/` | per-target work for an upstream contribution |
| `redis-hllmerge/` | reconstruction of the withdrawn Redis patch |

## What is committed

Checkouts, build products and raw profiles are gitignored — they are large and
regenerable from pinned versions by the scripts in `sir/scripts/`. **Patches,
notes, profile summaries, harness scripts and results are committed.** A result
that cannot be reproduced from this repository is not a result.

## Reproducing the audit

```sh
bash sir/scripts/audit-fetch.sh    work/audit   # pinned versions -> sources + binaries
bash sir/scripts/audit-workloads.sh work/audit  # deterministic inputs from the frozen commit
bash sir/scripts/audit-run.sh noise            # noise floor, before any measurement
```
