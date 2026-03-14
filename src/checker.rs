use crate::ast::{Expr, Program, WordDef};
use crate::error::CheckError;
use std::collections::HashMap;

/// Stack effect checker for Obsidian programs.
pub struct Checker {
    /// Known words: name -> (inputs, outputs)
    words: HashMap<String, (usize, usize)>,
    /// Errors collected during checking.
    errors: Vec<CheckError>,
}

impl Checker {
    /// Create a new checker with builtin word effects.
    pub fn new() -> Self {
        let mut words = HashMap::new();
        
        // Stack operations
        words.insert("dup".to_string(), (1, 2));     // (a -- a a)
        words.insert("drop".to_string(), (1, 0));    // (a --)
        words.insert("swap".to_string(), (2, 2));    // (a b -- b a)
        words.insert("over".to_string(), (2, 3));    // (a b -- a b a)
        words.insert("rot".to_string(), (3, 3));     // (a b c -- b c a)
        words.insert("nip".to_string(), (2, 1));     // (a b -- b)
        words.insert("tuck".to_string(), (2, 3));    // (a b -- b a b)
        words.insert("2dup".to_string(), (2, 4));    // (a b -- a b a b)
        words.insert("2drop".to_string(), (2, 0));   // (a b --)
        words.insert("2swap".to_string(), (4, 4));   // (a b c d -- c d a b)
        
        // Arithmetic (all binary ops: a b -- c)
        for op in ["+", "-", "*", "/", "mod", "min", "max"] {
            words.insert(op.to_string(), (2, 1));
        }
        
        // Unary arithmetic
        words.insert("negate".to_string(), (1, 1));  // (a -- -a)
        words.insert("abs".to_string(), (1, 1));     // (a -- |a|)
        
        // Comparison (all: a b -- bool)
        for op in ["=", "!=", "<", ">", "<=", ">="] {
            words.insert(op.to_string(), (2, 1));
        }
        
        // Logic
        words.insert("and".to_string(), (2, 1));     // (a b -- c)
        words.insert("or".to_string(), (2, 1));      // (a b -- c)
        words.insert("not".to_string(), (1, 1));     // (a -- b)
        
        // Bitwise
        words.insert("band".to_string(), (2, 1));    // (a b -- c)
        words.insert("bor".to_string(), (2, 1));     // (a b -- c)
        words.insert("bxor".to_string(), (2, 1));    // (a b -- c)
        words.insert("bnot".to_string(), (1, 1));    // (a -- b)
        words.insert("shl".to_string(), (2, 1));     // (a n -- b)
        words.insert("shr".to_string(), (2, 1));     // (a n -- b)
        
        // Memory
        words.insert("@".to_string(), (1, 1));       // (addr -- val)
        words.insert("!".to_string(), (2, 0));       // (val addr --)
        words.insert("c@".to_string(), (1, 1));      // (addr -- byte)
        words.insert("c!".to_string(), (2, 0));      // (byte addr --)
        words.insert("alloc".to_string(), (1, 1));   // (size -- addr)
        
        // IO
        words.insert("print".to_string(), (1, 0));   // (val --)
        words.insert("emit".to_string(), (1, 0));    // (char --)
        
        Self {
            words,
            errors: Vec::new(),
        }
    }
    
    /// Check an entire program, returning collected errors.
    pub fn check(&mut self, program: &Program) -> Result<(), Vec<CheckError>> {
        self.errors.clear();
        
        // First pass: register all user-defined words
        for word in &program.words {
            let inputs = word.effect.inputs.len();
            let outputs = word.effect.outputs.len();
            self.words.insert(word.name.clone(), (inputs, outputs));
        }
        
        // Second pass: check each word's body
        for word in &program.words {
            self.check_word(word);
        }
        
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }
    
