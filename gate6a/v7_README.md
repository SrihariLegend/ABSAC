# Gate 6A-v7 — fresh held-out corpus (Generation H7)

**Status:** frozen before evaluation (classification in
`v7_expected.csv`, hashes in `v7_manifest.sha256`). Raw output goes to
`v7_raw/` and `emitter_native/diff_gate6a_v7.txt` and is committed
before inspection. Failures promote the corpus to the next regression
set; it is never reused as held-out proof.

## Frontier

Created 2026-09-17 after the v6 remediation:

1. The P0A candidate gate requires at least one certificate for a
   region (v6 found 3 candidate-only safety failures).
2. Unguarded post-tested do-while carry tests are reconstructed
   (`carry < K + step`, checked) or refused loudly (v6 found a native
   miscompile at stride 4).
3. The sequential composer skips blocks consumed by earlier loops, so
   constant-extent promotion and whole-function candidates work for
   multi-loop kernels.

v7 probes unseen variants: strides 2 and 8 in the post-tested form, a
**runtime step** that must refuse, three sequential loops, two loops
with **different constant extents** (the promoted view must cover the
maximum), plus the recorded safety classes and known gaps.

## Pre-registered classification

**Positives (11):** five confident constant-extent reductions (sums,
cardinality `<=`/`!=`, all-`>=`, raw-element OR), a three-loop kernel,
a mixed-extent two-loop kernel, and four pre-registered known-gap
probes (do-while stride 2 and 8 for lowering/native fidelity,
early-return first-set search, map-then-sum).

**Contained rows (4):** runtime-extent sum/cardinality, signed
accumulation, and a post-tested runtime-step loop. Recognition is
allowed; no candidate and no rewrite may be produced.

**Negatives (9):** D5 masked sums (ternary and high-nibble), X06
running max, volatile load, atomic RMW, may-alias store, two-array
relation, the unsigned reverse search, and early exit with a global
write.

## Evaluation

```bash
gate6a_run --corpus gate6a/v7_corpus.ll --expectations gate6a/v7_expected.csv
emit_c_diff gate6a/v7_corpus.ll --cases 48
```

The first scores recognition/safety; the second compiles the original
LLVM IR and the emitted C and compares them natively (return value and
every buffer byte) on 48 random inputs.
