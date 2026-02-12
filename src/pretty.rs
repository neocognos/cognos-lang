/// Pretty-printer: renders AST back to readable Cognos-like syntax.

use crate::ast::*;

pub fn pretty_program(prog: &Program) -> String {
    let mut out = String::new();
    for td in &prog.types {
        match td {
            TypeDef::Struct { name, fields } => {
                out.push_str(&format!("type {}:\n", name));
                for f in fields {
                    let opt = if f.optional { "?" } else { "" };
                    out.push_str(&format!("    {}{}: {}\n", f.name, opt, pretty_type(&f.ty)));
                }
            }
            TypeDef::Enum { name, variants } => {
                let quoted: Vec<String> = variants.iter().map(|v| format!("\"{}\"", v)).collect();
                out.push_str(&format!("type {}: {}\n", name, quoted.join(" | ")));
            }
        }
        out.push('\n');
    }
    for (i, flow) in prog.flows.iter().enumerate() {
        if i > 0 || !prog.types.is_empty() { out.push('\n'); }
        pretty_flow(&mut out, flow, 0);
    }
    out
}

fn indent(out: &mut String, level: usize) {
    for _ in 0..level { out.push_str("    "); }
}

fn pretty_flow(out: &mut String, flow: &FlowDef, level: usize) {
    indent(out, level);
    out.push_str(&format!("flow {}", flow.name));
    if !flow.params.is_empty() {
        out.push('(');
        for (i, p) in flow.params.iter().enumerate() {
            if i > 0 { out.push_str(", "); }
            out.push_str(&format!("{}: {}", p.name, pretty_type(&p.ty)));
        }
        out.push(')');
    }
    if let Some(ref rt) = flow.return_type {
        out.push_str(&format!(" -> {}", pretty_type(rt)));
    }
    out.push_str(":\n");
    for stmt in &flow.body {
        pretty_stmt(out, stmt, level + 1);
    }
}

fn pretty_type(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Named(n) => n.clone(),
        TypeExpr::Generic(n, args) => {
            let a: Vec<String> = args.iter().map(|t| pretty_type(t)).collect();
            format!("{}[{}]", n, a.join(", "))
        }
        TypeExpr::Struct(fields) => {
            let f: Vec<String> = fields.iter().map(|(k, v)| format!("{}: {}", k, pretty_type(v))).collect();
            format!("{{ {} }}", f.join(", "))
        }
    }
}

fn pretty_stmt(out: &mut String, stmt: &Stmt, level: usize) {
    match stmt {
        Stmt::Assign { name, expr } => {
            indent(out, level);
            out.push_str(&format!("{} = {}\n", name, pretty_expr(expr)));
        }
        Stmt::Emit { value } => {
            indent(out, level);
            out.push_str(&format!("emit({})\n", pretty_expr(value)));
        }
        Stmt::Return { value } => {
            indent(out, level);
            out.push_str(&format!("return {}\n", pretty_expr(value)));
        }
        Stmt::Break => {
            indent(out, level);
            out.push_str("break\n");
        }
        Stmt::Continue => {
            indent(out, level);
            out.push_str("continue\n");
        }
        Stmt::Pass => {
            indent(out, level);
            out.push_str("pass\n");
        }
        Stmt::If { condition, body, elifs, else_body } => {
            indent(out, level);
            out.push_str(&format!("if {}:\n", pretty_expr(condition)));
            for s in body { pretty_stmt(out, s, level + 1); }
            for (cond, stmts) in elifs {
                indent(out, level);
                out.push_str(&format!("elif {}:\n", pretty_expr(cond)));
                for s in stmts { pretty_stmt(out, s, level + 1); }
            }
            if !else_body.is_empty() {
                indent(out, level);
                out.push_str("else:\n");
                for s in else_body { pretty_stmt(out, s, level + 1); }
            }
        }
        Stmt::TryCatch { body, error_var, catch_body } => {
            indent(out, level);
            out.push_str("try:\n");
            for s in body { pretty_stmt(out, s, level + 1); }
            indent(out, level);
            if let Some(var) = error_var {
                out.push_str(&format!("catch {}:\n", var));
            } else {
                out.push_str("catch:\n");
            }
            for s in catch_body { pretty_stmt(out, s, level + 1); }
        }
        Stmt::For { var, value_var, iterable, body } => {
            indent(out, level);
            if let Some(vv) = value_var {
                out.push_str(&format!("for {}, {} in {}:\n", var, vv, pretty_expr(iterable)));
            } else {
                out.push_str(&format!("for {} in {}:\n", var, pretty_expr(iterable)));
            }
            for s in body { pretty_stmt(out, s, level + 1); }
        }
        Stmt::Loop { max, body } => {
            indent(out, level);
            if let Some(n) = max {
                out.push_str(&format!("loop max={}:\n", n));
            } else {
                out.push_str("loop:\n");
            }
            for s in body { pretty_stmt(out, s, level + 1); }
        }
        Stmt::Parallel { body } => {
            indent(out, level);
            out.push_str("parallel:\n");
            for s in body { pretty_stmt(out, s, level + 1); }
        }
        Stmt::Expr(expr) => {
            indent(out, level);
            out.push_str(&format!("{}\n", pretty_expr(expr)));
        }
    }
}