    /// Check a single word definition.
    fn check_word(&mut self, word: &WordDef) {
        let initial_depth = word.effect.inputs.len() as isize;
        let expected_depth = word.effect.outputs.len() as isize;
        
        match self.check_body(&word.body, initial_depth) {
            Ok(final_depth) => {
                if final_depth != expected_depth {
                    self.errors.push(CheckError {
                        code: "E004",
                        message: format!(
                            "stack effect mismatch in '{}': declared ({} -- {}), actual ({} -- {})",
                            word.name,
                            word.effect.inputs.len(),
                            word.effect.outputs.len(),
                            word.effect.inputs.len(),
                            (final_depth + word.effect.inputs.len() as isize) as usize,
                        ),
                        span: word.span,
                        note: Some(format!(
                            "expected final stack depth {}, got {}",
                            expected_depth, final_depth
                        )),
                    });
                }
            }
            Err(e) => self.errors.push(e),
        }
    }
    
    /// Check a body (list of expressions), returning final stack depth.
    fn check_body(&self, body: &[Expr], mut depth: isize) -> Result<isize, CheckError> {
        for expr in body {
            depth = self.check_expr(expr, depth)?;
        }
        Ok(depth)
    }
    
    /// Check a single expression, returning new stack depth.
    fn check_expr(&self, expr: &Expr, depth: isize) -> Result<isize, CheckError> {
        match expr {
            Expr::Literal(_) => {
                // Pushes one value
                Ok(depth + 1)
            }
            
            Expr::Word { name, span } => {
                if let Some(&(inputs, outputs)) = self.words.get(name) {
                    let new_depth = depth - inputs as isize + outputs as isize;
                    if depth < inputs as isize {
                        return Err(CheckError {
                            code: "E001",
                            message: format!(
                                "stack underflow: '{}' requires {} values, stack has {}",
                                name, inputs, depth
                            ),
                            span: *span,
                            note: None,
                        });
                    }
                    Ok(new_depth)
                } else {
                    Err(CheckError {
                        code: "E003",
                        message: format!("undefined word '{}'", name),
                        span: *span,
                        note: self.suggest_word(name),
                    })
                }
            }
            
            Expr::If { then_branch, else_branch, span } => {
                // Condition pops one value
                if depth < 1 {
                    return Err(CheckError {
                        code: "E001",
                        message: "stack underflow: 'if' requires a condition value".to_string(),
                        span: *span,
                        note: None,
                    });
                }
                let depth_after_cond = depth - 1;
                
                // Check then branch
                let then_depth = self.check_body(then_branch, depth_after_cond)?;
                
                // Check else branch if present
                if let Some(else_body) = else_branch {
                    let else_depth = self.check_body(else_body, depth_after_cond)?;
                    if then_depth != else_depth {
                        return Err(CheckError {
                            code: "E004",
                            message: format!(
                                "if/else branches have different stack effects: then={}, else={}",
                                then_depth - depth_after_cond,
                                else_depth - depth_after_cond
                            ),
                            span: *span,
                            note: Some("both branches must leave the stack at the same depth".to_string()),
                        });
                    }
                    Ok(then_depth)
                } else {
                    // No else branch: then branch must have net zero effect
                    if then_depth != depth_after_cond {
                        return Err(CheckError {
                            code: "E004",
                            message: format!(
                                "if without else must have net zero effect, got {}",
                                then_depth - depth_after_cond
                            ),
                            span: *span,
                            note: Some("add an else branch or ensure the body doesn't change stack depth".to_string()),
                        });
                    }
                    Ok(then_depth)
                }
            }
            
            Expr::While { cond, body, span } => {
                // Check condition (must produce exactly one value to consume)
                let cond_depth = self.check_body(cond, depth)?;
                let cond_effect = cond_depth - depth;
                
                // Condition should push exactly 1 for the loop check
                if cond_effect != 1 {
                    return Err(CheckError {
                        code: "E004",
                        message: format!(
                            "while condition must push exactly 1 value, got {}",
                            cond_effect
                        ),
                        span: *span,
                        note: None,
                    });
                }
                
                // After condition check, one value is consumed
                let depth_after_check = cond_depth - 1;
                
                // Body must have net zero effect (loop invariant)
                let body_depth = self.check_body(body, depth_after_check)?;
                let body_effect = body_depth - depth_after_check;
                
                if body_effect != 0 {
                    return Err(CheckError {
                        code: "E004",
                        message: format!(
                            "while body must have net zero effect, got {}",
                            body_effect
                        ),
                        span: *span,
                        note: Some("loop body is executed multiple times, so stack must be balanced".to_string()),
                    });
                }
                
                // Loop exits with stack at depth_after_check
                Ok(depth_after_check)
            }
            
            Expr::Times { body, span } => {
                // Times consumes the count from stack
                if depth < 1 {
                    return Err(CheckError {
                        code: "E001",
                        message: "stack underflow: 'times' requires a count value".to_string(),
                        span: *span,
                        note: None,
                    });
                }
                let depth_after_count = depth - 1;
                
                // Body must have net zero effect
                let body_depth = self.check_body(body, depth_after_count)?;
                let body_effect = body_depth - depth_after_count;
                
                if body_effect != 0 {
                    return Err(CheckError {
                        code: "E004",
                        message: format!(
                            "times body must have net zero effect, got {}",
                            body_effect
                        ),
                        span: *span,
                        note: Some("loop body is executed multiple times, so stack must be balanced".to_string()),
                    });
                }
                
                Ok(depth_after_count)
            }
        }
    }
    
