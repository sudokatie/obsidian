use crate::ast::{Expr, Literal, Program, WordDef};
use std::collections::HashMap;

/// Runtime value on the stack.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Bool(bool),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::I32(n) => write!(f, "{}", n),
            Value::I64(n) => write!(f, "{}", n),
            Value::F32(n) => write!(f, "{:.6}", n),
            Value::F64(n) => write!(f, "{:.6}", n),
            Value::Bool(b) => write!(f, "{}", b),
        }
    }
}

/// Interpreter state.
pub struct Interpreter {
    stack: Vec<Value>,
    words: HashMap<String, WordDef>,
    trace: bool,
    /// Pending expressions for step mode.
    pending: Vec<Expr>,
    /// Step mode enabled.
    stepping: bool,
}

/// Interpreter error.
#[derive(Debug)]
pub struct InterpError {
    pub message: String,
}

impl std::fmt::Display for InterpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl InterpError {
    fn new(msg: impl Into<String>) -> Self {
        Self { message: msg.into() }
    }

    fn stack_underflow(op: &str, need: usize, have: usize) -> Self {
        Self::new(format!("{}: stack underflow (need {}, have {})", op, need, have))
    }
}

impl Interpreter {
    /// Create a new interpreter.
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            words: HashMap::new(),
            trace: false,
            pending: Vec::new(),
            stepping: false,
        }
    }

    /// Enable/disable trace mode (shows stack after each operation).
    pub fn set_trace(&mut self, enabled: bool) {
        self.trace = enabled;
    }

    /// Check if trace mode is enabled.
    pub fn trace_enabled(&self) -> bool {
        self.trace
    }

    /// Enable/disable step mode.
    pub fn set_stepping(&mut self, enabled: bool) {
        self.stepping = enabled;
    }

    /// Check if step mode is enabled.
    pub fn stepping(&self) -> bool {
        self.stepping
    }

    /// Check if there are pending expressions to step through.
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Get the next pending expression (for display).
    pub fn peek_pending(&self) -> Option<&Expr> {
        self.pending.first()
    }

    /// Load expressions for stepping (clears existing pending).
    pub fn load_for_stepping(&mut self, exprs: Vec<Expr>) {
        self.pending = exprs;
    }

    /// Execute one step (one expression) and return whether more remain.
    pub fn step_one(&mut self) -> Result<bool, InterpError> {
        if self.pending.is_empty() {
            return Ok(false);
        }
        let expr = self.pending.remove(0);
        self.execute_one(&expr)?;
        Ok(!self.pending.is_empty())
    }

    /// Get current stack contents.
    pub fn stack(&self) -> &[Value] {
        &self.stack
    }

    /// Clear the stack.
    pub fn clear(&mut self) {
        self.stack.clear();
        self.pending.clear();
    }

    /// Load word definitions from a program.
    pub fn load_program(&mut self, program: &Program) {
        for word in &program.words {
            self.words.insert(word.name.clone(), word.clone());
        }
    }

    /// Execute a list of expressions.
    pub fn execute(&mut self, exprs: &[Expr]) -> Result<(), InterpError> {
        for expr in exprs {
            self.execute_one(expr)?;
        }
        Ok(())
    }

    /// Execute a single expression.
    fn execute_one(&mut self, expr: &Expr) -> Result<(), InterpError> {
        match expr {
            Expr::Literal(lit) => {
                let val = match lit {
                    Literal::Integer(n) => Value::I64(*n),
                    Literal::Float(n) => Value::F64(*n),
                    Literal::Bool(b) => Value::Bool(*b),
                    Literal::String(_) => {
                        return Err(InterpError::new("string literals not supported in interpreter"));
                    }
                };
                self.stack.push(val);
                if self.trace {
                    println!("  push -> {}", self.format_stack());
                }
            }

            Expr::Word { name, .. } => {
                self.execute_word(name)?;
            }

            Expr::If { then_branch, else_branch, .. } => {
                let cond = self.pop_bool("if")?;
                if cond {
                    self.execute(then_branch)?;
                } else if let Some(else_body) = else_branch {
                    self.execute(else_body)?;
                }
            }

            Expr::While { cond, body, .. } => {
                loop {
                    self.execute(cond)?;
                    let should_continue = self.pop_bool("while")?;
                    if !should_continue {
                        break;
                    }
                    self.execute(body)?;
                }
            }

            Expr::Times { body, .. } => {
                let n = self.pop_i64("times")?;
                for _ in 0..n {
                    self.execute(body)?;
                }
            }
        }
        Ok(())
    }

    /// Execute a word by name.
    fn execute_word(&mut self, name: &str) -> Result<(), InterpError> {
        // Check for user-defined word first
        if let Some(word) = self.words.get(name).cloned() {
            if self.trace {
                println!("  call {} -> (entering)", name);
            }
            self.execute(&word.body)?;
            if self.trace {
                println!("  call {} -> {} (returned)", name, self.format_stack());
            }
            return Ok(());
        }

        // Built-in operations
        match name {
            // Stack ops
            "dup" => self.op_dup()?,
            "drop" => self.op_drop()?,
            "swap" => self.op_swap()?,
            "over" => self.op_over()?,
            "rot" => self.op_rot()?,
            "nip" => self.op_nip()?,
            "tuck" => self.op_tuck()?,
            "2dup" => self.op_2dup()?,
            "2drop" => self.op_2drop()?,
            "2swap" => self.op_2swap()?,
            "-rot" => self.op_neg_rot()?,

            // Arithmetic
            "+" => self.binop_i64(name, |a, b| a + b)?,
            "-" => self.binop_i64(name, |a, b| a - b)?,
            "*" => self.binop_i64(name, |a, b| a * b)?,
            "/" => self.binop_i64(name, |a, b| if b != 0 { a / b } else { 0 })?,
            "mod" => self.binop_i64(name, |a, b| if b != 0 { a % b } else { 0 })?,
            "negate" => self.unop_i64(name, |a| -a)?,
            "abs" => self.unop_i64(name, |a| a.abs())?,
            "min" => self.binop_i64(name, |a, b| a.min(b))?,
            "max" => self.binop_i64(name, |a, b| a.max(b))?,

            // Comparison
            "=" => self.cmp_i64(name, |a, b| a == b)?,
            "<>" => self.cmp_i64(name, |a, b| a != b)?,
            "<" => self.cmp_i64(name, |a, b| a < b)?,
            ">" => self.cmp_i64(name, |a, b| a > b)?,
            "<=" => self.cmp_i64(name, |a, b| a <= b)?,
            ">=" => self.cmp_i64(name, |a, b| a >= b)?,

            // Logic
            "and" => self.binop_bool(name, |a, b| a && b)?,
            "or" => self.binop_bool(name, |a, b| a || b)?,
            "not" => {
                let a = self.pop_bool(name)?;
                self.stack.push(Value::Bool(!a));
                if self.trace {
                    println!("  {} -> {}", name, self.format_stack());
                }
            }

            // I/O
            "." => {
                let val = self.pop(name)?;
                println!("{}", val);
                if self.trace {
                    println!("  {} -> {}", name, self.format_stack());
                }
            }
            ".s" => {
                println!("{}", self.format_stack());
            }

            _ => {
                return Err(InterpError::new(format!("unknown word: {}", name)));
            }
        }
        Ok(())
    }

    // Stack operations
    fn op_dup(&mut self) -> Result<(), InterpError> {
        let val = self.peek("dup")?;
        self.stack.push(val);
        if self.trace {
            println!("  dup -> {}", self.format_stack());
        }
        Ok(())
    }

    fn op_drop(&mut self) -> Result<(), InterpError> {
        self.pop("drop")?;
        if self.trace {
            println!("  drop -> {}", self.format_stack());
        }
        Ok(())
    }

    fn op_swap(&mut self) -> Result<(), InterpError> {
        if self.stack.len() < 2 {
            return Err(InterpError::stack_underflow("swap", 2, self.stack.len()));
        }
        let len = self.stack.len();
        self.stack.swap(len - 1, len - 2);
        if self.trace {
            println!("  swap -> {}", self.format_stack());
        }
        Ok(())
    }

    fn op_over(&mut self) -> Result<(), InterpError> {
        if self.stack.len() < 2 {
            return Err(InterpError::stack_underflow("over", 2, self.stack.len()));
        }
        let val = self.stack[self.stack.len() - 2].clone();
        self.stack.push(val);
        if self.trace {
            println!("  over -> {}", self.format_stack());
        }
        Ok(())
    }

    fn op_rot(&mut self) -> Result<(), InterpError> {
        if self.stack.len() < 3 {
            return Err(InterpError::stack_underflow("rot", 3, self.stack.len()));
        }
        let len = self.stack.len();
        let a = self.stack.remove(len - 3);
        self.stack.push(a);
        if self.trace {
            println!("  rot -> {}", self.format_stack());
        }
        Ok(())
    }

    fn op_neg_rot(&mut self) -> Result<(), InterpError> {
        if self.stack.len() < 3 {
            return Err(InterpError::stack_underflow("-rot", 3, self.stack.len()));
        }
        let val = self.stack.pop().unwrap();
        let len = self.stack.len();
        self.stack.insert(len - 2, val);
        if self.trace {
            println!("  -rot -> {}", self.format_stack());
        }
        Ok(())
    }

    fn op_nip(&mut self) -> Result<(), InterpError> {
        if self.stack.len() < 2 {
            return Err(InterpError::stack_underflow("nip", 2, self.stack.len()));
        }
        let len = self.stack.len();
        self.stack.remove(len - 2);
        if self.trace {
            println!("  nip -> {}", self.format_stack());
        }
        Ok(())
    }

    fn op_tuck(&mut self) -> Result<(), InterpError> {
        if self.stack.len() < 2 {
            return Err(InterpError::stack_underflow("tuck", 2, self.stack.len()));
        }
        let top = self.stack.last().unwrap().clone();
        let len = self.stack.len();
        self.stack.insert(len - 2, top);
        if self.trace {
            println!("  tuck -> {}", self.format_stack());
        }
        Ok(())
    }

    fn op_2dup(&mut self) -> Result<(), InterpError> {
        if self.stack.len() < 2 {
            return Err(InterpError::stack_underflow("2dup", 2, self.stack.len()));
        }
        let len = self.stack.len();
        let a = self.stack[len - 2].clone();
        let b = self.stack[len - 1].clone();
        self.stack.push(a);
        self.stack.push(b);
        if self.trace {
            println!("  2dup -> {}", self.format_stack());
        }
        Ok(())
    }

    fn op_2drop(&mut self) -> Result<(), InterpError> {
        if self.stack.len() < 2 {
            return Err(InterpError::stack_underflow("2drop", 2, self.stack.len()));
        }
        self.stack.pop();
        self.stack.pop();
        if self.trace {
            println!("  2drop -> {}", self.format_stack());
        }
        Ok(())
    }

    fn op_2swap(&mut self) -> Result<(), InterpError> {
        if self.stack.len() < 4 {
            return Err(InterpError::stack_underflow("2swap", 4, self.stack.len()));
        }
        let len = self.stack.len();
        self.stack.swap(len - 1, len - 3);
        self.stack.swap(len - 2, len - 4);
        if self.trace {
            println!("  2swap -> {}", self.format_stack());
        }
        Ok(())
    }

    // Helper for binary i64 operations
    fn binop_i64(&mut self, name: &str, f: impl Fn(i64, i64) -> i64) -> Result<(), InterpError> {
        let b = self.pop_i64(name)?;
        let a = self.pop_i64(name)?;
        self.stack.push(Value::I64(f(a, b)));
        if self.trace {
            println!("  {} -> {}", name, self.format_stack());
        }
        Ok(())
    }

    // Helper for unary i64 operations
    fn unop_i64(&mut self, name: &str, f: impl Fn(i64) -> i64) -> Result<(), InterpError> {
        let a = self.pop_i64(name)?;
        self.stack.push(Value::I64(f(a)));
        if self.trace {
            println!("  {} -> {}", name, self.format_stack());
        }
        Ok(())
    }

    // Helper for comparison operations
    fn cmp_i64(&mut self, name: &str, f: impl Fn(i64, i64) -> bool) -> Result<(), InterpError> {
        let b = self.pop_i64(name)?;
        let a = self.pop_i64(name)?;
        self.stack.push(Value::Bool(f(a, b)));
        if self.trace {
            println!("  {} -> {}", name, self.format_stack());
        }
        Ok(())
    }

    // Helper for binary bool operations
    fn binop_bool(&mut self, name: &str, f: impl Fn(bool, bool) -> bool) -> Result<(), InterpError> {
        let b = self.pop_bool(name)?;
        let a = self.pop_bool(name)?;
        self.stack.push(Value::Bool(f(a, b)));
        if self.trace {
            println!("  {} -> {}", name, self.format_stack());
        }
        Ok(())
    }

    // Stack access helpers
    fn pop(&mut self, op: &str) -> Result<Value, InterpError> {
        self.stack.pop().ok_or_else(|| InterpError::stack_underflow(op, 1, 0))
    }

    fn peek(&self, op: &str) -> Result<Value, InterpError> {
        self.stack.last().cloned().ok_or_else(|| InterpError::stack_underflow(op, 1, 0))
    }

    fn pop_i64(&mut self, op: &str) -> Result<i64, InterpError> {
        match self.pop(op)? {
            Value::I64(n) => Ok(n),
            Value::I32(n) => Ok(n as i64),
            other => Err(InterpError::new(format!("{}: expected integer, got {:?}", op, other))),
        }
    }

    fn pop_bool(&mut self, op: &str) -> Result<bool, InterpError> {
        match self.pop(op)? {
            Value::Bool(b) => Ok(b),
            other => Err(InterpError::new(format!("{}: expected bool, got {:?}", op, other))),
        }
    }

    /// Format stack for display.
    pub fn format_stack(&self) -> String {
        if self.stack.is_empty() {
            return "<empty>".to_string();
        }
        let items: Vec<String> = self.stack.iter().map(|v| v.to_string()).collect();
        format!("<{}>", items.join(" "))
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn make_int(n: i64) -> Expr {
        Expr::Literal(Literal::Integer(n))
    }

    fn make_word(name: &str) -> Expr {
        Expr::Word { name: name.to_string(), span: Span::default() }
    }

    #[test]
    fn test_push_literal() {
        let mut interp = Interpreter::new();
        interp.execute(&[make_int(42)]).unwrap();
        assert_eq!(interp.stack(), &[Value::I64(42)]);
    }

    #[test]
    fn test_dup() {
        let mut interp = Interpreter::new();
        interp.execute(&[make_int(5), make_word("dup")]).unwrap();
        assert_eq!(interp.stack(), &[Value::I64(5), Value::I64(5)]);
    }

    #[test]
    fn test_drop() {
        let mut interp = Interpreter::new();
        interp.execute(&[make_int(1), make_int(2), make_word("drop")]).unwrap();
        assert_eq!(interp.stack(), &[Value::I64(1)]);
    }

    #[test]
    fn test_swap() {
        let mut interp = Interpreter::new();
        interp.execute(&[make_int(1), make_int(2), make_word("swap")]).unwrap();
        assert_eq!(interp.stack(), &[Value::I64(2), Value::I64(1)]);
    }

    #[test]
    fn test_over() {
        let mut interp = Interpreter::new();
        interp.execute(&[make_int(1), make_int(2), make_word("over")]).unwrap();
        assert_eq!(interp.stack(), &[Value::I64(1), Value::I64(2), Value::I64(1)]);
    }

    #[test]
    fn test_rot() {
        let mut interp = Interpreter::new();
        interp.execute(&[make_int(1), make_int(2), make_int(3), make_word("rot")]).unwrap();
        assert_eq!(interp.stack(), &[Value::I64(2), Value::I64(3), Value::I64(1)]);
    }

    #[test]
    fn test_arithmetic() {
        let mut interp = Interpreter::new();
        interp.execute(&[make_int(10), make_int(3), make_word("+")]).unwrap();
        assert_eq!(interp.stack(), &[Value::I64(13)]);

        interp.clear();
        interp.execute(&[make_int(10), make_int(3), make_word("-")]).unwrap();
        assert_eq!(interp.stack(), &[Value::I64(7)]);

        interp.clear();
        interp.execute(&[make_int(10), make_int(3), make_word("*")]).unwrap();
        assert_eq!(interp.stack(), &[Value::I64(30)]);

        interp.clear();
        interp.execute(&[make_int(10), make_int(3), make_word("/")]).unwrap();
        assert_eq!(interp.stack(), &[Value::I64(3)]);

        interp.clear();
        interp.execute(&[make_int(10), make_int(3), make_word("mod")]).unwrap();
        assert_eq!(interp.stack(), &[Value::I64(1)]);
    }

    #[test]
    fn test_comparison() {
        let mut interp = Interpreter::new();
        interp.execute(&[make_int(5), make_int(5), make_word("=")]).unwrap();
        assert_eq!(interp.stack(), &[Value::Bool(true)]);

        interp.clear();
        interp.execute(&[make_int(5), make_int(3), make_word("<")]).unwrap();
        assert_eq!(interp.stack(), &[Value::Bool(false)]);

        interp.clear();
        interp.execute(&[make_int(3), make_int(5), make_word("<")]).unwrap();
        assert_eq!(interp.stack(), &[Value::Bool(true)]);
    }

    #[test]
    fn test_if_then() {
        let mut interp = Interpreter::new();
        let exprs = vec![
            make_int(1),
            Expr::Literal(Literal::Bool(true)),
            Expr::If {
                then_branch: vec![make_int(10), make_word("+")],
                else_branch: None,
                span: Span::default(),
            },
        ];
        interp.execute(&exprs).unwrap();
        assert_eq!(interp.stack(), &[Value::I64(11)]);
    }

    #[test]
    fn test_if_else() {
        let mut interp = Interpreter::new();
        let exprs = vec![
            make_int(1),
            Expr::Literal(Literal::Bool(false)),
            Expr::If {
                then_branch: vec![make_int(10), make_word("+")],
                else_branch: Some(vec![make_int(20), make_word("+")]),
                span: Span::default(),
            },
        ];
        interp.execute(&exprs).unwrap();
        assert_eq!(interp.stack(), &[Value::I64(21)]);
    }

    #[test]
    fn test_times_loop() {
        let mut interp = Interpreter::new();
        let exprs = vec![
            make_int(0),
            make_int(5),
            Expr::Times {
                body: vec![make_int(1), make_word("+")],
                span: Span::default(),
            },
        ];
        interp.execute(&exprs).unwrap();
        assert_eq!(interp.stack(), &[Value::I64(5)]);
    }

    #[test]
    fn test_trace_mode() {
        let mut interp = Interpreter::new();
        interp.set_trace(true);
        assert!(interp.trace_enabled());
        interp.set_trace(false);
        assert!(!interp.trace_enabled());
    }

    #[test]
    fn test_format_stack() {
        let mut interp = Interpreter::new();
        assert_eq!(interp.format_stack(), "<empty>");
        
        interp.execute(&[make_int(1), make_int(2), make_int(3)]).unwrap();
        assert_eq!(interp.format_stack(), "<1 2 3>");
    }

    #[test]
    fn test_stack_underflow() {
        let mut interp = Interpreter::new();
        let result = interp.execute(&[make_word("dup")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("stack underflow"));
    }

    #[test]
    fn test_step_mode() {
        let mut interp = Interpreter::new();
        interp.set_stepping(true);
        assert!(interp.stepping());
        interp.set_stepping(false);
        assert!(!interp.stepping());
    }

    #[test]
    fn test_load_for_stepping() {
        let mut interp = Interpreter::new();
        assert!(!interp.has_pending());
        
        interp.load_for_stepping(vec![make_int(1), make_int(2), make_word("+")]);
        assert!(interp.has_pending());
    }

    #[test]
    fn test_step_one() {
        let mut interp = Interpreter::new();
        interp.load_for_stepping(vec![make_int(5), make_int(3), make_word("+")]);
        
        // Step 1: push 5
        assert!(interp.step_one().unwrap());
        assert_eq!(interp.stack(), &[Value::I64(5)]);
        
        // Step 2: push 3
        assert!(interp.step_one().unwrap());
        assert_eq!(interp.stack(), &[Value::I64(5), Value::I64(3)]);
        
        // Step 3: add (returns false = no more steps)
        assert!(!interp.step_one().unwrap());
        assert_eq!(interp.stack(), &[Value::I64(8)]);
    }

    #[test]
    fn test_clear_clears_pending() {
        let mut interp = Interpreter::new();
        interp.load_for_stepping(vec![make_int(1), make_int(2)]);
        assert!(interp.has_pending());
        
        interp.clear();
        assert!(!interp.has_pending());
        assert!(interp.stack().is_empty());
    }
}
