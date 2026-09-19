use serde::{Deserialize, Serialize};

/// The width of an integer type in bits.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntegerWidth {
    I8,
    I16,
    I32,
    I64,
    I128,
}

impl IntegerWidth {
    /// Return the width in bits.
    pub fn bits(self) -> usize {
        match self {
            IntegerWidth::I8 => 8,
            IntegerWidth::I16 => 16,
            IntegerWidth::I32 => 32,
            IntegerWidth::I64 => 64,
            IntegerWidth::I128 => 128,
        }
    }
}

impl std::fmt::Display for IntegerWidth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegerWidth::I8 => write!(f, "i8"),
            IntegerWidth::I16 => write!(f, "i16"),
            IntegerWidth::I32 => write!(f, "i32"),
            IntegerWidth::I64 => write!(f, "i64"),
            IntegerWidth::I128 => write!(f, "i128"),
        }
    }
}

/// The width of a floating-point type in bits.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FloatWidth {
    F32,
    F64,
}

impl FloatWidth {
    /// Return the width in bits.
    pub fn bits(self) -> usize {
        match self {
            FloatWidth::F32 => 32,
            FloatWidth::F64 => 64,
        }
    }
}

impl std::fmt::Display for FloatWidth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FloatWidth::F32 => write!(f, "f32"),
            FloatWidth::F64 => write!(f, "f64"),
        }
    }
}

/// Overflow behavior for integer arithmetic.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OverflowBehavior {
    /// Wrap on overflow (standard two's complement wrapping).
    Wrapping,
    /// Saturate at the minimum/maximum value.
    Saturating,
    /// Overflow is undefined behavior (enables optimizations, matches C semantics).
    Unchecked,
}

impl std::fmt::Display for OverflowBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OverflowBehavior::Wrapping => write!(f, "wrapping"),
            OverflowBehavior::Saturating => write!(f, "saturating"),
            OverflowBehavior::Unchecked => write!(f, "unchecked"),
        }
    }
}

/// The type of a value in the SIR.
///
/// Every node has an associated `Type`. This enum covers all types
/// that the IR can represent. Recursive variants (Pointer, Reference,
/// Array, Slice, Function) use `Box<Type>` to keep the enum size bounded.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Type {
    /// The unit type — `()`. Used for side-effecting operations.
    Unit,
    /// Boolean type — `true` or `false`.
    Bool,
    /// Integer type with width, signedness, and overflow semantics.
    Integer {
        width: IntegerWidth,
        signed: bool,
        overflow: OverflowBehavior,
    },
    /// Floating-point type with width.
    Float { width: FloatWidth },
    /// Raw pointer type. Pointee is boxed for size.
    Pointer { pointee: Box<Type>, mutable: bool },
    /// Reference type (borrow). Pointee is boxed. Lifetime is optional.
    Reference {
        pointee: Box<Type>,
        mutable: bool,
        lifetime: Option<String>,
    },
    /// Fixed-size array type.
    Array { element: Box<Type>, length: usize },
    /// Dynamically-sized slice type.
    Slice { element: Box<Type> },
    /// Tuple type with ordered element types.
    Tuple { elements: Vec<Type> },
    /// Named struct type with named fields.
    Struct {
        name: String,
        fields: Vec<(String, Type)>,
    },
    /// Named enum type with named variants (each variant holds zero or more types).
    Enum {
        name: String,
        variants: Vec<(String, Vec<Type>)>,
    },
    /// Function pointer type (params → return).
    Function { params: Vec<Type>, ret: Box<Type> },
    /// Bit-vector type of arbitrary width (not an integer — no arithmetic semantics).
    BitVector { width: usize },
}

impl Type {
    /// Create a signed integer type with the given width and wrapping overflow.
    pub fn i8() -> Self {
        Type::Integer {
            width: IntegerWidth::I8,
            signed: true,
            overflow: OverflowBehavior::Wrapping,
        }
    }

