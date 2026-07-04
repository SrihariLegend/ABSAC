# ABSAC

**Automatic Bitwise Superoptimization of Arbitrary Code** — a research compiler exploring semantic recognition and verified bitwise transformations that are difficult to express using conventional syntax-directed optimization.

"Arbitrary" refers to arbitrary source programs as input. Only regions recognized as belonging to supported semantic transformation families are rewritten.

## Why this compiler exists

Traditional optimizing compilers are largely syntax-directed: they optimize the representation already present in the program.

ABSAC instead attempts to **recognize the underlying computation**, **prove semantic equivalence** of alternative representations, and **rewrite the program** into those representations. Unlike conventional optimization passes, ABSAC separates recognition, proof, and rewriting. Every transformation is first recognized, then formally verified for semantic equivalence, and only then mechanically rewritten.

The long-term goal is automatic discovery and application of bitwise formulations that today are typically written manually by expert programmers.

## Current scope

Research focus (v0.1):

- Boolean collections
- Bitsets
- Reduction operators: cardinality (`+`), disjunction (`||`), conjunction (`&&`), exclusive (`^`)
- Popcount-based transformations (fully implemented for BS001)

Additional transformation families (disjunctive, conjunctive, exclusive) are semantically recognized but await verification and rewrite recipes.

## Knowledge Pipeline

```
Source Program
      |
      v  sir_builder
SIR
      |
      v  sir_analysis
Compiler Facts           "What is provably true?"
      |
      v  sir_semantics
Semantic Truths          "What computation is being performed?"
Structural Descriptions  "How is the data organized?"
      |
      v  sir_inference
Representation Beliefs   "Which representation best explains it?"
Transformation Contexts  "What would have to be true to transform it?"
      |
      v  sir_generation
Candidate Plans          "What implementations are possible?"
      |
      v  sir_verification
Equivalence Proofs       "Is the rewrite mathematically correct?"
      |
      v  sir_selection
Cost Scores              "Which proven rewrite should we apply?"
      |
      v  sir_rewrite
Verified Mutations       "Execute the selected winner."
```

Each layer consumes only the knowledge of the immediately preceding layer. No layer reads upward or across.

### The Problem

Compilers optimize syntax. Humans optimize representations. Given:

```rust
let mut count = 0;
for i in 0..64 {
    if board[i] {
        count += 1;
    }
}
```

A traditional syntax-directed optimizer primarily sees: loop, load, branch, add. A human sees: **cardinality of a finite set.** ABSAC recognizes this as a bitset and proposes `popcount(board)` as an equivalent, faster implementation — then proves the transformation correct before applying it.

## Implementation Status

| # | Capability | Status |
|---|-----------|--------|
| 1 | SIR — typed SSA-form functional IR | Complete |
| 2 | SAF — 9 compiler analyses (Facts) | Complete |
| 3 | SRI — semantic reasoning + representation inference (Truths + Beliefs) | Complete |
| 4 | CGE — transformation planning (Contexts + Plans) | Complete |
| 5 | Equivalence verification (Proofs) | Complete |
| 6 | Verified rewriting (Mutations) | Complete |
| 7 | Cost model + Selection | Complete |
| 8 | End-to-end optimizer | In progress |

### Benchmark Coverage

| Benchmark | Pattern | Operator | Concept | Rewrite |
|-----------|---------|----------|---------|---------|
| BS001 | `count += board[i]` | `+` | CardinalityReduction | Popcount (complete) |
| BS002 | `found \|\|= board[i]` | `\|\|` | DisjunctiveReduction | Recognized, awaiting recipe |
| BS003 | `all &&= board[i]` | `&&` | ConjunctiveReduction | Recognized, awaiting recipe |
| BS004 | `parity ^= board[i]` | `!=` | ExclusiveReduction | Recognized, awaiting recipe |

## Quick Start

```bash
cd sir
cargo build
cargo test          # 380+ tests, all passing
```

