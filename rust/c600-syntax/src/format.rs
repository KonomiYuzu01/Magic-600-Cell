//! The canonical layout (LG1.4). It depends on the parsed program only, so formatting is idempotent and a formatted
//! file parses to the same program. Comments are not part of the parsed program and are dropped.

use crate::ast::*;
use crate::lex::quote;

const INDENT: &str = "    ";

/// The canonical text of a parsed file, or `None` for a file with syntax diagnostics or recovery holes: only a
/// complete parse has a canonical layout.
pub fn format(file: &ParsedFile) -> Option<String> {
    if !file.diagnostics.is_empty() || !file.holes.is_empty() {
        return None;
    }
    let (edition, module) = (file.edition.as_ref()?, file.module.as_ref()?);
    let mut out =
        format!("edition {};\nmodule {} requires {};\n", quote(edition), module.name.text, names(&module.requires));
    let mut after_declaration = false;
    for item in &file.items {
        match item {
            Item::Import(i) => {
                if after_declaration {
                    out.push('\n');
                }
                out.push_str(&format!("import {} {};\n", i.alias.text, quote(&i.digest)));
                after_declaration = false;
            }
            Item::Decl(d) => {
                out.push('\n');
                out.push_str(&declaration(d));
                after_declaration = true;
            }
        }
    }
    Some(out)
}

fn list(items: impl Iterator<Item = String>, separator: &str) -> String {
    items.collect::<Vec<_>>().join(separator)
}

fn names(names: &[Name]) -> String {
    format!("[{}]", list(names.iter().map(|n| n.text.clone()), ", "))
}

fn indent(depth: usize) -> String {
    INDENT.repeat(depth)
}

fn declaration(d: &Decl) -> String {
    let mut out = String::new();
    if d.exported {
        out.push_str("export ");
    }
    out.push_str(match d.kind {
        DeclKind::Fn => "fn ",
        DeclKind::View => "view ",
        DeclKind::Command => "command ",
    });
    out.push_str(&d.name.text);
    if !d.generics.is_empty() {
        out.push_str(&format!("<{}>", list(d.generics.iter().map(|g| g.text.clone()), ",")));
    }
    out.push_str(&format!("({}) -> {} effects {}", params(&d.params), ty(&d.result), names(&d.effects)));
    if let Some(charge) = &d.charge {
        out.push_str(&format!(" charge [{}]", charge.join(", ")));
    }
    if let Some(m) = &d.metadata {
        out.push_str(&format!(
            " row {} kind {} contexts {} permits {}",
            quote(&m.row),
            m.kind.text,
            names(&m.contexts),
            names(&m.permits)
        ));
    }
    format!("{out} {{\n{}}}\n", block(&d.body, 1))
}

fn params(params: &[ParamDecl]) -> String {
    list(params.iter().map(|p| format!("{}: {}", p.name.text, ty(&p.ty))), ", ")
}

/// The statements of a block, one per line at `depth`. The braces belong to the caller.
fn block(b: &Block, depth: usize) -> String {
    let mut out = String::new();
    for l in &b.lets {
        out.push_str(&format!("{}let {} = {};\n", indent(depth), l.name.text, expr(&l.value, depth)));
    }
    out.push_str(&format!("{}return {};\n", indent(depth), expr(&b.result, depth)));
    out
}

fn ty(t: &TypeExpr) -> String {
    match t {
        TypeExpr::Named { name, args, .. } if args.is_empty() => name.text.clone(),
        TypeExpr::Named { name, args, .. } => format!("{}<{}>", name.text, list(args.iter().map(ty), ",")),
        TypeExpr::Fn { params, result, effects, .. } => {
            format!("fn({}) -> {} effects {}", list(params.iter().map(ty), ", "), ty(result), names(effects))
        }
        TypeExpr::Record { fields, .. } if fields.is_empty() => "record {}".into(),
        TypeExpr::Record { fields, .. } => {
            format!("record {{ {} }}", list(fields.iter().map(|(n, t)| format!("{}: {}", n.text, ty(t))), ", "))
        }
    }
}

