# zen-js

Zen to JavaScript transpiler — written in Zen using compile-time AST introspection.

`meta.parse()` gives full AST access at compile time. The transpiler pattern-matches on node types and emits JavaScript.

```zen
to_js = (node: meta.ASTNode) String {
    node.variant_name() ?
        | "Function"   { return emit_function(node) }
        | "Struct"     { return emit_struct(node) }
        | "Enum"       { return emit_enum(node) }
        | "BinaryOp"   { return emit_binary_op(node) }
        | "Identifier"  { return node.name }
        | ...
}
```

## Zen → JS mapping

| Zen | JavaScript |
|---|---|
| `Point: { x: f64, y: f64 }` | `class Point { constructor(x, y) { ... } }` |
| `Color: Red, Green, Blue` | `const Color = Object.freeze({ ... })` |
| `x ? \| .Red { "r" } \| .Blue { "b" }` | if/else chain |
| `io.println("hi")` | `console.log("hi")` |
| `"Hello ${name}"` | `` `Hello ${name}` `` |
| `count ::= 0` | `let count = 0` |
| `x = 5` | `const x = 5` |

## Structure

```
src/
  to_js.zen        # Main walker — dispatches AST nodes
  emit_decl.zen    # Declaration emitters (program, function, struct, enum)
  emit_expr.zen    # Expression emitters (binary ops, calls, match, closures)
  emit_stmt.zen    # Statement emitters + shared helpers
  js/
    types.zen      # Core JS types (JsValue, errors)
    dom.zen        # DOM elements (HTMLElement, HTMLInputElement, ...)
    events.zen     # Event types (MouseEvent, KeyboardEvent, ...)
    css.zen        # CSS types (CSSStyleDeclaration)
    apis.zen       # Browser APIs (Document, Window, Fetch, Canvas, WebSocket)
examples/
  hello.zen        # Hello world
  fibonacci.zen    # Recursion + loops
  counter.zen      # Enums + mutable state
  todo_app.zen     # Multi-function composition
  fetch_api.zen    # Structs + struct literals
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
    io.println("Fibonacci sequence:")
    i ::= 0
    loop i <= 10 {
        io.println("  fib(${i}) = ${fibonacci(i)}")
        i = i + 1
    }
    return 0
}
```

### counter.zen

```zen
{ io } = @std

Action: Increment, Decrement, Reset

apply = (count: i32, action: Action) i32 {
    action ?
        | .Increment { return count + 1 }
        | .Decrement { return count - 1 }
        | .Reset     { return 0 }
}
```

## Requirements

[Zen compiler](https://github.com/lantos1618/zenlang)

## License

MIT
