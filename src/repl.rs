use crate::checker::Checker;
use crate::error::format_error;
use crate::lexer::Lexer;
use crate::parser;

/// Start interactive REPL.
pub fn run() -> i32 {
    println!("Obsidian REPL v0.1.0");
    println!("Type :help for commands, :quit to exit\n");

    let mut rl = match rustyline::DefaultEditor::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: failed to initialize readline: {}", e);
            return 1;
        }
    };

    let mut checker = Checker::new();

    loop {
        let line = match rl.readline("> ") {
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
                    println!("  :help, :h   Show this help");
                    println!("  :quit, :q   Exit REPL");
                    println!("  :clear      Clear screen");
                    println!("\nEnter Obsidian code to evaluate.");
                }
                ":clear" => {
                    print!("\x1b[2J\x1b[H");
                }
                _ => {
                    println!("Unknown command: {}", trimmed);
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

        println!("OK: {} words defined", program.words.len());
    }

    println!("Goodbye!");
    0
}
