use serde::{Deserialize, Serialize};

use crate::types::{FloatWidth, IntegerWidth};

/// A constant data value.
///
/// Integer and float values are stored as strings to preserve precision
/// for arbitrarily wide types (i128, u128) and to avoid floating-point
/// rounding issues during serialization.
///
/// Numeric consumers must parse the strings when they need actual values.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConstantData {
    /// The unit constant `()`.
    Unit,
    /// A boolean constant.
    Bool(bool),
    /// An integer constant. Value is stored as a decimal string.
    Integer {
        value: String,
        width: IntegerWidth,
        signed: bool,
    },
    /// A floating-point constant. Value is stored as a decimal string.
    Float { value: String, width: FloatWidth },
    /// A string literal constant.
    StringLiteral(String),
}

impl ConstantData {
    /// Create a signed integer constant from an i64.
    pub fn i8(v: i8) -> Self {
        ConstantData::Integer {
            value: v.to_string(),
            width: IntegerWidth::I8,
            signed: true,
        }
    }

    /// Create a signed integer constant.
    pub fn i16(v: i16) -> Self {
        ConstantData::Integer {
            value: v.to_string(),
            width: IntegerWidth::I16,
            signed: true,
        }
    }

    /// Create a signed integer constant.
    pub fn i32(v: i32) -> Self {
        ConstantData::Integer {
            value: v.to_string(),
            width: IntegerWidth::I32,
            signed: true,
        }
    }

    /// Create a signed integer constant.
    pub fn i64(v: i64) -> Self {
        ConstantData::Integer {
            value: v.to_string(),
            width: IntegerWidth::I64,
            signed: true,
        }
    }

    /// Create an unsigned integer constant.
    pub fn u8(v: u8) -> Self {
        ConstantData::Integer {
            value: v.to_string(),
            width: IntegerWidth::I8,
            signed: false,
        }
    }

    /// Create an unsigned integer constant.
    pub fn u16(v: u16) -> Self {
        ConstantData::Integer {
            value: v.to_string(),
            width: IntegerWidth::I16,
            signed: false,
        }
    }

    /// Create an unsigned integer constant.
    pub fn u32(v: u32) -> Self {
        ConstantData::Integer {
            value: v.to_string(),
            width: IntegerWidth::I32,
            signed: false,
        }
    }

    /// Create an unsigned integer constant.
    pub fn u64(v: u64) -> Self {
        ConstantData::Integer {
            value: v.to_string(),
            width: IntegerWidth::I64,
            signed: false,
        }
    }

    /// Create a float constant from an f32.
    pub fn f32(v: f32) -> Self {
        ConstantData::Float {
            value: v.to_string(),
            width: FloatWidth::F32,
        }
    }

    /// Create a float constant from an f64.
    pub fn f64(v: f64) -> Self {
        ConstantData::Float {
            value: v.to_string(),
            width: FloatWidth::F64,
        }
    }

    /// Create a boolean constant.
    pub fn boolean(v: bool) -> Self {
        ConstantData::Bool(v)
    }

    /// Create a string literal constant.
    pub fn string(s: impl Into<String>) -> Self {
        ConstantData::StringLiteral(s.into())
    }

    /// Try to parse the value as an i64 (for signed integer constants).
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ConstantData::Integer {
                value,
                signed: true,
                ..
            } => value.parse().ok(),
            _ => None,
        }
    }

    /// Try to parse the value as a u64 (for unsigned integer constants).
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            ConstantData::Integer {
                value,
                signed: false,
                ..
            } => value.parse().ok(),
            _ => None,
        }
    }

    /// Try to parse the value as an f64 (for float constants).
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ConstantData::Float { value, .. } => value.parse().ok(),
            _ => None,
        }
    }
}

impl std::fmt::Display for ConstantData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstantData::Unit => write!(f, "()"),
            ConstantData::Bool(v) => write!(f, "{v}"),
            ConstantData::Integer {
                value,
                width,
                signed,
            } => {
                let sign = if *signed { "i" } else { "u" };
                write!(f, "{value}{sign}{}", width.bits())
            }
            ConstantData::Float { value, width } => write!(f, "{value}f{}", width.bits()),
            ConstantData::StringLiteral(s) => write!(f, "\"{s}\""),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_constant_display() {
        let c = ConstantData::i32(42);
        assert_eq!(format!("{c}"), "42i32");
    }

    #[test]
    fn unsigned_constant_display() {
        let c = ConstantData::u64(255);
        assert_eq!(format!("{c}"), "255u64");
    }

    #[test]
    fn float_constant_display() {
        let c = ConstantData::f64(3.14);
        assert_eq!(format!("{c}"), "3.14f64");
    }

    #[test]
    fn bool_constant() {
        let c = ConstantData::boolean(true);
        assert_eq!(format!("{c}"), "true");
    }

    #[test]
    fn unit_constant() {
        let c = ConstantData::Unit;
        assert_eq!(format!("{c}"), "()");
    }

    #[test]
    fn string_constant() {
        let c = ConstantData::string("hello");
        assert_eq!(format!("{c}"), "\"hello\"");
    }

    #[test]
    fn parse_i64() {
        let c = ConstantData::i64(-100);
        assert_eq!(c.as_i64(), Some(-100));
        assert_eq!(c.as_u64(), None); // signed constant → as_u64 returns None
    }

    #[test]
    fn parse_u64() {
        let c = ConstantData::u64(1000);
        assert_eq!(c.as_u64(), Some(1000));
        assert_eq!(c.as_i64(), None); // unsigned constant → as_i64 returns None
    }

    #[test]
    fn parse_f64() {
        let c = ConstantData::f64(3.14);
        assert_eq!(c.as_f64(), Some(3.14));
    }

    #[test]
    fn non_numeric_parse_returns_none() {
        assert_eq!(ConstantData::Bool(true).as_i64(), None);
        assert_eq!(ConstantData::Unit.as_u64(), None);
    }

    #[test]
    fn serde_roundtrip_integer() {
        let c = ConstantData::i32(42);
        let json = serde_json::to_string(&c).unwrap();
        let parsed: ConstantData = serde_json::from_str(&json).unwrap();
        assert_eq!(c, parsed);
    }

    #[test]
    fn serde_roundtrip_float() {
        let c = ConstantData::f64(1.5);
        let json = serde_json::to_string(&c).unwrap();
        let parsed: ConstantData = serde_json::from_str(&json).unwrap();
        assert_eq!(c, parsed);
    }
}
