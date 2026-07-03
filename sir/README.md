# Semantic IR (SIR)

A compiler intermediate representation for **program meaning** — not instruction encoding.

SIR is the mathematical foundation of the [ABSAC](../) (Automatic Bitwise Superoptimization of Arbitrary Code) project. It represents programs as typed, SSA-form functional IR graphs with explicit effects, suitable for optimization, formal verification, and synthesis.

## Repository Layout

```
sir/
├── Cargo.toml                  # Workspace manifest (13 crates)
├── README.md
├── crates/
│   ├── sir_types/              # Type system, NodeId, Effects, Span, Metadata, Constants
│   ├── sir_nodes/              # NodeKind (40+ variants), NodeArena, Function, Module
│   ├── sir_builder/            # Type-safe construction API with type checking
│   ├── sir_printer/            # Human-readable text + JSON serialization
│   ├── sir_verify/             # Graph invariant verification (7 checks)
│   ├── sir_analysis/           # 9 compiler analyses (UseDef, Dominance, Constants, …)
│   ├── sir_semantics/          # Semantic truth + structural description recognition
│   ├── sir_inference/          # Representation belief inference (evidence aggregation)
│   ├── sir_transform/          # Transformation contract (data types + invariants)
│   ├── sir_generation/         # Candidate transformation plan generation (4 strategies)
│   ├── sir_verification/       # Obligation registry (verification scaffolding)
│   ├── sir_rewrite/            # Rewrite engine (subgraph patching + recipes)
│   └── sir_tests/              # Integration tests
└── docs/                       # Design documents and specifications
```

## Build & Test

```bash
cargo build              # Build all crates
cargo test               # Run all tests (365 tests, all passing)
cargo test -p <crate>    # Run one crate's tests
cargo test <test_name>   # Run a single test by name
```

## Architecture

### Four Layers

| Layer | Crates | Question |
|-------|--------|----------|
| **Representation** | `sir_types`, `sir_nodes` | How is the program encoded? |
| **Knowledge** | `sir_analysis`, `sir_semantics`, `sir_inference` | What is the program? |
| **Planning** | `sir_transform`, `sir_generation` | What should we do about it? |
| **Execution** | `sir_verification`, `sir_rewrite` | Is it correct and worthwhile? |

### Knowledge Pipeline

```
SIR
 │
 ▼  sir_analysis      → Facts        "What is provably true?"
 │
 ▼  sir_semantics     → Truths       "What computation is being performed?"
 │                    → Structures   "How is the data organized?"
 │
 ▼  sir_inference     → Beliefs      "Which representation best explains it?"
 │                    → Contexts     "What would have to be true to transform it?"
 │
 ▼  sir_generation    → Plans        "What implementations are possible?"
```

Data flow is strictly one-way, read-only. No layer reads upward or across. No layer below `sir_semantics` inspects SIR directly.

### Crate Dependency Graph

```
sir_types          — foundational
  ↓
sir_nodes
  ↓
sir_builder   sir_printer   sir_verify   sir_analysis
                                               ↓
                                         sir_semantics
                                               ↓
                                         sir_transform
                                          ↓         ↓
                                   sir_inference   sir_generation
                                                   ↓
                                   sir_verification  sir_rewrite
```

No cycles. Dependencies are strictly one-way.

## Design

### IR Design Principles

SIR is a **functional IR** in SSA form:

- Every value is assigned exactly once
- No mutable variables — mutations become new SSA values
- **Select** replaces `if`/`else` (branchless conditional selection)
- **Loop** with explicit carried inputs/outputs replaces phi nodes and back-edges
- No basic blocks, no control-flow graph, no gotos
- Explicit effects tracking per node (pure, memory, allocation, IO, atomic)

### Core Types

