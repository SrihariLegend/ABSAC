//! P0A hardening: adversarial authorization-binding tests (advisor
//! checklist item "useful adversarial tests").
//!
//! - A candidate on region B is never authorized by region A's grant.
//! - A stale authorization (function changed since derivation) is
//!   rejected by AuthorizationRef::matches_function.
//! - Authorization databases default to deny (unknown region → no
//!   authorized concepts → zero candidates).

use sir_semantics::authorization::{
    AuthorizationDatabase, DomainCertificate, IntegerSemantics, ReductionOperator,
    TransformationAuthorization,
};
use sir_types::RegionId;
use sir_semantics::authorization::FunctionFingerprint;

fn unit_auth(region: RegionId, fingerprint: u64) -> TransformationAuthorization {
    TransformationAuthorization {
        region,
        function_fingerprint: FunctionFingerprint(fingerprint),
        domain: DomainCertificate::Reduction {
            operator: ReductionOperator::Sum,
            integer_semantics: IntegerSemantics::Modular,
            stride_is_unit: true,
        },
        authorized_concepts: vec![sir_semantics::concepts::SemanticConcept::CardinalityReduction],
        provenance: vec![],
        concrete: sir_semantics::authorization::ConcreteFacts::default(),
    }
}

/// Region A's authorization must not authorize a candidate attributed
/// to region B (advisor adversarial test: "authorize array A, attempt
/// candidate on array B" — at region granularity).
#[test]
fn region_mismatch_denies() {
    let mut db = AuthorizationDatabase::new();
    db.grant_raw(unit_auth(RegionId::new(0), 123));

    // Region 1 was never granted anything.
    let authorized_b = db.authorized_concepts(RegionId::new(1));
    assert!(
        authorized_b.is_empty(),
        "unknown region must default to deny, never permissive"
    );
    // Region 0 has exactly the granted concepts.
    let authorized_a = db.authorized_concepts(RegionId::new(0));
    assert!(authorized_b.is_empty() && !authorized_a.is_empty());
}

/// "Reuse authorization after rewrite" (advisor adversarial test): a
/// candidate's authorization is bound to the function version it was
/// derived from. After any mutation the fingerprint changes and the
/// candidate must be rejected as stale before any rewrite.
#[test]
fn stale_authorization_rejected() {
    // Two structurally different functions → different fingerprints.
    let f1 = build_count_loop("f_a", 16);
    let f2 = build_count_loop("f_b", 32);
    let fp1 = sir_semantics::authorization::function_fingerprint(&f1);
    let fp2 = sir_semantics::authorization::function_fingerprint(&f2);
    assert_ne!(fp1, fp2, "distinct functions must not share a fingerprint");

    // An authorization minted against f1's version...
    let mut db = AuthorizationDatabase::new();
    db.grant_raw(unit_auth(RegionId::new(0), fp1));
    let auth = &db.for_region(RegionId::new(0))[0];

    // ...validates against f1...
    assert!(auth.is_valid_for(&f1));
    // ...and is stale for f2 (definitely rejected on mismatch).
    assert!(!auth.is_valid_for(&f2));
}

fn build_count_loop(name: &'static str, limit: u64) -> sir_nodes::Function {
    use sir_builder::Builder;
    use sir_types::{ConstantData, Span, Type};

    let mut b = Builder::new(
        name,
        &[(
            "board",
            Type::Array {
                element: Box::new(Type::Bool),
                length: 64,
            },
        )],
        Type::i32(),
    );

    let board = b.parameter_index(0).unwrap();
    let i_initial = b.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let i_step = b.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let bound = b.constant(ConstantData::u64(limit), Type::u64(), Span::unknown());
    let count_initial = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());
    let zero_i32 = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());
    let one_i32 = b.constant(ConstantData::i32(1), Type::i32(), Span::unknown());

    let elem = b
        .array_access(board, i_initial, Type::Bool, Span::unknown())
        .unwrap();
    let inc = b.select(elem, one_i32, zero_i32, Span::unknown()).unwrap();
    let new_count = b.add(count_initial, inc, Span::unknown()).unwrap();
    let i_next = b.add(i_initial, i_step, Span::unknown()).unwrap();
    let cond = b.lt(i_initial, bound, Span::unknown()).unwrap();

    let loop_node = b
        .r#loop(
            &[elem, inc, new_count, i_next, cond],
            cond,
            &[new_count, i_next],
            &[count_initial, i_initial],
            Type::Tuple {
                elements: vec![Type::i32(), Type::u64()],
            },
            Span::unknown(),
        )
        .unwrap();

    b.return_value(loop_node, Span::unknown()).unwrap();
    b.build()
}

