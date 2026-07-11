use sir_builder::Builder;
use sir_types::{ConstantData, Type, Span};

fn unknown_span() -> Span {
    Span::unknown()
}

// Naive popcount using a loop:
// count = 0
// temp = x
// for _ in 0..32 {
//     if temp & 1 != 0 { count += 1 }
//     temp >>= 1
// }
pub fn build_popcount_naive() -> sir_nodes::Function {
    let mut b = Builder::new("popcount_naive", &[("x", Type::u32())], Type::u32());
    let x = b.parameter_index(0).unwrap();
    
    // Initial loop variables
    let zero = b.constant(ConstantData::u32(0), Type::u32(), unknown_span());
    let count_init = zero;
    let temp_init = x;
    let i_init = zero;

    // We build the loop using sir_builder. Wait, loop requires specific structure.
    // The user mentioned loop has `carried_inputs` and `outputs`.
    // Let's create dummy nodes since loop body requires creating the loop first.
    // sir_builder has `loop_builder()` or similar?
    // Let's check sir_builder API for loops.
    
    // Actually, maybe we can just create another file to inspect `Builder` methods for `loop`.
    b.build()
}
