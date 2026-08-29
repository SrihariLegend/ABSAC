# Hacker's Delight Ontology Roadmap

This roadmap structures the implementation of Hacker's Delight bitwise algorithms not as independent hacks, but as a **knowledge dependency graph**. 

Our goal is to maximize knowledge reuse. A single core concept (like `ClearLowestSetBit` or `BitPermutation`) should unlock multiple downstream capabilities.

## Knowledge Dependency Map

### 1. The Mask Algebra Branch

```text
HD006  Clear Lowest Set Bit (x & (x - 1))
Requires:
  Concept: ClearLowestSetBit
  Representation: MaskAlgebra
  Rewrite: blsr

HD001  Isolate Lowest Set Bit (x & -x)
Requires:
  Concept: LowestSetBitMask
  Representation: MaskAlgebra
  Rewrite: blsi

HD007  Isolate Lowest Clear Bit (~x & (x + 1))
Requires:
  Concept: LowestClearBitMask
  Representation: MaskAlgebra
  Rewrite: blsi + bitwise NOT

HD008  Set Lowest Clear Bit (x | (x + 1))
Requires:
  Concept: SetLowestClearBit
  Representation: MaskAlgebra
  Rewrite: blsmsk + bitwise OR / custom
```

### 2. The Iteration and Cardinality Branch

```text
HD002  Brian Kernighan Popcount
Requires:
  Concept: BitsetIteration
  Closure: ClearLowestSetBit → BitsetIteration
  Representation: BitSet
  Rewrite: Popcount

HD012  Compute Parity
Requires:
  Concept: Parity
  Closure: BitsetIteration + Modulo 2 → Parity
  Representation: BitSet
  Rewrite: Popcount + And 1 (or native parity)
```

### 3. The Permutation Branch

```text
HD003  Rotate Left / Right
Requires:
  Concept: CircularPermutation
  Closure: Shift pair → CircularPermutation
  Representation: BitPermutation
  Rewrite: rol / ror

HD004  Byte Swap
Requires:
  Concept: BytePermutation
  Closure: CombinePermutations
  Representation: BitPermutation
  Rewrite: bswap

HD005  Reverse Bits
Requires:
  Concept: BitPermutation
  Closure: CombinePermutations
  Representation: BitPermutation
  Rewrite: rbit
```

## Knowledge Gap Table

| Benchmark | Target | Missing Concepts | Missing Closure | Missing Representation | Missing Rewrite |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **HD001** | Isolate lowest set bit | `LowestSetBitMask` | — | `MaskAlgebra` | `blsi` |
| **HD006** | Clear lowest set bit | `ClearLowestSetBit` | — | `MaskAlgebra` | `blsr` |
| **HD007** | Isolate lowest clear bit | `LowestClearBitMask` | — | `MaskAlgebra` | `blsi` (composed) |
| **HD002** | BK Popcount | `BitsetIteration` | `ClearLowestSetBit` → `BitsetIteration` | `BitSet` | `Popcount` |
| **HD012** | Parity | `Parity` | `BitsetIteration` + `Modulo 2` | `BitSet` | `Popcount` + `And` |
| **HD003** | Rotate | `CircularPermutation`| `Shift pair` → `CircularPermutation` | `BitPermutation` | `rol`/`ror` |
| **HD004** | Byte swap | `BytePermutation` | `CombinePermutations` | `BitPermutation` | `bswap` |
| **HD005** | Reverse bits | `BitPermutation` | `CombinePermutations` | `BitPermutation` | `rbit` |

## Execution Strategy

Instead of implementing independent algorithms, we build up the ontology from the root nodes:

1. **Mask Algebra Foundations:** Build `ClearLowestSetBit`, `LowestSetBitMask` concepts. This unlocks HD001, HD006, HD007.
2. **Set Iteration:** Build `BitsetIteration` using `ClearLowestSetBit`. This unlocks HD002 and HD012.
3. **Permutation Abstractions:** Build the `BitPermutation` representation and `CombinePermutations` closure rule to combine basic shifts. This sequentially unlocks HD003, HD004, and HD005.
