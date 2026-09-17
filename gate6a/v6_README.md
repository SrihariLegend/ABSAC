# Gate 6A-v6 — fresh held-out corpus (Generation H6)

**Status:** frozen before evaluation (classification in
`v6_expected.csv`, hashes in `v6_manifest.sha256`). Raw output
(`v6_raw/run1.txt`, `emitter_native/diff_gate6a_v6.txt`) is committed
before inspection. Failures promote the corpus to the next regression
set; it is never reused as held-out proof.

## Frontier

Created 2026-09-17 after three capability changes that this corpus is
the first blind test of:

1. **F6–F8 emitter fixes** (native C differential is the execution
   bridge, not the SIR interpreter).
2. **Sequential multi-loop emission + lowering** (whole-function
   two-loop composition; carriers/outputs namespaced per loop).
3. **Constant-extent buffer promotion** (a pointer parameter becomes a
   fixed array view only when every access is inside a proven constant
   loop extent).

## Pre-registered classification

**Positives (10):** constant-extent cardinality (u16 `>=` / u8 `==` /
u32 `<`), sums (u8→u64 bound 128, u16→u32 bound 40), all-`!=`
(conjunctive), raw-element OR (disjunctive), a sequential two-loop
count+sum kernel, plus two pre-registered *known gaps*: early-return
first-set search (no early-exit lowering) and map-then-sum over a
constant extent (recognition expected as `MappedSumReduction`, no
candidate yet).

**Contained rows (3):** runtime-extent sum and cardinality, and signed
nsw accumulation. Recognition is allowed; **no candidate and no
rewrite may be produced** (no fabricated extent / no overflow
authorization).

**Negatives (9):** running max (X06), masked-guard sum (D5) over a
constant extent, stride 4, volatile store, atomic load, may-alias
store, two-array relation, the unsigned `i >= 0` reverse search
(reverse-totality gate), and early exit with a global side effect.

## Evaluation

Two independent runs on the frozen corpus:

```bash
gate6a_run --corpus gate6a/v6_corpus.ll --expectations gate6a/v6_expected.csv
emit_c_diff gate6a/v6_corpus.ll --cases 48
```

The first scores recognition/safety; the second compiles the original
LLVM IR and the emitted C and compares them natively (return value and
every buffer byte) on 48 random inputs.