    /// Suggest a similar word name for typos.
    fn suggest_word(&self, name: &str) -> Option<String> {
        let mut best: Option<(&str, usize)> = None;
        
        for known in self.words.keys() {
            let dist = levenshtein(name, known);
            if dist <= 2 {
                match best {
                    None => best = Some((known, dist)),
                    Some((_, d)) if dist < d => best = Some((known, dist)),
                    _ => {}
                }
            }
        }
        
        best.map(|(s, _)| format!("did you mean '{}'?", s))
    }
}

impl Default for Checker {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple Levenshtein distance for word suggestions.
fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();
    
    if m == 0 { return n; }
    if n == 0 { return m; }
    
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0; n + 1];
    
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1)
                .min(curr[j - 1] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    
    prev[n]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Literal, Program, WordDef, StackEffect, StackItem};
    use crate::span::Span;
    
    fn make_word(name: &str, inputs: usize, outputs: usize, body: Vec<Expr>) -> WordDef {
        WordDef {
            name: name.to_string(),
            effect: StackEffect {
                inputs: (0..inputs).map(|i| StackItem::named(format!("a{}", i))).collect(),
                outputs: (0..outputs).map(|i| StackItem::named(format!("b{}", i))).collect(),
            },
            body,
            span: Span::new(0, 1, 1, 1),
        }
    }
    
    fn word_call(name: &str) -> Expr {
        Expr::Word { name: name.to_string(), span: Span::new(0, 1, 1, 1) }
    }
    
    fn int_literal(n: i64) -> Expr {
        Expr::Literal(Literal::Integer(n))
    }
    
    #[test]
    fn test_checker_new_has_builtins() {
        let checker = Checker::new();
        assert!(checker.words.contains_key("dup"));
        assert!(checker.words.contains_key("+"));
        assert!(checker.words.contains_key("print"));
    }
    
    #[test]
    fn test_empty_program() {
        let mut checker = Checker::new();
        let program = Program::new();
        assert!(checker.check(&program).is_ok());
    }
    
    #[test]
    fn test_simple_word_correct_effect() {
        let mut checker = Checker::new();
        // def square (n -- n) dup * end
        let word = make_word("square", 1, 1, vec![
            word_call("dup"),
            word_call("*"),
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        assert!(checker.check(&program).is_ok());
    }
    
    #[test]
    fn test_stack_underflow() {
        let mut checker = Checker::new();
        // def bad (--) drop end -- underflow!
        let word = make_word("bad", 0, 0, vec![
            word_call("drop"),
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        let result = checker.check(&program);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "E001");
        assert!(errors[0].message.contains("underflow"));
    }
    
    #[test]
    fn test_undefined_word() {
        let mut checker = Checker::new();
        // def bad (--) foobar end
        let word = make_word("bad", 0, 0, vec![
            word_call("foobar"),
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        let result = checker.check(&program);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors[0].code, "E003");
        assert!(errors[0].message.contains("undefined word"));
    }
    
    #[test]
    fn test_word_suggestion() {
        let mut checker = Checker::new();
        // def bad (--) prnt end -- should suggest 'print'
        let word = make_word("bad", 0, 0, vec![
            int_literal(42),
            word_call("prnt"),
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        let result = checker.check(&program);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].note.as_ref().map(|n| n.contains("print")).unwrap_or(false));
    }
    
    #[test]
    fn test_effect_mismatch() {
        let mut checker = Checker::new();
        // def bad (n -- n) drop end -- declares 1 output but produces 0
        let word = make_word("bad", 1, 1, vec![
            word_call("drop"),
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        let result = checker.check(&program);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors[0].code, "E004");
        assert!(errors[0].message.contains("mismatch"));
    }
    
    #[test]
    fn test_literal_pushes() {
        let mut checker = Checker::new();
        // def foo (-- n) 42 end
        let word = make_word("foo", 0, 1, vec![
            int_literal(42),
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        assert!(checker.check(&program).is_ok());
    }
    
    #[test]
    fn test_if_branches_same_effect() {
        let mut checker = Checker::new();
        // def foo (a -- b) if 1 else 2 end end
        let word = make_word("foo", 1, 1, vec![
            Expr::If {
                then_branch: vec![int_literal(1)],
                else_branch: Some(vec![int_literal(2)]),
                span: Span::default(),
            },
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        assert!(checker.check(&program).is_ok());
    }
    
    #[test]
    fn test_if_branches_different_effect() {
        let mut checker = Checker::new();
        // def bad (a --) if 1 else end end -- then pushes, else doesn't
        let word = make_word("bad", 1, 0, vec![
            Expr::If {
                then_branch: vec![int_literal(1)],
                else_branch: Some(vec![]),
                span: Span::new(0, 1, 1, 1),
            },
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        let result = checker.check(&program);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("different stack effects"));
    }
    
    #[test]
    fn test_if_no_else_must_be_balanced() {
        let mut checker = Checker::new();
        // def bad (a --) if 1 end end -- no else, but then pushes
        let word = make_word("bad", 1, 0, vec![
            Expr::If {
                then_branch: vec![int_literal(1)],
                else_branch: None,
                span: Span::new(0, 1, 1, 1),
            },
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        let result = checker.check(&program);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_while_body_must_be_balanced() {
        let mut checker = Checker::new();
        // def bad (--) while true do 1 end end -- body pushes
        let word = make_word("bad", 0, 0, vec![
            Expr::While {
                cond: vec![Expr::Literal(Literal::Bool(true))],
                body: vec![int_literal(1)],
                span: Span::new(0, 1, 1, 1),
            },
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        let result = checker.check(&program);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("net zero effect"));
    }
    
    #[test]
    fn test_while_correct() {
        let mut checker = Checker::new();
        // def countdown (n --) while dup 0 > do 1 - end drop end
        let word = make_word("countdown", 1, 0, vec![
            Expr::While {
                cond: vec![
                    word_call("dup"),
                    int_literal(0),
                    word_call(">"),
                ],
                body: vec![
                    int_literal(1),
                    word_call("-"),
                ],
                span: Span::default(),
            },
            word_call("drop"),
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        assert!(checker.check(&program).is_ok());
    }
    
    #[test]
    fn test_times_body_must_be_balanced() {
        let mut checker = Checker::new();
        // def bad (--) 5 times 1 end end -- body pushes
        let word = make_word("bad", 0, 0, vec![
            int_literal(5),
            Expr::Times {
                body: vec![int_literal(1)],
                span: Span::new(0, 1, 1, 1),
            },
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        let result = checker.check(&program);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_times_correct() {
        let mut checker = Checker::new();
        // def print5 (--) 5 times "hi" print end end
        let word = make_word("print5", 0, 0, vec![
            int_literal(5),
            Expr::Times {
                body: vec![
                    Expr::Literal(Literal::String("hi".to_string())),
                    word_call("print"),
                ],
                span: Span::default(),
            },
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        assert!(checker.check(&program).is_ok());
    }
    
    #[test]
    fn test_user_defined_word_call() {
        let mut checker = Checker::new();
        // def double (n -- n) dup + end
        // def quad (n -- n) double double end
        let double = make_word("double", 1, 1, vec![
            word_call("dup"),
            word_call("+"),
        ]);
        let quad = make_word("quad", 1, 1, vec![
            word_call("double"),
            word_call("double"),
        ]);
        let program = Program { imports: Vec::new(), words: vec![double, quad] };
        assert!(checker.check(&program).is_ok());
    }
    
    #[test]
    fn test_multiple_errors_collected() {
        let mut checker = Checker::new();
        // Two bad words
        let bad1 = make_word("bad1", 0, 0, vec![word_call("drop")]);
        let bad2 = make_word("bad2", 0, 0, vec![word_call("drop")]);
        let program = Program { imports: Vec::new(), words: vec![bad1, bad2] };
        let result = checker.check(&program);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 2);
    }
    
    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("abc", "abc"), 0);
        assert_eq!(levenshtein("abc", "ab"), 1);
        assert_eq!(levenshtein("abc", "abd"), 1);
        assert_eq!(levenshtein("abc", "xyz"), 3);
    }
    
    #[test]
    fn test_if_underflow() {
        let mut checker = Checker::new();
        // def bad (--) if end end -- no condition value
        let word = make_word("bad", 0, 0, vec![
            Expr::If {
                then_branch: vec![],
                else_branch: None,
                span: Span::new(0, 1, 1, 1),
            },
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        let result = checker.check(&program);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("underflow"));
    }
    
    #[test]
    fn test_times_underflow() {
        let mut checker = Checker::new();
        // def bad (--) times end end -- no count value
        let word = make_word("bad", 0, 0, vec![
            Expr::Times {
                body: vec![],
                span: Span::new(0, 1, 1, 1),
            },
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        let result = checker.check(&program);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_nested_control_flow() {
        let mut checker = Checker::new();
        // def foo (a b -- c) if dup if 1 else 2 end else 3 end end
        let word = make_word("foo", 2, 1, vec![
            Expr::If {
                then_branch: vec![
                    Expr::If {
                        then_branch: vec![int_literal(1)],
                        else_branch: Some(vec![int_literal(2)]),
                        span: Span::default(),
                    },
                ],
                else_branch: Some(vec![
                    word_call("drop"),
                    int_literal(3),
                ]),
                span: Span::default(),
            },
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        // This should work: both outer branches produce 1 value
        assert!(checker.check(&program).is_ok());
    }
    
    #[test]
    fn test_all_builtins_registered() {
        let checker = Checker::new();
        // Verify all expected builtins are present
        let expected = [
            "dup", "drop", "swap", "over", "rot", "nip", "tuck",
            "2dup", "2drop", "2swap",
            "+", "-", "*", "/", "mod", "min", "max", "negate", "abs",
            "=", "!=", "<", ">", "<=", ">=",
            "and", "or", "not", "band", "bor", "bxor", "bnot", "shl", "shr",
            "@", "!", "c@", "c!", "alloc", "print", "emit",
        ];
        for word in expected {
            assert!(checker.words.contains_key(word), "missing builtin: {}", word);
        }
    }
    
    #[test]
    fn test_complex_arithmetic() {
        let mut checker = Checker::new();
        // def calc (a b c -- d) + * abs negate end
        // (a b c --) + gives (a d --), * gives (e --), abs gives (e --), negate gives (f --)
        let word = make_word("calc", 3, 1, vec![
            word_call("+"),
            word_call("*"),
            word_call("abs"),
            word_call("negate"),
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        assert!(checker.check(&program).is_ok());
    }
    
    #[test]
    fn test_while_bad_condition() {
        let mut checker = Checker::new();
        // def bad (--) while drop do end end -- condition consumes but doesn't produce
        let word = make_word("bad", 1, 0, vec![
            Expr::While {
                cond: vec![word_call("drop")], // consumes 1, produces 0
                body: vec![],
                span: Span::new(0, 1, 1, 1),
            },
        ]);
        let program = Program { imports: Vec::new(), words: vec![word] };
        let result = checker.check(&program);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("condition must push exactly 1"));
    }
}
