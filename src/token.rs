/// Token types for the Cognos lexer.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Flow,
    Let,
    If,
    Else,
    Elif,
    Loop,
    Break,
    Continue,
    Return,
    Emit,
    Parallel,
    For,
    In,
    Try,
    Catch,
    Type,
    And,
    Or,
    Not,
    True,
    False,

    // Identifiers and literals
    Ident(String),
    StringLit(String),
    IntLit(i64),
    FloatLit(f64),

    // Operators
    Eq,         // =
    EqEq,       // ==
    NotEq,      // !=
    Lt,         // <
    Gt,         // >
    LtEq,       // <=
    GtEq,       // >=
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Dot,        // .
    Comma,      // ,
    Colon,      // :
    Arrow,      // ->
    FatArrow,   // =>

    // Delimiters
    LParen,     // (
    RParen,     // )
    LBracket,   // [
    RBracket,   // ]
    LBrace,     // {
    RBrace,     // }

    // Structure
    Newline,
    Indent,
    Dedent,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Spanned {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}
