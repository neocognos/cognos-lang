/// Tree-walking interpreter for Cognos.
/// Executes a parsed AST directly — no kernel needed.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use crate::ast::*;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum Value {
    String(std::string::String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<Value>),
    Map(Vec<(std::string::String, Value)>),  // ordered key-value pairs
    None,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Map(entries) => {
                write!(f, "{{")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "\"{}\": {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::None => write!(f, ""),
        }
    }
}

impl Value {
    fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::String(s) => !s.is_empty(),
            Value::Int(n) => *n != 0,
            Value::Float(n) => *n != 0.0,
            Value::List(items) => !items.is_empty(),
            Value::Map(entries) => !entries.is_empty(),
            Value::None => false,
        }
    }

    /// Get a field from a Map value
    fn get_field(&self, field: &str) -> Option<&Value> {
        if let Value::Map(entries) = self {
            entries.iter().find(|(k, _)| k == field).map(|(_, v)| v)
        } else {
            None
        }
    }
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::String(_) => "String",
        Value::Int(_) => "Int",
        Value::Float(_) => "Float",
        Value::Bool(_) => "Bool",
        Value::List(_) => "List",
        Value::Map(_) => "Map",
        Value::None => "None",
    }
}

fn op_str(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*", BinOp::Div => "/",
        BinOp::Eq => "==", BinOp::NotEq => "!=",
        BinOp::Lt => "<", BinOp::Gt => ">", BinOp::LtEq => "<=", BinOp::GtEq => ">=",
        BinOp::And => "and", BinOp::Or => "or",
    }
}

enum ControlFlow {
    Normal,
    Break,
    Continue,
    Return(Value),
}

