# Emitter findings F6–F8 — closure (2026-09-17)

**Status: closed.** The SIR → C loop emitter (`sir_benchmarks::emit`) now
executes natively under `clang` with the same semantics as the original
LLVM IR on every lowered loop-source kernel in the repository: **99
kernel-functions differential-clean, 0 mismatches**, including **19
functions whose committed rewrite output was executed natively**. The
three defects recorded by H3 and repeated as open scope limits in
`GATE4_RESULTS.md`, `GATE4B_PROOF.md` and `H4_RESULTS.md` are fixed.

## The findings and root causes

| Finding | Recorded symptom | Root cause | Fix |
|---|---|---|---|
| **F6** post-loop emission order | Emitted C for guarded loops did not compile: the reconstructed termination node (`Lt(carry, bound)`) was referenced but never defined, and statements were written in arena order | `emit_loop` emitted only the nodes listed in `Loop::body`; the synthesized termination lived outside it. Post-loop statements were emitted in arena order, not dataflow order | Dependency closure + topological emission (`topo_order`, `dependency_closure`); the termination and everything it needs is emitted before the pre-test |
| **F7** buffer element-width typing | `count_ge_u16` emitted `const uint8_t *p0` and indexed it as `u16`; the SIR interpreter also read 8-bit elements | LLVM's opaque `ptr` parameters were hard-typed `*const u8`, discarding the GEP source element type (`i16`, `i32`, …) | `infer_ptr_param_types`: recover the pointee from direct GEP/load/store uses; conflicting views fall back to the byte view; stores mark the parameter mutable |
| **F8** loop-control polarity / entry-guard | The emitter wrote post-tested `while (1) { body; if (!term) break; }` while SIR loop semantics are pre-tested, and the lowerer left successor tests (`Lt(carry+step, bound)`) in place | Emitter: wrong loop form (body ran once for `n == 0`, termination evaluated on updated carries). Lowerer: `continue_on_true` rotations were passed through unnormalized; dynamic/stride steps were not rebuilt onto the carry domain | Emitter emits the pre-test before the body and initializes outputs to the entry carries; lowerer rebuilds `Lt/Le(carry, bound)` for guarded successor-tested loops and refuses unguarded do-while successors loudly |

Two further bugs were found by the new native harness while fixing F8 and
are also closed:

- **zero-trip output initialization**: with a pre-tested loop, `o_i` was
  never assigned when the loop did not run (returned stack garbage for
  `n == 0`); outputs now start as the entry carries.
- **dynamic-stride domain**: `for (i = 0; i < n; i += step)` checked
  `i + step < n`, dropping the final iteration; the successor test is now
  rebuilt onto `i < n` (and only when the condition really compares the
  carry's own successor).

## Native differential harness

`sir/crates/sir_benchmarks/src/bin/emit_c_diff.rs`: for every function in
a `.ll` corpus it lowers, optimizes (rewrites included), emits C,
compiles the emitted C and a renamed copy of the original IR with
`clang -O1`, runs both on identical random inputs (buffer bytes random;
scalars small, including length 0), and compares the return value plus
every pointer buffer byte-for-byte.

Raw runs: `emitter_native/` (full per-function output and refusal
reasons). Totals: **99 clean / 0 mismatched / 49 lower-refused**, with
**19 rewritten functions** natively verified (Any-recipe
`ArrayCmpMask` masks on `h3/h4/h4b/h4c/d4` tier B). The same run also
required bitvector emission support (H3's "Pack/ArrayCmpMask emission is
unexercised"): `Type::BitVector` now has a runtime representation plus
`ArrayCmpMask`, `Pack`, equality, popcount and bit-scan helpers.

## Regression tests

- `sir/crates/sir_benchmarks/tests/emit_c_native.rs` (4 tests): u16
  buffer typing + `n == 0` entry guard, stride-2 pre-test, dynamic stride
  final element, and the rewritten `any` mask executed natively (asserts
  the rewrite fired).
- `sir/crates/sir_lower/tests/lowering_regressions.rs` (5 new tests):
  pointer pointee inference for `*u16`, mixed-view byte fallback, mutable
  store parameter, successor-tested stride loop rebuilt onto the carry
  domain, and loud refusal of unguarded successor do-whiles.

Full workspace: **603 passed / 0 failed**.

## Scope honesty

- This is native execution *of the emitted C*, compiled by `clang -O1`,
  compared input-by-input against the original IR compiled by the same
  compiler. It is not a verified emitter: the differential is random
  input testing, not a proof, and the scalar/buffer domain is bounded
  (buffers ≤ 2 KiB, scalars small).
- 49 kernels refuse at lowering by design (signed comparisons, atomics,
  volatile stores, multi-loop CFGs, separate latch blocks, unsupported
  store/global forms); refusal counts are part of the output, and a
  refusal is never counted as a pass.
- Gate 4B's machine-checked evidence and Gate 6B's fusion measurements
  are unchanged; this closes the *emitter boundary* those documents
  recorded as open, it does not upgrade their assurance class.

## Regressions checked

- `gate6a_run` on the sealed v3 and v5 corpora: `GATE6A_VERDICT PASS`
  (v4 remains FAIL, its recorded corpus-classification defect).
- Full workspace suite: 603/0.

## Sanitizer-instrumented differential (2026-09-17, follow-up)

The native bridge now has a stricter mode: `emit_c_diff --sanitize`
compiles the reference/emitted pair with
`-fsanitize=address,undefined -fno-sanitize-recover=all` and reports any
diagnostic as a violation. On the full corpus set — v2–v11 plus
h3/h4/h4b/h4c/d4, 48 cases each — the result is
**223 clean / 0 mismatched / 76 lower-refused / 0 sanitizer
violations** (`emitter_native/sanitize/*.txt`). The emitted C,
including every applied rewrite, is free of ASan/UBSan diagnostics
under this mode.

The same instrumentation was extended to the Gate 6B fusion harness
(`gate6b_run --sanitize`, 10/10, no diagnostics) and to the native
fixture suite (`emit_c_native.rs` compiles every test driver with
ASan+UBSan: 20/20 pass, no diagnostics), so the builder-fixture paths
not exercised by the corpora are sanitizer-clean too.
