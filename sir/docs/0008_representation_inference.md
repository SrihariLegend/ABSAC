# 0008 — Representation Inference (Historical)

> **Superseded by [0010 — Semantic Representation Inference](0010_semantic_representation_inference.md).**
>
> This document originally reserved the topic of representation inference for a
> future phase. The actual specification has been written as 0010, which defines
> a three-layer knowledge architecture (Facts → Truths → Beliefs) that
> supersedes the placeholder description below.

## Original placeholder (retained for reference)

Representation inference is the problem of identifying code fragments where a
bitwise representation is possible — without being told.

### The Problem

Given arbitrary source code, find every fragment where an equivalent bitwise
formulation exists. Examples:

- A `for` loop counting from 0 to 63, indexing a `bool` array → bit-pop loop
- An `if (x > 0) { a } else { b }` → branchless conditional select via sign mask
- A `bool[8]` array → single `u8` bitfield
- Loop accumulating `if condition(i) { total += 1 }` → `popcount(mask)`
- Sequence of `if field_a { ... } if field_b { ... }` → masked operation on packed bitfield

### Approach

This is a pattern-recognition problem at the SIR level:

1. **Scan** the SIR graph for subgraphs with specific structural properties
2. **Classify** each subgraph by the bitwise idiom it could express
3. **Rank** candidates by potential speedup
4. **Synthesize** the actual bitwise equivalent (handed to the rewrite engine)

### Challenges

- The opportunities look like ordinary code — no annotations, no DSL
- Must recognize intent, not just instructions
- Must handle arbitrary compositions of patterns
- False positives are acceptable (synthesis will fail), but false negatives are not

### Research Directions

- Graph neural networks for pattern detection in IR graphs
- E-graph based enumeration of equivalent expressions
- Sketch-based synthesis with bitwise primitives as the sketch language
