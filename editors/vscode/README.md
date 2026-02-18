# Obsidian Language Support for VS Code

Syntax highlighting and language support for Obsidian, a stack-based language that compiles to WASM.

## Features

- Syntax highlighting for `.obs` files
- Comment toggling (`--`)
- Bracket matching for control structures

## Highlighted Elements

- Keywords: `def`, `end`, `if`, `else`, `while`, `do`, `times`
- Stack operations: `dup`, `drop`, `swap`, `over`, `rot`, etc.
- Arithmetic and comparison operators
- Memory operations: `@`, `!`, `c@`, `c!`, `alloc`
- Types: `i32`, `i64`, `f32`, `f64`, `bool`
- Stack effect annotations: `(a b -- c)`
- Comments: `-- comment`
- Strings with escape sequences
- Numbers (decimal, hex, binary, float)

## Installation

### From Source

1. Copy this directory to your VS Code extensions folder:
   - Windows: `%USERPROFILE%\.vscode\extensions\obsidian-lang`
   - macOS: `~/.vscode/extensions/obsidian-lang`
   - Linux: `~/.vscode/extensions/obsidian-lang`

2. Restart VS Code

### Manual Package

```bash
cd editors/vscode
npm install -g vsce
vsce package
code --install-extension obsidian-lang-0.1.0.vsix
```

## Example

```obsidian
-- Calculate factorial
def factorial (n -- result)
  1 swap
  while dup 0 > do
    swap over *
    swap 1 -
  end
  drop
end

def main (--)
  5 factorial print
end
```

## License

MIT
