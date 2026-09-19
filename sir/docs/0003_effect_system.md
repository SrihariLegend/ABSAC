# 0003 — Effect System

## Motivation

The SIR must distinguish pure expressions (which can be freely reordered,
duplicated, or eliminated) from impure ones (which cannot). This is essential
for optimization legality and correctness.

## Effect Flags

```rust
bitflags! {
    pub struct Effects: u32 {
        const READ_MEMORY  = 0b0000_0001;
        const WRITE_MEMORY = 0b0000_0010;
        const ALLOCATE     = 0b0000_0100;
        const IO           = 0b0000_1000;
        const ATOMIC       = 0b0001_0000;
    }
}
```

## Purity

`Effects::empty()` (all bits zero) represents a **pure** operation — one with
no observable side effects. Pure operations include:
- Arithmetic (Add, Sub, Mul, Div, Rem, Neg)
- Bitwise (And, Or, Xor, Not, Shl, Shr, Rol, Ror, Popcount, LeadingZeros, TrailingZeros)
- Comparisons (Eq, Ne, Lt, Le, Gt, Ge)
- Boolean (BoolAnd, BoolOr, BoolNot)
- Select (branchless conditional)
- FieldAccess, ArrayAccess

## Impure Operations

| Operation | Effects |
|-----------|---------|
| Load | READ_MEMORY |
| Store | WRITE_MEMORY |
| Allocate | ALLOCATE |
| Deallocate | WRITE_MEMORY |
| Call (general) | READ_MEMORY \| WRITE_MEMORY |
| Intrinsic | caller-specified |
| ExternalCall | caller-specified |
| Iterator | READ_MEMORY |
| Loop | READ_MEMORY \| WRITE_MEMORY (conservative v0.1) |

## Effect Propagation

In v0.1, effects are assigned at node creation time by the Builder. Future
versions will propagate effects through the call graph (a pure function called
with pure arguments yields a pure call).

## Querying Effects

```rust
effects.is_pure()         // true if empty
effects.touches_memory()  // true if any of READ_MEMORY | WRITE_MEMORY | ALLOCATE
```
