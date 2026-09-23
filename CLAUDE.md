# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**ABSAC** (Automatic Bitwise Superoptimization of Arbitrary Code)

*Evolved Research Question:*
> **Can a compiler recover the mathematical object implemented by arbitrary code and choose the best representation for that object?**

ABSAC is a semantic compiler. It lowers LLVM IR into a typed functional IR, recognises mathematical domains embedded in loops, discharges an obligation proving an equivalent cheaper implementation, and emits C.

**Two things to know before planning work here:**

- **What is demonstrated.** On 19 independently authored benchmarks, the only family that fires at any optimization level is `arithmetic` (multiply→shift). The domain families recognise counted, effect-free, single-stream array reductions — shapes clang vectorizes before the lowerer ever sees them. The one third-party kernel that has been measured (Redis `redisPopcount`) lost to clang -O3.
- **What is not.** `docs/ROADMAP.md`, `docs/NORTH_STAR.md` and older `docs/GATE*` files describe intended or historical systems, and some of their claims were never demonstrated beyond project-authored kernels. **`docs/CAPABILITY_LEDGER.md` is the record** — quote it, not those documents, when stating what exists.

## Workspace

Nine crates, ~33k lines. All commands run from `sir/`.

| Crate | LOC | Role |
|---|---:|---|
| `sir_types` | 935 | Foundational types: `NodeId`, `Type`, `Effects`, `ConstantData`. No internal deps. |
| `sir_nodes` | 587 | `NodeKind`, `Node`, `NodeArena`, `Function`, `Module`. |
| `sir_builder` | 915 | `Builder` — type checking and effect computation at node-creation time. |
| `sir_verify` | 1002 | Seven structural invariants. |
| `sir_analysis` | 627 | Read-only facts: loop/reduction facts, graph traversal helpers. |
| `sir_mech` | 6950 | The proof kernel: bitvector terms, CNF, CDCL SAT, equivalence decision, induction, integer linear arithmetic. |
| `sir_lower` | 11645 | LLVM IR text → SIR. A single `lib.rs`. The largest crate and the project's LOC-budget risk — additions here need a counted target. |
| `sir_opt` | 5380 | The rewrite families and the driver. |
| `sir_benchmarks` | 4877 | SIR → C emission, and the `emit_c_diff` differential referee. |

`crates/sir_gate4b/` and `crates/sir_tests/` exist on disk but contain no sources and are **not** workspace members.

Dependencies (production only — `sir_lower` is a dev-dependency of `sir_opt`, used by its tests):

```
sir_types      ← (nothing)
sir_nodes      ← sir_types
sir_builder    ← sir_nodes, sir_types
sir_verify     ← sir_builder, sir_nodes, sir_types
sir_analysis   ← sir_builder, sir_nodes, sir_types, sir_verify
sir_mech       ← sir_types
sir_lower      ← sir_builder, sir_nodes, sir_types, sir_verify
sir_opt        ← sir_analysis, sir_mech, sir_nodes, sir_types, sir_verify
sir_benchmarks ← sir_analysis, sir_lower, sir_nodes, sir_opt, sir_types
```

### Pipeline

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

`lower_function` is the **only** supported entry point into the lowerer: it is where the structural verifier and the emission-blowup guard run. `lower()` is crate-private for that reason.

## Core data model (`sir_types` + `sir_nodes`)

- **`NodeId`** (`sir_types/src/node_id.rs`) — `Copy` newtype over `u64`, displayed as `%0`, `%1`. Monotonically increasing within a function; keys in `NodeArena`.
- **`Type`** (`sir_types/src/types.rs`) — Unit, Bool, Integer (width/signedness/overflow), Float, Pointer, Reference, Array, Slice, Tuple, Struct, Enum, Function, BitVector.
- **`Effects`** (`sir_types/src/effects.rs`) — `bitflags`: `READ_MEMORY`, `WRITE_MEMORY`, `ALLOCATE`, `IO`, `ATOMIC`. Every node carries a mask; pure nodes are `Effects::empty()`.
- **`NodeKind`** (`sir_nodes/src/node_kind.rs`) — 40+ variants: arithmetic, bitwise (incl. `Popcount`, `LeadingZeros`, `TrailingZeros`), comparisons, boolean, `Select`, memory (`Load`/`Store`/`ArrayAccess`/…), calls (`Call`/`Intrinsic`/`ExternalCall`), `Loop`, `Iterator`, `Return`.
- **`NodeArena`** (`sir_nodes/src/arena.rs`) — `BTreeMap<NodeId, Node>`, so iteration order is deterministic. Inserting a duplicate id returns the old node (SSA violation).
- **`Function`** (`sir_nodes/src/function.rs`) — `{ name, params, return_ty, arena, return_node }`. Parameters appear both as signature entries and as `Parameter` nodes.

### Builder (`sir_builder`)

