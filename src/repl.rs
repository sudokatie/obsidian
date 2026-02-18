use crate::checker::Checker;
use crate::error::format_error;
use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::parser;

/// Start interactive REPL.
pub fn run() -> i32 {
    println!("Obsidian REPL v0.2.0");
    println!("Type :help for commands, :quit to exit\n");

    let mut rl = match rustyline::DefaultEditor::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: failed to initialize readline: {}", e);
            return 1;
        }
    };

    let mut checker = Checker::new();
    let mut interp = Interpreter::new();

    loop {
        let prompt = if interp.trace_enabled() { "[trace] > " } else { "> " };
        let line = match rl.readline(prompt) {
            Ok(l) => l,
            Err(rustyline::error::ReadlineError::Interrupted) => continue,
            Err(rustyline::error::ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("error: {}", e);
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // REPL commands
        if trimmed.starts_with(':') {
            match trimmed {
                ":quit" | ":q" => break,
                ":help" | ":h" => {
                    println!("Commands:");
                    println!("  :help, :h     Show this help");
                    println!("  :quit, :q     Exit REPL");
                    println!("  :stack, :s    Display current stack");
                    println!("  :clear        Clear stack");
                    println!("  :trace        Toggle trace mode (show stack after each op)");
                    println!("  :reset        Clear stack and defined words");
                    println!("\nEnter Obsidian code to evaluate.");
                    println!("\nExamples:");
                    println!("  5 3 +         Push 5 and 3, add them");
                    println!("  dup *         Duplicate top, multiply (square)");
                    println!("  : square ( n -- n ) dup * ;");
                }
                ":stack" | ":s" => {
                    println!("{}", interp.format_stack());
                }
                ":clear" => {
                    interp.clear();
                    println!("Stack cleared.");
                }
                ":trace" => {
                    let new_state = !interp.trace_enabled();
                    interp.set_trace(new_state);
                    println!("Trace mode: {}", if new_state { "ON" } else { "OFF" });
                }
                ":reset" => {
                    interp = Interpreter::new();
                    checker = Checker::new();
                    println!("Interpreter reset.");
                }
                _ => {
                    println!("Unknown command: {}", trimmed);
                    println!("Type :help for available commands.");
                }
            }
            continue;
        }

        let _ = rl.add_history_entry(&line);

        // Try to parse and check
        let mut lexer = Lexer::new(&line);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{}", format_error(&line, &e.into()));
                continue;
            }
        };

        let mut parser = parser::Parser::new(tokens);
        let program = match parser.parse() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{}", format_error(&line, &e.into()));
                continue;
            }
        };

        if let Err(errors) = checker.check(&program) {
            for e in errors {
                eprintln!("{}", format_error(&line, &e.into()));
            }
            continue;
        }

        // Load any word definitions
        if !program.words.is_empty() {
            interp.load_program(&program);
            for word in &program.words {
                println!("Defined: {}", word.name);
            }
        }

        // Execute top-level expressions (if any)
        // In Obsidian, the top-level is just word definitions or direct expressions
        // For REPL, we parse bare expressions as a word body and execute them
        if program.words.is_empty() {
            // Re-parse as expression sequence
            let mut lexer2 = Lexer::new(&line);
            let tokens2 = match lexer2.tokenize() {
                Ok(t) => t,
                Err(_) => continue,
            };

            let mut parser2 = parser::Parser::new(tokens2);
            let exprs = match parser2.parse_expr_list() {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("{}", format_error(&line, &e.into()));
                    continue;
                }
            };

            if let Err(e) = interp.execute(&exprs) {
                eprintln!("error: {}", e);
                continue;
            }

            // Show stack after execution (unless trace is on, which already shows it)
            if !interp.trace_enabled() && !exprs.is_empty() {
                println!("{}", interp.format_stack());
            }
        }
    }

    println!("Goodbye!");
    0
}
