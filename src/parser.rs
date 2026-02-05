use crate::ast::{Expr, Literal, Program, StackEffect, StackItem, Type, WordDef};
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// Parser error.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
    pub expected: Option<String>,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(expected) = &self.expected {
            write!(f, "{}:{}: expected {}, {}", self.span.line, self.span.col, expected, self.message)
        } else {
            write!(f, "{}:{}: {}", self.span.line, self.span.col, self.message)
        }
    }
}

impl std::error::Error for ParseError {}

/// Parser for Obsidian tokens.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    /// Create a new parser.
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }
    
    /// Parse the token stream into a program.
    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let mut program = Program::new();
        
        while !self.at_end() {
            if self.check(&TokenKind::Eof) {
                break;
            }
            let word = self.parse_word_def()?;
            program.words.push(word);
        }
        
        Ok(program)
    }
    
    /// Parse a word definition.
    fn parse_word_def(&mut self) -> Result<WordDef, ParseError> {
        let start_span = self.peek().span;
        
        self.expect(&TokenKind::Def)?;
        
        let name = self.expect_ident()?;
        let effect = self.parse_stack_effect()?;
        let body = self.parse_body(&[TokenKind::End])?;
        
        self.expect(&TokenKind::End)?;
        
        let end_span = self.previous().span;
        
        Ok(WordDef {
            name,
            effect,
            body,
            span: start_span.merge(end_span),
        })
    }
    
    /// Parse a stack effect: (inputs -- outputs)
    fn parse_stack_effect(&mut self) -> Result<StackEffect, ParseError> {
        self.expect(&TokenKind::LParen)?;
        
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut in_outputs = false;
        
        while !self.check(&TokenKind::RParen) && !self.at_end() {
            if self.check(&TokenKind::DashDash) {
                self.advance();
                in_outputs = true;
                continue;
            }
            
            let item = self.parse_stack_item()?;
            
            if in_outputs {
                outputs.push(item);
            } else {
                inputs.push(item);
            }
        }
        
        self.expect(&TokenKind::RParen)?;
        
        Ok(StackEffect { inputs, outputs })
    }
    
    /// Parse a stack item: name or name: type or just type
    fn parse_stack_item(&mut self) -> Result<StackItem, ParseError> {
        // Could be: name, name: type, or just type
        if let Some(typ) = self.try_parse_type() {
            // Just a type
            return Ok(StackItem::anonymous_typed(typ));
        }
        
        let name = self.expect_ident()?;
        
        if self.check(&TokenKind::Colon) {
            self.advance();
            let typ = self.parse_type()?;
            Ok(StackItem::typed(Some(name), typ))
        } else {
            Ok(StackItem::named(name))
        }
    }
    
    /// Try to parse a type, returning None if not a type keyword.
    fn try_parse_type(&mut self) -> Option<Type> {
        let typ = match self.peek().kind {
            TokenKind::I32 => Type::I32,
            TokenKind::I64 => Type::I64,
            TokenKind::F32 => Type::F32,
            TokenKind::F64 => Type::F64,
            TokenKind::Bool => Type::Bool,
            _ => return None,
        };
        self.advance();
        Some(typ)
    }
    
    /// Parse a type (required).
    fn parse_type(&mut self) -> Result<Type, ParseError> {
        self.try_parse_type().ok_or_else(|| ParseError {
            message: format!("got {:?}", self.peek().kind),
            span: self.peek().span,
            expected: Some("type (i32, i64, f32, f64, bool)".to_string()),
        })
    }
    
    /// Parse a body (list of expressions) until one of the terminators.
    fn parse_body(&mut self, terminators: &[TokenKind]) -> Result<Vec<Expr>, ParseError> {
        let mut exprs = Vec::new();
        
        while !self.at_end() && !terminators.iter().any(|t| self.check(t)) {
            let expr = self.parse_expr()?;
            exprs.push(expr);
        }
        
        Ok(exprs)
    }
    
    /// Parse a single expression.
    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let token = self.peek().clone();
        
        match &token.kind {
            // Literals
            TokenKind::Integer(n) => {
                let n = *n;
                self.advance();
                Ok(Expr::Literal(Literal::Integer(n)))
            }
            TokenKind::Float(f) => {
                let f = *f;
                self.advance();
                Ok(Expr::Literal(Literal::Float(f)))
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::Literal(Literal::String(s)))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(true)))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(false)))
            }
            
            // Control flow
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::Times => self.parse_times(),
            
            // Word calls (includes builtins and user-defined)
            TokenKind::Ident(name) => {
                let name = name.clone();
                let span = token.span;
                self.advance();
                Ok(Expr::Word { name, span })
            }
            
            // Built-in operations as words
            TokenKind::Plus => self.word_expr("+"),
            TokenKind::Minus => self.word_expr("-"),
            TokenKind::Star => self.word_expr("*"),
            TokenKind::Slash => self.word_expr("/"),
            TokenKind::Percent => self.word_expr("%"),
            TokenKind::Eq => self.word_expr("="),
            TokenKind::NotEq => self.word_expr("!="),
            TokenKind::Lt => self.word_expr("<"),
            TokenKind::Gt => self.word_expr(">"),
            TokenKind::LtEq => self.word_expr("<="),
            TokenKind::GtEq => self.word_expr(">="),
            TokenKind::Dup => self.word_expr("dup"),
            TokenKind::Drop => self.word_expr("drop"),
            TokenKind::Swap => self.word_expr("swap"),
            TokenKind::Over => self.word_expr("over"),
            TokenKind::Rot => self.word_expr("rot"),
            TokenKind::Nip => self.word_expr("nip"),
            TokenKind::Tuck => self.word_expr("tuck"),
            TokenKind::Dup2 => self.word_expr("2dup"),
            TokenKind::Drop2 => self.word_expr("2drop"),
            TokenKind::Swap2 => self.word_expr("2swap"),
            TokenKind::And => self.word_expr("and"),
            TokenKind::Or => self.word_expr("or"),
            TokenKind::Not => self.word_expr("not"),
            TokenKind::Band => self.word_expr("band"),
            TokenKind::Bor => self.word_expr("bor"),
            TokenKind::Bxor => self.word_expr("bxor"),
            TokenKind::Bnot => self.word_expr("bnot"),
            TokenKind::Shl => self.word_expr("shl"),
            TokenKind::Shr => self.word_expr("shr"),
            TokenKind::Fetch => self.word_expr("@"),
            TokenKind::Store => self.word_expr("!"),
            TokenKind::CFetch => self.word_expr("c@"),
            TokenKind::CStore => self.word_expr("c!"),
            TokenKind::Alloc => self.word_expr("alloc"),
            TokenKind::Print => self.word_expr("print"),
            TokenKind::Emit => self.word_expr("emit"),
            TokenKind::Negate => self.word_expr("negate"),
            TokenKind::Abs => self.word_expr("abs"),
            TokenKind::Min => self.word_expr("min"),
            TokenKind::Max => self.word_expr("max"),
            TokenKind::Mod => self.word_expr("mod"),
            
            _ => Err(ParseError {
                message: format!("unexpected token {:?}", token.kind),
                span: token.span,
                expected: Some("expression".to_string()),
            }),
        }
    }
    
    /// Create a word expression from a builtin name.
    fn word_expr(&mut self, name: &str) -> Result<Expr, ParseError> {
        let span = self.peek().span;
        self.advance();
        Ok(Expr::Word { name: name.to_string(), span })
    }
    
    /// Parse an if expression.
    fn parse_if(&mut self) -> Result<Expr, ParseError> {
        let start_span = self.peek().span;
        self.expect(&TokenKind::If)?;
        
        let then_branch = self.parse_body(&[TokenKind::Else, TokenKind::End])?;
        
        let else_branch = if self.check(&TokenKind::Else) {
            self.advance();
            Some(self.parse_body(&[TokenKind::End])?)
        } else {
            None
        };
        
        self.expect(&TokenKind::End)?;
        let end_span = self.previous().span;
        
        Ok(Expr::If {
            then_branch,
            else_branch,
            span: start_span.merge(end_span),
        })
    }
    
    /// Parse a while expression.
    fn parse_while(&mut self) -> Result<Expr, ParseError> {
        let start_span = self.peek().span;
        self.expect(&TokenKind::While)?;
        
        let cond = self.parse_body(&[TokenKind::Do])?;
        
        self.expect(&TokenKind::Do)?;
        
        let body = self.parse_body(&[TokenKind::End])?;
        
        self.expect(&TokenKind::End)?;
        let end_span = self.previous().span;
        
        Ok(Expr::While {
            cond,
            body,
            span: start_span.merge(end_span),
        })
    }
    
    /// Parse a times expression.
    fn parse_times(&mut self) -> Result<Expr, ParseError> {
        let start_span = self.peek().span;
        self.expect(&TokenKind::Times)?;
        
        let body = self.parse_body(&[TokenKind::End])?;
        
        self.expect(&TokenKind::End)?;
        let end_span = self.previous().span;
        
        Ok(Expr::Times {
            body,
            span: start_span.merge(end_span),
        })
    }
    
    // Helper methods
    
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }
    
    fn previous(&self) -> &Token {
        &self.tokens[self.pos.saturating_sub(1)]
    }
    
    fn advance(&mut self) -> &Token {
        if !self.at_end() {
            self.pos += 1;
        }
        self.previous()
    }
    
    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len() || matches!(self.peek().kind, TokenKind::Eof)
    }
    
    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }
    
    fn expect(&mut self, kind: &TokenKind) -> Result<&Token, ParseError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(ParseError {
                message: format!("got {:?}", self.peek().kind),
                span: self.peek().span,
                expected: Some(format!("{:?}", kind)),
            })
        }
    }
    
    fn expect_ident(&mut self) -> Result<String, ParseError> {
        if let TokenKind::Ident(name) = &self.peek().kind {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            Err(ParseError {
                message: format!("got {:?}", self.peek().kind),
                span: self.peek().span,
                expected: Some("identifier".to_string()),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    
    fn parse(source: &str) -> Result<Program, ParseError> {
        let tokens = Lexer::new(source).tokenize().unwrap();
        Parser::new(tokens).parse()
    }
    
    #[test]
    fn test_empty_program() {
        let prog = parse("").unwrap();
        assert!(prog.words.is_empty());
    }
    
    #[test]
    fn test_simple_word() {
        let prog = parse("def foo (--) end").unwrap();
        assert_eq!(prog.words.len(), 1);
        assert_eq!(prog.words[0].name, "foo");
        assert!(prog.words[0].effect.inputs.is_empty());
        assert!(prog.words[0].effect.outputs.is_empty());
        assert!(prog.words[0].body.is_empty());
    }
    
    #[test]
    fn test_word_with_effect() {
        let prog = parse("def square (n -- n) end").unwrap();
        let word = &prog.words[0];
        assert_eq!(word.effect.inputs.len(), 1);
        assert_eq!(word.effect.outputs.len(), 1);
        assert_eq!(word.effect.inputs[0].name, Some("n".to_string()));
    }
    
    #[test]
    fn test_typed_effect() {
        let prog = parse("def add (a: i32 b: i32 -- i32) end").unwrap();
        let word = &prog.words[0];
        assert_eq!(word.effect.inputs.len(), 2);
        assert_eq!(word.effect.inputs[0].typ, Some(Type::I32));
        assert_eq!(word.effect.outputs.len(), 1);
        assert_eq!(word.effect.outputs[0].typ, Some(Type::I32));
    }
    
    #[test]
    fn test_word_with_body() {
        let prog = parse("def square (n -- n) dup * end").unwrap();
        let word = &prog.words[0];
        assert_eq!(word.body.len(), 2);
    }
    
    #[test]
    fn test_literals() {
        let prog = parse("def test (--) 42 3.14 \"hello\" true false end").unwrap();
        let body = &prog.words[0].body;
        assert_eq!(body.len(), 5);
        assert!(matches!(&body[0], Expr::Literal(Literal::Integer(42))));
        assert!(matches!(&body[1], Expr::Literal(Literal::Float(_))));
        assert!(matches!(&body[2], Expr::Literal(Literal::String(s)) if s == "hello"));
        assert!(matches!(&body[3], Expr::Literal(Literal::Bool(true))));
        assert!(matches!(&body[4], Expr::Literal(Literal::Bool(false))));
    }
    
    #[test]
    fn test_operators() {
        let prog = parse("def test (--) + - * / = end").unwrap();
        let body = &prog.words[0].body;
        assert_eq!(body.len(), 5);
    }
    
    #[test]
    fn test_stack_ops() {
        let prog = parse("def test (--) dup drop swap over rot end").unwrap();
        let body = &prog.words[0].body;
        assert_eq!(body.len(), 5);
    }
    
    #[test]
    fn test_if_simple() {
        let prog = parse("def test (--) if 42 end end").unwrap();
        let body = &prog.words[0].body;
        assert_eq!(body.len(), 1);
        assert!(matches!(&body[0], Expr::If { else_branch: None, .. }));
    }
    
    #[test]
    fn test_if_else() {
        let prog = parse("def test (--) if 1 else 2 end end").unwrap();
        let body = &prog.words[0].body;
        assert!(matches!(&body[0], Expr::If { else_branch: Some(_), .. }));
    }
    
    #[test]
    fn test_while_loop() {
        let prog = parse("def test (--) while dup 0 > do 1 - end end").unwrap();
        let body = &prog.words[0].body;
        assert_eq!(body.len(), 1);
        if let Expr::While { cond, body: while_body, .. } = &body[0] {
            assert_eq!(cond.len(), 3); // dup 0 >
            assert_eq!(while_body.len(), 2); // 1 -
        } else {
            panic!("expected while");
        }
    }
    
    #[test]
    fn test_times_loop() {
        let prog = parse("def test (--) times 1 + end end").unwrap();
        let body = &prog.words[0].body;
        assert_eq!(body.len(), 1);
        if let Expr::Times { body: times_body, .. } = &body[0] {
            assert_eq!(times_body.len(), 2); // 1 +
        } else {
            panic!("expected times");
        }
    }
    
    #[test]
    fn test_nested_if() {
        let prog = parse("def test (--) if if 1 end else 2 end end").unwrap();
        let body = &prog.words[0].body;
        assert_eq!(body.len(), 1);
    }
    
    #[test]
    fn test_multiple_words() {
        let prog = parse("def foo (--) end def bar (--) end").unwrap();
        assert_eq!(prog.words.len(), 2);
        assert_eq!(prog.words[0].name, "foo");
        assert_eq!(prog.words[1].name, "bar");
    }
    
    #[test]
    fn test_factorial() {
        let source = r#"
            def factorial (n -- result)
                1 swap
                while dup 0 > do
                    swap over *
                    swap 1 -
                end
                drop
            end
        "#;
        let prog = parse(source).unwrap();
        assert_eq!(prog.words.len(), 1);
        assert_eq!(prog.words[0].name, "factorial");
    }
    
    #[test]
    fn test_error_missing_end() {
        let result = parse("def foo (--)");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_error_missing_name() {
        let result = parse("def (--) end");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_error_bad_effect() {
        let result = parse("def foo -- end");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_word_call() {
        let prog = parse("def test (--) foo bar end").unwrap();
        let body = &prog.words[0].body;
        assert_eq!(body.len(), 2);
        assert!(matches!(&body[0], Expr::Word { name, .. } if name == "foo"));
        assert!(matches!(&body[1], Expr::Word { name, .. } if name == "bar"));
    }
    
    #[test]
    fn test_builtins() {
        let prog = parse("def test (--) print emit negate abs min max mod end").unwrap();
        assert_eq!(prog.words[0].body.len(), 7);
    }
    
    #[test]
    fn test_bitwise() {
        let prog = parse("def test (--) band bor bxor bnot shl shr end").unwrap();
        assert_eq!(prog.words[0].body.len(), 6);
    }
    
    #[test]
    fn test_memory_ops() {
        let prog = parse("def test (--) @ ! alloc end").unwrap();
        assert_eq!(prog.words[0].body.len(), 3);
    }
    
    #[test]
    fn test_comparison_ops() {
        let prog = parse("def test (--) = != < > <= >= end").unwrap();
        assert_eq!(prog.words[0].body.len(), 6);
    }
    
    #[test]
    fn test_2dup_2drop_2swap() {
        let prog = parse("def test (--) 2dup 2drop 2swap end").unwrap();
        assert_eq!(prog.words[0].body.len(), 3);
    }
    
    #[test]
    fn test_cfetch_cstore() {
        let prog = parse("def test (--) c@ c! end").unwrap();
        assert_eq!(prog.words[0].body.len(), 2);
    }
}
