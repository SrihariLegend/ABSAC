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
                expected: ExpectedKnowledge::NonOptimizable {
                    reason: "BitScanForward definition is Stub-quarantined: obligation does not bind actual source/candidate operands",
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
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "PS002",
                name: "last_set_bit",
                category: "Positional search",
                input_desc: "find last true in array",
                // PS002 ALSO matches an Any-style reduction over the
                // same loop — that AnyDefinition candidate is
                // SchemaChecked and its rewrite is independent of the
                // quarantined BitscanReverse path, so it still rewrites.
                expected: ExpectedKnowledge::Optimizes {
                    semantic_domain: "Search",
                    concepts: vec!["LastOccurrence", "LogicalSequence"],
                    representation: "BitScan",
                    candidate: "BitscanReverse",
                    proof: "Last(LogicalSequence) == LeadingZeros(Pack(LogicalSequence))",
                    rewrite: "Loop -> LeadingZeros",
                },
            },
            func: || {
                let mut b = Builder::new("array_find_last", &[("arr", Type::Array { element: Box::new(Type::Bool), length: 64 })], Type::u64());
                let arr = b.parameter_index(0).unwrap();
                let i_step = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let zero = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                let found_init = b.constant(ConstantData::boolean(false), Type::Bool, unknown_span());
                // Compute the starting index and sentinel as SSA nodes (not
                // constants): the position-search recognizer distinguishes a
                // position search from a cardinality reduction by the select not
                // choosing between constants.
                let sixty_four = b.constant(ConstantData::u64(64), Type::u64(), unknown_span());
                let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let sixty_three = b.constant(ConstantData::u64(63), Type::u64(), unknown_span());
                let i_init = b.sub(sixty_four, one, unknown_span()).unwrap();
                let index_init = b.add(sixty_three, one, unknown_span()).unwrap();
                let is_true = b.array_access(arr, i_init, Type::Bool, unknown_span()).unwrap();
                let new_found = b.bool_or(found_init, is_true, unknown_span()).unwrap();
                let not_found_yet = b.bool_not(found_init, unknown_span()).unwrap();
                let is_last = b.bool_and(is_true, not_found_yet, unknown_span()).unwrap();
                let new_index = b.select(is_last, i_init, index_init, unknown_span()).unwrap();
                let next_i = b.sub(i_init, i_step, unknown_span()).unwrap();

                let not_found = b.bool_not(found_init, unknown_span()).unwrap();
                let in_bounds = b.ge(i_init, zero, unknown_span()).unwrap();
                let cond = b.bool_and(not_found, in_bounds, unknown_span()).unwrap();

                let loop_node = b.r#loop(
                    &[is_true, new_found, not_found_yet, is_last, new_index, next_i, not_found, in_bounds, cond],
                    cond,
                    &[new_found, new_index, next_i],
                    &[found_init, index_init, i_init],
                    Type::Tuple { elements: vec![Type::Bool, Type::u64(), Type::u64()] },
                    unknown_span()
                ).unwrap();

                let res = b.field_access(loop_node, "1", Type::u64(), unknown_span()).unwrap();
                b.return_value(res, unknown_span()).unwrap();
                b.build()
            },
        },
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "PS003",
                name: "trailing_zero_count",
                category: "Positional search",
                input_desc: "count trailing zeros of a scalar",
                expected: ExpectedKnowledge::NonOptimizable {
                    reason: "TrailingZeroCount definition is Stub-quarantined: obligation is a tautology (LeadingZeros(v)==LeadingZeros(v))",
                },
            },
            func: || {
                let mut b = Builder::new("trailing_zero_count", &[("value", Type::u64())], Type::u64());
                let value = b.parameter_index(0).unwrap();
                let n_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                let x_init = value;
                let n_step = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let x_step = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let zero = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());

                let bit = b.bit_and(x_init, one, unknown_span()).unwrap();
                let cond = b.eq(bit, zero, unknown_span()).unwrap();
                let x_next = b.shr(x_init, x_step, unknown_span()).unwrap();
                let n_next = b.add(n_init, n_step, unknown_span()).unwrap();

                let loop_node = b.r#loop(
                    &[bit, cond, x_next, n_next],
                    cond,
                    &[x_next, n_next],
                    &[x_init, n_init],
                    Type::Tuple { elements: vec![Type::u64(), Type::u64()] },
                    unknown_span()
                ).unwrap();

                let res = b.field_access(loop_node, "1", Type::u64(), unknown_span()).unwrap();
                b.return_value(res, unknown_span()).unwrap();
                b.build()
            },
        },
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "PS004",
                name: "leading_zero_count",
                category: "Positional search",
                input_desc: "count leading zeros of a scalar",
                expected: ExpectedKnowledge::NonOptimizable {
                    reason: "LeadingZeroCount definition is Stub-quarantined: obligation is a tautology",
                },
            },
            func: || {
                let mut b = Builder::new("leading_zero_count", &[("value", Type::u64())], Type::u64());
                let value = b.parameter_index(0).unwrap();
                let n_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                let mask_init = b.constant(ConstantData::u64(1 << 63), Type::u64(), unknown_span());
                let n_step = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let mask_step = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let zero = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());

                let bit = b.bit_and(value, mask_init, unknown_span()).unwrap();
                let cond = b.eq(bit, zero, unknown_span()).unwrap();
                let mask_next = b.shr(mask_init, mask_step, unknown_span()).unwrap();
                let n_next = b.add(n_init, n_step, unknown_span()).unwrap();

                let loop_node = b.r#loop(
                    &[bit, cond, mask_next, n_next],
                    cond,
                    &[mask_next, n_next],
                    &[mask_init, n_init],
                    Type::Tuple { elements: vec![Type::u64(), Type::u64()] },
                    unknown_span()
                ).unwrap();

                let res = b.field_access(loop_node, "1", Type::u64(), unknown_span()).unwrap();
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
    fn test_ps001() {
        for def in benchmarks() {
            run_benchmark((def.func)(), &def.spec);
        }
    }
}