    /// Create a signed integer type.
    pub fn i16() -> Self {
        Type::Integer {
            width: IntegerWidth::I16,
            signed: true,
            overflow: OverflowBehavior::Wrapping,
        }
    }

    /// Create a signed integer type.
    pub fn i32() -> Self {
        Type::Integer {
            width: IntegerWidth::I32,
            signed: true,
            overflow: OverflowBehavior::Wrapping,
        }
    }

    /// Create a signed integer type.
    pub fn i64() -> Self {
        Type::Integer {
            width: IntegerWidth::I64,
            signed: true,
            overflow: OverflowBehavior::Wrapping,
        }
    }

    /// Create an unsigned integer type with the given width and wrapping overflow.
    pub fn u8() -> Self {
        Type::Integer {
            width: IntegerWidth::I8,
            signed: false,
            overflow: OverflowBehavior::Wrapping,
        }
    }

    /// Create an unsigned integer type.
    pub fn u16() -> Self {
        Type::Integer {
            width: IntegerWidth::I16,
            signed: false,
            overflow: OverflowBehavior::Wrapping,
        }
    }

    /// Create an unsigned integer type.
    pub fn u32() -> Self {
        Type::Integer {
            width: IntegerWidth::I32,
            signed: false,
            overflow: OverflowBehavior::Wrapping,
        }
    }

    /// Create an unsigned integer type.
    pub fn u64() -> Self {
        Type::Integer {
            width: IntegerWidth::I64,
            signed: false,
            overflow: OverflowBehavior::Wrapping,
        }
    }
    /// Check if this type is an integer type.
    pub fn is_integer(&self) -> bool {
        matches!(self, Type::Integer { .. })
    }

    /// Check if this type is an integer or bitvector type.
    pub fn is_integer_or_bitvector(&self) -> bool {
        if matches!(self, Type::BitVector { .. }) {
            true
        } else {
            matches!(self, Type::Integer { .. })
        }
    }

    /// Check if this type is a float type.
    pub fn is_float(&self) -> bool {
        matches!(self, Type::Float { .. })
    }

    /// Check if this type is numeric (integer or float).
    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    /// Check if this type is a pointer type (raw pointer or reference).
    pub fn is_pointer_like(&self) -> bool {
        matches!(self, Type::Pointer { .. } | Type::Reference { .. })
    }

    /// Check if this type is Bool.
    pub fn is_bool(&self) -> bool {
        matches!(self, Type::Bool)
    }
    /// Return a human-readable name for the type.
    pub fn type_name(&self) -> String {
        match self {
            Type::Unit => "()".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Integer {
                width,
                signed,
                overflow,
            } => {
                let sign = if *signed { "i" } else { "u" };
                format!("{sign}{} ({overflow})", width.bits())
            }
            Type::Float { width } => format!("f{}", width.bits()),
            Type::Pointer { pointee, mutable } => {
                let m = if *mutable { "mut " } else { "const " };
                format!("*{m}{}", pointee.type_name())
            }
            Type::Reference {
                pointee,
                mutable,
                lifetime,
            } => {
                let m = if *mutable { "mut " } else { "" };
                if let Some(lt) = lifetime {
                    format!("&'{lt} {m}{}", pointee.type_name())
                } else {
                    format!("&{m}{}", pointee.type_name())
                }
            }
            Type::Array { element, length } => format!("[{}; {length}]", element.type_name()),
            Type::Slice { element } => format!("[{}]", element.type_name()),
            Type::Tuple { elements } => {
                let elems: Vec<String> = elements.iter().map(|e| e.type_name()).collect();
                format!("({})", elems.join(", "))
            }
            Type::Struct { name, .. } => name.clone(),
            Type::Enum { name, .. } => name.clone(),
            Type::Function { params, ret } => {
                let p: Vec<String> = params.iter().map(|p| p.type_name()).collect();
                format!("fn({}) -> {}", p.join(", "), ret.type_name())
            }
            Type::BitVector { width } => format!("bv{width}"),
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.type_name())
    }
}
