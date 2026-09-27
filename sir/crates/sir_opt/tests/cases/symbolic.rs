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