/// An expression whose first line continues the current line; its further lines are indented from `depth`.
fn expr(e: &Expr, depth: usize) -> String {
    match &e.kind {
        ExprKind::Int(n) => n.clone(),
        ExprKind::Rat { num, den } => format!("{num}/{den}"),
        ExprKind::Q5 { one, sqrt5 } => format!("q5({}/{}, {}/{})", one.0, one.1, sqrt5.0, sqrt5.1),
        ExprKind::Approx { precision, decimal } => format!("~{precision}{}", quote(decimal)),
        ExprKind::Text(s) => quote(s),
        ExprKind::Bool(b) => b.to_string(),
        ExprKind::Unit => "()".into(),
        ExprKind::Word(letters) => format!("word[{}]", list(letters.iter().map(|l| format!("{l:+}")), ", ")),
        ExprKind::Menu(letters) => format!("menu[{}]", list(letters.iter().map(|(g, r)| format!("{g}:{r}")), ", ")),
        ExprKind::Name(n) => n.clone(),
        ExprKind::Call { callee, type_args, args } => {
            let mut out = operand(callee, depth);
            if !type_args.is_empty() {
                out.push_str(&format!("<{}>", list(type_args.iter().map(ty), ",")));
            }
            format!("{out}({})", list(args.iter().map(|a| expr(a, depth)), ", "))
        }
        ExprKind::Field { value, field } => format!("{}.{}", operand(value, depth), field.text),
        ExprKind::Record(fields) if fields.is_empty() => "record {}".into(),
        ExprKind::Record(fields) => format!(
            "record {{ {} }}",
            list(fields.iter().map(|(n, e)| format!("{} = {}", n.text, expr(e, depth))), ", ")
        ),
        ExprKind::Closure { params: ps, result, body } => {
            format!("fn({}) -> {} {{\n{}{}}}", params(ps), ty(result), block(body, depth + 1), indent(depth))
        }
        ExprKind::Match { scrutinee, arms } => {
            let s = expr(scrutinee, depth);
            // In a scrutinee, `record {` and `match` cannot start the expression without parentheses.
            let s = if matches!(head(scrutinee).kind, ExprKind::Record(_) | ExprKind::Match { .. }) {
                format!("({s})")
            } else {
                s
            };
            if arms.is_empty() {
                return format!("match {s} {{}}");
            }
            let mut out = format!("match {s} {{\n");
            for arm in arms {
                out.push_str(&format!(
                    "{}{} => {};\n",
                    indent(depth + 1),
                    pattern(&arm.pattern, depth + 1),
                    expr(&arm.body, depth + 1)
                ));
            }
            out + &indent(depth) + "}"
        }
    }
}

/// The callee or record of a postfix operation. A `match` is parenthesized, and so is an integer before `.`, which
/// would otherwise read as a decimal number.
fn operand(e: &Expr, depth: usize) -> String {
    match e.kind {
        ExprKind::Match { .. } | ExprKind::Int(_) | ExprKind::Rat { .. } => format!("({})", expr(e, depth)),
        _ => expr(e, depth),
    }
}

/// The first atom of a chain of calls and field accesses.
fn head(e: &Expr) -> &Expr {
    match &e.kind {
        ExprKind::Call { callee, .. } => head(callee),
        ExprKind::Field { value, .. } => head(value),
        _ => e,
    }
}

fn pattern(p: &Pattern, depth: usize) -> String {
    match p {
        Pattern::Wildcard => "_".into(),
        Pattern::Binder { tag, binder } => format!("{}({})", tag.text, binder.text),
        Pattern::Lit(e) if plain_pattern(e) => expr(e, depth),
        Pattern::Lit(e) => format!("({})", expr(e, depth)),
    }
}

/// Whether a literal pattern reads back as itself without parentheses: not `_`, not a binder such as `yes(proof)`,
/// and no other construct that an arm would read differently.
fn plain_pattern(e: &Expr) -> bool {
    let name = |e: &Expr| matches!(&e.kind, ExprKind::Name(n) if n != "_");
    match &e.kind {
        ExprKind::Int(_)
        | ExprKind::Rat { .. }
        | ExprKind::Q5 { .. }
        | ExprKind::Approx { .. }
        | ExprKind::Text(_)
        | ExprKind::Bool(_)
        | ExprKind::Unit
        | ExprKind::Word(_)
        | ExprKind::Menu(_) => true,
        ExprKind::Name(_) => name(e),
        // `f(x)` and `f(true)` read as binders: one name token between the parentheses.
        ExprKind::Call { callee, type_args, args } => {
            let binder = |a: &Expr| matches!(a.kind, ExprKind::Name(_) | ExprKind::Bool(_));
            name(callee) && !(type_args.is_empty() && args.len() == 1 && binder(&args[0]))
        }
        _ => false,
    }
}
