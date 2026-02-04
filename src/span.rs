/// Source location tracking for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// Byte offset start (inclusive)
    pub start: usize,
    /// Byte offset end (exclusive)
    pub end: usize,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub col: usize,
}

impl Span {
    /// Create a new span.
    pub fn new(start: usize, end: usize, line: usize, col: usize) -> Self {
        Self { start, end, line, col }
    }
    
    /// Merge two spans into one covering both.
    pub fn merge(self, other: Span) -> Span {
        let start = self.start.min(other.start);
        let end = self.end.max(other.end);
        // Use the earlier span's line/col
        let (line, col) = if self.start <= other.start {
            (self.line, self.col)
        } else {
            (other.line, other.col)
        };
        Span { start, end, line, col }
    }
    
    /// Length in bytes.
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
    
    /// Check if span is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_span_new() {
        let span = Span::new(0, 5, 1, 1);
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 5);
        assert_eq!(span.line, 1);
        assert_eq!(span.col, 1);
    }
    
    #[test]
    fn test_span_merge() {
        let a = Span::new(0, 5, 1, 1);
        let b = Span::new(10, 15, 2, 3);
        let merged = a.merge(b);
        assert_eq!(merged.start, 0);
        assert_eq!(merged.end, 15);
        assert_eq!(merged.line, 1);
        assert_eq!(merged.col, 1);
    }
    
    #[test]
    fn test_span_merge_reverse() {
        let a = Span::new(10, 15, 2, 3);
        let b = Span::new(0, 5, 1, 1);
        let merged = a.merge(b);
        assert_eq!(merged.start, 0);
        assert_eq!(merged.end, 15);
        assert_eq!(merged.line, 1);
        assert_eq!(merged.col, 1);
    }
    
    #[test]
    fn test_span_len() {
        let span = Span::new(5, 10, 1, 1);
        assert_eq!(span.len(), 5);
    }
    
    #[test]
    fn test_span_is_empty() {
        let empty = Span::new(5, 5, 1, 1);
        assert!(empty.is_empty());
        
        let non_empty = Span::new(5, 10, 1, 1);
        assert!(!non_empty.is_empty());
    }
    
    #[test]
    fn test_span_default() {
        let span = Span::default();
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 0);
        assert_eq!(span.line, 0);
        assert_eq!(span.col, 0);
    }
}
