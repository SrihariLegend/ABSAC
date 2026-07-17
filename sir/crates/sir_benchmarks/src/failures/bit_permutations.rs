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
                id: "BP001",
                name: "rotate_left",
                category: "Bit permutations",
                input_desc: "(x << n) | (x >> (64 - n))",
                expected: ExpectedKnowledge::MissingKnowledge {
                    concepts: vec!["Rotate"],
                    closure: vec![],
                    representations: vec!["BitPermutations"],
                    rewrites: vec!["rol/ror"],
                },
            },
            func: || {
                let mut b = Builder::new("rotate_naive", &[("x", Type::u64()), ("n", Type::u64())], Type::u64());
                let x = b.parameter_index(0).unwrap();
                let n = b.parameter_index(1).unwrap();
                
                let sixty_four = b.constant(ConstantData::u64(64), Type::u64(), unknown_span());
                
                let left_shift = b.shl(x, n, unknown_span()).unwrap();
                let diff = b.sub(sixty_four, n, unknown_span()).unwrap();
                let right_shift = b.shr(x, diff, unknown_span()).unwrap();
                let or = b.bit_or(left_shift, right_shift, unknown_span()).unwrap();
                
                b.return_value(or, unknown_span()).unwrap();
                b.build()
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bp001() {
        for def in benchmarks() {
            ((def.func)(), &def.spec);
        }
    }
}
