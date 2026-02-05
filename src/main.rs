use clap::{Parser, Subcommand};
use obsidian::{checker::Checker, codegen::CodeGen, error::format_error, lexer::Lexer, parser, repl};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "obsidian")]
#[command(about = "A concatenative language that compiles to WASM")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile source to WASM binary
    Build {
        /// Input source file
        file: PathBuf,
        /// Output WASM file (default: input.wasm)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Type check source without compiling
    Check {
        /// Input source file
        file: PathBuf,
    },
    /// Compile and run source
    Run {
        /// Input source file
        file: PathBuf,
    },
    /// Start interactive REPL
    Repl,
    /// Format source file
    Fmt {
        /// Input source file
        file: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Command::Build { file, output } => cmd_build(&file, output.as_deref()),
        Command::Check { file } => cmd_check(&file),
        Command::Run { file } => cmd_run(&file),
        Command::Repl => cmd_repl(),
        Command::Fmt { file } => cmd_fmt(&file),
    };

    std::process::exit(exit_code);
}

/// Compile source to WASM binary.
fn cmd_build(file: &PathBuf, output: Option<&std::path::Path>) -> i32 {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to read {}: {}", file.display(), e);
            return 3; // IO error
        }
    };

    // Lex
    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", format_error(&source, &e.into()));
            return 1; // Parse error
        }
    };

    // Parse
    let mut parser = parser::Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", format_error(&source, &e.into()));
            return 1;
        }
    };

    // Check
    let mut checker = Checker::new();
    if let Err(errors) = checker.check(&program) {
        for e in errors {
            eprintln!("{}", format_error(&source, &e.into()));
        }
        return 2; // Type/check error
    }

    // Generate WASM
    let mut codegen = CodeGen::new();
    let wasm = match codegen.generate(&program) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: codegen failed: {}", e);
            return 2;
        }
    };

    // Write output
    let output_path = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| file.with_extension("wasm"));

    if let Err(e) = std::fs::write(&output_path, &wasm) {
        eprintln!("error: failed to write {}: {}", output_path.display(), e);
        return 3;
    }

    println!(
        "Compiled {} -> {} ({} bytes)",
        file.display(),
        output_path.display(),
        wasm.len()
    );
    0
}

/// Type check source without compiling.
fn cmd_check(file: &PathBuf) -> i32 {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to read {}: {}", file.display(), e);
            return 3;
        }
    };

    // Lex
    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", format_error(&source, &e.into()));
            return 1;
        }
    };

    // Parse
    let mut parser = parser::Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", format_error(&source, &e.into()));
            return 1;
        }
    };

    // Check
    let mut checker = Checker::new();
    if let Err(errors) = checker.check(&program) {
        for e in errors {
            eprintln!("{}", format_error(&source, &e.into()));
        }
        return 2;
    }

    println!("OK: {} words checked", program.words.len());
    0
}

/// Compile and run source with wasmtime.
fn cmd_run(file: &PathBuf) -> i32 {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to read {}: {}", file.display(), e);
            return 3;
        }
    };

    // Lex
    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", format_error(&source, &e.into()));
            return 1;
        }
    };

    // Parse
    let mut parser = parser::Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", format_error(&source, &e.into()));
            return 1;
        }
    };

    // Check
    let mut checker = Checker::new();
    if let Err(errors) = checker.check(&program) {
        for e in errors {
            eprintln!("{}", format_error(&source, &e.into()));
        }
        return 2;
    }

    // Generate WASM
    let mut codegen = CodeGen::new();
    let wasm = match codegen.generate(&program) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: codegen failed: {}", e);
            return 2;
        }
    };

    // Run with wasmtime
    match run_wasm(&wasm) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: runtime error: {}", e);
            1
        }
    }
}

