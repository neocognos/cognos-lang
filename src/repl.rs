/// Interactive REPL for Cognos.

use std::io::{self, BufRead, Write};
use crate::interpreter::{Interpreter, Value};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::ast::Program;
use anyhow::Result;

pub fn run_repl() -> Result<()> {
    eprintln!("Cognos REPL v0.1.0");
    eprintln!("Type expressions or statements. Use 'exit' or Ctrl-D to quit.\n");

    let mut interp = Interpreter::new();
    let empty = Program { flows: vec![] };
    let _ = interp.run(&empty);

    let stdin = io::stdin();
    let mut lines = String::new();
    let mut in_block = false;

    loop {
        if in_block {
            eprint!("... ");
        } else {
            eprint!(">>> ");
        }
        io::stderr().flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            eprintln!();
            break;
        }

        let trimmed = line.trim();

        if !in_block && (trimmed == "exit" || trimmed == "quit") {
            break;
        }

        // Skip empty lines outside blocks
        if !in_block && trimmed.is_empty() {
            continue;
        }

        // Empty line in block mode ends the block
        if in_block && trimmed.is_empty() {
            in_block = false;
            eval_repl_input(&mut interp, &lines);
            lines.clear();
            continue;
        }

        lines.push_str(&line);

        if trimmed.ends_with(':') || in_block {
            in_block = true;
            continue;
        }

        in_block = false;
        eval_repl_input(&mut interp, &lines);
        lines.clear();
    }

    Ok(())
}

fn eval_repl_input(interp: &mut Interpreter, input: &str) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return;
    }

    // Flow definition
    if trimmed.starts_with("flow ") {
        match parse_and_register_flow(interp, input) {
            Ok(name) => eprintln!("✓ Defined flow '{}'", name),
            Err(e) => eprintln!("Error: {}", e),
        }
        return;
    }

    // Friendly errors for bare keywords
    if trimmed == "flow" {
        eprintln!("Error: incomplete flow definition — usage: flow name(params): ...");
        return;
    }
    let bare_fns = ["emit", "think", "act", "run", "log", "remember", "recall"];
    let bare = trimmed.trim_end_matches("()");
    if bare_fns.contains(&bare) && (trimmed == bare || trimmed == format!("{}()", bare)) {
        eprintln!("Error: '{}' needs arguments — did you mean {}(...)?", bare, bare);
        return;
    }

    // Detect if this is a statement (starts with keyword or is an assignment)
    let is_statement = trimmed.starts_with("emit(")
        || trimmed.starts_with("if ")
        || trimmed.starts_with("loop ")
        || trimmed.starts_with("for ")
        || trimmed.starts_with("log(")
        || trimmed.starts_with("pass")
        || trimmed.starts_with("break")
        || trimmed.starts_with("continue")
        || trimmed.starts_with("return ")
        || is_assignment(trimmed);

    let wrapped = if is_statement {
        format!("flow __repl__():\n    {}\n", indent_block(trimmed))
    } else {
        // Expression — wrap in emit to show result
        format!("flow __repl__():\n    emit({})\n", trimmed)
    };

    let mut lexer = Lexer::new(&wrapped);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);

    match parser.parse_program() {
        Ok(program) => {
            if let Some(flow) = program.flows.first() {
                interp.register_flow(flow.clone());
            }
            if let Err(e) = interp.call_flow_entry("__repl__") {
                eprintln!("Error: {}", e);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

/// Check if input looks like an assignment: `name = expr`
fn is_assignment(s: &str) -> bool {
    // Find first `=` that isn't `==`
    let bytes = s.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'=' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                continue; // skip ==
            }
            if i > 0 && (bytes[i - 1] == b'!' || bytes[i - 1] == b'<' || bytes[i - 1] == b'>') {
                continue; // skip !=, <=, >=
            }
            // Check left side is a simple identifier
            let left = s[..i].trim();
            return left.chars().all(|c| c.is_alphanumeric() || c == '_');
        }
    }
    false
}

/// Indent a multi-line block for wrapping in a flow
fn indent_block(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();
    if lines.len() <= 1 {
        return s.to_string();
    }
    lines.iter().enumerate().map(|(i, line)| {
        if i == 0 {
            line.to_string()
        } else {
            format!("    {}", line)
        }
    }).collect::<Vec<_>>().join("\n")
}

fn parse_and_register_flow(interp: &mut Interpreter, input: &str) -> Result<String> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;

    if let Some(flow) = program.flows.first() {
        let name = flow.name.clone();
        interp.register_flow(flow.clone());
        Ok(name)
    } else {
        anyhow::bail!("no flow found")
    }
}
