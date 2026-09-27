// The translator's central property: lowering under two different
// substitution environments must give two different terms.
//
// An unroller visits a loop body once per iteration, each time binding the
// counter to a different constant. If the memo were keyed by `NodeId`
// alone and shared across those visits, the first iteration's term would
// be returned for every later iteration — the original would lower to the
// same expression as a patch that read `a[0]` throughout, and the equality
// query would certify exactly the miscompile this whole repair exists to
// prevent.

use sir_mech::CheckResult;
use sir_nodes::{Function, NodeKind};
use sir_types::{NodeId, Type};

fn element_access(function: &Function) -> (NodeId, NodeId) {
    let loop_id = function
        .arena
        .iter()
        .find(|node| matches!(node.kind, NodeKind::Loop { .. }))
        .expect("fixture has a loop")
        .id;
    let NodeKind::Loop { carried_inputs, .. } = &function.get_node(loop_id).unwrap().kind else {
        panic!("not a loop");
    };
    // The counter is the carried input typed i64 — the accumulator is i32.
    let counter = carried_inputs
        .iter()
        .copied()
        .find(|id| {
            matches!(
                function.get_node(*id).map(|n| &n.ty),
                Some(Type::Integer {
                    width: sir_types::IntegerWidth::I64,
                    ..
                })
            )
        })
        .expect("an i64 counter among the carried inputs");
    let access = function
        .arena
        .iter()
        .find(|node| matches!(&node.kind, NodeKind::ArrayAccess { index, .. } if *index == counter))
        .expect("an element access indexed by the counter")
        .id;
    (counter, access)
}

pub fn symbolic_lowering_is_environment_sensitive() {
    let ll = std::fs::read_to_string("../../../gate6a/v6_corpus.ll").expect("corpus");
    let function = sir_lower::lower_function(&ll, "p07_any_or_const_u32").expect("lower");
    let (counter, access) = element_access(&function);

    let mut translator = sir_opt::symbolic::Translator::new(&function);
    let zero = translator.constant(0, 64);
    let last = translator.constant(63, 64);

    let mut first = sir_opt::symbolic::Subst::new();
    first.bind(counter, zero);
    let at_zero = translator
        .lower(access, &first)
        .expect("the element access lowers at k=0");

    let mut final_iteration = sir_opt::symbolic::Subst::new();
    final_iteration.bind(counter, last);
    let at_last = translator
        .lower(access, &final_iteration)
        .expect("the element access lowers at k=63");

    // The trap, tested directly: two environments, two terms.
    assert_ne!(
        at_zero, at_last,
        "lowering at k=0 and k=63 must produce different terms — a memo \
         shared across environments would return the first for both, and \
         an equality query would then certify a wrong unrolling"
    );

    // And they really are different reads, not merely different terms.
    match sir_mech::prove_equal(translator.bv(), at_zero, at_last) {
        CheckResult::Counterexample(_) => {}
        other => panic!(
            "expected the two iterations to be refutable against each other, got {other:?}"
        ),
    }

    // The converse property: the *same* read, reached from two separate
    // calls, must denote the *same* variable — otherwise the original and
    // the patch could never be proved equal, and the check would refuse
    // everything.
    let mut again = sir_opt::symbolic::Subst::new();
    again.bind(counter, translator.constant(0, 64));
    let at_zero_again = translator
        .lower(access, &again)
        .expect("the element access lowers at k=0 a second time");
    assert_eq!(
        at_zero, at_zero_again,
        "the same (array, index) read must denote the same symbolic value \
         across calls, or no transformation could ever be proved"
    );
}

/// The gate: prove the *actual* artifact the compiler would ship, not the
/// transformation it intended.
pub fn the_gate_validates_the_actual_artifact() {
    let ll = std::fs::read_to_string("../../../gate6a/v6_corpus.ll").expect("corpus");
    let function = sir_lower::lower_function(&ll, "p07_any_or_const_u32").expect("lower");
    let optimized = sir_opt::optimize_any(&function).expect("the any family fires on p07");

    // The candidate is a real rewritten function, produced by the family
    // through `patch::apply`, with its own arena and its own ids. If the
    // gate were proving something about the original instead of about this
    // artifact, this assertion would pass for the wrong reason.
    assert_ne!(
        function.arena.len(),
        optimized.arena.len(),
        "the candidate really is a different function, not the original back"
    );
    assert!(
        optimized
            .arena
            .iter()
            .all(|node| !matches!(node.kind, NodeKind::Loop { .. })),
        "the reduction loop is gone in the candidate"
    );
    // Which side is wrong? Each function against itself must hold; if
    // either does not, the fault is in the lowering, not the transform.
    assert!(
        sir_opt::symbolic::equivalent(&function, &function),
        "the original must be equivalent to itself"
    );
    assert!(
        sir_opt::symbolic::equivalent(&optimized, &optimized),
        "the candidate must be equivalent to itself"
    );
    assert!(
        sir_opt::symbolic::equivalent(&function, &optimized),
        "the artifact the family actually builds must prove equivalent to \
         the original it replaces"
    );

    // A candidate that computes something else must not pass. Rewrite one
    // operand of the accumulator chain so the value differs, leaving the
    // shape alone — the same kind of change that shipped as S2.
    let mut wrong = optimized.clone();
    let victim = wrong
        .arena
        .iter()
        .find_map(|node| match &node.kind {
            NodeKind::Or { lhs, .. } if wrong.get_node(*lhs).is_some() => Some(node.id),
            _ => None,
        })
        .expect("an accumulator node");
    if let Some(node) = wrong.arena.get_mut(victim) {
        if let NodeKind::Or { lhs, .. } = &node.kind {
            let zero = *lhs;
            node.kind = NodeKind::Or {
                lhs: zero,
                rhs: zero,
            };
        }
    }
    assert!(
        !sir_opt::symbolic::equivalent(&function, &wrong),
        "a candidate whose accumulator no longer depends on its inputs \
         must be refused rather than certified"
    );
}

