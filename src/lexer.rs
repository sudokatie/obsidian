use crate::span::Span;
use crate::token::{Token, TokenKind};

/// Lexer error with location.
#[derive(Debug, Clone)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.span.line, self.span.col, self.message)
    }
}

impl std::error::Error for LexError {}

/// Lexer for Obsidian source code.
pub struct Lexer<'a> {
    #[allow(dead_code)]
    source: &'a str,
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given source.
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }
    
    /// Tokenize the entire source.
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        
        loop {
            self.skip_whitespace_and_comments();
            
            if self.at_end() {
                tokens.push(Token::new(
                    TokenKind::Eof,
                    Span::new(self.pos, self.pos, self.line, self.col),
                ));
                break;
            }
            
            let token = self.next_token()?;
            tokens.push(token);
        }
        
        Ok(tokens)
    }
    
    /// Read the next token.
    fn next_token(&mut self) -> Result<Token, LexError> {
        let start_pos = self.pos;
        let start_line = self.line;
        let start_col = self.col;
        
        let c = self.advance().unwrap();
        
        let kind = match c {
            // Single-character tokens
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            ':' => TokenKind::Colon,
            '+' => TokenKind::Plus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '@' => TokenKind::Fetch,
            
            // Could be minus, dash-dash, or comment start
            '-' => {
                if self.peek() == Some('-') {
                    self.advance();
                    // Check if this is a comment: -- followed by space, with no ) before newline
                    // (stack effects have ) at the end, comments don't)
                    if self.peek() == Some(' ') || self.peek() == Some('\t') {
                        // Look for ) before newline - if found, it's a stack effect
                        let mut has_rparen = false;
                        let mut peek_pos = self.pos;
                        while peek_pos < self.chars.len() {
                            let c = self.chars[peek_pos];
                            if c == ')' {
                                has_rparen = true;
                                break;
                            }
                            if c == '\n' {
                                break;
                            }
                            peek_pos += 1;
                        }
                        
                        if !has_rparen {
                            // No ) on this line - it's a comment, skip to end of line
                            while let Some(c) = self.peek() {
                                self.advance();
                                if c == '\n' {
                                    break;
                                }
                            }
                            // Skip whitespace and try to get next token
                            self.skip_whitespace_and_comments();
                            if self.at_end() {
                                return Ok(Token::new(
                                    TokenKind::Eof,
                                    Span::new(self.pos, self.pos, self.line, self.col),
                                ));
                            }
                            // Recursively get the next real token
                            return self.next_token();
                        }
                    }
                    TokenKind::DashDash
                } else {
                    TokenKind::Minus
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::NotEq
                } else {
                    TokenKind::Store
                }
            }
            '=' => TokenKind::Eq,
            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                }
            }
            
            // String literal
            '"' => return self.read_string(start_pos, start_line, start_col),
            
            // c@ and c! (byte fetch/store)
            'c' => {
                if self.peek() == Some('@') {
                    self.advance();
                    TokenKind::CFetch
                } else if self.peek() == Some('!') {
                    self.advance();
                    TokenKind::CStore
                } else {
                    return self.read_ident(c, start_pos, start_line, start_col);
                }
            }
            
            // Number literal (including 2dup, 2drop, 2swap)
            '0'..='9' => {
                // Check for 2dup, 2drop, 2swap
                if c == '2' {
                    let next_chars: String = self.chars[self.pos..].iter().take(4).collect();
                    if next_chars.starts_with("dup") && !self.chars.get(self.pos + 3).map(|c| c.is_alphanumeric() || *c == '_').unwrap_or(false) {
                        self.pos += 3;
                        self.col += 3;
                        return Ok(Token::new(TokenKind::Dup2, Span::new(start_pos, self.pos, start_line, start_col)));
                    } else if next_chars.starts_with("drop") && !self.chars.get(self.pos + 4).map(|c| c.is_alphanumeric() || *c == '_').unwrap_or(false) {
                        self.pos += 4;
                        self.col += 4;
                        return Ok(Token::new(TokenKind::Drop2, Span::new(start_pos, self.pos, start_line, start_col)));
                    } else if next_chars.starts_with("swap") && !self.chars.get(self.pos + 4).map(|c| c.is_alphanumeric() || *c == '_').unwrap_or(false) {
                        self.pos += 4;
                        self.col += 4;
                        return Ok(Token::new(TokenKind::Swap2, Span::new(start_pos, self.pos, start_line, start_col)));
                    }
                }
                return self.read_number(c, start_pos, start_line, start_col);
            }
            
            // Identifier or keyword
            'a'..='z' | 'A'..='Z' | '_' => {
                return self.read_ident(c, start_pos, start_line, start_col);
            }
            
            _ => {
                return Err(LexError {
                    message: format!("unexpected character '{}'", c),
                    span: Span::new(start_pos, self.pos, start_line, start_col),
                });
            }
        };
        
        Ok(Token::new(kind, Span::new(start_pos, self.pos, start_line, start_col)))
    }
    
    /// Read a string literal.
    fn read_string(&mut self, start_pos: usize, start_line: usize, start_col: usize) -> Result<Token, LexError> {
        let mut value = String::new();
        
        while let Some(c) = self.peek() {
            if c == '"' {
                self.advance();
                return Ok(Token::new(
                    TokenKind::String(value),
                    Span::new(start_pos, self.pos, start_line, start_col),
                ));
            }
            
            self.advance();
            
            if c == '\\' {
                // Escape sequence
                match self.peek() {
                    Some('n') => { self.advance(); value.push('\n'); }
                    Some('t') => { self.advance(); value.push('\t'); }
                    Some('r') => { self.advance(); value.push('\r'); }
                    Some('\\') => { self.advance(); value.push('\\'); }
                    Some('"') => { self.advance(); value.push('"'); }
                    Some(other) => {
                        return Err(LexError {
                            message: format!("invalid escape sequence '\\{}'", other),
                            span: Span::new(self.pos - 1, self.pos + 1, self.line, self.col - 1),
                        });
                    }
                    None => {
                        return Err(LexError {
                            message: "unterminated string".to_string(),
                            span: Span::new(start_pos, self.pos, start_line, start_col),
                        });
                    }
                }
            } else if c == '\n' {
                return Err(LexError {
                    message: "unterminated string (newline in string)".to_string(),
                    span: Span::new(start_pos, self.pos, start_line, start_col),
                });
            } else {
                value.push(c);
            }
        }
        
        Err(LexError {
            message: "unterminated string".to_string(),
            span: Span::new(start_pos, self.pos, start_line, start_col),
        })
    }
    
    /// Read a number literal (integer or float).
    fn read_number(&mut self, first: char, start_pos: usize, start_line: usize, start_col: usize) -> Result<Token, LexError> {
        let mut num_str = String::new();
        num_str.push(first);
        
        // Check for hex or binary prefix
        if first == '0' {
            match self.peek() {
                Some('x') | Some('X') => {
                    self.advance();
                    return self.read_hex(start_pos, start_line, start_col);
                }
                Some('b') | Some('B') => {
                    self.advance();
                    return self.read_binary(start_pos, start_line, start_col);
                }
                _ => {}
            }
        }
        
        // Read integer part
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                num_str.push(c);
                self.advance();
            } else {
                break;
            }
        }
        
        // Check for decimal point or exponent (makes it a float)
        let mut is_float = false;
        
        if self.peek() == Some('.') {
            // Look ahead to make sure it's followed by a digit
            if self.peek_next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                is_float = true;
                num_str.push('.');
                self.advance();
                
                // Read fractional part
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        num_str.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }
        
        // Check for exponent (with or without decimal point)
        if matches!(self.peek(), Some('e') | Some('E')) {
            is_float = true;
            num_str.push('e');
            self.advance();
            
            if matches!(self.peek(), Some('+') | Some('-')) {
                num_str.push(self.advance().unwrap());
            }
            
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    num_str.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
        }
        
        if is_float {
            let value: f64 = num_str.parse().map_err(|_| LexError {
                message: format!("invalid float literal '{}'", num_str),
                span: Span::new(start_pos, self.pos, start_line, start_col),
            })?;
            
            return Ok(Token::new(
                TokenKind::Float(value),
                Span::new(start_pos, self.pos, start_line, start_col),
            ));
        }
        
        // Parse as integer
        let value: i64 = num_str.parse().map_err(|_| LexError {
            message: format!("invalid integer literal '{}'", num_str),
            span: Span::new(start_pos, self.pos, start_line, start_col),
        })?;
        
        Ok(Token::new(
            TokenKind::Integer(value),
            Span::new(start_pos, self.pos, start_line, start_col),
        ))
    }
    
    /// Read a hexadecimal number (after 0x).
    fn read_hex(&mut self, start_pos: usize, start_line: usize, start_col: usize) -> Result<Token, LexError> {
        let mut hex_str = String::new();
        
        while let Some(c) = self.peek() {
            if c.is_ascii_hexdigit() {
                hex_str.push(c);
                self.advance();
            } else {
                break;
            }
        }
        
        if hex_str.is_empty() {
            return Err(LexError {
                message: "expected hex digits after 0x".to_string(),
                span: Span::new(start_pos, self.pos, start_line, start_col),
            });
        }
        
        let value = i64::from_str_radix(&hex_str, 16).map_err(|_| LexError {
            message: format!("invalid hex literal '0x{}'", hex_str),
            span: Span::new(start_pos, self.pos, start_line, start_col),
        })?;
        
        Ok(Token::new(
            TokenKind::Integer(value),
            Span::new(start_pos, self.pos, start_line, start_col),
        ))
    }
    
    /// Read a binary number (after 0b).
    fn read_binary(&mut self, start_pos: usize, start_line: usize, start_col: usize) -> Result<Token, LexError> {
        let mut bin_str = String::new();
        
        while let Some(c) = self.peek() {
            if c == '0' || c == '1' {
                bin_str.push(c);
                self.advance();
            } else {
                break;
            }
        }
        
        if bin_str.is_empty() {
            return Err(LexError {
                message: "expected binary digits after 0b".to_string(),
                span: Span::new(start_pos, self.pos, start_line, start_col),
            });
        }
        
        let value = i64::from_str_radix(&bin_str, 2).map_err(|_| LexError {
            message: format!("invalid binary literal '0b{}'", bin_str),
            span: Span::new(start_pos, self.pos, start_line, start_col),
        })?;
        
        Ok(Token::new(
            TokenKind::Integer(value),
            Span::new(start_pos, self.pos, start_line, start_col),
        ))
    }
    
    /// Read an identifier or keyword.
    fn read_ident(&mut self, first: char, start_pos: usize, start_line: usize, start_col: usize) -> Result<Token, LexError> {
        let mut ident = String::new();
        ident.push(first);
        
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }
        
        let kind = Self::keyword_or_ident(&ident);
        
        Ok(Token::new(kind, Span::new(start_pos, self.pos, start_line, start_col)))
    }
    
    /// Convert identifier to keyword if it matches.
    fn keyword_or_ident(s: &str) -> TokenKind {
        match s {
            // Keywords
            "def" => TokenKind::Def,
            "end" => TokenKind::End,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "do" => TokenKind::Do,
            "times" => TokenKind::Times,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "import" => TokenKind::Import,
            
            // Types
            "i32" => TokenKind::I32,
            "i64" => TokenKind::I64,
            "f32" => TokenKind::F32,
            "f64" => TokenKind::F64,
            "bool" => TokenKind::Bool,
            
            // Stack operations
            "dup" => TokenKind::Dup,
            "drop" => TokenKind::Drop,
            "swap" => TokenKind::Swap,
            "over" => TokenKind::Over,
            "rot" => TokenKind::Rot,
            "nip" => TokenKind::Nip,
            "tuck" => TokenKind::Tuck,
            
            // Logic
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            
            // Bitwise
            "band" => TokenKind::Band,
            "bor" => TokenKind::Bor,
            "bxor" => TokenKind::Bxor,
            "bnot" => TokenKind::Bnot,
            "shl" => TokenKind::Shl,
            "shr" => TokenKind::Shr,
            
            // Memory
            "alloc" => TokenKind::Alloc,
            
            // IO
            "print" => TokenKind::Print,
            "emit" => TokenKind::Emit,
            
            // Other builtins
            "negate" => TokenKind::Negate,
            "abs" => TokenKind::Abs,
            "min" => TokenKind::Min,
            "max" => TokenKind::Max,
            "mod" => TokenKind::Mod,
            
            // Identifier
            _ => TokenKind::Ident(s.to_string()),
        }
    }
    
    /// Skip whitespace only (comments handled in tokenizer).
    fn skip_whitespace_and_comments(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }
    
    /// Peek at current character without advancing.
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }
    
    /// Peek at next character (pos + 1).
    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }
    
    /// Advance position and return character.
    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        if let Some(c) = c {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        c
    }
    
    /// Check if at end of source.
    fn at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
        Lexer::new(source).tokenize()
    }
    
    fn token_kinds(source: &str) -> Vec<TokenKind> {
        tokenize(source)
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .filter(|k| *k != TokenKind::Eof)
            .collect()
    }
    
    #[test]
    fn test_empty() {
        let tokens = tokenize("").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }
    
    #[test]
    fn test_simple_word() {
        let kinds = token_kinds("def foo (--) end");
        assert_eq!(kinds, vec![
            TokenKind::Def,
            TokenKind::Ident("foo".to_string()),
            TokenKind::LParen,
            TokenKind::DashDash,
            TokenKind::RParen,
            TokenKind::End,
        ]);
    }
    
    #[test]
    fn test_integers() {
        let kinds = token_kinds("42 -7 0 123456");
        assert_eq!(kinds, vec![
            TokenKind::Integer(42),
            TokenKind::Minus,
            TokenKind::Integer(7),
            TokenKind::Integer(0),
            TokenKind::Integer(123456),
        ]);
    }
    
    #[test]
    fn test_hex() {
        let kinds = token_kinds("0xFF 0x10 0xABCD");
        assert_eq!(kinds, vec![
            TokenKind::Integer(255),
            TokenKind::Integer(16),
            TokenKind::Integer(0xABCD),
        ]);
    }
    
    #[test]
    fn test_binary() {
        let kinds = token_kinds("0b101 0b1111 0b0");
        assert_eq!(kinds, vec![
            TokenKind::Integer(5),
            TokenKind::Integer(15),
            TokenKind::Integer(0),
        ]);
    }
    
    #[test]
    fn test_floats() {
        let kinds = token_kinds("2.5 0.5 1.0 1e10 2.5e-3");
        assert_eq!(kinds.len(), 5);
        assert!(matches!(kinds[0], TokenKind::Float(f) if (f - 2.5).abs() < 0.001));
        assert!(matches!(kinds[1], TokenKind::Float(f) if (f - 0.5).abs() < 0.001));
        assert!(matches!(kinds[2], TokenKind::Float(f) if (f - 1.0).abs() < 0.001));
        assert!(matches!(kinds[3], TokenKind::Float(f) if (f - 1e10).abs() < 1.0));
        assert!(matches!(kinds[4], TokenKind::Float(f) if (f - 2.5e-3).abs() < 0.0001));
    }
    
    #[test]
    fn test_strings() {
        let kinds = token_kinds(r#""hello" "world""#);
        assert_eq!(kinds, vec![
            TokenKind::String("hello".to_string()),
            TokenKind::String("world".to_string()),
        ]);
    }
    
    #[test]
    fn test_string_escapes() {
        let kinds = token_kinds(r#""hello\nworld" "tab\there" "quote\"here""#);
        assert_eq!(kinds, vec![
            TokenKind::String("hello\nworld".to_string()),
            TokenKind::String("tab\there".to_string()),
            TokenKind::String("quote\"here".to_string()),
        ]);
    }
    
    #[test]
    fn test_operators() {
        let kinds = token_kinds("+ - * / % = != < > <= >=");
        assert_eq!(kinds, vec![
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash,
            TokenKind::Percent,
            TokenKind::Eq,
            TokenKind::NotEq,
            TokenKind::Lt,
            TokenKind::Gt,
            TokenKind::LtEq,
            TokenKind::GtEq,
        ]);
    }
    
    #[test]
    fn test_keywords() {
        let kinds = token_kinds("def end if else while do times true false");
        assert_eq!(kinds, vec![
            TokenKind::Def,
            TokenKind::End,
            TokenKind::If,
            TokenKind::Else,
            TokenKind::While,
            TokenKind::Do,
            TokenKind::Times,
            TokenKind::True,
            TokenKind::False,
        ]);
    }
    
    #[test]
    fn test_types() {
        let kinds = token_kinds("i32 i64 f32 f64 bool");
        assert_eq!(kinds, vec![
            TokenKind::I32,
            TokenKind::I64,
            TokenKind::F32,
            TokenKind::F64,
            TokenKind::Bool,
        ]);
    }
    
    #[test]
    fn test_stack_ops() {
        let kinds = token_kinds("dup drop swap over rot nip tuck");
        assert_eq!(kinds, vec![
            TokenKind::Dup,
            TokenKind::Drop,
            TokenKind::Swap,
            TokenKind::Over,
            TokenKind::Rot,
            TokenKind::Nip,
            TokenKind::Tuck,
        ]);
    }
    
    #[test]
    fn test_builtins() {
        let kinds = token_kinds("and or not negate abs min max mod print emit");
        assert_eq!(kinds, vec![
            TokenKind::And,
            TokenKind::Or,
            TokenKind::Not,
            TokenKind::Negate,
            TokenKind::Abs,
            TokenKind::Min,
            TokenKind::Max,
            TokenKind::Mod,
            TokenKind::Print,
            TokenKind::Emit,
        ]);
    }
    
    #[test]
    fn test_memory_ops() {
        let kinds = token_kinds("@ ! alloc");
        assert_eq!(kinds, vec![
            TokenKind::Fetch,
            TokenKind::Store,
            TokenKind::Alloc,
        ]);
    }
    
    #[test]
    fn test_comments() {
        let kinds = token_kinds("42 -- this is a comment\n43");
        assert_eq!(kinds, vec![
            TokenKind::Integer(42),
            TokenKind::Integer(43),
        ]);
    }
    
    #[test]
    fn test_comment_at_end() {
        let kinds = token_kinds("42 -- comment");
        assert_eq!(kinds, vec![TokenKind::Integer(42)]);
    }
    
    #[test]
    fn test_identifiers() {
        let kinds = token_kinds("foo bar_baz _private CamelCase");
        assert_eq!(kinds, vec![
            TokenKind::Ident("foo".to_string()),
            TokenKind::Ident("bar_baz".to_string()),
            TokenKind::Ident("_private".to_string()),
            TokenKind::Ident("CamelCase".to_string()),
        ]);
    }
    
    #[test]
    fn test_word_definition() {
        let kinds = token_kinds("def square (n -- n) dup * end");
        assert_eq!(kinds, vec![
            TokenKind::Def,
            TokenKind::Ident("square".to_string()),
            TokenKind::LParen,
            TokenKind::Ident("n".to_string()),
            TokenKind::DashDash,
            TokenKind::Ident("n".to_string()),
            TokenKind::RParen,
            TokenKind::Dup,
            TokenKind::Star,
            TokenKind::End,
        ]);
    }
    
    #[test]
    fn test_typed_effect() {
        let kinds = token_kinds("(a: i32 b: i32 -- i32)");
        assert_eq!(kinds, vec![
            TokenKind::LParen,
            TokenKind::Ident("a".to_string()),
            TokenKind::Colon,
            TokenKind::I32,
            TokenKind::Ident("b".to_string()),
            TokenKind::Colon,
            TokenKind::I32,
            TokenKind::DashDash,
            TokenKind::I32,
            TokenKind::RParen,
        ]);
    }
    
    #[test]
    fn test_line_tracking() {
        let tokens = tokenize("42\n43\n44").unwrap();
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[1].span.line, 2);
        assert_eq!(tokens[2].span.line, 3);
    }
    
    #[test]
    fn test_column_tracking() {
        let tokens = tokenize("abc def").unwrap();
        assert_eq!(tokens[0].span.col, 1);
        assert_eq!(tokens[1].span.col, 5);
    }
    
    #[test]
    fn test_unterminated_string() {
        let result = tokenize(r#""hello"#);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("unterminated"));
    }
    
    #[test]
    fn test_invalid_escape() {
        let result = tokenize(r#""hello\z""#);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("escape"));
    }
    
    #[test]
    fn test_invalid_hex() {
        let result = tokenize("0x");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_invalid_binary() {
        let result = tokenize("0b");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_unexpected_char() {
        let result = tokenize("$");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("unexpected"));
    }
    
    #[test]
    fn test_bitwise_ops() {
        let kinds = token_kinds("band bor bxor bnot shl shr");
        assert_eq!(kinds, vec![
            TokenKind::Band,
            TokenKind::Bor,
            TokenKind::Bxor,
            TokenKind::Bnot,
            TokenKind::Shl,
            TokenKind::Shr,
        ]);
    }
    
    #[test]
    fn test_2dup_2drop_2swap() {
        let kinds = token_kinds("2dup 2drop 2swap");
        assert_eq!(kinds, vec![
            TokenKind::Dup2,
            TokenKind::Drop2,
            TokenKind::Swap2,
        ]);
    }
    
    #[test]
    fn test_2_not_keyword() {
        // Plain number 2 should still work
        let kinds = token_kinds("2 23 200");
        assert_eq!(kinds, vec![
            TokenKind::Integer(2),
            TokenKind::Integer(23),
            TokenKind::Integer(200),
        ]);
    }
    
    #[test]
    fn test_cfetch_cstore() {
        let kinds = token_kinds("c@ c!");
        assert_eq!(kinds, vec![
            TokenKind::CFetch,
            TokenKind::CStore,
        ]);
    }
    
    #[test]
    fn test_c_identifier() {
        // Plain 'c' followed by something other than @ or ! should be identifier
        let kinds = token_kinds("count c");
        assert_eq!(kinds, vec![
            TokenKind::Ident("count".to_string()),
            TokenKind::Ident("c".to_string()),
        ]);
    }
}
