#![allow(dead_code)]
/// Abstract Syntax Tree for Cognos programs.

#[derive(Debug, Clone)]
pub struct Program {
    pub imports: Vec<String>,
    pub types: Vec<TypeDef>,
    pub flows: Vec<FlowDef>,
}

#[derive(Debug, Clone)]
pub enum TypeDef {
    Struct {
        name: String,
        fields: Vec<TypeField>,
    },
    Enum {
        name: String,
        variants: Vec<String>,
    },
}

impl TypeDef {
    pub fn name(&self) -> &str {
        match self {
            TypeDef::Struct { name, .. } => name,
            TypeDef::Enum { name, .. } => name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeField {
    pub name: String,
    pub ty: TypeExpr,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub struct FlowDef {
    pub name: String,
    pub description: Option<String>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Named(String),                        // Text, Bool, Int
    Generic(String, Vec<TypeExpr>),       // List[Text], Map[Text, Int]
    Struct(Vec<(String, TypeExpr)>),      // { field: Type, ... }
}

#[derive(Debug, Clone)]
pub enum Stmt {
    /// `name = expr`
    Assign { name: String, expr: Expr },
    /// `emit(expr)`
    Emit { value: Expr },
    /// `return expr`
    Return { value: Expr },
    /// `break`
    Break,
    /// `continue`
    Continue,
    /// `pass` (no-op)
    Pass,
    /// `if cond: body elif cond: body else: body`
    If {
        condition: Expr,
        body: Vec<Stmt>,
        elifs: Vec<(Expr, Vec<Stmt>)>,
        else_body: Vec<Stmt>,
    },
    /// `loop max=N: body`
    Loop {
        max: Option<u32>,
        body: Vec<Stmt>,
    },
    /// `for item in collection: body`
    For {
        var: String,
        value_var: Option<String>,  // for k, v in map
        iterable: Expr,
        body: Vec<Stmt>,
    },
    /// `try: body catch err: handler`
    TryCatch {
        body: Vec<Stmt>,
        error_var: Option<String>,
        catch_body: Vec<Stmt>,
    },
    /// `parallel: body` — run all statements concurrently
    Parallel { body: Vec<Stmt> },
    /// Bare expression (function call as statement)
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    /// Variable reference
    Ident(String),
    /// String literal
    StringLit(String),
    /// Integer literal
    IntLit(i64),
    /// Float literal
    FloatLit(f64),
    /// Boolean literal
    BoolLit(bool),
    /// Function call: name(args, key=val, ...)
    Call {
        name: String,
        args: Vec<Expr>,
        kwargs: Vec<(String, Expr)>,
    },
    /// Async expression: async func(args)
    Async(Box<Expr>),
    /// Field access: expr.field
    Field { object: Box<Expr>, field: String },
    /// Index access: expr[expr]
    Index { object: Box<Expr>, index: Box<Expr> },
    /// Slice access: expr[start:end]
    Slice { object: Box<Expr>, start: Option<Box<Expr>>, end: Option<Box<Expr>> },
    /// Method call: expr.method(args)
    MethodCall { object: Box<Expr>, method: String, args: Vec<Expr> },
    /// Binary op: left op right
    BinOp { left: Box<Expr>, op: BinOp, right: Box<Expr> },
    /// Unary op: not expr
    UnaryOp { op: UnaryOp, operand: Box<Expr> },
    /// List literal: [a, b, c]
    List(Vec<Expr>),
    /// Map literal: {"key": value, ...}
    Map(Vec<(String, Expr)>),
    /// F-string: f"hello {name}, you have {count} items"
    /// Parts alternate between literal strings and expressions
    FString(Vec<FStringPart>),
}

#[derive(Debug, Clone)]
pub enum FStringPart {
    Literal(String),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,    // +
    Sub,    // -
    Mul,    // *
    Div,    // /
    Eq,     // ==
    NotEq,  // !=
    Lt,     // <
    Gt,     // >
    LtEq,   // <=
    GtEq,   // >=
    And,    // and
    Or,     // or
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
}
