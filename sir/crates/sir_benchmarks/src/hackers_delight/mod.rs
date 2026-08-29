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

        // ── Hacker's Delight Roadmap (Failures mapping missing knowledge) ──

        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "HD001",
                name: "isolate_lowest_set_bit",
                category: "Hacker's Delight",
                input_desc: "x & -x",
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "MaskAlgebra",
                    concepts: vec!["LowestSetBit"],
                    representation: "MaskAlgebra",
                    candidate: "IsolateLowestBit",
                    proof: "LowestSetBit(x) == And(x, Neg(x))",
                    rewrite: "And -> Intrinsic(blsi)",
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
                id: "HD007",
                name: "isolate_lowest_clear_bit",
                category: "Hacker's Delight",
                input_desc: "~x & (x + 1)",
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "MaskAlgebra",
                    concepts: vec!["LowestClearBitMask"],
                    representation: "MaskAlgebra",
                    candidate: "IsolateLowestClearBit",
                    proof: "LowestClearBitMask(x) == And(Not(x), Add(x, 1))",
                    rewrite: "And -> Intrinsic(blsi(Not(x)))",
                },
            },
            func: || {
                let mut b = Builder::new("isolate_lowest_clear_bit", &[("x", Type::u64())], Type::u64());
                let x = b.parameter_index(0).unwrap();
                let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let not_x = b.bit_not(x, unknown_span()).unwrap();
                let x_plus_one = b.add(x, one, unknown_span()).unwrap();
                let res = b.bit_and(not_x, x_plus_one, unknown_span()).unwrap();
                b.return_value(res, unknown_span()).unwrap();
                b.build()
            },
        },
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "HD008",
                name: "set_lowest_clear_bit",
                category: "Hacker's Delight",
                input_desc: "x | (x + 1)",
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "MaskAlgebra",
                    concepts: vec!["SetLowestClearBit"],
                    representation: "MaskAlgebra",
                    candidate: "SetLowestClearBit",
                    proof: "SetLowestClearBit(x) == Or(x, Add(x, 1))",
                    rewrite: "Or -> Or(x, Intrinsic(blsmsk(Not(x))))",
                },
            },
            func: || {
                let mut b = Builder::new("set_lowest_clear_bit", &[("x", Type::u64())], Type::u64());
                let x = b.parameter_index(0).unwrap();
                let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let x_plus_one = b.add(x, one, unknown_span()).unwrap();
                let res = b.bit_or(x, x_plus_one, unknown_span()).unwrap();
                b.return_value(res, unknown_span()).unwrap();
                b.build()
            },
        },
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "HD012",
                name: "brian_kernighan_parity",
                category: "Hacker's Delight",
                input_desc: "while x != 0 { parity ^= 1; x &= x - 1; } return parity != 0",
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "Collection",
                    concepts: vec!["BitsetIteration", "Parity"],
                    representation: "BitSet",
                    candidate: "Parity",
                    proof: "Parity(x) == BitwiseAndOne(Popcount(x))",
                    rewrite: "Loop -> Popcount & 1",
                },
            },
            func: || {
                let mut b = Builder::new("bk_parity", &[("x", Type::u64())], Type::Bool);
                let x_init = b.parameter_index(0).unwrap();
                let zero = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let parity_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());

                // x &= x - 1
                let x_minus_1 = b.sub(x_init, one, unknown_span()).unwrap();
                let next_x = b.bit_and(x_init, x_minus_1, unknown_span()).unwrap();
                // parity ^= 1
                let next_parity = b.bit_xor(parity_init, one, unknown_span()).unwrap();
                // cond: next_x != 0
                let cond = b.ne(next_x, zero, unknown_span()).unwrap();

                let loop_node = b.r#loop(
                    &[next_x, next_parity, cond],
                    cond,
                    &[next_x, next_parity],
                    &[x_init, parity_init],
                    Type::Tuple { elements: vec![Type::u64(), Type::u64()] },
                    unknown_span(),
                ).unwrap();

                let extracted = b.tuple_extract(loop_node, 1, Type::u64(), unknown_span()).unwrap();
                let parity_bool = b.ne(extracted, zero, unknown_span()).unwrap();
                b.return_value(parity_bool, unknown_span()).unwrap();
                b.build()
            },
        },
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "HD002",
                name: "brian_kernighan_popcount",
                category: "Hacker's Delight",
                input_desc: "while x != 0 { count++; x &= x - 1; }",
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "Collection",
                    concepts: vec!["BitsetIteration", "LoopUntilZero"],
                    representation: "BitSet",
                    candidate: "Popcount",
                    proof: "Valid rewrite",
                    rewrite: "Loop -> Popcount",
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
                
                let extracted_count = b.tuple_extract(loop_node, 1, Type::u64(), unknown_span()).unwrap();
                b.return_value(extracted_count, unknown_span()).unwrap();
                b.build()
            },
        },

        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "HD003",
                name: "rotate_left",
                category: "Hacker's Delight",
                input_desc: "(x << k) | (x >> (64 - k))",
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "BitPermutation",
                    concepts: vec!["CircularPermutation"],
                    representation: "BitPermutation",
                    candidate: "RotateLeft",
                    proof: "RotateLeft(x, k) == Or(Shl(x, k), Shr(x, Sub(64, k)))",
                    rewrite: "Or -> Rol",
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
        
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "HD004",
                name: "byte_swap",
                category: "Hacker's Delight",
                input_desc: "((x & 0xFF) << 8) | ((x >> 8) & 0xFF)",
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "BitPermutation",
                    concepts: vec!["BytePermutation"],
                    representation: "BitPermutation",
                    candidate: "ByteSwap",
                    proof: "ByteSwap(x) == Or(Shl(And(x, 0xFF), 8), And(Shr(x, 8), 0xFF))",
                    rewrite: "Or -> Intrinsic(bswap) >> 16",
                },
            },
            func: || {
                let mut b = Builder::new("byte_swap_16", &[("x", Type::u32())], Type::u32());
                let x = b.parameter_index(0).unwrap();
                let mask = b.constant(ConstantData::u32(0xFF), Type::u32(), unknown_span());
                let eight = b.constant(ConstantData::u32(8), Type::u32(), unknown_span());
                
                let low = b.bit_and(x, mask, unknown_span()).unwrap();
                let low_shifted = b.shl(low, eight, unknown_span()).unwrap();
                
                let high = b.shr(x, eight, unknown_span()).unwrap();
                let high_masked = b.bit_and(high, mask, unknown_span()).unwrap();
                
                let res = b.bit_or(low_shifted, high_masked, unknown_span()).unwrap();
                
                b.return_value(res, unknown_span()).unwrap();
                b.build()
            },
        },
        
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "HD005",
                name: "reverse_bits",
                category: "Hacker's Delight",
                input_desc: "swap adjacent bits, then pairs, then nibbles...",
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "BitPermutation",
                    concepts: vec!["BitPermutation"],
                    representation: "BitPermutation",
                    candidate: "ReverseBits",
                    proof: "BitReverse(x) == S3(S2(S1(x)))",
                    rewrite: "Or -> Intrinsic(rbit) >> 24",
                },
            },
            func: || {
                let mut b = Builder::new("reverse_bits_8", &[("x", Type::u32())], Type::u32());
                let x = b.parameter_index(0).unwrap();
                
                // This is a simplified 8-bit version
                let m1 = b.constant(ConstantData::u32(0x55), Type::u32(), unknown_span());
                let m2 = b.constant(ConstantData::u32(0x33), Type::u32(), unknown_span());
                let m3 = b.constant(ConstantData::u32(0x0F), Type::u32(), unknown_span());
                
                let one = b.constant(ConstantData::u32(1), Type::u32(), unknown_span());
                let two = b.constant(ConstantData::u32(2), Type::u32(), unknown_span());
                let four = b.constant(ConstantData::u32(4), Type::u32(), unknown_span());
                
                // Swap adjacent bits
                let shr1 = b.shr(x, one, unknown_span()).unwrap();
                let and1_1 = b.bit_and(shr1, m1, unknown_span()).unwrap();
                let and1_2 = b.bit_and(x, m1, unknown_span()).unwrap();
                let shl1 = b.shl(and1_2, one, unknown_span()).unwrap();
                let x1 = b.bit_or(and1_1, shl1, unknown_span()).unwrap();
                
                // Swap pairs
                let shr2 = b.shr(x1, two, unknown_span()).unwrap();
                let and2_1 = b.bit_and(shr2, m2, unknown_span()).unwrap();
                let and2_2 = b.bit_and(x1, m2, unknown_span()).unwrap();
                let shl2 = b.shl(and2_2, two, unknown_span()).unwrap();
                let x2 = b.bit_or(and2_1, shl2, unknown_span()).unwrap();
                
                // Swap nibbles
                let shr3 = b.shr(x2, four, unknown_span()).unwrap();
                let and3_1 = b.bit_and(shr3, m3, unknown_span()).unwrap();
                let and3_2 = b.bit_and(x2, m3, unknown_span()).unwrap();
                let shl3 = b.shl(and3_2, four, unknown_span()).unwrap();
                let res = b.bit_or(and3_1, shl3, unknown_span()).unwrap();
                
                b.return_value(res, unknown_span()).unwrap();
                b.build()
            },
        },
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "BP001",
                name: "rotate_left_naive",
                category: "Hacker's Delight",
                input_desc: "(x << n) | (x >> (64 - n))",
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "BitPermutation",
                    concepts: vec!["CircularPermutation"],
                    representation: "BitPermutation",
                    candidate: "RotateLeft",
                    proof: "RotateLeft(x, n) == Or(Shl(x, n), Shr(x, Sub(64, n)))",
                    rewrite: "Or -> Rol",
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
    use crate::framework::run_benchmark;
    
    #[test]
    fn test_arithmetic() {
        for def in benchmarks() {
            run_benchmark((def.func)(), &def.spec);
        }
    }
}
