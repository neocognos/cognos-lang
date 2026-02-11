/// Indentation-aware lexer for Cognos.
/// Produces Indent/Dedent tokens based on leading whitespace (Python-style).

use crate::token::{Token, Spanned};

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    indent_stack: Vec<usize>,
    pending: Vec<Spanned>,
    at_line_start: bool,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            indent_stack: vec![0],
            pending: Vec::new(),
            at_line_start: true,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Spanned> {
        let mut tokens = Vec::new();

        loop {
            // Drain pending tokens (dedents)
            while let Some(t) = self.pending.pop() {
                tokens.push(t);
            }

            if self.pos >= self.source.len() {
                // Emit remaining dedents
                while self.indent_stack.len() > 1 {
                    self.indent_stack.pop();
                    tokens.push(self.spanned(Token::Dedent));
                }
                tokens.push(self.spanned(Token::Eof));
                break;
            }

            // Handle line start — check indentation
            if self.at_line_start {
                self.handle_indentation(&mut tokens);
                self.at_line_start = false;
                continue;
            }

            let ch = self.source[self.pos];

            // Skip inline whitespace
            if ch == ' ' || ch == '\t' {
                self.advance();
                continue;
            }

            // Comments
            if ch == '#' {
                while self.pos < self.source.len() && self.source[self.pos] != '\n' {
                    self.advance();
                }
                continue;
            }

            // Newline
            if ch == '\n' {
                tokens.push(self.spanned(Token::Newline));
                self.advance();
                self.at_line_start = true;
                continue;
            }

            // String literals
            if ch == '"' {
                tokens.push(self.read_string());
                continue;
            }

            // Numbers
            if ch.is_ascii_digit() {
                tokens.push(self.read_number());
                continue;
            }

            // Identifiers and keywords
            if ch.is_alphabetic() || ch == '_' {
                tokens.push(self.read_ident());
                continue;
            }

            // Two-char operators
            if self.pos + 1 < self.source.len() {
                let next = self.source[self.pos + 1];
                let two = match (ch, next) {
                    ('=', '=') => Some(Token::EqEq),
                    ('!', '=') => Some(Token::NotEq),
                    ('<', '=') => Some(Token::LtEq),
                    ('>', '=') => Some(Token::GtEq),
                    ('-', '>') => Some(Token::Arrow),
                    ('=', '>') => Some(Token::FatArrow),
                    _ => None,
                };
                if let Some(tok) = two {
                    let s = self.spanned(tok);
                    self.advance();
                    self.advance();
                    tokens.push(s);
                    continue;
                }
            }

            // Single-char operators
            let tok = match ch {
                '=' => Token::Eq,
                '+' => Token::Plus,
                '-' => Token::Minus,
                '*' => Token::Star,
                '/' => Token::Slash,
                '.' => Token::Dot,
                ',' => Token::Comma,
                ':' => Token::Colon,
                '<' => Token::Lt,
                '>' => Token::Gt,
                '(' => Token::LParen,
                ')' => Token::RParen,
                '[' => Token::LBracket,
                ']' => Token::RBracket,
                '{' => Token::LBrace,
                '}' => Token::RBrace,
                _ => {
                    // Skip unknown chars
                    self.advance();
                    continue;
                }
            };
            tokens.push(self.spanned(tok));
            self.advance();
        }

        tokens
    }

    fn handle_indentation(&mut self, tokens: &mut Vec<Spanned>) {
        // Skip blank lines
        let start = self.pos;
        let mut spaces = 0;
        while self.pos < self.source.len() && self.source[self.pos] == ' ' {
            spaces += 1;
            self.pos += 1;
            self.col += 1;
        }
        // Blank line or comment-only line — skip
        if self.pos >= self.source.len() || self.source[self.pos] == '\n' || self.source[self.pos] == '#' {
            return;
        }

        let current = *self.indent_stack.last().unwrap();
        if spaces > current {
            self.indent_stack.push(spaces);
            tokens.push(self.spanned(Token::Indent));
        } else if spaces < current {
            while self.indent_stack.len() > 1 && *self.indent_stack.last().unwrap() > spaces {
                self.indent_stack.pop();
                tokens.push(self.spanned(Token::Dedent));
            }
        }
    }

    fn read_string(&mut self) -> Spanned {
        let line = self.line;
        let col = self.col;
        self.advance(); // skip opening "
        let mut s = String::new();
        while self.pos < self.source.len() && self.source[self.pos] != '"' {
            if self.source[self.pos] == '\\' && self.pos + 1 < self.source.len() {
                self.advance();
                match self.source[self.pos] {
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    '"' => s.push('"'),
                    '\\' => s.push('\\'),
                    c => { s.push('\\'); s.push(c); }
                }
            } else {
                s.push(self.source[self.pos]);
            }
            self.advance();
        }
        if self.pos < self.source.len() {
            self.advance(); // skip closing "
        }
        Spanned { token: Token::StringLit(s), line, col }
    }

    fn read_number(&mut self) -> Spanned {
        let line = self.line;
        let col = self.col;
        let mut s = String::new();
        let mut is_float = false;
        while self.pos < self.source.len() && (self.source[self.pos].is_ascii_digit() || self.source[self.pos] == '.') {
            if self.source[self.pos] == '.' {
                is_float = true;
            }
            s.push(self.source[self.pos]);
            self.advance();
        }
        let token = if is_float {
            Token::FloatLit(s.parse().unwrap_or(0.0))
        } else {
            Token::IntLit(s.parse().unwrap_or(0))
        };
        Spanned { token, line, col }
    }

    fn read_ident(&mut self) -> Spanned {
        let line = self.line;
        let col = self.col;
        let mut s = String::new();
        while self.pos < self.source.len() && (self.source[self.pos].is_alphanumeric() || self.source[self.pos] == '_') {
            s.push(self.source[self.pos]);
            self.advance();
        }
        let token = match s.as_str() {
            "flow" => Token::Flow,
            "let" => Token::Let,
            "if" => Token::If,
            "else" => Token::Else,
            "elif" => Token::Elif,
            "loop" => Token::Loop,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "return" => Token::Return,
            "emit" => Token::Emit,
            "parallel" => Token::Parallel,
            "for" => Token::For,
            "in" => Token::In,
            "try" => Token::Try,
            "catch" => Token::Catch,
            "type" => Token::Type,
            "and" => Token::And,
            "or" => Token::Or,
            "not" => Token::Not,
            "true" => Token::True,
            "false" => Token::False,
            "pass" => Token::Pass,
            _ => Token::Ident(s),
        };
        Spanned { token, line, col }
    }

    fn advance(&mut self) {
        if self.pos < self.source.len() {
            if self.source[self.pos] == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }
    }

    fn spanned(&self, token: Token) -> Spanned {
        Spanned { token, line: self.line, col: self.col }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world() {
        let source = r#"flow hello:
    input = receive(Text)
    emit(input)
"#;
        let mut lexer = Lexer::new(source);
        let tokens: Vec<Token> = lexer.tokenize().into_iter().map(|s| s.token).collect();
        assert_eq!(tokens, vec![
            Token::Flow,
            Token::Ident("hello".into()),
            Token::Colon,
            Token::Newline,
            Token::Indent,
            Token::Ident("input".into()),
            Token::Eq,
            Token::Ident("receive".into()),
            Token::LParen,
            Token::Ident("Text".into()),
            Token::RParen,
            Token::Newline,
            Token::Emit,
            Token::LParen,
            Token::Ident("input".into()),
            Token::RParen,
            Token::Newline,
            Token::Dedent,
            Token::Eof,
        ]);
    }

    #[test]
    fn test_string_and_numbers() {
        let source = r#"x = "hello world"
y = 42
z = 3.14
"#;
        let mut lexer = Lexer::new(source);
        let tokens: Vec<Token> = lexer.tokenize().into_iter().map(|s| s.token).collect();
        assert!(tokens.contains(&Token::StringLit("hello world".into())));
        assert!(tokens.contains(&Token::IntLit(42)));
        assert!(tokens.contains(&Token::FloatLit(3.14)));
    }

    #[test]
    fn test_operators() {
        let source = "a == b != c -> d => e";
        let mut lexer = Lexer::new(source);
        let tokens: Vec<Token> = lexer.tokenize().into_iter().map(|s| s.token).collect();
        assert!(tokens.contains(&Token::EqEq));
        assert!(tokens.contains(&Token::NotEq));
        assert!(tokens.contains(&Token::Arrow));
        assert!(tokens.contains(&Token::FatArrow));
    }

    #[test]
    fn test_nested_indent() {
        let source = "flow f:\n    if true:\n        x = 1\n    y = 2\n";
        let mut lexer = Lexer::new(source);
        let tokens: Vec<Token> = lexer.tokenize().into_iter().map(|s| s.token).collect();
        let indent_count = tokens.iter().filter(|t| **t == Token::Indent).count();
        let dedent_count = tokens.iter().filter(|t| **t == Token::Dedent).count();
        assert_eq!(indent_count, dedent_count, "indents and dedents must balance");
    }
}
