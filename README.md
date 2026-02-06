# zen-js

Zen to JavaScript transpiler — written in Zen using compile-time AST introspection.

The transpiler walks the Zen AST at comptime and emits JavaScript. No external tooling, no Rust glue — just Zen meta-programming.

## How it works

```zen
to_js = comptime (source: String) String {
    ast = meta.parse(source)
    output ::= ""
    nodes = ast.children()
    // walk AST nodes, emit JS for each
}
```

Zen's `comptime` + `meta.parse()` gives full AST access at compile time. The transpiler pattern-matches on AST node types and emits the corresponding JavaScript.

## Zen → JavaScript mapping

| Zen | JavaScript |
|---|---|
| `Name: { x: i32, y: f64 }` | `class Name { constructor(x, y) { ... } }` |
| `Color: Red, Green, Blue` | `const Color = Object.freeze({ Red: ..., ... })` |
| `x ? \| .Red { "r" } \| .Blue { "b" }` | if/else chain |
| `io.println("hi")` | `console.log("hi")` |
| `"Hello ${name}"` | `` `Hello ${name}` `` |
| `count ::= 0` | `let count = 0` |
| `x = 5` | `const x = 5` |

## Project structure

```
src/
  to_js.zen       # The transpiler — walks AST, emits JS
  js_types.zen    # Core JS type definitions (JsValue, Promise, Error, ...)
  js_dom.zen      # DOM API types (HTMLElement, Document, Event, Canvas, ...)
examples/
  hello.zen       # Hello world
  fibonacci.zen   # Recursion + pattern matching
  counter.zen     # Enums + mutable state
  todo_app.zen    # Multi-function composition
  fetch_api.zen   # Structs + string interpolation
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

- **`js_types.zen`** — Core JS types: `JsValue`, `Promise`, `Map`, `Set`
- **`js_dom.zen`** — Full DOM API: `HTMLElement`, `Document`, `Event`, `Window`, `CSSStyleDeclaration`, Canvas, Fetch, WebSocket

## Requirements

Requires the [Zen compiler](https://github.com/lantos1618/zenlang):

```bash
zen run src/to_js.zen -- examples/fibonacci.zen
```

## License

MIT
