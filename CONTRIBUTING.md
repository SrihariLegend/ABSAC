# Contributing to ABSAC

Welcome to ABSAC. This is a research project, not a production compiler. Our goal is to explore the limits of automatic semantic knowledge acquisition, not to ship the fastest binaries at any cost.

If you are contributing to this repository, please internalize the following core principles.

## Principle 1: ABSAC is allowed to say "I don't know."

Most optimizers are rewarded simply for transforming code. **ABSAC is rewarded for only transforming code it understands.** 

We do not want heuristic guesses or brittle syntactic pattern-matching. We want a rigorous chain of reasoning.

### The Semantic Chain Invariant

Every optimization performed by ABSAC **must** correspond to an explicit chain of semantic understanding:

```text
Program
   ↓
Semantic Concepts
   ↓
Mathematical Domain
   ↓
Representation Hypothesis
   ↓
Transformation Candidate
   ↓
Proof
   ↓
Rewrite
```

If any link in this chain is missing, the optimization **should not happen**. Declining to optimize code because it lacks a formal representation or proof is a success, not a failure.

## Principle 2: The Benchmark Corpus is a Ledger of Knowledge

We treat our benchmarks (`Corpus v1.x`) as a living history of the compiler's understanding. 

- **Do not delete expected failures.** If a benchmark fails to optimize because we lack the semantic ontology for it (e.g., Mask Algebra), that failure is a first-class research artifact. When it eventually turns green, it tells the story of acquired knowledge.
- **Do not optimize for benchmark count.** Going from 30 benchmarks to 300 redundant benchmarks does not improve the research. Every benchmark should exist because it asks a genuinely new question or probes a specific boundary of the compiler's ontology.
- **Benchmarks are immutable specifications.** A benchmark's ID and naive semantic intent do not change. We only change its expected outcome from `ExpectedFailure` to `Optimizes`.

## Principle 3: No Pipeline Hacks

If adding a new mathematical domain requires breaking the fixed-point optimizer, circumventing the formal proof layer, or writing an ad-hoc pass, the abstraction is wrong. Growth must happen by expanding the knowledge base (new facts, concepts, or beliefs), not by breaking the machine.
