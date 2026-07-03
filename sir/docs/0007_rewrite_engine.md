# 0007 — Rewrite Engine (Future)

## Planned for v0.4+

The rewrite engine applies semantics-preserving transformations to SIR graphs.

### Types of Rewrites

1. **Canonicalization** — normalize to a standard form
2. **Algebraic simplification** — `x & 0 → 0`, `x | 0 → x`
3. **Bitwise formulation** — the core of ABSAC
4. **Strength reduction** — `x * 2 → x << 1`
5. **Constant folding** — evaluate constant subgraphs
6. **Dead code elimination** — remove unused nodes

### Representation

Rewrites are expressed as pattern-matching rules:
```
(pattern) → (replacement)  if (condition)
```

### Bitwise Reformulation

The critical rewrite is recognizing code patterns that can be expressed
as bitwise operations:

| Original Pattern | Bitwise Reformulation |
|-----------------|----------------------|
| `if cond { a } else { b }` | `(a & mask) \| (b & !mask)` where `mask = -cond` |
| `for i in 0..64 { if arr[i] { ... } }` | popcount loop over u64 bitfield |
| `if x > 0 { a } else { b }` | sign-bit mask: `(a & mask) \| (b & ~mask)` where `mask = x >> 63` |
| `count += if cond { 1 } else { 0 }` | `count += cond as u64` (population count accumulation) |

### Search Strategy

Rather than pattern-matching against a fixed library, the rewrite engine must
**discover** bitwise formulations from scratch. This requires:

1. **Representation inference** — identify opportunities (see 0008)
2. **Synthesis** — search the space of bitwise expression trees
3. **Verification** — prove equivalence of candidate rewrites (see 0006)
4. **Cost evaluation** — ensure the rewrite is actually faster (see 0009)
