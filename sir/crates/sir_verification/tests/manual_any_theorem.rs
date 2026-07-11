use sir_generation::candidate::CandidateId;
use sir_transform::ids::{DefinitionId, ObligationId, VariableId};
use sir_types::RegionId;

use sir_verification::backends::symbolic::SymbolicVerifier;
use sir_verification::obligation::ProofObligation;
use sir_verification::semantic::expression::SemanticExpression;
use sir_verification::semantic::theorem::Theorem;

#[test]
fn manual_test_any_symbolic() {
    let board_var = VariableId::new(0);

    let lhs = SemanticExpression::Exists(Box::new(SemanticExpression::LogicalSequence {
        variable: board_var,
    }));

    let rhs = SemanticExpression::NotEqualZero(Box::new(SemanticExpression::Pack(Box::new(
        SemanticExpression::LogicalSequence {
            variable: board_var,
        },
    ))));

    let theorem = Theorem::new(lhs, rhs);

    let obligation = ProofObligation {
        id: ObligationId::new(0),
        region: RegionId::new(0),
        candidate: CandidateId::new(0),
        definition: DefinitionId::new(0),
        theorem,
        assumptions: vec![],
        domain: None,
    };

    let verifier = SymbolicVerifier::new();
    let result = verifier.verify(&obligation);

    assert!(
        matches!(result, sir_verification::VerificationResult::Proven(_)),
        "Expected Proven, got {:#?}",
        result
    );
}
