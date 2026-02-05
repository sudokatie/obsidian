use crate::span::Span;
use std::fmt;

/// Token kinds for the Obsidian language.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Def,
    End,
    If,
    Else,
    While,
    Do,
    Times,
    True,
    False,
    
    // Type keywords
    I32,
    I64,
    F32,
    F64,
    Bool,
    
    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    
    // Identifiers
    Ident(String),
    
    // Punctuation
    LParen,     // (
    RParen,     // )
    Colon,      // :
    DashDash,   // --
    
    // Arithmetic operators
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    
    // Comparison operators
    Eq,         // =
    NotEq,      // !=
    Lt,         // <
    Gt,         // >
    LtEq,       // <=
    GtEq,       // >=
    
    // Stack operations
    Dup,
    Drop,
    Swap,
    Over,
    Rot,
    Nip,
    Tuck,
    Dup2,
    Drop2,
    Swap2,
    
    // Logic operations
    And,
    Or,
    Not,
    
    // Bitwise operations
    Band,
    Bor,
    Bxor,
    Bnot,
    Shl,
    Shr,
    
    // Memory operations
    Fetch,      // @
    Store,      // !
    CFetch,     // c@
    CStore,     // c!
    Alloc,
    
    // IO operations
    Print,
    Emit,
    
    // Other builtins
    Negate,
    Abs,
    Min,
    Max,
    Mod,
    
    // Special
    Eof,
    Newline,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Def => write!(f, "def"),
            TokenKind::End => write!(f, "end"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::While => write!(f, "while"),
            TokenKind::Do => write!(f, "do"),
            TokenKind::Times => write!(f, "times"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::I32 => write!(f, "i32"),
            TokenKind::I64 => write!(f, "i64"),
            TokenKind::F32 => write!(f, "f32"),
            TokenKind::F64 => write!(f, "f64"),
            TokenKind::Bool => write!(f, "bool"),
            TokenKind::Integer(n) => write!(f, "{}", n),
            TokenKind::Float(n) => write!(f, "{}", n),
            TokenKind::String(s) => write!(f, "\"{}\"", s),
            TokenKind::Ident(s) => write!(f, "{}", s),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::DashDash => write!(f, "--"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::Eq => write!(f, "="),
            TokenKind::NotEq => write!(f, "!="),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::LtEq => write!(f, "<="),
            TokenKind::GtEq => write!(f, ">="),
            TokenKind::Dup => write!(f, "dup"),
            TokenKind::Drop => write!(f, "drop"),
            TokenKind::Swap => write!(f, "swap"),
            TokenKind::Over => write!(f, "over"),
            TokenKind::Rot => write!(f, "rot"),
            TokenKind::Nip => write!(f, "nip"),
            TokenKind::Tuck => write!(f, "tuck"),
            TokenKind::Dup2 => write!(f, "2dup"),
            TokenKind::Drop2 => write!(f, "2drop"),
            TokenKind::Swap2 => write!(f, "2swap"),
            TokenKind::And => write!(f, "and"),
            TokenKind::Or => write!(f, "or"),
            TokenKind::Not => write!(f, "not"),
            TokenKind::Band => write!(f, "band"),
            TokenKind::Bor => write!(f, "bor"),
            TokenKind::Bxor => write!(f, "bxor"),
            TokenKind::Bnot => write!(f, "bnot"),
            TokenKind::Shl => write!(f, "shl"),
            TokenKind::Shr => write!(f, "shr"),
            TokenKind::Fetch => write!(f, "@"),
            TokenKind::Store => write!(f, "!"),
            TokenKind::CFetch => write!(f, "c@"),
            TokenKind::CStore => write!(f, "c!"),
            TokenKind::Alloc => write!(f, "alloc"),
            TokenKind::Print => write!(f, "print"),
            TokenKind::Emit => write!(f, "emit"),
            TokenKind::Negate => write!(f, "negate"),
            TokenKind::Abs => write!(f, "abs"),
            TokenKind::Min => write!(f, "min"),
            TokenKind::Max => write!(f, "max"),
            TokenKind::Mod => write!(f, "mod"),
            TokenKind::Eof => write!(f, "EOF"),
            TokenKind::Newline => write!(f, "newline"),
        }
    }
}

/// A token with its source location.
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    /// Create a new token.
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_token_kind_display() {
        assert_eq!(format!("{}", TokenKind::Def), "def");
        assert_eq!(format!("{}", TokenKind::Plus), "+");
        assert_eq!(format!("{}", TokenKind::Integer(42)), "42");
        assert_eq!(format!("{}", TokenKind::String("hello".into())), "\"hello\"");
        assert_eq!(format!("{}", TokenKind::Ident("foo".into())), "foo");
    }
    
    #[test]
    fn test_token_new() {
        let token = Token::new(TokenKind::Def, Span::new(0, 3, 1, 1));
        assert_eq!(token.kind, TokenKind::Def);
        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 3);
    }
    
    #[test]
    fn test_token_kind_equality() {
        assert_eq!(TokenKind::Def, TokenKind::Def);
        assert_ne!(TokenKind::Def, TokenKind::End);
        assert_eq!(TokenKind::Integer(42), TokenKind::Integer(42));
        assert_ne!(TokenKind::Integer(42), TokenKind::Integer(43));
    }
}
