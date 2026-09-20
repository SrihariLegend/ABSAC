//! Single proof suite for `sir_mech`.
//!
//! The case bodies live under `tests/cases/` so the workspace keeps a
//! single-digit test count: three proof groups below, one arithmetic /
//! term contract test in-crate, three optimizer tests, and one
//! end-to-end harness test.

mod cases {
    pub mod bitvector {
        include!("cases/bitvector_lemmas.rs");

        pub fn run() {
            division_and_remainder_by_power_of_two_match_shift_and_mask();
            division_remainder_decomposition_holds();
            false_remainder_claim_is_refuted();
            per_byte_pcmpeqb_movemask_bit_is_equality();
            per_byte_masked_compare();
            popcount_of_movemask_is_sum_of_predicate_bits();
            sad_lane_equals_byte_sum();
            add_sub_cancel();
            false_identity_yields_real_counterexample();
            tautology_and_implication();
            solver_model_reconstructs_variables();
        }
    }

    pub mod intrinsics {
        include!("cases/intrinsic_lemmas.rs");

        pub fn run() {
            per_byte_lemma_proved_and_replayed();
            all_chunk_lemma_proved();
            sum_context_rewrite();
            schema_instantiation_composes();
            // The two 32-lane bit-blasts are independent; run them on
            // separate cores so the proof suite stays wall-clock short.
            std::thread::scope(|scope| {
                let popcount = scope.spawn(popcount_is_sum_of_bits);
                let cardinality = scope.spawn(cardinality_chunk_lemma_proved);
                popcount.join().expect("popcount lemma");
                cardinality.join().expect("cardinality lemma");
            });
        }
    }

    pub mod induction {
        include!("cases/kernel_induction.rs");

        pub fn run() {
            fold_shift_lemma_holds_for_all_m();
            fold_shift_instantiates_to_classic_split();
        }
    }
}

#[test]
fn bitvector_decision_procedure() {
    cases::bitvector::run();
}

#[test]
fn kernel_intrinsics_proved() {
    cases::intrinsics::run();
}

#[test]
fn kernel_induction_proofs() {
    cases::induction::run();
}
