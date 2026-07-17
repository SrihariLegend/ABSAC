use sir_builder::Builder;
use sir_types::{ConstantData, Type, Span};
use crate::framework::{BenchmarkDef, BenchmarkSpec, ExpectedKnowledge};
use sir_semantics::truth::{SemanticTruth, Provenance};
use sir_semantics::concepts::SemanticConcept;

fn unknown_span() -> Span {
    Span::unknown()
}

fn comp002_validate_provenance(truths: &[SemanticTruth], _: &[sir_generation::candidate::Candidate]) {
    let at_most_one = truths.iter().find(|t| t.concept == SemanticConcept::AtMostOneBitSet).expect("Expected AtMostOneBitSet");
    
    match &at_most_one.provenance {
        Provenance::Derived { from_truths } => {
            assert_eq!(from_truths.len(), 2, "Expected exactly 2 parent truths");
            
            let parent_1 = truths.iter().find(|t| t.id == from_truths[0]).unwrap();
            let parent_2 = truths.iter().find(|t| t.id == from_truths[1]).unwrap();
            
            let concepts = vec![parent_1.concept, parent_2.concept];
            assert!(concepts.contains(&SemanticConcept::ClearLowestSetBit));
            assert!(concepts.contains(&SemanticConcept::IsZero));
            
            assert!(matches!(parent_1.provenance, Provenance::Physical { .. }));
            assert!(matches!(parent_2.provenance, Provenance::Physical { .. }));
        },
        _ => panic!("Expected AtMostOneBitSet to have Derived provenance"),
    }
}

fn comp003_validate_provenance(truths: &[SemanticTruth], _: &[sir_generation::candidate::Candidate]) {
    let at_most_ones: Vec<_> = truths.iter().filter(|t| t.concept == SemanticConcept::AtMostOneBitSet).collect();
    assert_eq!(at_most_ones.len(), 2, "Expected exactly 2 independent AtMostOneBitSet truths");
    
    for t in &at_most_ones {
        match &t.provenance {
            Provenance::Derived { from_truths } => {
                assert_eq!(from_truths.len(), 2);
            },
            _ => panic!("Expected Derived provenance"),
        }
    }
    
    // Ensure they don't share parents
    if let Provenance::Derived { from_truths: f1 } = &at_most_ones[0].provenance {
        if let Provenance::Derived { from_truths: f2 } = &at_most_ones[1].provenance {
            for id in f1 {
                assert!(!f2.contains(id), "Independent derived truths should not share provenance");
            }
        }
    }
}

fn comp004_validate_provenance(truths: &[SemanticTruth], _: &[sir_generation::candidate::Candidate]) {
    let clear_lowest = truths.iter().find(|t| t.concept == SemanticConcept::ClearLowestSetBit);
    assert!(clear_lowest.is_some(), "Expected to find physical ClearLowestSetBit");
    
    let at_most_one = truths.iter().find(|t| t.concept == SemanticConcept::AtMostOneBitSet);
    assert!(at_most_one.is_none(), "Should not derive AtMostOneBitSet because condition was == 1, not == 0");
}


fn comp005_validate_provenance(truths: &[SemanticTruth], candidates: &[sir_generation::candidate::Candidate]) {
    // We want to assert the entire derivation chain from LogicalSequence + CardinalityReduction
    // all the way back to the Collection, ElementSequence, and PredicateMap.
    
    // First, verify that we generated the Popcount candidate based on this chain
    let popcount_candidate = candidates.iter().find(|c| c.strategy == sir_generation::candidate::ImplementationStrategy::Popcount);
    assert!(popcount_candidate.is_some(), "Expected to generate a Popcount candidate");
    let expl = &popcount_candidate.unwrap().explanation;
    assert!(expl.source_concepts.contains(&SemanticConcept::CardinalityReduction));
    assert!(expl.source_concepts.contains(&SemanticConcept::LogicalSequence));
    
    // Now verify the semantic truth graph itself
    // 1. CardinalityReduction (Physical)
    let card_red = truths.iter().find(|t| t.concept == SemanticConcept::CardinalityReduction).expect("Missing CardinalityReduction");
    assert!(matches!(card_red.provenance, Provenance::Physical { .. }));
    
    // 2. LogicalSequence (Derived)
    let log_seq = truths.iter().find(|t| t.concept == SemanticConcept::LogicalSequence).expect("Missing LogicalSequence");
    if let Provenance::Derived { from_truths } = &log_seq.provenance {
        assert_eq!(from_truths.len(), 2, "LogicalSequence should derive from PredicateMap and ElementSequence");
        
        let p1 = truths.iter().find(|t| t.id == from_truths[0]).unwrap();
        let p2 = truths.iter().find(|t| t.id == from_truths[1]).unwrap();
        
        let concepts = vec![p1.concept, p2.concept];
        assert!(concepts.contains(&SemanticConcept::PredicateMap));
        assert!(concepts.contains(&SemanticConcept::ElementSequence));
        
        // 3. Both of those should be physical
        assert!(matches!(p1.provenance, Provenance::Physical { .. }));
        assert!(matches!(p2.provenance, Provenance::Physical { .. }));
    } else {
        panic!("Expected LogicalSequence to be Derived");
    }
}

