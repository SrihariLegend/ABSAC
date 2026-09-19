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
    /// Create a boolean constant.
    pub fn boolean(v: bool) -> Self {
        ConstantData::Bool(v)
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

