# 0000 — Project Vision

## What is ABSAC?

**Automatic Bitwise Superoptimization of Arbitrary Code** — a tool that reads
plain source code and produces an equivalent version where every fragment that
*can* be expressed as bitwise operations *is* expressed as bitwise operations.

## What is SIR?

**Semantic IR** is the mathematical foundation of ABSAC. It is not an
instruction representation — it is a representation of **program meaning**.

Every optimization, proof, synthesis pass, and performance model operates
exclusively on SIR. No pass is allowed to inspect source syntax or LLVM IR
directly.

## Design Philosophy

1. **Lossless** — Every semantic fact from the source language is representable
2. **Language-independent** — Rust, C, C++, Zig, Go all lower into identical SIR
3. **SSA form** — Every value assigned exactly once, no mutable variables
4. **Typed** — Every node has an exact type
5. **Explicit effects** — Pure vs. impure operations are distinguishable
6. **Extensible** — SIMD, GPU, tensors, FSMs can be added without redesign

## v0.1 Scope

Core data model only:
- Type system with all primitive and compound types
- NodeKind enumeration (40+ IR operations)
- Arena-based graph storage (BTreeMap)
- Type-safe Builder API with effect auto-computation
- Graph invariant verifier (7 checks)
- Human-readable pretty-printer (compact + detailed modes)
- JSON serialization with serde roundtrip

**Not in v0.1**: Optimization passes, source-language parsing, lowering, SMT
verification, cost modeling, code generation.

## Repository

```
sir/
├── Cargo.toml              (workspace)
├── crates/
│   ├── sir_types/          (type system)
│   ├── sir_nodes/          (graph structures)
│   ├── sir_builder/        (construction API)
│   ├── sir_printer/        (text + JSON output)
│   ├── sir_verify/         (graph verification)
│   └── sir_tests/          (integration tests)
└── docs/                   (specifications)
```
