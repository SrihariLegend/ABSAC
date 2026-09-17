use sir_analysis::facts::FactDatabase;
use sir_analysis::manager::AnalysisManager;
use sir_builder::Builder;
use sir_nodes::ConvertKind;
use sir_nodes::Function;
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::recognizers::boolean_collection::recognize_boolean_collection;
use sir_semantics::semantics::SemanticEngine;
use sir_types::{ConstantData, Span, Type};

/// A predicate-collection All reduction over a DECLARED u8 array must
/// receive a structural description (and the predicate-collection role)
/// even though it has no LogicalSequence truth of its own — the w07
/// recall gap. Runtime-length pointer collections must NOT get a
/// fabricated fixed extent.
#[test]
fn predicate_reduction_over_declared_array_gets_a_structure() {
    let n = 32usize;
    let elem_ty = Type::u8();
    let array_ty = Type::Array {
        element: Box::new(elem_ty.clone()),
        length: n,
    };
    let mut b = Builder::new(
        "all_ge_declared",
        &[("vals", array_ty), ("floor_v", elem_ty.clone())],
        Type::Bool,
    );
    let vals = b.parameter_index(0).unwrap();
    let floor_v = b.parameter_index(1).unwrap();
    let i_init = b.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let one = b.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let limit = b.constant(ConstantData::u64(n as u64), Type::u64(), Span::unknown());
    let ok_init = b.constant(ConstantData::Bool(true), Type::Bool, Span::unknown());
    let elem = b
        .array_access(vals, i_init, elem_ty, Span::unknown())
        .unwrap();
    let ge = b.ge(elem, floor_v, Span::unknown()).unwrap();
    let next_ok = b.bool_and(ok_init, ge, Span::unknown()).unwrap();
    let i_next = b.add(i_init, one, Span::unknown()).unwrap();
    let cond = b.lt(i_init, limit, Span::unknown()).unwrap();
    let loop_node = b
        .r#loop(
            &[elem, ge, next_ok, i_next, cond],
            cond,
            &[next_ok, i_next],
            &[ok_init, i_init],
            Type::Tuple {
                elements: vec![Type::Bool, Type::u64()],
            },
            Span::unknown(),
        )
        .unwrap();
    let res = b.field_access(loop_node, "0", Type::Bool, Span::unknown()).unwrap();
    b.return_value(res, Span::unknown()).unwrap();
    let func = b.build();

    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);
    let mut engine = SemanticEngine::new();
    engine.derive(&func, analysis.database());

    let region = engine
        .database()
        .regions()
        .find(|(_, r)| r.contains(SemanticConcept::ConjunctiveReduction))
        .map(|(rid, _)| rid)
        .expect("ConjunctiveReduction region");
    let desc = engine
        .structural_database()
        .region(region)
        .expect("predicate reduction needs a structural description");
    assert_eq!(
        desc.source_structure,
        sir_transform::structures::SourceStructure::DynamicBooleanSequence { length: n }
    );
    assert!(
        desc.roles.iter().any(|role| matches!(
            role,
            sir_transform::roles::RegionRoles::PredicateCollectionReduction { .. }
        )),
        "role must be attached: {:?}",
        desc.roles
    );
}

/// Verify the recognizer is callable with a minimal function and returns
/// the expected result type.
#[test]
fn boolean_collection_recognizer_is_callable() {
    let mut func = Function::new("empty", sir_types::Type::Unit);
    func.add_param("x", sir_types::Type::i32(), sir_types::Span::unknown());
    let analysis = FactDatabase::new();
    let results = recognize_boolean_collection(&func, &analysis);

    // With no boolean arrays in the function, should return empty.
    assert!(
        results.is_empty(),
        "Expected no boolean collections in empty function"
    );
}

/// Build `uint64_t f(const uint8_t buf[64])` with `s += buf[i]`, either
/// plainly or guarded by `if (buf[i] & 1)` (lowered as
/// `s += select(cond, zext(buf[i]), 0)`).
fn build_sum_loop(conditional: bool) -> Function {
    let n = 64usize;
    let elem_ty = Type::u8();
    let array_ty = Type::Array {
        element: Box::new(elem_ty.clone()),
        length: n,
    };
    let name = if conditional {
        "conditional_sum"
    } else {
        "plain_sum"
    };
    let span = Span::unknown();
    let mut b = Builder::new(name, &[("buf", array_ty)], Type::u64());
    let buf = b.parameter_index(0).unwrap();
    // Distinct carried-initial nodes: one accumulator, one counter.
    let acc0 = b.constant(ConstantData::u64(0), Type::u64(), span);
    let i0 = b.constant(ConstantData::u64(0), Type::u64(), span);
    let one = b.constant(ConstantData::u64(1), Type::u64(), span);
    let bound = b.constant(ConstantData::u64(n as u64), Type::u64(), span);
    let elem = b.array_access(buf, i0, elem_ty, span).unwrap();
    let elem64 = b
        .convert(elem, Type::u64(), ConvertKind::ZeroExtend, span)
        .unwrap();
    let added = if conditional {
        let one8 = b.constant(ConstantData::u8(1), Type::u8(), span);
        let zero8 = b.constant(ConstantData::u8(0), Type::u8(), span);
        let masked = b.bit_and(elem, one8, span).unwrap();
        let cond = b.ne(masked, zero8, span).unwrap();
        let zero64 = b.constant(ConstantData::u64(0), Type::u64(), span);
        b.select(cond, elem64, zero64, span).unwrap()
    } else {
        elem64
    };
    let acc_next = b.add(acc0, added, span).unwrap();
    let i_next = b.add(i0, one, span).unwrap();
    let termination = b.lt(i0, bound, span).unwrap();
    let loop_ty = Type::Tuple {
        elements: vec![Type::u64(), Type::u64()],
    };
    let loop_node = b
        .r#loop(
            &[elem, added, acc_next, i_next, termination],
            termination,
            &[acc_next, i_next],
            &[acc0, i0],
            loop_ty,
            span,
        )
        .unwrap();
    let result = b.field_access(loop_node, "0", Type::u64(), span).unwrap();
    b.return_value(result, span).unwrap();
    b.build()
}

