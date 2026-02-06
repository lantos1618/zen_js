pub mod emitter;

use zen::lexer::Lexer;
use zen::parser::Parser;

/// Transpile Zen source code to JavaScript
pub fn transpile(source: &str) -> Result<String, String> {
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().map_err(|e| format!("Parse error: {}", e))?;

    let mut emitter = emitter::JsEmitter::new();
    Ok(emitter.emit_program(&program))
}
