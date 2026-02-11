/// Recursive descent parser for Cognos.
/// Parses a token stream into an AST.

use crate::ast::*;
use crate::token::{Token, Spanned};
use crate::error::{CognosError, unexpected_token};
use anyhow::{bail, Result};

/// Parse f-string content into parts: literal text and {expr} interpolations
fn parse_fstring_parts(raw: &str) -> Result<Vec<FStringPart>> {
    let mut parts = Vec::new();
    let mut literal = String::new();
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' {
            // Save accumulated literal
            if !literal.is_empty() {
                parts.push(FStringPart::Literal(literal.clone()));
                literal.clear();
            }
            // Find matching }
            i += 1;
            let mut expr_str = String::new();
            let mut depth = 1;
            while i < chars.len() && depth > 0 {
                if chars[i] == '{' { depth += 1; }
                if chars[i] == '}' { depth -= 1; }
                if depth > 0 { expr_str.push(chars[i]); }
                i += 1;
            }
            // Parse the expression
            let mut lexer = crate::lexer::Lexer::new(&expr_str);
            let tokens = lexer.tokenize();
            // Remove EOF
            let tokens: Vec<_> = tokens.into_iter()
                .filter(|t| !matches!(t.token, Token::Eof | Token::Newline))
                .collect();
            if tokens.is_empty() {
                bail!("empty expression in f-string");
            }
            let mut parser = Parser::new(tokens);
            let expr = parser.parse_expr()?;
            parts.push(FStringPart::Expr(expr));
        } else {
            literal.push(chars[i]);
            i += 1;
        }
    }

    if !literal.is_empty() {
        parts.push(FStringPart::Literal(literal));
    }

    Ok(parts)
}

pub struct Parser {
    tokens: Vec<Spanned>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Spanned>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse_program(&mut self) -> Result<Program> {
        let mut flows = Vec::new();
        self.skip_newlines();
        while !self.is_at_end() {
            flows.push(self.parse_flow()?);
            self.skip_newlines();
        }
        Ok(Program { flows })
    }

    // ─── Flow ───

    fn parse_flow(&mut self) -> Result<FlowDef> {
        self.expect(Token::Flow)?;
        let name = self.expect_ident()?;

        // Optional params: flow name(param: Type, ...)
        let mut params = Vec::new();
        if self.check(&Token::LParen) {
            self.advance();
            while !self.check(&Token::RParen) {
                let pname = self.expect_ident()?;
                self.expect(Token::Colon)?;
                let ty = self.parse_type()?;
                params.push(Param { name: pname, ty });
                if !self.check(&Token::RParen) {
                    self.expect(Token::Comma)?;
                }
            }
            self.expect(Token::RParen)?;
        }

        // Optional return type: -> Type
        let return_type = if self.check(&Token::Arrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(Token::Colon)?;
        self.expect_newline()?;
        let body = self.parse_block()?;

        Ok(FlowDef { name, params, return_type, body })
    }

    // ─── Block (indented) ───

    fn parse_block(&mut self) -> Result<Vec<Stmt>> {
        self.expect(Token::Indent)?;
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            if self.check(&Token::Dedent) || self.is_at_end() {
                break;
            }
            stmts.push(self.parse_stmt()?);
        }
        if self.check(&Token::Dedent) {
            self.advance();
        }
        Ok(stmts)
    }

    // ─── Statements ───

