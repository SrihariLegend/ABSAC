# Gate 6A-v11 — fresh held-out corpus (Generation H11)

**Status:** frozen before evaluation (classification in
`v11_expected.csv`, hashes in `v11_manifest.sha256`). Raw output goes
to `v11_raw/` and `emitter_native/diff_gate6a_v11.txt` and is committed
before inspection. Failures promote the corpus to the next regression
set; it is never reused as held-out proof.

## Frontier

Created 2026-09-17 after the early-exit synthesis gained merge-form
matching, the guard-on-trip-bound rule, and latch selection that
ignores the entry guard. v11 blind-tests:

- runtime searches with unseen header polarities and predicates
  (`!buf[i]`, `== key`, `!= key`), u16 elements, and `-1`/`n` sentinels;
- recorded shape limits (computed `n-1` sentinel, do-while, descending);
- constant searches for zero and ordered predicates (rewrites), stable
  cardinality/multi-loop/sum positives, and the safety classes.

## Evaluation

```bash
gate6a_run --corpus gate6a/v11_corpus.ll --expectations gate6a/v11_expected.csv
emit_c_diff gate6a/v11_corpus.ll --cases 48
```

The first scores recognition/safety; the second compiles the original
LLVM IR and the emitted C and compares them natively (return value and
every buffer byte) on 48 random inputs.
