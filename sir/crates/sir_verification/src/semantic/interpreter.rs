//! Interpreter — canonical operational semantics of SemanticExpression.
//!
//! The reference implementation against which all verification backends
//! must agree. Deliberately dumb — one recursive walk, no optimization,
//! no caching. Always returns Result — never panics.

use crate::errors::InterpreterError;
use crate::semantic::expression::{Predicate, SemanticExpression};
use crate::semantic::value::{pack_bits, Environment, Value};

/// The canonical operational semantics of SemanticExpression.
///
/// Deliberately dumb — one recursive walk, no optimization, no caching.
/// The reference implementation against which all backends are validated.
///
/// Invariant: Every verification backend (symbolic, exhaustive, SMT, SAT,
/// theorem prover) must agree with the interpreter on all supported expressions.
#[derive(Clone, Debug)]
pub struct Interpreter;

impl Interpreter {
    /// Evaluate an expression in the given environment.
    /// Never panics — returns InterpreterError on malformed states.
    pub fn evaluate(
        &self,
        expr: &SemanticExpression,
        env: &Environment,
    ) -> Result<Value, InterpreterError> {
        match expr {
            SemanticExpression::Variable(id) => env
                .lookup(*id)
                .cloned()
                .ok_or(InterpreterError::UnboundVariable(*id)),

            SemanticExpression::Constant(c) => Self::constant_to_value(c),

            SemanticExpression::BooleanArray { variable } => match env.lookup(*variable) {
                Some(Value::BooleanArray(bits)) => Ok(Value::BooleanArray(bits.clone())),
                Some(other) => Err(InterpreterError::TypeMismatch {
                    expected: "BooleanArray",
                    found: other.clone(),
                }),
                None => Err(InterpreterError::UnboundVariable(*variable)),
            },

            SemanticExpression::Pack(inner) => {
                let val = self.evaluate(inner, env)?;
                match val {
                    Value::BooleanArray(bits) => Ok(Value::BitVector(pack_bits(&bits)?)),
                    other => Err(InterpreterError::TypeMismatch {
                        expected: "BooleanArray",
                        found: other,
                    }),
                }
            }

            SemanticExpression::Filter { input, predicate } => {
                let val = self.evaluate(input, env)?;
                match val {
                    Value::BooleanArray(bits) => {
                        let filtered: Vec<bool> =
                            bits.into_iter().filter(|b| predicate.test(*b)).collect();
                        Ok(Value::BooleanArray(filtered))
                    }
                    other => Err(InterpreterError::TypeMismatch {
                        expected: "BooleanArray",
                        found: other,
                    }),
                }
            }

            SemanticExpression::Count(inner) => {
                let val = self.evaluate(inner, env)?;
                match val {
                    Value::BooleanArray(bits) => {
                        let count = bits.iter().filter(|b| **b).count() as u64;
                        Ok(Value::Integer(count))
                    }
                    other => Err(InterpreterError::TypeMismatch {
                        expected: "BooleanArray",
                        found: other,
                    }),
                }
            }

            SemanticExpression::Popcount(inner) => {
                let val = self.evaluate(inner, env)?;
                match val {
                    Value::BitVector(bv) => Ok(Value::Integer(bv.bits.count_ones() as u64)),
                    other => Err(InterpreterError::TypeMismatch {
                        expected: "BitVector",
                        found: other,
                    }),
                }
            }

            SemanticExpression::Exists(inner) => {
                let val = self.evaluate(inner, env)?;
                match val {
                    Value::BooleanArray(bits) => Ok(Value::Bool(bits.iter().any(|b| *b))),
                    other => Err(InterpreterError::TypeMismatch {
                        expected: "BooleanArray",
                        found: other,
                    }),
                }
            }

            SemanticExpression::All(inner) => {
                let val = self.evaluate(inner, env)?;
                match val {
                    Value::BooleanArray(bits) => Ok(Value::Bool(bits.iter().all(|b| *b))),
                    other => Err(InterpreterError::TypeMismatch {
                        expected: "BooleanArray",
                        found: other,
                    }),
                }
            }

            SemanticExpression::Parity(inner) => {
                let val = self.evaluate(inner, env)?;
                match val {
                    Value::BooleanArray(bits) => {
                        let count = bits.iter().filter(|b| **b).count();
                        Ok(Value::Bool(count % 2 == 1))
                    }
                    other => Err(InterpreterError::TypeMismatch {
                        expected: "BooleanArray",
                        found: other,
                    }),
                }
            }

            SemanticExpression::NotEqualZero(inner) => {
                let val = self.evaluate(inner, env)?;
                match val {
                    Value::BitVector(bv) => Ok(Value::Bool(bv.bits != 0)),
                    other => Err(InterpreterError::TypeMismatch {
                        expected: "BitVector",
                        found: other,
                    }),
                }
            }

            SemanticExpression::EqualFullMask(inner) => {
                let val = self.evaluate(inner, env)?;
                match val {
                    Value::BitVector(bv) => {
                        let full_mask = if bv.width == 128 {
                            u128::MAX
                        } else {
                            (1u128 << bv.width) - 1
                        };
                        Ok(Value::Bool(bv.bits == full_mask))
                    }
                    other => Err(InterpreterError::TypeMismatch {
                        expected: "BitVector",
                        found: other,
                    }),
                }
            }

            SemanticExpression::BitwiseAndOne(inner) => {
                let val = self.evaluate(inner, env)?;
                match val {
                    Value::Integer(i) => Ok(Value::Integer(i & 1)),
                    other => Err(InterpreterError::TypeMismatch {
                        expected: "Integer",
                        found: other,
                    }),
                }
            }

            SemanticExpression::Modulo(lhs, rhs) => {
                let l = self.evaluate(lhs, env)?;
                let r = self.evaluate(rhs, env)?;
                match (l, r) {
                    (Value::Integer(lv), Value::Integer(rv)) => {
                        if rv == 0 {
                            Err(InterpreterError::TypeMismatch {
                                expected: "non-zero",
                                found: Value::Integer(0),
                            })
                        } else {
                            Ok(Value::Integer(lv % rv))
                        }
                    }
                    _ => Err(InterpreterError::TypeMismatch {
                        expected: "Integer",
                        found: Value::Integer(0),
                    }),
                }
            }

            SemanticExpression::BitwiseAnd(lhs, rhs) => {
                let l = self.evaluate(lhs, env)?;
                let r = self.evaluate(rhs, env)?;
                match (l, r) {
                    (Value::Integer(lv), Value::Integer(rv)) => Ok(Value::Integer(lv & rv)),
                    _ => Err(InterpreterError::TypeMismatch {
                        expected: "Integer",
                        found: Value::Integer(0),
                    }),
                }
            }

            SemanticExpression::Divide(lhs, rhs) => {
                let l = self.evaluate(lhs, env)?;
                let r = self.evaluate(rhs, env)?;
                match (l, r) {
                    (Value::Integer(lv), Value::Integer(rv)) => {
                        if rv == 0 {
                            Err(InterpreterError::TypeMismatch {
                                expected: "non-zero",
                                found: Value::Integer(0),
                            })
                        } else {
                            Ok(Value::Integer(lv / rv))
                        }
                    }
                    _ => Err(InterpreterError::TypeMismatch {
                        expected: "Integer",
                        found: Value::Integer(0),
                    }),
                }
            }

            SemanticExpression::ShiftRight(lhs, rhs) => {
                let l = self.evaluate(lhs, env)?;
                let r = self.evaluate(rhs, env)?;
                match (l, r) {
                    (Value::Integer(lv), Value::Integer(rv)) => Ok(Value::Integer(lv >> rv)),
                    _ => Err(InterpreterError::TypeMismatch {
                        expected: "Integer",
                        found: Value::Integer(0),
                    }),
                }
            }

            SemanticExpression::Multiply(lhs, rhs) => {
                let l = self.evaluate(lhs, env)?;
                let r = self.evaluate(rhs, env)?;
                match (l, r) {
                    (Value::Integer(lv), Value::Integer(rv)) => {
                        Ok(Value::Integer(lv.wrapping_mul(rv)))
                    }
                    _ => Err(InterpreterError::TypeMismatch {
                        expected: "Integer",
                        found: Value::Integer(0),
                    }),
                }
            }

            SemanticExpression::ShiftLeft(lhs, rhs) => {
                let l = self.evaluate(lhs, env)?;
                let r = self.evaluate(rhs, env)?;
                match (l, r) {
                    (Value::Integer(lv), Value::Integer(rv)) => Ok(Value::Integer(lv << rv)),
                    _ => Err(InterpreterError::TypeMismatch {
                        expected: "Integer",
                        found: Value::Integer(0),
                    }),
                }
            }
        }
    }

