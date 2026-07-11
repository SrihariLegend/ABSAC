//! Value — operational semantics result types.
//!
//! Defines `Value` (the result type of interpreting a `SemanticExpression`),
//! `BitVectorValue` (a fixed-width bitvector), `pack_bits` (canonical boolean
//! array to bitvector conversion), and `Environment` (variable to value mapping).

use sir_transform::ids::VariableId;
use std::collections::BTreeMap;

use crate::errors::InterpreterError;

/// The result type of the operational semantics (interpreter).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// A boolean value.
    Bool(bool),
    /// A fixed-width unsigned integer value.
    Integer(u64),
    /// A fixed-size array of boolean values.
    BooleanArray(Vec<bool>),
    /// A bitvector value with explicit width.
    BitVector(BitVectorValue),
}

/// A bitvector value with explicit width.
///
/// Width is semantically significant — two bitvectors with
/// the same bits but different widths are different values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitVectorValue {
    pub bits: u128,
    pub width: usize,
}

impl BitVectorValue {
    /// Create a new bitvector value.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `width` exceeds 128 (u128 capacity).
    pub fn new(bits: u128, width: usize) -> Self {
        debug_assert!(
            width <= 128,
            "BitVectorValue width {} exceeds u128 capacity",
            width
        );
        Self { bits, width }
    }
}

/// Pack a boolean array into a bitvector.
///
/// Bit i of the resulting `BitVector` = element i of the input array
/// (little-endian bit numbering: element 0 → bit 0).
///
/// Host-endianness independence: bit ordering is defined purely in terms
/// of bit shifts (`1 << i`), never memory-casting or pointer transmutation.
/// This ensures identical results on all architectures.
///
/// `width = bits.len()`
/// Unused high bits (beyond width) in the u128 are zero.
///
/// This is the canonical bit-ordering. Any change to this specification
/// would invalidate all proofs that involve `Pack` or `Popcount`.
///
/// # Errors
///
/// Returns `InterpreterError::InputTooLarge` if the input length exceeds 128
/// (u128 capacity).
pub fn pack_bits(bits: &[bool]) -> Result<BitVectorValue, InterpreterError> {
    let mut packed: u128 = 0;
    for (i, &bit) in bits.iter().enumerate() {
        if i >= 128 {
            return Err(InterpreterError::InputTooLarge {
                max: 128,
                found: bits.len(),
            });
        }
        if bit {
            packed |= 1u128 << i;
        }
    }
    Ok(BitVectorValue {
        bits: packed,
        width: bits.len(),
    })
}

/// Maps variables to their concrete values for a single test case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Environment {
    bindings: BTreeMap<VariableId, Value>,
}

impl Environment {
    /// Create an empty environment.
    pub fn new() -> Self {
        Self {
            bindings: BTreeMap::new(),
        }
    }

    /// Bind a variable to a value.
    pub fn bind(&mut self, id: VariableId, value: Value) {
        self.bindings.insert(id, value);
    }

    /// Look up a variable's value.
    pub fn lookup(&self, id: VariableId) -> Option<&Value> {
        self.bindings.get(&id)
    }

    /// Returns true if no variables are bound.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_bits_empty_array() {
        let result = pack_bits(&[]).unwrap();
        assert_eq!(result.bits, 0);
        assert_eq!(result.width, 0);
    }

    #[test]
    fn pack_bits_all_false() {
        let result = pack_bits(&[false, false, false, false]).unwrap();
        assert_eq!(result.bits, 0);
        assert_eq!(result.width, 4);
    }

    #[test]
    fn pack_bits_all_true() {
        let result = pack_bits(&[true, true, true, true]).unwrap();
        assert_eq!(result.bits, 0b1111);
        assert_eq!(result.width, 4);
    }

    #[test]
    fn pack_bits_bit_ordering() {
        // bit 0 = element 0
        let result = pack_bits(&[true, false, true, false]).unwrap();
        assert_eq!(result.bits, 0b0101); // bits: 0=1, 1=0, 2=1, 3=0
        assert_eq!(result.width, 4);
    }

    #[test]
    fn pack_bits_mixed_pattern() {
        // Only element 0 and element 63 set
        let mut input = vec![false; 64];
        input[0] = true;
        input[63] = true;
        let result = pack_bits(&input).unwrap();
        assert_eq!(result.bits, 1 | (1u128 << 63));
        assert_eq!(result.width, 64);
    }

    #[test]
    fn pack_bits_too_large_returns_error() {
        let input = vec![false; 200];
        let result = pack_bits(&input);
        assert!(matches!(
            result,
            Err(InterpreterError::InputTooLarge {
                max: 128,
                found: 200
            })
        ));
    }

    #[test]
    fn bitvector_value_equality_uses_width() {
        let a = BitVectorValue { bits: 0, width: 4 };
        let b = BitVectorValue { bits: 0, width: 8 };
        assert_ne!(a, b, "Same bits but different widths must not be equal");
    }

    #[test]
    fn environment_bind_and_lookup() {
        let mut env = Environment::new();
        let vid = VariableId::new(0);
        env.bind(vid, Value::Integer(42));
        assert_eq!(env.lookup(vid), Some(&Value::Integer(42)));
    }

    #[test]
    fn environment_unbound_lookup() {
        let env = Environment::new();
        assert_eq!(env.lookup(VariableId::new(99)), None);
    }

    #[test]
    fn environment_is_empty() {
        let mut env = Environment::new();
        assert!(env.is_empty());
        env.bind(VariableId::new(0), Value::Bool(true));
        assert!(!env.is_empty());
    }

    #[test]
    fn environment_default_is_empty() {
        let env: Environment = Default::default();
        assert!(env.is_empty());
    }

    #[test]
    fn bitvector_value_new() {
        let bv = BitVectorValue::new(42, 16);
        assert_eq!(bv.bits, 42);
        assert_eq!(bv.width, 16);
    }
}
