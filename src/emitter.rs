// Zen → JavaScript emitter
// Walks the Zen AST and produces JavaScript source code.

use zen::ast::{
    self, AstType, BinaryOperator, Declaration, Expression, Function, MatchArm, Pattern, Program,
    Statement, StringPart, StructDefinition,
};

pub struct JsEmitter {
    indent: usize,
    output: String,
    /// Track variables that have been declared in the current scope
    declared_vars: Vec<std::collections::HashSet<String>>,
}

impl JsEmitter {
    pub fn new() -> Self {
        JsEmitter {
            indent: 0,
            output: String::new(),
            declared_vars: vec![std::collections::HashSet::new()],
        }
    }

    fn is_var_declared(&self, name: &str) -> bool {
        self.declared_vars.iter().any(|scope| scope.contains(name))
    }

    fn declare_var(&mut self, name: &str) {
        if let Some(scope) = self.declared_vars.last_mut() {
            scope.insert(name.to_string());
        }
    }

    fn push_scope(&mut self) {
        self.declared_vars.push(std::collections::HashSet::new());
    }

    fn pop_scope(&mut self) {
        self.declared_vars.pop();
    }

    pub fn emit_program(&mut self, program: &Program) -> String {
        self.output.clear();

        // Emit imports as comments (Zen @std → JS runtime)
        for decl in &program.declarations {
            if let Declaration::ModuleImport { alias, module_path, .. } = decl {
                self.emit_line(&format!("// import {} from \"{}\";", alias, module_path));
            }
        }

        // Emit declarations
        for decl in &program.declarations {
            self.emit_declaration(decl);
            self.emit_newline();
        }

        // Emit top-level statements
        for stmt in &program.statements {
            self.emit_statement(stmt);
        }

        // If there's a main function, call it
        let has_main = program.declarations.iter().any(|d| {
            matches!(d, Declaration::Function(f) if f.name == "main")
        });
        if has_main {
            self.emit_newline();
            self.emit_line("// Entry point");
            self.emit_line("main();");
        }

        self.output.clone()
    }

    // === Declarations ===

