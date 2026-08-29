# ABSAC

**A Semantic Knowledge-Acquisition Compiler for Bitwise Superoptimization**

> *"How much of the known universe of bitwise optimization can be automatically rediscovered from naive programs?"*

ABSAC is an experimental semantic compiler architecture. Instead of hardcoding peephole patterns or using expensive stochastic search over assembly instructions, ABSAC discovers mathematical domains (like Finite Sets or Logical Sequences) embedded in arbitrary functional source code. It systematically proves the equivalence of hardware-efficient representations (like `BitSet`) to replace naive algorithms with optimal bitwise logic.

The compiler acts as a **knowledge acquisition system**. It doesn't use syntactic pattern matching; it relies entirely on a pipeline of semantic deduction, representation inference, candidate generation, and formal verification.

## The Knowledge Pipeline

ABSAC relies on a pure, feed-forward knowledge pipeline operating over **SIR** (Semantic IR), a typed, functional SSA-form graph.

```text
Source Program
      │
      ▼
Compiler Facts           "What is provably true?" (Purity, Finiteness, Loops, Ranges)
      │
      ▼
Semantic Truths          "What mathematical object / domain does this represent?"
      │
      ▼
Representation Beliefs   "Which hardware representation best models this object?"
      │
      ▼
Candidate Plans          "What implementations are possible?"
      │
      ▼
Equivalence Proofs       "Is the chosen rewrite mathematically exact?"
      │
      ▼
Verified Mutations       "Execute the rewrite."
```

Each layer strictly consumes only the knowledge produced by the immediately preceding layer.

## Corpus v1.0: Empirical Evaluation

ABSAC's progress is measured not by features, but by the boundaries of its mathematical knowledge. We maintain an immutable, reproducible benchmark corpus (**Corpus v1.0**) tracking exactly what the compiler can synthesize from first principles. 

Every benchmark acts as a declarative specification of the expected reasoning chain. Expected failures are treated as first-class artifacts mapping the exact domain of missing knowledge (e.g., *Mask Algebra* or *Bit Permutations*).

You can run the live coverage report at any time:

```bash
cd sir
cargo run -p sir_benchmarks --bin report
```

### Current Semantic Coverage (Corpus v1.0)

```text
Ontology Coverage (Architectural Metrics)
=========================================
Concepts implemented:     12
Concepts exercised:       8
Closure rules:            4
Average reasoning depth:  3.2
Maximum reasoning depth:  5

ABSAC Benchmark Status
======================

Benchmarks:             23

Optimized:              11
Expected failures:       5
Correctly declined:      1

Semantic Compression

  Total Initial IR nodes:   101
  Total Semantic truths:    31
  Total Final IR nodes:     93
  Compression ratio:        1.09x

Semantic domains

  Boolean reductions        ✓
  Arithmetic identities     ✓
  Positional search         ✓
  Set algebra               Partial
  Mask algebra              ✓
  Bit permutations          Missing

Representations

  BitSet                    ✓
  BitwiseArithmetic         ✓
  BitScan                   ✓
  MaskAlgebra               ✓
```

As the compiler acquires new mathematical concepts, benchmarks graduate from "Expected Failure" to "Optimized."

## Project Architecture

The compiler framework is implemented purely in Rust (`sir/` workspace). It is architecturally frozen at the IR and reasoning substrate levels to guarantee stability.

- **`sir_types` / `sir_nodes`**: Core data models and SSA-form IR.
- **`sir_analysis`**: Read-only framework (Dataflow, Ranges, Purity, Dominance).
- **`sir_semantics`**: Structural and semantic concept recognizers.
- **`sir_inference`**: Cost and representation mapping.
- **`sir_generation`**: Combinatorial implementation strategies.
- **`sir_verification`**: Exhaustive and symbolic equivalence provers.
- **`sir_rewrite`**: Deterministic, verified subgraph replacement.
- **`sir_optimizer`**: Global fixed-point orchestrator executing beam search over valid rewrite paths to maximize Semantic Compression.

## Building and Testing

There are no external dependencies beyond a standard Rust toolchain.

```bash
cd sir
cargo build
cargo test
cargo run -p sir_benchmarks --bin report
```