**Requirements:** Rust 2021 edition (stable).

## Repository Structure

```
ABSAC/
├── README.md
├── CLAUDE.md                   # Project instructions
├── sir/                        # Semantic IR — the active component
│   ├── Cargo.toml              # Workspace manifest (15 crates)
│   ├── README.md               # SIR-specific documentation
│   ├── crates/
│   │   ├── sir_types/          # Type system, NodeId, Effects, CostProfile
│   │   ├── sir_nodes/          # NodeKind (40+ variants), NodeArena, Function
│   │   ├── sir_builder/        # Type-safe construction API
│   │   ├── sir_printer/        # Text + JSON serialization
│   │   ├── sir_verify/         # Graph invariant verification (7 checks)
│   │   ├── sir_analysis/       # 9 compiler analyses
│   │   ├── sir_semantics/      # Semantic truth + structural recognition (7 recognizers)
│   │   ├── sir_inference/      # Representation belief inference
│   │   ├── sir_transform/      # Transformation contract
│   │   ├── sir_generation/     # Candidate plan generation (4 strategies)
│   │   ├── sir_verification/   # Proof engine (symbolic + exhaustive backends)
│   │   ├── sir_selection/      # Cost model + deterministic selector
│   │   ├── sir_rewrite/        # Verified rewrite engine (subgraph patching)
│   │   ├── sir_optimizer/      # Fixed-point optimization driver
│   │   └── sir_tests/          # Integration tests
│   └── docs/                   # Design documents (13 specs)
├── phase0.xml                  # External project data
└── phase1.xml                  # External project data
```

## Architecture

### Crate dependency graph (layered, no cycles)

```
sir_types                 — foundational (no internal deps)
  │
  ├── sir_transform       — depends on sir_types
  │
  └── sir_nodes           — depends on sir_types
        │
        ├── sir_builder   — depends on sir_nodes, sir_types
        ├── sir_printer   — depends on sir_nodes, sir_types
        ├── sir_verify    — depends on sir_nodes, sir_types
        │
        ├── sir_analysis  — depends on sir_nodes, sir_types (read-only)
        │     │
        │     ▼
        ├── sir_semantics — depends on sir_types, sir_nodes, sir_analysis, sir_transform
        │     │
        │     ├── sir_inference  — depends on sir_types, sir_semantics, sir_transform
        │     │
        │     └── sir_generation — depends on sir_types, sir_transform, sir_semantics
        │           │
        │           ├── sir_verification — depends on sir_types, sir_transform, sir_generation
        │           │     │
        │           │     ├── sir_selection — depends on sir_types, sir_generation, sir_verification
        │           │     │     │
        │           │     │     ▼
        │           │     └── sir_rewrite — depends on sir_types, sir_nodes, sir_transform,
        │           │                        sir_generation, sir_verification, sir_verify,
        │           │                        sir_semantics
        │           │                        │
        │           │                        ▼
        │           │              sir_optimizer — depends on all above (fixed-point driver)
        │
        └── sir_tests     — integration tests (depends on all of the above)
```

## Design Philosophy

1. **Lossless** — Every semantic fact from the source language is representable
2. **Language-independent** — Rust, C, C++, Zig, Go all lower into identical SIR
3. **SSA form** — Every value assigned exactly once, no mutable variables
4. **Typed** — Every node has an exact type
5. **Explicit effects** — Pure vs. impure operations are distinguishable
6. **Extensible** — SIMD, GPU, tensors, FSMs can be added without redesign

## IR Design

SIR is a **functional IR** — no basic blocks, no control-flow graph, no phi nodes:

- **Select** replaces `if`/`else` (branchless conditional)
- **Loop** handles iteration with explicit carried inputs/outputs (no back-edges)
- **No control flow** beyond Select, Loop, and Return
- **Explicit effects tracking** per node (pure, memory, allocation, IO, atomic)

## License

MIT
