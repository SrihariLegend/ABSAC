# Opportunity audit — results

Against `docs/AUDIT_PREREGISTRATION.md` (frozen 2026-09-27, commit `687b347`).
Outcomes use the frozen six categories; refusals stay in the denominator.

## Summary

| # | Application | Sites | intake-failed | unsupported | unproved | identical | unprofitable | profitable |
|---|---|---|---|---|---|---|---|---|
| A1 | GNU grep | 4 | **4** | | | | | |
| A2 | zstd | 4 | **4** | | | | | |
| A3 | jq | 4 | **4** | | | | | |
| A4 | sqlite | 4 | **4** | | | | | |
| A5 | libpng | 4 | **4** | | | | | |
| A6 | lua | 4 | **4** | | | | | |
| | **total** | **24** | **24** | 0 | 0 | 0 | 0 | **0** |

**Useful-patch yield: 0/24 = 0%. Cost-weighted coverage: 0%. Saved cost: 0.**
Projects with a certified improvement: 0 of 6.

**Success criterion A (two independent certified improvements): not met.**
**Success criterion B (≥1 exceeding 5% end-to-end): not met, and not evaluable in
substance — there was nothing to measure.** Criterion C therefore never arises.

**No proposal was ever constructed.** Zero sites reached `unsupported`, which is the
category meaning "lowered, but no family's pattern applies". The families did not run on
any of the 24 sites, so **their coverage of real code is untested by this audit** — the
audit measured the lowerer's intake, not the domain hypothesis.

## Every site, with tag and refusal reason

Sites are tagged `S1` scan/reduction/set, `S2` other numeric, `S3` pointer/control-heavy.
Share is of the application's own instructions (libc excluded — it is not ours to
transform).

| App | Site | Tag | Share | Refusal reason |
|---|---|---|---|---|
| A1 | `bmexec_trans` | S1 | 97.45% | external call `@memchr2` |
| A1 | `Fexecute` | S3 | 1.19% | external call `@kwsexec` |
| A1 | `grepbuf` | S3 | 0.73% | external call `@rawmemchr` |
| A1 | `bmexec` | S3 | 0.15% | external call `@memchr2` |
| A2 | `ZSTD_compressBlock_doubleFast` | S1 | 55.48% | multiple loops sharing an exit CFG |
| A2 | `ZSTD_encodeSequences_bmi2` | S2 | 26.68% | loop with a separate latch block |
| A2 | `ZSTD_buildSequencesStatistics` | S1 | 4.98% | external call `@HIST_countFast_wksp` |
| A2 | `ZSTD_splitBlock` | S3 | 1.99% | external call `@HIST_add` |
| A3 | `jv_parser_next` | S3 | 21.16% | external call `@jv_invalid` |
| A3 | `jv_free` | S3 | 14.05% | external call `@__assert_fail` |
| A3 | `jq_next` | S3 | 13.55% | external call `@jv_nomem_handler` |
| A3 | `jv_array_get` | S3 | 1.43% | external call `@__assert_fail` |
| A4 | `sqlite3VdbeExec` | S3 | 38.72% | external call `@strlen` |
| A4 | `vdbeSorterCompareInt` | S2 | 14.09% | branch-guarded memory (path predication) |
| A4 | `sqlite3BtreeInsert` | S3 | 7.27% | branch-guarded memory (path predication) |
| A4 | `moveToRoot` | S3 | 0.98% | branch-guarded memory (path predication) |
| A5 | `png_write_find_filter` | S1 | 92.68% | external call `@png_write_flush` |
| A5 | `count_zero_samples` | S1 | 7.24% | external call `@png_error` |
| A5 | `png_do_read_transformations` | S3 | 0.02% | external call `@png_error` |
| A5 | `png_calculate_crc` | S1 | 0.01% | external call `@crc32` |
| A6 | `luaV_execute` | S3 | 97.75% | external call `@luaG_tracecall` |
| A6 | `internshrstr` | S3 | 0.22% | external call `@luaC_fullgc` |
| A6 | `lua_tolstring` | S3 | 0.15% | external call `@luaO_tostring` |
| A6 | `luaH_getshortstr` | S3 | 0.06% | loop with a separate latch block |

## What the refusals are made of

Sorting the 24 reasons into classes:

| Refusal class | Sites | Share |
|---|---|---|
| **the code calls a function we do not model** | **18** | **75%** |
| branch-guarded memory through pointer operands | 3 | 12.5% |
| loop with a separate latch block | 2 | 8.3% |
| multiple loops sharing an exit CFG | 1 | 4.2% |

Within the 18 call refusals, both kinds appear: **libc** (`memchr2`, `rawmemchr`,
`memchr`, `strlen`, `crc32`, `__assert_fail`) and **the application's own functions**
(`png_error`, `jv_invalid`, `HIST_add`, `luaG_tracecall`).

## The finding

**The binding constraint is intake, not domain recognition.** The audit was designed to
test whether profitable transformations concentrate in scan/reduction shapes, and it
cannot answer that question, because **the lowerer accepted none of the 24 sites** — not
one of the six projects, not at 97% of the workload or at 0.01%.

This inverts the project's own account of its problem. The recorded story is that clang
vectorizes the target shapes before the lowerer sees them; the remedy implied was better
or different *recognition*. What actually happens is one stage earlier: **we refuse to
read the code.** A single unmodelled call — an `assert`, an error-reporting helper, a
`memchr` — is enough to reject a function that is otherwise exactly the shape the
families exist to transform.

Three consequences worth separating:

1. **The domain hypothesis is untested, not refuted.** Astra's W1 warning — that our
   apparent concentration in algebraic shapes may be an artifact of our own selection and
   vocabulary — is neither confirmed nor falsified here. A 2×2 of tag against outcome has
   an empty outcome row. The honest statement is that this sample produced no evidence
   either way about where wins are, and a great deal of evidence about where they are
   *not even searched for*.
2. **This contradicts the standing advice to leave the lowerer alone.** The advisor's
   instruction was "do not broaden the lowerer to enlarge the intake funnel". This audit
   says the intake funnel *is* the constraint. Either the advice assumed intake was
   adequate, or it was about a different goal (not inflating `sir_lower` for its own
   sake). The tension is recorded rather than resolved: the advice and the measurement
   disagree, and the measurement is the newer fact.
3. **Yield is zero, so no criterion was met.** Not "almost": every one of the 24 sites
   died at the first stage, including the S1 sites that the concentration thesis predicts
   should be the winning class. libpng's `png_calculate_crc` — a CRC over a buffer, about
   as close to a counted single-stream reduction as real code gets — is refused because it
   calls `crc32`.

## Predictions, scored

| # | Frozen prediction | Result |
|---|---|---|
| A1 | Yield expected (the wedge) | **Falsified** — 4/4 intake-failed |
| A2 | Partial (bit paths, not the match finder) | **Falsified** — 4/4 intake-failed |
| A3 | No yield expected | Consistent — 4/4 intake-failed |
| A4 | No yield in the VDBE; possible in page byte work | Consistent — 4/4 intake-failed |
| A5 | Yield expected (row filters) | **Falsified** — 4/4 intake-failed |
| A6 | No yield expected | Consistent — 4/4 intake-failed |

Both predictions of yield were wrong, and so were both of the "possible" qualifiers. The
three "no yield" predictions were right, but for a reason the prediction did not name:
they did not fail because the domains were wrong, they failed at the same intake wall as
the ones predicted to succeed. **A prediction that is right for the wrong reason is not
evidence for the reason.**

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
