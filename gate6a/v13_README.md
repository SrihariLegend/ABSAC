# Gate 6A-v13 — occurrence recovery + scan-soundness regression corpus

**Status:** frozen before evaluation (classification in
`v13_expected.csv`, hashes in `v13_manifest.sha256`). Raw output goes to
`v13_raw/` and `emitter_native/diff_gate6a_v13.txt`.

## Frontier

Created 2026-09-18 after the v12 corpus exposed a real miscompile: the
v0.1 zero-count scan heuristic classified any loop containing `Shr` and
`And` as a scan, so valid arithmetic rewrites could turn an ordinary
accumulation loop into a `clz` replacement. The heuristic now requires
the canonical carried-shift/probe/termination shape.

v13 exercises two things:

- **Positives:** merged occurrence recovery on independently written
  loop variants (four mask patterns, merged remainder/division/
  shift-mask, chained power-of-two operations, u64 and u16 widths).
- **Negatives:** ordinary accumulation loops that contain shift and
  mask operations but are not zero-count scans. These must produce zero
  candidates and zero rewrites.

Unlike v10/v11, this corpus was authored *after* the fix it pins. It is
a regression/fresh-input corpus, not a blind discovery set; v12 remains
the corpus that found the defect.

## Evaluation

```bash
gate6a_run --corpus gate6a/v13_corpus.ll --expectations gate6a/v13_expected.csv
emit_c_diff gate6a/v13_corpus.ll --cases 48
```

## Results (2026-09-18)

The first evaluation scored 3/5 positives: the two merged-mask rows
missed `LowestSetBit` and `LowestClearBitMask`. LLVM lowers unary minus
to `Sub(0, x)` and bitwise not to `Xor(x, all-ones)`, while the
recognizers and mask binding only accepted the `Neg` / `Not` SIR forms.
The shared `mask_occurrence` helpers now accept both forms for
recognition, proof binding, and application binding; this raises
`a01`/`a04` from 2 to 4 native rewrites and leaves the three negatives
at zero candidates/rewrites.

After the fix the frozen corpus passes as pre-registered:

- `gate6a_run`: positives 5/5 recognized, recall misses 0; negatives
  3/3 safe; `GATE6A_VERDICT PASS`.
- `emit_c_diff`: 8 clean / 0 mismatched / 0 lower-refused, 16 native
  rewrites (`a01` 4, `a02` 3, `a03` 3, `a04` 4, `a05` 2; negatives 0).

Raw output is in `v13_raw/` and `emitter_native/diff_gate6a_v13.txt`.
