mod token;
mod lexer;
mod ast;
mod parser;
mod pretty;
mod interpreter;
mod repl;
mod environment;
mod error;
mod trace;

use std::env;
use std::fs;
use std::collections::HashMap;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cognos <file.cog>              # run the program");
        eprintln!("       cognos run [-v|-vv|-vvv] <file> # run with verbosity");
        eprintln!("       cognos parse <file.cog>         # parse and pretty-print");
        eprintln!("       cognos tokens <file.cog>        # show raw tokens");
        eprintln!("       cognos repl                     # interactive REPL");
        eprintln!("       cognos trace-to-mock <file.jsonl> # convert trace to mock JSON");
        eprintln!("\nEnv: COGNOS_LOG=info|debug|trace");
        std::process::exit(1);
    }

    // Handle trace-to-mock before normal arg parsing
    if args.len() >= 3 && args[1] == "trace-to-mock" {
        trace_to_mock(&args[2]);
        return;
    }

    // Parse args: find command, verbosity flags, and file path
    let mut command = "run";
    let mut verbosity = 0u8;
    let mut file_path = None;
    let mut allow_shell = false;
    let mut trace_path: Option<String> = None;
    let mut trace_level = trace::TraceLevel::Metrics;
    let mut env_path: Option<String> = None;
    let mut session_path: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "run" | "parse" | "tokens" | "repl" | "test" | "trace-to-mock" => command = match args[i].as_str() {
                "run" => "run",
                "parse" => "parse",
                "tokens" => "tokens",
                "repl" => "repl",
                "test" => "test",
                "trace-to-mock" => "trace-to-mock",
                _ => unreachable!(),
            },
            "-v" => verbosity = verbosity.max(1),
            "-vv" => verbosity = verbosity.max(2),
            "-vvv" => verbosity = verbosity.max(3),
            "--allow-shell" => allow_shell = true,
            "--trace" => {
                i += 1;
                if i < args.len() {
                    trace_path = Some(args[i].clone());
                } else {
                    eprintln!("--trace requires a file path");
                    std::process::exit(1);
                }
            }
            "--session" => {
                i += 1;
                if i < args.len() {
                    session_path = Some(args[i].clone());
                } else {
                    eprintln!("--session requires a file path");
                    std::process::exit(1);
                }
            }
            "--env" => {
                i += 1;
                if i < args.len() {
                    env_path = Some(args[i].clone());
                } else {
                    eprintln!("--env requires a file path");
                    std::process::exit(1);
                }
            }
            "--trace-level" => {
                i += 1;
                if i < args.len() {
                    trace_level = match args[i].as_str() {
                        "metrics" => trace::TraceLevel::Metrics,
                        "full" => trace::TraceLevel::Full,
                        other => {
                            eprintln!("Unknown trace level: {} (use 'metrics' or 'full')", other);
                            std::process::exit(1);
                        }
                    };
                }
            }
            s if s.starts_with('-') => {
                eprintln!("Unknown flag: {}", s);
                std::process::exit(1);
            }
            _ => file_path = Some(args[i].as_str()),
        }
        i += 1;
    }

    // Initialize logging: CLI flag overrides env var
    if verbosity > 0 || env::var("COGNOS_LOG").is_ok() {
        let level = if verbosity > 0 {
            match verbosity {
                1 => "info",
                2 => "debug",
                _ => "trace",
            }
        } else {
            // env var is set, let env_logger handle it
            ""
        };

        if !level.is_empty() {
            env::set_var("RUST_LOG", format!("cognos={}", level));
        } else if let Ok(val) = env::var("COGNOS_LOG") {
            env::set_var("RUST_LOG", format!("cognos={}", val));
        }
    }
    env_logger::Builder::from_default_env()
        .format_timestamp(None)
        .format_target(false)
        .init();

    // REPL mode — no file needed
    if command == "repl" {
        if let Err(e) = repl::run_repl() {
            eprintln!("REPL error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    let file_path = match file_path {
        Some(p) => p,
        None => {
            eprintln!("No input file specified");
            std::process::exit(1);
        }
    };

    let source = match fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading {}: {}", file_path, e);
            std::process::exit(1);
        }
    };

    log::info!("Loading {}", file_path);

    let mut lexer = lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();
    log::debug!("Lexed {} tokens", tokens.len());

    match command {
        "tokens" => {
            for t in &tokens {
                println!("  {:>3}:{:<3} {:?}", t.line, t.col, t.token);
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
        "run" => {
            let mut p = parser::Parser::new(tokens);
            let program = match p.parse_program() {
                Ok(prog) => prog,
                Err(e) => { eprintln!("Parse error: {}", e); std::process::exit(1); }
            };
            log::info!("Parsed {} flow(s)", program.flows.len());
            let tracer = trace_path.as_ref().map(|p| {
                std::sync::Arc::new(trace::Tracer::new_file(p, trace_level).unwrap_or_else(|e| {
                    eprintln!("Failed to open trace file {}: {}", p, e);
                    std::process::exit(1);
                }))
            });
            let mut interp = interpreter::Interpreter::with_full_options(allow_shell, tracer);
            // Load session state if --session provided
            if let Some(ref sp) = session_path {
                if std::path::Path::new(sp).exists() {
                    if let Err(e) = interp.load_session(sp) {
                        eprintln!("Warning: failed to load session: {}", e);
                    }
                }
            }
            if let Err(e) = interp.run_with_base(&program, Some(std::path::Path::new(file_path))) {
                eprintln!("Runtime error: {}", e);
                // Still save session on error
                if let Some(ref sp) = session_path {
                    let _ = interp.save_session(sp);
                }
                std::process::exit(1);
            }
            // Save session state
            if let Some(ref sp) = session_path {
                if let Err(e) = interp.save_session(sp) {
                    eprintln!("Warning: failed to save session: {}", e);
                }
            }
        }
        "test" => {
            let env_file = env_path.unwrap_or_else(|| {
                eprintln!("cognos test requires --env <mock.json>");
                std::process::exit(1);
            });
            let env_json: serde_json::Value = serde_json::from_str(
                &fs::read_to_string(&env_file).unwrap_or_else(|e| {
                    eprintln!("Cannot read env file {}: {}", env_file, e);
                    std::process::exit(1);
                })
            ).unwrap_or_else(|e| {
                eprintln!("Invalid JSON in {}: {}", env_file, e);
                std::process::exit(1);
            });
            let mock_env = environment::MockEnv::from_json(&env_json).unwrap_or_else(|e| {
                eprintln!("Invalid mock env: {}", e);
                std::process::exit(1);
            });
            let mut p = parser::Parser::new(tokens);
            let program = match p.parse_program() {
                Ok(prog) => prog,
                Err(e) => { eprintln!("Parse error: {}", e); std::process::exit(1); }
            };
            let tracer = trace_path.as_ref().map(|p| {
                std::sync::Arc::new(trace::Tracer::new_file(p, trace_level).unwrap_or_else(|e| {
                    eprintln!("Failed to open trace file {}: {}", p, e);
                    std::process::exit(1);
                }))
            });
            let mut interp = interpreter::Interpreter::with_env(Box::new(mock_env), tracer);
            if let Err(e) = interp.run_with_base(&program, Some(std::path::Path::new(file_path))) {
                eprintln!("Runtime error: {}", e);
                std::process::exit(1);
            }
            // Print captured stdout
            if let Some(output) = interp.captured_stdout() {
                println!("─── Mock Output ({} lines) ───", output.len());
                for line in &output {
                    println!("  {}", line);
                }
                println!("─── Pass ✓ ───");
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }
}

fn trace_to_mock(path: &str) {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Cannot read {}: {}", path, e);
        std::process::exit(1);
    });

    let mut stdin_lines: Vec<String> = Vec::new();
    let mut llm_responses: Vec<serde_json::Value> = Vec::new();
    let mut shell_commands: HashMap<String, String> = HashMap::new();
    let mut files: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let event: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        match event.get("event").and_then(|v| v.as_str()) {
            Some("io") => {
                let op = event.get("op").and_then(|v| v.as_str()).unwrap_or("");
                let handle = event.get("handle").and_then(|v| v.as_str()).unwrap_or("");
                match (op, handle) {
                    ("read", "stdin") => {
                        if let Some(c) = event.get("content").and_then(|v| v.as_str()) {
                            stdin_lines.push(c.to_string());
                        }
                    }
                    ("read", "file") => {
                        if let (Some(p), Some(c)) = (
                            event.get("path").and_then(|v| v.as_str()),
                            event.get("content").and_then(|v| v.as_str()),
                        ) {
                            files.insert(p.to_string(), c.to_string());
                        }
                    }
                    _ => {}
                }
            }
            Some("llm_call") => {
                if let Some(resp) = event.get("response").and_then(|v| v.as_str()) {
                    let has_tc = event.get("has_tool_calls").and_then(|v| v.as_bool()).unwrap_or(false);
                    if has_tc {
                        // Try to extract tool calls from response text
                        llm_responses.push(serde_json::json!({
                            "content": resp,
                            "tool_calls": []
                        }));
                    } else {
                        llm_responses.push(serde_json::Value::String(resp.to_string()));
                    }
                } else {
                    llm_responses.push(serde_json::Value::String("".to_string()));
                }
            }
            Some("shell_exec") => {
                if let Some(cmd) = event.get("command").and_then(|v| v.as_str()) {
                    let output = event.get("output").and_then(|v| v.as_str()).unwrap_or("");
                    shell_commands.insert(cmd.to_string(), output.to_string());
                }
            }
            _ => {}
        }
    }

    let mock = serde_json::json!({
        "stdin": stdin_lines,
        "llm_responses": llm_responses,
        "shell": shell_commands,
        "files": files,
        "allow_shell": true
    });

    println!("{}", serde_json::to_string_pretty(&mock).unwrap_or_default());
}
