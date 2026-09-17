# Native emitter differential evidence — H3 findings F6–F8

Raw output of `emit_c_diff` (bin in `sir/crates/sir_benchmarks`) on every
loop-source corpus in the repository, after the F6–F8 emitter/lowerer
fixes of 2026-09-17. Each row is one function:

```
NATIVE <name>: clean cases=N rewrites=R
NATIVE <name>: MISMATCH ...
NATIVE <name>: lower_refused (<fail-closed reason>)
EMITC_SUMMARY clean=... mismatched=... lower_refused=... harness_errors=...
```

Method: lower the function to SIR, run the default optimizer (rewrites
included), emit C, compile the emitted C together with a renamed copy of
the original LLVM IR using `clang -O1`, run both on identical random
inputs (random buffer bytes; scalar parameters drawn from small values
including length 0), and compare the return value and every pointer
buffer byte-for-byte.

| Corpus | Clean | Mismatched | Lower-refused | Cases each | Rewrites applied |
|---|---:|---:|---:|---:|---:|
| `gate6a/v2_corpus.ll` | 8 | 0 | 8 | 48 | 1 |
| `gate6a/v3_corpus.ll` | 16 | 0 | 8 | 48 | 0 |
| `gate6a/v4_corpus.ll` | 21 | 0 | 3 | 48 | 0 |
| `gate6a/v5_corpus.ll` | 20 | 0 | 4 | 48 | 1 |
| `h3/tier_b.ll` | 7 | 0 | 5 | 24 | 4 |
| `h4/tier_b.ll` | 7 | 0 | 6 | 24 | 4 |
| `h4b/tier_b.ll` | 7 | 0 | 6 | 24 | 4 |
| `h4c/tier_b.ll` | 6 | 0 | 4 | 24 | 3 |
| `d4/tier_b.ll` | 7 | 0 | 5 | 24 | 4 |
| **Total** | **99** | **0** | **49** | — | **22** |

Re-measured 2026-09-17 after constant-extent buffer promotion: v2
`w06_count_mismatch_const`, v3 `p04_count_ge_const_u32` and v5
`p07_count_eq_const_bound` each apply 1 native-clean rewrite (24/24
cases), so the total rises 19 → 22 with 0 mismatches. The multi-loop
kernels (v2 `w08`, v3/v4/v5 `p12`) are lower-refused again: a two-loop
SIR composition was prototyped, but this harness caught wrong emitted C
(21–24/24 mismatching cases, 0 rewrites) because `emit.rs` models a
single loop. `emit_c` now panics loudly on >1 Loop node and the lowerer
keeps its explicit multi-loop refusal until the emitter composes
sequential loops.

Lower refusals are the documented fail-closed classes (signed icmp,
atomic/volatile ordering, multi-loop CFGs, separate latch blocks,
unsupported store forms); they are not silent miscompilations.

Reproduce:

```bash
cd sir
cargo run -q -p sir_benchmarks --bin emit_c_diff -- ../gate6a/v5_corpus.ll --cases 48
cargo run -q -p sir_benchmarks --bin emit_c_diff -- ../h4c/tier_b.ll --cases 24
```
