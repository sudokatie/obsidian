# Changelog

All notable changes to Obsidian will be documented in this file.

## [0.1.0] - 2026-02-05

Initial release.

### Features

- Concatenative language with stack-based semantics
- Compiles directly to WebAssembly binary
- Stack effect verification at compile time
- Five primitive types: i32, i64, f32, f64, bool
- Control flow: if/else/end, while/do/end, times/end
- Stack manipulation: dup, drop, swap, over, rot
- Arithmetic, comparison, and logic operators
- CLI with build, check, run, repl, and fmt commands
- Interactive REPL with readline support
- Error messages with source context and line numbers
- Example programs: hello, factorial, fibonacci, arithmetic, countdown

### Technical

- 139 tests (125 unit, 14 integration)
- Output typically under 1KB for simple programs
- No runtime dependencies in generated WASM
