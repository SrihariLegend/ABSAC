# ABSAC Benchmark Corpus v1.0

This directory contains the frozen, immutable empirical evaluation corpus for ABSAC.

The goal of this corpus is not just to test if the compiler executes, but to act as a **declarative specification** of the semantic knowledge the compiler has acquired. It systematically measures exactly which classes of bitwise superoptimizations can be automatically rediscovered from naive programs.

## The Benchmark as a Specification

Every benchmark is a formal specification of knowledge. 

Instead of just checking if output code is faster, ABSAC verifies its own reasoning chain. For example:

```yaml
Benchmark:
  modulo_power_of_two
Category:
  Arithmetic identities
Input:
  x % 8

Specification:
  Expected: Optimizes
  SemanticDomain: Arithmetic
  Concepts: ["ModuloPowerOfTwo"]
  Representation: BitwiseArithmetic
  Candidate: BitwiseAnd
  Proof: Modulo(x, 2^k) == And(x, 2^k - 1)
  Rewrite: Rem -> And
```

## Immutable Research Artifacts

As part of **Corpus v1.0**, these benchmarks are frozen. 
- Benchmark IDs (e.g., `BR001`, `MA002`) **never change**.
- The naive input semantics **never change**.
- The expected logical outcomes **never change**.

The only permitted evolution is updating a benchmark's specification from `ExpectedFailure` to `Optimizes` as the compiler's knowledge graph expands. 

Failures are treated as first-class research artifacts. When the compiler declines to optimize, or fails to recognize a concept, the benchmark formally asserts exactly which domain of knowledge is missing (e.g., `MissingKnowledge: MaskAlgebra`).

## Live Coverage Report

To view the live knowledge boundaries of the compiler against Corpus v1.0, run:

```bash
cd sir
cargo run -p sir_benchmarks --bin report
```

This generates the **ABSAC Semantic Coverage Report**, detailing exactly which mathematical domains are currently understood.
