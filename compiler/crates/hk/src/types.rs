use std::fmt;

use crate::ast::TypeName;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Type {
    Number,
    String,
    Bool,
    Null,
    Array,
    Any,
}

impl From<TypeName> for Type {
    fn from(value: TypeName) -> Self {
        match value {
            TypeName::Number => Self::Number,
            TypeName::String => Self::String,
            TypeName::Bool => Self::Bool,
            TypeName::Null => Self::Null,
            TypeName::Array => Self::Array,
            TypeName::Any => Self::Any,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Number => "number",
            Self::String => "string",
            Self::Bool => "bool",
            Self::Null => "null",
            Self::Array => "array",
            Self::Any => "any",
        };
        f.write_str(name)
    }
}

impl Type {
    pub(crate) fn accepts(self, actual: Self) -> bool {
        self == Self::Any || actual == Self::Any || self == actual
    }
}