    fn parse_stmt(&mut self) -> Result<Stmt> {
        // Check for keywords first
        match self.peek_token() {
            Token::If => return self.parse_if(),
            Token::Loop => return self.parse_loop(),
            Token::For => return self.parse_for(),
            Token::Emit => return self.parse_emit(),
            Token::Return => return self.parse_return(),
            Token::Break => { self.advance(); self.skip_newlines(); return Ok(Stmt::Break); }
            Token::Continue => { self.advance(); self.skip_newlines(); return Ok(Stmt::Continue); }
            Token::Pass => { self.advance(); self.skip_newlines(); return Ok(Stmt::Pass); }
            _ => {}
        }

        // Assignment or bare expression
        let expr = self.parse_expr()?;

        // Check for assignment: name = expr
        if self.check(&Token::Eq) {
            if let Expr::Ident(name) = expr {
                self.advance(); // consume =
                let value = self.parse_expr()?;
                self.skip_newlines();
                return Ok(Stmt::Assign { name, expr: value });
            }
            bail!("line {}: left side of assignment must be a name", self.current_line());
        }

        self.skip_newlines();
        Ok(Stmt::Expr(expr))
    }

    fn parse_emit(&mut self) -> Result<Stmt> {
        self.expect(Token::Emit)?;
        self.expect(Token::LParen)?;
        let value = self.parse_expr()?;
        self.expect(Token::RParen)?;
        self.skip_newlines();
        Ok(Stmt::Emit { value })
    }

    fn parse_return(&mut self) -> Result<Stmt> {
        self.advance(); // consume 'return'
        let value = self.parse_expr()?;
        self.skip_newlines();
        Ok(Stmt::Return { value })
    }

    fn parse_if(&mut self) -> Result<Stmt> {
        self.expect(Token::If)?;
        let condition = self.parse_expr()?;
        self.expect(Token::Colon)?;

        // Body can be inline (single stmt on same line) or block
        let body = if self.check(&Token::Newline) {
            self.advance();
            self.parse_block()?
        } else {
            let stmt = self.parse_stmt()?;
            vec![stmt]
        };

        let mut elifs = Vec::new();
        let mut else_body = Vec::new();

        self.skip_newlines();
        while self.check(&Token::Elif) {
            self.advance();
            let cond = self.parse_expr()?;
            self.expect(Token::Colon)?;
            let elif_body = if self.check(&Token::Newline) {
                self.advance();
                self.parse_block()?
            } else {
                vec![self.parse_stmt()?]
            };
            elifs.push((cond, elif_body));
            self.skip_newlines();
        }

        if self.check(&Token::Else) {
            self.advance();
            self.expect(Token::Colon)?;
            else_body = if self.check(&Token::Newline) {
                self.advance();
                self.parse_block()?
            } else {
                vec![self.parse_stmt()?]
            };
        }

        Ok(Stmt::If { condition, body, elifs, else_body })
    }

    fn parse_loop(&mut self) -> Result<Stmt> {
        self.expect(Token::Loop)?;
        // Optional: loop max=N
        let max = if self.check_ident("max") {
            self.advance(); // consume 'max'
            self.expect(Token::Eq)?;
            if let Token::IntLit(n) = self.peek_token() {
                let n = n as u32;
                self.advance();
                Some(n)
            } else {
                bail!("line {}: expected integer after max=", self.current_line());
            }
        } else {
            None
        };
        self.expect(Token::Colon)?;
        self.expect_newline()?;
        let body = self.parse_block()?;
        Ok(Stmt::Loop { max, body })
    }

    fn parse_for(&mut self) -> Result<Stmt> {
        self.expect(Token::For)?;
        let var = self.expect_ident()?;
        self.expect(Token::In)?;
        let iterable = self.parse_expr()?;
        self.expect(Token::Colon)?;
        self.expect_newline()?;
        let body = self.parse_block()?;
        Ok(Stmt::For { var, iterable, body })
    }