- NodeIds are auto-generated after the parameters; type checking happens at creation, so most methods return `Result<NodeId, BuildError>`.
- `compute_effects()` derives a node's mask from its kind. Two consequences worth knowing, because both have caused bugs: kinds with no explicit arm (`ArrayAccess` among them) fall through to `Effects::empty()`, and `Builder::r#loop` unions the effects of **body** members only — a loop whose body stores carries `WRITE_MEMORY`, while an effectful *carried input* or *termination* does not show on the loop node.
- `return_value()` enforces a single return. `create_node()` escapes type checking when needed.

### Verifier (`sir_verify`)

Seven checks, all run (errors accumulate rather than short-circuit): SSA, references, cycle freedom (loop back-edges allowed), types, return, parameters, loop well-formedness (`carried_inputs.len() == outputs.len()`).

Note what the verifier does **not** check: that a loop's carried-input and output types agree. They can differ, the emitter will spell the C accordingly, and nothing objects — one such mismatch produced C that clang accepted and `aarch64-gcc` rejected.

### Analysis (`sir_analysis`)

`facts.rs`, `graph.rs`, `loops.rs`, `manager.rs`. The surviving facts are the loop and reduction facts (`LoopFact`, `ReductionVar`, `FactDatabase`); the older nine-analysis layer is gone. `estimate_trip_count` is where trip counts come from and it is deliberately conservative: a guarded termination yields no count unless the other conjunct is provably constant-true. `graph.rs` provides `users()`, `topological_sort()`, `reachable()`, `transitive_inputs()`.

### Families (`sir_opt`)

Ten family modules — `all`, `any`, `arithmetic`, `count`, `parity`, `popcount`, `runtime_count`, `runtime_sum`, `search`, `sum` — plus `family` (registry), `optimize` (driver), `patch` (guarded application) and `prove` (the obligations). Each family:

1. recognises a shape and refuses on any ambiguity,
2. calls `sir_mech` through its `prove::*` obligation and refuses if it does not discharge,
3. builds a `Patch`, which `patch::apply` rejects unless the replacement verifies structurally, fits the frame, and declares the effects its nodes actually perform.

`search` is the one family whose applicability also depends on a measurement (it is retained only when measured faster).

### Proof kernel (`sir_mech`)

`bv` (bitvector term language + evaluator), `cnf` (Tseitin bit-blasting), `sat` (deterministic CDCL — a refuted claim comes back with a model), `equiv` (the decision procedure), plus `kernel`, `kernighan`, `arith`, `scan`, `term`. Nothing here trusts a caller: a claim is either discharged or refuted with a counterexample.

## IR design principles

- **Functional SSA** — every value assigned once; mutations become new values.
- **Branchless selection** — `Select { cond, true_val, false_val }` replaces `if`/`else`. There are no basic blocks and no CFG — which is why lowering a branchy function flattens into one large select DAG, and why the lowerer refuses when that blowup passes a calibrated bound.
- **Loop with explicit carried values** — `Loop { body, termination, outputs, carried_inputs }`, no phi nodes.
- **No control flow** beyond `Select`, `Loop`, `Return`.

## Build & test

```bash
cd sir
cargo build --release            # must stay warning-free
cargo test --workspace           # 10 tests, ~1 s
cargo test -p <crate>            # one crate
```

There is no lint config and no CI. `sir_benchmarks` is the only crate with a binary (`emit_c_diff`, the referee); everything else is a library.

**Keep the test count in single digits** — a standing instruction from the project owner, aimed at preventing test bloat. Add cases behind an existing `#[test]` (see `sir_opt/tests/cases/` and the `run()` dispatch in `sir_opt/tests/plan.rs`) rather than adding new test functions.

## Evidence

```bash
bash sir/scripts/verify.sh fast   # tests, base-14, external Embench (19 corpora), native
bash sir/scripts/verify.sh full   # adds AArch64, fuzz, sanitizers
```

Green means: every corpus reports `EMITC_SUMMARY` with `mismatched=0` and `harness_errors=0`, workspace tests pass, and native totals equal the AArch64 and sanitizer totals section for section. External corpora come from `scripts/fetch-external.sh`; a missing corpus is a `WARN`, not a pass.

Working rules that this project learned the hard way, all of them enforced somewhere in the tree:

- **Fail closed.** A refusal is a correct outcome; a guess is not. When a guard costs firings, keep the guard and record the cost.
- **Performance never decides correctness.** A counterexample on a defined execution blocks the transformation until the applicability condition is fixed.
- **A check that cannot fail has not checked anything.** A run that reports nothing, a gate that cannot fire, and a fixture that only ever sees well-formed input are all silent passes.
- **Find the bound before writing the code.** Reach work in `sir_lower` needs a counted target; today's evidence retired two proposed reach changes precisely because nobody had counted first.
- Record counterexamples, negative results and corrections in `docs/CAPABILITY_LEDGER.md` — including corrections to earlier entries, marked in place rather than silently edited.