| Type | Description |
|------|-------------|
| `NodeId` | `Copy` newtype over `u64`, displayed as `%0`, `%1`, … |
| `Type` | 13 variants: Unit, Bool, Integer (width/signedness/overflow), Float, Pointer, Reference, Array, Slice, Tuple, Struct, Enum, Function, BitVector |
| `Effects` | `bitflags` bitmask: `READ_MEMORY`, `WRITE_MEMORY`, `ALLOCATE`, `IO`, `ATOMIC` |
| `NodeKind` | 40+ IR operations: arithmetic, bitwise, comparisons, boolean, Select, memory, calls, Loop, Iterator, Return, Pack |
| `Node` | `{ id, kind, ty, effects, metadata, span }` |
| `NodeArena` | `BTreeMap<NodeId, Node>` with deterministic iteration order |
| `Function` | `{ name, params, return_ty, arena, return_node }` |
| `Module` | Top-level compilation unit: `{ name, functions }` |

### Verifier (7 Invariant Checks)

1. **SSA** — defensive duplicate NodeId check
2. **References** — no dangling NodeId references
3. **Cycles** — DAG enforcement via three-color DFS
4. **Types** — structural type checking per operation kind
5. **Return** — exactly one Return node with matching return type
6. **Parameters** — valid indices, one-to-one with function params
7. **Loops** — termination is Bool, body/output/carried nodes exist, counts match

### Analyses (9 Analyses)

UseDef, Dominance, Constants (three-level lattice: Top→Constant→Bottom), Purity, Ranges, Alias (MustAlias/MayAlias/NoAlias), Escape, Loops (trip counts, reductions, carried vars), ValueNumbering (congruence classes).

### SRI Engine (Semantic Representation Inference)

A three-layer knowledge architecture:

1. **Facts** — compiler analyses (provable)
2. **Truths** — semantic recognition: identifies *what computation is being performed* (deterministic, 4 concepts)
3. **Beliefs** — representation inference: *which representation best explains these operations* (heuristic, evidence-weighted)

Currently recognizes one representation: `BitSet`. The architecture supports extension to additional representations (Bitmap, DenseSet, BitField, etc.) without structural changes.

### CGE Engine (Candidate Generation)

Transforms representation beliefs into concrete candidate plans via 4 strategies:

- **BitIteration** — visit only set bits via trailing-zero scan
- **Popcount** — single `popcount(bb)` instruction
- **PackedBitfield** — change data representation (e.g., `bool[64]` → `u64`)
- **MaskConstruction** — replace boolean predicates with AND/OR/XOR masks

## Status

### Completed (v0.1)
- Core data model (types, nodes, arena, functions, modules)
- Type-safe Builder API with effect auto-computation
- Graph invariant verifier (7 checks)
- Human-readable pretty-printer (compact + detailed modes)
- JSON serialization with serde roundtrip
- 9 compiler analyses (facts layer)
- Semantic truth recognition (4 concepts)
- Structural description recognition (2 structures)
- Representation belief inference (evidence aggregation with support scoring)
- Transformation contract (Representation, Context, Constraints, Assumptions)
- Candidate plan generation (4 strategies from BitSet contexts)
- Verification obligation registry (scaffolding for proof engine)
- Rewrite engine (subgraph patching, region rewriting, popcount recipe)
- 365 tests, all passing

### Not Yet Started
- Equivalence verification (SMT-based proof)
- Cost model (microarchitectural performance prediction)
- End-to-end pipeline (analysis → rewrite → verify → select)
- Source-language lowering (Rust, C, C++ → SIR)
- Additional representations beyond BitSet

## Documentation

Design documents are in [`docs/`](docs/):

| # | Title |
|---|-------|
| 0000 | Project Vision |
| 0001 | Semantic IR Specification |
| 0002 | Type System |
| 0003 | Effect System |
| 0004 | Graph Invariants |
| 0005 | Lowering (Future) |
| 0006 | Equivalence Checking (Future) |
| 0007 | Rewrite Engine (Future) |
| 0008 | Representation Inference (Historical) |
| 0009 | Cost Model (Future) |
| 0010 | Semantic Representation Inference |
| 0011 | Transformation Planning |

## License

MIT
