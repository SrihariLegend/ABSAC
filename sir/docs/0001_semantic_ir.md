# 0001 — Semantic IR v0.1

## Overview

SIR is a **functional intermediate representation** in SSA form. It represents
programs as typed, directed acyclic graphs (DAGs) with explicit loop constructs.

## Key Properties

### Lossless
Every semantic fact from the source language (integer width, signedness,
overflow behavior, aliasing, mutability, purity) is encoded in the IR.

### Language-Independent
The IR never exposes source-language syntax. Multiple languages lower into
identical SIR.

### SSA Form
Every value is assigned exactly once. Mutations become new SSA values.
Selection uses `Select` (branchless); loops use `Loop` constructs (not back-edges
or phi nodes).

### Typed
Every node carries an exact `Type`. Type inference is not deferred.

### Explicit Effects
Side effects (memory reads/writes, allocation, I/O, atomics) are tracked
per-node via `Effects` bitflags.

## Core Architecture

```
Source Language
    ↓
  Parser (future)
    ↓
  AST → HIR → MIR (future)
    ↓
  SIR
    ↓
  Optimization / Verification / Synthesis
    ↓
  Code Generation (future)
```

## Core Data Structures

### NodeId
```rust
pub struct NodeId(pub u64);
```
Globally unique within a module. Never reused. Copy type.

### Type
```rust
pub enum Type {
    Unit, Bool,
    Integer { width: IntegerWidth, signed: bool, overflow: OverflowBehavior },
    Float { width: FloatWidth },
    Pointer { pointee: Box<Type>, mutable: bool },
    Reference { pointee: Box<Type>, mutable: bool, lifetime: Option<String> },
    Array { element: Box<Type>, length: usize },
    Slice { element: Box<Type> },
    Tuple { elements: Vec<Type> },
    Struct { name: String, fields: Vec<(String, Type)> },
    Enum { name: String, variants: Vec<(String, Vec<Type>)> },
    Function { params: Vec<Type>, ret: Box<Type> },
    BitVector { width: usize },
    Unknown,
}
```

### Node
```rust
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub ty: Type,
    pub effects: Effects,
    pub metadata: Metadata,
    pub span: Span,
}
```

### Function
```rust
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_ty: Type,
    pub arena: NodeArena,
    pub return_node: Option<NodeId>,
}
```

### Module
```rust
pub struct Module {
    pub name: String,
    pub functions: Vec<Function>,
}
```

## Serialization

All types derive `serde::Serialize` and `serde::Deserialize` for JSON roundtrip.
Binary serialization is planned for v0.2.

## Testing

Every crate has exhaustive unit tests. Integration tests cover the full
build→verify→print→serialize pipeline. See `crates/sir_tests/tests/`.
