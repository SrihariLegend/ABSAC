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
| `gate6a/v2_corpus.ll` | 8 | 0 | 8 | 48 | 0 |
| `gate6a/v3_corpus.ll` | 16 | 0 | 8 | 48 | 0 |
| `gate6a/v4_corpus.ll` | 21 | 0 | 3 | 48 | 0 |
| `gate6a/v5_corpus.ll` | 20 | 0 | 4 | 48 | 0 |
| `h3/tier_b.ll` | 7 | 0 | 5 | 24 | 4 |
| `h4/tier_b.ll` | 7 | 0 | 6 | 24 | 4 |
| `h4b/tier_b.ll` | 7 | 0 | 6 | 24 | 4 |
| `h4c/tier_b.ll` | 6 | 0 | 4 | 24 | 3 |
| `d4/tier_b.ll` | 7 | 0 | 5 | 24 | 4 |
| **Total** | **99** | **0** | **49** | — | **19** |

Lower refusals are the documented fail-closed classes (signed icmp,
atomic/volatile ordering, multi-loop CFGs, separate latch blocks,
unsupported store forms); they are not silent miscompilations.

Reproduce:

```bash
cd sir
cargo run -q -p sir_benchmarks --bin emit_c_diff -- ../gate6a/v5_corpus.ll --cases 48
cargo run -q -p sir_benchmarks --bin emit_c_diff -- ../h4c/tier_b.ll --cases 24
```
