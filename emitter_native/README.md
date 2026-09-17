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
| `gate6a/v6_corpus.ll` | 17 | 0 | 5 | 48 | 6 |
| `gate6a/v7_corpus.ll` | 19 | 0 | 5 | 48 | 5 |
| `gate6a/v8_corpus.ll` | 23 | 0 | 5 | 48 | 8 |
| `gate6a/v9_corpus.ll` | 25 | 0 | 4 | 48 | 13 |
| **Total** | **187** | **0** | **64** | — | **54** |

**Runtime-extent search lowering** (2026-09-17): clang's guarded form
(`n == 0` entry guard, header/latch search, `llvm.umin(phi, n)` clamp
before the return) now lowers via the found-flag synthesis — the merge's
post-processing is emitted and the extra zero-trip predecessor is
validated — with the pointer preserved (no fabricated extent). v8 `p14`
and v9 `p16` move from lower-refused to native-clean with 0 rewrites.

**v9** (2026-09-17): the predicate extraction generalizes blind —
Eq/Ne/Lt/Ge/Gt, u8/u16/u32, zero/literal/parameter scalars, swapped
operands, extents 48–128 — 13 native-clean rewrites; compound
predicates and sentinel-mismatched sub-range searches recognize but
apply 0 rewrites. 0 mismatches.

**Scalar-predicate searches** (2026-09-17): the position pack helper now
extracts the hit comparison (`Eq`/`Ne`/ordered, negation-normalized)
from the loop's position select instead of assuming an implicit
non-zero predicate, so v8 `p04_first_eq_key_u8_64` rewrites to
`mask_cmp(Eq, key)` + `ctz` and is native-clean; compound predicates
are still refused.

**v8** (2026-09-17): early-exit searches at extents 48/96/128 (u8) and
80 (u16) rewrite natively (≤64 via the concrete solver, >64 via the
symbolic identity); sub-range (sentinel ≠ extent) and zero-sentinel
searches are recognized but never rewritten; a scalar-eq search is not
masked as non-zero. 0 mismatches; 7 native-clean rewrites.
See docs/GATE6A_V8_RESULTS.md.

**>64-element scan rewrites** (2026-09-17): v6 `p09` (96) and v7 `p09`
(80) now rewrite to `ctz(pack)` and are native-clean 48/48 each. The
64-bit concrete solver cannot bit-blast those extents, so the obligation
is discharged by the symbolic scan identity and honestly issued
**SchemaChecked** (not solver-checked); a sentinel guard requires the
no-hit result to equal the extent.

Re-measured after **early-exit search lowering** (2026-09-17): v6/v7
`p09` header/latch/merge search CFGs lower via found-flag synthesis and
are native-clean (extents 96/80), so each moves from lower-refused to
clean. Their extents exceed the 64-bit solver cap, so they stay
unrewritten; a 48-element early-exit search rewrites to `ctz(pack)` and
runs natively (5/0/47/48) in the `emit_c_native` test. Landing this
also fixed emitter gaps for `BoolAnd`/`BoolOr`/`BoolNot` (previously
silent `0`).

**v7** (2026-09-17) is the first fresh generation after the v6
remediation: post-tested do-whiles at strides 2 and 8 are natively
faithful, a runtime step refuses loudly, a three-loop kernel composes
and rewrites natively, and mixed constant extents promote to the max
view — 18 clean / 0 mismatched / 6 lower-refused, 4 native-clean
rewrites. See docs/GATE6A_V7_RESULTS.md.

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
