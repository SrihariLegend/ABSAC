# Gate 6A-v8 — fresh held-out corpus (Generation H8)

**Status:** frozen before evaluation (classification in
`v8_expected.csv`, hashes in `v8_manifest.sha256`). Raw output goes to
`v8_raw/` and `emitter_native/diff_gate6a_v8.txt` and is committed
before inspection. Failures promote the corpus to the next regression
set; it is never reused as held-out proof.

## Frontier

Created 2026-09-17 after the early-exit search lowering, the scan
binding extension to 512 elements (with the >64 symbolic identity
path), and the sentinel guard:

1. Early-exit searches across extents 48/64/80/96/128, element types
   u8/u16, and multi-loop contexts.
2. Sentinel discipline: sub-range search (128 sentinel over 48
   elements) and zero sentinel must be recognized but never rewritten.
3. Predicate-collection recipe discipline: an implicit non-zero search
   rewrites, a scalar-eq search must not.
4. The recorded safety classes and contained rows (runtime extents,
   signed, volatile, atomic, stores, two arrays, reverse search,
   early-exit global write).

## Evaluation

```bash
gate6a_run --corpus gate6a/v8_corpus.ll --expectations gate6a/v8_expected.csv
emit_c_diff gate6a/v8_corpus.ll --cases 48
```

The first scores recognition/safety; the second compiles the original
LLVM IR and the emitted C and compares them natively (return value and
every buffer byte) on 48 random inputs.
