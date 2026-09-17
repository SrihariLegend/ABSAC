# Gate 6A-v10 — fresh held-out corpus (Generation H10)

**Status:** frozen before evaluation (classification in
`v10_expected.csv`, hashes in `v10_manifest.sha256`). Raw output goes
to `v10_raw/` and `emitter_native/diff_gate6a_v10.txt` and is committed
before inspection. Failures promote the corpus to the next regression
set; it is never reused as held-out proof.

## Frontier

Created 2026-09-17 after the runtime-search zero-trip rule was
generalized by merge form (identity vs `umin` clamp) and hardened to
require a guard on the trip bound:

1. Runtime searches with sentinel `n` (clamp), `-1` (identity), `n-1`
   (computed), and over `bool` elements.
2. Loop-shape limits: do-while search, descending search, and a search
   whose merge path continues into a second loop.
3. Sentinel discipline on a constant extent (sentinel 99 over 32
   elements must recognize and refuse the rewrite).
4. Stable cardinality/multi-loop/sum positives and the recorded safety
   classes.

## Evaluation

```bash
gate6a_run --corpus gate6a/v10_corpus.ll --expectations gate6a/v10_expected.csv
emit_c_diff gate6a/v10_corpus.ll --cases 48
```

The first scores recognition/safety; the second compiles the original
LLVM IR and the emitted C and compares them natively (return value and
every buffer byte) on 48 random inputs.
