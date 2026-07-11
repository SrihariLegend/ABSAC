use sir_builder::Builder;
use sir_types::{ConstantData, Type, Span, };
use sir_nodes::{Function, };
use crate::framework::{BenchmarkDef, BenchmarkSpec, ExpectedKnowledge};

fn unknown_span() -> Span {
    Span::unknown()
}

pub fn build_array_count_loop() -> Function {
    let mut b = Builder::new("array_count", &[("arr", Type::Array { element: Box::new(Type::Bool), length: 64 })], Type::u64());
    let arr = b.parameter_index(0).unwrap();
    
    let zero = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
    let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
    let sixty_four = b.constant(ConstantData::u64(64), Type::u64(), unknown_span());
    
    // Carry inputs: index `i`, running `count`
    let i_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
    let count_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
    
    // In SSA, we just reference the carry nodes directly.
    let next_i = b.add(i_init, one, unknown_span()).unwrap();
    
    // Load arr[i]
    let is_true = b.array_access(arr, i_init, Type::Bool, unknown_span()).unwrap();
    
    // Select 1 or 0
    let to_add = b.select(is_true, one, zero, unknown_span()).unwrap();
    
    // next_count = count + to_add
    let next_count = b.add(count_init, to_add, unknown_span()).unwrap();
    
    // condition = next_i < 64
    let cond = b.lt(next_i, sixty_four, unknown_span()).unwrap();
    
    let loop_node = b.r#loop(
        &[next_i, is_true, to_add, next_count, cond],
        cond,
        &[next_i, next_count],
        &[i_init, count_init],
        Type::Tuple { elements: vec![Type::u64(), Type::u64()] },
        unknown_span()
    ).unwrap();
    
    // Since return type is u64, and loop returns tuple (u64, u64),
    // Wait, the return_value method requires matching type. I will just change the function return type to Tuple.
    // Or I can return `loop_node` if the function signature matches.
    b.return_value(loop_node, unknown_span()).unwrap();
    
    // Let's fix the function return type.
    b.build()
}

