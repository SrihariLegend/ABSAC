# Gate 6A-v19 — extended randomized differential corpora

**Status:** generated corpora frozen with `v19_manifest.sha256`; raw runs in
`v19_raw/`. Same generator as v17
(`sir/crates/sir_benchmarks/src/bin/gen_random_corpus.rs`), extended shape set.

## New shapes over v17

- scalar straight-line trees over i32 **and i64**;
- constant-extent reductions with randomized comparison predicates
  (`eq/ne/ult/ule/ugt/uge`) and random constants;
- **runtime-extent reductions with clang's explicit `n == 0` guard and a tail
  phi** (sum and predicate-count variants). This stresses the tail-select
  lowering added for v16 `r01` across many element widths, extents, and
  predicates instead of one hand-written fixture.

## Generation

```bash
gen_random_corpus --seed 4242  --count 160 --output gate6a/v19_fuzz_corpus.ll
gen_random_corpus --seed 31337 --count 200 --output gate6a/v19b_fuzz_corpus.ll
```

Regenerating both seeds reproduces the frozen SHA-256 hashes exactly.

## Results (2026-09-18)

Seed `4242`, 160 functions:

- `emit_c_diff --cases 24`: **160 clean / 0 mismatched / 0 lower-refused**,
  15 rewrites;
- `--sanitize`: 160 clean / 0 mismatched / 0 lower-refused, 0 violations.

Seed `31337`, 200 functions:

- `emit_c_diff --cases 24`: **200 clean / 0 mismatched / 0 lower-refused**,
  17 rewrites;
- `--sanitize`: 200 clean / 0 mismatched / 0 lower-refused, 0 violations.

Combined: 360 randomized programs, 32 rewrites, 0 differential mismatches,
0 refusals, 0 sanitizer violations.

## Scope

Same-author generator and machine, restricted to CFG templates already known to
lower; correctness fuzzing, not recognition coverage. The generator source hash
in the v17 manifest is historical (the generator gained the v19 shapes after
that corpus was frozen); the v17 corpus hashes themselves are unchanged.
