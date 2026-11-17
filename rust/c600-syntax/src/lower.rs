//! Lowering parsed files into the IR draft (LG1.8, LG4.12).
//!
//! The IR's operations are reserved names called like functions, and every IR node keeps the source range it came
//! from, keyed by the node's path: the path the checker's diagnostics carry. A draft with syntax diagnostics or
//! recovery holes does not lower (LG1.5). What the IR draft cannot hold yet is refused with a message that names
//! what it waits for.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use c600_ir::Expr as Ir;
use c600_ir::{
    Accounting, Callee, Capability, Effect, EffectBound, EndTest, ExactRole, FnKind, FnTy, Function, Lit, Module,
    Order, Param, Prim, Program, QTy, ReceiptField, Ty,
};

use crate::ast::*;
use crate::Span;

/// Words a program cannot bind.
const KEYWORDS: [&str; 10] = ["fn", "let", "return", "match", "true", "false", "q5", "word", "menu", "_"];

/// The IR's operations, called like functions. A program cannot bind these names either.
const BUILTINS: [&str; 61] = [
    "if",
    "display_if",
    "exact_add",
    "exact_less",
    "exact_eq",
    "and",
    "or",
    "not",
    "approx24",
    "approx53",
    "approx113",
    "to_exact",
    "approx_add",
    "approx_less",
    "approx_and",
    "approx_not",
    "tag",
    "colour",
    "target",
    "read_snapshot",
    "publish_view",
    "write_workspace",
    "propose",
    "receipt_id",
    "receipt_base",
    "receipt_effect",
    "receipt_protection",
    "receipt_end_state",
    "receipt_n",
    "receipt_error",
    "use_budget",
    "use_goal",
    "use_evidence",
    "used_steps",
    "unused_steps",
    "budget_status",
    "declassified",
    "file_read",
    "file_write",
    "random_draw",
    "read_clock",
    // Lists and words (LP2 drafts).
    "list",
    "length",
    "index",
    "push",
    "concat",
    "inverse",
    "conjugate",
    "commutator",
    "compose",
    "word_key",
    // Sets and maps (LP2 drafts).
    "set",
    "map",
    "contains",
    "insert",
    "lookup",
    "elements",
    "keys",
    "entries",
    // Compressed words (LP2 drafts).
    "compress",
    "expand",
];

/// The named orders of an ordered collection result (LG4.3; LP2 draft). A program cannot bind these names either.
const ORDERS: [&str; 2] = ["ascending", "descending"];

/// The effect names of an `effects` list (LG3.1). `file` and `random` also name their capability (LG3.8).
const EFFECTS: [&str; 5] = ["snapshot", "view", "workspace", "preview", "budget"];

const BASE_TYPES: [&str; 20] = [
    "Unit",
    "Bool",
    "Nat",
    "Int",
    "Rat",
    "Q5",
    "Text",
    "Word",
    "Target",
    "Budget",
    "Status",
    "Colour",
    "ApproxBool",
    "Receipt",
    "WordEqualityKey",
    "CompressedWord",
    "MenuWord",
    // The verdicts of LB3.2's three end tests (LP2 draft); the tests' own names stay the goals' (LG2.10).
    "RetainedCheckpointVerdict",
    "LabelCheckpointVerdict",
    "StickerTestVerdict",
];

fn reserved(name: &str) -> bool {
    KEYWORDS.contains(&name) || BUILTINS.contains(&name) || ORDERS.contains(&name)
}

/// Whether a type is or holds a function. The LP2 draft's lists hold none, so a callable's needs never hide in one.
fn holds_callable(ty: &Ty) -> bool {
    match ty {
        Ty::Fn(_) => true,
        Ty::Record(fields) => fields.iter().any(|(_, f)| holds_callable(&f.ty)),
        other => other.elements().into_iter().any(|e| holds_callable(&e.ty)),
    }
}

/// Whether a set element or a map key may have this type (LG2.20; LP2 draft): exact and without the display qualifier
/// at any depth, and without a function, a receipt, `Unit`, a type parameter, a compressed word or an end test's
/// verdict, so that its canonical key order is defined (DET.9) and a verdict is only ever matched (LG2.9).
fn key_type(q: &QTy) -> bool {
    !q.display
        && match &q.ty {
            Ty::Unit
            | Ty::Approx(_)
            | Ty::ApproxBool
            | Ty::Fn(_)
            | Ty::Receipt
            | Ty::Var(_)
            | Ty::CompressedWord
            | Ty::EndTest(_) => false,
            Ty::Record(fields) => fields.iter().all(|(_, f)| key_type(f)),
            other => other.elements().into_iter().all(key_type),
        }
}

fn capability(name: &str) -> Option<Capability> {
    Some(match name {
        "file" => Capability::File,
        "clock" => Capability::Clock,
        "random" => Capability::Randomness,
        "network" => Capability::Network,
        "process" => Capability::Process,
        "session_file" => Capability::SessionFile,
        _ => return None,
    })
}

/// A lowered program and, for each module, where its IR came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lowered {
    pub program: Program,
    pub sources: Vec<SourceMap>,
}

impl Lowered {
    pub fn source(&self, module: &str) -> Option<&SourceMap> {
        self.sources.iter().find(|s| s.module == module)
    }
}

