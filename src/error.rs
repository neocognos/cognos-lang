#![allow(dead_code)]
/// Cognos error system.
/// Every error has a code, location, message, and optional hint.

use crate::token::Token;
use std::fmt;

#[derive(Debug)]
pub struct CognosError {
    pub kind: ErrorKind,
    pub line: usize,
    pub message: String,
    pub hint: Option<String>,
}

#[derive(Debug)]
pub enum ErrorKind {
    Parse,
    Runtime,
    Type,
}

impl fmt::Display for CognosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "line {}: {}", self.line, self.message)?;
        } else {
            write!(f, "{}", self.message)?;
        }
        if let Some(hint) = &self.hint {
            write!(f, "\n  hint: {}", hint)?;
        }
        Ok(())
    }
}

impl std::error::Error for CognosError {}

impl CognosError {
    pub fn parse(line: usize, message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Parse, line, message: message.into(), hint: None }
    }

    pub fn parse_hint(line: usize, message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Parse, line, message: message.into(), hint: Some(hint.into()) }
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Runtime, line: 0, message: message.into(), hint: None }
    }

    pub fn runtime_hint(message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Runtime, line: 0, message: message.into(), hint: Some(hint.into()) }
    }

    pub fn type_error(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Type, line: 0, message: message.into(), hint: None }
    }

    pub fn type_hint(message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Type, line: 0, message: message.into(), hint: Some(hint.into()) }
    }
}

/// Generate a context-aware parse error for an unexpected token.
/// This is the single place that maps every token to a helpful message.
pub fn unexpected_token(line: usize, got: &Token, context: &str) -> CognosError {
    let (msg, hint) = match got {
        // Keywords in wrong position
        Token::Flow => (
            "found 'flow' where an expression was expected".into(),
            Some("'flow' defines a new flow — it can't be used inside an expression".into()),
        ),
        Token::If => (
            "found 'if' where an expression was expected".into(),
            Some("'if' is a statement — use it on its own line, not inside an expression".into()),
        ),
        Token::Else | Token::Elif => (
            format!("found {} without a matching 'if'", got),
            Some("'else'/'elif' must follow an 'if' block".into()),
        ),
        Token::Loop => (
            "found 'loop' where an expression was expected".into(),
            Some("'loop' is a statement — usage: loop max=N: ...".into()),
        ),
        Token::For => (
            "found 'for' where an expression was expected".into(),
            Some("'for' is a statement — usage: for item in collection: ...".into()),
        ),
        Token::Break => (
            "'break' outside of a loop".into(),
            Some("'break' can only be used inside a loop body".into()),
        ),
        Token::Continue => (
            "'continue' outside of a loop".into(),
            Some("'continue' can only be used inside a loop body".into()),
        ),
        Token::Return => (
            "found 'return' where an expression was expected".into(),
            Some("'return' is a statement — usage: return value".into()),
        ),
        Token::Emit => (
            "found 'emit' where an expression was expected".into(),
            Some("'emit' is a statement — usage: emit(value)".into()),
        ),
        Token::Pass => (
            "found 'pass' where an expression was expected".into(),
            None,
        ),
        Token::Type => (
            "found 'type' where an expression was expected".into(),
            Some("type definitions are not yet supported".into()),
        ),
        Token::Try | Token::Catch => (
            format!("found {} where an expression was expected", got),
            Some("try/catch is not yet supported".into()),
        ),
        Token::Parallel => (
            "found 'parallel' where an expression was expected".into(),
            Some("parallel execution is not yet supported".into()),
        ),

        // Operators in wrong position
        Token::Plus => ("unexpected '+' — missing left operand".into(), None),
        Token::Minus => ("unexpected '-' — missing left operand".into(), None),
        Token::Star => ("unexpected '*' — missing left operand".into(), None),
        Token::Slash => ("unexpected '/' — missing left operand".into(), None),
        Token::Eq => (
            "unexpected '=' — not a valid expression".into(),
            Some("did you mean '==' for comparison?".into()),
        ),
        Token::EqEq => ("unexpected '==' — missing left operand".into(), None),
        Token::NotEq => ("unexpected '!=' — missing left operand".into(), None),
        Token::Lt | Token::Gt | Token::LtEq | Token::GtEq => (
            format!("unexpected {} — missing left operand", got),
            None,
        ),
        Token::Dot => (
            "unexpected '.' — missing object before field access".into(),
            Some("usage: object.field".into()),
        ),
        Token::Arrow => (
            "unexpected '->' outside of flow signature".into(),
            None,
        ),
        Token::FatArrow => (
            "unexpected '=>' — lambda expressions are not yet supported".into(),
            None,
        ),

        // Delimiters
        Token::RParen => (
            "unexpected ')' — no matching '('".into(),
            Some("check for missing arguments or extra closing parenthesis".into()),
        ),
        Token::RBracket => (
            "unexpected ']' — no matching '['".into(),
            None,
        ),
        Token::RBrace => (
            "unexpected '}}' — no matching '{{'".into(),
            None,
        ),
        Token::Comma => (
            "unexpected ',' — not inside a list or function call".into(),
            None,
        ),
        Token::Colon => (
            "unexpected ':' — not after if/loop/flow".into(),
            None,
        ),

        // Structure
        Token::Indent => (
            "unexpected indentation".into(),
            Some("check your indentation — use consistent 4-space indents".into()),
        ),
        Token::Dedent => (
            "unexpected dedent".into(),
            None,
        ),
        Token::Newline => (
            "unexpected end of line — expression is incomplete".into(),
            Some("did you forget to finish the expression?".into()),
        ),
        Token::Eof => (
            "unexpected end of file — expression is incomplete".into(),
            None,
        ),

        // Values — these shouldn't be "unexpected" in expression context
        Token::Ident(name) => (
            format!("unexpected '{}' {}", name, context),
            None,
        ),
        Token::StringLit(_) | Token::FStringLit(_) => (
            format!("unexpected string literal {}", context),
            None,
        ),
        Token::IntLit(_) | Token::FloatLit(_) => (
            format!("unexpected number {}", context),
            None,
        ),
        Token::True | Token::False => (
            format!("unexpected {} {}", got, context),
            None,
        ),

        // Keywords that could be confused
        Token::And => ("unexpected 'and' — missing left operand".into(), None),
        Token::Or => ("unexpected 'or' — missing left operand".into(), None),
        Token::Not => ("unexpected 'not' here".into(), None),
        Token::In => (
            "unexpected 'in' — not inside a for loop".into(),
            Some("usage: for item in collection: ...".into()),
        ),
        Token::Let => (
            "'let' is not needed — just write: name = value".into(),
            None,
        ),
        Token::LParen | Token::LBracket | Token::LBrace => (
            format!("unexpected {} {}", got, context),
            None,
        ),
    };

    CognosError {
        kind: ErrorKind::Parse,
        line,
        message: msg,
        hint,
    }
}
