# Changelog

All notable changes to the Obsidian VS Code extension.

## [0.1.0] - 2026-02-18

### Added
- Initial release
- Syntax highlighting for `.obs` files
- TextMate grammar covering:
  - Keywords (def, end, if, else, while, do, times)
  - Stack operations (dup, drop, swap, over, rot, etc.)
  - Arithmetic and comparison operators
  - Memory operations (@, !, c@, c!, alloc)
  - Types (i32, i64, f32, f64, bool)
  - Stack effect annotations
  - Comments (-- style)
  - String literals with escape sequences
  - Numeric literals (decimal, hex, binary, float)
- Language configuration:
  - Comment toggling
  - Bracket matching for control structures
  - Auto-closing pairs
- Code snippets:
  - def, main, if, ifelse, while, times
  - square, factorial examples