pub struct Interpreter {
    vars: HashMap<std::string::String, Value>,
    flows: HashMap<std::string::String, crate::ast::FlowDef>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self { vars: HashMap::new(), flows: HashMap::new() }
    }

    pub fn run(&mut self, program: &Program) -> Result<()> {
        // Register all flows
        for flow in &program.flows {
            self.flows.insert(flow.name.clone(), flow.clone());
        }

        // Find "main" flow, or use the first one
        let flow = program.flows.iter()
            .find(|f| f.name == "main")
            .or_else(|| program.flows.first())
            .cloned();

        match flow {
            Some(f) => {
                // Bind flow parameters — in CLI mode, read from stdin
                log::info!("Running flow '{}'", f.name);
                for param in &f.params {
                    log::debug!("Reading param '{}' from stdin", param.name);
                    print!("> ");
                    io::stdout().flush()?;
                    let mut line = std::string::String::new();
                    io::stdin().lock().read_line(&mut line)?;
                    let val = line.trim_end().to_string();
                    log::debug!("  {} = {:?}", param.name, val);
                    self.vars.insert(param.name.clone(), Value::String(val));
                }
                self.run_block(&f.body)?;
                Ok(())
            }
            None => Ok(()),
        }
    }

    /// Register a flow (for REPL use)
    pub fn register_flow(&mut self, flow: crate::ast::FlowDef) {
        self.flows.insert(flow.name.clone(), flow);
    }

    /// Call a flow with no args, keeping current vars (for REPL use)
    pub fn call_flow_entry(&mut self, name: &str) -> Result<()> {
        let flow = self.flows.get(name).cloned()
            .ok_or_else(|| anyhow::anyhow!("unknown flow: {}", name))?;
        self.run_block(&flow.body)?;
        Ok(())
    }

    /// Call a user-defined flow with arguments
    fn call_flow(&mut self, name: &str, args: Vec<Value>) -> Result<Value> {
        let flow = self.flows.get(name).cloned()
            .ok_or_else(|| anyhow::anyhow!("unknown flow: {}", name))?;

        if args.len() != flow.params.len() {
            bail!("{}() expects {} args, got {}", name, flow.params.len(), args.len());
        }

        // Save current vars, set up new scope
        let saved_vars = self.vars.clone();
        self.vars.clear();

        // Bind parameters
        for (param, val) in flow.params.iter().zip(args) {
            self.vars.insert(param.name.clone(), val);
        }

        log::info!("Calling flow '{}'", name);
        let result = self.run_block(&flow.body)?;

        // Restore vars
        self.vars = saved_vars;

        match result {
            ControlFlow::Return(v) => Ok(v),
            _ => Ok(Value::None),
        }
    }

    fn run_block(&mut self, stmts: &[Stmt]) -> Result<ControlFlow> {
        for stmt in stmts {
            match self.run_stmt(stmt)? {
                ControlFlow::Normal => {}
                other => return Ok(other),
            }
        }
        Ok(ControlFlow::Normal)
    }

    fn run_stmt(&mut self, stmt: &Stmt) -> Result<ControlFlow> {
        match stmt {
            Stmt::Pass => Ok(ControlFlow::Normal),

            Stmt::Assign { name, expr } => {
                let val = self.eval(expr)?;
                self.vars.insert(name.clone(), val);
                Ok(ControlFlow::Normal)
            }

            Stmt::Emit { value } => {
                let val = self.eval(value)?;
                println!("{}", val);
                Ok(ControlFlow::Normal)
            }

            Stmt::Return { value } => {
                let val = self.eval(value)?;
                Ok(ControlFlow::Return(val))
            }

            Stmt::Break => Ok(ControlFlow::Break),
            Stmt::Continue => Ok(ControlFlow::Continue),

            Stmt::Expr(expr) => {
                self.eval(expr)?;
                Ok(ControlFlow::Normal)
            }

            Stmt::If { condition, body, elifs, else_body } => {
                let cond = self.eval(condition)?;
                if cond.is_truthy() {
                    return self.run_block(body);
                }
                for (elif_cond, elif_body) in elifs {
                    let c = self.eval(elif_cond)?;
                    if c.is_truthy() {
                        return self.run_block(elif_body);
                    }
                }
                if !else_body.is_empty() {
                    return self.run_block(else_body);
                }
                Ok(ControlFlow::Normal)
            }

            Stmt::For { var, iterable, body } => {
                let collection = self.eval(iterable)?;
                let items = match collection {
                    Value::List(items) => items,
                    Value::Map(entries) => entries.into_iter()
                        .map(|(k, _)| Value::String(k))
                        .collect(),
                    Value::String(s) => s.chars()
                        .map(|c| Value::String(c.to_string()))
                        .collect(),
                    other => bail!("cannot iterate over {} (type: {})", other, type_name(&other)),
                };
                for item in items {
                    self.vars.insert(var.clone(), item);
                    match self.run_block(body)? {
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        ControlFlow::Normal => {}
                    }
                }
                Ok(ControlFlow::Normal)
            }

            Stmt::Loop { max, body } => {
                let limit = max.unwrap_or(1000);
                for _ in 0..limit {
                    match self.run_block(body)? {
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        ControlFlow::Normal => {}
                    }
                }
                Ok(ControlFlow::Normal)
            }
        }
    }

    fn eval(&mut self, expr: &Expr) -> Result<Value> {
        match expr {
            Expr::StringLit(s) => Ok(Value::String(s.clone())),
            Expr::IntLit(n) => Ok(Value::Int(*n)),
            Expr::FloatLit(n) => Ok(Value::Float(*n)),
            Expr::BoolLit(b) => Ok(Value::Bool(*b)),

            Expr::Ident(name) => {
                match self.vars.get(name) {
                    Some(v) => Ok(v.clone()),
                    None => {
                        let builtins = ["think", "act", "emit", "run", "log", "print", "remember", "recall"];
                        if builtins.contains(&name.as_str()) {
                            bail!("'{}' is a function — did you mean {}(...)?", name, name)
                        } else if self.flows.contains_key(name) {
                            bail!("'{}' is a flow — did you mean {}(...)?", name, name)
                        } else {
                            bail!("undefined variable: '{}'", name)
                        }
                    }
                }
            }

            Expr::List(items) => {
                let vals: Result<Vec<Value>> = items.iter().map(|i| self.eval(i)).collect();
                Ok(Value::List(vals?))
            }

            Expr::Map(entries) => {
                let mut result = Vec::new();
                for (k, v) in entries {
                    let val = self.eval(v)?;
                    result.push((k.clone(), val));
                }
                Ok(Value::Map(result))
            }

            Expr::FString(parts) => {
                let mut result = std::string::String::new();
                for part in parts {
                    match part {
                        crate::ast::FStringPart::Literal(s) => result.push_str(s),
                        crate::ast::FStringPart::Expr(e) => {
                            let val = self.eval(e)?;
                            result.push_str(&val.to_string());
                        }
                    }
                }
                Ok(Value::String(result))
            }

            Expr::Call { name, args, kwargs } => {
                self.call_builtin(name, args, kwargs)
            }

            Expr::Field { object, field } => {
                let val = self.eval(object)?;
                match (&val, field.as_str()) {
                    (Value::String(s), "length") => Ok(Value::Int(s.len() as i64)),
                    (Value::List(l), "length") => Ok(Value::Int(l.len() as i64)),
                    (Value::Map(_), _) => {
                        match val.get_field(field) {
                            Some(v) => Ok(v.clone()),
                            None => bail!("map has no key '{}'", field),
                        }
                    }
                    _ => bail!("cannot access field '{}' on {} (type: {})", field, val, type_name(&val)),
                }
            }

            Expr::BinOp { left, op, right } => {
                let l = self.eval(left)?;
                let r = self.eval(right)?;
                self.eval_binop(&l, op, &r)
            }

            Expr::UnaryOp { op, operand } => {
                let v = self.eval(operand)?;
                match op {
                    UnaryOp::Not => Ok(Value::Bool(!v.is_truthy())),
                }
            }
        }
    }

    fn call_builtin(&mut self, name: &str, args: &[Expr], kwargs: &[(std::string::String, Expr)]) -> Result<Value> {
        match name {
            "print" | "emit" => {
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { print!(" "); }
                    let val = self.eval(arg)?;
                    print!("{}", val);
                }
                println!();
                Ok(Value::None)
            }
            "think" => {
                if args.is_empty() {
                    bail!("think() requires at least one argument");
                }
                let context = self.eval(&args[0])?;

                let mut model = "qwen2.5:1.5b".to_string();
                let mut system = std::string::String::new();
                for (k, v) in kwargs {
                    let val = self.eval(v)?;
                    match k.as_str() {
                        "model" => model = val.to_string(),
                        "system" => system = val.to_string(),
                        "tools" | "output" => {} // TODO
                        _ => bail!("think(): unknown kwarg '{}'", k),
                    }
                }

                self.call_ollama(&model, &system, &context.to_string())
            }
            "act" => {
                if args.is_empty() {
                    bail!("act() requires at least one argument");
                }
                let val = self.eval(&args[0])?;
                eprintln!("⚠ act() stubbed in interpreter mode");
                Ok(val)
            }
            "run" => {
                if args.is_empty() {
                    bail!("run() requires a command string");
                }
                let cmd = self.eval(&args[0])?;
                log::info!("run() → {:?}", cmd.to_string());
                let output = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(cmd.to_string())
                    .output()?;
                let stdout = std::string::String::from_utf8_lossy(&output.stdout).to_string();
                let code = output.status.code().unwrap_or(-1);
                log::debug!("run() exit={} stdout={} chars", code, stdout.len());
                Ok(Value::String(stdout.trim_end().to_string()))
            }
            "log" => {
                for arg in args {
                    let val = self.eval(arg)?;
                    eprintln!("[log] {}", val);
                }
                Ok(Value::None)
            }
            _ => {
                // Try user-defined flow
                if self.flows.contains_key(name) {
                    let mut arg_vals = Vec::new();
                    for arg in args {
                        arg_vals.push(self.eval(arg)?);
                    }
                    return self.call_flow(name, arg_vals);
                }
                bail!("unknown function: {}()", name)
            }
        }
    }

    fn call_ollama(&self, model: &str, system: &str, prompt: &str) -> Result<Value> {
        #[derive(Serialize)]
        struct OllamaRequest {
            model: std::string::String,
            prompt: std::string::String,
            #[serde(skip_serializing_if = "str::is_empty")]
            system: std::string::String,
            stream: bool,
        }

        #[derive(Deserialize)]
        struct OllamaResponse {
            response: std::string::String,
        }

        log::info!("think() → model={}", model);
        log::debug!("think() system={:?}", if system.is_empty() { "(none)" } else { system });
        log::debug!("think() prompt={:?}", if prompt.len() > 200 { &prompt[..200] } else { prompt });

        let start = std::time::Instant::now();
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;
        let req_body = OllamaRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            system: system.to_string(),
            stream: false,
        };
        log::trace!("Ollama request: {}", serde_json::to_string_pretty(&req_body)?);

        let resp = client
            .post("http://localhost:11434/api/generate")
            .json(&req_body)
            .send()?
            .error_for_status()?
            .json::<OllamaResponse>()?;

        let elapsed = start.elapsed();
        log::info!("think() completed in {:.1}s ({} chars)", elapsed.as_secs_f64(), resp.response.len());
        log::trace!("Ollama response: {:?}", resp.response);

        Ok(Value::String(resp.response.trim().to_string()))
    }

    fn eval_binop(&self, left: &Value, op: &BinOp, right: &Value) -> Result<Value> {
        match (left, op, right) {
            // String concat
            (Value::String(a), BinOp::Add, Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),

            // Int arithmetic
            (Value::Int(a), BinOp::Add, Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Int(a), BinOp::Sub, Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Int(a), BinOp::Mul, Value::Int(b)) => Ok(Value::Int(a * b)),
            (Value::Int(a), BinOp::Div, Value::Int(b)) => {
                if *b == 0 { bail!("division by zero"); }
                Ok(Value::Int(a / b))
            }

            // Comparisons
            (Value::Int(a), BinOp::Eq, Value::Int(b)) => Ok(Value::Bool(a == b)),
            (Value::Int(a), BinOp::NotEq, Value::Int(b)) => Ok(Value::Bool(a != b)),
            (Value::Int(a), BinOp::Lt, Value::Int(b)) => Ok(Value::Bool(a < b)),
            (Value::Int(a), BinOp::Gt, Value::Int(b)) => Ok(Value::Bool(a > b)),
            (Value::Int(a), BinOp::LtEq, Value::Int(b)) => Ok(Value::Bool(a <= b)),
            (Value::Int(a), BinOp::GtEq, Value::Int(b)) => Ok(Value::Bool(a >= b)),

            (Value::String(a), BinOp::Eq, Value::String(b)) => Ok(Value::Bool(a == b)),
            (Value::String(a), BinOp::NotEq, Value::String(b)) => Ok(Value::Bool(a != b)),

            // Boolean logic
            (Value::Bool(a), BinOp::And, Value::Bool(b)) => Ok(Value::Bool(*a && *b)),
            (Value::Bool(a), BinOp::Or, Value::Bool(b)) => Ok(Value::Bool(*a || *b)),

            // Truthy logic
            (_, BinOp::And, _) => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            (_, BinOp::Or, _) => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),

            _ => bail!("cannot {} {} {} — {} {} {} not supported",
                type_name(left), op_str(op), type_name(right),
                type_name(left), op_str(op), type_name(right)),
        }
    }
}