/// Where a module's IR came from (LG4.12): the SHA-256 of its pinned source, each function's declaration range, and
/// the source range of each IR node of each function body, keyed by path text ("root" or dot-joined child indices,
/// as in the checker's diagnostics).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceMap {
    pub module: String,
    pub digest: String,
    pub declarations: BTreeMap<String, Span>,
    pub paths: BTreeMap<String, BTreeMap<String, Span>>,
}

impl SourceMap {
    /// The source range of the node at `path` in `function`'s body.
    pub fn span(&self, function: &str, path: &str) -> Option<Span> {
        self.paths.get(function)?.get(path).copied()
    }
}

/// Why a set of parsed files does not lower.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LowerError {
    pub module: Option<String>,
    pub span: Option<Span>,
    pub message: String,
}

impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(m) = &self.module {
            write!(f, "module {m}: ")?;
        }
        if let Some(s) = self.span {
            write!(f, "{}:{}: ", s.line, s.column)?;
        }
        f.write_str(&self.message)
    }
}

impl std::error::Error for LowerError {}

fn error(module: &str, span: Span, message: impl Into<String>) -> LowerError {
    LowerError { module: Some(module.into()), span: Some(span), message: message.into() }
}

/// What a module declares: its functions, each with what a call of it needs, and its import aliases.
struct Scope {
    name: String,
    functions: BTreeMap<String, Signature>,
    aliases: BTreeMap<String, String>,
}

/// What a call of a module function needs: one argument per parameter and, from another module, an export.
#[derive(Clone, Copy)]
struct Signature {
    params: usize,
    exported: bool,
}

/// Lowers a set of parsed files, one module each, into one program. Imports resolve by source digest among the
/// given files.
pub fn lower(files: &[ParsedFile]) -> Result<Lowered, LowerError> {
    let mut digests = BTreeMap::new();
    let mut headers = Vec::new();
    for file in files {
        let refused = || LowerError {
            module: file.module.as_ref().map(|m| m.name.text.clone()),
            span: file.diagnostics.iter().find_map(|d| d.span),
            message: "a draft with syntax diagnostics or recovery holes does not lower (LG1.5)".into(),
        };
        if !file.diagnostics.is_empty() || !file.holes.is_empty() {
            return Err(refused());
        }
        let (Some(digest), Some(header)) = (&file.digest, &file.module) else { return Err(refused()) };
        let name = &header.name.text;
        if headers.iter().any(|(_, h, _): &(_, &ModuleHeader, _)| h.name.text == *name) {
            return Err(error(name, header.name.loc.0, format!("two given files declare module `{name}`")));
        }
        digests.insert(digest.as_str(), name.as_str());
        headers.push((file, header, digest));
    }
    let mut scopes = Vec::new();
    let mut capabilities = Vec::new();
    for (file, header, _) in &headers {
        capabilities.push(requires(header)?);
        scopes.push(declare(file, header, &digests)?);
    }
    let mut modules = Vec::new();
    let mut sources = Vec::new();
    for (((file, _, digest), scope), capabilities) in headers.iter().zip(&scopes).zip(capabilities) {
        let mut source = SourceMap { module: scope.name.clone(), digest: (*digest).clone(), ..SourceMap::default() };
        let mut functions = Vec::new();
        for d in file.declarations() {
            let mut body =
                Body { scope, all: &scopes, generics: Vec::new(), locals: Vec::new(), paths: BTreeMap::new() };
            let function = body.declaration(d)?;
            source.declarations.insert(function.name.clone(), d.loc.0);
            source.paths.insert(function.name.clone(), body.paths);
            functions.push(function);
        }
        modules.push(Module { name: scope.name.clone(), capabilities, functions });
        sources.push(source);
    }
    Ok(Lowered { program: Program::new(modules), sources })
}

/// The capabilities a module header requests (LG3.8, LG3.9).
fn requires(header: &ModuleHeader) -> Result<BTreeSet<Capability>, LowerError> {
    let module = header.name.text.as_str();
    let mut out = BTreeSet::new();
    for r in &header.requires {
        let name = r.text.as_str();
        let c = match capability(name) {
            Some(c) => c,
            None if EFFECTS.contains(&name) => {
                let message = format!(
                    "`{name}` is an effect: a declaration names it in `effects`, and `requires` lists capabilities only \
                     (LG3.8, LG3.9)"
                );
                return Err(error(module, r.loc.0, message));
            }
            None => return Err(error(module, r.loc.0, format!("`{name}` is not a capability"))),
        };
        if !out.insert(c) {
            return Err(error(module, r.loc.0, format!("`{name}` is required twice")));
        }
    }
    Ok(out)
}

/// The names a module declares: import aliases, resolved by digest, and function names.
fn declare(file: &ParsedFile, header: &ModuleHeader, digests: &BTreeMap<&str, &str>) -> Result<Scope, LowerError> {
    let module = header.name.text.as_str();
    let mut scope = Scope { name: module.into(), functions: BTreeMap::new(), aliases: BTreeMap::new() };
    for item in &file.items {
        let Item::Import(i) = item else { continue };
        let alias = i.alias.text.as_str();
        let target = digests
            .get(i.digest.as_str())
            .ok_or_else(|| error(module, i.loc.0, format!("no given source file has the digest {}", i.digest)))?;
        if reserved(alias) {
            return Err(error(module, i.alias.loc.0, format!("`{alias}` is reserved")));
        }
        if scope.aliases.insert(alias.into(), target.to_string()).is_some() {
            return Err(error(module, i.alias.loc.0, format!("the alias `{alias}` is imported twice")));
        }
    }
    for d in file.declarations() {
        let name = d.name.text.as_str();
        let why = if reserved(name) {
            "is reserved"
        } else if scope.aliases.contains_key(name) {
            "already names an imported module"
        } else if scope.functions.contains_key(name) {
            "is declared twice"
        } else {
            scope.functions.insert(name.into(), Signature { params: d.params.len(), exported: d.exported });
            continue;
        };
        return Err(error(module, d.name.loc.0, format!("`{name}` {why}")));
    }
    Ok(scope)
}

