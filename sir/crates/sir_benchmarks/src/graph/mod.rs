use sir_builder::Builder;
use sir_types::{ConstantData, Type, Span};
use crate::framework::{BenchmarkDef, BenchmarkSpec, ExpectedKnowledge};
use sir_semantics::truth::{SemanticTruth, Provenance};
use sir_semantics::concepts::SemanticConcept;

fn unknown_span() -> Span {
    Span::unknown()
}

fn graph001_validate_provenance(truths: &[SemanticTruth], candidates: &[sir_generation::candidate::Candidate]) {
    // We expect independent subgraphs.
    let card = truths.iter().find(|t| t.concept == SemanticConcept::CardinalityReduction).expect("Missing CardinalityReduction");
    let clear = truths.iter().find(|t| t.concept == SemanticConcept::ClearLowestSetBit).expect("Missing ClearLowestSetBit");
    let intersect = truths.iter().find(|t| t.concept == SemanticConcept::SetIntersection).expect("Missing SetIntersection");
    let modulo = truths.iter().find(|t| t.concept == SemanticConcept::ModuloPowerOfTwo);
    // ModuloPowerOfTwo does not yet generate Truths because it was not completely ported to Truth generation
    // (We did physical mapping for it just now, wait, we *did* update `semantics.rs` to generate Truth for ModuloPowerOfTwo!)
    // Wait, the panic says `Missing ModuloPowerOfTwo`.
    let modulo = modulo.unwrap();
    
    // They should all be distinct and Physical
    assert!(matches!(card.provenance, Provenance::Physical { .. }));
    assert!(matches!(clear.provenance, Provenance::Physical { .. }));
    assert!(matches!(intersect.provenance, Provenance::Physical { .. }));
    assert!(matches!(modulo.provenance, Provenance::Physical { .. }));
    
    // Check that we generated distinct candidates
    // Modulo -> Mask construction? We don't have generation for everything, but let's check what we have.
    // For CardinalityReduction we might generate Popcount if it's over a LogicalSequence.
    let log_seq = truths.iter().find(|t| t.concept == SemanticConcept::LogicalSequence);
    assert!(log_seq.is_some(), "Should find LogicalSequence for count");
}

fn graph002_validate_provenance(truths: &[SemanticTruth], candidates: &[sir_generation::candidate::Candidate]) {
    // This is overlapping domains: count += ((x & mask) != 0)
    let log_seq = truths.iter().find(|t| t.concept == SemanticConcept::LogicalSequence).expect("Missing LogicalSequence");
    let card = truths.iter().find(|t| t.concept == SemanticConcept::CardinalityReduction).expect("Missing CardinalityReduction");
    
    // Is MaskOperation extracted? Currently we extract MaskOperation structurally in sir_semantics.
    // We don't represent MaskOperation as a SemanticTruth yet (it is RegionRoles::MaskOperation).
    // Let's assert the truths we *do* expect:
    assert!(matches!(card.provenance, Provenance::Physical { .. }));
    assert!(matches!(log_seq.provenance, Provenance::Derived { .. }));
}


