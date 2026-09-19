# Gate 6A-v17 — deterministic randomized differential corpora

**Status:** generated corpora frozen with `v17_manifest.sha256`; raw runs in
`v17_raw/`. The generator is committed at
`sir/crates/sir_benchmarks/src/bin/gen_random_corpus.rs`.

## Front

The v12–v16 corpora are hand-written. v17 fuzzes the same end-to-end path
(lower → semantics/optimizer → emit C → clang differential) over randomized
operator, constant, element-width, and extent combinations:

- scalar straight-line i32 expression trees (add/sub/mul/and/or/xor/shl/lshr);
- constant-extent sums (u8/u16/u32 → u64);
- conditional sums;
- count / any / parity reductions over byte arrays.

Shapes are deliberately restricted to CFG templates already known to lower, so
a clean run is correctness evidence, not recognition-coverage evidence. The
generator, not the compiler, decides the shapes; no claim of blind frontier
discovery.

## Generation

```bash
gen_random_corpus --seed 20260918 --count 60  --output gate6a/v17_fuzz_corpus.ll
gen_random_corpus --seed 777      --count 120 --output gate6a/v17b_fuzz_corpus.ll
```

## Results (2026-09-18)

Seed `20260918`, 60 functions:

- `emit_c_diff --cases 24`: **60 clean / 0 mismatched / 0 lower-refused**,
  11 rewrites applied;
- `--sanitize`: 60 clean / 0 mismatched / 0 lower-refused, **0 violations**.

Seed `777`, 120 functions:

- `emit_c_diff --cases 24`: **120 clean / 0 mismatched / 0 lower-refused**,
  30 rewrites applied;
- `--sanitize`: 120 clean / 0 mismatched / 0 lower-refused, **0 violations**.

Combined: 180 randomized programs, 41 rewrites applied, 0 differential
mismatches, 0 sanitizer violations, 0 harness errors.

## Scope

Same-author generator and same machine; shapes are restricted to known-lowering
templates, and all differential comparisons use the same clang `-O1` baseline
as the other corpora. This strengthens correctness evidence beyond hand-written
corpora but does not replace external baselines, multiple architectures, or
independently authored programs.