    fn emit_declaration(&mut self, decl: &Declaration) {
        match decl {
            Declaration::Function(f) => self.emit_function(f),
            Declaration::Struct(s) => self.emit_struct(s),
            Declaration::Enum(e) => self.emit_enum(e),
            Declaration::Constant { name, value, .. } => {
                self.push_indent();
                self.output.push_str(&format!("const {} = ", name));
                self.emit_expression(value);
                self.output.push_str(";\n");
            }
            Declaration::TypeAlias(ta) => {
                self.emit_line(&format!(
                    "/** @typedef {{{}}} {} */",
                    self.type_to_jsdoc(&ta.target_type),
                    ta.name
                ));
            }
            Declaration::ImplBlock(imp) => {
                self.emit_line(&format!("// impl {} {{", imp.type_name));
                for method in &imp.methods {
                    self.emit_line(&format!(
                        "{}.prototype.{} = function({}) {{",
                        imp.type_name,
                        method.name,
                        method
                            .args
                            .iter()
                            .filter(|(n, _)| n != "self")
                            .map(|(n, _)| n.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                    self.indent += 1;
                    for stmt in &method.body {
                        self.emit_statement(stmt);
                    }
                    self.indent -= 1;
                    self.emit_line("};");
                    self.emit_newline();
                }
                self.emit_line("// }");
            }
            Declaration::Export { symbols } => {
                self.emit_line(&format!("export {{ {} }};", symbols.join(", ")));
            }
            Declaration::ModuleImport { .. } => {
                // Already handled in program-level pass
            }
            Declaration::ComptimeBlock(_) => {
                self.emit_line("// [comptime block elided]");
            }
            _ => {
                self.emit_line(&format!("// unsupported declaration: {:?}", std::mem::discriminant(decl)));
            }
        }
    }

    fn emit_function(&mut self, f: &Function) {
        // JSDoc for parameter types
        if !f.args.is_empty() || f.return_type != AstType::Void {
            self.push_indent();
            self.output.push_str("/**\n");
            for (name, ty) in &f.args {
                self.push_indent();
                self.output
                    .push_str(&format!(" * @param {{{}}} {}\n", self.type_to_jsdoc(ty), name));
            }
            if f.return_type != AstType::Void {
                self.push_indent();
                self.output.push_str(&format!(
                    " * @returns {{{}}}\n",
                    self.type_to_jsdoc(&f.return_type)
                ));
            }
            self.push_indent();
            self.output.push_str(" */\n");
        }

        self.push_indent();
        self.output.push_str(&format!(
            "function {}({}) {{\n",
            self.mangle_name(&f.name),
            f.args
                .iter()
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ));

        self.push_scope();
        // Declare function parameters as already-declared variables
        for (name, _) in &f.args {
            self.declare_var(name);
        }
        self.indent += 1;
        // Emit function body, converting last expression-statement match into a return
        if let Some((last, rest)) = f.body.split_last() {
            for stmt in rest {
                self.emit_statement(stmt);
            }
            // If the last statement is an expression (e.g. pattern match), return it
            if let Statement::Expression { expr, .. } = last {
                self.push_indent();
                self.output.push_str("return ");
                self.emit_expression(expr);
                self.output.push_str(";\n");
            } else {
                self.emit_statement(last);
            }
        }
        self.indent -= 1;
        self.pop_scope();

        self.emit_line("}");
    }

    fn emit_struct(&mut self, s: &StructDefinition) {
        self.emit_line(&format!("class {} {{", s.name));
        self.indent += 1;

        // Constructor
        let field_names: Vec<&str> = s.fields.iter().map(|f| f.name.as_str()).collect();
        self.push_indent();
        self.output
            .push_str(&format!("constructor({}) {{\n", field_names.join(", ")));
        self.indent += 1;
        for field in &s.fields {
            if let Some(default) = &field.default_value {
                self.push_indent();
                self.output
                    .push_str(&format!("this.{} = {} ?? ", field.name, field.name));
                self.emit_expression(default);
                self.output.push_str(";\n");
            } else {
                self.emit_line(&format!("this.{} = {};", field.name, field.name));
            }
        }
        self.indent -= 1;
        self.emit_line("}");

        // Methods
        for method in &s.methods {
            self.emit_newline();
            let params: Vec<String> = method
                .args
                .iter()
                .filter(|(n, _)| n != "self")
                .map(|(n, _)| n.clone())
                .collect();
            self.push_indent();
            self.output
                .push_str(&format!("{}({}) {{\n", method.name, params.join(", ")));
            self.indent += 1;
            for stmt in &method.body {
                self.emit_statement(stmt);
            }
            self.indent -= 1;
            self.emit_line("}");
        }

        self.indent -= 1;
        self.emit_line("}");
    }

    fn emit_enum(&mut self, e: &ast::EnumDefinition) {
        self.emit_line(&format!("// enum {}", e.name));
        self.emit_line(&format!("const {} = Object.freeze({{", e.name));
        self.indent += 1;
        for variant in &e.variants {
            match &variant.payload {
                None => {
                    self.emit_line(&format!(
                        "{}: Object.freeze({{ tag: \"{}\" }}),",
                        variant.name, variant.name
                    ));
                }
                Some(_payload_type) => {
                    self.emit_line(&format!(
                        "{}: (value) => Object.freeze({{ tag: \"{}\", value }}),",
                        variant.name, variant.name
                    ));
                }
            }
        }
        self.indent -= 1;
        self.emit_line("});");

        // Emit methods as standalone functions
        for method in &e.methods {
            self.emit_newline();
            let all_params: Vec<String> = method.args.iter().map(|(n, _)| n.clone()).collect();
            self.push_indent();
            self.output.push_str(&format!(
                "function {}__{}({}) {{\n",
                e.name,
                method.name,
                all_params.join(", ")
            ));
            self.indent += 1;
            for stmt in &method.body {
                self.emit_statement(stmt);
            }
            self.indent -= 1;
            self.emit_line("}");
        }
    }

    // === Statements ===

    fn emit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.push_indent();
                self.emit_expression(expr);
                self.output.push_str(";\n");
            }

            Statement::Return { expr, .. } => {
                self.push_indent();
                self.output.push_str("return ");
                self.emit_expression(expr);
                self.output.push_str(";\n");
            }

            Statement::VariableDeclaration {
                name,
                initializer,
                is_mutable,
                ..
            } => {
                self.push_indent();
                if self.is_var_declared(name) {
                    // Already declared — emit as assignment
                    if let Some(init) = initializer {
                        self.output.push_str(&format!("{} = ", name));
                        self.emit_expression(init);
                        self.output.push_str(";\n");
                    }
                } else {
                    self.declare_var(name);
                    let keyword = if *is_mutable { "let" } else { "const" };
                    if let Some(init) = initializer {
                        self.output.push_str(&format!("{} {} = ", keyword, name));
                        self.emit_expression(init);
                        self.output.push_str(";\n");
                    } else {
                        self.output
                            .push_str(&format!("{} {};\n", keyword, name));
                    }
                }
            }

            Statement::VariableAssignment { name, value, .. } => {
                self.push_indent();
                self.output.push_str(&format!("{} = ", name));
                self.emit_expression(value);
                self.output.push_str(";\n");
            }

            Statement::Loop { kind, body, .. } => {
                match kind {
                    ast::LoopKind::Infinite => {
                        self.emit_line("while (true) {");
                    }
                    ast::LoopKind::Condition(cond) => {
                        self.push_indent();
                        self.output.push_str("while (");
                        self.emit_expression(cond);
                        self.output.push_str(") {\n");
                    }
                }
                self.indent += 1;
                for s in body {
                    self.emit_statement(s);
                }
                self.indent -= 1;
                self.emit_line("}");
            }

            Statement::Break { .. } => {
                self.emit_line("break;");
            }

            Statement::Continue { .. } => {
                self.emit_line("continue;");
            }

            Statement::Block { statements, .. } => {
                self.emit_line("{");
                self.indent += 1;
                for s in statements {
                    self.emit_statement(s);
                }
                self.indent -= 1;
                self.emit_line("}");
            }

            Statement::DestructuringImport { names, source, .. } => {
                self.push_indent();
                self.output.push_str(&format!("// {{ {} }} = ", names.join(", ")));
                self.emit_expression(source);
                self.output.push_str("\n");
            }

            Statement::Defer { statement, .. } => {
                self.emit_line("// defer {");
                self.indent += 1;
                self.emit_statement(statement);
                self.indent -= 1;
                self.emit_line("// }");
            }

            Statement::PointerAssignment { pointer, value, .. } => {
                self.push_indent();
                self.emit_expression(pointer);
                self.output.push_str(" = ");
                self.emit_expression(value);
                self.output.push_str(";\n");
            }

            _ => {
                self.emit_line("// [unsupported statement]");
            }
        }
    }