fn pretty_expr(expr: &Expr) -> String {
    match expr {
        Expr::Ident(name) => name.clone(),
        Expr::StringLit(s) => format!("\"{}\"", s),
        Expr::IntLit(n) => n.to_string(),
        Expr::FloatLit(n) => format!("{}", n),
        Expr::BoolLit(b) => b.to_string(),
        Expr::Call { name, args, kwargs } => {
            let mut parts: Vec<String> = args.iter().map(|a| pretty_expr(a)).collect();
            for (k, v) in kwargs {
                parts.push(format!("{}={}", k, pretty_expr(v)));
            }
            format!("{}({})", name, parts.join(", "))
        }
        Expr::Field { object, field } => {
            format!("{}.{}", pretty_expr(object), field)
        }
        Expr::Index { object, index } => {
            format!("{}[{}]", pretty_expr(object), pretty_expr(index))
        }
        Expr::Slice { object, start, end } => {
            let s = start.as_ref().map(|e| pretty_expr(e)).unwrap_or_default();
            let e = end.as_ref().map(|e| pretty_expr(e)).unwrap_or_default();
            format!("{}[{}:{}]", pretty_expr(object), s, e)
        }
        Expr::MethodCall { object, method, args } => {
            let a: Vec<String> = args.iter().map(|e| pretty_expr(e)).collect();
            format!("{}.{}({})", pretty_expr(object), method, a.join(", "))
        }
        Expr::BinOp { left, op, right } => {
            let op_str = match op {
                BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*", BinOp::Div => "/",
                BinOp::Eq => "==", BinOp::NotEq => "!=",
                BinOp::Lt => "<", BinOp::Gt => ">", BinOp::LtEq => "<=", BinOp::GtEq => ">=",
                BinOp::And => "and", BinOp::Or => "or",
            };
            format!("{} {} {}", pretty_expr(left), op_str, pretty_expr(right))
        }
        Expr::UnaryOp { op, operand } => {
            let op_str = match op { UnaryOp::Not => "not " };
            format!("{}{}", op_str, pretty_expr(operand))
        }
        Expr::List(items) => {
            let parts: Vec<String> = items.iter().map(|i| pretty_expr(i)).collect();
            format!("[{}]", parts.join(", "))
        }
        Expr::FString(parts) => {
            let mut s = String::from("f\"");
            for part in parts {
                match part {
                    crate::ast::FStringPart::Literal(lit) => s.push_str(lit),
                    crate::ast::FStringPart::Expr(e) => {
                        s.push('{');
                        s.push_str(&pretty_expr(e));
                        s.push('}');
                    }
                }
            }
            s.push('"');
            s
        }
        Expr::Async(inner) => {
            format!("async {}", pretty_expr(inner))
        }
        Expr::Map(entries) => {
            let parts: Vec<String> = entries.iter()
                .map(|(k, v)| format!("\"{}\": {}", k, pretty_expr(v)))
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
    }
}