    // ─── Expressions ───

    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr> {
        let mut left = self.parse_and()?;
        while self.check(&Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinOp { left: Box::new(left), op: BinOp::Or, right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr> {
        let mut left = self.parse_comparison()?;
        while self.check(&Token::And) {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinOp { left: Box::new(left), op: BinOp::And, right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr> {
        let mut left = self.parse_addition()?;
        loop {
            let op = match self.peek_token() {
                Token::EqEq => BinOp::Eq,
                Token::NotEq => BinOp::NotEq,
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::LtEq => BinOp::LtEq,
                Token::GtEq => BinOp::GtEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_addition()?;
            left = Expr::BinOp { left: Box::new(left), op, right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_addition(&mut self) -> Result<Expr> {
        let mut left = self.parse_multiplication()?;
        loop {
            let op = match self.peek_token() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplication()?;
            left = Expr::BinOp { left: Box::new(left), op, right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek_token() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinOp { left: Box::new(left), op, right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        if self.check(&Token::Not) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::UnaryOp { op: UnaryOp::Not, operand: Box::new(operand) });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.check(&Token::Dot) {
                self.advance();
                let field = self.expect_ident()?;
                expr = Expr::Field { object: Box::new(expr), field };
            } else if self.check(&Token::LParen) {
                // Function call on ident
                if let Expr::Ident(name) = expr {
                    expr = self.parse_call(name)?;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_call(&mut self, name: String) -> Result<Expr> {
        self.expect(Token::LParen)?;
        let mut args = Vec::new();
        let mut kwargs = Vec::new();

        while !self.check(&Token::RParen) {
            // Check for kwarg: name=expr
            if let Token::Ident(pname) = self.peek_token() {
                if self.peek_ahead(1) == Token::Eq {
                    let pname = pname.clone();
                    self.advance(); // consume name
                    self.advance(); // consume =
                    let val = self.parse_expr()?;
                    kwargs.push((pname, val));
                    if !self.check(&Token::RParen) {
                        self.expect(Token::Comma)?;
                    }
                    continue;
                }
            }
            // Positional arg
            args.push(self.parse_expr()?);
            if !self.check(&Token::RParen) {
                self.expect(Token::Comma)?;
            }
        }
        self.expect(Token::RParen)?;
        Ok(Expr::Call { name, args, kwargs })
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        match self.peek_token() {
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Ident(name))
            }
            Token::StringLit(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::StringLit(s))
            }
            Token::FStringLit(raw) => {
                let raw = raw.clone();
                self.advance();
                Ok(Expr::FString(parse_fstring_parts(&raw)?))
            }
            Token::IntLit(n) => {
                let n = n;
                self.advance();
                Ok(Expr::IntLit(n))
            }
            Token::FloatLit(n) => {
                let n = n;
                self.advance();
                Ok(Expr::FloatLit(n))
            }
            Token::True => { self.advance(); Ok(Expr::BoolLit(true)) }
            Token::False => { self.advance(); Ok(Expr::BoolLit(false)) }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Token::LBracket => {
                self.advance();
                let mut items = Vec::new();
                while !self.check(&Token::RBracket) {
                    items.push(self.parse_expr()?);
                    if !self.check(&Token::RBracket) {
                        self.expect(Token::Comma)?;
                    }
                }
                self.expect(Token::RBracket)?;
                Ok(Expr::List(items))
            }
            Token::LBrace => {
                self.advance();
                let mut entries = Vec::new();
                while !self.check(&Token::RBrace) {
                    // key must be a string literal
                    let key = if let Token::StringLit(s) = self.peek_token() {
                        self.advance();
                        s
                    } else {
                        bail!("line {}: map key must be a string literal", self.current_line());
                    };
                    self.expect(Token::Colon)?;
                    let value = self.parse_expr()?;
                    entries.push((key, value));
                    if !self.check(&Token::RBrace) {
                        self.expect(Token::Comma)?;
                    }
                }
                self.expect(Token::RBrace)?;
                Ok(Expr::Map(entries))
            }
            other => return Err(unexpected_token(self.current_line(), &other, "").into()),
        }
    }

    // ─── Types ───

    fn parse_type(&mut self) -> Result<TypeExpr> {
        let name = self.expect_ident()?;
        if self.check(&Token::LBracket) {
            self.advance();
            let mut args = vec![self.parse_type()?];
            while self.check(&Token::Comma) {
                self.advance();
                args.push(self.parse_type()?);
            }
            self.expect(Token::RBracket)?;
            Ok(TypeExpr::Generic(name, args))
        } else {
            Ok(TypeExpr::Named(name))
        }
    }

    // ─── Helpers ───

    fn peek_token(&self) -> Token {
        if self.pos < self.tokens.len() {
            self.tokens[self.pos].token.clone()
        } else {
            Token::Eof
        }
    }

    fn peek_ahead(&self, n: usize) -> Token {
        let idx = self.pos + n;
        if idx < self.tokens.len() {
            self.tokens[idx].token.clone()
        } else {
            Token::Eof
        }
    }

    fn check(&self, expected: &Token) -> bool {
        std::mem::discriminant(&self.peek_token()) == std::mem::discriminant(expected)
    }

    fn check_ident(&self, name: &str) -> bool {
        matches!(self.peek_token(), Token::Ident(ref s) if s == name)
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn expect(&mut self, expected: Token) -> Result<()> {
        let got = self.peek_token();
        if std::mem::discriminant(&got) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(CognosError::parse(
                self.current_line(),
                format!("expected {}, got {}", expected, got),
            ).into())
        }
    }

    fn expect_ident(&mut self) -> Result<String> {
        if let Token::Ident(name) = self.peek_token() {
            self.advance();
            Ok(name)
        } else {
            Err(CognosError::parse(
                self.current_line(),
                format!("expected a name, got {}", self.peek_token()),
            ).into())
        }
    }

    fn expect_newline(&mut self) -> Result<()> {
        if self.check(&Token::Newline) {
            self.advance();
            Ok(())
        } else {
            // Tolerate missing newline at certain positions
            Ok(())
        }
    }

    fn skip_newlines(&mut self) {
        while self.check(&Token::Newline) {
            self.advance();
        }
    }

    fn is_at_end(&self) -> bool {
        self.check(&Token::Eof)
    }

    fn current_line(&self) -> usize {
        if self.pos < self.tokens.len() {
            self.tokens[self.pos].line
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(source: &str) -> Result<Program> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    #[test]
    fn test_hello_world() {
        let program = parse(r#"flow hello:
    input = receive(String)
    emit(input)
"#).unwrap();
        assert_eq!(program.flows.len(), 1);
        assert_eq!(program.flows[0].name, "hello");
        assert_eq!(program.flows[0].body.len(), 2);
    }

    #[test]
    fn test_flow_with_params() {
        let program = parse(r#"flow greet(name: String) -> String:
    msg = think(name, system="Say hello.")
    return msg
"#).unwrap();
        let flow = &program.flows[0];
        assert_eq!(flow.name, "greet");
        assert_eq!(flow.params.len(), 1);
        assert_eq!(flow.params[0].name, "name");
        assert!(flow.return_type.is_some());
    }

    #[test]
    fn test_if_else() {
        let program = parse(r#"flow test:
    x = true
    if x:
        emit(x)
    else:
        emit(false)
"#).unwrap();
        let body = &program.flows[0].body;
        assert_eq!(body.len(), 2); // assign + if
        assert!(matches!(body[1], Stmt::If { .. }));
    }

    #[test]
    fn test_loop() {
        let program = parse(r#"flow test:
    loop max=10:
        x = think(y)
        break
"#).unwrap();
        let body = &program.flows[0].body;
        assert!(matches!(body[0], Stmt::Loop { max: Some(10), .. }));
    }

    #[test]
    fn test_kwargs() {
        let program = parse(r#"flow test:
    x = think(input, system="hello", tools=[])
"#).unwrap();
        let body = &program.flows[0].body;
        if let Stmt::Assign { expr: Expr::Call { kwargs, .. }, .. } = &body[0] {
            assert_eq!(kwargs.len(), 2);
            assert_eq!(kwargs[0].0, "system");
        } else {
            panic!("expected assignment with call");
        }
    }
}
