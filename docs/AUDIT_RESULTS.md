# Opportunity audit — results

Against `docs/AUDIT_PREREGISTRATION.md` (frozen 2026-09-27, commit `687b347`).
Outcomes use the frozen six categories; refusals stay in the denominator.

## Summary

| # | Application | Sites | intake-failed | unsupported | unproved | identical | unprofitable | profitable |
|---|---|---|---|---|---|---|---|---|
| A1 | GNU grep | 4 | 4 | | | | | |
| A2 | zstd | — | | | | | | |
| A3 | jq | — | | | | | | |
| A4 | sqlite | — | | | | | | |
| A5 | libpng | — | | | | | | |
| A6 | lua | — | | | | | | |

---

## A1 — GNU grep

**Prediction (frozen): "Yield expected. If the wedge cannot produce a certified
profitable patch here, the wedge is dead and that is the audit's most important
result."**

**Result: 4/4 sites `intake-failed`. The prediction is falsified.**

Profile (callgrind, `Ir`, corrected — see addendum 2): 266,783,794 instructions total,
of which 257,335,787 (96.5%) are in grep's own code; the rest is libc and the dynamic
loader.

| Site | Tag | Self cost | Share of app | Outcome | Reason |
|---|---|---|---|---|---|
| `bmexec_trans` | S1 scan/reduction/set | 250,768,233 | **97.45%** | `intake-failed` | unsupported external call `@memchr2` |
| `Fexecute` | S3 pointer/control-heavy | 3,065,116 | 1.19% | `intake-failed` | unsupported external call `@kwsexec` |
| `grepbuf` | S3 pointer/control-heavy | 1,866,624 | 0.73% | `intake-failed` | unsupported external call `@rawmemchr` |
| `bmexec` | S3 pointer/control-heavy | 377,104 | 0.15% | `intake-failed` | unsupported external call `@memchr2` |

**Ordering deviation, recorded.** `bmexec_trans`'s outcome was observed *before* its tag
was assigned; the tag was then written from the source. The other three were tagged
before evaluation. The deviation is disclosed rather than silently repaired. It cannot
have flattered the result: the tag `S1` is the one the concentration thesis *predicts*
should yield, and this site is listed under it while failing.

### What this means

The declared wedge is **bounded unsigned first-match scans**. `bmexec_trans` is the
bounded first-match scan that the world actually runs — GNU grep's Boyer-Moore executor
— and it is 97.45% of this workload. **The lowerer cannot ingest it**, because it calls
`memchr2`, and the lowerer refuses external calls.

This is not a tuning failure and not a missing family. Every one of the four sites is
refused at intake, and **three of the four are refused for calling a libc memory
primitive** — `memchr2`, `rawmemchr`, `memchr` — which is precisely what makes grep's
scan fast. The refusal is *upstream* of any question about what we could have proven.

Two distinct walls, in order:

1. **Intake.** Real scan code is built on libc primitives we do not model, so we never
   reach the question of whether we could improve it.
2. **Even past intake, there is nothing to win here.** `bmexec_trans` is hand-written
   SIMD. Beating it requires emitting better vector code, which is not what any family
   does. Astra's W2 applies exactly: this is a **product null against a strong
   baseline**, not evidence of headroom.

Neither wall is the one the project's story assumed. The story was that clang vectorizes
our target shapes before we see them; the observed obstacle is earlier and blunter — we
cannot see the code at all.

**Caveat, stated so the result is not over-read.** One workload, one literal pattern,
one project. This falsifies the prediction for A1 as measured; it does not establish that
no bounded scan anywhere is reachable. It does establish that the canonical one is not.
