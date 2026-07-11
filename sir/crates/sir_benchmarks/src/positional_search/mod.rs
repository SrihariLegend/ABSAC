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
                id: "PS001",
                name: "first_set_bit",
                category: "Positional search",
                input_desc: "find first true in array",
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "Search",
                    concepts: vec!["PositionSearch", "LogicalSequence"],
                    representation: "BitScan",
                    candidate: "BitscanForward",
                    proof: "First(LogicalSequence) == TrailingZeros(Pack(LogicalSequence))",
                    rewrite: "Loop -> TrailingZeros",
                },
            },
            func: || {
                let mut b = Builder::new("array_find_first", &[("arr", Type::Array { element: Box::new(Type::Bool), length: 64 })], Type::Tuple { elements: vec![Type::u64()] });
                let arr = b.parameter_index(0).unwrap();
                let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let sixty_four = b.constant(ConstantData::u64(64), Type::u64(), unknown_span());
                
                let i_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                
                let is_true = b.array_access(arr, i_init, Type::Bool, unknown_span()).unwrap();
                let not_found = b.bool_not(is_true, unknown_span()).unwrap();
                
                let next_i = b.add(i_init, one, unknown_span()).unwrap();
                
                let bounds_check = b.lt(next_i, sixty_four, unknown_span()).unwrap();
                let cond = b.bool_and(not_found, bounds_check, unknown_span()).unwrap();
                
                let loop_node = b.r#loop(
                    &[is_true, not_found, next_i, bounds_check, cond],
                    cond,
                    &[next_i],
                    &[i_init],
                    Type::Tuple { elements: vec![Type::u64()] },
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
    fn test_ps001() {
        for def in benchmarks() {
            ((def.func)(), &def.spec);
        }
    }
}
