use clap::{Parser, Subcommand};
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
    
    let result = match cli.command {
        Command::Build { file, output } => cmd_build(&file, output.as_deref()),
        Command::Check { file } => cmd_check(&file),
        Command::Run { file } => cmd_run(&file),
        Command::Repl => cmd_repl(),
        Command::Fmt { file } => cmd_fmt(&file),
    };
    
    if let Err(e) = result {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn cmd_build(file: &PathBuf, output: Option<&std::path::Path>) -> Result<(), String> {
    let _source = std::fs::read_to_string(file)
        .map_err(|e| format!("failed to read {}: {}", file.display(), e))?;
    
    let output_path = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| file.with_extension("wasm"));
    
    // TODO: Implement compilation
    println!("Would compile {} to {}", file.display(), output_path.display());
    
    Ok(())
}

fn cmd_check(file: &PathBuf) -> Result<(), String> {
    let _source = std::fs::read_to_string(file)
        .map_err(|e| format!("failed to read {}: {}", file.display(), e))?;
    
    // TODO: Implement type checking
    println!("Would check {}", file.display());
    
    Ok(())
}

fn cmd_run(file: &PathBuf) -> Result<(), String> {
    let _source = std::fs::read_to_string(file)
        .map_err(|e| format!("failed to read {}: {}", file.display(), e))?;
    
    // TODO: Implement run
    println!("Would run {}", file.display());
    
    Ok(())
}

fn cmd_repl() -> Result<(), String> {
    println!("Obsidian REPL - type :quit to exit");
    // TODO: Implement REPL
    Ok(())
}

fn cmd_fmt(file: &PathBuf) -> Result<(), String> {
    let _source = std::fs::read_to_string(file)
        .map_err(|e| format!("failed to read {}: {}", file.display(), e))?;
    
    // TODO: Implement formatter
    println!("Would format {}", file.display());
    
    Ok(())
}
