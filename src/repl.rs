/// Interactive REPL for Cognos.
/// Wraps input in a flow and evaluates it.

use std::io::{self, BufRead, Write};
use crate::interpreter::{Interpreter, Value};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::ast::Program;
use anyhow::Result;

pub fn run_repl() -> Result<()> {
    println!("Cognos REPL v0.1.0");
    println!("Type expressions or statements. Use 'exit' or Ctrl-D to quit.\n");

    let mut interp = Interpreter::new();
    // Initialize with empty program so flows HashMap exists
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
            // EOF
            eprintln!();
            break;
        }

        let trimmed = line.trim();

        // Exit commands
        if !in_block && (trimmed == "exit" || trimmed == "quit") {
            break;
        }

        // Empty line in block mode ends the block
        if in_block && trimmed.is_empty() {
            in_block = false;
            eval_repl_input(&mut interp, &lines);
            lines.clear();
            continue;
        }

        // Accumulate lines
        lines.push_str(&line);

        // Check if we're starting/continuing a block
        if trimmed.ends_with(':') || in_block {
            in_block = true;
            continue;
        }

        // Single line — evaluate
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

    // Try as a flow definition first
    if trimmed.starts_with("flow ") {
        match parse_and_register_flow(interp, input) {
            Ok(name) => eprintln!("✓ Defined flow '{}'", name),
            Err(e) => eprintln!("Error: {}", e),
        }
        return;
    }

    // Wrap in a temporary flow and execute
    // If it looks like an expression, emit the result
    let wrapped = if trimmed.starts_with("if ") || trimmed.starts_with("loop ")
        || trimmed.starts_with("for ") || trimmed.contains(" = ") {
        format!("flow __repl__():\n    {}\n", trimmed)
    } else {
        // Expression — wrap in emit to show result
        format!("flow __repl__():\n    __result__ = {}\n    emit(__result__)\n", trimmed)
    };

    let mut lexer = Lexer::new(&wrapped);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);

    match parser.parse_program() {
        Ok(program) => {
            // Register the temp flow
            if let Some(flow) = program.flows.first() {
                interp.register_flow(flow.clone());
            }
            // Run it
            if let Err(e) = interp.call_flow_entry("__repl__") {
                eprintln!("Error: {}", e);
            }
        }
        Err(e) => eprintln!("Parse error: {}", e),
    }
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