/// Definedness is not implied by the equality query, and this shows both
/// that the lowering refuses an out-of-bounds read and that it has to.
pub fn an_out_of_bounds_read_is_refused_and_value_equality_is_blind() {
    let ll = std::fs::read_to_string("../../../gate6a/v6_corpus.ll").expect("corpus");
    let function = sir_lower::lower_function(&ll, "p07_any_or_const_u32").expect("lower");
    let (counter, access) = element_access(&function);
    let NodeKind::ArrayAccess { base, .. } = function.get_node(access).unwrap().kind else {
        panic!("not an array access");
    };
    let mut translator = sir_opt::symbolic::Translator::new(&function);

    // The last in-bounds read lowers.
    let mut inside = sir_opt::symbolic::Subst::new();
    inside.bind(counter, translator.constant(63, 64));
    assert!(
        translator.lower(access, &inside).is_some(),
        "a[63] is inside a 64-element object and must lower"
    );

    // One past the end does not. This is the address half of definedness,
    // and it is checked against the object's declared extent rather than
    // inferred from how the value is used.
    let mut past = sir_opt::symbolic::Subst::new();
    past.bind(counter, translator.constant(64, 64));
    assert!(
        translator.lower(access, &past).is_none(),
        "a[64] is outside the object and must be refused"
    );

    // And why it must be its own check rather than a consequence of the
    // equality query. `a[64] & 0` is numerically null, so it proves equal
    // to zero — an out-of-bounds read rides into the result and the query
    // reports `Valid`. The read is assembled here by hand precisely because
    // the lowering refuses to produce it.
    let element = translator
        .element(base, 64, 32)
        .expect("the element variable for the out-of-bounds index");
    let zero = translator.constant(0, 32);
    let masked = translator.bv_mut().and(element, zero);
    assert!(
        matches!(
            sir_mech::prove_equal(translator.bv(), masked, zero),
            CheckResult::Valid
        ),
        "a[64] & 0 must prove equal to 0 — if it did not, this test would \
         not be demonstrating the blindness it claims to"
    );
}

/// The unroller, and the query that finally refutes the mutant.
pub fn the_unrolled_original_refutes_the_wrong_patch() {
    let ll = std::fs::read_to_string("../../../gate6a/v6_corpus.ll").expect("corpus");
    let function = sir_lower::lower_function(&ll, "p07_any_or_const_u32").expect("lower");
    let loop_id = function
        .arena
        .iter()
        .find(|node| matches!(node.kind, NodeKind::Loop { .. }))
        .expect("fixture has a loop")
        .id;
    let (_, access) = element_access(&function);
    let NodeKind::ArrayAccess { base, .. } = function.get_node(access).unwrap().kind else {
        panic!("not an array access");
    };

    let mut translator = sir_opt::symbolic::Translator::new(&function);
    let unrolled = sir_opt::symbolic::unroll(
        &mut translator,
        &function,
        loop_id,
        &sir_opt::symbolic::Subst::new(),
    )
    .expect("the canonical reduction unrolls");

    // The loop runs 64 times and the checker had to validate that against
    // the loop's own exit expression, not take it from an analysis fact.
    assert_eq!(unrolled.trip_count, 64, "the canonical loop runs 64 times");

    // The accumulator is the u32 carried variable, and its final value must
    // be the OR of all 64 distinct element reads.
    let NodeKind::Loop {
        carried_inputs,
        outputs,
        ..
    } = &function.get_node(loop_id).unwrap().kind
    else {
        unreachable!()
    };
    let slot = carried_inputs
        .iter()
        .position(|id| {
            matches!(
                function.get_node(*id).map(|n| &n.ty),
                Some(Type::Integer {
                    width: sir_types::IntegerWidth::I32,
                    ..
                })
            )
        })
        .expect("a u32 accumulator");
    assert_eq!(outputs.len(), unrolled.outputs.len(), "every live-out");
    let original = unrolled.outputs[slot];

    // The patch as built today: an OR chain over a[0]..a[63].
    let correct = {
        let mut acc = translator.constant(0, 32);
        for k in 0..64u64 {
            let element = translator.element(base, k, 32).expect("element read");
            acc = translator.bv_mut().or(acc, element);
        }
        acc
    };
    assert!(
        matches!(
            sir_mech::prove_equal(translator.bv(), original, correct),
            CheckResult::Valid
        ),
        "the unrolled original equals the correct chain — otherwise the \
         check would refuse real transformations and be useless"
    );

    // The mutant: the same chain reading a[0] sixty-four times. This is the
    // transformation the current apparatus accepts and the
    // vacuous-obligation hole certifies.
    let mutant = {
        let mut acc = translator.constant(0, 32);
        for _ in 0..64u64 {
            let element = translator.element(base, 0, 32).expect("element read");
            acc = translator.bv_mut().or(acc, element);
        }
        acc
    };
    match sir_mech::prove_equal(translator.bv(), original, mutant) {
        CheckResult::Counterexample(_) => {}
        other => panic!(
            "the repeated-a[0] patch must be refuted against the unrolled \
             original, got {other:?}"
        ),
    }
}