/// The text of a path: "root" or dot-joined child indices.
fn path_text(path: &[u32]) -> String {
    if path.is_empty() {
        "root".into()
    } else {
        path.iter().map(u32::to_string).collect::<Vec<_>>().join(".")
    }
}

/// The literal an expression writes, if it is one.
fn literal(kind: &ExprKind) -> Option<Lit> {
    Some(match kind {
        ExprKind::Int(n) if n.starts_with('-') => Lit::Int(n.clone()),
        ExprKind::Int(n) => Lit::Nat(n.clone()),
        ExprKind::Rat { num, den } => Lit::Rat { num: num.clone(), den: den.clone() },
        ExprKind::Q5 { one, sqrt5 } => Lit::Q5 { one: one.clone(), sqrt5: sqrt5.clone() },
        ExprKind::Approx { precision, decimal } => Lit::Approx { decimal: decimal.clone(), precision: *precision },
        ExprKind::Text(s) => Lit::Text(s.clone()),
        ExprKind::Bool(b) => Lit::Bool(*b),
        ExprKind::Unit => Lit::Unit,
        ExprKind::Word(w) => Lit::Word(w.clone()),
        // Letters of the run's pinned menu (LB1.8; LP2 draft); LX-8's fixture rotation names are no item.
        ExprKind::Menu(letters) => {
            Lit::MenuWord(letters.iter().map(|(cap, item)| Some([*cap, item.parse().ok()?])).collect::<Option<_>>()?)
        }
        _ => return None,
    })
}

const MENU: &str = "LX-8's fixture menu letters do not lower: a letter of the run's pinned menu is cap:item, two \
                    naturals below 600 (LB1.8)";

/// One declaration being lowered.
struct Body<'a> {
    scope: &'a Scope,
    all: &'a [Scope],
    generics: Vec<String>,
    /// The names in scope. A binding never shadows another.
    locals: Vec<String>,
    paths: BTreeMap<String, Span>,
}

type R<T> = Result<T, LowerError>;

