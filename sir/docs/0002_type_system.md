# 0002 — Type System

## Type Hierarchy

```
Type
├── Unit                — ()
├── Bool                — true | false
├── Integer             — { width, signed, overflow }
│   ├── IntegerWidth    — I8 | I16 | I32 | I64 | I128
│   └── OverflowBehavior — Wrapping | Saturating | Unchecked
├── Float               — { width: FloatWidth }
│   └── FloatWidth      — F32 | F64
├── Pointer             — *const T | *mut T
├── Reference           — &T | &'a T | &mut T
├── Array               — [T; N]
├── Slice               — [T]
├── Tuple               — (T1, T2, ...)
├── Struct              — named { field: T, ... }
├── Enum                — named { Variant(T...), ... }
├── Function            — fn(params) -> ret
├── BitVector           — arbitrary-width bit vector
└── Unknown             — opaque / unresolved
```

## Integer Semantics

Integers carry three orthogonal properties:

1. **Width** — 8, 16, 32, 64, or 128 bits
2. **Signedness** — signed (two's complement) or unsigned
3. **Overflow behavior**:
   - `Wrapping` — wrap on overflow (Rust's default, C unsigned)
   - `Saturating` — clamp to min/max
   - `Unchecked` — undefined behavior on overflow (C signed)

## Recap: Why Box<Type>?

Recursive type variants (Pointer, Reference, Array, Slice, Function) use
`Box<Type>` to keep the enum size bounded. This allows:
- `Pointer<Pointer<i32>>` — pointer to pointer
- `Struct { next: Pointer<Self> }` — linked structures
- `Array<Bool, 8>` — fixed-size bitfield

## Type Equality

Two types are equal iff they are structurally identical. Named types
(Struct, Enum) compare by both name AND structural content.

## Type Checking

The builder and verifier enforce type rules for all operations. See
`0004_graph_invariants.md` for the complete rules table.

## Convenience Constructors

```rust
Type::i8()    → Integer { width: I8, signed: true, overflow: Wrapping }
Type::u64()   → Integer { width: I64, signed: false, overflow: Wrapping }
Type::f32()   → Float { width: F32 }
Type::f64()   → Float { width: F64 }
```