/// Every candidate that emerges from the pipeline carries gate-minted
/// authorization provenance: fingerprint of the exact function, its
/// region, and at least one covering domain. (Advisor item 3: provenance
/// travels with the candidate.)
#[test]
fn pipeline_candidates_carry_authorization_provenance() {
    use sir_generation::generator::CandidateGenerator;
    use sir_semantics::semantics::SemanticEngine;
    use sir_analysis::manager::AnalysisManager;
    use sir_inference::engine::InferenceEngine;

    let func = build_count_loop("count_loop", 64);
    let fingerprint = sir_semantics::authorization::function_fingerprint(&func);

    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);

    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());

    let mut inference = InferenceEngine::new();
    inference.infer(semantics.database(), semantics.structural_database());

    let authorizations = sir_semantics::authorization::derive_authorizations(
        &func,
        analysis.database(),
        semantics.database(),
    );
    let mut generator = CandidateGenerator::new();
    generator.generate(
        inference.context_database(),
        semantics.database(),
        &authorizations,
        &func,
    );

    let mut checked = 0;
    for candidate in generator.database().all_candidates() {
        assert!(
            candidate.authorization.matches_function(&func),
            "every pipeline candidate must carry a current-function authorization"
        );
        let cites_non_data = candidate
            .explanation
            .source_concepts
            .iter()
            .any(|c| !sir_semantics::authorization::is_data_concept(c));
        if cites_non_data {
            assert!(
                !candidate.authorization.domains.is_empty(),
                "candidate citing operation concepts must record which domains authorized it"
            );
        }
        assert_eq!(
            candidate.authorization.region,
            candidate.region,
            "authorization region must match the candidate's region"
        );
        checked += 1;
    }
    assert!(checked > 0, "board-scan pipeline must produce candidates");
    }

// ─────────────────────────────────────────────────────────────────
// ConcreteBindingDigest mutation tests (advisor adversarial list):
// take a pipeline-authorized candidate and independently mutate one
// bound field at a time — every mutation must invalidate the digest.
// ─────────────────────────────────────────────────────────────────

#[test]
fn mutated_bound_fields_invalidate_digest() {
    use sir_generation::generator::CandidateGenerator;
    use sir_semantics::semantics::SemanticEngine;
    use sir_analysis::manager::AnalysisManager;
    use sir_inference::engine::InferenceEngine;
    use sir_transform::ids::DefinitionId;
    use sir_types::RegionId;

    let func = build_count_loop("digest_loop", 64);
    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);
    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());
    let mut inference = sir_inference::engine::InferenceEngine::new();
    inference.infer(semantics.database(), semantics.structural_database());
    let authorizations = sir_semantics::authorization::derive_authorizations(
        &func,
        analysis.database(),
        semantics.database(),
    );
    let mut generator = CandidateGenerator::new();
    generator.generate(
        inference.context_database(),
        semantics.database(),
        &authorizations,
        &func,
    );

    let candidates: Vec<_> = generator.database().all_candidates().cloned().collect();
    assert!(!candidates.is_empty(), "pipeline must produce candidates");

    for candidate in candidates {
        // Baseline: the pipeline-minted digest is valid.
        assert!(candidate.binding_digest_valid());

        // Mutation 1: definition swap.
        let mut m = candidate.clone();
        m.definition_id = DefinitionId(9999);
        assert!(
            !m.binding_digest_valid(),
            "definition mutation must invalidate the binding digest"
        );

        // 2: region rebind.
        let mut r = candidate.clone();
        r.region = sir_types::RegionId::new(u64::MAX - 1);
        assert!(!r.binding_digest_valid());

        // 3: authorization fingerprint flip
        let mut f = candidate.clone();
        f.authorization.function_fingerprint ^= 0xFF;
        assert!(
            !f.authorization.matches_function(&func),
            "fingerprint mutation must be caught by the version key"
        );
        assert!(!f.binding_digest_valid());

        // 4: strategy family swap
        let mut s2 = candidate.clone();
        s2.strategy = match candidate.strategy {
            sir_generation::candidate::ImplementationStrategy::Popcount => {
                sir_generation::candidate::ImplementationStrategy::All
            }
            _ => sir_generation::candidate::ImplementationStrategy::Popcount,
        };
        assert!(!s2.binding_digest_valid(), "strategy mutation must invalidate");

        let _ = DefinitionId(0);
        let _ = RegionId::new(0);
    }
}
