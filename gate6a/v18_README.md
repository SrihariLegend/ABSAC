# Gate 6A-v18 — scalar bit-count intrinsics

**Status:** fresh probe with manifest `v18_manifest.sha256` and raw runs in
`v18_raw/`.

## Frontier

Clang lowers scalar `__builtin_popcount/ctz/clz` to `llvm.ctpop/cttz/ctlz`.
Before this change the lowerer refused every such call; now the well-defined
(`is_zero_undef = false`) forms lower to the SIR `Popcount`,
`TrailingZeros`, and `LeadingZeros` nodes, with the i32 builder result widened
to the IR width. The poison form (`is_zero_undef = true`) still fails closed.

These functions are already in target form: clang emits the same hardware
intrinsic the recipes would select, so the expected rewrite count is zero. The
value is input coverage and differential correctness, not speed.

## Evaluation

```bash
emit_c_diff gate6a/v18_corpus.ll --cases 48
emit_c_diff gate6a/v18_corpus.ll --cases 48 --sanitize
```

## Results (2026-09-18)

- `emit_c_diff`: 6 clean / 0 mismatched / 0 lower-refused, 0 rewrites
  (popcount u32/u64, cttz u32/u64, ctlz u32/u64).
- `--sanitize` (ASan+UBSan): 6 clean / 0 mismatched / 0 lower-refused,
  0 violations.
- Lowerer regressions: `bit_count_intrinsics_lower_to_sir_operations`
  (including the `range(...)` attribute and i64 widening) and
  `poison_bit_count_form_is_refused`.

## Scope

Lowering coverage only. No concept is derived for a bare intrinsic node and no
rewrite is expected, because the source already expresses the target operation;
using these nodes to enable composed patterns (for example popcount of a masked
value) would require new recognizers and is not claimed here.
