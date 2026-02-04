use crate::lexer::LexError;
use crate::parser::ParseError;
use crate::span::Span;

/// Unified error type for the compiler.
#[derive(Debug)]
pub enum Error {
    Lex(LexError),
    Parse(ParseError),
    Check(CheckError),
    CodeGen(CodeGenError),
    Io(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Lex(e) => write!(f, "lex error: {}", e),
            Error::Parse(e) => write!(f, "parse error: {}", e),
            Error::Check(e) => write!(f, "check error: {}", e),
            Error::CodeGen(e) => write!(f, "codegen error: {}", e),
            Error::Io(e) => write!(f, "io error: {}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<LexError> for Error {
    fn from(e: LexError) -> Self {
        Error::Lex(e)
    }
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Error::Parse(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<CheckError> for Error {
    fn from(e: CheckError) -> Self {
        Error::Check(e)
    }
}

/// Type/stack checking error.
#[derive(Debug, Clone)]
pub struct CheckError {
    pub code: &'static str,
    pub message: String,
    pub span: Span,
    pub note: Option<String>,
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CheckError {}

/// Code generation error.
#[derive(Debug, Clone)]
pub struct CodeGenError {
    pub message: String,
}

impl std::fmt::Display for CodeGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CodeGenError {}

/// Format an error with source context.
pub fn format_error(source: &str, error: &Error) -> String {
    let (span, code, message, note) = match error {
        Error::Lex(e) => (e.span, "E001", e.message.clone(), None),
        Error::Parse(e) => {
            let msg = if let Some(exp) = &e.expected {
                format!("expected {}, {}", exp, e.message)
            } else {
                e.message.clone()
            };
            (e.span, "E002", msg, None)
        }
        Error::Check(e) => (e.span, e.code, e.message.clone(), e.note.clone()),
        Error::CodeGen(e) => {
            return format!("error[E004]: {}", e.message);
        }
        Error::Io(e) => {
            return format!("error[E005]: {}", e);
        }
    };
    
    format_with_context(source, span, code, &message, note.as_deref())
}

/// Format error with source context lines.
fn format_with_context(
    source: &str,
    span: Span,
    code: &str,
    message: &str,
    note: Option<&str>,
) -> String {
    let mut out = String::new();
    
    // Error header
    out.push_str(&format!("error[{}]: {}\n", code, message));
    
    // Location
    out.push_str(&format!("  --> <input>:{}:{}\n", span.line, span.col));
    
    // Get the source line
    let lines: Vec<&str> = source.lines().collect();
    if span.line > 0 && span.line <= lines.len() {
        let line_num = span.line;
        let line = lines[line_num - 1];
        let line_num_width = format!("{}", line_num).len();
        
        // Separator
        out.push_str(&format!("{:width$} |\n", "", width = line_num_width));
        
        // Source line
        out.push_str(&format!("{} | {}\n", line_num, line));
        
        // Underline
        let underline_start = span.col.saturating_sub(1);
        let underline_len = (span.end - span.start).max(1);
        out.push_str(&format!(
            "{:width$} | {:>pad$}{}\n",
            "",
            "",
            "^".repeat(underline_len),
            width = line_num_width,
            pad = underline_start,
        ));
    }
    
    // Note
    if let Some(note) = note {
        out.push_str(&format!("   = note: {}\n", note));
    }
    
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_lex_error() {
        let source = "def foo\n$$$\nend";
        let error = Error::Lex(LexError {
            message: "unexpected character '$'".to_string(),
            span: Span::new(8, 9, 2, 1),
        });
        let formatted = format_error(source, &error);
        assert!(formatted.contains("error[E001]"));
        assert!(formatted.contains("unexpected character"));
        assert!(formatted.contains("2:1"));
    }
    
    #[test]
    fn test_format_parse_error() {
        let source = "def foo end";
        let error = Error::Parse(ParseError {
            message: "got End".to_string(),
            span: Span::new(8, 11, 1, 9),
            expected: Some("'('".to_string()),
        });
        let formatted = format_error(source, &error);
        assert!(formatted.contains("error[E002]"));
        assert!(formatted.contains("expected"));
    }
    
    #[test]
    fn test_format_check_error() {
        let source = "def foo (--) bar end";
        let error = Error::Check(CheckError {
            code: "E003",
            message: "undefined word 'bar'".to_string(),
            span: Span::new(13, 16, 1, 14),
            note: Some("did you mean 'bor'?".to_string()),
        });
        let formatted = format_error(source, &error);
        assert!(formatted.contains("error[E003]"));
        assert!(formatted.contains("undefined word"));
        assert!(formatted.contains("note: did you mean"));
    }
    
    #[test]
    fn test_format_io_error() {
        let error = Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        let formatted = format_error("", &error);
        assert!(formatted.contains("error[E005]"));
        assert!(formatted.contains("file not found"));
    }
    
    #[test]
    fn test_format_codegen_error() {
        let error = Error::CodeGen(CodeGenError {
            message: "stack overflow".to_string(),
        });
        let formatted = format_error("", &error);
        assert!(formatted.contains("error[E004]"));
        assert!(formatted.contains("stack overflow"));
    }
    
    #[test]
    fn test_error_display() {
        let error = Error::Lex(LexError {
            message: "test".to_string(),
            span: Span::default(),
        });
        let display = format!("{}", error);
        assert!(display.contains("lex error"));
    }
    
    #[test]
    fn test_check_error_new() {
        let err = CheckError {
            code: "E003",
            message: "test".to_string(),
            span: Span::new(0, 1, 1, 1),
            note: None,
        };
        assert_eq!(err.code, "E003");
    }
}
