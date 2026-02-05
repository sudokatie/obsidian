// Integration tests: compile examples, validate WASM, check sizes

use std::fs;

use obsidian::lexer::Lexer;
use obsidian::parser::Parser;
use obsidian::checker::Checker;
use obsidian::codegen::CodeGen;

fn compile_file(path: &str) -> Result<Vec<u8>, String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path, e))?;
    
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize()
        .map_err(|e| format!("lexer error: {:?}", e))?;
    
    let mut parser = Parser::new(tokens);
    let program = parser.parse()
        .map_err(|e| format!("parser error: {:?}", e))?;
    
    let mut checker = Checker::new();
    checker.check(&program)
        .map_err(|e| format!("checker error: {:?}", e))?;
    
    let mut codegen = CodeGen::new();
    let wasm = codegen.generate(&program)
        .map_err(|e| format!("codegen error: {:?}", e))?;
    
    Ok(wasm)
}

// Validate WASM magic bytes and basic structure
fn validate_wasm(wasm: &[u8]) -> bool {
    // WASM magic: \0asm
    if wasm.len() < 8 {
        return false;
    }
    wasm[0] == 0x00 && wasm[1] == 0x61 && wasm[2] == 0x73 && wasm[3] == 0x6d
}

// --- Compilation Tests ---

#[test]
fn test_hello_compiles() {
    let result = compile_file("examples/hello.obs");
    assert!(result.is_ok(), "hello.obs should compile: {:?}", result.err());
}

#[test]
fn test_factorial_compiles() {
    let result = compile_file("examples/factorial.obs");
    assert!(result.is_ok(), "factorial.obs should compile: {:?}", result.err());
}

#[test]
fn test_fibonacci_compiles() {
    let result = compile_file("examples/fibonacci.obs");
    assert!(result.is_ok(), "fibonacci.obs should compile: {:?}", result.err());
}

#[test]
fn test_arithmetic_compiles() {
    let result = compile_file("examples/arithmetic.obs");
    assert!(result.is_ok(), "arithmetic.obs should compile: {:?}", result.err());
}

#[test]
fn test_countdown_compiles() {
    let result = compile_file("examples/countdown.obs");
    assert!(result.is_ok(), "countdown.obs should compile: {:?}", result.err());
}

// --- WASM Validity Tests ---

#[test]
fn test_hello_wasm_valid() {
    let wasm = compile_file("examples/hello.obs").expect("should compile");
    assert!(validate_wasm(&wasm), "hello.obs should produce valid WASM");
}

#[test]
fn test_factorial_wasm_valid() {
    let wasm = compile_file("examples/factorial.obs").expect("should compile");
    assert!(validate_wasm(&wasm), "factorial.obs should produce valid WASM");
}

#[test]
fn test_fibonacci_wasm_valid() {
    let wasm = compile_file("examples/fibonacci.obs").expect("should compile");
    assert!(validate_wasm(&wasm), "fibonacci.obs should produce valid WASM");
}

// --- Compilation Tests (additional) ---

#[test]
fn test_fizzbuzz_compiles() {
    let result = compile_file("examples/fizzbuzz.obs");
    assert!(result.is_ok(), "fizzbuzz.obs should compile: {:?}", result.err());
}

// --- Size Tests ---

#[test]
fn test_hello_wasm_size() {
    let wasm = compile_file("examples/hello.obs").expect("should compile");
    assert!(wasm.len() < 1024, "hello.wasm should be under 1KB, got {} bytes", wasm.len());
}

#[test]
fn test_factorial_wasm_size() {
    let wasm = compile_file("examples/factorial.obs").expect("should compile");
    assert!(wasm.len() < 1024, "factorial.wasm should be under 1KB, got {} bytes", wasm.len());
}

#[test]
fn test_fibonacci_wasm_size() {
    let wasm = compile_file("examples/fibonacci.obs").expect("should compile");
    assert!(wasm.len() < 1024, "fibonacci.wasm should be under 1KB, got {} bytes", wasm.len());
}

// --- Error Detection Tests ---

#[test]
fn test_undefined_word_error() {
    let source = r#"
def main (--)
  undefined_word
end
"#;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("lexer ok");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parser ok");
    let mut checker = Checker::new();
    let result = checker.check(&program);
    assert!(result.is_err(), "undefined word should cause error");
}

#[test]
fn test_stack_underflow_error() {
    let source = r#"
def main (--)
  +
end
"#;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("lexer ok");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parser ok");
    let mut checker = Checker::new();
    let result = checker.check(&program);
    assert!(result.is_err(), "stack underflow should cause error");
}

#[test]
fn test_effect_mismatch_error() {
    let source = r#"
def wrong (-- n)
end
"#;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("lexer ok");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parser ok");
    let mut checker = Checker::new();
    let result = checker.check(&program);
    assert!(result.is_err(), "effect mismatch should cause error");
}
