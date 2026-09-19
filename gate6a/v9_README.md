# Gate 6A-v9 — fresh held-out corpus (Generation H9)

**Status:** frozen before evaluation (classification in
`v9_expected.csv`, hashes in `v9_manifest.sha256`). Raw output goes to
`v9_raw/` and `emitter_native/diff_gate6a_v9.txt` and is committed
before inspection. Failures promote the corpus to the next regression
set; it is never reused as held-out proof.

## Frontier

Created 2026-09-17 after the position-predicate extraction (the search
recipe derives the hit comparison — Eq/Ne/ordered, negation- and
swap-normalized — from the loop's position select):

1. Predicate coverage across operators (Eq/Ne/Lt/Le/Gt), element widths
   (u8/u16/u32), extents in both the solver (≤64) and symbolic (>64)
   ranges, parameter vs literal scalars, and swapped operands.
2. Refusal discipline: a compound predicate and a sentinel-mismatched
   sub-range search must be recognized but never masked/rewritten.
3. Stable multi-loop and cardinality positives, plus the recorded
   safety classes and contained rows.

## Evaluation

```bash
gate6a_run --corpus gate6a/v9_corpus.ll --expectations gate6a/v9_expected.csv
emit_c_diff gate6a/v9_corpus.ll --cases 48
```

The first scores recognition/safety; the second compiles the original
LLVM IR and the emitted C and compares them natively (return value and
every buffer byte) on 48 random inputs.
