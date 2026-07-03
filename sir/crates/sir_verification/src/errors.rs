//! Error and rejection types for verification.
//!
//! Defines `RejectReason` (why a proof was rejected definitively),
//! `UnknownReason` (why equivalence could not be determined), and
//! `InterpreterError` (runtime errors during expression evaluation).

use sir_transform::assumptions::Assumption;

use crate::semantic::expression::SemanticExpression;
use crate::semantic::value::{Environment, Value};

/// Reason a proof obligation was rejected definitively.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RejectReason {
    /// A required assumption was violated by the context.
    AssumptionViolated {
        assumption: Assumption,
    },
    /// The normalized expressions differ structurally.
    SemanticMismatch {
        lhs: SemanticExpression,
        rhs: SemanticExpression,
    },
    /// A counterexample was found during exhaustive verification.
    CounterExample {
        environment: Environment,
        lhs: Value,
        rhs: Value,
    },
    /// An expression variant is not supported by any backend.
    UnsupportedExpression {
        expr: SemanticExpression,
    },
}

/// Reason the verifier could not determine equivalence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnknownReason {
    /// No backend is applicable to this obligation.
    NoApplicableBackend,
    /// The domain is too large for exhaustive verification.
    DomainTooLarge {
        states: Option<u64>,
        max: u64,
    },
    /// The domain state count overflowed u64 during computation.
    DomainOverflow,
    /// A SemanticExpression variant has no handler in any backend.
    UnsupportedExpression {
        expr: SemanticExpression,
    },
    /// Normalization exceeded the maximum step count.
    NonTerminatingNormalization {
        steps: usize,
    },
}

/// Error during expression interpretation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InterpreterError {
    /// A variable was referenced but not bound in the environment.
    UnboundVariable(sir_transform::ids::VariableId),
    /// A value had an unexpected type during evaluation.
    TypeMismatch {
        expected: &'static str,
        found: Value,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_transform::ids::VariableId;

    #[test]
    fn reject_reason_assumption_violated() {
        let reason = RejectReason::AssumptionViolated {
            assumption: Assumption::EquivalentCardinality,
        };
        assert!(matches!(reason, RejectReason::AssumptionViolated { .. }));
    }

    #[test]
    fn reject_reason_semantic_mismatch() {
        let reason = RejectReason::SemanticMismatch {
            lhs: SemanticExpression::Variable(VariableId::new(0)),
            rhs: SemanticExpression::Variable(VariableId::new(1)),
        };
        assert!(matches!(reason, RejectReason::SemanticMismatch { .. }));
    }

    #[test]
    fn reject_reason_counter_example() {
        let env = Environment::new();
        let reason = RejectReason::CounterExample {
            environment: env,
            lhs: Value::Integer(0),
            rhs: Value::Integer(1),
        };
        assert!(matches!(reason, RejectReason::CounterExample { .. }));
    }

    #[test]
    fn reject_reason_unsupported_expression() {
        let expr = SemanticExpression::Variable(VariableId::new(0));
        let reason = RejectReason::UnsupportedExpression { expr };
        assert!(matches!(
            reason,
            RejectReason::UnsupportedExpression { .. }
        ));
    }

    #[test]
    fn unknown_reason_variants() {
        let no_backend = UnknownReason::NoApplicableBackend;
        assert_eq!(no_backend, UnknownReason::NoApplicableBackend);

        let too_large = UnknownReason::DomainTooLarge {
            states: Some(100),
            max: 50,
        };
        assert!(matches!(too_large, UnknownReason::DomainTooLarge { .. }));

        let overflow = UnknownReason::DomainOverflow;
        assert_eq!(overflow, UnknownReason::DomainOverflow);

        let steps = UnknownReason::NonTerminatingNormalization { steps: 1000 };
        assert!(matches!(
            steps,
            UnknownReason::NonTerminatingNormalization { .. }
        ));
    }

    #[test]
    fn interpreter_error_unbound_variable() {
        let err = InterpreterError::UnboundVariable(VariableId::new(5));
        assert!(matches!(err, InterpreterError::UnboundVariable(v) if v == VariableId::new(5)));
    }

    #[test]
    fn interpreter_error_type_mismatch() {
        let err = InterpreterError::TypeMismatch {
            expected: "Bool",
            found: Value::Integer(42),
        };
        assert!(matches!(err, InterpreterError::TypeMismatch { .. }));
    }
}
