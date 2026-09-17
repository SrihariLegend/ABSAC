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
| `gate6a/v2_corpus.ll` | 9 | 0 | 7 | 48 | 1 |
| `gate6a/v3_corpus.ll` | 17 | 0 | 7 | 48 | 1 |
| `gate6a/v4_corpus.ll` | 22 | 0 | 2 | 48 | 0 |
| `gate6a/v5_corpus.ll` | 21 | 0 | 3 | 48 | 1 |
| `h3/tier_b.ll` | 7 | 0 | 5 | 24 | 4 |
| `h4/tier_b.ll` | 7 | 0 | 6 | 24 | 4 |
| `h4b/tier_b.ll` | 7 | 0 | 6 | 24 | 4 |
| `h4c/tier_b.ll` | 6 | 0 | 4 | 24 | 3 |
| `d4/tier_b.ll` | 7 | 0 | 5 | 24 | 4 |
| `gate6a/v6_corpus.ll` | 16 | 0 | 6 | 48 | 5 |
| **Total** | **119** | **0** | **51** | — | **27** |

The **v6 generation** was created 2026-09-17 after the F6–F8 emitter
fixes and the multi-loop/constant-extent work, and evaluated one-shot:
this harness caught a real lowering bug (`n06_stride4_const`, a
post-tested do-while lowered as pre-tested: 48/48 mismatches; raw
failure preserved in `gate6a/v6_raw/run1_native_failure.txt`). After the
fix the v6 run is 16 clean / 0 mismatched / 6 lower-refused with 5
native-clean rewrites (including the whole-function two-loop kernel
`p08_two_loops_const`). See docs/GATE6A_V6_RESULTS.md.

Re-measured 2026-09-17 (final): constant-extent buffer promotion adds
native-clean rewrites for v2 `w06_count_mismatch_const`, v3
`p04_count_ge_const_u32` and v5 `p07_count_eq_const_bound` (19 → 22
rewrites), and the SIR→C emitter now composes **sequential loops**
(carrier/output variables are namespaced by the loop ordinal;
TupleExtract/FieldAccess resolve to the producing loop). The four
multi-loop kernels (v2 `w08`, v3/v4/v5 `p12`) moved from lower-refused
to native-clean, so clean rises 99 → 103 and lower-refused falls
49 → 45, still with **0 mismatches**.

History: the first two-loop composition attempt (before the emitter
learned sequential loops) emitted wrong C — this harness caught it
(w08 21/24, p12 21–24/24 mismatches, 0 rewrites) — and the lowering was
reverted fail-closed until the emitter was fixed. The failed attempt is
preserved in the git history and in docs/RECALL_RESULTS.md.

Lower refusals are the documented fail-closed classes (signed icmp,
atomic/volatile ordering, separate latch blocks / early exits,
unsupported store forms, unguarded post-tested loops without a
constant-step reconstruction); they are not silent miscompilations.
Sequential multi-loop CFGs now LOWER and emit (a two-loop composition
was once reverted after this harness caught it; the emitter gained
per-loop namespacing and TupleExtract/FieldAccess resolution before it
was re-landed).

Reproduce:

```bash
cd sir
cargo run -q -p sir_benchmarks --bin emit_c_diff -- ../gate6a/v5_corpus.ll --cases 48
cargo run -q -p sir_benchmarks --bin emit_c_diff -- ../h4c/tier_b.ll --cases 24
```