/// Execute WASM binary with wasmtime.
fn run_wasm(wasm: &[u8]) -> Result<(), String> {
    use wasmtime::{Engine, Instance, Module, Store};

    let engine = Engine::default();
    let module =
        Module::from_binary(&engine, wasm).map_err(|e| format!("invalid wasm: {}", e))?;

    let mut store = Store::new(&engine, ());
    let instance =
        Instance::new(&mut store, &module, &[]).map_err(|e| format!("instantiation: {}", e))?;

    // Try to call _start if it exists
    if let Some(start) = instance.get_func(&mut store, "_start") {
        start
            .call(&mut store, &[], &mut [])
            .map_err(|e| format!("_start: {}", e))?;
    }

    Ok(())
}

/// Start interactive REPL.
fn cmd_repl() -> i32 {
    repl::run()
}

/// Format source file (pretty-print).
fn cmd_fmt(file: &PathBuf) -> i32 {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to read {}: {}", file.display(), e);
            return 3;
        }
    };

    // Lex
    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", format_error(&source, &e.into()));
            return 1;
        }
    };

    // Parse
    let mut parser = parser::Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", format_error(&source, &e.into()));
            return 1;
        }
    };

    // Pretty print
    let formatted = format_program(&program);

    // Write back
    if let Err(e) = std::fs::write(file, &formatted) {
        eprintln!("error: failed to write {}: {}", file.display(), e);
        return 3;
    }

    println!("Formatted {}", file.display());
    0
}

/// Format a program as source code.
fn format_program(program: &obsidian::ast::Program) -> String {

    let mut out = String::new();

    for word in &program.words {
        out.push_str("def ");
        out.push_str(&word.name);
        out.push_str(" (");

        // Inputs
        for (i, item) in word.effect.inputs.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            if let Some(name) = &item.name {
                out.push_str(name);
            }
            if let Some(typ) = &item.typ {
                out.push_str(": ");
                out.push_str(&format!("{}", typ));
            }
        }

        out.push_str(" -- ");

        // Outputs
        for (i, item) in word.effect.outputs.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            if let Some(name) = &item.name {
                out.push_str(name);
            }
            if let Some(typ) = &item.typ {
                out.push_str(": ");
                out.push_str(&format!("{}", typ));
            }
        }

        out.push_str(")\n");

        // Body
        format_body(&mut out, &word.body, 1);

        out.push_str("end\n\n");
    }

    out
}

fn format_body(out: &mut String, body: &[obsidian::ast::Expr], indent: usize) {
    use obsidian::ast::{Expr, Literal};

    let prefix = "  ".repeat(indent);

    for expr in body {
        out.push_str(&prefix);
        match expr {
            Expr::Literal(lit) => {
                match lit {
                    Literal::Integer(n) => out.push_str(&n.to_string()),
                    Literal::Float(f) => out.push_str(&f.to_string()),
                    Literal::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
                    Literal::String(s) => {
                        out.push('"');
                        out.push_str(&s.replace('\\', "\\\\").replace('"', "\\\""));
                        out.push('"');
                    }
                }
                out.push('\n');
            }
            Expr::Word { name, .. } => {
                out.push_str(name);
                out.push('\n');
            }
            Expr::If {
                then_branch,
                else_branch,
                ..
            } => {
                out.push_str("if\n");
                format_body(out, then_branch, indent + 1);
                if let Some(else_body) = else_branch {
                    out.push_str(&prefix);
                    out.push_str("else\n");
                    format_body(out, else_body, indent + 1);
                }
                out.push_str(&prefix);
                out.push_str("end\n");
            }
            Expr::While { cond, body, .. } => {
                out.push_str("while\n");
                format_body(out, cond, indent + 1);
                out.push_str(&prefix);
                out.push_str("do\n");
                format_body(out, body, indent + 1);
                out.push_str(&prefix);
                out.push_str("end\n");
            }
            Expr::Times { body, .. } => {
                out.push_str("times\n");
                format_body(out, body, indent + 1);
                out.push_str(&prefix);
                out.push_str("end\n");
            }
        }
    }
}