fn comp005() -> BenchmarkDef {
    BenchmarkDef {
        spec: BenchmarkSpec {
            id: "COMP005",
            name: "full_derivation_chain_to_popcount",
            category: "Semantic Composition",
            input_desc: "count += (arr[i] > 0)",
            expected: ExpectedKnowledge::ProvenanceGraph {
                expected_truths: vec!["LogicalSequence", "CardinalityReduction", "PredicateMap", "ElementSequence"],
                validation: comp005_validate_provenance,
            },
        },
        func: || {
            let mut b = Builder::new("count_positives", &[("arr", Type::Array { element: Box::new(Type::u64()), length: 64 })], Type::u64());
            let arr = b.parameter_index(0).unwrap();
            
            let zero = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
            let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
            let sixty_four = b.constant(ConstantData::u64(64), Type::u64(), unknown_span());
            
            let i_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
            let count_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
            
            let next_i = b.add(i_init, one, unknown_span()).unwrap();
            
            let x = b.array_access(arr, i_init, Type::u64(), unknown_span()).unwrap();
            let is_positive = b.gt(x, zero, unknown_span()).unwrap();
            let to_add = b.select(is_positive, one, zero, unknown_span()).unwrap();
            
            let next_count = b.add(count_init, to_add, unknown_span()).unwrap();
            let cond = b.lt(next_i, sixty_four, unknown_span()).unwrap();
            
            let loop_node = b.r#loop(
                &[next_i, x, is_positive, to_add, next_count, cond],
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
    }
}

fn comp001() -> BenchmarkDef {
    BenchmarkDef {
        spec: BenchmarkSpec {
            id: "COMP001",
            name: "count_clear_lowest_set_bit_is_zero",
            category: "Semantic Composition",
            input_desc: "count += ((x & (x - 1)) == 0)",
            expected: ExpectedKnowledge::Optimizes {
                semantic_domain: "MaskAlgebra",
                concepts: vec!["AtMostOneBitSet", "LogicalSequence", "ClearLowestSetBit"],
                representation: "MaskAlgebra",
                candidate: "ClearLowestBit",
                proof: "Success",
                rewrite: "blsr",
            },
        },
        func: || {
            let mut b = Builder::new("count_powers_of_two", &[("arr", Type::Array { element: Box::new(Type::u64()), length: 64 })], Type::u64());
            let arr = b.parameter_index(0).unwrap();
            
            let zero = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
            let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
            let sixty_four = b.constant(ConstantData::u64(64), Type::u64(), unknown_span());
            
            let i_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
            let count_init = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
            
            let next_i = b.add(i_init, one, unknown_span()).unwrap();
            
            let x = b.array_access(arr, i_init, Type::u64(), unknown_span()).unwrap();
            
            let x_minus_one = b.sub(x, one, unknown_span()).unwrap();
            let x_and_x_minus_one = b.bit_and(x, x_minus_one, unknown_span()).unwrap();
            let is_zero = b.eq(x_and_x_minus_one, zero, unknown_span()).unwrap();
            let to_add = b.select(is_zero, one, zero, unknown_span()).unwrap();
            let next_count = b.add(count_init, to_add, unknown_span()).unwrap();
            
            let cond = b.lt(next_i, sixty_four, unknown_span()).unwrap();
            
            let loop_node = b.r#loop(
                &[next_i, x, x_minus_one, x_and_x_minus_one, is_zero, to_add, next_count, cond],
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
    }
}

fn comp002() -> BenchmarkDef {
    BenchmarkDef {
        spec: BenchmarkSpec {
            id: "COMP002",
            name: "provenance_inheritance_at_most_one_bit_set",
            category: "Semantic Composition",
            input_desc: "(x & (x - 1)) == 0",
            expected: ExpectedKnowledge::ProvenanceGraph {
                expected_truths: vec!["AtMostOneBitSet", "ClearLowestSetBit", "IsZero"],
                validation: comp002_validate_provenance,
            },
        },
        func: || {
            let mut b = Builder::new("check_power_of_two", &[("x", Type::u64())], Type::Bool);
            let x = b.parameter_index(0).unwrap();
            let zero = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
            let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
            
            let x_minus_one = b.sub(x, one, unknown_span()).unwrap();
            let x_and_x_minus_one = b.bit_and(x, x_minus_one, unknown_span()).unwrap();
            let is_zero = b.eq(x_and_x_minus_one, zero, unknown_span()).unwrap();
            
            b.return_value(is_zero, unknown_span()).unwrap();
            b.build()
        },
    }
}

fn comp003() -> BenchmarkDef {
    BenchmarkDef {
        spec: BenchmarkSpec {
            id: "COMP003",
            name: "multiple_independent_derived_truths",
            category: "Semantic Composition",
            input_desc: "((x & (x - 1)) == 0) && ((y & (y - 1)) == 0)",
            expected: ExpectedKnowledge::ProvenanceGraph {
                expected_truths: vec!["AtMostOneBitSet", "AtMostOneBitSet"],
                validation: comp003_validate_provenance,
            },
        },
        func: || {
            let mut b = Builder::new("two_independent_powers", &[("x", Type::u64()), ("y", Type::u64())], Type::Bool);
            let x = b.parameter_index(0).unwrap();
            let y = b.parameter_index(1).unwrap();
            let zero = b.constant(ConstantData::u64(0), Type::u64(), unknown_span());
            let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
            
            let x_minus_one = b.sub(x, one, unknown_span()).unwrap();
            let x_and_x_minus_one = b.bit_and(x, x_minus_one, unknown_span()).unwrap();
            let x_is_zero = b.eq(x_and_x_minus_one, zero, unknown_span()).unwrap();
            
            let y_minus_one = b.sub(y, one, unknown_span()).unwrap();
            let y_and_y_minus_one = b.bit_and(y, y_minus_one, unknown_span()).unwrap();
            let y_is_zero = b.eq(y_and_y_minus_one, zero, unknown_span()).unwrap();
            
            let res = b.bool_and(x_is_zero, y_is_zero, unknown_span()).unwrap();
            b.return_value(res, unknown_span()).unwrap();
            b.build()
        },
    }
}

fn comp004() -> BenchmarkDef {
    BenchmarkDef {
        spec: BenchmarkSpec {
            id: "COMP004",
            name: "conflicting_truths_negative_case",
            category: "Semantic Composition",
            input_desc: "(x & (x - 1)) == 1",
            expected: ExpectedKnowledge::ProvenanceGraph {
                expected_truths: vec!["ClearLowestSetBit"],
                validation: comp004_validate_provenance,
            },
        },
        func: || {
            let mut b = Builder::new("not_power_of_two", &[("x", Type::u64())], Type::Bool);
            let x = b.parameter_index(0).unwrap();
            let one = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
            
            let x_minus_one = b.sub(x, one, unknown_span()).unwrap();
            let x_and_x_minus_one = b.bit_and(x, x_minus_one, unknown_span()).unwrap();
            let is_one = b.eq(x_and_x_minus_one, one, unknown_span()).unwrap();
            
            b.return_value(is_one, unknown_span()).unwrap();
            b.build()
        },
    }
}

pub fn benchmarks() -> Vec<BenchmarkDef> {
    vec![
        comp001(),
        comp002(),
        comp003(),
        comp004(),
        comp005(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::run_benchmark;
    
    #[test]
    fn test_composition_benchmarks() {
        for def in benchmarks() {
            run_benchmark((def.func)(), &def.spec);
        }
    }
}
