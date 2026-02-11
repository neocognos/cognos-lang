mod token;
mod lexer;
mod ast;
mod parser;

use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cognos <file.cog>");
        eprintln!("       cognos parse <file.cog>   # parse and dump AST");
        std::process::exit(1);
    }

    let (command, file_path) = if args.len() == 2 {
        ("run", args[1].as_str())
    } else {
        (args[1].as_str(), args[2].as_str())
    };

    let source = match fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading {}: {}", file_path, e);
            std::process::exit(1);
        }
    };

    match command {
        "parse" => {
            let mut lexer = lexer::Lexer::new(&source);
            let tokens = lexer.tokenize();

            println!("── Tokens ──");
            for t in &tokens {
                println!("  {:>3}:{:<3} {:?}", t.line, t.col, t.token);
            }

            let mut parser = parser::Parser::new(tokens);
            match parser.parse_program() {
                Ok(program) => {
                    println!("\n── AST ──");
                    println!("{:#?}", program);
                }
                Err(e) => {
                    eprintln!("Parse error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "run" => {
            let mut lexer = lexer::Lexer::new(&source);
            let tokens = lexer.tokenize();
            let mut parser = parser::Parser::new(tokens);
            match parser.parse_program() {
                Ok(program) => {
                    println!("✓ Parsed {} flow(s):", program.flows.len());
                    for flow in &program.flows {
                        let params: Vec<String> = flow.params.iter()
                            .map(|p| format!("{}: {:?}", p.name, p.ty))
                            .collect();
                        let ret = flow.return_type.as_ref()
                            .map(|t| format!(" -> {:?}", t))
                            .unwrap_or_default();
                        println!("  flow {}({}){} [{} statements]",
                            flow.name, params.join(", "), ret, flow.body.len());
                    }
                    // TODO: compile to kernel stages and execute
                    println!("\n⚠ Execution not yet implemented. Use 'cognos parse' to inspect.");
                }
                Err(e) => {
                    eprintln!("Parse error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }
}
