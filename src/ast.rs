/// Abstract Syntax Tree for Cognos programs.

#[derive(Debug, Clone)]
pub struct Program {
    pub flows: Vec<FlowDef>,
}

#[derive(Debug, Clone)]
pub struct FlowDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
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
    /// Field access: expr.field
    Field { object: Box<Expr>, field: String },
    /// Binary op: left op right
    BinOp { left: Box<Expr>, op: BinOp, right: Box<Expr> },
    /// Unary op: not expr
    UnaryOp { op: UnaryOp, operand: Box<Expr> },
    /// List literal: [a, b, c]
    List(Vec<Expr>),
    /// Map literal: {"key": value, ...}
    Map(Vec<(String, Expr)>),
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