    // === Expressions ===

    fn emit_expression(&mut self, expr: &Expression) {
        match expr {
            // Literals
            Expression::Integer8(v) => self.output.push_str(&v.to_string()),
            Expression::Integer16(v) => self.output.push_str(&v.to_string()),
            Expression::Integer32(v) => self.output.push_str(&v.to_string()),
            Expression::Integer64(v) => {
                self.output.push_str(&format!("{}n", v)); // BigInt for i64
            }
            Expression::Unsigned8(v) => self.output.push_str(&v.to_string()),
            Expression::Unsigned16(v) => self.output.push_str(&v.to_string()),
            Expression::Unsigned32(v) => self.output.push_str(&v.to_string()),
            Expression::Unsigned64(v) => {
                self.output.push_str(&format!("{}n", v));
            }
            Expression::Float32(v) => {
                self.output.push_str(&format!("{}", v));
            }
            Expression::Float64(v) => {
                self.output.push_str(&format!("{}", v));
            }
            Expression::Boolean(v) => {
                self.output.push_str(if *v { "true" } else { "false" });
            }
            Expression::String(s) => {
                self.output
                    .push_str(&format!("\"{}\"", s.replace('"', "\\\"")));
            }
            Expression::Identifier(name) => {
                self.output.push_str(&self.mangle_name(name));
            }
            Expression::Unit => {
                self.output.push_str("undefined");
            }
            Expression::None => {
                self.output.push_str("null");
            }

            // Binary operations
            Expression::BinaryOp { left, op, right } => {
                self.output.push('(');
                self.emit_expression(left);
                self.output.push_str(&format!(" {} ", self.binary_op_to_js(op)));
                self.emit_expression(right);
                self.output.push(')');
            }

            // Function calls
            Expression::FunctionCall { name, args, .. } => {
                self.emit_function_call(name, args);
            }

            // Method calls
            Expression::MethodCall {
                object,
                method,
                args,
                ..
            } => {
                // Map io.println → console.log, io.print → process.stdout.write
                if let Expression::Identifier(obj_name) = object.as_ref() {
                    let qualified = format!("{}.{}", obj_name, method);
                    match qualified.as_str() {
                        "io.println" => {
                            self.output.push_str("console.log(");
                            for (i, arg) in args.iter().enumerate() {
                                if i > 0 { self.output.push_str(", "); }
                                self.emit_expression(arg);
                            }
                            self.output.push(')');
                            return;
                        }
                        "io.print" => {
                            self.output.push_str("process.stdout.write(String(");
                            if let Some(arg) = args.first() {
                                self.emit_expression(arg);
                            }
                            self.output.push_str("))");
                            return;
                        }
                        "io.read_line" => {
                            self.output.push_str("prompt(\"\")");
                            return;
                        }
                        "JSON.parse" => {
                            self.output.push_str("JSON.parse(");
                            if let Some(arg) = args.first() {
                                self.emit_expression(arg);
                            }
                            self.output.push(')');
                            return;
                        }
                        "JSON.stringify" => {
                            self.output.push_str("JSON.stringify(");
                            if let Some(arg) = args.first() {
                                self.emit_expression(arg);
                            }
                            self.output.push(')');
                            return;
                        }
                        "document.getElementById" => {
                            self.output.push_str("document.getElementById(");
                            if let Some(arg) = args.first() {
                                self.emit_expression(arg);
                            }
                            self.output.push(')');
                            return;
                        }
                        "document.createElement" => {
                            self.output.push_str("document.createElement(");
                            if let Some(arg) = args.first() {
                                self.emit_expression(arg);
                            }
                            self.output.push(')');
                            return;
                        }
                        "document.querySelector" => {
                            self.output.push_str("document.querySelector(");
                            if let Some(arg) = args.first() {
                                self.emit_expression(arg);
                            }
                            self.output.push(')');
                            return;
                        }
                        "document.querySelectorAll" => {
                            self.output.push_str("document.querySelectorAll(");
                            if let Some(arg) = args.first() {
                                self.emit_expression(arg);
                            }
                            self.output.push(')');
                            return;
                        }
                        "Math.floor" | "Math.ceil" | "Math.round" | "Math.random"
                        | "Math.min" | "Math.max" | "Math.abs" | "Math.sqrt" | "Math.pow" => {
                            self.output.push_str(&qualified);
                            self.output.push('(');
                            for (i, arg) in args.iter().enumerate() {
                                if i > 0 { self.output.push_str(", "); }
                                self.emit_expression(arg);
                            }
                            self.output.push(')');
                            return;
                        }
                        _ => {}
                    }
                }

                // Default: object.method(args)
                self.emit_expression(object);
                self.output.push_str(&format!(".{}(", method));
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.emit_expression(arg);
                }
                self.output.push(')');
            }

            // Member access
            Expression::MemberAccess { object, member } => {
                self.emit_expression(object);
                self.output.push('.');
                self.output.push_str(member);
            }

            // Array literal
            Expression::ArrayLiteral(elements) => {
                self.output.push('[');
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.emit_expression(elem);
                }
                self.output.push(']');
            }

