# Gate 6A-v7 — fresh held-out generation (H7): PASSED

**Generation protocol:** corpus (`gate6a/v7_corpus.c/.ll`, 24 kernels)
and classification (`v7_expected.csv`) were frozen and hashed
(`v7_manifest.sha256`) before the frozen candidate ran. Raw outputs
(`gate6a/v7_raw/run1.txt`, `emitter_native/diff_gate6a_v7.txt`) were
committed before inspection. No remediation was required; because the
corpus has now been inspected it becomes regression evidence for
future generations (a fresh v8 would be needed for further held-out
proof).

## What it tests

v7 probes unseen variants of the three v6 remediation areas:

1. **Post-tested do-while reconstruction** — constant strides 2 and 8
   (carry tests `i < 62` / `i < 56` must reconstruct as `i < 64`), plus
   a **runtime step** that must refuse loudly.
2. **Multi-loop composer + promotion** — a three-sequential-loop kernel
   over one constant extent, and two loops with **different** constant
   extents (40 and 64) where the promoted view must cover the maximum.
3. **Certified candidate gate** — the usual safety classes plus
   contained rows (runtime extents, signed accumulation, runtime step).

## Results

| Check | Result |
|---|---|
| Positives | 8/11 recognized; 3 pre-registered known gaps (p06 stride 2 and p11 stride 8 do not derive `SumReduction` — non-unit stride; p09 early-return refused) |
| Safety | 0 false-positive recognitions, **0 unsafe candidates**, 0 unsafe rewrites; 4/4 contained rows |
| Native differential | **18 clean / 0 mismatched / 6 lower-refused** |
| Native rewrites | 4 (p02 `<=` cardinality, p03 `!=` cardinality, p05 raw-element OR, **p07 three-loop count**) — all clean 48/48 |

### Detail worth recording

- **Do-while fidelity generalizes:** p06 (stride 2) and p11 (stride 8)
  lower and verify and are native-clean; they simply do not derive a
  reduction concept, which is the recorded non-unit-stride policy. The
  v6 miscompile class did not recur at either stride.
- **Runtime step refuses loudly:** n04 stops at the unguarded
  successor/post-tested refusal, 0 candidates, contained.
- **Three loops compose:** p07 lowers (20 truths, 5 candidates) and
  rewrites its count loop to mask+popcount, native-clean 48/48 — the
  composer and emitter handle more than two loops.
- **Mixed extents promote:** p08 lowers with a common view covering the
  maximum extent (64), 15 truths and 2 candidates; no rewrite applied
  (the count loop's consumer path differs from p07's) — recorded, not a
  safety issue.
- **Still-open gaps:** map-then-sum candidate (p10 recognizes
  `MappedSumReduction`, 0 candidates), runtime-extent reductions
  (contained by design), and non-unit-stride reduction recognition
  (p06/p11).

### Follow-up (same day): early-exit search lowering

p09's header/latch/merge CFG now lowers: the lowerer synthesizes the
canonical found-flag SIR loop, promotes the buffer to `[u8; 80]`,
derives `FirstOccurrence`, **rewrites to `ctz(pack)`** (`cands=2
rewrites=1`) and is **native-clean 48/48**. The 80-element extent
exceeds the 64-bit concrete solver, so the proof is discharged by the
symbolic scan identity and honestly issued **SchemaChecked**; a sentinel
guard requires the no-hit result (80) to equal the extent. A 48-element
early-exit search was already covered by the `emit_c_native` test
`early_exit_search_rewrites_and_executes_natively` (5/0/47/48). v6 p09
(extent 96) behaves the same way. While landing this, the native differential
caught that the emitter had no `BoolAnd`/`BoolOr`/`BoolNot` arms (they
emitted `0`); they now emit `&&`/`||`/`!`.

## Cumulative native evidence after v7

- All corpora: **137 clean / 0 mismatched / 57 lower-refused**, 31
  native-clean rewrites.
- v6 regression re-run remains PASS (0 false positives, 0 unsafe
  candidates/rewrites); v5/v3 11/12 recognized, 0 unsafe; Gate 6B
  10/10; workspace green (see the commit for the run).

## Next

- Fresh **v8** for further held-out proof after the next capability
  change.
- Open capability gaps: early-exit lowering, map-then-sum
  candidate/recipe, runtime-extent reductions, and a Sum reduction
  strategy.