fn sum_truths(func: &Function) -> usize {
    let mut mgr = AnalysisManager::new();
    mgr.run_all(func);
    let mut engine = SemanticEngine::new();
    engine.derive(func, mgr.database());
    engine
        .database()
        .truths()
        .filter(|t| t.concept == SemanticConcept::SumReduction)
        .count()
}

fn concept_truths(func: &Function, concept: SemanticConcept) -> usize {
    let mut mgr = AnalysisManager::new();
    mgr.run_all(func);
    let mut engine = SemanticEngine::new();
    engine.derive(func, mgr.database());
    engine
        .database()
        .truths()
        .filter(|t| t.concept == concept)
        .count()
}

/// `s += (uint64_t)(buf[i] ^ 0x0F)`: the per-element XOR map.
fn build_xor_sum_loop() -> Function {
    let n = 64usize;
    let elem_ty = Type::u8();
    let array_ty = Type::Array {
        element: Box::new(elem_ty.clone()),
        length: n,
    };
    let span = Span::unknown();
    let mut b = Builder::new("xor_sum", &[("buf", array_ty)], Type::u64());
    let buf = b.parameter_index(0).unwrap();
    let acc0 = b.constant(ConstantData::u64(0), Type::u64(), span);
    let i0 = b.constant(ConstantData::u64(0), Type::u64(), span);
    let one = b.constant(ConstantData::u64(1), Type::u64(), span);
    let bound = b.constant(ConstantData::u64(n as u64), Type::u64(), span);
    let elem = b.array_access(buf, i0, elem_ty, span).unwrap();
    let elem64 = b
        .convert(elem, Type::u64(), ConvertKind::ZeroExtend, span)
        .unwrap();
    let mask = b.constant(ConstantData::u64(0x0F), Type::u64(), span);
    let mapped = b.bit_xor(elem64, mask, span).unwrap();
    let acc_next = b.add(acc0, mapped, span).unwrap();
    let i_next = b.add(i0, one, span).unwrap();
    let termination = b.lt(i0, bound, span).unwrap();
    let loop_node = b
        .r#loop(
            &[elem, mapped, acc_next, i_next, termination],
            termination,
            &[acc_next, i_next],
            &[acc0, i0],
            Type::Tuple {
                elements: vec![Type::u64(), Type::u64()],
            },
            span,
        )
        .unwrap();
    let result = b.field_access(loop_node, "0", Type::u64(), span).unwrap();
    b.return_value(result, span).unwrap();
    b.build()
}

/// Map-then-sum recall (v5 p11, `x ^ const`): the mapped value is a
/// legitimate reduction of the mapped elements, but it is NOT a raw
/// element sum — it must derive MappedSumReduction and never
/// SumReduction.
#[test]
fn xor_const_map_derives_mapped_sum_not_raw_sum() {
    let func = build_xor_sum_loop();
    assert_eq!(
        concept_truths(&func, SemanticConcept::SumReduction),
        0,
        "a mapped sum must not be labelled a raw-element SumReduction"
    );
    assert!(
        concept_truths(&func, SemanticConcept::MappedSumReduction) > 0,
        "the x ^ const map must derive MappedSumReduction"
    );
}

/// D5 boundary: the conditional masked sum derives NEITHER concept.
#[test]
fn conditional_sum_derives_neither_sum_family_concept() {
    let func = build_sum_loop(true);
    assert_eq!(concept_truths(&func, SemanticConcept::SumReduction), 0);
    assert_eq!(
        concept_truths(&func, SemanticConcept::MappedSumReduction),
        0,
        "select(cond, x, 0) is not a whitelisted per-element map"
    );
}

/// The plain raw-element sum stays SumReduction-only.
#[test]
fn plain_sum_derives_no_mapped_sum() {
    let func = build_sum_loop(false);
    assert!(concept_truths(&func, SemanticConcept::SumReduction) > 0);
    assert_eq!(concept_truths(&func, SemanticConcept::MappedSumReduction), 0);
}

/// Gate 6A-v3 finding (n08_conditional_sum): a conditional accumulation
/// was accepted as a raw-element sum. The masked value is not the element,
/// so SumReduction must not fire for it (Gate 6A-v3 D5 remediation).
#[test]
fn conditional_sum_is_not_recognized_as_raw_element_sum() {
    assert_eq!(sum_truths(&build_sum_loop(true)), 0);
}

/// Control: the plain element sum must still be recognized.
#[test]
fn plain_element_sum_is_still_recognized() {
    assert!(sum_truths(&build_sum_loop(false)) > 0);
}
