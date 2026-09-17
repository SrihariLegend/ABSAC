# Gate 6A-v9 — fresh held-out generation (H9): PASSED

**Generation protocol:** the 29-kernel corpus
(`gate6a/v9_corpus.c/.ll`) and its classification (`v9_expected.csv`)
were frozen and hashed (`v9_manifest.sha256`) before the frozen
candidate ran. Raw outputs (`gate6a/v9_raw/run1.txt`,
`emitter_native/diff_gate6a_v9.txt`) were committed before inspection.
No remediation was required; the corpus is now regression evidence (a
fresh v10 would be needed for further held-out proof).

## What it tests

The generation blind-tests the position-predicate extraction (the
search recipe derives the hit comparison from the loop's position
select, negation- and swap-normalized):

- operators **Eq, Ne, Lt, Ge, Gt** (and zero/literal/parameter scalars);
- element widths **u8, u16, u32**;
- extents in the concrete-solver range (48/56/64) and the symbolic
  range (72/80/96/128);
- **swapped operands** (`key < elem` → `Gt`, `key >= elem` → `Le`);
- refusal discipline: a **compound** predicate (`(x & 1) && x > 3`) and
  a sentinel-mismatched sub-range search must be recognized but never
  masked;
- stable cardinality/multi-loop positives and the recorded safety
  classes / contained rows.

## Results

| Check | Result |
|---|---|
| Positives | 15/17 recognized; 2 pre-registered known gaps (p16 runtime-extent search, p17 stride-4 fidelity probe) |
| Safety | 0 false-positive recognitions, **0 unsafe candidates**, 0 unsafe rewrites; 3/3 contained rows |
| Native differential | **24 clean / 0 mismatched / 5 lower-refused** |
| Native rewrites | **13** — p01–p10 (all predicate probes), p11 cardinality, p12 two-loop, p13 (Eq u32, extent 128) — all 48/48 clean |

### Discipline confirmed

- Every extracted predicate mask is native-correct against clang's
  original IR: Eq/Ne/Lt/Ge/Gt, zero and literal scalars, swapped
  operands, u8/u16/u32 elements, and the >64 symbolic path.
- p14 (compound predicate) and p15 (sentinel 128 over extent 48) both
  recognize `FirstOccurrence` with 2 candidates but apply **0
  rewrites** — no mask is approximated.
- Lower refusals remain the documented classes (atomic load, may-alias
  store, early-exit global write, unsigned reverse search,
  runtime-extent search).

## Cumulative native evidence

- All corpora: **185 clean / 0 mismatched / 66 lower-refused**, 54
  native-clean rewrites.
- v6/v7/v8 regression re-runs unchanged; v5/v3 11/12 with 0 unsafe;
  Gate 6B 10/10.

## Next

- Fresh **v10** after the next capability change.
- Open gaps: runtime-extent reductions, multi-loop + early-exit
  composition (break-carrying peeled shape), map-then-sum
  candidate/recipe, Sum reduction strategy, deliberate non-unit-stride
  abstention.

### Follow-up (same day): runtime-extent search lowering

p16 (`p16_runtime_extent_search`) now lowers with the guarded
entry/merge-clamp form handled by the found-flag synthesis (the
`llvm.umin(phi, n)` merge instruction is emitted after the loop and the
`n == 0` predecessor is validated). The pointer is preserved, the
extent is not fabricated, the candidate proof fails, and the function
is native-clean with 0 rewrites; v9 lower-refused drops 5 → 4.
