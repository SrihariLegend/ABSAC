# ABSAC

**Automatic Bitwise Superoptimization of Arbitrary Code**

> Can a compiler recover the mathematical object implemented by arbitrary code and
> choose the best representation for that object?

ABSAC is a semantic compiler. It lowers LLVM IR into a typed functional IR,
recognises mathematical structure embedded in loops, discharges an obligation
proving an equivalent cheaper implementation, and emits C.

It is not pattern matching. A rewrite is only applied when the proof kernel
returns a verdict, and a refuted claim comes back with a counterexample.

## The result

A hand-written equality search is compiled by `clang -O3` to a **scalar
byte-at-a-time loop with two branches per byte**. Verified, not assumed:
`clang -O3 -S` on it emits **0 `memchr` calls and 0 vector instructions**. An
early-exit search returning an index has a data-dependent trip count, so it does
not vectorize, and LLVM has no `memchr` idiom recognition for it. The library
call is available and clang will not use it.

ABSAC recognises the loop and rewrites it (family `search`, definition 403) to a
16-byte inline head scan followed by the vectorized library call:

| scan length | `clang -O3` loop | ABSAC rewrite | |
|---:|---:|---:|---:|
| 1 KiB | 145 ns | 18 ns | **8.06×** |
| 64 KiB | 8,428 ns | 329 ns | **25.62×** |
| 1 MiB | 134,862 ns | 6,952 ns | **19.40×** |
| 16 MiB | 3,013,860 ns | 273,428 ns | **11.02×** |
| 64 MiB | 16,330,263 ns | 2,529,531 ns | **6.46×** |

Same compiler, same `-O3`, both `noinline`, needle absent. Correctness is
machine-checked on two paths — the equivalence obligation plus a differential,
and separately 90 configurations across 13 sizes and 9 needle positions up to
4 MiB, since the differential alone only reaches inputs of 32 bytes.

Full measurement, the emitted kernel, and the limits of the claim:
[`work/search-case/`](work/search-case/USE_CASE.md).

## What is demonstrated, and what is not

The project's rule is that [`docs/CAPABILITY_LEDGER.md`](docs/CAPABILITY_LEDGER.md)
is the record. Quote it, not the design documents, when stating what exists.

**Demonstrated.**

- The pipeline runs end to end on unmodified LLVM IR: lowering, analysis,
  recognition, proof, guarded patch, emission, and a differential referee that
  compares the emitted C against the original IR natively, on AArch64, and under
  sanitizers.
- The proof kernel is real. `sir_mech` decides bitvector equivalence by CDCL SAT
  and refutes with a model; induction and integer linear arithmetic are there for
  the obligations that need them.
- **One family produces a large, correct win over `clang -O3` on a shape clang
  handles badly** — `search`, above. It is the first such result the project has.

**Not demonstrated.**

- **No end-to-end win on third-party code.** The `search` result was verified on a
  hand-written kernel of the shape the family accepts, not discovered by lowering
  a project nobody here wrote. That is the test that matters and it has not been
  passed.
- On 19 independently-authored benchmarks, the only family that fires at any
  optimization level is `arithmetic` (multiply→shift) — a peephole clang already
  performs, so firing there means nothing.
- The one third-party kernel measured historically (Redis `redisPopcount`) lost to
  `clang -O3`, and Redis has since shipped its own hand-written replacement.
- The correctness of the `search` win was established on inputs of 32 bytes or
  fewer by the referee's own harness, and had to be re-established by hand for the
  larger inputs the win depends on. See
  [the 32-byte ruler](work/search-case/USE_CASE.md) — the harness measured its own
  size guard, not the transformation.

`docs/ROADMAP.md`, `docs/NORTH_STAR.md` and the older `docs/GATE*` files describe
intended or historical systems, and some of their claims were never demonstrated
beyond project-authored kernels.

## Pipeline

