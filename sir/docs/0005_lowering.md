# 0005 — Lowering (Future)

## Planned for v0.2+

Lowering transforms source-language ASTs/HIR/MIR into SIR. Each source language
requires its own lowering pass, but all produce identical SIR.

### Rust → SIR

The Rust lowering path:
1. `rustc` → HIR (via rustc_driver)
2. HIR → THIR (typed HIR)
3. THIR → MIR (via rustc_mir)
4. MIR → SIR

### C/C++ → SIR

Options under consideration:
- Clang plugin producing SIR directly
- CIL (C Intermediate Language) → SIR translation
- libclang AST → SIR

### Key Challenges

- **Aliasing information**: Rust's borrow checker provides precise aliasing info.
  For C, we need alias analysis (Andersen/Steensgaard).
- **Lifetime erasure**: SIR does not need full lifetime information for most
  optimizations. Only `Reference` types carry optional lifetimes.
- **Unsafe code**: `unsafe` blocks in Rust require conservative assumptions.
- **FFI boundaries**: External function calls are treated as fully impure.

### Open Questions

- Should MIR→SIR lowering happen inside rustc or as a standalone tool?
- How to preserve debug information through the lowering?
- What is the minimal set of MIR constructs needed before lowering?
