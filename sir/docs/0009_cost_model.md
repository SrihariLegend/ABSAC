# 0009 — Cost Model (Future)

## Planned for v0.6+

The cost model predicts wall-clock performance of SIR fragments on real
microarchitectures, preventing "clever" rewrites that run slower than the
original.

### Why Cost Modeling is Hard

Bitwise code is not always faster:

- **Branch prediction**: When branches are highly predictable (e.g., error
  handling), branchless code adds overhead with no benefit.
- **Popcount overhead**: A popcount-based loop has fixed overhead that might
  exceed a short indexed loop.
- **Bitfield packing latency**: Packing and unpacking values into bitfields
  costs instructions that may not be amortized.
- **Port pressure**: Bitwise operations compete for the same execution ports
  as other integer operations.
- **Cache effects**: Reorganizing data into bitfields changes memory layout,
  potentially hurting cache locality.

### Approach

Model two things:

1. **Instruction-level cost** — latency, throughput, port usage for each SIR
   node on target microarchitectures (x86-64, ARM64, RISC-V).

2. **Microarchitectural effects** — branch predictor behavior, cache hierarchy,
   instruction-level parallelism (ILP), register pressure.

### Integration

The cost model is consulted after synthesis produces a candidate rewrite.
If predicted speedup < threshold (e.g., 5%), the rewrite is suppressed.

### Implementation Strategy

Start with a simple model (count nodes, weight by latency) and iterate:

1. **v0.6**: Static instruction count with latency weights
2. **v0.7**: Add branch predictor model
3. **v0.8**: Add cache model
4. **v0.9**: Integrate with LLVM-MCA or IACA for microarchitectural simulation
