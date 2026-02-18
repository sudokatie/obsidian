# Obsidian

A concatenative language that compiles to WebAssembly. Stack-based, tiny runtime, zero dependencies at runtime.

## Why This Exists

Sometimes you need code that runs anywhere, fits in a few kilobytes, and doesn't bring along half of npm. Obsidian compiles to WASM small enough to fit in a QR code. The factorial function compiles to under 200 bytes.

Also, Forth is beautiful but stuck in the 1970s. This is what happens when you give it a modern type system and target the web.

## Quick Start

```bash
# Install from source
git clone https://github.com/sudokatie/obsidian
cd obsidian
cargo install --path .

# Hello world
echo 'def main (--) 42 print end' > hello.obs
obsidian run hello.obs

# Compile to WASM
obsidian build hello.obs -o hello.wasm
```

## Language Overview

Obsidian is concatenative: functions compose by juxtaposition. No parentheses, no commas, no ceremony.

```obsidian
-- Push values onto the stack
42 3.14 true "hello"

-- Arithmetic (postfix)
2 3 +        -- 5
10 4 - 2 *   -- 12

-- Stack manipulation
dup          -- duplicate top
drop         -- discard top
swap         -- swap top two
over         -- copy second to top
rot          -- rotate top three
```

### Word Definitions

```obsidian
def square (n -- n*n)
  dup *
end

def factorial (n -- n!)
  1 swap
  while dup 0 > do
    swap over *
    swap 1 -
  end
  drop
end
```

Stack effects `(inputs -- outputs)` are mandatory. The compiler verifies them.

### Control Flow

```obsidian
-- Conditionals
x 0 > if
  "positive" print
else
  "not positive" print
end

-- Loops
10 while dup 0 > do
  dup print
  1 -
end
drop

-- Counted iteration
5 times
  "hello" print
end
```

### Types

Obsidian has five primitive types:

| Type | Description |
|------|-------------|
| `i32` | 32-bit integer |
| `i64` | 64-bit integer |
| `f32` | 32-bit float |
| `f64` | 64-bit float |
| `bool` | Boolean |

Type annotations in stack effects are optional but recommended:

```obsidian
def add-i64 (a:i64 b:i64 -- sum:i64)
  +
end
```

## CLI Reference

```
obsidian build <file> [-o output.wasm]   Compile to WASM
obsidian check <file>                    Type check only
obsidian run <file>                      Compile and execute
obsidian repl                            Interactive mode
obsidian fmt <file>                      Format source
```

### REPL Commands

```
:help, :h      Show commands
:stack, :s     Display current stack
:clear         Clear stack
:trace         Toggle trace mode (shows stack after each operation)
:reset         Clear stack and defined words
:quit, :q      Exit
```

The REPL now executes code interactively with a built-in interpreter:

```
> 5 3 +
<8>
> dup *
<64>
> :trace
Trace mode: ON
[trace] > 2 3 +
  push -> <64 2>
  push -> <64 2 3>
  + -> <64 5>
<64 5>
```

## Built-in Words

### Stack Operations

| Word | Effect | Description |
|------|--------|-------------|
| `dup` | `(a -- a a)` | Duplicate top |
| `drop` | `(a --)` | Discard top |
| `swap` | `(a b -- b a)` | Swap top two |
| `over` | `(a b -- a b a)` | Copy second to top |
| `rot` | `(a b c -- b c a)` | Rotate top three |
| `-rot` | `(a b c -- c a b)` | Reverse rotate |
| `nip` | `(a b -- b)` | Drop second |
| `tuck` | `(a b -- b a b)` | Copy top under second |
| `2dup` | `(a b -- a b a b)` | Duplicate pair |
| `2drop` | `(a b --)` | Drop pair |
| `2swap` | `(a b c d -- c d a b)` | Swap pairs |
| `2over` | `(a b c d -- a b c d a b)` | Copy second pair |

### Arithmetic

| Word | Effect | Description |
|------|--------|-------------|
| `+` | `(a b -- sum)` | Add |
| `-` | `(a b -- diff)` | Subtract |
| `*` | `(a b -- prod)` | Multiply |
| `/` | `(a b -- quot)` | Divide |
| `mod` | `(a b -- rem)` | Remainder |
| `negate` | `(n -- -n)` | Negate |
| `abs` | `(n -- abs)` | Absolute value |
| `min` | `(a b -- min)` | Minimum |
| `max` | `(a b -- max)` | Maximum |
| `clamp` | `(val lo hi -- clamped)` | Clamp to range |
| `sqr` | `(n -- n*n)` | Square |

### Comparison

| Word | Effect | Description |
|------|--------|-------------|
| `=` | `(a b -- bool)` | Equal |
| `!=` | `(a b -- bool)` | Not equal |
| `<` | `(a b -- bool)` | Less than |
| `>` | `(a b -- bool)` | Greater than |
| `<=` | `(a b -- bool)` | Less or equal |
| `>=` | `(a b -- bool)` | Greater or equal |

### Logic

| Word | Effect | Description |
|------|--------|-------------|
| `and` | `(a b -- a&b)` | Logical and |
| `or` | `(a b -- a|b)` | Logical or |
| `not` | `(a -- !a)` | Logical not |

### Memory

| Word | Effect | Description |
|------|--------|-------------|
| `@` | `(addr -- val)` | Fetch i64 from address |
| `!` | `(val addr --)` | Store i64 at address |
| `c@` | `(addr -- byte)` | Fetch byte |
| `c!` | `(byte addr --)` | Store byte |
| `alloc` | `(size -- addr)` | Allocate memory |

## Examples

### Factorial

```obsidian
def factorial (n -- result)
  1 swap
  while dup 0 > do
    swap over *
    swap 1 -
  end
  drop
end

def main (--)
  5 factorial print  -- 120
end
```

### Fibonacci

```obsidian
def fib (n -- fib-n)
  dup 1 <= if
    -- base case
  else
    dup 1 - fib
    swap 2 - fib
    +
  end
end

def main (--)
  10 fib print  -- 55
end
```

## Building from Source

```bash
# Requirements: Rust 1.70+
git clone https://github.com/sudokatie/obsidian
cd obsidian
cargo build --release
```

Binary will be at `target/release/obsidian`.

### Running Tests

```bash
cargo test                    # All tests
cargo test --test integration # Integration tests only
```

## Roadmap

### v0.2 (Released)
- [x] Standard library (abs, min, max, stack ops, memory ops)
- [x] String interning and memory layout

### v0.3 (In Progress)
- [x] Interactive interpreter in REPL
- [x] Trace mode (stack after each operation)
- [ ] Step mode (execute one word at a time)
- [ ] Breakpoints
- [ ] Stack trace on error
- [ ] IDE integration (VS Code extension)

See FEATURE-BACKLOG.md in the clawd repo for detailed acceptance criteria.

## Technical Details

- Compiles directly to WASM binary (not WAT text format)
- No runtime dependencies - output is standalone WASM
- Stack effect verification at compile time
- Generated WASM typically under 1KB for simple programs

## License

MIT

## Author

Katie

---

*Stack-based programming: where composition is concatenation and reading right-to-left eventually becomes natural.*
