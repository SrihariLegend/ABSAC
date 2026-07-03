# Semantic IR (SIR) v0.1

A compiler intermediate representation for **program meaning** — not instruction encoding.

SIR is the mathematical foundation of the ABSAC (Automatic Bitwise Superoptimization of Arbitrary Code) project. It represents programs as typed, SSA-form functional IR graphs with explicit effects, suitable for optimization, formal verification, and synthesis.

## Repository Layout

```
crates/
├── sir_types/     — Type system, NodeId, Effects, Span, Metadata, Constants
├── sir_nodes/     — NodeKind enum, Node, NodeArena, Function, Module
├── sir_builder/   — Type-safe construction API with type checking
├── sir_printer/   — Human-readable text + JSON serialization
├── sir_verify/    — Graph invariant verification (SSA, types, cycles, effects)
└── sir_tests/     — Integration tests
docs/              — Design documents and specifications
```

## Build

```bash
cargo build
cargo test
```

## Design

SIR is a **functional IR** in SSA form:
- Every value is assigned exactly once
- No mutable variables — mutations become new SSA values
- Selection via `Select` (not `If`)
- Loops via `Loop` construct (not back-edges)
- No phi nodes — loop-carried values via explicit Loop outputs
- Explicit effects tracking (pure, memory, allocation, IO, atomic)

## Status

v0.1 — Core data model, builder, verifier, printer, serialization.
No optimization passes or source-language parsing yet.