pub fn benchmarks() -> Vec<BenchmarkDef> {
    vec![
        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "GRAPH001",
                name: "multiple_independent_subgraphs",
                category: "Semantic Composition",
                input_desc: "Four independent semantic operations in one function",
                expected: ExpectedKnowledge::ProvenanceGraph {
                    expected_truths: vec!["CardinalityReduction", "ClearLowestSetBit", "SetIntersection", "ModuloPowerOfTwo"],
                    validation: graph001_validate_provenance,
                },
            },
            func: || {
                let mut b = Builder::new("four_operations", &[("arr", Type::Array { element: Box::new(Type::u64()), length: 64 }), ("x", Type::u64()), ("y", Type::u64()), ("a", Type::Array { element: Box::new(Type::Bool), length: 64 }), ("c", Type::Array { element: Box::new(Type::Bool), length: 64 })], Type::u64());
                let arr = b.parameter_index(0).unwrap();
                let x = b.parameter_index(1).unwrap();
                let y = b.parameter_index(2).unwrap();
                let a = b.parameter_index(3).unwrap();
                let c = b.parameter_index(4).unwrap();
                
                let zero = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let sixteen = b.constant(ConstantData::u64(16), Type::u64(), unknown_span());
                let sixty_four = b.constant(ConstantData::u64(64), Type::u64(), unknown_span());
                
                // Op 1: count += (arr[i] > 0)
                let i_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                let count_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                
                let next_i = b.add(i_init, one, unknown_span()).unwrap();
                let val = b.array_access(arr, i_init, Type::u64(), unknown_span()).unwrap();
                let is_positive = b.gt(val, zero, unknown_span()).unwrap();
                let to_add = b.select(is_positive, one, zero, unknown_span()).unwrap();
                let next_count = b.add(count_init, to_add, unknown_span()).unwrap();
                let cond = b.lt(next_i, sixty_four, unknown_span()).unwrap();
                
                let loop_node = b.r#loop(
                    &[next_i, val, is_positive, to_add, next_count, cond],
                    cond,
                    &[next_i, next_count],
                    &[i_init, count_init],
                    Type::Tuple { elements: vec![Type::u64(), Type::u64()] },
                    unknown_span()
                ).unwrap();
                
                // Op 2: y = x & (x - 1)
                let x_minus_one = b.sub(x, one, unknown_span()).unwrap();
                let x_cleared = b.bit_and(x, x_minus_one, unknown_span()).unwrap();
                
                // Op 3: a & c (pointwise set intersection)
                // actually we need to make it simple enough. Set intersection is recognized on ArrayAccess of bools or BoolAnd of elements
                // Let's do `a[i] && c[i]`
                // Wait, let's just make it a loop over a[i] && c[i]
                let a_val = b.array_access(a, i_init, Type::Bool, unknown_span()).unwrap();
                let c_val = b.array_access(c, i_init, Type::Bool, unknown_span()).unwrap();
                let a_and_c = b.bool_and(a_val, c_val, unknown_span()).unwrap();
                
                // Op 4: y % 16
                let y_mod_16 = b.rem(y, sixteen, unknown_span()).unwrap();
                
                let ret_val = b.tuple_extract(loop_node, 1, Type::u64(), unknown_span()).unwrap();
                b.return_value(ret_val, unknown_span()).unwrap();
                b.build()
            },
        },

        BenchmarkDef {
            spec: BenchmarkSpec {
                id: "GRAPH002",
                name: "overlapping_semantic_domains",
                category: "Semantic Composition",
                input_desc: "count += ((x & mask) != 0)",
                expected: ExpectedKnowledge::ProvenanceGraph {
                    expected_truths: vec!["CardinalityReduction", "PredicateMap", "LogicalSequence", "ElementSequence"],
                    validation: graph002_validate_provenance,
                },
            },
            func: || {
                let mut b = Builder::new("overlap_test", &[("arr", Type::Array { element: Box::new(Type::u64()), length: 64 }), ("mask", Type::u64())], Type::u64());
                let arr = b.parameter_index(0).unwrap();
                let mask = b.parameter_index(1).unwrap();
                
                let zero = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
                let sixty_four = b.constant(ConstantData::u64(64), Type::u64(), unknown_span());
                
                let i_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                let count_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
                
                let next_i = b.add(i_init, one, unknown_span()).unwrap();
                
                let x = b.array_access(arr, i_init, Type::u64(), unknown_span()).unwrap();
                let x_and_mask = b.bit_and(x, mask, unknown_span()).unwrap();
                let not_zero = b.ne(x_and_mask, zero, unknown_span()).unwrap();
                
                let to_add = b.select(not_zero, one, zero, unknown_span()).unwrap();
                let next_count = b.add(count_init, to_add, unknown_span()).unwrap();
                
                let cond = b.lt(next_i, sixty_four, unknown_span()).unwrap();
                
                let loop_node = b.r#loop(
                    &[next_i, x, x_and_mask, not_zero, to_add, next_count, cond],
                    cond,
                    &[next_i, next_count],
                    &[i_init, count_init],
                    Type::Tuple { elements: vec![Type::u64(), Type::u64()] },
                    unknown_span()
                ).unwrap();
                
                let ret_val = b.tuple_extract(loop_node, 1, Type::u64(), unknown_span()).unwrap();
                b.return_value(ret_val, unknown_span()).unwrap();
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
    fn test_graph_benchmarks() {
        for def in benchmarks() {
            run_benchmark((def.func)(), &def.spec);
        }
    }
}
