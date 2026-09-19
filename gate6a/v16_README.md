# Gate 6A-v16 — runtime-extent, wider-reduction, and permutation corpus

**Status:** frozen before evaluation (classification in `v16_expected.csv`,
hashes in `v16_manifest.sha256`). Raw output goes to `v16_raw/`.

## Frontier

Created 2026-09-18 after the measured-weight and P2 investigation increments.
v12–v14 cover merged arithmetic/mask recovery and search role binding but have
no coverage of:

- runtime-extent searches with explicit zero-length guards, alternate
  predicates (`ne`, `ge`), u16/u32 elements, and descending order;
- wider-element sums and xor map-then-sum reductions (recognized but
  unrewritten per `sir_optimizer/tests/unrewritten_reductions.rs`);
- scalar bit permutations (rotate, byte swap, bit reverse), which had no corpus
  coverage at all.

Uncertain lowering/recognition shapes are pre-registered as
`recall_gap_known`; the corpus does not claim recognition for them.

## Evaluation

```bash
gate6a_run --corpus gate6a/v16_corpus.ll --expectations gate6a/v16_expected.csv
emit_c_diff gate6a/v16_corpus.ll --cases 48
emit_c_diff gate6a/v16_corpus.ll --cases 48 --sanitize
```

## Results (2026-09-18)

### First frozen run (pre-fix)

- `gate6a_run`: positives 12, recognized 4, recall misses 0, known-gap misses 8;
  `GATE6A_VERDICT PASS`.
- `emit_c_diff`: 6 clean / **1 mismatched** / 5 lower-refused.
  `r03_runtime_last_eq_key_u16` mismatched in 38/48 cases with **zero
  rewrites**: the descending runtime search's synthesized loop aliased the
  no-hit sentinel with the mutable counter carrier, so the emitted C returned
  the last scanned index instead of `n`. Raw output:
  `v16_raw/emit_c_diff_pre_fix.txt`.

### Fix

`sir_lower` now freezes the descending index seed into a distinct pre-loop
snapshot (`sentinel + 0`) before building the select and carried inputs.
Regression: `descending_search_lowers_with_a_sub_successor` asserts the index
seed differs from the counter seed and that the select fallback is not the
counter; it was red-verified by restoring the aliased lowering.

### Post-fix

- `emit_c_diff`: 7 clean / 0 mismatched / 5 lower-refused / 0 rewrites.
- `emit_c_diff --sanitize` (ASan+UBSan, `detect_leaks=0`): 7 clean /
  0 mismatched / 5 lower-refused / 0 sanitizer violations.
- `--time-baseline` on the 7 lowered functions: 0.99–1.02x (no rewrites
  applied), i.e. no unintended codegen delta.

### Disclosed unsuccessful cases

- Lower-refused (5): `r01_runtime_first_zero_guard` (merge does not return),
  `p01_rotl13_u32`, `p02_bswap_u32`, `p03_bitrev_u32` (random-access
  permutation shapes not modeled), `c01_search_then_sum` (unresolved GEP index
  in the composed form).
- Recognized-but-unrewritten: `r02`/`r04` runtime searches (runtime extent has
  no authorized replacement), `s01`/`s02`/`s04` sums and map-then-sum
  (P2 investigation: no reduction strategy/theorem/recipe), `s03` conditional
  sum (known gap).
- No positive rewrite occurred on this corpus; it is negative/refusal and
  correctness evidence, not rewrite-rate evidence.

### Enabler follow-up (post-freeze)

After the first run, three lowering gaps that the frozen CSV conservatively
marked `recall_gap_known` were closed:

- single-block straight-line functions are now lowered (previously refused by
  the "too few blocks" gate);
- `llvm.fshl`/`llvm.fshr` lower to the canonical `Or(Shl(x,k),
  Shr(x,Sub(width,k)))` shift pair, so `p01_rotl13_u32` recognizes
  `CircularPermutation`/`ShiftPairLeft` and rewrites to `RotateLeft` (def 310);
- `llvm.bswap` / `llvm.bitreverse` lower to their canonical masked-swap stage
  chains, so `p02_bswap_u32` rewrites to `ByteSwap` (def 312) and
  `p03_bitrev_u32` rewrites to `ReverseBits` (def 313).
- the runtime zero-length guard (`r01`) now lowers: the merge's `umin` clamp
  plus the tail phi are modeled as a select between the entry-guard value and
  the clamped result, so the function identifies as `FirstOccurrence` with
  PositionSearch roles assigned (0 rewrites — runtime extent still has no
  authorized replacement). Regression:
  `guarded_runtime_search_lowers_with_a_tail_select`.

Post-enabler results:

- `emit_c_diff`: **11 clean / 0 mismatched / 1 lower-refused / 3 rewrites**
  (rotate, byte swap, bit reverse). The remaining refusal is
  `c01_search_then_sum` (composition GEP).
- `--sanitize`: 11 clean / 0 mismatched / 1 lower-refused / 0 violations.
- `gate6a_run` still reports 4 recognized / 8 known-gap misses because the
  frozen CSV intentionally classified the three permutation rows as gaps;
  recognition was achieved after the freeze and is recorded here rather than
  rewriting the pre-registered classification.
- Lowerer regressions: `single_block_funnel_rotate_lowers_to_canonical_shift_pair`,
  `single_block_bswap_lowers_to_two_masked_swap_stages`, and
  `single_block_bitreverse_lowers_to_five_stages`.

The 18 prior corpora are unchanged in rewrite counts (243 → 244 clean, 72 → 71
lower-refused, 98 rewrites, zero per-function rewrite differences); the only
status change is `n09_self_recurrence` (`ret i64 1`), which now lowers as a
single straight-line block instead of being refused.

### Measured profitability of the new permutations

`--time-baseline` (unoptimized emitted C vs optimized emitted C, 200k reps):

- `p01_rotl13_u32`: 1.00x (clang already folds the shift pair);
- `p02_bswap_u32`: 1.03x;
- `p03_bitrev_u32`: initially **0.21x** — the ReverseBits recipe selected
  `__sir_rbit`, which was a 32-iteration per-bit loop. The emitted helper is now
  a branchless SWAR chain (`0x5555…`/`0x3333…` masks), restoring p03 to 1.00x.
  The native rbit regression now asserts the SWAR masks are present and the
  per-bit loop is absent.

`--measured-retain` rejects all three permutation rewrites at the 5% margin
(1.00x, ~1.03x, 1.00x), and the retained composition stays baseline. The
default optimizer still applies them; enabling a measured/target profile is
what suppresses neutral rewrites. This is disclosed rather than counted as a
performance win.
