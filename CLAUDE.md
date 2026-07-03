# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**ABSAC** (Automatic Bitwise Superoptimization of Arbitrary Code) — a compiler toolchain that reads source code and produces an equivalent version where every fragment expressible as bitwise operations is expressed that way.

The active component is **SIR** (Semantic IR), located in `sir/`. SIR is a typed, SSA-form functional IR for representing program meaning — not instruction encoding. The raw `.xml` files at the repo root are external project data, not part of SIR.

### Architecture Freeze (2026-07-03)

The reasoning substrate and core IR layers are frozen. The following crates are considered **architecturally stable** — no redesigns, no interface changes beyond extension (such as supporting IR evolution):

| Crate | Status | Allowed changes |
|-------|--------|-----------------|
| `sir_types` | Frozen | New types, new `RegionMap` usage |
| `sir_nodes` | Frozen | New NodeKind variants (such as `Pack`) |
| `sir_analysis` | Frozen | New analyses (new fact types in FactDatabase) |
| `sir_semantics` | Frozen | New semantic/structural recognizers |
| `sir_inference` | Frozen | New evidence sources |
| `sir_transform` | Frozen | New enum variants (Representation, SourceStructure, Constraint, Assumption) |
| `sir_builder`, `sir_printer`, `sir_verify` | Frozen | Support for new NodeKind variants |

### Active Development Crates

The downstream translation, verification, and transformation execution layers are in active development:

| Crate | Status | Active Work Areas / Allowed changes |
|-------|--------|-------------------------------------|
| `sir_generation` | In Development / Frozen Interface | New generator strategies, refining candidate generation |
| `sir_verification` | In Development | Equivalence proof engine (exhaustive and symbolic backends) |
| `sir_rewrite` | In Development | AST/graph rewrite machinery, verified mutations and patch generation |

### Implemented Capabilities

| # | Capability | Status |
|---|-----------|--------|
| 1 | SIR — typed SSA-form functional IR | Complete |
| 2 | SAF — 9 compiler analyses (Facts) | Complete |
| 3 | SRI — semantic reasoning + representation inference (Truths + Beliefs) | Complete |
| 4 | CGE — transformation planning (Contexts + Plans) | Complete |
| 5 | Equivalence verification (Proofs) | In progress (`sir_verification`) |
| 6 | Verified rewriting (Mutations) | In progress (`sir_rewrite`) |
| 7 | Cost model (Selection) | Not started |
| 8 | End-to-end optimizer | Not started |

### Knowledge Pipeline

```
Source Program
      │
      ▼  sir_builder
SIR
      │
      ▼  sir_analysis
Compiler Facts           "What is provably true?"
      │
      ▼  sir_semantics
Semantic Truths          "What computation is being performed?"
Structural Descriptions  "How is the data organized?"
      │
      ▼  sir_inference
Representation Beliefs   "Which representation best explains it?"
Transformation Contexts  "What would have to be true to transform it?"
      │
      ▼  sir_generation [Active Development]
Candidate Plans          "What implementations are possible?"
      │
      ▼  sir_verification / sir_rewrite [Active Development]
Verification → Rewrite → Cost → Selection
```

Each layer consumes only the knowledge of the immediately preceding layer. No layer reads upward or across. No layer below `sir_semantics` inspects SIR directly.

*Note: The layers `sir_generation` and `sir_rewrite` (and the verification/rewrite phases) are under active development. While the strict feed-forward boundaries of the knowledge pipeline are the architectural target, they are considered aspirational during current prototyping.*

## Build & Test

All commands run from `sir/`:

```bash
cargo build              # build all crates
cargo test               # run all tests (350+ tests, all passing)
cargo test -p <crate>    # run one crate's tests (e.g. sir_verify, sir_builder)
cargo test <test_name>   # run a single test by name
```

There is no lint config, no CI, and no binary crate — this is a library-only workspace.

## Architecture

### Crate dependency graph (layered, no cycles)

```
sir_types                 — no internal deps (foundational)
  │
  ├─► sir_transform       — depends on sir_types
  │
  └─► sir_nodes           — depends on sir_types
        │
        ├─► sir_builder   — depends on sir_nodes, sir_types
        ├─► sir_printer   — depends on sir_nodes, sir_types
        ├─► sir_verify    — depends on sir_nodes, sir_types
        │
        ├─► sir_analysis  — depends on sir_nodes, sir_types (read-only analysis framework)
        │     │
        │     ▼
        ├─► sir_semantics — depends on sir_types, sir_nodes, sir_analysis, sir_transform
        │     │
        │     ├─► sir_inference  — depends on sir_types, sir_semantics, sir_transform
        │     │
        │     └─► sir_generation — depends on sir_types, sir_transform, sir_semantics
        │           │
        │           ├─► sir_verification — depends on sir_types, sir_transform, sir_generation
        │           │     │
        │           │     ▼
        │           └────► sir_rewrite — depends on sir_types, sir_nodes, sir_transform, sir_generation, sir_verification, sir_verify, sir_semantics
        │
        └─► sir_tests     — depends on sir_types, sir_nodes, sir_builder, sir_printer, sir_verify (integration tests only)
```

### Core data model (`sir_types` + `sir_nodes`)

