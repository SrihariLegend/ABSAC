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
                id: "NO001",
                name: "modulo_non_power_of_two",
                category: "Non-optimizable",
                input_desc: "x % 3",
                expected: ExpectedKnowledge::NonOptimizable {
                    reason: "3 is not a power of two; cannot be reduced to a bitwise AND",
                },
            },
            func: || {
                let mut b = Builder::new("modulo_non_pow2", &[("x", Type::u32())], Type::u32());
                let x = b.parameter_index(0).unwrap();
                let three = b.constant(ConstantData::u32(3), Type::u32(), unknown_span());
                let rem = b.rem(x, three, unknown_span()).unwrap();
                b.return_value(rem, unknown_span()).unwrap();
                b.build()
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_non_optimizable() {
        for def in benchmarks() {
            ((def.func)(), &def.spec);
        }
    }
}
