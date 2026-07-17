import re

content = """use sir_builder::Builder;
use sir_types::{ConstantData, Type, Span};
use crate::framework::{BenchmarkDef, BenchmarkSpec, ExpectedKnowledge};

fn unknown_span() -> Span {
    Span::unknown()
}

pub fn benchmarks() -> Vec<BenchmarkDef> {
    vec![
        // ── Currently Supported (Basic Arithmetic Identities) ──
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
        // ... I will skip AR002 and AR003 for brevity in this roadmap setup ...
        
        // ── Hacker's Delight Roadmap (Failures mapping missing knowledge) ──

        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "HD001",
                name: "isolate_lowest_set_bit",
                category: "Hacker's Delight",
                input_desc: "x & -x",
                expected: ExpectedKnowledge::ExpectedFailure {
                    stage: "Semantics",
                    missing_knowledge: "Missing recognizer for isolating lowest bit",
                    needed_concept: "IsolateLowestSetBit",
                },
            },
            func: || {
                let mut b = Builder::new("isolate_lowest_bit", &[("x", Type::u64())], Type::u64());
                let x = b.parameter_index(0).unwrap();
                let neg_x = b.neg(x, unknown_span()).unwrap();
                let res = b.bit_and(x, neg_x, unknown_span()).unwrap();
                b.return_value(res, unknown_span()).unwrap();
                b.build()
            },
        },

        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "HD002",
                name: "brian_kernighan_popcount",
                category: "Hacker's Delight",
                input_desc: "while x != 0 { count++; x &= x - 1; }",
                expected: ExpectedKnowledge::ExpectedFailure {
                    stage: "Semantics",
                    missing_knowledge: "Needs to recognize integer mutation via ClearLowestSetBit as set iteration",
                    needed_concept: "BitsetIteration",
                },
            },
            func: || {
                let mut b = Builder::new("bk_popcount", &[("x", Type::u64())], Type::u64());
                let x_init = b.parameter_index(0).unwrap();
                let zero = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let count_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                
                // x &= x - 1
                let x_minus_1 = b.sub(x_init, one, unknown_span()).unwrap();
                let next_x = b.bit_and(x_init, x_minus_1, unknown_span()).unwrap();
                
                // count++
                let next_count = b.add(count_init, one, unknown_span()).unwrap();
                
                // cond: next_x != 0
                let cond = b.ne(next_x, zero, unknown_span()).unwrap();
                
                let loop_node = b.r#loop(
                    &[next_x, next_count, cond],
                    cond,
                    &[next_x, next_count],
                    &[x_init, count_init],
                    Type::Tuple { elements: vec![Type::u64(), Type::u64()] },
                    unknown_span()
                ).unwrap();
                
                b.return_value(loop_node, unknown_span()).unwrap();
                b.build()
            },
        },

        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "HD003",
                name: "rotate_left",
                category: "Hacker's Delight",
                input_desc: "(x << k) | (x >> (64 - k))",
                expected: ExpectedKnowledge::ExpectedFailure {
                    stage: "Semantics",
                    missing_knowledge: "Missing concept and recognizer for circular shifts",
                    needed_concept: "CircularShift",
                },
            },
            func: || {
                let mut b = Builder::new("rotate_left", &[("x", Type::u64()), ("k", Type::u64())], Type::u64());
                let x = b.parameter_index(0).unwrap();
                let k = b.parameter_index(1).unwrap();
                let sixty_four = b.constant(ConstantData::u64(64), Type::u64(), unknown_span());
                
                let shl = b.shl(x, k, unknown_span()).unwrap();
                let diff = b.sub(sixty_four, k, unknown_span()).unwrap();
                let shr = b.shr(x, diff, unknown_span()).unwrap();
                let res = b.bit_or(shl, shr, unknown_span()).unwrap();
                
                b.return_value(res, unknown_span()).unwrap();
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
    fn test_hackers_delight() {
        for def in benchmarks() {
            run_benchmark((def.func)(), &def.spec);
        }
    }
}
"""
open("sir/crates/sir_benchmarks/src/hackers_delight/mod.rs", "w").write(content)
