/// Tree-walking interpreter for Cognos.
/// Executes a parsed AST directly — no kernel needed.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::ast::*;
use crate::environment::{Env, RealEnv};
use crate::trace::{Tracer, TraceEvent};
use anyhow::{bail, Result};


#[derive(Debug, Clone)]
pub enum Value {
    String(std::string::String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<Value>),
    Map(Vec<(std::string::String, Value)>),  // ordered key-value pairs
    Handle(Handle),
    Module(std::string::String),
    Future(u64),
    None,
}

#[derive(Debug, Clone)]
pub enum Handle {
    Stdin,
    Stdout,
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
            Value::Module(name) => write!(f, "<module '{}'>", name),
            Value::Handle(Handle::Stdin) => write!(f, "stdin"),
            Value::Handle(Handle::Stdout) => write!(f, "stdout"),
            Value::Handle(Handle::File(path)) => write!(f, "file(\"{}\")", path),
            Value::Future(id) => write!(f, "<future:{}>", id),
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
            Value::Module(_) => true,
            Value::Future(_) => true,
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
        Value::Module(_) => "Module",
        Value::Future(_) => "Future",
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
    env: Arc<Mutex<Box<dyn Env + Send>>>,
    tracer: Option<Arc<Tracer>>,
    import_stack: Vec<std::string::String>,
    conversation_history: Vec<(std::string::String, std::string::String)>,
    next_future_id: u64,
    async_handles: HashMap<u64, std::thread::JoinHandle<Result<Value>>>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self::with_options(false)
    }

    pub fn with_options(allow_shell: bool) -> Self {
        Self::with_full_options(allow_shell, None)
    }

    pub fn with_full_options(allow_shell: bool, tracer: Option<Arc<Tracer>>) -> Self {
        Self::with_env(Box::new(RealEnv::new(allow_shell)), tracer)
    }

    pub fn with_env(env: Box<dyn Env + Send>, tracer: Option<Arc<Tracer>>) -> Self {
        let mut vars = HashMap::new();
        vars.insert("stdin".to_string(), Value::Handle(Handle::Stdin));
        vars.insert("stdout".to_string(), Value::Handle(Handle::Stdout));
        // math module removed (P11: lean core runtime)
        vars.insert("http".to_string(), Value::Module("http".to_string()));
        Self { vars, flows: HashMap::new(), types: HashMap::new(), env: Arc::from(Mutex::new(env)), tracer, import_stack: Vec::new(), conversation_history: Vec::new(), next_future_id: 0, async_handles: HashMap::new() }
    }

    pub fn load_session(&mut self, path: &str) -> anyhow::Result<()> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("cannot load session '{}': {}", path, e))?;
        let json: serde_json::Value = serde_json::from_str(&content)?;
        if let Some(obj) = json.as_object() {
            for (k, v) in obj {
                self.vars.insert(k.clone(), self.json_to_value(v.clone()));
            }
        }
        log::info!("Loaded session from {}", path);
        Ok(())
    }

    pub fn save_session(&self, path: &str) -> anyhow::Result<()> {
        let mut map = serde_json::Map::new();
        for (k, v) in &self.vars {
            // Skip builtins
            match k.as_str() {
                "stdin" | "stdout" | "http" => continue,
                _ => {}
            }
            map.insert(k.clone(), self.value_to_json(v));
        }
        std::fs::write(path, serde_json::to_string_pretty(&serde_json::Value::Object(map))?)?;
        log::info!("Saved session to {}", path);
        Ok(())
    }

    pub fn captured_stdout(&self) -> Option<Vec<String>> {
        self.env.lock().unwrap().captured_stdout()
    }

    fn trace(&self, event: TraceEvent) {
        if let Some(ref tracer) = self.tracer {
            tracer.emit(event);
        }
    }

    fn is_full_trace(&self) -> bool {
        self.tracer.as_ref().map(|t| t.level == crate::trace::TraceLevel::Full).unwrap_or(false)
    }

    fn trace_llm(&self, model: &str, provider: &str, latency_ms: u64, prompt: &str, system: &str, response: &str, has_tool_calls: bool) {
        let full = self.is_full_trace();
        self.trace(TraceEvent::LlmCall {
            model: model.to_string(), provider: provider.to_string(),
            latency_ms, prompt_chars: prompt.len(), response_chars: response.len(),
            has_tool_calls, error: None,
            prompt: if full { Some(prompt.to_string()) } else { None },
            response: if full { Some(response.to_string()) } else { None },
            system: if full { Some(system.to_string()) } else { None },
        });
    }

    pub fn run(&mut self, program: &Program) -> Result<()> {
        self.run_with_base(program, None)
    }

    pub fn run_with_base(&mut self, program: &Program, base_path: Option<&std::path::Path>) -> Result<()> {
        // Resolve imports
        for import_path in &program.imports {
            let resolved = if let Some(base) = base_path {
                base.parent().unwrap_or(base).join(import_path)
            } else {
                std::path::PathBuf::from(import_path)
            };
            let canonical = resolved.canonicalize()
                .unwrap_or_else(|_| resolved.clone())
                .to_string_lossy().to_string();
            if self.import_stack.contains(&canonical) {
                bail!("circular import detected: '{}' is already being imported", import_path);
            }
            self.import_stack.push(canonical.clone());
            log::info!("Importing {:?}", resolved);
            let source = std::fs::read_to_string(&resolved)
                .map_err(|e| anyhow::anyhow!("cannot import '{}': {}", import_path, e))?;
            let mut lexer = crate::lexer::Lexer::new(&source);
            let tokens = lexer.tokenize();
            let mut parser = crate::parser::Parser::new(tokens);
            let imported = parser.parse_program()
                .map_err(|e| anyhow::anyhow!("error in '{}': {}", import_path, e))?;
            // Recursively resolve imports in the imported file
            self.run_with_base(&Program {
                imports: imported.imports,
                types: imported.types,
                flows: vec![], // don't run flows from imports
            }, Some(&resolved))?;
            // Register imported flows
            for flow in &imported.flows {
                log::info!("Imported flow '{}'", flow.name);
                self.flows.insert(flow.name.clone(), flow.clone());
            }
            self.import_stack.pop();
        }

        // Register all types
        for td in &program.types {
            log::info!("Registered type '{}'", td.name());
            self.types.insert(td.name().to_string(), td.clone());
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
                    let val = self.env.lock().unwrap().read_stdin()?;
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
        self.types.insert(td.name().to_string(), td);
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

    /// Call a user-defined flow with positional and keyword arguments
    fn call_flow(&mut self, name: &str, args: Vec<Value>, kwargs: Vec<(std::string::String, Value)>) -> Result<Value> {
        let flow = self.flows.get(name).cloned()
            .ok_or_else(|| anyhow::anyhow!("unknown flow: {}", name))?;

        // Build parameter bindings from positional args + kwargs
        let mut bindings: HashMap<std::string::String, Value> = HashMap::new();

        // First, bind positional args in order
        if args.len() > flow.params.len() {
            bail!("{}() expects {} args, got {}", name, flow.params.len(), args.len());
        }
        for (i, val) in args.iter().enumerate() {
            bindings.insert(flow.params[i].name.clone(), val.clone());
        }

        // Then, bind kwargs by name
        for (k, v) in &kwargs {
            // Check kwarg name is a valid parameter
            if !flow.params.iter().any(|p| &p.name == k) {
                bail!("{}(): unknown keyword argument '{}'", name, k);
            }
            // Check for duplicate (already bound by positional)
            if bindings.contains_key(k) {
                bail!("{}(): duplicate argument for '{}'", name, k);
            }
            bindings.insert(k.clone(), v.clone());
        }

        // Check all params are bound; use defaults if available
        for param in &flow.params {
            if !bindings.contains_key(&param.name) {
                if let Some(ref default_expr) = param.default {
                    let val = self.eval(default_expr)?;
                    bindings.insert(param.name.clone(), val);
                } else {
                    bail!("{}(): missing required argument '{}'", name, param.name);
                }
            }
        }

        // Save current vars, set up new scope (preserve builtins)
        let saved_vars = self.vars.clone();
        let mut new_vars = HashMap::new();
        // Preserve builtins
        for key in &["stdin", "stdout", "http"] {
            if let Some(v) = saved_vars.get(*key) {
                new_vars.insert(key.to_string(), v.clone());
            }
        }
        for (k, v) in bindings {
            new_vars.insert(k, v);
        }
        self.vars = new_vars;

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
                // emit(x) is sugar for write(stdout, x)
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

            Stmt::TryCatch { body, error_var, catch_body } => {
                match self.run_block(body) {
                    Ok(cf) => Ok(cf),
                    Err(e) => {
                        if let Some(var) = error_var {
                            self.vars.insert(var.clone(), Value::String(format!("{}", e)));
                        }
                        self.run_block(catch_body)
                    }
                }
            }

            Stmt::For { var, value_var, iterable, body } => {
                let collection = self.eval(iterable)?;
                match (&collection, value_var) {
                    (Value::Map(entries), Some(vv)) => {
                        // for key, value in map:
                        let entries = entries.clone();
                        for (k, v) in entries {
                            self.vars.insert(var.clone(), Value::String(k));
                            self.vars.insert(vv.clone(), v);
                            match self.run_block(body)? {
                                ControlFlow::Break => break,
                                ControlFlow::Continue => continue,
                                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                                ControlFlow::Normal => {}
                            }
                        }
                    }
                    (Value::List(items), Some(vv)) => {
                        // for index, value in list:
                        let items = items.clone();
                        for (i, item) in items.into_iter().enumerate() {
                            self.vars.insert(var.clone(), Value::Int(i as i64));
                            self.vars.insert(vv.clone(), item);
                            match self.run_block(body)? {
                                ControlFlow::Break => break,
                                ControlFlow::Continue => continue,
                                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                                ControlFlow::Normal => {}
                            }
                        }
                    }
                    (_, Some(_)) => bail!("two-variable for loop requires a Map or List"),
                    _ => {
                        // Single variable iteration
                        let items: Vec<Value> = match collection {
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
                    }
                }
                Ok(ControlFlow::Normal)
            }

            Stmt::Parallel { branches } => {
                self.run_parallel(branches)?;
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

    fn run_parallel(&mut self, branches: &[Vec<Stmt>]) -> Result<()> {
        // Each branch runs concurrently as a block of statements.
        // Each branch gets a snapshot of current vars; new/changed vars are merged back.
        let env = self.env.clone();
        let flows = self.flows.clone();
        let types = self.types.clone();
        let vars = self.vars.clone();
        let tracer = self.tracer.clone();

        // Each branch returns its final vars (new/changed only)
        let results: Vec<Result<HashMap<String, Value>>> = std::thread::scope(|s| {
            let handles: Vec<_> = branches.iter().map(|branch| {
                let env = env.clone();
                let flows = flows.clone();
                let types = types.clone();
                let vars = vars.clone();
                let tracer = tracer.clone();
                let branch = branch.clone();
                s.spawn(move || {
                    let mut interp = Interpreter {
                        vars: vars.clone(),
                        flows,
                        types,
                        env,
                        tracer,
                        import_stack: Vec::new(),
                        conversation_history: Vec::new(),
                        next_future_id: 0,
                        async_handles: HashMap::new(),
                    };
                    interp.run_block(&branch)?;
                    // Return only new/changed vars
                    let mut changed = HashMap::new();
                    for (k, v) in &interp.vars {
                        match vars.get(k) {
                            None => { changed.insert(k.clone(), v.clone()); }
                            Some(old) => {
                                if old.to_string() != v.to_string() {
                                    changed.insert(k.clone(), v.clone());
                                }
                            }
                        }
                    }
                    Ok(changed)
                })
            }).collect();

            handles.into_iter().map(|h| {
                match h.join() {
                    Ok(r) => r,
                    Err(_) => Err(anyhow::anyhow!("parallel branch panicked")),
                }
            }).collect()
        });

        // Check for errors, merge results
        let mut errors = Vec::new();
        for result in results {
            match result {
                Ok(changed) => {
                    for (k, v) in changed {
                        self.vars.insert(k, v);
                    }
                }
                Err(e) => errors.push(e),
            }
        }

        if !errors.is_empty() {
            let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            bail!("parallel block errors:\n  {}", msgs.join("\n  "));
        }

        Ok(())
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
                        let builtins = ["think", "invoke", "emit", "log", "print", "remember", "recall", "read", "write", "file", "__exec_shell__", "history", "clear_history"];
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

            Expr::Async(inner) => {
                // Spawn the expression evaluation in a background thread
                let env = self.env.clone();
                let flows = self.flows.clone();
                let types = self.types.clone();
                let vars = self.vars.clone();
                let tracer = self.tracer.clone();
                let inner = (**inner).clone();

                let handle = std::thread::spawn(move || {
                    let mut interp = Interpreter {
                        vars,
                        flows,
                        types,
                        env,
                        tracer,
                        import_stack: Vec::new(),
                        conversation_history: Vec::new(),
                        next_future_id: 0,
                        async_handles: HashMap::new(),
                    };
                    interp.eval(&inner)
                });

                let id = self.next_future_id;
                self.next_future_id += 1;
                self.async_handles.insert(id, handle);
                Ok(Value::Future(id))
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
                // Module constants: math.pi, math.e
                if let Value::Module(ref mod_name) = val {
                    return match (mod_name.as_str(), field.as_str()) {
                        // math module removed (P11)
                        _ => bail!("{} has no constant '{}'", mod_name, field),
                    };
                }
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

            Expr::Slice { object, start, end } => {
                let val = self.eval(object)?;
                let s = start.as_ref().map(|e| self.eval(e)).transpose()?;
                let e = end.as_ref().map(|e| self.eval(e)).transpose()?;

                // Helper to resolve a slice index (supports negative)
                fn resolve_slice_idx(idx: i64, len: usize) -> usize {
                    if idx < 0 {
                        let resolved = len as i64 + idx;
                        if resolved < 0 { 0 } else { resolved as usize }
                    } else {
                        (idx as usize).min(len)
                    }
                }

                match val {
                    Value::String(ref sv) => {
                        let chars: Vec<char> = sv.chars().collect();
                        let len = chars.len();
                        let start_idx = match s { Some(Value::Int(i)) => resolve_slice_idx(i, len), None => 0, _ => bail!("slice start must be Int") };
                        let end_idx = match e { Some(Value::Int(i)) => resolve_slice_idx(i, len), None => len, _ => bail!("slice end must be Int") };
                        if start_idx >= end_idx { return Ok(Value::String(String::new())); }
                        Ok(Value::String(chars[start_idx..end_idx].iter().collect()))
                    }
                    Value::List(ref items) => {
                        let len = items.len();
                        let start_idx = match s { Some(Value::Int(i)) => resolve_slice_idx(i, len), None => 0, _ => bail!("slice start must be Int") };
                        let end_idx = match e { Some(Value::Int(i)) => resolve_slice_idx(i, len), None => len, _ => bail!("slice end must be Int") };
                        if start_idx >= end_idx { return Ok(Value::List(vec![])); }
                        Ok(Value::List(items[start_idx..end_idx].to_vec()))
                    }
                    other => bail!("cannot slice {} (type: {})", other, type_name(&other)),
                }
            }

            Expr::MethodCall { object, method, args } => {
                let val = self.eval(object)?;
                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(self.eval(a)?);
                }
                if let Value::Module(ref mod_name) = val {
                    return self.call_module(mod_name, method, arg_vals);
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
                let mut tool_names: Vec<std::string::String> = Vec::new();
                for (k, v) in kwargs {
                    let val = self.eval(v)?;
                    match k.as_str() {
                        "model" => model = val.to_string(),
                        "system" => system = val.to_string(),
                        "format" => format_type = Some(val.to_string()),
                        "tools" => {
                            if let Value::List(items) = val {
                                for item in items {
                                    tool_names.push(item.to_string());
                                }
                            } else {
                                bail!("tools= must be a list, got {}", type_name(&val));
                            }
                        }
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

                // Build tool definitions from flow signatures
                let tool_defs = if !tool_names.is_empty() {
                    let mut tools = Vec::new();
                    for name in &tool_names {
                        let flow = self.flows.get(name)
                            .ok_or_else(|| anyhow::anyhow!("tools: flow '{}' not defined", name))?
                            .clone();
                        tools.push(self.flow_to_tool_json(&flow));
                    }
                    Some(tools)
                } else {
                    None
                };

                let prompt_text = context.to_string();
                let result = self.call_llm(&model, &system, &prompt_text, tool_defs.clone())?;

                // Track conversation history
                self.conversation_history.push(("user".to_string(), prompt_text.clone()));
                let response_text = match &result {
                    Value::String(s) => s.clone(),
                    Value::Map(entries) => entries.iter()
                        .find(|(k, _)| k == "content")
                        .map(|(_, v)| v.to_string())
                        .unwrap_or_default(),
                    other => other.to_string(),
                };
                self.conversation_history.push(("assistant".to_string(), response_text));

                // If format= specified, parse JSON and validate against type
                if let Some(ref tn) = format_type {
                    let parsed = self.parse_json_response(&result)?;
                    if tn != "json" {
                        if let Some(td) = self.types.get(tn).cloned() {
                            self.validate_type(&parsed, &td)?;
                        }
                    }
                    Ok(parsed)
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
                        other => bail!("read() expects a handle, got {} (type: {})", other, type_name(&other)),
                    }
                };
                match handle {
                    Handle::Stdin => {
                        let val = self.env.lock().unwrap().read_stdin()?;
                        if let Some(ref tracer) = self.tracer {
                            tracer.increment_turn();
                        }
                        let full = self.is_full_trace();
                        self.trace(TraceEvent::IoOp {
                            operation: "read".into(), handle_type: "stdin".into(),
                            path: None, bytes: val.len(),
                            content: if full { Some(val.clone()) } else { None },
                        });
                        Ok(Value::String(val))
                    }
                    Handle::Stdout => bail!("cannot read from stdout"),
                    Handle::File(path) => {
                        let content = self.env.lock().unwrap().read_file(&path)?;
                        let full = self.is_full_trace();
                        self.trace(TraceEvent::IoOp {
                            operation: "read".into(), handle_type: "file".into(),
                            path: Some(path), bytes: content.len(),
                            content: if full { Some(content.chars().take(1000).collect()) } else { None },
                        });
                        Ok(Value::String(content))
                    }
                }
            }
            "write" => {
                if args.len() < 2 { bail!("write(handle, content) — e.g. write(stdout, \"hello\") or write(file(\"path\"), content)"); }
                let handle = match self.eval(&args[0])? {
                    Value::Handle(h) => h,
                    other => bail!("write() first argument must be a handle, got {}", type_name(&other)),
                };
                let content = self.eval(&args[1])?.to_string();
                match handle {
                    Handle::Stdin => bail!("cannot write to stdin"),
                    Handle::Stdout => {
                        self.env.lock().unwrap().write_stdout(&content)?;
                        let full = self.is_full_trace();
                        self.trace(TraceEvent::IoOp {
                            operation: "write".into(), handle_type: "stdout".into(),
                            path: None, bytes: content.len(),
                            content: if full { Some(content) } else { None },
                        });
                        Ok(Value::None)
                    }
                    Handle::File(path) => {
                        self.env.lock().unwrap().write_file(&path, &content)?;
                        let full = self.is_full_trace();
                        self.trace(TraceEvent::IoOp {
                            operation: "write".into(), handle_type: "file".into(),
                            path: Some(path), bytes: content.len(),
                            content: if full { Some(content) } else { None },
                        });
                        Ok(Value::None)
                    }
                }
            }
            "invoke" => {
                // invoke(name, args) — call a flow by string name with a Map of arguments
                if args.is_empty() {
                    bail!("invoke() requires a flow name: invoke(\"flow_name\", {{\"arg\": value}})");
                }
                let flow_name = self.eval(&args[0])?.to_string();
                let flow_args = if args.len() > 1 {
                    self.eval(&args[1])?
                } else {
                    Value::Map(vec![])
                };

                // Convert Map to kwargs
                let kwarg_vals = match flow_args {
                    Value::Map(entries) => entries,
                    other => bail!("invoke() second argument must be a Map, got {}", type_name(&other)),
                };

                self.call_flow(&flow_name, vec![], kwarg_vals)
            }
            "__exec_shell__" => {
                if !self.env.lock().unwrap().allow_shell() {
                    bail!("shell execution is disabled — use: cognos run --allow-shell file.cog");
                }
                if args.is_empty() { bail!("__exec_shell__() requires a command string"); }
                let cmd = self.eval(&args[0])?.to_string();
                log::info!("__exec_shell__ → {:?}", cmd);
                let shell_start = std::time::Instant::now();
                let result = self.env.lock().unwrap().exec_shell(&cmd)?;
                let shell_output = if self.is_full_trace() { Some(result.stdout.clone()) } else { None };
                self.trace(TraceEvent::ShellExec {
                    command: cmd, latency_ms: shell_start.elapsed().as_millis() as u64,
                    exit_code: result.exit_code, output_chars: result.stdout.len(), output: shell_output,
                });
                Ok(Value::String(result.stdout))
            }
            "save" => {
                // save(path, value) — persist a value as JSON via Env
                if args.len() < 2 { bail!("save(path, value)"); }
                let path = self.eval(&args[0])?.to_string();
                let value = self.eval(&args[1])?;
                let json = self.value_to_json(&value);
                let content = serde_json::to_string_pretty(&json)?;
                self.env.lock().unwrap().write_file(&path, &content)?;
                log::info!("Saved to {}", path);
                Ok(Value::None)
            }
            "load" => {
                // load(path) — load a JSON file back to a Value via Env
                if args.is_empty() { bail!("load(path)"); }
                let path = self.eval(&args[0])?.to_string();
                let content = self.env.lock().unwrap().read_file(&path)?;
                let json: serde_json::Value = serde_json::from_str(&content)
                    .map_err(|e| anyhow::anyhow!("load JSON error: {}", e))?;
                log::info!("Loaded from {}", path);
                Ok(self.json_to_value(json))
            }
            "await" => {
                if args.is_empty() { bail!("await() requires a future handle"); }
                let val = self.eval(&args[0])?;
                match val {
                    Value::Future(id) => {
                        let handle = self.async_handles.remove(&id)
                            .ok_or_else(|| anyhow::anyhow!("invalid or already-consumed future handle {}", id))?;
                        match handle.join() {
                            Ok(result) => result,
                            Err(_) => bail!("async task panicked"),
                        }
                    }
                    other => bail!("await() expects a Future, got {} (type: {})", other, type_name(&other)),
                }
            }
            "log" => {
                for arg in args {
                    let val = self.eval(arg)?;
                    eprintln!("[log] {}", val);
                }
                Ok(Value::None)
            }
            "history" => {
                let entries: Vec<Value> = self.conversation_history.iter().map(|(role, content)| {
                    Value::Map(vec![
                        ("role".to_string(), Value::String(role.clone())),
                        ("content".to_string(), Value::String(content.clone())),
                    ])
                }).collect();
                Ok(Value::List(entries))
            }
            "clear_history" => {
                self.conversation_history.clear();
                Ok(Value::None)
            }
            _ => {
                // Try user-defined flow
                if self.flows.contains_key(name) {
                    let mut arg_vals = Vec::new();
                    for arg in args {
                        arg_vals.push(self.eval(arg)?);
                    }
                    let mut kwarg_vals = Vec::new();
                    for (k, v) in kwargs {
                        kwarg_vals.push((k.clone(), self.eval(v)?));
                    }
                    return self.call_flow(name, arg_vals, kwarg_vals);
                }
                bail!("unknown function: {}()", name)
            }
        }
    }

    fn call_module(&mut self, module: &str, method: &str, args: Vec<Value>) -> Result<Value> {
        match module {
            "math" => bail!("math module was removed (P11: lean core runtime). Use shell() for math operations."),
            "http" => self.call_http(method, args),
            _ => bail!("unknown module '{}'", module),
        }
    }

    fn to_float(v: &Value) -> Result<f64> {
        match v {
            Value::Float(f) => Ok(*f),
            Value::Int(i) => Ok(*i as f64),
            other => bail!("expected a number, got {} (type: {})", other, type_name(other)),
        }
    }

    // math module removed — P11: lean core runtime

    fn call_http(&mut self, method: &str, args: Vec<Value>) -> Result<Value> {
        match method {
            "get" => {
                if args.is_empty() { bail!("http.get() requires a URL"); }
                let url = args[0].to_string();
                log::info!("http.get({})", url);
                let body = self.env.lock().unwrap().http_get(&url)?;
                Ok(Value::String(body))
            }
            "post" => {
                if args.len() < 2 { bail!("http.post(url, body)"); }
                let url = args[0].to_string();
                let body = args[1].to_string();
                log::info!("http.post({})", url);
                let resp = self.env.lock().unwrap().http_post(&url, &body)?;
                Ok(Value::String(resp))
            }
            _ => bail!("http has no function '{}'", method),
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
            (Value::String(s), "truncate") => {
                let max = match args.get(0) {
                    Some(Value::Int(n)) => *n as usize,
                    _ => bail!(".truncate() requires an Int argument"),
                };
                if s.len() <= max {
                    Ok(Value::String(s.clone()))
                } else {
                    let truncated: String = s.chars().take(max).collect();
                    Ok(Value::String(format!("{}...", truncated)))
                }
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
        match td {
            TypeDef::Struct { fields, .. } => {
                let field_strs: Vec<std::string::String> = fields.iter().map(|f| {
                    let ty_str = self.type_expr_to_json_type(&f.ty);
                    if f.optional {
                        format!("  \"{}\"?: {}", f.name, ty_str)
                    } else {
                        format!("  \"{}\": {}", f.name, ty_str)
                    }
                }).collect();
                format!("{{\n{}\n}}", field_strs.join(",\n"))
            }
            TypeDef::Enum { variants, .. } => {
                let quoted: Vec<std::string::String> = variants.iter().map(|v| format!("\"{}\"", v)).collect();
                format!("one of: {}", quoted.join(", "))
            }
        }
    }

    fn type_expr_to_json_type(&self, ty: &TypeExpr) -> std::string::String {
        match ty {
            TypeExpr::Named(n) => match n.as_str() {
                "String" | "Text" => "<string>".to_string(),
                "Int" => "<integer>".to_string(),
                "Float" => "<number>".to_string(),
                "Bool" => "<boolean>".to_string(),
                other => {
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
                "Map" => {
                    if args.len() >= 2 {
                        let key = self.type_expr_to_json_type(&args[0]);
                        let val = self.type_expr_to_json_type(&args[1]);
                        format!("{{{}: {}, ...}}", key, val)
                    } else {
                        "<object>".to_string()
                    }
                }
                _ => format!("<{}>", name),
            }
            TypeExpr::Struct(_) => "<object>".to_string(),
        }
    }

    fn validate_type(&self, val: &Value, td: &crate::ast::TypeDef) -> Result<()> {
        match td {
            TypeDef::Enum { name, variants } => {
                match val {
                    Value::String(s) => {
                        if !variants.contains(s) {
                            bail!("type {} validation failed: '{}' is not one of [{}]",
                                name, s, variants.join(", "));
                        }
                        Ok(())
                    }
                    other => bail!("expected {} (String enum), got {}", name, type_name(other)),
                }
            }
            TypeDef::Struct { name, fields } => {
                let map = match val {
                    Value::Map(entries) => entries,
                    other => bail!("expected {} (Map), got {}", name, type_name(other)),
                };

                let mut errors = Vec::new();

                for field in fields {
                    match map.iter().find(|(k, _)| k == &field.name) {
                        None => {
                            if !field.optional {
                                errors.push(format!("missing field '{}'", field.name));
                            }
                        }
                        Some((_, val)) => {
                            if let Err(e) = self.validate_field_value(val, &field.ty) {
                                errors.push(format!("field '{}': {}", field.name, e));
                            }
                        }
                    }
                }

                if errors.is_empty() {
                    Ok(())
                } else {
                    bail!("type {} validation failed:\n  {}\nLLM response: {}", name, errors.join("\n  "), val)
                }
            }
        }
    }

    fn validate_field_value(&self, val: &Value, ty: &crate::ast::TypeExpr) -> Result<()> {
        match ty {
            crate::ast::TypeExpr::Named(name) => {
                let ok = match name.as_str() {
                    "Int" => matches!(val, Value::Int(_)),
                    "Float" => matches!(val, Value::Float(_) | Value::Int(_)),
                    "String" | "Text" => matches!(val, Value::String(_)),
                    "Bool" => matches!(val, Value::Bool(_)),
                    "List" => matches!(val, Value::List(_)),
                    "Map" => matches!(val, Value::Map(_)),
                    other => {
                        // Check for user-defined type (struct or enum)
                        if let Some(td) = self.types.get(other) {
                            return self.validate_type(val, td);
                        }
                        true
                    }
                };
                if !ok {
                    bail!("expected {}, got {} ({})", name, type_name(val), val);
                }
                Ok(())
            }
            crate::ast::TypeExpr::Generic(name, args) => {
                match name.as_str() {
                    "List" => {
                        let items = match val {
                            Value::List(items) => items,
                            other => bail!("expected List, got {}", type_name(other)),
                        };
                        // Validate inner type if specified
                        if let Some(inner_ty) = args.first() {
                            for (i, item) in items.iter().enumerate() {
                                if let Err(e) = self.validate_field_value(item, inner_ty) {
                                    bail!("element [{}]: {}", i, e);
                                }
                            }
                        }
                        Ok(())
                    }
                    "Map" => {
                        let entries = match val {
                            Value::Map(entries) => entries,
                            other => bail!("expected Map, got {}", type_name(other)),
                        };
                        if args.len() >= 2 {
                            let val_ty = &args[1];
                            for (k, v) in entries {
                                if let Err(e) = self.validate_field_value(v, val_ty) {
                                    bail!("key '{}': {}", k, e);
                                }
                            }
                        }
                        Ok(())
                    }
                    _ => Ok(()),
                }
            }
            crate::ast::TypeExpr::Struct(_) => Ok(()),
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

    fn value_to_json(&self, value: &Value) -> serde_json::Value {
        match value {
            Value::String(s) => serde_json::Value::String(s.clone()),
            Value::Int(n) => serde_json::json!(*n),
            Value::Float(f) => serde_json::json!(*f),
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::None => serde_json::Value::Null,
            Value::List(items) => serde_json::Value::Array(items.iter().map(|v| self.value_to_json(v)).collect()),
            Value::Map(pairs) => {
                let mut map = serde_json::Map::new();
                for (k, v) in pairs { map.insert(k.clone(), self.value_to_json(v)); }
                serde_json::Value::Object(map)
            }
            Value::Handle(_) => serde_json::Value::String("<handle>".into()),
            Value::Module(name) => serde_json::Value::String(format!("<module:{}>", name)),
            Value::Future(id) => serde_json::Value::String(format!("<future:{}>", id)),
        }
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

    fn flow_to_tool_json(&self, flow: &FlowDef) -> serde_json::Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();
        for param in &flow.params {
            let ty = match &param.ty {
                TypeExpr::Named(n) => match n.as_str() {
                    "String" => "string",
                    "Int" => "integer",
                    "Float" => "number",
                    "Bool" => "boolean",
                    _ => "string",
                },
                _ => "string",
            };
            properties.insert(param.name.clone(), serde_json::json!({
                "type": ty,
                "description": format!("Parameter '{}'", param.name)
            }));
            required.push(serde_json::Value::String(param.name.clone()));
        }
        let desc = flow.description.clone()
            .unwrap_or_else(|| format!("Flow '{}'", flow.name));
        serde_json::json!({
            "type": "function",
            "function": {
                "name": flow.name,
                "description": desc,
                "parameters": {
                    "type": "object",
                    "properties": properties,
                    "required": required
                }
            }
        })
    }

    fn call_llm(&mut self, model: &str, system: &str, prompt: &str, tools: Option<Vec<serde_json::Value>>) -> Result<Value> {
        // Check if mock env handles LLM calls
        if self.env.lock().unwrap().is_mock() {
            // Mock environment — use env.call_llm
            let request = crate::environment::LlmRequest {
                model: model.to_string(), system: system.to_string(),
                prompt: prompt.to_string(), tools: tools.clone(),
                format: None, history: vec![],
            };
            let resp = self.env.lock().unwrap().call_llm(request)?;
            let has_tc = resp.tool_calls.is_some();
            self.trace_llm(model, "mock", 0, prompt, system, &resp.content, has_tc);
            if let Some(tc) = resp.tool_calls {
                let tool_calls: Vec<Value> = tc.iter().map(|c| {
                    let name = c["name"].as_str().unwrap_or("").to_string();
                    let arguments = self.json_to_value(c["arguments"].clone());
                    Value::Map(vec![
                        ("name".to_string(), Value::String(name)),
                        ("arguments".to_string(), arguments),
                    ])
                }).collect();
                return Ok(Value::Map(vec![
                    ("content".to_string(), Value::String(resp.content)),
                    ("tool_calls".to_string(), Value::List(tool_calls)),
                    ("has_tool_calls".to_string(), Value::Bool(true)),
                ]));
            }
            if tools.is_some() {
                return Ok(Value::Map(vec![
                    ("content".to_string(), Value::String(resp.content)),
                    ("has_tool_calls".to_string(), Value::Bool(false)),
                ]));
            }
            return Ok(Value::String(resp.content));
        }
        // Real environment — route to correct provider
        if model.starts_with("claude") {
            return self.call_claude_cli(model, system, prompt, tools);
        }
        if model.starts_with("gpt-") || model.starts_with("o1-") || model.starts_with("o3-") {
            return self.call_openai(model, system, prompt, tools);
        }
        self.call_ollama(model, system, prompt, tools)
    }

    fn call_claude_cli(&self, model: &str, system: &str, prompt: &str, tools: Option<Vec<serde_json::Value>>) -> Result<Value> {
        log::info!("Calling Claude CLI: model={}, tools={}", model, tools.as_ref().map(|t| t.len()).unwrap_or(0));
        let call_start = std::time::Instant::now();

        // Build system prompt with tools embedded
        let mut full_system = system.to_string();
        if let Some(ref tool_defs) = tools {
            full_system.push_str("\n\n## Available Tools\n\nYou have access to these tools. To use a tool, respond with a JSON block:\n```json\n{\"tool_calls\": [{\"name\": \"tool_name\", \"arguments\": {\"arg\": \"value\"}}]}\n```\n\nTools:\n");
            for t in tool_defs {
                let name = t["function"]["name"].as_str().unwrap_or("");
                let desc = t["function"]["description"].as_str().unwrap_or("");
                let params = serde_json::to_string_pretty(&t["function"]["parameters"]).unwrap_or_default();
                full_system.push_str(&format!("\n### {}\n{}\nParameters: {}\n", name, desc, params));
            }
            full_system.push_str("\nIMPORTANT: If you want to use a tool, respond ONLY with the JSON block above. No other text. If you don't need a tool, respond normally.\n");
        }

        let output = std::process::Command::new("claude")
            .args([
                "-p",
                "--output-format", "json",
                "--no-session-persistence",
                "--model", model,
                "--system-prompt", &full_system,
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin {
                    stdin.write_all(prompt.as_bytes())?;
                }
                child.wait_with_output()
            })
            .map_err(|e| anyhow::anyhow!("Claude CLI error: {}. Is 'claude' installed?", e))?;

        if !output.status.success() {
            let err = std::string::String::from_utf8_lossy(&output.stdout);
            bail!("Claude CLI failed (exit {}): {}", output.status, err);
        }

        let stdout = std::string::String::from_utf8_lossy(&output.stdout);

        // Parse JSON response from claude CLI
        let parsed: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| anyhow::anyhow!("Failed to parse Claude CLI output: {}\nRaw: {}", e, &stdout[..stdout.len().min(500)]))?;

        let raw_text = parsed["result"].as_str().unwrap_or("").to_string();

        if parsed["is_error"] == serde_json::Value::Bool(true) {
            bail!("Claude CLI error: {}", raw_text);
        }

        let latency = call_start.elapsed().as_millis() as u64;
        log::info!("Claude CLI response: {} chars in {}ms", raw_text.len(), latency);

        // Parse tool calls from response text
        if tools.is_some() {
            if let Some(tool_calls) = self.parse_tool_calls_from_text(&raw_text) {
                let content = raw_text.split("```json").next().unwrap_or("").trim().to_string();
                self.trace_llm(model, "claude-cli", latency, prompt, &full_system, &raw_text, true);
                return Ok(Value::Map(vec![
                    ("content".to_string(), Value::String(content)),
                    ("tool_calls".to_string(), Value::List(tool_calls)),
                    ("has_tool_calls".to_string(), Value::Bool(true)),
                ]));
            }
            self.trace_llm(model, "claude-cli", latency, prompt, &full_system, &raw_text, false);
            return Ok(Value::Map(vec![
                ("content".to_string(), Value::String(raw_text)),
                ("has_tool_calls".to_string(), Value::Bool(false)),
            ]));
        }

        self.trace_llm(model, "claude-cli", latency, prompt, &full_system, &raw_text, false);
        Ok(Value::String(raw_text))
    }

    fn parse_tool_calls_from_text(&self, text: &str) -> Option<Vec<Value>> {
        // Look for ```json { "tool_calls": [...] } ```
        let json_str = if let Some(start) = text.find("```json") {
            let after = &text[start + 7..];
            after.find("```").map(|end| after[..end].trim())
        } else if let Some(start) = text.find("{\"tool_calls\"") {
            // Raw JSON
            let after = &text[start..];
            let mut depth = 0;
            let mut end = 0;
            for (i, c) in after.chars().enumerate() {
                match c {
                    '{' => depth += 1,
                    '}' => { depth -= 1; if depth == 0 { end = i + 1; break; } }
                    _ => {}
                }
            }
            if end > 0 { Some(&after[..end]) } else { None }
        } else {
            None
        }?;

        let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;
        let calls = parsed["tool_calls"].as_array()?;

        if calls.is_empty() {
            return None;
        }

        let result: Vec<Value> = calls.iter().map(|c| {
            let name = c["name"].as_str().unwrap_or("").to_string();
            let arguments = self.json_to_value(c["arguments"].clone());
            Value::Map(vec![
                ("name".to_string(), Value::String(name)),
                ("arguments".to_string(), arguments),
            ])
        }).collect();

        Some(result)
    }

    fn call_openai(&self, model: &str, system: &str, prompt: &str, tools: Option<Vec<serde_json::Value>>) -> Result<Value> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .or_else(|_| {
                let env_path = std::path::Path::new(".env");
                if env_path.exists() {
                    std::fs::read_to_string(env_path).ok().and_then(|content| {
                        content.lines().find_map(|line| {
                            let line = line.trim();
                            line.strip_prefix("OPENAI_API_KEY=")
                                .map(|val| val.trim_matches('"').trim_matches('\'').to_string())
                        })
                    }).ok_or_else(|| std::env::VarError::NotPresent)
                } else { Err(std::env::VarError::NotPresent) }
            })
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set. Set it in env or .env file."))?;

        log::info!("Calling OpenAI: model={}, tools={}", model, tools.as_ref().map(|t| t.len()).unwrap_or(0));
        let call_start = std::time::Instant::now();

        let mut messages = Vec::new();
        if !system.is_empty() {
            messages.push(serde_json::json!({"role": "system", "content": system}));
        }
        messages.push(serde_json::json!({"role": "user", "content": prompt}));

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages
        });

        if let Some(ref tool_defs) = tools {
            body["tools"] = serde_json::json!(tool_defs);
            body["tool_choice"] = serde_json::json!("auto");
        }

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;

        let resp = client.post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| anyhow::anyhow!("OpenAI error: {}", e))?;

        let json: serde_json::Value = resp.json()
            .map_err(|e| anyhow::anyhow!("OpenAI JSON error: {}", e))?;

        if let Some(err) = json.get("error") {
            bail!("OpenAI API error: {}", err);
        }

        let choice = &json["choices"][0]["message"];
        let content = choice["content"].as_str().unwrap_or("").to_string();

        // Check for tool calls
        if let Some(tool_calls_arr) = choice.get("tool_calls").and_then(|v| v.as_array()) {
            if !tool_calls_arr.is_empty() {
                let tc: Vec<Value> = tool_calls_arr.iter().map(|c| {
                    let func = &c["function"];
                    let name = func["name"].as_str().unwrap_or("").to_string();
                    let args_str = func["arguments"].as_str().unwrap_or("{}");
                    let arguments = serde_json::from_str::<serde_json::Value>(args_str)
                        .map(|v| self.json_to_value(v))
                        .unwrap_or(Value::Map(vec![]));
                    Value::Map(vec![
                        ("name".to_string(), Value::String(name)),
                        ("arguments".to_string(), arguments),
                    ])
                }).collect();

                let latency = call_start.elapsed().as_millis() as u64;
                self.trace_llm(model, "openai", latency, prompt, system, &content, true);
                return Ok(Value::Map(vec![
                    ("content".to_string(), Value::String(content)),
                    ("tool_calls".to_string(), Value::List(tc)),
                    ("has_tool_calls".to_string(), Value::Bool(true)),
                ]));
            }
        }

        let latency = call_start.elapsed().as_millis() as u64;
        self.trace_llm(model, "openai", latency, prompt, system, &content, false);

        if tools.is_some() {
            Ok(Value::Map(vec![
                ("content".to_string(), Value::String(content)),
                ("has_tool_calls".to_string(), Value::Bool(false)),
            ]))
        } else {
            Ok(Value::String(content))
        }
    }

    fn call_anthropic(&self, model: &str, system: &str, prompt: &str, tools: Option<Vec<serde_json::Value>>) -> Result<Value> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .or_else(|_| {
                // Try .env file
                let env_path = std::path::Path::new(".env");
                if env_path.exists() {
                    std::fs::read_to_string(env_path).ok().and_then(|content| {
                        content.lines().find_map(|line| {
                            let line = line.trim();
                            line.strip_prefix("ANTHROPIC_API_KEY=")
                                .map(|val| val.trim_matches('"').trim_matches('\'').to_string())
                        })
                    }).ok_or_else(|| std::env::VarError::NotPresent)
                } else { Err(std::env::VarError::NotPresent) }
            })
            .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set. Set it in env or .env file."))?;

        log::info!("Calling Anthropic: model={}, system={:?}, tools={}", model, system, tools.as_ref().map(|t| t.len()).unwrap_or(0));

        let messages = vec![serde_json::json!({"role": "user", "content": prompt})];

        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": 4096,
            "messages": messages,
            "stream": false
        });

        if !system.is_empty() {
            body["system"] = serde_json::Value::String(system.to_string());
        }

        if let Some(ref tool_defs) = tools {
            // Convert from OpenAI format to Anthropic format
            let anthropic_tools: Vec<serde_json::Value> = tool_defs.iter().map(|t| {
                serde_json::json!({
                    "name": t["function"]["name"],
                    "description": t["function"]["description"],
                    "input_schema": t["function"]["parameters"]
                })
            }).collect();
            body["tools"] = serde_json::json!(anthropic_tools);
        }

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;

        let is_oauth = api_key.starts_with("sk-ant-oat");
        let mut req = client.post("https://api.anthropic.com/v1/messages")
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json");

        if is_oauth {
            req = req.header("Authorization", format!("Bearer {}", api_key))
                .header("anthropic-beta", "claude-code-20250219,oauth-2025-04-20");
        } else {
            req = req.header("x-api-key", &api_key);
        }

        let resp = req.json(&body)
            .send()
            .map_err(|e| anyhow::anyhow!("Anthropic error: {}", e))?;

        let json: serde_json::Value = resp.json()
            .map_err(|e| anyhow::anyhow!("Anthropic JSON error: {}", e))?;

        if let Some(err) = json.get("error") {
            bail!("Anthropic API error: {}", err);
        }

        // Parse response
        let content_blocks = json["content"].as_array()
            .ok_or_else(|| anyhow::anyhow!("Anthropic: no content in response"))?;

        let mut text_content = std::string::String::new();
        let mut tool_calls = Vec::new();

        for block in content_blocks {
            match block["type"].as_str() {
                Some("text") => {
                    text_content.push_str(block["text"].as_str().unwrap_or(""));
                }
                Some("tool_use") => {
                    let name = block["name"].as_str().unwrap_or("").to_string();
                    let arguments = self.json_to_value(block["input"].clone());
                    tool_calls.push(Value::Map(vec![
                        ("name".to_string(), Value::String(name)),
                        ("arguments".to_string(), arguments),
                    ]));
                }
                _ => {}
            }
        }

        if !tool_calls.is_empty() {
            return Ok(Value::Map(vec![
                ("content".to_string(), Value::String(text_content)),
                ("tool_calls".to_string(), Value::List(tool_calls)),
                ("has_tool_calls".to_string(), Value::Bool(true)),
            ]));
        }

        if tools.is_some() {
            Ok(Value::Map(vec![
                ("content".to_string(), Value::String(text_content)),
                ("has_tool_calls".to_string(), Value::Bool(false)),
            ]))
        } else {
            Ok(Value::String(text_content))
        }
    }

    fn call_ollama(&self, model: &str, system: &str, prompt: &str, tools: Option<Vec<serde_json::Value>>) -> Result<Value> {
        log::info!("Calling Ollama: model={}, system={:?}, tools={}", model, system, tools.as_ref().map(|t| t.len()).unwrap_or(0));
        let call_start = std::time::Instant::now();

        let mut messages = Vec::new();
        if !system.is_empty() {
            messages.push(serde_json::json!({"role": "system", "content": system}));
        }
        messages.push(serde_json::json!({"role": "user", "content": prompt}));

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false
        });

        if let Some(ref tool_defs) = tools {
            body["tools"] = serde_json::json!(tool_defs);
        }

        let client = reqwest::blocking::Client::new();
        let resp = client.post("http://localhost:11434/api/chat")
            .json(&body)
            .send()
            .map_err(|e| anyhow::anyhow!("Ollama error: {}", e))?;

        let json: serde_json::Value = resp.json()
            .map_err(|e| anyhow::anyhow!("Ollama JSON error: {}", e))?;

        let message = &json["message"];
        let content = message["content"].as_str().unwrap_or("").to_string();

        // Check for tool calls
        if let Some(tool_calls) = message.get("tool_calls") {
            if let Some(calls) = tool_calls.as_array() {
                if !calls.is_empty() {
                    let tc: Vec<Value> = calls.iter().map(|c| {
                        let func = &c["function"];
                        let name = func["name"].as_str().unwrap_or("").to_string();
                        let arguments = self.json_to_value(func["arguments"].clone());
                        Value::Map(vec![
                            ("name".to_string(), Value::String(name)),
                            ("arguments".to_string(), arguments),
                        ])
                    }).collect();

                    let latency = call_start.elapsed().as_millis() as u64;
                    self.trace_llm(model, "ollama", latency, prompt, system, &content, true);
                    return Ok(Value::Map(vec![
                        ("content".to_string(), Value::String(content)),
                        ("tool_calls".to_string(), Value::List(tc)),
                        ("has_tool_calls".to_string(), Value::Bool(true)),
                    ]));
                }
            }
        }

        let latency = call_start.elapsed().as_millis() as u64;
        self.trace_llm(model, "ollama", latency, prompt, system, &content, false);
        // No tool calls — return simple string or structured map
        if tools.is_some() {
            Ok(Value::Map(vec![
                ("content".to_string(), Value::String(content)),
                ("has_tool_calls".to_string(), Value::Bool(false)),
            ]))
        } else {
            Ok(Value::String(content))
        }
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

            // String repeat
            (Value::String(s), BinOp::Mul, Value::Int(n)) => {
                if *n < 0 { bail!("cannot repeat string a negative number of times"); }
                Ok(Value::String(s.repeat(*n as usize)))
            }
            (Value::Int(n), BinOp::Mul, Value::String(s)) => {
                if *n < 0 { bail!("cannot repeat string a negative number of times"); }
                Ok(Value::String(s.repeat(*n as usize)))
            }

            // Int arithmetic
            (Value::Int(a), BinOp::Add, Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Int(a), BinOp::Sub, Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Int(a), BinOp::Mul, Value::Int(b)) => Ok(Value::Int(a * b)),
            (Value::Int(a), BinOp::Div, Value::Int(b)) => {
                if *b == 0 { bail!("division by zero"); }
                Ok(Value::Int(a / b))
            }

            // Float arithmetic
            (Value::Float(a), BinOp::Add, Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Float(a), BinOp::Sub, Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Float(a), BinOp::Mul, Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Float(a), BinOp::Div, Value::Float(b)) => {
                if *b == 0.0 { bail!("division by zero"); }
                Ok(Value::Float(a / b))
            }

            // Mixed Int/Float arithmetic (promote to Float)
            (Value::Int(a), BinOp::Add, Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), BinOp::Add, Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::Int(a), BinOp::Sub, Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
            (Value::Float(a), BinOp::Sub, Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
            (Value::Int(a), BinOp::Mul, Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
            (Value::Float(a), BinOp::Mul, Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
            (Value::Int(a), BinOp::Div, Value::Float(b)) => {
                if *b == 0.0 { bail!("division by zero"); }
                Ok(Value::Float(*a as f64 / b))
            }
            (Value::Float(a), BinOp::Div, Value::Int(b)) => {
                if *b == 0 { bail!("division by zero"); }
                Ok(Value::Float(a / *b as f64))
            }

            // Comparisons
            (Value::Int(a), BinOp::Eq, Value::Int(b)) => Ok(Value::Bool(a == b)),
            (Value::Int(a), BinOp::NotEq, Value::Int(b)) => Ok(Value::Bool(a != b)),
            (Value::Int(a), BinOp::Lt, Value::Int(b)) => Ok(Value::Bool(a < b)),
            (Value::Int(a), BinOp::Gt, Value::Int(b)) => Ok(Value::Bool(a > b)),
            (Value::Int(a), BinOp::LtEq, Value::Int(b)) => Ok(Value::Bool(a <= b)),
            (Value::Int(a), BinOp::GtEq, Value::Int(b)) => Ok(Value::Bool(a >= b)),

            (Value::Float(a), BinOp::Eq, Value::Float(b)) => Ok(Value::Bool(a == b)),
            (Value::Float(a), BinOp::NotEq, Value::Float(b)) => Ok(Value::Bool(a != b)),
            (Value::Float(a), BinOp::Lt, Value::Float(b)) => Ok(Value::Bool(a < b)),
            (Value::Float(a), BinOp::Gt, Value::Float(b)) => Ok(Value::Bool(a > b)),
            (Value::Float(a), BinOp::LtEq, Value::Float(b)) => Ok(Value::Bool(a <= b)),
            (Value::Float(a), BinOp::GtEq, Value::Float(b)) => Ok(Value::Bool(a >= b)),

            (Value::String(a), BinOp::Eq, Value::String(b)) => Ok(Value::Bool(a == b)),
            (Value::String(a), BinOp::NotEq, Value::String(b)) => Ok(Value::Bool(a != b)),

            // Boolean comparison
            (Value::Bool(a), BinOp::Eq, Value::Bool(b)) => Ok(Value::Bool(a == b)),
            (Value::Bool(a), BinOp::NotEq, Value::Bool(b)) => Ok(Value::Bool(a != b)),

            // None comparison
            (Value::None, BinOp::Eq, Value::None) => Ok(Value::Bool(true)),
            (Value::None, BinOp::Eq, _) => Ok(Value::Bool(false)),
            (_, BinOp::Eq, Value::None) => Ok(Value::Bool(false)),
            (Value::None, BinOp::NotEq, Value::None) => Ok(Value::Bool(false)),
            (Value::None, BinOp::NotEq, _) => Ok(Value::Bool(true)),
            (_, BinOp::NotEq, Value::None) => Ok(Value::Bool(true)),

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
