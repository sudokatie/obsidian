use crate::span::Span;

/// An import declaration.
#[derive(Debug, Clone)]
pub struct Import {
    /// Path to the imported file (relative or absolute)
    pub path: String,
    /// Optional alias/namespace
    pub alias: Option<String>,
    pub span: Span,
}

/// A complete Obsidian program.
#[derive(Debug, Clone)]
pub struct Program {
    pub imports: Vec<Import>,
    pub words: Vec<WordDef>,
}

impl Program {
    /// Create an empty program.
    pub fn new() -> Self {
        Self { 
            imports: Vec::new(),
            words: Vec::new(),
        }
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

/// A word definition (function).
#[derive(Debug, Clone)]
pub struct WordDef {
    pub name: String,
    pub effect: StackEffect,
    pub body: Vec<Expr>,
    pub span: Span,
}

/// Stack effect declaration: (inputs -- outputs)
#[derive(Debug, Clone)]
pub struct StackEffect {
    pub inputs: Vec<StackItem>,
    pub outputs: Vec<StackItem>,
}

impl StackEffect {
    /// Create an empty stack effect (--).
    pub fn empty() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }
    
    /// Net stack change (outputs - inputs).
    pub fn net_change(&self) -> isize {
        self.outputs.len() as isize - self.inputs.len() as isize
    }
}

/// A single item in a stack effect.
#[derive(Debug, Clone)]
pub struct StackItem {
    /// Optional name (e.g., "n" in "(n -- n)")
    pub name: Option<String>,
    /// Optional type annotation (e.g., i32 in "(n: i32 -- i32)")
    pub typ: Option<Type>,
}

impl StackItem {
    /// Create an untyped item with just a name.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            typ: None,
        }
    }
    
    /// Create a typed item.
    pub fn typed(name: Option<String>, typ: Type) -> Self {
        Self { name, typ: Some(typ) }
    }
    
    /// Create an anonymous typed item (for outputs like "-- i32").
    pub fn anonymous_typed(typ: Type) -> Self {
        Self { name: None, typ: Some(typ) }
    }
}

/// Primitive types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    I32,
    I64,
    F32,
    F64,
    Bool,
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            Type::Bool => write!(f, "bool"),
        }
    }
}

/// An expression in the body of a word.
#[derive(Debug, Clone)]
pub enum Expr {
    /// A literal value.
    Literal(Literal),
    
    /// A word call (built-in or user-defined).
    Word { name: String, span: Span },
    
    /// Conditional: if ... end or if ... else ... end
    If {
        then_branch: Vec<Expr>,
        else_branch: Option<Vec<Expr>>,
        span: Span,
    },
    
    /// While loop: while <cond> do ... end
    While {
        cond: Vec<Expr>,
        body: Vec<Expr>,
        span: Span,
    },
    
    /// Times loop: <n> times ... end
    Times {
        body: Vec<Expr>,
        span: Span,
    },
}

/// A literal value.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

impl Literal {
    /// Get the type of this literal.
    pub fn typ(&self) -> Type {
        match self {
            Literal::Integer(_) => Type::I64, // Default to i64, can be narrowed later
            Literal::Float(_) => Type::F64,
            Literal::String(_) => Type::I32, // String pointer is i32
            Literal::Bool(_) => Type::Bool,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_program_new() {
        let prog = Program::new();
        assert!(prog.imports.is_empty());
        assert!(prog.words.is_empty());
    }
    
    #[test]
    fn test_import() {
        let import = Import {
            path: "std/math.obs".to_string(),
            alias: Some("math".to_string()),
            span: Span::default(),
        };
        assert_eq!(import.path, "std/math.obs");
        assert_eq!(import.alias, Some("math".to_string()));
    }
    
    #[test]
    fn test_stack_effect_empty() {
        let effect = StackEffect::empty();
        assert!(effect.inputs.is_empty());
        assert!(effect.outputs.is_empty());
        assert_eq!(effect.net_change(), 0);
    }
    
    #[test]
    fn test_stack_effect_net_change() {
        let effect = StackEffect {
            inputs: vec![StackItem::named("a"), StackItem::named("b")],
            outputs: vec![StackItem::named("c")],
        };
        assert_eq!(effect.net_change(), -1);
    }
    
    #[test]
    fn test_stack_item_named() {
        let item = StackItem::named("foo");
        assert_eq!(item.name, Some("foo".to_string()));
        assert_eq!(item.typ, None);
    }
    
    #[test]
    fn test_stack_item_typed() {
        let item = StackItem::typed(Some("n".to_string()), Type::I32);
        assert_eq!(item.name, Some("n".to_string()));
        assert_eq!(item.typ, Some(Type::I32));
    }
    
    #[test]
    fn test_type_display() {
        assert_eq!(format!("{}", Type::I32), "i32");
        assert_eq!(format!("{}", Type::F64), "f64");
        assert_eq!(format!("{}", Type::Bool), "bool");
    }
    
    #[test]
    fn test_literal_typ() {
        assert_eq!(Literal::Integer(42).typ(), Type::I64);
        assert_eq!(Literal::Float(2.5).typ(), Type::F64);
        assert_eq!(Literal::Bool(true).typ(), Type::Bool);
    }
    
    #[test]
    fn test_word_def() {
        let word = WordDef {
            name: "square".to_string(),
            effect: StackEffect {
                inputs: vec![StackItem::named("n")],
                outputs: vec![StackItem::named("n")],
            },
            body: vec![
                Expr::Word { name: "dup".to_string(), span: Span::default() },
                Expr::Word { name: "*".to_string(), span: Span::default() },
            ],
            span: Span::default(),
        };
        assert_eq!(word.name, "square");
        assert_eq!(word.effect.net_change(), 0);
        assert_eq!(word.body.len(), 2);
    }
}
