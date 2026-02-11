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
    Handle(Handle),
    None,
}

#[derive(Debug, Clone)]
pub enum Handle {
    Stdin,
    File(std::string::String),
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
            Value::Handle(Handle::Stdin) => write!(f, "stdin"),
            Value::Handle(Handle::File(path)) => write!(f, "file(\"{}\")", path),
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
            Value::Handle(_) => true,
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

fn value_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        _ => false,
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
        Value::Handle(_) => "Handle",
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
    types: HashMap<std::string::String, crate::ast::TypeDef>,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut vars = HashMap::new();
        vars.insert("stdin".to_string(), Value::Handle(Handle::Stdin));
        Self { vars, flows: HashMap::new(), types: HashMap::new() }
    }

    pub fn run(&mut self, program: &Program) -> Result<()> {
        // Register all types
        for td in &program.types {
            log::info!("Registered type '{}'", td.name);
            self.types.insert(td.name.clone(), td.clone());
        }

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

    /// Register a type (for REPL use)
    pub fn register_type(&mut self, td: crate::ast::TypeDef) {
        self.types.insert(td.name.clone(), td);
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
                match max {
                    Some(limit) => {
                        for _ in 0..*limit {
                            match self.run_block(body)? {
                                ControlFlow::Break => break,
                                ControlFlow::Continue => continue,
                                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                                ControlFlow::Normal => {}
                            }
                        }
                    }
                    None => {
                        loop {
                            match self.run_block(body)? {
                                ControlFlow::Break => break,
                                ControlFlow::Continue => continue,
                                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                                ControlFlow::Normal => {}
                            }
                        }
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
                        let builtins = ["think", "act", "emit", "run", "log", "print", "remember", "recall", "read", "write", "file"];
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
                    (Value::Map(e), "length") => Ok(Value::Int(e.len() as i64)),
                    (Value::Map(_), _) => {
                        match val.get_field(field) {
                            Some(v) => Ok(v.clone()),
                            None => bail!("map has no key '{}'", field),
                        }
                    }
                    _ => bail!("cannot access field '{}' on {} (type: {})", field, val, type_name(&val)),
                }
            }

            Expr::Index { object, index } => {
                let val = self.eval(object)?;
                let idx = self.eval(index)?;
                match (&val, &idx) {
                    (Value::List(items), Value::Int(i)) => {
                        let i = if *i < 0 { items.len() as i64 + i } else { *i } as usize;
                        items.get(i).cloned()
                            .ok_or_else(|| anyhow::anyhow!("index {} out of range (list has {} elements)", i, items.len()))
                    }
                    (Value::String(s), Value::Int(i)) => {
                        let chars: Vec<char> = s.chars().collect();
                        let i = if *i < 0 { chars.len() as i64 + i } else { *i } as usize;
                        chars.get(i).map(|c| Value::String(c.to_string()))
                            .ok_or_else(|| anyhow::anyhow!("index {} out of range (string has {} characters)", i, chars.len()))
                    }
                    (Value::Map(entries), Value::String(key)) => {
                        entries.iter().find(|(k, _)| k == key)
                            .map(|(_, v)| v.clone())
                            .ok_or_else(|| anyhow::anyhow!("map has no key '{}'", key))
                    }
                    _ => bail!("cannot index {} with {} (type: {}[{}])", type_name(&val), idx, type_name(&val), type_name(&idx)),
                }
            }

            Expr::MethodCall { object, method, args } => {
                let val = self.eval(object)?;
                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(self.eval(a)?);
                }
                self.call_method(val, method, arg_vals)
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
                let mut format_type: Option<std::string::String> = None;
                for (k, v) in kwargs {
                    let val = self.eval(v)?;
                    match k.as_str() {
                        "model" => model = val.to_string(),
                        "system" => system = val.to_string(),
                        "format" => format_type = Some(val.to_string()),
                        "tools" => {} // TODO
                        _ => bail!("think(): unknown kwarg '{}'", k),
                    }
                }

                // If format= is a type name, inject schema into system prompt
                if let Some(ref type_name) = format_type {
                    let schema_instruction = if type_name == "json" {
                        "Respond ONLY with valid JSON. No markdown, no explanation.".to_string()
                    } else if let Some(td) = self.types.get(type_name).cloned() {
                        let schema = self.type_to_schema(&td);
                        format!("Respond ONLY with valid JSON matching this exact schema:\n{}\nNo markdown, no explanation, just the JSON object.", schema)
                    } else {
                        bail!("think(): unknown format type '{}' — define it with: type {}: ...", type_name, type_name)
                    };
                    if system.is_empty() {
                        system = schema_instruction;
                    } else {
                        system = format!("{}\n\n{}", system, schema_instruction);
                    }
                }

                let result = self.call_ollama(&model, &system, &context.to_string())?;

                // If format= specified, parse JSON response into a Map
                if format_type.is_some() {
                    self.parse_json_response(&result)
                } else {
                    Ok(result)
                }
            }
            "file" => {
                if args.is_empty() { bail!("file() requires a path argument"); }
                let path = self.eval(&args[0])?.to_string();
                Ok(Value::Handle(Handle::File(path)))
            }
            "read" => {
                // read() or read(handle) — default: stdin
                let handle = if args.is_empty() {
                    Handle::Stdin
                } else {
                    match self.eval(&args[0])? {
                        Value::Handle(h) => h,
                        other => bail!("read() expects a handle, got {} (type: {}). Use read() for stdin or read(file(\"path\")) for files", other, type_name(&other)),
                    }
                };
                // Optional prompt kwarg for stdin
                let mut prompt = std::string::String::new();
                for (k, v) in kwargs {
                    match k.as_str() {
                        "prompt" => prompt = self.eval(v)?.to_string(),
                        _ => bail!("read(): unknown kwarg '{}'", k),
                    }
                }
                match handle {
                    Handle::Stdin => {
                        if !prompt.is_empty() {
                            print!("{}: ", prompt);
                        } else {
                            print!("> ");
                        }
                        io::stdout().flush()?;
                        let mut line = std::string::String::new();
                        io::stdin().lock().read_line(&mut line)?;
                        if line.is_empty() { bail!("end of input"); }
                        Ok(Value::String(line.trim_end().to_string()))
                    }
                    Handle::File(path) => {
                        let content = std::fs::read_to_string(&path)
                            .map_err(|e| anyhow::anyhow!("cannot read '{}': {}", path, e))?;
                        Ok(Value::String(content))
                    }
                }
            }
            "write" => {
                if args.len() < 2 { bail!("write() requires a handle and content: write(file(\"path\"), content)"); }
                let handle = match self.eval(&args[0])? {
                    Value::Handle(h) => h,
                    other => bail!("write() first argument must be a handle, got {}", type_name(&other)),
                };
                let content = self.eval(&args[1])?.to_string();
                match handle {
                    Handle::Stdin => bail!("cannot write to stdin"),
                    Handle::File(path) => {
                        std::fs::write(&path, &content)
                            .map_err(|e| anyhow::anyhow!("cannot write '{}': {}", path, e))?;
                        Ok(Value::None)
                    }
                }
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

    fn call_method(&mut self, obj: Value, method: &str, args: Vec<Value>) -> Result<Value> {
        match (&obj, method) {
            // ── String methods ──
            (Value::String(s), "upper") => Ok(Value::String(s.to_uppercase())),
            (Value::String(s), "lower") => Ok(Value::String(s.to_lowercase())),
            (Value::String(s), "strip") => Ok(Value::String(s.trim().to_string())),
            (Value::String(s), "starts_with") => {
                let prefix = self.expect_string_arg(method, &args, 0)?;
                Ok(Value::Bool(s.starts_with(&prefix)))
            }
            (Value::String(s), "ends_with") => {
                let suffix = self.expect_string_arg(method, &args, 0)?;
                Ok(Value::Bool(s.ends_with(&suffix)))
            }
            (Value::String(s), "contains") => {
                let needle = self.expect_string_arg(method, &args, 0)?;
                Ok(Value::Bool(s.contains(&needle)))
            }
            (Value::String(s), "replace") => {
                let from = self.expect_string_arg(method, &args, 0)?;
                let to = self.expect_string_arg(method, &args, 1)?;
                Ok(Value::String(s.replace(&from, &to)))
            }
            (Value::String(s), "split") => {
                let delim = self.expect_string_arg(method, &args, 0)?;
                let parts: Vec<Value> = s.split(&delim).map(|p| Value::String(p.to_string())).collect();
                Ok(Value::List(parts))
            }

            // ── List methods ──
            (Value::List(items), "contains") => {
                if args.is_empty() { bail!(".contains() requires an argument"); }
                let needle = &args[0];
                let found = items.iter().any(|item| value_eq(item, needle));
                Ok(Value::Bool(found))
            }
            (Value::List(items), "join") => {
                let sep = if args.is_empty() { "".to_string() } else { args[0].to_string() };
                let joined: Vec<std::string::String> = items.iter().map(|v| v.to_string()).collect();
                Ok(Value::String(joined.join(&sep)))
            }
            (Value::List(items), "reversed") => {
                let mut rev = items.clone();
                rev.reverse();
                Ok(Value::List(rev))
            }
            (Value::List(_items), "push") => {
                // push mutates — need to handle specially
                bail!("push() not yet supported — lists are immutable. Use: new_list = old_list + [item]")
            }

            // ── Map methods ──
            (Value::Map(entries), "keys") => {
                let keys: Vec<Value> = entries.iter().map(|(k, _)| Value::String(k.clone())).collect();
                Ok(Value::List(keys))
            }
            (Value::Map(entries), "values") => {
                let vals: Vec<Value> = entries.iter().map(|(_, v)| v.clone()).collect();
                Ok(Value::List(vals))
            }
            (Value::Map(entries), "contains") => {
                let key = self.expect_string_arg(method, &args, 0)?;
                Ok(Value::Bool(entries.iter().any(|(k, _)| k == &key)))
            }

            _ => bail!("'{}' has no method '{}' (type: {})", obj, method, type_name(&obj)),
        }
    }

    fn expect_string_arg(&self, method: &str, args: &[Value], idx: usize) -> Result<std::string::String> {
        match args.get(idx) {
            Some(Value::String(s)) => Ok(s.clone()),
            Some(other) => bail!(".{}() argument {} must be a String, got {}", method, idx + 1, type_name(other)),
            None => bail!(".{}() requires at least {} argument(s)", method, idx + 1),
        }
    }

    fn type_to_schema(&self, td: &TypeDef) -> std::string::String {
        let fields: Vec<std::string::String> = td.fields.iter().map(|f| {
            let ty_str = self.type_expr_to_json_type(&f.ty);
            format!("  \"{}\": {}", f.name, ty_str)
        }).collect();
        format!("{{\n{}\n}}", fields.join(",\n"))
    }

    fn type_expr_to_json_type(&self, ty: &TypeExpr) -> std::string::String {
        match ty {
            TypeExpr::Named(n) => match n.as_str() {
                "String" => "<string>".to_string(),
                "Int" => "<integer>".to_string(),
                "Float" => "<number>".to_string(),
                "Bool" => "<boolean>".to_string(),
                other => {
                    // Check if it's a nested type
                    if let Some(td) = self.types.get(other) {
                        self.type_to_schema(td)
                    } else {
                        format!("<{}>", other)
                    }
                }
            }
            TypeExpr::Generic(name, args) => match name.as_str() {
                "List" => {
                    let inner = args.first().map(|a| self.type_expr_to_json_type(a)).unwrap_or("<any>".to_string());
                    format!("[{}, ...]", inner)
                }
                "Map" => "<object>".to_string(),
                _ => format!("<{}>", name),
            }
            TypeExpr::Struct(_) => "<object>".to_string(),
        }
    }

    fn parse_json_response(&self, val: &Value) -> Result<Value> {
        let s = val.to_string();
        // Strip markdown code fences if present
        let json_str = s.trim();
        let json_str = if json_str.starts_with("```") {
            let inner = json_str.trim_start_matches("```json").trim_start_matches("```");
            inner.trim_end_matches("```").trim()
        } else {
            json_str
        };

        let parsed: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| anyhow::anyhow!("LLM returned invalid JSON: {}\nResponse was: {}", e, json_str))?;

        Ok(self.json_to_value(parsed))
    }

    fn json_to_value(&self, v: serde_json::Value) -> Value {
        match v {
            serde_json::Value::Null => Value::None,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else {
                    Value::Float(n.as_f64().unwrap_or(0.0))
                }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(arr) => {
                Value::List(arr.into_iter().map(|v| self.json_to_value(v)).collect())
            }
            serde_json::Value::Object(map) => {
                let entries: Vec<(std::string::String, Value)> = map.into_iter()
                    .map(|(k, v)| (k, self.json_to_value(v)))
                    .collect();
                Value::Map(entries)
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

            // List concatenation
            (Value::List(a), BinOp::Add, Value::List(b)) => {
                let mut result = a.clone();
                result.extend(b.clone());
                Ok(Value::List(result))
            }

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
