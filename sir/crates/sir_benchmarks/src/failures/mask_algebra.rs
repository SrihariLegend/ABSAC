use sir_builder::Builder;
use sir_types::{ConstantData, Type, Span};
use crate::framework::{BenchmarkDef, BenchmarkSpec, ExpectedKnowledge, };

fn unknown_span() -> Span {
    Span::unknown()
}

pub fn benchmarks() -> Vec<BenchmarkDef> {
    vec![
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "MA002",
                name: "clear_lowest_set_bit",
                category: "Mask algebra",
                input_desc: "x & (x - 1)",
                expected: ExpectedKnowledge::ExpectedFailure {
                    stage: "Semantics",
                    missing_knowledge: "MaskAlgebra",
                    needed_concept: "LowestSetBit",
                },
            },
            func: || {
                let mut b = Builder::new("clear_lowest_set_bit_naive", &[("x", Type::u32())], Type::u32());
                let x = b.parameter_index(0).unwrap();
                let one = b.constant(ConstantData::u32(1), Type::u32(), unknown_span());
                let x_minus_one = b.sub(x, one, unknown_span()).unwrap();
                let clear_lowest = b.bit_and(x, x_minus_one, unknown_span()).unwrap();
                
                b.return_value(clear_lowest, unknown_span()).unwrap();
                b.build()
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mask_algebra_failures() {
        for def in benchmarks() {
            ((def.func)(), &def.spec);
        }
    }
}
