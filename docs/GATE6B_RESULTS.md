# Gate 6B — automated multi-reduction fusion (closes Gate 5B)

**Status (2026-09-17): Gate 5B closed — 5B-Automation PASSED and
5B-Search PASSED.** The system generated the fused single-pass
implementation from primitive actions (per-region recognition + target
plans + a shared-traversal composition) with no hand-written fused source,
and the composition search selected the fusion where the deterministic
pipeline (Engine 0) cannot — on a corpus that was sealed *before* any
fusion automation existed.

## Evidence chain

| Artifact | Commit / hash |
|---|---|
| Sealed fusion corpus + expectations (`gate6b/`) | `4aedce1`, `gate6b/manifest.sha256` |
| Harness frozen + hashed (extractor, composition, emitters, search, runner) | `7ab8edc`, `gate6b/manifest.harness.sha256` |
| Sealed run 1 (raw, committed before inspection) | `915e163`, `gate6b/raw/run1.txt` |
| Harness accounting fix + re-freeze | `2594d1a` |
| Sealed run 2 (raw, committed before inspection) | `04ec483`, `gate6b/raw/run2.txt` |

Run 1 produced no fusion on any negative row; two negative rows refuse at
analysis/lowering (a store loop that clang lowers to `llvm.memset`, and a
descending scan with a signed `icmp`), and the harness counted those
refusals as expectation failures. The corpus criterion for N rows is "no
fused plan + recorded reason", so the harness accounting was corrected
(no corpus or expectation change) and the unchanged sealed corpus was
re-run as run 2. Both raw runs are preserved; run 1's outcomes and run 2's
outcomes agree on every kernel decision.

## What was built (the automation)

```text
multi-loop kernel (.ll)
  → extract_loop_regions      one single-loop region per reduction;
                              original params keep their positions,
                              cross-loop values become dependency params
  → per-region lowering + analysis + semantic recognition
  → per-region primitive plan (vector_derive, region-scoped: buffer and
                              length resolved from the loop, not param 0/1)
  → composition               group regions sharing buffer + length with
                              no dependency edge
  → search                    action space {engine0 per-region, fuse any
                              subset} × memory-traffic cost model
  → fused emitter             ONE pass, ONE load per chunk, per-reduction
                              targets (movemask+popcount, psadbw,
                              cmpeq+movemask) + 16-byte + scalar tails
  → differential + benchmark  scalar reference oracle; in-place 3vec
                              baseline; CPU-pinned medians
```

No fused source is written by hand anywhere in the path; the fused C is
generated from the recognized per-region plans. Search choices are
validated by measurement, not asserted.

## Sealed-corpus results (run 2)

Tier P (must fuse; pre-registered `min_speedup_vs_3vec`):

| Kernel | Reductions | Fused plan | Checks | ratio @4096 (equal) | ratio @65536 (equal) | Speedup vs Engine 0 | Required |
|---|---|---:|---:|---:|---:|---:|---:|
| g6p01_triad | count+sum+all | fuse all 3 | 399 / 0 fail | 0.505 | 0.482 | **1.98–2.07×** | 1.20 |
| g6p02_count_sum | count+sum | fuse 2 | 266 / 0 fail | 0.560 | 0.567 | **1.76–1.78×** | 1.10 |
| g6p03_sum_all | sum+all | fuse 2 | 266 / 0 fail | 0.590 | 0.549 | **1.70–1.82×** | 1.10 |
| g6p04_two_predicates | 2×count+sum | fuse all 3 | 399 / 0 fail | 0.561 | 0.563 | **1.78×** | 1.20 |
| g6p05_literals | count+sum+all (literals) | fuse all 3 | 399 / 0 fail | 0.511 | 0.490 | **1.96–2.04×** | 1.20 |

Ratios are `plan_ns / engine0_ns` on the acceptance distribution (all
bytes equal to the All predicate value, so an independent All loop cannot
early-exit); the random distribution is reported in the raw output and
agrees within a few percent. Correctness compares every per-reduction
result against an independent scalar reference over 19 sizes × 7
distributions; zero mismatches on every row.

