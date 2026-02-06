// zen-js: Zen → JavaScript transpiler
// Demonstrates Zen's meta-programming vision: AST walking as code generation.

use std::env;
use std::io::{self, Read};
use std::path::Path;

use zen_js::transpile;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 => {
            // Read from stdin
            let mut source = String::new();
            io::stdin().read_to_string(&mut source)?;
            do_transpile(&source, None)?;
        }
        2 => {
            let arg = &args[1];
            if arg == "--help" || arg == "-h" {
                print_usage();
                return Ok(());
            }
            let source = std::fs::read_to_string(arg).map_err(|e| {
                io::Error::new(io::ErrorKind::NotFound, format!("Cannot read '{}': {}", arg, e))
            })?;
            do_transpile(&source, Some(arg))?;
        }
        3 => {
            if args[1] == "-o" || args[2] == "-o" {
                let (input, output) = if args[1] == "-o" {
                    (&args[2], Some(args[1].clone()))
                } else {
                    (&args[1], Some(args[2].clone()))
                };
                let source = std::fs::read_to_string(input).map_err(|e| {
                    io::Error::new(io::ErrorKind::NotFound, format!("Cannot read '{}': {}", input, e))
                })?;
                let js = transpile_to_string(&source, Some(input))?;
                let out_path = output.unwrap_or_else(|| {
                    Path::new(input)
                        .with_extension("js")
                        .to_string_lossy()
                        .to_string()
                });
                std::fs::write(&out_path, &js)?;
                eprintln!("Wrote {}", out_path);
            } else {
                print_usage();
            }
        }
        _ => {
            print_usage();
        }
    }

    Ok(())
}

fn do_transpile(source: &str, filename: Option<&str>) -> io::Result<()> {
    let js = transpile_to_string(source, filename)?;
    print!("{}", js);
    Ok(())
}

fn transpile_to_string(source: &str, filename: Option<&str>) -> io::Result<String> {
    transpile(source).map_err(|e| {
        io::Error::other(format!(
            "{}{}",
            filename
                .map(|f| format!("{}: ", f))
                .unwrap_or_default(),
            e
        ))
    })
}

fn print_usage() {
    eprintln!("zen-js: Zen to JavaScript transpiler");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  zen-js <file.zen>          Transpile and print to stdout");
    eprintln!("  zen-js <file.zen> -o       Write to <file>.js");
    eprintln!("  zen-js < input.zen         Read from stdin");
    eprintln!("  zen-js --help              Show this message");
}
