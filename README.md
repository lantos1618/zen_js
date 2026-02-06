# zen-js

Zen to JavaScript transpiler — powered by Zen's AST introspection and meta-programming system.

Parses `.zen` source files using the Zen compiler frontend and emits clean, readable JavaScript that runs in Node.js or the browser.

## What it does

```
┌──────────┐     ┌────────────┐     ┌────────────┐     ┌──────────┐
│ .zen file │ ──▶ │ Zen Lexer  │ ──▶ │ Zen Parser │ ──▶ │ JS Emit  │
│           │     │            │     │   (AST)    │     │          │
└──────────┘     └────────────┘     └────────────┘     └──────────┘
```

The emitter walks the Zen AST and produces JavaScript:

| Zen construct | JavaScript output |
|---|---|
| `Name: { x: i32, y: f64 }` | `class Name { constructor(x, y) { ... } }` |
| `Color: Red, Green, Blue` | `const Color = Object.freeze({ Red: ..., Green: ... })` |
| `x ? \| .Red { "r" } \| .Blue { "b" }` | IIFE with if/else chain |
| `io.println("hi")` | `console.log("hi")` |
| `"Hello ${name}"` | `` `Hello ${name}` `` |
| `count ::= 0` | `let count = 0` |
| `x = 5` | `const x = 5` |

## Usage

```bash
# Transpile to stdout
zen-js examples/fibonacci.zen

# Pipe to Node.js
zen-js examples/counter.zen | node

# Write .js file
zen-js examples/hello.zen -o
```

## Examples

### fibonacci.zen

```zen
{ io } = @std

fibonacci = (n: i32) i32 {
    n <= 1 ?
        | true { return n }
        | false { return fibonacci(n - 1) + fibonacci(n - 2) }
}

main = () i32 {
    io.println("fib(10) = ${fibonacci(10)}")
    return 0
}
```

Transpiles to:

```javascript
function fibonacci(n) {
  return ((__match) => {
    if (__match === true) {
      return (() => { return n; })();
    } else if (__match === false) {
      return (() => { return (fibonacci((n - 1)) + fibonacci((n - 2))); })();
    }
  })((n <= 1));
}

function main() {
  console.log(`fib(10) = ${fibonacci(10)}`);
  return 0;
}

main();
```

### counter.zen (enums + pattern matching)

```zen
{ io } = @std

Action: Increment, Decrement, Reset

apply_action = (count: i32, action: Action) i32 {
    action ?
        | .Increment { return count + 1 }
        | .Decrement { return count - 1 }
        | .Reset { return 0 }
}

main = () i32 {
    count ::= 0
    count = apply_action(count, .Increment)
    io.println("After increment: ${count}")
    return 0
}
```

### todo_app.zen (multi-function composition)

```zen
{ io } = @std

Priority: Low, Medium, High

priority_label = (p: Priority) String {
    p ?
        | .Low { return "low" }
        | .Medium { return "med" }
        | .High { return "HIGH" }
}

format_todo = (id: i32, text: String, done: bool, priority: Priority) String {
    status = done ?
        | true { "[x]" }
        | false { "[ ]" }
    return "${status} #${id} [${priority_label(priority)}] ${text}"
}
```

## JS DOM Type Definitions

`src/` includes Zen type definitions for browser APIs:

- **`js_types.zen`** — Core JS types: `JsValue`, `Promise`, `Map`, `Set`, `JSON`
- **`js_dom.zen`** — Full DOM API: `HTMLElement`, `Document`, `Event`, `Window`, `CSSStyleDeclaration`, Canvas, Fetch, WebSocket

These define the type surface that Zen programs targeting JS can use.

## The Vision: `spec/to_js.zen`

The `spec/to_js.zen` file shows how this transpiler will eventually be written *in Zen itself* using compile-time meta-programming:

```zen
to_js = comptime (source: String) String {
    ast = meta.parse(source)
    output ::= ""
    nodes = ast.children()
    // ... walk AST and emit JS at compile time
}
```

When Zen's comptime system is complete, the Rust emitter becomes unnecessary — the transpiler becomes a Zen program that walks its own AST.

## Building

Requires the [Zen compiler](https://github.com/lantos1618/zenlang) as a sibling directory:

```bash
git clone https://github.com/lantos1618/zenlang.git
git clone https://github.com/lantos1618/zen_js.git
cd zen_js
cargo build --release
```

## Tests

```bash
cargo test
```

17 integration tests covering: function declarations, enums, structs, pattern matching, string interpolation, variable scoping, IO mappings, and full example transpilation.

## License

MIT