Tier N (must refuse fusion):

| Kernel | Refusal | Stage |
|---|---|---|
| g6n06_two_buffers | distinct buffer bases | composition |
| g6n07_half_domain | same base, different domains (n vs n/2) | composition |
| g6n08_store_loop | store loop (clang `llvm.memset`) is not a reduction | analysis/lowering |
| g6n09_reverse | descending traversal outside the forward dialect | analysis/lowering |
| g6n10_dependent | second domain depends on the first result (dependency param) | composition |

`GATE6B_SUMMARY pass=10 fail=0`.

## Gate 5B-Search

Engine 0 has only the per-region action; it vectorizes each loop
independently (3vec). The enriched action space adds the fusion primitive
for any compatible subset, priced by a memory-traffic cost model (one
shared load per chunk) with an early-exit discount for an independent All
loop.

- On the sealed corpus the search selects a fusion plan on all five P rows
  (e.g. triad: `fuse_3` modelled 2.200 vs Engine 0's 4.200) and the
  measured result is 1.7–2.1× faster, validating the model.
- Sensitivity: under early-exit-favouring assumptions (random data, All
  exits at the first mismatch) the same model prefers a *mixed* plan —
  fuse count+sum, keep the All loop independent — which was also measured
  faster in the development harness. The distribution-dependent choice is
  exactly the interaction documented in `docs/GATE5B_RESULTS.md`
  ("Early-Exit Interaction"); the search now makes that choice explicitly
  instead of relying on one hand-written fused kernel.

## Protocol and provenance

- The corpus was authored from the documented Gate 5B challenge and the
  Gate 5A recognized dialect, sealed (manifest) and committed (`4aedce1`)
  before any fusion automation existed. It was not run or inspected during
  automation development: development used separate kernels under `/tmp`.
- The harness was frozen and hashed before the runs
  (`gate6b/manifest.harness.sha256`). Raw outputs were committed before
  inspection in both runs.
- The harness manifest covers apparatus code only; the run-time manifest
  (which also pinned the Gate 5B specification document) is preserved in
  git history at `7ab8edc`/`2594d1a`. `docs/GATE5B_RESULTS.md` was
  annotated with the closed status after the runs, so it is covered by
  git history rather than by the code manifest.
- Independence caveat (disclosed): the corpus author is the same agent
  that implemented the automation; the operative criteria are
  pre-registration, the seal, the frozen harness, and the one-shot runs.

## Scope honesty

- **Plan-level composition, SIR-level plans.** The fused C is emitted from
  per-region primitive plans; the pipeline's assurance levels are
  unchanged (`SchemaChecked` for the reduction families; Gate 4B's
  machine-checked evidence covers the three development kernels, not this
  emitter).
- **Region outlining, not multi-loop SIR.** `extract_loop_regions` gives
  each loop its own single-loop function while preserving parameter
  positions and dependency edges. Lowering a multi-loop function into one
  SIR function (the "two-loop lowering" queue item) remains open; it is
  not required for this result and is not claimed.
- **Coverage limits found by the sealed run:** the outward-extraction pass
  can pull an unrelated pre-block instruction into the first region (the
  store-loop kernel's `llvm.memset`), and descending scans keep refusing
  at the signed-`icmp` gate. Both are refusals — they cannot cause a false
  fusion — but they are recorded coverage gaps for future recall work.
- **Not native-object equivalence.** The emitted C is compiled and
  differentially executed in the harness; no claim is made about ABSAC's
  broader emitter (defects F6–F8 remain open for loop sources).
- **Performance is machine-specific.** Median-of-25 trials × 5000
  iterations, CPU-pinned (core 0), AVX2/SSE2/`-mpopcnt` with
  `clang -O2 -march=native`. Absolute numbers are in `run2.txt`.
