# 0006 — Equivalence Checking (Future)

## Planned for v0.3+

Equivalence checking verifies that a transformed (bitwise-optimized) SIR graph
is semantically identical to the original.

### Approach

Translation validation: rather than proving the transformation correct in
general, verify that the specific output is equivalent to the specific input.

### Techniques Under Consideration

1. **SMT-based equivalence** (Z3, CVC5)
   - Encode both graphs as SMT formulas
   - Query: ∃ inputs where outputs differ?
   - UNSAT = equivalent

2. **Symbolic execution**
   - Execute both graphs symbolically
   - Compare symbolic outputs
   - Handles loops via bounded unrolling + induction

3. **Relational verification**
   - Prove a relational property between old and new programs
   - Stronger guarantee, more automation overhead

### Scalability

For functions with hundreds of operations and billions of input states,
SMT-based approaches need careful encoding:
- Bit-blast everything to SAT (for small functions)
- Use bit-vector theory (for larger ones)
- Bounded model checking for loops

### Integration with SIR

The verifier will operate on `(old_function, new_function)` pairs and produce
either a proof (UNSAT core or inductive invariant) or a counterexample.
