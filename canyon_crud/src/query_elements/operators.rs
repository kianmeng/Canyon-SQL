pub trait Operator {
    fn as_str(&self) -> &'static str;
}

/// Enumerated type for represent the comparison operations
/// in SQL sentences
pub enum Comp {
    /// Operator "=" equals
    Eq,
    /// Operator "!=" not equals
    Neq,
    /// Operator ">" greater than value
    Gt,
    /// Operator ">=" greater or equals than value
    GtEq,
    /// Operator "<" less than value
    Lt,
    /// Operator "=<" less or equals than value
    LtEq,
}
impl Operator for Comp {
    fn as_str(&self) -> &'static str {
        match *self {
            Self::Eq => " = ",
            Self::Neq => " <> ",
            Self::Gt => " > ",
            Self::GtEq => " >= ",
            Self::Lt => " < ",
            Self::LtEq => " <= ",
        }
    }
}