```
  .ll text ──► sir_lower::lower_function ──► SIR Function
                                                │
                                                ▼  sir_analysis
                                             Facts (loop / reduction)
                                                │
                                                ▼  sir_opt
                                             a family recognises a shape and
                                             discharges its sir_mech obligation
                                                │
                                                ▼  sir_opt::patch::apply
                                             guarded replacement (structural
                                             verifier + frame + effects floor)
                                                │
                                                ▼  sir_benchmarks::emit
                                             C, compiled and compared against
                                             the original IR by the referee
```

`lower_function` is the only supported entry point into the lowerer: it is where
the structural verifier and the emission-blowup guard run. `lower()` is
crate-private for that reason.

The IR has no control flow. Branching is `Select`, iteration is a `Loop` with
explicit carried values, and there are no phi nodes. A branchy function therefore
flattens into a large select DAG, and the lowerer refuses when that blowup passes
a calibrated bound rather than emitting something enormous.

## Workspace

Nine crates, all commands run from `sir/`.

| Crate | Role |
|---|---|
| `sir_types` | Foundational types: `NodeId`, `Type`, `Effects`, `ConstantData`. No internal deps. |
| `sir_nodes` | `NodeKind`, `Node`, `NodeArena`, `Function`, `Module`. |
| `sir_builder` | `Builder` — type checking and effect computation at node-creation time. |
| `sir_verify` | Seven structural invariants. |
| `sir_analysis` | Read-only facts: loop and reduction facts, graph traversal. |
| `sir_mech` | The proof kernel: bitvector terms, CNF, CDCL SAT, equivalence decision, induction, integer linear arithmetic. |
| `sir_lower` | LLVM IR text → SIR. The largest crate. |
| `sir_opt` | The rewrite families and the driver. |
| `sir_benchmarks` | SIR → C emission, and `emit_c_diff`, the differential referee. |

Ten families live in `sir_opt`: `all`, `any`, `arithmetic`, `count`, `parity`,
`popcount`, `runtime_count`, `runtime_sum`, `search`, `sum`. Each recognises a
shape and refuses on any ambiguity, calls `sir_mech` through its `prove::*`
obligation and refuses if it does not discharge, then builds a `Patch` that
`patch::apply` rejects unless the replacement verifies structurally, fits the
frame, and declares the effects its nodes actually perform.

`sir_gate4b/` and `sir_tests/` exist on disk but contain no sources and are **not**
workspace members.

## Build and test

```bash
cd sir
cargo build --release            # must stay warning-free
cargo test --workspace           # 10 tests, ~1s
```

There is no lint config and no CI. `sir_benchmarks` is the only crate with a
binary (`emit_c_diff`, the referee); everything else is a library.

The test count is deliberately kept small — the standing instruction is single
digits, and it currently sits at 10. New cases go behind an existing `#[test]`
via the `run()` dispatch in `sir_opt/tests/plan.rs`, rather than as new test
functions.

## Evidence

```bash
bash sir/scripts/verify.sh fast   # tests, base corpora, external Embench, native
bash sir/scripts/verify.sh full   # adds AArch64, fuzz, sanitizers
```

Green means every corpus reports `EMITC_SUMMARY` with `mismatched=0` and
`harness_errors=0`, workspace tests pass, and native totals equal the AArch64 and
sanitizer totals section for section. External corpora come from
`scripts/fetch-external.sh`; a missing corpus is a `WARN`, not a pass.

## Working rules

These are enforced somewhere in the tree, and they were learned the hard way.

- **Fail closed.** A refusal is a correct outcome; a guess is not. When a guard
  costs firings, keep the guard and record the cost.
- **Performance never decides correctness.** A counterexample on a defined
  execution blocks the transformation until the applicability condition is fixed.
- **A check that cannot fail has not checked anything.** A run that reports
  nothing, a gate that cannot fire, and a fixture that only ever sees well-formed
  input are all silent passes. This has been the defective component more often
  than the code has.
- **Find the bound before writing the code.** Reach work in `sir_lower` needs a
  counted target.
- **Record counterexamples, negative results and corrections** in
  [`docs/CAPABILITY_LEDGER.md`](docs/CAPABILITY_LEDGER.md) — including corrections
  to earlier entries, marked in place rather than silently edited.
