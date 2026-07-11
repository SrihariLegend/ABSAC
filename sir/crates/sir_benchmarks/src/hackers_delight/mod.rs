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
                id: "AR001",
                name: "modulo_power_of_two",
                category: "Arithmetic identities",
                input_desc: "x % 8",
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "Arithmetic",
                    concepts: vec!["ModuloPowerOfTwo"],
                    representation: "BitwiseArithmetic",
                    candidate: "BitwiseAnd",
                    proof: "Modulo(x, 2^k) == And(x, 2^k - 1)",
                    rewrite: "Rem -> And",
                },
            },
            func: || {
                let mut b = Builder::new("modulo_naive", &[("x", Type::u32())], Type::u32());
                let x = b.parameter_index(0).unwrap();
                let eight = b.constant(ConstantData::u32(8), Type::u32(), unknown_span());
                let rem = b.rem(x, eight, unknown_span()).unwrap();
                b.return_value(rem, unknown_span()).unwrap();
                b.build()
            },
        },
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "AR002",
                name: "divide_power_of_two",
                category: "Arithmetic identities",
                input_desc: "x / 16",
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "Arithmetic",
                    concepts: vec!["DividePowerOfTwo"],
                    representation: "BitwiseArithmetic",
                    candidate: "ShiftRight",
                    proof: "Div(x, 2^k) == Shr(x, k)",
                    rewrite: "Div -> Shr",
                },
            },
            func: || {
                let mut b = Builder::new("divide_naive", &[("x", Type::u32())], Type::u32());
                let x = b.parameter_index(0).unwrap();
                let sixteen = b.constant(ConstantData::u32(16), Type::u32(), unknown_span());
                let div = b.div(x, sixteen, unknown_span()).unwrap();
                b.return_value(div, unknown_span()).unwrap();
                b.build()
            },
        },
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "AR003",
                name: "multiply_power_of_two",
                category: "Arithmetic identities",
                input_desc: "x * 32",
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "Arithmetic",
                    concepts: vec!["MultiplyPowerOfTwo"],
                    representation: "BitwiseArithmetic",
                    candidate: "ShiftLeft",
                    proof: "Mul(x, 2^k) == Shl(x, k)",
                    rewrite: "Mul -> Shl",
                },
            },
            func: || {
                let mut b = Builder::new("multiply_naive", &[("x", Type::u32())], Type::u32());
                let x = b.parameter_index(0).unwrap();
                let thirty_two = b.constant(ConstantData::u32(32), Type::u32(), unknown_span());
                let mul = b.mul(x, thirty_two, unknown_span()).unwrap();
                b.return_value(mul, unknown_span()).unwrap();
                b.build()
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_popcount() {
        let def = benchmarks().into_iter().find(|b| b.spec.id == "BR001").unwrap();
        ((def.func)(), &def.spec);
    }
}