impl Body<'_> {
    fn err(&self, span: Span, message: impl Into<String>) -> LowerError {
        error(&self.scope.name, span, message)
    }

    fn is_local(&self, name: &str) -> bool {
        self.locals.iter().any(|l| l == name)
    }

    fn is_generic(&self, name: &str) -> bool {
        self.generics.iter().any(|g| g == name)
    }

    fn declaration(&mut self, d: &Decl) -> R<Function> {
        if d.charge.is_some() {
            return Err(self.err(d.name.loc.0, "a declared charge waits for LG4.8's tariffs, so it does not lower yet"));
        }
        if d.metadata.is_some() {
            return Err(
                self.err(d.name.loc.0, "command metadata waits for the command table (LP3), so it does not lower yet")
            );
        }
        for g in &d.generics {
            let name = g.text.as_str();
            if reserved(name)
                || BASE_TYPES.contains(&name)
                || matches!(name, "Display" | "Approx")
                || EFFECTS.contains(&name)
                || capability(name).is_some()
                || self.is_generic(name)
            {
                return Err(self.err(g.loc.0, format!("`{name}` cannot name a generic parameter")));
            }
            self.generics.push(name.into());
        }
        let mut params = Vec::new();
        for p in &d.params {
            let ty = self.ty(&p.ty)?;
            self.bind(&p.name)?;
            params.push(Param { name: p.name.text.clone(), ty });
        }
        let result = self.ty(&d.result)?;
        let effects = self.effects(&d.effects)?;
        let body = self.block(&d.body, &mut Vec::new())?;
        let kind = match d.kind {
            DeclKind::Fn => FnKind::Helper,
            DeclKind::View => FnKind::View,
            DeclKind::Command => FnKind::Command,
        };
        Ok(Function {
            name: d.name.text.clone(),
            kind,
            exported: d.exported,
            params,
            result,
            effects: Some(effects),
            body,
        })
    }

    fn bind(&mut self, name: &Name) -> R<()> {
        let n = name.text.as_str();
        let why = if reserved(n) {
            "is reserved"
        } else if self.is_local(n) {
            "is already bound here, and a binding does not shadow another"
        } else if self.scope.functions.contains_key(n) {
            "names a function of this module"
        } else if self.scope.aliases.contains_key(n) {
            "names an imported module"
        } else {
            self.locals.push(n.into());
            return Ok(());
        };
        Err(self.err(name.loc.0, format!("`{n}` {why}")))
    }

    /// Lowers `f` at child `i` of `path`.
    fn at<T>(&mut self, path: &mut Vec<u32>, i: usize, f: impl FnOnce(&mut Self, &mut Vec<u32>) -> R<T>) -> R<T> {
        path.push(i as u32);
        let out = f(self, path);
        path.pop();
        out
    }

    fn block(&mut self, b: &Block, path: &mut Vec<u32>) -> R<Ir> {
        let depth = self.locals.len();
        let out = self.lets(&b.lets, &b.result, path);
        self.locals.truncate(depth);
        out
    }

    /// `let a = v; rest` is `Let { a, v, rest }`: the value at child 0 and the rest at child 1. Its range runs from
    /// `let` to the end of the block's result.
    fn lets(&mut self, lets: &[LetStmt], result: &Expr, path: &mut Vec<u32>) -> R<Ir> {
        let Some((first, rest)) = lets.split_first() else { return self.expr(result, path) };
        self.paths.insert(path_text(path), Span { end: result.loc.0.end, ..first.loc.0 });
        let value = self.at(path, 0, |b, p| b.expr(&first.value, p))?;
        self.bind(&first.name)?;
        let body = self.at(path, 1, |b, p| b.lets(rest, result, p))?;
        Ok(Ir::Let { name: first.name.text.clone(), value: Box::new(value), body: Box::new(body) })
    }

    fn expr(&mut self, e: &Expr, path: &mut Vec<u32>) -> R<Ir> {
        let span = e.loc.0;
        self.paths.insert(path_text(path), span);
        if let Some(lit) = literal(&e.kind) {
            return Ok(Ir::Lit(lit));
        }
        Ok(match &e.kind {
            ExprKind::Menu(_) => return Err(self.err(span, MENU)),
            ExprKind::Name(n) => self.name(n, span)?,
            ExprKind::Field { value, field } => match self.alias_of(value) {
                Some(module) => {
                    self.imported(module, field)?;
                    Ir::FnRef { module: Some(module.into()), name: field.text.clone() }
                }
                None => {
                    let value = self.at(path, 0, |b, p| b.expr(value, p))?;
                    Ir::Field { value: Box::new(value), field: field.text.clone() }
                }
            },
            ExprKind::Call { callee, type_args, args } => self.call(callee, type_args, args, span, path)?,
            ExprKind::Record(fields) => {
                let mut seen = BTreeSet::new();
                let mut out = Vec::new();
                for (i, (name, value)) in fields.iter().enumerate() {
                    if !seen.insert(name.text.as_str()) {
                        return Err(self.err(name.loc.0, format!("field `{}` appears twice", name.text)));
                    }
                    out.push((name.text.clone(), self.at(path, i, |b, p| b.expr(value, p))?));
                }
                Ir::Record(out)
            }
            ExprKind::Closure { params, result, body } => {
                let depth = self.locals.len();
                let out = self.closure(params, result, body, path);
                self.locals.truncate(depth);
                out?
            }
            ExprKind::Match { scrutinee, arms } => self.matching(scrutinee, arms, span, path)?,
            _ => unreachable!("literals return above"),
        })
    }

    fn name(&self, n: &str, span: Span) -> R<Ir> {
        if self.is_local(n) {
            return Ok(Ir::Var(n.into()));
        }
        if self.scope.functions.contains_key(n) {
            return Ok(Ir::FnRef { module: None, name: n.into() });
        }
        let message = if BUILTINS.contains(&n) {
            format!("`{n}` is a reserved operation: call it")
        } else if self.scope.aliases.contains_key(n) {
            format!("`{n}` names an imported module: write `{n}.f` for its function `f`")
        } else {
            format!("`{n}` is not bound")
        };
        Err(self.err(span, message))
    }

    /// The module an `alias` in `alias.f` names.
    fn alias_of(&self, value: &Expr) -> Option<&str> {
        match &value.kind {
            ExprKind::Name(a) if !self.is_local(a) => self.scope.aliases.get(a).map(String::as_str),
            _ => None,
        }
    }

    /// The signature of `f` in an imported module, which must declare `f` and export it, as the checker requires of
    /// a function another module calls or names.
    fn imported(&self, module: &str, f: &Name) -> R<Signature> {
        let name = f.text.as_str();
        match self.all.iter().find(|s| s.name == module).and_then(|s| s.functions.get(name)) {
            None => Err(self.err(f.loc.0, format!("module `{module}` declares no function `{name}`"))),
            Some(s) if !s.exported => Err(self.err(f.loc.0, format!("module `{module}` does not export `{name}`"))),
            Some(s) => Ok(*s),
        }
    }

    /// A call gives one argument per parameter. `callee` is the operation or function as the call writes it.
    fn takes(&self, callee: &str, params: usize, args: &[Expr], span: Span) -> R<()> {
        if args.len() == params {
            return Ok(());
        }
        let plural = if params == 1 { "" } else { "s" };
        Err(self.err(span, format!("`{callee}` takes {params} argument{plural}")))
    }

    fn args(&mut self, args: &[Expr], path: &mut Vec<u32>) -> R<Vec<Ir>> {
        args.iter().enumerate().map(|(i, a)| self.at(path, i, |b, p| b.expr(a, p))).collect()
    }

    /// A `List`, `Set` or `Map` type from its lowered type arguments (LP2 drafts). A collection holds no function, and
    /// a set element or a map key is exact (LG2.20).
    fn collection(&self, kind: &str, mut types: Vec<QTy>, span: Span) -> R<Ty> {
        if types.iter().any(|t| holds_callable(&t.ty)) {
            return Err(self.err(span, format!("a `{kind}` holds no function in the LP2 draft")));
        }
        if kind != "List" && !key_type(&types[0]) {
            let note = format!(
                "a `{kind}` element or key is exact, without a display qualifier, an approximation, a function, a \
                 receipt, `Unit`, a type parameter, an end test's verdict or a compressed word (LG2.20)"
            );
            return Err(self.err(span, note));
        }
        let mut next = || Box::new(types.remove(0));
        Ok(match kind {
            "List" => Ty::List(next()),
            "Set" => Ty::Set(next()),
            _ => {
                let key = next();
                Ty::Map(key, next())
            }
        })
    }

    fn untyped(&self, type_args: &[TypeExpr], span: Span) -> R<()> {
        if type_args.is_empty() {
            Ok(())
        } else {
            Err(self.err(
                span,
                "only a reserved operation takes type arguments; the IR draft has no explicit instantiation",
            ))
        }
    }

    /// A call of a reserved operation, a module function, an imported function or a callable value. A call of a
    /// reserved operation or a module function gives one argument per parameter; a callable value's number of
    /// parameters is part of its type, which the checker compares (LG2.1). A callable value's arguments come first
    /// in path order, then the callee.
    fn call(&mut self, callee: &Expr, type_args: &[TypeExpr], args: &[Expr], span: Span, path: &mut Vec<u32>) -> R<Ir> {
        if let ExprKind::Name(n) = &callee.kind {
            if !self.is_local(n) && BUILTINS.contains(&n.as_str()) {
                return self.builtin(n, type_args, args, span, path);
            }
            if let Some(&signature) = self.scope.functions.get(n).filter(|_| !self.is_local(n)) {
                self.untyped(type_args, span)?;
                self.takes(n, signature.params, args, span)?;
                let args = self.args(args, path)?;
                return Ok(Ir::Call { callee: Callee::Fn { module: None, name: n.clone() }, args });
            }
        }
        if let ExprKind::Field { value, field } = &callee.kind {
            if let (ExprKind::Name(alias), Some(module)) = (&value.kind, self.alias_of(value)) {
                let module = module.to_string();
                let signature = self.imported(&module, field)?;
                self.untyped(type_args, span)?;
                self.takes(&format!("{alias}.{}", field.text), signature.params, args, span)?;
                let args = self.args(args, path)?;
                return Ok(Ir::Call { callee: Callee::Fn { module: Some(module), name: field.text.clone() }, args });
            }
        }
        self.untyped(type_args, span)?;
        let lowered = self.args(args, path)?;
        let f = self.at(path, args.len(), |b, p| b.expr(callee, p))?;
        Ok(Ir::Call { callee: Callee::Value(Box::new(f)), args: lowered })
    }

    /// A literal string argument of a reserved operation; it names something and is not an IR node.
    fn text(&self, op: &str, arg: &Expr) -> R<String> {
        match &arg.kind {
            ExprKind::Text(s) => Ok(s.clone()),
            _ => Err(self.err(arg.loc.0, format!("`{op}` takes a literal string here"))),
        }
    }

    fn unqualified(&self, op: &str, t: &TypeExpr, span: Span) -> R<Ty> {
        let q = self.ty(t)?;
        if q.display {
            return Err(self.err(span, format!("`{op}` takes a type without the display qualifier")));
        }
        Ok(q.ty)
    }

    fn builtin(&mut self, op: &str, type_args: &[TypeExpr], args: &[Expr], span: Span, path: &mut Vec<u32>) -> R<Ir> {
        let typed = match op {
            "read_snapshot" | "write_workspace" | "declassified" | "file_read" | "list" | "set" => 1,
            "map" => 2,
            _ => 0,
        };
        if type_args.len() != typed {
            let wanted = ["no type argument", "one type argument", "two type arguments"][typed];
            return Err(self.err(span, format!("`{op}` takes {wanted}")));
        }
        let arity = match op {
            "if" | "display_if" => 3,
            "exact_add" | "exact_less" | "exact_eq" | "and" | "or" | "approx_add" | "approx_less" | "approx_and"
            | "write_workspace" | "propose" | "file_write" => 2,
            "index" | "push" | "concat" | "conjugate" | "commutator" | "compose" => 2,
            // An ordered result names its order (LG4.3).
            "contains" | "lookup" | "elements" | "keys" | "entries" => 2,
            // `read_snapshot<T>("q", x)` reads the engine's row `q` for `x` (LP2 draft).
            "read_snapshot" if args.len() == 2 => 2,
            // `insert(s, x)` adds to a set, `insert(m, k, v)` to a map (LP2 drafts).
            "insert" if args.len() == 3 => 3,
            "insert" => 2,
            // `expand(c, start, count)` (LP2 draft).
            "expand" => 3,
            "used_steps" | "unused_steps" | "budget_status" | "random_draw" | "read_clock" => 0,
            // `list<T>(a, b, ...)` and `set<T>(a, b, ...)` take their items, `map<K, V>(k, v, ...)` its keys and values
            // in turn (LP2 drafts).
            "list" | "set" | "map" => args.len(),
            _ => 1,
        };
        self.takes(op, arity, args, span)?;
        if let "list" | "set" | "map" = op {
            let mut types = Vec::new();
            for t in type_args {
                types.push(self.ty(t)?);
            }
            if op == "map" && args.len() % 2 == 1 {
                return Err(self.err(span, "`map` takes its keys and values in pairs"));
            }
            let kind = match op {
                "list" => "List",
                "set" => "Set",
                _ => "Map",
            };
            let ty = self.collection(kind, types, span)?;
            return Ok(Ir::Collection { ty, items: self.args(args, path)? });
        }
        if let "elements" | "keys" | "entries" = op {
            let order = match &args[1].kind {
                ExprKind::Name(name) if name == "ascending" => Order::Ascending,
                ExprKind::Name(name) if name == "descending" => Order::Descending,
                _ => {
                    let note = format!("`{op}` names its order, `ascending` or `descending` (LG4.3)");
                    return Err(self.err(args[1].loc.0, note));
                }
            };
            let prim = match op {
                "elements" => Prim::Elements(order),
                "keys" => Prim::Keys(order),
                _ => Prim::Entries(order),
            };
            return Ok(Ir::Prim { op: prim, args: self.args(&args[..1], path)? });
        }
        let prim = match op {
            "length" => Some(Prim::Length),
            "index" => Some(Prim::Index),
            "push" => Some(Prim::Push),
            "concat" => Some(Prim::Concat),
            "inverse" => Some(Prim::Inverse),
            "conjugate" => Some(Prim::Conjugate),
            "commutator" => Some(Prim::Commutator),
            "compose" => Some(Prim::Compose),
            "word_key" => Some(Prim::WordKey),
            "contains" => Some(Prim::Contains),
            "lookup" => Some(Prim::Lookup),
            "insert" if args.len() == 3 => Some(Prim::InsertEntry),
            "insert" => Some(Prim::Insert),
            "compress" => Some(Prim::Compress),
            "expand" => Some(Prim::Expand),
            "exact_add" => Some(Prim::ExactAdd),
            "exact_less" => Some(Prim::ExactLess),
            "exact_eq" => Some(Prim::ExactEq),
            "and" => Some(Prim::And),
            "or" => Some(Prim::Or),
            "not" => Some(Prim::Not),
            "approx24" => Some(Prim::ToApprox(24)),
            "approx53" => Some(Prim::ToApprox(53)),
            "approx113" => Some(Prim::ToApprox(113)),
            "to_exact" => Some(Prim::ToExact),
            "approx_add" => Some(Prim::ApproxAdd),
            "approx_less" => Some(Prim::ApproxLess),
            "approx_and" => Some(Prim::ApproxAnd),
            "approx_not" => Some(Prim::ApproxNot),
            _ => None,
        };
        if let Some(op) = prim {
            return Ok(Ir::Prim { op, args: self.args(args, path)? });
        }
        let receipt = match op {
            "receipt_id" => Some(ReceiptField::Id),
            "receipt_base" => Some(ReceiptField::Base),
            "receipt_effect" => Some(ReceiptField::Effect),
            "receipt_protection" => Some(ReceiptField::Protection),
            "receipt_end_state" => Some(ReceiptField::EndState),
            "receipt_n" => Some(ReceiptField::N),
            "receipt_error" => Some(ReceiptField::Error),
            _ => None,
        };
        if let Some(field) = receipt {
            let receipt = self.at(path, 0, |b, p| b.expr(&args[0], p))?;
            return Ok(Ir::ReadReceipt { receipt: Box::new(receipt), field });
        }
        let role = match op {
            "use_budget" => Some(ExactRole::Budget),
            "use_goal" => Some(ExactRole::Goal),
            "use_evidence" => Some(ExactRole::Evidence),
            _ => None,
        };
        if let Some(role) = role {
            let value = self.at(path, 0, |b, p| b.expr(&args[0], p))?;
            return Ok(Ir::UseExact { role, value: Box::new(value) });
        }
        Ok(match op {
            "if" | "display_if" => {
                let mut a = self.args(args, path)?.into_iter().map(Box::new);
                let (cond, then, els) = (a.next().unwrap(), a.next().unwrap(), a.next().unwrap());
                if op == "if" {
                    Ir::If { cond, then, els }
                } else {
                    Ir::DisplayIf { cond, then, els }
                }
            }
            "tag" => Ir::Lit(Lit::Tag(self.text(op, &args[0])?)),
            "colour" => Ir::Lit(Lit::Colour(self.text(op, &args[0])?)),
            "target" => Ir::Lit(Lit::Target(self.text(op, &args[0])?)),
            "read_snapshot" if args.len() == 2 => {
                let (query, ty) = (self.text(op, &args[0])?, self.ty(&type_args[0])?);
                let arg = self.at(path, 0, |b, p| b.expr(&args[1], p))?;
                Ir::ReadSnapshotWith { query, arg: Box::new(arg), ty }
            }
            "read_snapshot" => Ir::ReadSnapshot { query: self.text(op, &args[0])?, ty: self.ty(&type_args[0])? },
            "publish_view" => Ir::PublishView { value: Box::new(self.at(path, 0, |b, p| b.expr(&args[0], p))?) },
            "write_workspace" => {
                let (record, ty) = (self.text(op, &args[0])?, self.ty(&type_args[0])?);
                let value = self.at(path, 0, |b, p| b.expr(&args[1], p))?;
                Ir::WriteWorkspace { record, ty, value: Box::new(value) }
            }
            "propose" => {
                let mut a = self.args(args, path)?.into_iter().map(Box::new);
                let (word, target) = (a.next().unwrap(), a.next().unwrap());
                Ir::Propose { word, target }
            }
            "used_steps" => Ir::Accounting(Accounting::UsedSteps),
            "unused_steps" => Ir::Accounting(Accounting::UnusedSteps),
            "budget_status" => Ir::Accounting(Accounting::Status),
            "declassified" => {
                Ir::Declassified { id: self.text(op, &args[0])?, ty: self.unqualified(op, &type_args[0], span)? }
            }
            "file_read" => {
                Ir::FileRead { path: self.text(op, &args[0])?, ty: self.unqualified(op, &type_args[0], span)? }
            }
            "file_write" => {
                let file = self.text(op, &args[0])?;
                let value = self.at(path, 0, |b, p| b.expr(&args[1], p))?;
                Ir::FileWrite { path: file, value: Box::new(value) }
            }
            "random_draw" => Ir::RandomDraw,
            "read_clock" => Ir::ReadClock,
            _ => return Err(self.err(span, format!("`{op}` is reserved and has no lowering"))),
        })
    }

    fn closure(&mut self, params: &[ParamDecl], result: &TypeExpr, body: &Block, path: &mut Vec<u32>) -> R<Ir> {
        let mut out = Vec::new();
        for p in params {
            let ty = self.ty(&p.ty)?;
            self.bind(&p.name)?;
            out.push(Param { name: p.name.text.clone(), ty });
        }
        let result = self.ty(result)?;
        let body = self.at(path, 0, |b, p| b.block(body, p))?;
        Ok(Ir::Closure { params: out, result, body: Box::new(body) })
    }

    /// A match on literals with a final `_` arm: the scrutinee at child 0, arm `i` at child `i + 1` and the `_` arm
    /// last.
    fn matching(&mut self, scrutinee: &Expr, arms: &[Arm], span: Span, path: &mut Vec<u32>) -> R<Ir> {
        if let Some(arm) = arms.iter().find(|a| matches!(a.pattern, Pattern::Binder { .. })) {
            return Err(self.err(arm.loc.0, "a verdict match waits for the IR's domain families (M4)"));
        }
        let Some((last, rest)) = arms.split_last().filter(|(last, _)| last.pattern == Pattern::Wildcard) else {
            return Err(self.err(span, "a match ends with a `_` arm"));
        };
        let mut lits = Vec::new();
        for arm in rest {
            let Pattern::Lit(e) = &arm.pattern else {
                return Err(self.err(arm.loc.0, "only the last arm of a match is `_`"));
            };
            lits.push(self.pattern(e)?);
        }
        let scrutinee = self.at(path, 0, |b, p| b.expr(scrutinee, p))?;
        let mut out = Vec::new();
        for (i, (arm, lit)) in rest.iter().zip(lits).enumerate() {
            out.push((lit, self.at(path, i + 1, |b, p| b.expr(&arm.body, p))?));
        }
        let default = self.at(path, rest.len() + 1, |b, p| b.expr(&last.body, p))?;
        Ok(Ir::Match { scrutinee: Box::new(scrutinee), arms: out, default: Box::new(default) })
    }

    /// A literal pattern: a literal, or `tag`, `colour` or `target` of a literal string.
    fn pattern(&self, e: &Expr) -> R<Lit> {
        if let Some(lit) = literal(&e.kind) {
            return Ok(lit);
        }
        if let ExprKind::Call { callee, type_args, args } = &e.kind {
            if let (ExprKind::Name(n), [arg], true) = (&callee.kind, args.as_slice(), type_args.is_empty()) {
                if let ExprKind::Text(s) = &arg.kind {
                    match n.as_str() {
                        "tag" => return Ok(Lit::Tag(s.clone())),
                        "colour" => return Ok(Lit::Colour(s.clone())),
                        "target" => return Ok(Lit::Target(s.clone())),
                        _ => {}
                    }
                }
            }
        }
        if let ExprKind::Menu(_) = e.kind {
            return Err(self.err(e.loc.0, MENU));
        }
        Err(self.err(e.loc.0, "a match arm's pattern is a literal, such as `3` or `tag(\"...\")`"))
    }

    fn ty(&self, t: &TypeExpr) -> R<QTy> {
        let exact = |ty| QTy { ty, display: false };
        match t {
            TypeExpr::Named { name, args, loc } => {
                let n = name.text.as_str();
                match n {
                    "Display" => {
                        let [inner] = args.as_slice() else {
                            return Err(self.err(loc.0, "`Display` takes one type argument"));
                        };
                        let q = self.ty(inner)?;
                        if q.display {
                            return Err(self.err(loc.0, "`Display` does not nest"));
                        }
                        Ok(QTy { ty: q.ty, display: true })
                    }
                    // `List<T>` (LP2 draft).
                    "List" | "Set" | "Map" if !self.is_generic(n) => {
                        let count = if n == "Map" { 2 } else { 1 };
                        if args.len() != count {
                            let wanted = if count == 1 { "one type argument" } else { "two type arguments" };
                            return Err(self.err(loc.0, format!("`{n}` takes {wanted}")));
                        }
                        let mut types = Vec::new();
                        for a in args {
                            types.push(self.ty(a)?);
                        }
                        Ok(exact(self.collection(n, types, loc.0)?))
                    }
                    "Approx" => {
                        let precision = match args.as_slice() {
                            [TypeExpr::Named { name, args, .. }] if args.is_empty() => match name.text.as_str() {
                                "P24" => Some(24),
                                "P53" => Some(53),
                                "P113" => Some(113),
                                _ => None,
                            },
                            _ => None,
                        };
                        let p = precision
                            .ok_or_else(|| self.err(loc.0, "`Approx` takes one precision: `P24`, `P53` or `P113`"))?;
                        Ok(exact(Ty::Approx(p)))
                    }
                    _ if BASE_TYPES.contains(&n) || self.is_generic(n) => {
                        if !args.is_empty() {
                            let message = format!(
                                "`{n}` takes no type arguments here; LX-8's domain types wait for the IR's domain \
                                 families (M4, M5)"
                            );
                            return Err(self.err(loc.0, message));
                        }
                        Ok(exact(match n {
                            "Unit" => Ty::Unit,
                            "Bool" => Ty::Bool,
                            "Nat" => Ty::Nat,
                            "Int" => Ty::Int,
                            "Rat" => Ty::Rat,
                            "Q5" => Ty::Q5,
                            "Text" => Ty::Text,
                            "Word" => Ty::Word,
                            "Target" => Ty::Target,
                            "Budget" => Ty::Budget,
                            "Status" => Ty::Status,
                            "Colour" => Ty::Colour,
                            "ApproxBool" => Ty::ApproxBool,
                            "Receipt" => Ty::Receipt,
                            "WordEqualityKey" => Ty::WordEqualityKey,
                            "CompressedWord" => Ty::CompressedWord,
                            "MenuWord" => Ty::MenuWord,
                            _ => match EndTest::of_type_name(n) {
                                Some(test) => Ty::EndTest(test),
                                None => Ty::Var(n.into()),
                            },
                        }))
                    }
                    _ => {
                        let message = format!(
                            "unknown type `{n}`; LX-8's domain types wait for the IR's domain families (M4, M5)"
                        );
                        Err(self.err(loc.0, message))
                    }
                }
            }
            TypeExpr::Fn { params, result, effects, .. } => {
                let params = params.iter().map(|p| self.ty(p)).collect::<R<Vec<_>>>()?;
                let (result, effects) = (self.ty(result)?, self.effects(effects)?);
                Ok(exact(Ty::Fn(Box::new(FnTy { params, result, effects }))))
            }
            TypeExpr::Record { fields, .. } => {
                let mut seen = BTreeSet::new();
                let mut out = Vec::new();
                for (name, t) in fields {
                    if !seen.insert(name.text.as_str()) {
                        return Err(self.err(name.loc.0, format!("field `{}` appears twice", name.text)));
                    }
                    out.push((name.text.clone(), self.ty(t)?));
                }
                Ok(exact(Ty::Record(out)))
            }
        }
    }

    /// An `effects` list (LG3.1, LG3.2, LG3.8): effect names, capability names and generic effect variables.
    fn effects(&self, names: &[Name]) -> R<EffectBound> {
        let mut bound = EffectBound::default();
        let mut seen = BTreeSet::new();
        for name in names {
            let n = name.text.as_str();
            if !seen.insert(n) {
                return Err(self.err(name.loc.0, format!("`{n}` is listed twice")));
            }
            let effect = match n {
                "snapshot" => Some(Effect::ReadSnapshot),
                "view" => Some(Effect::ChangeView),
                "workspace" => Some(Effect::ChangeWorkspace),
                "preview" => Some(Effect::PreparePreview),
                "budget" => Some(Effect::Budgets),
                "file" => Some(Effect::GrantedFile),
                "random" => Some(Effect::SeededRandomness),
                _ => None,
            };
            let cap = capability(n);
            if effect.is_none() && cap.is_none() {
                if !self.is_generic(n) {
                    let message = format!("`{n}` is not an effect, a capability or a generic parameter");
                    return Err(self.err(name.loc.0, message));
                }
                bound.vars.insert(n.into());
            }
            bound.effects.extend(effect);
            bound.capabilities.extend(cap);
        }
        Ok(bound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `BUILTINS` and `Body::builtin` name the same operations: each reserved name lowers when it is called with
    /// arguments of its shape. Lowering does not type the call; the checker does.
    #[test]
    fn every_reserved_operation_has_a_lowering() {
        for op in BUILTINS {
            let call = match op {
                "read_snapshot" | "declassified" | "file_read" => format!("{op}<Nat>(\"q\")"),
                "write_workspace" => format!("{op}<Nat>(\"r\", 1)"),
                "file_write" => format!("{op}(\"p\", 1)"),
                "tag" | "colour" | "target" => format!("{op}(\"x\")"),
                "if" | "display_if" => format!("{op}(true, 1, 2)"),
                "used_steps" | "unused_steps" | "budget_status" | "random_draw" | "read_clock" => format!("{op}()"),
                "exact_add" | "exact_less" | "exact_eq" | "and" | "or" | "approx_add" | "approx_less"
                | "approx_and" | "propose" | "index" | "push" | "concat" | "conjugate" | "commutator" | "compose"
                | "contains" | "insert" | "lookup" => format!("{op}(1, 2)"),
                "list" | "set" => format!("{op}<Nat>(1, 2)"),
                "map" => format!("{op}<Nat, Nat>(1, 2)"),
                "elements" | "keys" | "entries" => format!("{op}(1, ascending)"),
                "expand" => format!("{op}(1, 2, 3)"),
                _ => format!("{op}(1)"),
            };
            let source =
                format!("edition \"6cs-draft-1\";\nmodule m requires [];\n\nfn f() -> Nat effects [] {{\n    return {call};\n}}\n");
            let file = crate::parse(source.as_bytes());
            assert!(file.diagnostics.is_empty(), "{op}: {:#?}", file.diagnostics);
            if let Err(e) = lower(&[file]) {
                panic!("{op}: {e}");
            }
        }
    }
}
