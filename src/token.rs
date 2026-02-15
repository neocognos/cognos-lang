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
    Branch,
    Async,
    Await,
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
    None_,
    Pass,
    Select,

    // Identifiers and literals
    Ident(String),
    StringLit(String),
    FStringLit(String),  // f"..." — raw content, parsed later
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
    Percent,    // %
    Dot,        // .
    Comma,      // ,
    Colon,      // :
    Arrow,      // ->
    FatArrow,   // =>
    Question,   // ?
    Pipe,       // |

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

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Flow => write!(f, "'flow'"),
            Token::Let => write!(f, "'let'"),
            Token::If => write!(f, "'if'"),
            Token::Else => write!(f, "'else'"),
            Token::Elif => write!(f, "'elif'"),
            Token::Loop => write!(f, "'loop'"),
            Token::Break => write!(f, "'break'"),
            Token::Continue => write!(f, "'continue'"),
            Token::Return => write!(f, "'return'"),
            Token::Emit => write!(f, "'emit'"),
            Token::Parallel => write!(f, "'parallel'"),
            Token::Branch => write!(f, "'branch'"),
            Token::Async => write!(f, "'async'"),
            Token::Await => write!(f, "'await'"),
            Token::For => write!(f, "'for'"),
            Token::In => write!(f, "'in'"),
            Token::Try => write!(f, "'try'"),
            Token::Catch => write!(f, "'catch'"),
            Token::Type => write!(f, "'type'"),
            Token::And => write!(f, "'and'"),
            Token::Or => write!(f, "'or'"),
            Token::Not => write!(f, "'not'"),
            Token::True => write!(f, "'true'"),
            Token::False => write!(f, "'false'"),
            Token::None_ => write!(f, "'none'"),
            Token::Pass => write!(f, "'pass'"),
            Token::Select => write!(f, "'select'"),
            Token::Ident(s) => write!(f, "'{}'", s),
            Token::StringLit(s) => write!(f, "\"{}\"", s),
            Token::FStringLit(s) => write!(f, "f\"{}\"", s),
            Token::IntLit(n) => write!(f, "{}", n),
            Token::FloatLit(n) => write!(f, "{}", n),
            Token::Eq => write!(f, "'='"),
            Token::EqEq => write!(f, "'=='"),
            Token::NotEq => write!(f, "'!='"),
            Token::Lt => write!(f, "'<'"),
            Token::Gt => write!(f, "'>'"),
            Token::LtEq => write!(f, "'<='"),
            Token::GtEq => write!(f, "'>='"),
            Token::Plus => write!(f, "'+'"),
            Token::Minus => write!(f, "'-'"),
            Token::Star => write!(f, "'*'"),
            Token::Slash => write!(f, "'/'"),
            Token::Percent => write!(f, "'%'"),
            Token::Dot => write!(f, "'.'"),
            Token::Comma => write!(f, "','"),
            Token::Colon => write!(f, "':'"),
            Token::Arrow => write!(f, "'->'"),
            Token::FatArrow => write!(f, "'=>'"),
            Token::Question => write!(f, "'?'"),
            Token::Pipe => write!(f, "'|'"),
            Token::LParen => write!(f, "'('"),
            Token::RParen => write!(f, "')'"),
            Token::LBracket => write!(f, "'['"),
            Token::RBracket => write!(f, "']'"),
            Token::LBrace => write!(f, "'{{'"),
            Token::RBrace => write!(f, "'}}'"),
            Token::Newline => write!(f, "newline"),
            Token::Indent => write!(f, "indent"),
            Token::Dedent => write!(f, "dedent"),
            Token::Eof => write!(f, "end of file"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Spanned {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}
