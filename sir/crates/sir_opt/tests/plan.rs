//! Single optimizer suite: all family slices, the kernel differential,
//! and the patch gate.

use sir_nodes::{Function, Node, NodeKind};
use sir_opt::patch::{self, Patch};
use sir_types::{Effects, NodeId, Span};

fn lower(corpus: &str, function: &str) -> Function {
    let ll = std::fs::read_to_string(format!("../../../gate6a/{corpus}"))
        .unwrap_or_else(|error| panic!("read {corpus}: {error}"));
    sir_lower::lower_function(&ll, function)
        .unwrap_or_else(|error| panic!("lower {function}: {error}"))
}

mod cases {
    pub mod all {
        include!("cases/all_slice.rs");
        pub fn run() {
            fixed_extent_all_and_chain_unrolls();
        }
    }
    pub mod arithmetic {
        include!("cases/arithmetic_slice.rs");
        pub fn run() {
            pow2_arithmetic_rewrites_to_shifts_and_masks();
            constant_left_division_is_refused_and_commutative_mul_still_fires();
        }
    }
    pub mod any {
        include!("cases/any_slice.rs");
        pub fn run() {
            fixed_extent_any_or_reduction_unrolls();
        }
    }
    pub mod count {
        include!("cases/count_slice.rs");
        pub fn run() {
            fixed_extent_cardinality_unrolls_predicates();
            runtime_bound_count_uses_helper();
        }
    }
    pub mod memchr {
        include!("cases/memchr_slice.rs");
        pub fn run() {
            fixed_extent_byte_search_uses_library_primitive();
        }
    }
    pub mod parity {
        include!("cases/parity_slice.rs");
        pub fn run() {
            fixed_extent_xor_reduction_unrolls();
        }
    }
    pub mod runtime_loop {
        include!("cases/runtime_loop.rs");
        pub fn run() {
            runtime_bound_search_lowers_and_uses_memchr();
            runtime_zero_test_loop_lowers();
            effectful_kernighan_loops_are_refused();
        }
    }
    pub mod search {
        include!("cases/search_slice.rs");
        pub fn run() {
            fixed_extent_inequality_search_is_retained_only_when_measured_faster();
        }
    }
    pub mod sum {
        include!("cases/sum_slice.rs");
        pub fn run() {
            fixed_extent_sum_unrolls();
            mapped_sum_reinstantiates_map_per_index();
            conditional_sum_over_pointer_unrolls();
            runtime_bound_sum_uses_helper();
        }
    }
    pub mod registry {
        include!("cases/family_registry.rs");
        pub fn run() {
            registry_exposes_expected_families_and_definitions();
            driver_runs_through_the_registry();
        }
    }
    pub mod z3 {
        include!("cases/z3_kernel_differential.rs");
        pub fn run() {
            kernel_agrees_with_z3_on_random_bitvector_equivalences();
        }
    }
}

#[test]
fn families_rewrite_and_prove() {
    assert!(sir_opt::prove::sum_laws(8));
    assert!(sir_opt::prove::or_laws(8));
    assert!(sir_opt::prove::and_laws(8));
    assert!(sir_opt::prove::first_match_law());

    cases::all::run();
    cases::arithmetic::run();
    cases::any::run();
    cases::count::run();
    cases::memchr::run();
    cases::parity::run();
    cases::runtime_loop::run();
    cases::search::run();
    cases::sum::run();
    cases::registry::run();
}

#[test]
fn kernel_agrees_with_z3() {
    cases::z3::run();
}

#[test]
fn patch_gate_rejects_dangling_replacement() {
    let function = lower("v6_corpus.ll", "p04_sum_u8_const_u64");
    let old = function.return_node.expect("fixture has a return");
    let dangling = Patch {
        nodes: Vec::new(),
        replacements: vec![(old, 0)],
    };
    assert!(
        patch::apply(&function, &dangling).is_none(),
        "a replacement naming a missing patch node must be rejected"
    );

    // A reachable patch node with a dangling operand must also fail the
    // post-patch structural verifier, not reach emission.
    let value = match function.arena.get(old).map(|node| node.kind.clone()) {
        Some(NodeKind::Return { value }) => value,
        other => panic!("fixture return shape changed: {other:?}"),
    };
    let value_ty = function
        .arena
        .get(value)
        .expect("return value exists")
        .ty
        .clone();
    let bad_node = Node::new(
        NodeId::new(0),
        NodeKind::Add {
            lhs: NodeId::new(u64::MAX),
            rhs: value,
        },
        value_ty.clone(),
        Effects::empty(),
        Span::unknown(),
    );
    let bad_add = Patch {
        nodes: vec![(0, bad_node)],
        replacements: vec![(value, 0)],
    };
    assert!(
        patch::apply(&function, &bad_add).is_none(),
        "a reachable node naming a nonexistent operand must be rejected"
    );

    // A patch node that introduces a write must fail the frame gate.
    let write_node = Node::new(
        NodeId::new(0),
        NodeKind::Constant(sir_types::ConstantData::u64(7)),
        value_ty.clone(),
        Effects::WRITE_MEMORY,
        Span::unknown(),
    );
    let write_patch = Patch {
        nodes: vec![(0, write_node)],
        replacements: vec![(value, 0)],
    };
    assert!(
        patch::apply(&function, &write_patch).is_none(),
        "a patch node that introduces a write must be rejected"
    );

    // Patch-local ids must not collide with the function's SSA ids.
    let collision = Patch {
        nodes: vec![(
            0,
            Node::new(
                NodeId::new(0),
                NodeKind::Constant(sir_types::ConstantData::u64(1)),
                value_ty.clone(),
                Effects::empty(),
                Span::unknown(),
            ),
        )],
        replacements: Vec::new(),
    };
    assert!(
        patch::apply(&function, &collision).is_none(),
        "patch-local id collisions must be rejected"
    );
}
