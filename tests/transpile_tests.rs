use zen_js::transpile;

// ============================================================================
// BASIC TRANSPILATION
// ============================================================================

#[test]
fn test_hello_world() {
    let source = r#"
        { io } = @std
        main = () i32 {
            io.println("Hello, world!")
            return 0
        }
    "#;
    let js = transpile(source).unwrap();
    assert!(js.contains("console.log(\"Hello, world!\")"));
    assert!(js.contains("main();"));
}

#[test]
fn test_function_declaration() {
    let source = r#"
        add = (a: i32, b: i32) i32 {
            return a + b
        }
    "#;
    let js = transpile(source).unwrap();
    assert!(js.contains("function add(a, b)"));
    assert!(js.contains("return (a + b)"));
}

#[test]
fn test_string_interpolation() {
    let source = r#"
        { io } = @std
        greet = (name: String) String {
            return "Hello, ${name}!"
        }
    "#;
    let js = transpile(source).unwrap();
    assert!(js.contains("`Hello, ${name}!`"));
}

// ============================================================================
// ENUMS AND PATTERN MATCHING
// ============================================================================

#[test]
fn test_enum_definition() {
    let source = r#"
        Color: Red, Green, Blue
    "#;
    let js = transpile(source).unwrap();
    assert!(js.contains("const Color = Object.freeze({"));
    assert!(js.contains("Red: Object.freeze({ tag: \"Red\" })"));
    assert!(js.contains("Green: Object.freeze({ tag: \"Green\" })"));
    assert!(js.contains("Blue: Object.freeze({ tag: \"Blue\" })"));
}

#[test]
fn test_pattern_matching_enum() {
    let source = r#"
        Status: Active, Inactive

        check = (s: Status) i32 {
            s ?
                | .Active { return 1 }
                | .Inactive { return 0 }
        }
    "#;
    let js = transpile(source).unwrap();
    assert!(js.contains("__match.tag === \"Active\""));
    assert!(js.contains("__match.tag === \"Inactive\""));
}

#[test]
fn test_pattern_matching_bool() {
    let source = r#"
        check = (b: bool) String {
            b ?
                | true { return "yes" }
                | false { return "no" }
        }
    "#;
    let js = transpile(source).unwrap();
    assert!(js.contains("__match === true"));
    assert!(js.contains("__match === false"));
}

// ============================================================================
// STRUCT DEFINITIONS
// ============================================================================

#[test]
fn test_struct_definition() {
    let source = r#"
        Point: {
            x: f64,
            y: f64,
        }
    "#;
    let js = transpile(source).unwrap();
    assert!(js.contains("class Point"));
    assert!(js.contains("constructor(x, y)"));
    assert!(js.contains("this.x = x"));
    assert!(js.contains("this.y = y"));
}

// ============================================================================
// VARIABLE DECLARATIONS
// ============================================================================

#[test]
fn test_mutable_variable() {
    let source = r#"
        main = () i32 {
            count ::= 0
            count = 1
            return count
        }
    "#;
    let js = transpile(source).unwrap();
    assert!(js.contains("let count = 0"));
    assert!(js.contains("count = 1;"));
    // Should NOT re-declare with const/let
    let count_decls: Vec<_> = js.match_indices("let count").collect();
    assert_eq!(count_decls.len(), 1, "count should only be declared once");
}

#[test]
fn test_immutable_variable() {
    let source = r#"
        main = () i32 {
            x = 42
            return x
        }
    "#;
    let js = transpile(source).unwrap();
    assert!(js.contains("const x = 42"));
}

// ============================================================================
// IO MAPPINGS
// ============================================================================

#[test]
fn test_io_println_mapping() {
    let source = r#"
        { io } = @std
        main = () i32 {
            io.println("test")
            return 0
        }
    "#;
    let js = transpile(source).unwrap();
    assert!(js.contains("console.log(\"test\")"));
    assert!(!js.contains("io.println"));
}

// ============================================================================
// ENUM LITERALS
// ============================================================================

#[test]
fn test_enum_literal_in_call() {
    let source = r#"
        Status: Active, Inactive

        check = (s: Status) i32 {
            return 0
        }

        main = () i32 {
            return check(.Active)
        }
    "#;
    let js = transpile(source).unwrap();
    assert!(js.contains("{ tag: \"Active\" }"));
}

// ============================================================================
// JSDOC ANNOTATIONS
// ============================================================================

#[test]
fn test_jsdoc_params() {
    let source = r#"
        add = (a: i32, b: i32) i32 {
            return a + b
        }
    "#;
    let js = transpile(source).unwrap();
    assert!(js.contains("@param {number} a"));
    assert!(js.contains("@param {number} b"));
    assert!(js.contains("@returns {number}"));
}

// ============================================================================
// BINARY OPERATIONS
// ============================================================================

#[test]
fn test_comparison_operators() {
    let source = r#"
        check = (a: i32, b: i32) bool {
            return a == b
        }
    "#;
    let js = transpile(source).unwrap();
    assert!(js.contains("==="), "== should map to ===");
}

// ============================================================================
// FULL EXAMPLE TRANSPILATION
// ============================================================================

#[test]
fn test_fibonacci_example() {
    let source = std::fs::read_to_string("examples/fibonacci.zen").unwrap();
    let js = transpile(&source).unwrap();
    assert!(js.contains("function fibonacci(n)"));
    assert!(js.contains("fibonacci((n - 1))"));
    assert!(js.contains("main();"));
}

#[test]
fn test_counter_example() {
    let source = std::fs::read_to_string("examples/counter.zen").unwrap();
    let js = transpile(&source).unwrap();
    assert!(js.contains("const Action = Object.freeze"));
    assert!(js.contains("function apply_action(count, action)"));
    assert!(js.contains("let count = 0"));
    assert!(js.contains("console.log"));
}

#[test]
fn test_todo_app_example() {
    let source = std::fs::read_to_string("examples/todo_app.zen").unwrap();
    let js = transpile(&source).unwrap();
    assert!(js.contains("const Priority = Object.freeze"));
    assert!(js.contains("function priority_label(p)"));
    assert!(js.contains("function format_todo(id, text, done, priority)"));
}

#[test]
fn test_fetch_api_example() {
    let source = std::fs::read_to_string("examples/fetch_api.zen").unwrap();
    let js = transpile(&source).unwrap();
    assert!(js.contains("class User"));
    assert!(js.contains("function format_user(id, name, email)"));
}
