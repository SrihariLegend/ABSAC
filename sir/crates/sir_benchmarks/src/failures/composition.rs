use sir_builder::Builder;
use sir_types::{ConstantData, Type, Span};
use crate::framework::{BenchmarkDef, BenchmarkSpec, ExpectedKnowledge};

fn unknown_span() -> Span {
    Span::unknown()
}

pub fn benchmarks() -> Vec<BenchmarkDef> {
    vec![
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "COMP001",
                name: "count_clear_lowest_set_bit_is_zero",
                category: "Semantic Composition",
                input_desc: "count += ((x & (x - 1)) == 0)",
                expected: ExpectedKnowledge::ExpectedFailure {
                    stage: "Rewrite",
                    missing_knowledge: "Missing implementation anchor / provenance for rewriting",
                    needed_concept: "MaskOperation", 
                },
            },
            func: || {
                let mut b = Builder::new("count_powers_of_two", &[("arr", Type::Array { element: Box::new(Type::u64()), length: 64 })], Type::Tuple { elements: vec![Type::u64(), Type::u64()] });
                let arr = b.parameter_index(0).unwrap();
                
                let zero = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let sixty_four = b.constant(ConstantData::u64(64), Type::u64(), unknown_span());
                
                // Carry inputs: index `i`, running `count`
                let i_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                let count_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                
                // next_i = i + 1
                let next_i = b.add(i_init, one, unknown_span()).unwrap();
                
                // Load arr[i]
                let x = b.array_access(arr, i_init, Type::u64(), unknown_span()).unwrap();
                
                // x & (x - 1)
                let x_minus_one = b.sub(x, one, unknown_span()).unwrap();
                let x_and_x_minus_one = b.bit_and(x, x_minus_one, unknown_span()).unwrap();
                
                // == 0
                let is_zero = b.eq(x_and_x_minus_one, zero, unknown_span()).unwrap();
                
                // Select 1 or 0
                let to_add = b.select(is_zero, one, zero, unknown_span()).unwrap();
                
                // next_count = count + to_add
                let next_count = b.add(count_init, to_add, unknown_span()).unwrap();
                
                // condition = next_i < 64
                let cond = b.lt(next_i, sixty_four, unknown_span()).unwrap();
                
                let loop_node = b.r#loop(
                    &[next_i, x, x_minus_one, x_and_x_minus_one, is_zero, to_add, next_count, cond],
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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::run_benchmark;
    
    #[test]
    fn test_composition_failures() {
        for def in benchmarks() {
            run_benchmark((def.func)(), &def.spec);
        }
    }
}
