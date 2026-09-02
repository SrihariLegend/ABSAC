# H3 — Fresh held-out evaluation of the frozen C3 (Any vertical slice)

Protocol artifacts for the third held-out corpus. Frozen configuration: commit
`49999a4` (git tag `c3-freeze-any`), registry `any_only_registry()` (DefinitionId 4),
SchemaChecked assurance, immutable authorization database. See
`docs/C3_FREEZE_ANY.md` and `docs/C3_FREEZE_RECORD.md`.

| File | Role |
|------|------|
| `tier_a.tsv` | SIR-source corpus (frozen C3's supported input dialect): typed-array Any loops built as SIR fixtures. 26 rows: 12 eligible positives (P), 12 near-miss negatives (N), 2 safety near-misses (S1). |
| `tier_b.c` / `tier_b.ll` | LLVM-source corpus compiled with the freeze flags (`clang -O1 -emit-llvm -S`). 12 kernels. |
| `expected.csv` | Expected outcomes + expected safety per input, recorded before the sealed run (authoring required stage-level probing of the frozen commit; probes are disclosed in the report). |
| `raw/run1.txt` | Raw one-shot run output (committed before inspection). |
| `raw/exec1.txt` | Differential SIR execution of every committed rewrite (committed before inspection). |
| `manifest.sha256` | Hashes of every sealed artifact. |

## Tier A semantics (per row, by construction)

Columns: `id class kind elem op scalar extent acc_slot identity consumer deviation`.

- `bool` rows: `acc = false; for i in 0..extent { acc |= collection[i] }` over `[bool; extent]`;
  return acc (slot `acc_slot` of the loop tuple).
- `pred` rows: `acc = false; for i in 0..extent { acc |= (collection[i] op scalar) }` over
  `[elem; extent]` with elem ∈ {u8,u16,u32,u64,i8,i32}; scalar is a parameter or literal `const:N`.
- Deviations encode near-miss semantics: `deadidx`, `twofield`, `tuple` (whole-tuple return),
  `nz_start`, `partial_bound`, `reverse`, `unstable_bound`, `two_reductions`,
  `unstable_scalar` (predicate against the induction counter), `identity=1`.

## Tier B kernels

`h3b01..04` are positive-shaped fixed-extent global scans (the only LLVM-sourced shapes that
reach Any candidate generation under the frozen C3); `h3b05..12` are near-miss negatives.
