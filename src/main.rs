mod token;
mod lexer;
mod ast;
mod parser;
mod pretty;

use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cognos <file.cog>         # parse and pretty-print");
        eprintln!("       cognos parse <file.cog>    # parse and pretty-print");
        eprintln!("       cognos tokens <file.cog>   # show raw tokens");
        eprintln!("       cognos ast <file.cog>      # show raw AST debug");
        std::process::exit(1);
    }

    let (command, file_path) = if args.len() == 2 {
        ("parse", args[1].as_str())
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

    let mut lexer = lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();

    match command {
        "tokens" => {
            for t in &tokens {
                println!("  {:>3}:{:<3} {:?}", t.line, t.col, t.token);
            }
        }
        "ast" => {
            let mut p = parser::Parser::new(tokens);
            match p.parse_program() {
                Ok(program) => println!("{:#?}", program),
                Err(e) => { eprintln!("Parse error: {}", e); std::process::exit(1); }
            }
        }
        "parse" => {
            let mut p = parser::Parser::new(tokens);
            match p.parse_program() {
                Ok(program) => {
                    println!("✓ Parsed {} flow(s)\n", program.flows.len());
                    print!("{}", pretty::pretty_program(&program));
                }
                Err(e) => { eprintln!("Parse error: {}", e); std::process::exit(1); }
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }
}