- **`NodeId`** (`sir_types/src/node_id.rs`) — a `Copy` newtype over `u64`. Displayed as `%0`, `%1`, etc. Monotonically increasing within a function. Used as keys in `NodeArena`.
- **`Type`** (`sir_types/src/types.rs`) — enum covering Unit, Bool, Integer (with width/signedness/overflow), Float, Pointer, Reference (with optional lifetime), Array, Slice, Tuple, Struct, Enum, Function, and BitVector.
- **`Effects`** (`sir_types/src/effects.rs`) — `bitflags` bitmask: `READ_MEMORY`, `WRITE_MEMORY`, `ALLOCATE`, `IO`, `ATOMIC`. Every node carries an effects mask. Pure nodes have `Effects::empty()`.
- **`NodeKind`** (`sir_nodes/src/node_kind.rs`) — 40+ variant enum of IR operations: arithmetic (Add/Sub/Mul/Div/Rem/Neg), bitwise (And/Or/Xor/Shl/Shr/Rol/Ror/Not/Popcount/LeadingZeros/TrailingZeros), comparisons (Eq/Ne/Lt/Le/Gt/Ge), boolean (BoolAnd/BoolOr/BoolNot), Select (branchless conditional), memory (Load/Store/Allocate/Deallocate/FieldAccess/ArrayAccess), calls (Call/Intrinsic/ExternalCall), Loop (with explicit carried inputs/outputs — no phi nodes), Iterator, and Return. Each variant carries `NodeId` operands.
- **`Node`** (`sir_nodes/src/node.rs`) — `{ id, kind, ty, effects, metadata, span }`.
- **`NodeArena`** (`sir_nodes/src/arena.rs`) — `BTreeMap<NodeId, Node>` with deterministic iteration order. Inserting a duplicate `NodeId` returns the old node (SSA violation).
- **`Function`** (`sir_nodes/src/function.rs`) — `{ name, params, return_ty, arena, return_node }`. Parameters are stored both as `Param` entries (for the signature) and as `Parameter` nodes in the arena (for uniform graph traversal).
- **`Module`** (`sir_nodes/src/module.rs`) — top-level compilation unit: `{ name, functions }`.

### Builder (`sir_builder`)

The `Builder` wraps a `Function` under construction. Key design decisions:
- NodeIds are auto-generated from an internal counter starting after parameters.
- Type checking happens at node creation time — most methods return `Result<NodeId, BuildError>`.
- Effects are auto-computed by `compute_effects()` based on `NodeKind` (e.g., `Load` → `READ_MEMORY`, `Store` → `WRITE_MEMORY`).
- Arithmetic ops require both operands to be the same numeric type. Bitwise ops require integer types. Comparisons produce `Bool`. Boolean ops require `Bool` operands.
- `return_value()` enforces a single return — calling it twice returns `DuplicateReturn`.
- A low-level `create_node()` escapes type checking when needed.

### Verifier (`sir_verify`)

Seven invariant checks run on every function (all run regardless of failures — errors accumulate):
1. **SSA** — defensive duplicate `NodeId` check
2. **References** — no dangling `NodeId` references
3. **Cycles** — DAG enforcement via three-color DFS; loop body nodes may reference their own termination (allowed back-edges)
4. **Types** — structural type checking per operation kind
5. **Return** — exactly one `Return` node with matching return type
6. **Parameters** — valid indices, one-to-one with function params
7. **Loops** — termination is `Bool`, body/output/carried nodes exist, `carried_inputs.len() == outputs.len()`

### Analysis framework (`sir_analysis`)

A **read-only** analysis layer. Facts are stored in `FactDatabase` (one `HashMap` per analysis), never inside SIR nodes. Key concepts:
- **`Analysis` trait** — `{ Output, name(), analyze(func, facts) -> AnalysisResult }`
- **`AnalysisManager`** — lazy execution, caching per function via `TypeId`, invalidation support. Owns the `FactDatabase`.
- **`dataflow_inputs()`** in `graph.rs` — critical distinction from `NodeKind::input_nodes()`: filters out Loop containment edges (body, outputs, carried_inputs) that are NOT dataflow edges. All traversal-based analyses use this.
- Nine analyses: UseDef, Dominance, Constants (three-level lattice: Top→Constant→Bottom), Purity, Ranges, Alias (MustAlias/MayAlias/NoAlias), Escape, Loops (trip counts, reductions, carried vars), ValueNumbering (congruence classes).
- `graph.rs` provides: `users()`, `topological_sort()`, `reachable()`, `is_leaf()`, `transitive_inputs()`, `predecessor_map()`.

### IR design principles

- **Functional SSA** — every value assigned once, no mutable variables. Mutations become new SSA values.
- **Branchless selection** — `Select { cond, true_val, false_val }` replaces `if`/`else`. No basic blocks or CFG.
- **Loop with explicit carried values** — `Loop { body, termination, outputs, carried_inputs }`. No phi nodes. `carried_inputs` feed each iteration; `outputs` are the final values after termination. `carried_inputs.len()` must equal `outputs.len()`.
- **No control flow** beyond Select, Loop, and Return. No gotos, no branches, no exception handling.

## Subagent Dispatch (Vertex AI Goons)

The `Agent` tool only accepts Claude model aliases. To dispatch non-interactive subagents to Vertex AI models, use `pi --print` via Bash:

```bash
pi --model google-vertex/<model-id> --print "<prompt>" 2>&1
```

| Model ID | Role | Use Case |
|---|---|---|
| `gemini-3.1-pro-preview` | Heavy lifter | Complex reasoning, design, review |
| `gemini-3.5-flash` | Fast parallel | Quick analysis, bulk tasks |
| `gemini-3.1-flash-lite-preview` | Cheap scout | Trivial lookups, git queries, file counts |

- **Always dispatch in background** (`run_in_background: true`) — never block the main loop waiting for a goon. Fire multiple in parallel and the harness notifies when each completes.
- **Don't wait for results before responding** — acknowledge the dispatch, keep working, and incorporate results when they land.
- Always use `2>&1` to capture stderr alongside stdout.
- These are non-interactive — they run and return output, no conversation loop.
