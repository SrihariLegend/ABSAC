# Gate 6A-v3 — fresh blind recognition corpus (Generation H3-recog)

Status: **sealed before the run** (see `v3_manifest.sha256`). This corpus
exists to close the open gate

> Gate 6A-v3: OPEN — requires a fresh corpus after D3 remediation

following the generational protocol used for H0/H1/H2: classify and seal
the corpus before the frozen candidate runs; run once; commit the raw
output before inspection; promote failures to the next regression set.

## Why v3 is different from v1/v2

D3 (certificate-gated candidate generation, overflow semantics, mandatory
verification) and D4 (total analyses, dead-projection classification,
loop-domain normalization, implicit element-nonzero) landed after the H2
evaluation. v3 therefore re-tests the historical failure classes on fresh
kernels *and* probes classes that H2 exposed:

- signed overflow accumulation (H2 X02 class),
- partial-truth candidate generation (H2 X06 running-max class),
- footprint completeness with two arrays (H2 N15 class),
plus the standing abstention classes (effects, stride, reversal, atomics,
conditional accumulators, self-recurrence, early exit with side effects).

## Corpus

24 kernels (`v3_corpus.c`, compiled with the freeze flags
`clang -O1 -emit-llvm -S`):

- 12 positives: Cardinality (u16 `>=`, u32 `>=` constant bound, u8 `!=`,
  do-while), Sum (u8→u32, u16→u64), All (i8 `!=`, u64 `==`, u8 `>=`),
  Disjunctive (raw-byte OR), plus two pre-registered known-gap probes
  (map-then-sum, two-loop).
- 12 negatives: signed nsw sum, running max, two-array relation,
  may-alias store, volatile load, stride 2, dynamic stride, conditional
  accumulator, self-recurrence, early exit with a global write, descending
  scan, C11 atomics.

Pre-registered classifications are in `v3_expected.csv`.

## Pass criteria (pre-registered)

Safety (the gate's decisive criteria):

1. **Zero** false-positive *reduction* recognitions on negative rows
   (no `CardinalityReduction` / `SumReduction` / `ConjunctiveReduction` /
   `DisjunctiveReduction` concept on a negative).
2. **Zero** candidates generated on negative rows.
3. **Zero** rewrites applied on negative rows.

Recall:

4. Every positive row not marked `recall_gap_known` must lower, pass SIR
   verification, and be recognized with its pre-registered concept.
5. Rows marked `recall_gap_known` (p11 map-then-sum, p12 two-loop) are
   recorded as known misses, not gate failures; recognition there is a
   positive surprise reported as an improvement.

The gate verdict is PASS only if safety criteria 1–3 hold and no
unexpected recall miss occurs (criterion 4).

## Frozen candidate

The evaluated pipeline is the repository at the seal commit (recorded in
the results document); the evaluation harness is
`sir/crates/sir_benchmarks/src/bin/gate6a_v3_run.rs` (hashed in
`v3_manifest.sha256`), running the standard pipeline with
`OptimizerConfig::default()` and the default registry (Any is enabled
since the H4 closure).

## Run (one-shot)

```bash
cargo run -q -p sir_benchmarks --bin gate6a_v3_run -- \
  --corpus gate6a/v3_corpus.ll --expectations gate6a/v3_expected.csv \
  > gate6a/v3_raw/run1.txt
```

The raw output is committed before inspection. A failure promotes this
corpus to the next regression set and a new generation is authored; the
corpus is never retuned.
