import re

hd_benchmarks = """
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
        
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "HD004",
                name: "byte_swap",
                category: "Hacker's Delight",
                input_desc: "((x & 0xFF) << 8) | ((x >> 8) & 0xFF)",
                expected: ExpectedKnowledge::ExpectedFailure {
                    stage: "Semantics",
                    missing_knowledge: "Missing concept and recognizer for byte level permutations",
                    needed_concept: "ByteSwap",
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
                expected: ExpectedKnowledge::ExpectedFailure {
                    stage: "Semantics",
                    missing_knowledge: "Missing structural recognizer for bit reversal pattern",
                    needed_concept: "BitReversal",
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
"""

content = open("sir/crates/sir_benchmarks/src/hackers_delight/mod.rs").read()
# Insert before the last `]` in the `vec![` block.
parts = content.rsplit("    ]\n}", 1)
new_content = parts[0] + hd_benchmarks + "    ]\n}" + parts[1]

open("sir/crates/sir_benchmarks/src/hackers_delight/mod.rs", "w").write(new_content)
