//!
//! The type.
//!

pub mod builder;
pub mod variant;

use self::variant::Variant;
use crate::test::function_call::parser::lexical::Location;

///
/// The type.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type {
    /// The location of the syntax construction.
    pub location: Location,
    /// The type variant.
    pub variant: Variant,
}

impl Type {
    ///
    /// Creates a type.
    ///
    pub fn new(location: Location, variant: Variant) -> Self {
        Self { location, variant }
    }
}

impl ToString for Type {
    fn to_string(&self) -> String {
        match &self.variant {
            Variant::String => "string".to_owned(),
            Variant::Boolean => "bool".to_owned(),
            Variant::Address => "address".to_owned(),
            Variant::Function => "function".to_owned(),
            Variant::Bytes { byte_length } => match byte_length {
                Some(byte_length) => format!("bytes{byte_length}"),
                None => "bytes".to_owned(),
            },
            Variant::IntegerUnsigned { bit_length } => format!("uint{bit_length}"),
            Variant::IntegerSigned { bit_length } => format!("int{bit_length}"),
            Variant::Array { inner, size } => match size {
                Some(size) => format!("{}[{}]", inner.to_string(), size),
                None => format!("{}[]", inner.to_string()),
            },
            Variant::Tuple { inners } => {
                format!(
                    "({})",
                    inners
                        .iter()
                        .map(|r#type| r#type.to_string())
                        .collect::<Vec<String>>()
                        .join(",")
                )
            }
        }
    }
}