    /// Convert a ConstantData to a Value.
    fn constant_to_value(c: &sir_types::ConstantData) -> Result<Value, InterpreterError> {
        match c {
            sir_types::ConstantData::Bool(b) => Ok(Value::Bool(*b)),
            sir_types::ConstantData::Integer { value, signed, .. } => {
                if *signed {
                    value
                        .parse::<i64>()
                        .map(|v| Value::Integer(v as u64))
                        .map_err(|_| InterpreterError::MalformedConstant {
                            value: value.clone(),
                            reason: "signed integer string failed to parse as i64",
                        })
                } else {
                    value.parse::<u64>().map(Value::Integer).map_err(|_| {
                        InterpreterError::MalformedConstant {
                            value: value.clone(),
                            reason: "unsigned integer string failed to parse as u64",
                        }
                    })
                }
            }
            sir_types::ConstantData::Unit => Ok(Value::Integer(0)),
            _ => Ok(Value::Integer(0)), // fallback for constant types without a corresponding Value variant
        }
    }
}

impl Predicate {
    /// Test whether a boolean value satisfies this predicate.
    pub fn test(&self, _value: bool) -> bool {
        match self {
            Predicate::True => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::expression::SemanticExpression;
    use crate::semantic::value::{BitVectorValue, Environment, Value};
    use sir_transform::ids::VariableId;
    fn board_env(bits: Vec<bool>) -> Environment {
        let mut env = Environment::new();
        env.bind(VariableId::new(0), Value::BooleanArray(bits));
        env
    }

    #[test]
    fn evaluate_variable() {
        let env = board_env(vec![true, false, true, false]);
        let expr = SemanticExpression::Variable(VariableId::new(0));
        let result = Interpreter.evaluate(&expr, &env).unwrap();
        assert_eq!(result, Value::BooleanArray(vec![true, false, true, false]));
    }

    #[test]
    fn evaluate_unbound_variable() {
        let env = Environment::new();
        let expr = SemanticExpression::Variable(VariableId::new(99));
        let result = Interpreter.evaluate(&expr, &env);
        assert!(matches!(result, Err(InterpreterError::UnboundVariable(_))));
    }

    #[test]
    fn evaluate_boolean_array() {
        let env = board_env(vec![true, false, true, false]);
        let expr = SemanticExpression::BooleanArray {
            variable: VariableId::new(0),
        };
        let result = Interpreter.evaluate(&expr, &env).unwrap();
        assert_eq!(result, Value::BooleanArray(vec![true, false, true, false]));
    }

    #[test]
    fn evaluate_pack() {
        let env = board_env(vec![true, false, true, false]);
        let expr = SemanticExpression::Pack(Box::new(SemanticExpression::BooleanArray {
            variable: VariableId::new(0),
        }));
        let result = Interpreter.evaluate(&expr, &env).unwrap();
        assert_eq!(
            result,
            Value::BitVector(BitVectorValue {
                bits: 0b0101,
                width: 4
            })
        );
    }

    #[test]
    fn evaluate_filter_true() {
        let env = board_env(vec![true, false, true, false]);
        let expr = SemanticExpression::Filter {
            input: Box::new(SemanticExpression::BooleanArray {
                variable: VariableId::new(0),
            }),
            predicate: Predicate::True,
        };
        let result = Interpreter.evaluate(&expr, &env).unwrap();
        // True predicate is identity — all elements pass
        assert_eq!(result, Value::BooleanArray(vec![true, false, true, false]));
    }

    #[test]
    fn evaluate_count() {
        let env = board_env(vec![true, false, true, false]);
        let expr = SemanticExpression::Count(Box::new(SemanticExpression::BooleanArray {
            variable: VariableId::new(0),
        }));
        let result = Interpreter.evaluate(&expr, &env).unwrap();
        assert_eq!(result, Value::Integer(2));
    }

    #[test]
    fn evaluate_popcount() {
        let mut env = Environment::new();
        // 0b1010 = bits 1 and 3 set → popcount = 2
        env.bind(
            VariableId::new(0),
            Value::BitVector(BitVectorValue {
                bits: 0b1010,
                width: 4,
            }),
        );
        let expr = SemanticExpression::Popcount(Box::new(SemanticExpression::Variable(
            VariableId::new(0),
        )));
        let result = Interpreter.evaluate(&expr, &env).unwrap();
        assert_eq!(result, Value::Integer(2));
    }

    #[test]
    fn evaluate_bs001_lhs() {
        // Count(Filter(BooleanArray(v), True))
        let env = board_env(vec![true, true, false, true]); // 3 true
        let lhs = SemanticExpression::Count(Box::new(SemanticExpression::Filter {
            input: Box::new(SemanticExpression::BooleanArray {
                variable: VariableId::new(0),
            }),
            predicate: Predicate::True,
        }));
        let result = Interpreter.evaluate(&lhs, &env).unwrap();
        assert_eq!(result, Value::Integer(3));
    }

    #[test]
    fn evaluate_bs001_rhs() {
        // Popcount(Pack(BooleanArray(v)))
        let env = board_env(vec![true, true, false, true]); // 3 true
        let rhs = SemanticExpression::Popcount(Box::new(SemanticExpression::Pack(Box::new(
            SemanticExpression::BooleanArray {
                variable: VariableId::new(0),
            },
        ))));
        let result = Interpreter.evaluate(&rhs, &env).unwrap();
        assert_eq!(result, Value::Integer(3));
    }

    #[test]
    fn evaluate_bs001_lhs_equals_rhs() {
        // For any given input, the lhs and rhs produce the same result
        let env = board_env(vec![false, true, false, true, true, false, false, true]); // 4 true
        let lhs = SemanticExpression::Count(Box::new(SemanticExpression::Filter {
            input: Box::new(SemanticExpression::BooleanArray {
                variable: VariableId::new(0),
            }),
            predicate: Predicate::True,
        }));
        let rhs = SemanticExpression::Popcount(Box::new(SemanticExpression::Pack(Box::new(
            SemanticExpression::BooleanArray {
                variable: VariableId::new(0),
            },
        ))));
        let lhs_result = Interpreter.evaluate(&lhs, &env).unwrap();
        let rhs_result = Interpreter.evaluate(&rhs, &env).unwrap();
        assert_eq!(lhs_result, rhs_result);
        assert_eq!(lhs_result, Value::Integer(4));
    }

    #[test]
    fn evaluate_type_mismatch_pack_on_non_array() {
        let mut env = Environment::new();
        env.bind(VariableId::new(0), Value::Integer(42));
        let expr =
            SemanticExpression::Pack(Box::new(SemanticExpression::Variable(VariableId::new(0))));
        let result = Interpreter.evaluate(&expr, &env);
        assert!(matches!(result, Err(InterpreterError::TypeMismatch { .. })));
    }
}
