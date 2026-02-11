/// Tree-walking interpreter for Cognos.
/// Executes a parsed AST directly — no kernel needed.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use crate::ast::*;
use anyhow::{bail, Result};

#[derive(Debug, Clone)]
pub enum Value {
    Text(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<Value>),
    None,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Text(s) => write!(f, "{}", s),
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
            Value::None => write!(f, ""),
        }
    }
}

impl Value {
    fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Text(s) => !s.is_empty(),
            Value::Int(n) => *n != 0,
            Value::Float(n) => *n != 0.0,
            Value::List(items) => !items.is_empty(),
            Value::None => false,
        }
    }
}

enum ControlFlow {
    Normal,
    Break,
    Continue,
    Return(Value),
}

pub struct Interpreter {
    vars: HashMap<String, Value>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self { vars: HashMap::new() }
    }

    pub fn run(&mut self, program: &Program) -> Result<()> {
        // Find "main" flow, or use the first one
        let flow = program.flows.iter()
            .find(|f| f.name == "main")
            .or_else(|| program.flows.first());

        match flow {
            Some(f) => {
                self.run_block(&f.body)?;
                Ok(())
            }
            None => {
                // Empty program — do nothing
                Ok(())
            }
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
            Expr::StringLit(s) => Ok(Value::Text(s.clone())),
            Expr::IntLit(n) => Ok(Value::Int(*n)),
            Expr::FloatLit(n) => Ok(Value::Float(*n)),
            Expr::BoolLit(b) => Ok(Value::Bool(*b)),

            Expr::Ident(name) => {
                match self.vars.get(name) {
                    Some(v) => Ok(v.clone()),
                    None => bail!("undefined variable: {}", name),
                }
            }

            Expr::List(items) => {
                let vals: Result<Vec<Value>> = items.iter().map(|i| self.eval(i)).collect();
                Ok(Value::List(vals?))
            }

            Expr::Call { name, args, kwargs: _ } => {
                self.call_builtin(name, args)
            }

            Expr::Field { object, field } => {
                let val = self.eval(object)?;
                match (&val, field.as_str()) {
                    (Value::Text(s), "length") => Ok(Value::Int(s.len() as i64)),
                    (Value::List(l), "length") => Ok(Value::Int(l.len() as i64)),
                    _ => bail!("cannot access field '{}' on {:?}", field, val),
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

    fn call_builtin(&mut self, name: &str, args: &[Expr]) -> Result<Value> {
        match name {
            "receive" => {
                // Read a line from stdin
                print!("> ");
                io::stdout().flush()?;
                let mut line = String::new();
                io::stdin().lock().read_line(&mut line)?;
                Ok(Value::Text(line.trim_end().to_string()))
            }
            "print" | "emit" => {
                // emit is handled at stmt level, but support as expr too
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { print!(" "); }
                    let val = self.eval(arg)?;
                    print!("{}", val);
                }
                println!();
                Ok(Value::None)
            }
            "think" => {
                // Stub: in interpreter mode, think just returns its first arg
                if args.is_empty() {
                    bail!("think() requires at least one argument");
                }
                let val = self.eval(&args[0])?;
                eprintln!("⚠ think() stubbed in interpreter mode — returning input as-is");
                Ok(val)
            }
            "act" => {
                if args.is_empty() {
                    bail!("act() requires at least one argument");
                }
                let val = self.eval(&args[0])?;
                eprintln!("⚠ act() stubbed in interpreter mode — returning input as-is");
                Ok(val)
            }
            "run" => {
                // Execute a shell command
                if args.is_empty() {
                    bail!("run() requires a command string");
                }
                let cmd = self.eval(&args[0])?;
                let output = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(cmd.to_string())
                    .output()?;
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                Ok(Value::Text(stdout.trim_end().to_string()))
            }
            _ => bail!("unknown function: {}()", name),
        }
    }

    fn eval_binop(&self, left: &Value, op: &BinOp, right: &Value) -> Result<Value> {
        match (left, op, right) {
            // String concat
            (Value::Text(a), BinOp::Add, Value::Text(b)) => Ok(Value::Text(format!("{}{}", a, b))),

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

            (Value::Text(a), BinOp::Eq, Value::Text(b)) => Ok(Value::Bool(a == b)),
            (Value::Text(a), BinOp::NotEq, Value::Text(b)) => Ok(Value::Bool(a != b)),

            // Boolean logic
            (Value::Bool(a), BinOp::And, Value::Bool(b)) => Ok(Value::Bool(*a && *b)),
            (Value::Bool(a), BinOp::Or, Value::Bool(b)) => Ok(Value::Bool(*a || *b)),

            // Truthy logic
            (_, BinOp::And, _) => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            (_, BinOp::Or, _) => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),

            _ => bail!("unsupported operation: {:?} {:?} {:?}", left, op, right),
        }
    }
}