            // Array index
            Expression::ArrayIndex { array, index } => {
                self.emit_expression(array);
                self.output.push('[');
                self.emit_expression(index);
                self.output.push(']');
            }

            // Struct literal → object literal
            Expression::StructLiteral { name, fields } => {
                self.output.push_str(&format!("new {}(", name));
                for (i, (_field_name, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.emit_expression(value);
                }
                self.output.push(')');
            }

            // String interpolation → template literal
            Expression::StringInterpolation { parts } => {
                self.output.push('`');
                for part in parts {
                    match part {
                        StringPart::Literal(s) => {
                            self.output.push_str(&s.replace('`', "\\`"));
                        }
                        StringPart::Interpolation(e) => {
                            self.output.push_str("${");
                            self.emit_expression(e);
                            self.output.push('}');
                        }
                    }
                }
                self.output.push('`');
            }

            // Pattern matching → if/else chain
            Expression::QuestionMatch { scrutinee, arms } => {
                self.emit_match(scrutinee, arms);
            }

            // Closures → arrow functions
            Expression::Closure {
                params,
                body,
                ..
            } => {
                let param_str: Vec<String> =
                    params.iter().map(|(name, _)| name.clone()).collect();
                self.output
                    .push_str(&format!("({}) => ", param_str.join(", ")));
                self.emit_expression(body);
            }

            // Block expression → IIFE
            Expression::Block(stmts) => {
                self.output.push_str("(() => {\n");
                self.indent += 1;
                if let Some((last, rest)) = stmts.split_last() {
                    for stmt in rest {
                        self.emit_statement(stmt);
                    }
                    // If the last statement is an expression (not a return),
                    // emit it as a return so the IIFE produces a value
                    match last {
                        Statement::Expression { expr, .. } => {
                            self.push_indent();
                            self.output.push_str("return ");
                            self.emit_expression(expr);
                            self.output.push_str(";\n");
                        }
                        _ => self.emit_statement(last),
                    }
                }
                self.indent -= 1;
                self.push_indent();
                self.output.push_str("})()");
            }

            // Return expression
            Expression::Return(expr) => {
                self.output.push_str("return ");
                self.emit_expression(expr);
            }

            // Enum variant constructors
            Expression::EnumVariant {
                enum_name,
                variant,
                payload,
            } => {
                if let Some(p) = payload {
                    self.output.push_str(&format!("{}.{}(", enum_name, variant));
                    self.emit_expression(p);
                    self.output.push(')');
                } else {
                    self.output.push_str(&format!("{}.{}", enum_name, variant));
                }
            }
            Expression::EnumLiteral { variant, payload } => {
                if let Some(p) = payload {
                    self.output
                        .push_str(&format!("{{ tag: \"{}\", value: ", variant));
                    self.emit_expression(p);
                    self.output.push_str(" }");
                } else {
                    self.output
                        .push_str(&format!("{{ tag: \"{}\" }}", variant));
                }
            }

            // Some/None → value/null
            Expression::Some(inner) => {
                self.emit_expression(inner);
            }

            // Range → Array.from or custom range
            Expression::Range {
                start,
                end,
                inclusive,
            } => {
                self.output.push_str("Array.from({ length: ");
                self.emit_expression(end);
                self.output.push_str(" - ");
                self.emit_expression(start);
                if *inclusive {
                    self.output.push_str(" + 1");
                }
                self.output.push_str(" }, (_, i) => ");
                self.emit_expression(start);
                self.output.push_str(" + i)");
            }

            // Loop expression → while(true) IIFE
            Expression::Loop { body } => {
                self.output.push_str("(() => { while (true) { ");
                self.emit_expression(body);
                self.output.push_str(" } })()");
            }

            // Collection loop → .forEach or for...of
            Expression::CollectionLoop {
                collection,
                param,
                index_param,
                body,
            } => {
                self.emit_expression(collection);
                if let Some((idx_name, _)) = index_param {
                    self.output
                        .push_str(&format!(".forEach(({}, {}) => ", param.0, idx_name));
                } else {
                    self.output
                        .push_str(&format!(".forEach(({}) => ", param.0));
                }
                self.emit_expression(body);
                self.output.push(')');
            }

            // Break/Continue
            Expression::Break { .. } => {
                self.output.push_str("break");
            }
            Expression::Continue { .. } => {
                self.output.push_str("continue");
            }

            // Raise → throw
            Expression::Raise(inner) => {
                self.output.push_str("throw ");
                self.emit_expression(inner);
            }

            // Comptime → elided
            Expression::Comptime(inner) => {
                self.output.push_str("/* comptime */ ");
                self.emit_expression(inner);
            }

            // References → identity in JS
            Expression::StdReference => self.output.push_str("globalThis.__std"),
            Expression::ThisReference => self.output.push_str("this"),

            _ => {
                self.output
                    .push_str(&format!("/* unsupported: {:?} */", std::mem::discriminant(expr)));
            }
        }
    }

    // === Pattern Matching ===

    fn emit_match(&mut self, scrutinee: &Expression, arms: &[MatchArm]) {
        // Emit as IIFE with if/else chain
        self.output.push_str("((__match) => {\n");
        self.indent += 1;

        for (i, arm) in arms.iter().enumerate() {
            self.push_indent();
            if i == 0 {
                self.output.push_str("if (");
            } else {
                self.output.push_str("} else if (");
            }

            self.emit_pattern_condition("__match", &arm.pattern);

            if let Some(guard) = &arm.guard {
                self.output.push_str(" && (");
                self.emit_expression(guard);
                self.output.push(')');
            }

            self.output.push_str(") {\n");
            self.indent += 1;

            // Emit bindings
            self.emit_pattern_bindings("__match", &arm.pattern);

            self.push_indent();
            self.output.push_str("return ");
            self.emit_expression(&arm.body);
            self.output.push_str(";\n");
            self.indent -= 1;
        }

        self.emit_line("}");
        self.indent -= 1;
        self.push_indent();
        self.output.push_str("})(");
        self.emit_expression(scrutinee);
        self.output.push(')');
    }

    fn emit_pattern_condition(&mut self, var: &str, pattern: &Pattern) {
        match pattern {
            Pattern::Wildcard => {
                self.output.push_str("true");
            }
            Pattern::Literal(expr) => {
                self.output.push_str(&format!("{} === ", var));
                self.emit_expression(expr);
            }
            Pattern::Identifier(_) => {
                self.output.push_str("true"); // Always matches, binds
            }
            Pattern::EnumLiteral { variant, .. } => {
                self.output
                    .push_str(&format!("{}.tag === \"{}\"", var, variant));
            }
            Pattern::EnumVariant { variant, .. } => {
                self.output
                    .push_str(&format!("{}.tag === \"{}\"", var, variant));
            }
            Pattern::Type { type_name, .. } => {
                match type_name.as_str() {
                    "true" => self.output.push_str(&format!("{} === true", var)),
                    "false" => self.output.push_str(&format!("{} === false", var)),
                    _ => self
                        .output
                        .push_str(&format!("typeof {} === \"{}\"", var, self.type_name_to_js_typeof(type_name))),
                }
            }
            Pattern::Or(patterns) => {
                self.output.push('(');
                for (i, pat) in patterns.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(" || ");
                    }
                    self.emit_pattern_condition(var, pat);
                }
                self.output.push(')');
            }
            Pattern::Range {
                start,
                end,
                inclusive,
            } => {
                self.output.push_str(&format!("({} >= ", var));
                self.emit_expression(start);
                if *inclusive {
                    self.output.push_str(&format!(" && {} <= ", var));
                } else {
                    self.output.push_str(&format!(" && {} < ", var));
                }
                self.emit_expression(end);
                self.output.push(')');
            }
            _ => {
                self.output.push_str("true /* unsupported pattern */");
            }
        }
    }

    fn emit_pattern_bindings(&mut self, var: &str, pattern: &Pattern) {
        match pattern {
            Pattern::Identifier(name) => {
                self.emit_line(&format!("const {} = {};", name, var));
            }
            Pattern::EnumLiteral {
                payload: Some(inner),
                ..
            } => {
                self.emit_pattern_bindings(&format!("{}.value", var), inner);
            }
            _ => {}
        }
    }

    // === Helpers ===

    fn emit_function_call(&mut self, name: &str, args: &[Expression]) {
        // Map Zen stdlib calls to JS equivalents
        match name {
            "io.println" | "println" => {
                self.output.push_str("console.log(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.emit_expression(arg);
                }
                self.output.push(')');
            }
            "io.print" | "print" => {
                self.output.push_str("process.stdout.write(");
                if let Some(arg) = args.first() {
                    self.emit_expression(arg);
                }
                self.output.push(')');
            }
            "cast" => {
                // Type casts are no-ops in JS, just emit the value
                if let Some(arg) = args.first() {
                    self.emit_expression(arg);
                }
            }
            _ => {
                self.output.push_str(&self.mangle_name(name));
                self.output.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.emit_expression(arg);
                }
                self.output.push(')');
            }
        }
    }

    fn binary_op_to_js(&self, op: &BinaryOperator) -> &'static str {
        match op {
            BinaryOperator::Add => "+",
            BinaryOperator::Subtract => "-",
            BinaryOperator::Multiply => "*",
            BinaryOperator::Divide => "/",
            BinaryOperator::Modulo => "%",
            BinaryOperator::Equals => "===",
            BinaryOperator::NotEquals => "!==",
            BinaryOperator::LessThan => "<",
            BinaryOperator::GreaterThan => ">",
            BinaryOperator::LessThanEquals => "<=",
            BinaryOperator::GreaterThanEquals => ">=",
            BinaryOperator::And => "&&",
            BinaryOperator::Or => "||",
            BinaryOperator::BitwiseAnd => "&",
            BinaryOperator::BitwiseOr => "|",
            BinaryOperator::BitwiseXor => "^",
            BinaryOperator::ShiftLeft => "<<",
            BinaryOperator::ShiftRight => ">>",
            BinaryOperator::StringConcat => "+",
        }
    }

    fn type_to_jsdoc(&self, ty: &AstType) -> String {
        match ty {
            AstType::I8 | AstType::I16 | AstType::I32 | AstType::U8 | AstType::U16
            | AstType::U32 | AstType::F32 | AstType::F64 | AstType::Usize => "number".to_string(),
            AstType::I64 | AstType::U64 => "bigint".to_string(),
            AstType::Bool => "boolean".to_string(),
            AstType::StaticString | AstType::StaticLiteral => "string".to_string(),
            AstType::Void => "void".to_string(),
            AstType::Slice(inner) => format!("Array<{}>", self.type_to_jsdoc(inner)),
            AstType::FixedArray { element_type, size } => {
                format!("Array<{}> /* [{}] */", self.type_to_jsdoc(element_type), size)
            }
            AstType::Struct { name, .. } => name.clone(),
            AstType::Generic { name, type_args } => {
                if type_args.is_empty() {
                    name.clone()
                } else {
                    format!(
                        "{}<{}>",
                        name,
                        type_args
                            .iter()
                            .map(|t| self.type_to_jsdoc(t))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            AstType::Function { args, return_type } => {
                format!(
                    "function({}) : {}",
                    args.iter()
                        .map(|a| self.type_to_jsdoc(a))
                        .collect::<Vec<_>>()
                        .join(", "),
                    self.type_to_jsdoc(return_type)
                )
            }
            AstType::Ref(inner) => self.type_to_jsdoc(inner),
            _ => "*".to_string(),
        }
    }

    fn type_name_to_js_typeof(&self, name: &str) -> &str {
        match name {
            "i8" | "i16" | "i32" | "u8" | "u16" | "u32" | "f32" | "f64" => "number",
            "i64" | "u64" => "bigint",
            "bool" => "boolean",
            "String" | "string" => "string",
            _ => "object",
        }
    }

    fn mangle_name(&self, name: &str) -> String {
        // Replace dots with underscores for qualified names
        name.replace('.', "_")
    }

    fn push_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
    }

    fn emit_line(&mut self, s: &str) {
        self.push_indent();
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn emit_newline(&mut self) {
        self.output.push('\n');
    }
}