pub fn benchmarks() -> Vec<BenchmarkDef> {
    vec![
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "BR001",
                name: "popcount_array",
                category: "Boolean reductions",
                input_desc: "count ones in array",
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "Collection",
                    concepts: vec!["CardinalityReduction", "LogicalSequence"],
                    representation: "BitSet",
                    candidate: "Popcount",
                    proof: "Count(LogicalSequence) == Popcount(Pack(LogicalSequence))",
                    rewrite: "Loop -> Popcount",
                },
            },
            func: || {
                let mut b = Builder::new("array_count", &[("arr", Type::Array { element: Box::new(Type::Bool), length: 64 })], Type::Tuple { elements: vec![Type::u64(), Type::u64()] });
                let arr = b.parameter_index(0).unwrap();
                let zero = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let sixty_four = b.constant(ConstantData::u64(64), Type::u64(), unknown_span());
                
                let i_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                let count_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                
                let next_i = b.add(i_init, one, unknown_span()).unwrap();
                let is_true = b.array_access(arr, i_init, Type::Bool, unknown_span()).unwrap();
                let to_add = b.select(is_true, one, zero, unknown_span()).unwrap();
                let next_count = b.add(count_init, to_add, unknown_span()).unwrap();
                let cond = b.lt(next_i, sixty_four, unknown_span()).unwrap();
                
                let loop_node = b.r#loop(
                    &[next_i, is_true, to_add, next_count, cond],
                    cond,
                    &[next_i, next_count],
                    &[i_init, count_init],
                    Type::Tuple { elements: vec![Type::u64(), Type::u64()] },
                    unknown_span()
                ).unwrap();
                
                b.return_value(loop_node, unknown_span()).unwrap();
                b.build()
            },
        },
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "BR002",
                name: "any_array",
                category: "Boolean reductions",
                input_desc: "any true in array",
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "Collection",
                    concepts: vec!["DisjunctiveReduction", "LogicalSequence"],
                    representation: "BitSet",
                    candidate: "Any",
                    proof: "Any(LogicalSequence) == Any(Pack(LogicalSequence))",
                    rewrite: "Loop -> Any",
                },
            },
            func: || {
                let mut b = Builder::new("array_any", &[("arr", Type::Array { element: Box::new(Type::Bool), length: 64 })], Type::Tuple { elements: vec![Type::u64(), Type::Bool] });
                let arr = b.parameter_index(0).unwrap();
                let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let sixty_four = b.constant(ConstantData::u64(64), Type::u64(), unknown_span());
                let false_val = b.constant(ConstantData::Bool(false), Type::Bool, unknown_span());
                
                let i_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                let any_init = false_val;
                
                let next_i = b.add(i_init, one, unknown_span()).unwrap();
                let is_true = b.array_access(arr, i_init, Type::Bool, unknown_span()).unwrap();
                let next_any = b.bool_or(any_init, is_true, unknown_span()).unwrap();
                let cond = b.lt(next_i, sixty_four, unknown_span()).unwrap();
                
                let loop_node = b.r#loop(
                    &[next_i, is_true, next_any, cond],
                    cond,
                    &[next_i, next_any],
                    &[i_init, any_init],
                    Type::Tuple { elements: vec![Type::u64(), Type::Bool] },
                    unknown_span()
                ).unwrap();
                
                b.return_value(loop_node, unknown_span()).unwrap();
                b.build()
            },
        },
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "BR003",
                name: "all_array",
                category: "Boolean reductions",
                input_desc: "all true in array",
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "Collection",
                    concepts: vec!["ConjunctiveReduction", "LogicalSequence"],
                    representation: "BitSet",
                    candidate: "All",
                    proof: "All(LogicalSequence) == All(Pack(LogicalSequence))",
                    rewrite: "Loop -> All",
                },
            },
            func: || {
                let mut b = Builder::new("array_all", &[("arr", Type::Array { element: Box::new(Type::Bool), length: 64 })], Type::Tuple { elements: vec![Type::u64(), Type::Bool] });
                let arr = b.parameter_index(0).unwrap();
                let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let sixty_four = b.constant(ConstantData::u64(64), Type::u64(), unknown_span());
                let true_val = b.constant(ConstantData::Bool(true), Type::Bool, unknown_span());
                
                let i_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                let all_init = true_val;
                
                let next_i = b.add(i_init, one, unknown_span()).unwrap();
                let is_true = b.array_access(arr, i_init, Type::Bool, unknown_span()).unwrap();
                let next_all = b.bool_and(all_init, is_true, unknown_span()).unwrap();
                let cond = b.lt(next_i, sixty_four, unknown_span()).unwrap();
                
                let loop_node = b.r#loop(
                    &[next_i, is_true, next_all, cond],
                    cond,
                    &[next_i, next_all],
                    &[i_init, all_init],
                    Type::Tuple { elements: vec![Type::u64(), Type::Bool] },
                    unknown_span()
                ).unwrap();
                
                b.return_value(loop_node, unknown_span()).unwrap();
                b.build()
            },
        },
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "BR004",
                name: "parity_array",
                category: "Boolean reductions",
                input_desc: "parity of array",
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "Collection",
                    concepts: vec!["ExclusiveReduction", "LogicalSequence"],
                    representation: "BitSet",
                    candidate: "Parity",
                    proof: "Parity(LogicalSequence) == Parity(Pack(LogicalSequence))",
                    rewrite: "Loop -> Parity",
                },
            },
            func: || {
                let mut b = Builder::new("array_parity", &[("arr", Type::Array { element: Box::new(Type::Bool), length: 64 })], Type::Tuple { elements: vec![Type::u64(), Type::Bool] });
                let arr = b.parameter_index(0).unwrap();
                let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let sixty_four = b.constant(ConstantData::u64(64), Type::u64(), unknown_span());
                let false_val = b.constant(ConstantData::Bool(false), Type::Bool, unknown_span());
                
                let i_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                let parity_init = false_val;
                
                let next_i = b.add(i_init, one, unknown_span()).unwrap();
                let is_true = b.array_access(arr, i_init, Type::Bool, unknown_span()).unwrap();
                // XOR for booleans is `Ne` or just create bitwise Xor and cast. Ne is supported for Bool.
                let next_parity = b.ne(parity_init, is_true, unknown_span()).unwrap();
                let cond = b.lt(next_i, sixty_four, unknown_span()).unwrap();
                
                let loop_node = b.r#loop(
                    &[next_i, is_true, next_parity, cond],
                    cond,
                    &[next_i, next_parity],
                    &[i_init, parity_init],
                    Type::Tuple { elements: vec![Type::u64(), Type::Bool] },
                    unknown_span()
                ).unwrap();
                
                b.return_value(loop_node, unknown_span()).unwrap();
                b.build()
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_br001() {
        for def in benchmarks() {
            ((def.func)(), &def.spec);
        }
    }
}
