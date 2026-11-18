//! Parsing with recovery (LG1.1 to LG1.3, LG1.5): LX-8's candidate A with the additions listed in `rust/README.md`.

use crate::ast::*;
use crate::code::*;
use crate::lex::{canonical, is_decimal, is_digest, lex, ratio, unescape, Kind, Token};
use crate::{sha256_hex, Diagnostic, RawContext, Span, EDITION};
use c600_ir::MENU_INDEX_LIMIT;

/// Names that begin a declaration in column 1. Recovery resumes at the next one (LG1.5).
const STARTS: [&str; 6] = ["fn", "view", "command", "contract", "import", "export"];

struct Problem {
    code: &'static str,
    message: String,
    span: Span,
}

impl From<Problem> for Diagnostic {
    fn from(p: Problem) -> Diagnostic {
        Diagnostic { code: p.code, message: p.message, span: Some(p.span), raw: None }
    }
}

type R<T> = Result<T, Problem>;

fn literal(span: Span, message: &str) -> Problem {
    Problem { code: INVALID_LITERAL, message: message.into(), span }
}

/// Parses one source file. A refused encoding, a missing or unsupported edition and a broken module header end the
/// parse: nothing after them is read, and no other edition is tried (LG1.2). A broken declaration becomes a recovery
/// hole, and parsing resumes at the next declaration in column 1 (LG1.5).
pub fn parse(bytes: &[u8]) -> ParsedFile {
    let mut file = ParsedFile::default();
    let source = match pinned(bytes) {
        Ok(source) => source,
        Err(diagnostic) => {
            file.diagnostics.push(diagnostic);
            return file;
        }
    };
    file.digest = Some(sha256_hex(bytes));
    let mut p = Parser { tokens: lex(source), pos: 0 };
    if let Err(problem) = p.header(&mut file) {
        file.diagnostics.push(problem.into());
        return file;
    }
    while p.tok().kind != Kind::Eof {
        let (position, start) = (p.pos, p.tok().span);
        match p.item() {
            Ok(item) => file.items.push(item),
            Err(problem) => {
                file.diagnostics.push(problem.into());
                p.recover(position);
                let hole = Span { end: p.tok().span.start, ..start };
                file.holes.push(hole);
                file.diagnostics.push(Diagnostic {
                    code: RECOVERY_GAP,
                    message: "declaration omitted; a draft with a recovery hole does not lower".into(),
                    span: Some(hole),
                    raw: None,
                });
            }
        }
    }
    file
}

/// The source text, if the bytes are UTF-8 with LF line ends and no byte-order mark (LG1.3). Otherwise ERR.10's
/// encoding diagnostic: no span, the raw input digest and the first offending byte interval.
fn pinned(bytes: &[u8]) -> Result<&str, Diagnostic> {
    let refuse = |start: usize, end: usize, message: &str| Diagnostic {
        code: INVALID_ENCODING,
        message: message.into(),
        span: None,
        raw: Some(RawContext { digest: sha256_hex(bytes), start, end }),
    };
    let text = std::str::from_utf8(bytes).map_err(|e| {
        let start = e.valid_up_to();
        refuse(start, start + e.error_len().unwrap_or(bytes.len() - start), "source is not UTF-8")
    })?;
    if text.starts_with('\u{feff}') {
        return Err(refuse(0, 3, "source starts with a byte-order mark"));
    }
    if let Some(i) = bytes.iter().position(|&b| b == b'\r') {
        return Err(refuse(i, i + 1, "source line ends are not pinned to LF"));
    }
    Ok(text)
}

fn is(t: &Token, text: &str) -> bool {
    matches!(t.kind, Kind::Id | Kind::Punct) && t.text == text
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn tok(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek(&self, n: usize) -> &Token {
        &self.tokens[(self.pos + n).min(self.tokens.len() - 1)]
    }

    fn take(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if t.kind != Kind::Eof {
            self.pos += 1;
        }
        t
    }

    fn at(&self, text: &str) -> bool {
        is(self.tok(), text)
    }

    fn accept(&mut self, text: &str) -> bool {
        let found = self.at(text);
        if found {
            self.take();
        }
        found
    }

    fn expect(&mut self, text: &str) -> R<Token> {
        if self.at(text) {
            Ok(self.take())
        } else {
            Err(self.unexpected(&format!("expected `{text}`")))
        }
    }

    fn unexpected(&self, what: &str) -> Problem {
        let t = self.tok();
        let found = if t.kind == Kind::Eof { "the end of the file".to_string() } else { format!("`{}`", t.text) };
        Problem { code: UNEXPECTED_TOKEN, message: format!("{what}, found {found}"), span: t.span }
    }

    /// The range from `start` to the end of the last token taken.
    fn since(&self, start: Span) -> Span {
        let end = self.tokens[self.pos.saturating_sub(1)].span.end;
        Span { end: end.max(start.start), ..start }
    }

    fn name(&mut self) -> R<Name> {
        if self.tok().kind != Kind::Id {
            return Err(self.unexpected("expected a name"));
        }
        let t = self.take();
        Ok(Name { text: t.text, loc: Loc(t.span) })
    }

    fn string(&mut self) -> R<(String, Span)> {
        if self.tok().kind != Kind::Str {
            return Err(self.unexpected("expected a quoted string"));
        }
        let t = self.take();
        match unescape(&t.text) {
            Some(s) => Ok((s, t.span)),
            None => {
                Err(literal(t.span, "a string takes JSON escapes only, and no raw control character or lone surrogate"))
            }
        }
    }

    fn natural(&mut self) -> R<String> {
        let t = self.tok();
        let n = if t.kind == Kind::Int { canonical(&t.text) } else { String::new() };
        if n.is_empty() || n.starts_with('-') {
            return Err(literal(t.span, "expected an exact nonnegative integer"));
        }
        self.take();
        Ok(n)
    }

    /// Items up to `close`, separated by commas, without a trailing comma.
    fn separated<T>(&mut self, close: &str, mut item: impl FnMut(&mut Self) -> R<T>) -> R<Vec<T>> {
        let mut out = Vec::new();
        if !self.at(close) {
            out.push(item(self)?);
            while self.accept(",") {
                out.push(item(self)?);
            }
        }
        self.expect(close)?;
        Ok(out)
    }

    fn names(&mut self) -> R<Vec<Name>> {
        self.expect("[")?;
        self.separated("]", Self::name)
    }

    /// `edition "6cs-draft-1";` and `module name requires [..];`. The edition is refused as soon as its string is read.
    fn header(&mut self, file: &mut ParsedFile) -> R<()> {
        if !self.at("edition") {
            let first = self.tok().span;
            return Err(Problem {
                code: EDITION_MISSING,
                message: format!("a file starts with the edition header `edition \"{EDITION}\";`"),
                span: Span { end: first.start, ..first },
            });
        }
        self.take();
        let (edition, span) = self.string()?;
        file.edition = Some(edition.clone());
        if edition != EDITION {
            return Err(Problem {
                code: EDITION_UNSUPPORTED,
                message: format!("edition `{edition}` is not supported; this parser reads `{EDITION}` only"),
                span,
            });
        }
        self.expect(";")?;
        let start = self.tok().span;
        if self.accept("stage") {
            // A stage module (LG5.12, RPI.68; LP2 draft): `stage vertex name ops 12 overdraw 1 inputs [a: 4];`.
            if !self.at("vertex") && !self.at("fragment") {
                return Err(self.unexpected("expected the stage's kind, `vertex` or `fragment`"));
            }
            let kind = self.name()?;
            let name = self.name()?;
            self.expect("ops")?;
            let ops = self.natural()?;
            self.expect("overdraw")?;
            let overdraw = self.natural()?;
            let mut inputs = Vec::new();
            if self.accept("inputs") {
                self.expect("[")?;
                inputs = self.separated("]", |p| {
                    let input = p.name()?;
                    p.expect(":")?;
                    Ok((input, p.natural()?))
                })?;
            }
            self.expect(";")?;
            let stage = Some(StageHeader { kind, ops, overdraw, inputs });
            file.module = Some(ModuleHeader { name, requires: Vec::new(), stage, loc: Loc(self.since(start)) });
            return Ok(());
        }
        self.expect("module")?;
        let name = self.name()?;
        self.expect("requires")?;
        let requires = self.names()?;
        self.expect(";")?;
        file.module = Some(ModuleHeader { name, requires, stage: None, loc: Loc(self.since(start)) });
        Ok(())
    }

    fn item(&mut self) -> R<Item> {
        let start = self.tok().span;
        if start.column != 1 {
            return Err(self.unexpected("a declaration starts in column 1"));
        }
        if self.accept("import") {
            let alias = self.name()?;
            let (digest, span) = self.string()?;
            if !is_digest(&digest) {
                return Err(literal(span, "an import names its module by the SHA-256 of its source, in lowercase hex"));
            }
            self.expect(";")?;
            return Ok(Item::Import(Import { alias, digest, loc: Loc(self.since(start)) }));
        }
        let exported = self.accept("export");
        let kind = if self.accept("fn") {
            DeclKind::Fn
        } else if self.accept("view") {
            DeclKind::View
        } else if self.accept("command") {
            DeclKind::Command
        } else if self.accept("contract") {
            DeclKind::Contract
        } else {
            return Err(self.unexpected("expected `fn`, `view`, `command`, `contract` or `import`"));
        };
        let name = self.name()?;
        let mut generics = Vec::new();
        if self.accept("<") {
            if self.at(">") {
                return Err(self.unexpected("expected a generic parameter"));
            }
            generics = self.separated(">", Self::name)?;
        }
        let params = self.params()?;
        self.expect("->")?;
        let result = self.ty()?;
        self.expect("effects")?;
        let effects = self.names()?;
        let charge = if self.accept("charge") { Some(self.charge()?) } else { None };
        let metadata = if self.at("row") {
            if kind != DeclKind::Command {
                return Err(self.unexpected("only a command has command metadata"));
            }
            Some(self.metadata()?)
        } else {
            None
        };
        let body = self.block()?;
        Ok(Item::Decl(Box::new(Decl {
            exported,
            kind,
            name,
            generics,
            params,
            result,
            effects,
            charge,
            metadata,
            body,
            loc: Loc(self.since(start)),
        })))
    }

    fn params(&mut self) -> R<Vec<ParamDecl>> {
        self.expect("(")?;
        self.separated(")", |p| {
            let name = p.name()?;
            p.expect(":")?;
            Ok(ParamDecl { name, ty: p.ty()? })
        })
    }

    fn charge(&mut self) -> R<Vec<String>> {
        let open = self.expect("[")?.span;
        let values = self.separated("]", Self::natural)?;
        if values.len() != 4 {
            return Err(Problem {
                code: UNEXPECTED_TOKEN,
                message: "a charge lists four naturals: steps, kernel, memory and output".into(),
                span: self.since(open),
            });
        }
        Ok(values)
    }

    fn metadata(&mut self) -> R<Metadata> {
        self.expect("row")?;
        let (row, _) = self.string()?;
        self.expect("kind")?;
        let kind = self.name()?;
        self.expect("contexts")?;
        let contexts = self.names()?;
        self.expect("permits")?;
        let permits = self.names()?;
        Ok(Metadata { row, kind, contexts, permits })
    }

    /// `{ let a = e; ... return e; }`: every binding comes before the single return.
    fn block(&mut self) -> R<Block> {
        let start = self.expect("{")?.span;
        let mut lets = Vec::new();
        while self.at("let") {
            let at = self.take().span;
            let name = self.name()?;
            self.expect("=")?;
            let value = self.expr()?;
            self.expect(";")?;
            lets.push(LetStmt { name, value, loc: Loc(self.since(at)) });
        }
        self.expect("return")?;
        let result = self.expr()?;
        self.expect(";")?;
        self.expect("}")?;
        Ok(Block { lets, result, loc: Loc(self.since(start)) })
    }

    fn ty(&mut self) -> R<TypeExpr> {
        let start = self.tok().span;
        if self.at("fn") && is(self.peek(1), "(") {
            self.take();
            self.take();
            let params = self.separated(")", Self::ty)?;
            self.expect("->")?;
            let result = Box::new(self.ty()?);
            self.expect("effects")?;
            let effects = self.names()?;
            return Ok(TypeExpr::Fn { params, result, effects, loc: Loc(self.since(start)) });
        }
        if self.at("record") && is(self.peek(1), "{") {
            self.take();
            self.take();
            let fields = self.separated("}", |p| {
                let name = p.name()?;
                p.expect(":")?;
                Ok((name, p.ty()?))
            })?;
            return Ok(TypeExpr::Record { fields, loc: Loc(self.since(start)) });
        }
        let name = self.name()?;
        let mut args = Vec::new();
        if self.accept("<") {
            if self.at(">") {
                return Err(self.unexpected("expected a type argument"));
            }
            args = self.separated(">", Self::ty)?;
        }
        Ok(TypeExpr::Named { name, args, loc: Loc(self.since(start)) })
    }

    fn expr(&mut self) -> R<Expr> {
        if !self.at("match") {
            return self.postfix(false);
        }
        let start = self.take().span;
        let scrutinee = Box::new(self.postfix(true)?);
        self.expect("{")?;
        let mut arms = Vec::new();
        while !self.at("}") {
            arms.push(self.arm()?);
            self.expect(";")?;
        }
        self.expect("}")?;
        Ok(Expr { kind: ExprKind::Match { scrutinee, arms }, loc: Loc(self.since(start)) })
    }

    fn arm(&mut self) -> R<Arm> {
        let start = self.tok().span;
        let pattern = if self.at("_") {
            self.take();
            Pattern::Wildcard
        } else if self.tok().kind == Kind::Id
            && is(self.peek(1), "(")
            && self.peek(2).kind == Kind::Id
            && is(self.peek(3), ")")
        {
            let tag = self.name()?;
            self.take();
            let binder = self.name()?;
            self.take();
            Pattern::Binder { tag, binder }
        } else {
            Pattern::Lit(self.postfix(false)?)
        };
        self.expect("=>")?;
        let body = self.expr()?;
        Ok(Arm { pattern, body, loc: Loc(self.since(start)) })
    }

    /// An atom followed by field accesses and calls. In a match scrutinee, `record {` is not a record literal.
    fn postfix(&mut self, scrutinee: bool) -> R<Expr> {
        let start = self.tok().span;
        let mut e = self.atom(scrutinee)?;
        loop {
            let kind = if self.accept(".") {
                ExprKind::Field { value: Box::new(e), field: self.name()? }
            } else if self.accept("(") {
                let args = self.separated(")", Self::expr)?;
                ExprKind::Call { callee: Box::new(e), type_args: Vec::new(), args }
            } else {
                return Ok(e);
            };
            e = Expr { kind, loc: Loc(self.since(start)) };
        }
    }

    fn atom(&mut self, scrutinee: bool) -> R<Expr> {
        let t = self.tok().clone();
        let start = t.span;
        let kind = match t.kind {
            Kind::Int => {
                self.take();
                ExprKind::Int(canonical(&t.text))
            }
            Kind::Rat => {
                let (num, den) =
                    ratio(&t.text).ok_or_else(|| literal(t.span, "a rational has a nonzero denominator"))?;
                self.take();
                ExprKind::Rat { num, den }
            }
            Kind::BadNumber => {
                return Err(literal(t.span, "a decimal number is approximate and needs a precision, as in ~53\"0.5\""))
            }
            Kind::Str => ExprKind::Text(self.string()?.0),
            Kind::Punct if t.text == "~" => {
                self.take();
                let precision = self.natural()?;
                let (decimal, _) = self.string()?;
                let span = self.since(start);
                let precision = match precision.as_str() {
                    "24" => 24,
                    "53" => 53,
                    "113" => 113,
                    _ => return Err(literal(span, "an approximate literal has 24, 53 or 113 bits of precision")),
                };
                if !is_decimal(&decimal) {
                    return Err(literal(span, "an approximate literal holds a finite decimal number"));
                }
                ExprKind::Approx { precision, decimal }
            }
            Kind::Punct if t.text == "(" => {
                self.take();
                if !self.accept(")") {
                    let inner = self.expr()?;
                    self.expect(")")?;
                    return Ok(inner);
                }
                ExprKind::Unit
            }
            Kind::Id => match t.text.as_str() {
                "true" | "false" => {
                    self.take();
                    ExprKind::Bool(t.text == "true")
                }
                "q5" => {
                    self.take();
                    self.expect("(")?;
                    let one = self.coefficient()?;
                    self.expect(",")?;
                    let sqrt5 = self.coefficient()?;
                    self.expect(")")?;
                    ExprKind::Q5 { one, sqrt5 }
                }
                "word" => {
                    self.take();
                    self.expect("[")?;
                    ExprKind::Word(self.separated("]", Self::letter)?)
                }
                "menu" => {
                    self.take();
                    self.expect("[")?;
                    ExprKind::Menu(self.separated("]", Self::menu_letter)?)
                }
                "record" if !scrutinee && is(self.peek(1), "{") => {
                    self.take();
                    self.take();
                    ExprKind::Record(self.separated("}", |p| {
                        let name = p.name()?;
                        p.expect("=")?;
                        Ok((name, p.expr()?))
                    })?)
                }
                "fn" if is(self.peek(1), "(") => {
                    self.take();
                    let params = self.params()?;
                    self.expect("->")?;
                    let result = self.ty()?;
                    ExprKind::Closure { params, result, body: Box::new(self.block()?) }
                }
                "fn" | "match" => return Err(self.unexpected("expected an expression")),
                _ => {
                    self.take();
                    if !self.accept("<") {
                        ExprKind::Name(t.text)
                    } else {
                        if self.at(">") {
                            return Err(self.unexpected("expected a type argument"));
                        }
                        let type_args = self.separated(">", Self::ty)?;
                        let callee = Box::new(Expr { kind: ExprKind::Name(t.text), loc: Loc(t.span) });
                        if !self.accept("(") {
                            return Err(self.unexpected("type arguments belong to a call: expected `(`"));
                        }
                        let args = self.separated(")", Self::expr)?;
                        ExprKind::Call { callee, type_args, args }
                    }
                }
            },
            _ => return Err(self.unexpected("expected an expression")),
        };
        Ok(Expr { kind, loc: Loc(self.since(start)) })
    }

    fn coefficient(&mut self) -> R<(String, String)> {
        let t = self.tok();
        let value = if t.kind == Kind::Rat { ratio(&t.text) } else { None };
        let value = value.ok_or_else(|| literal(t.span, "q5 takes two rational coefficients, as in q5(1/2, 1/2)"))?;
        self.take();
        Ok(value)
    }

    fn letter(&mut self) -> R<i32> {
        let t = self.tok();
        let n = if t.kind == Kind::Int { canonical(&t.text) } else { String::new() };
        let magnitude = n.trim_start_matches('-');
        let value = if magnitude.len() <= 4 { magnitude.parse::<i32>().unwrap_or(0) } else { 0 };
        if !(1..=1200).contains(&value) {
            return Err(literal(
                t.span,
                "a word letter is a retained generator: a signed integer of magnitude 1 to 1200",
            ));
        }
        self.take();
        Ok(if n.starts_with('-') { -value } else { value })
    }

    /// A menu letter: `cap:item`, two naturals below [`MENU_INDEX_LIMIT`] (LB1.8; LP2 draft), or LX-8's fixture
    /// `grip:rotation`, a grip below 120 and `spin`, `unspin` or `identity`, which does not lower.
    fn menu_letter(&mut self) -> R<(u32, String)> {
        let start = self.tok().span;
        let index = |n: &str| if n.len() <= 3 { n.parse::<u32>().unwrap_or(u32::MAX) } else { u32::MAX };
        let cap = index(&self.natural()?);
        self.expect(":")?;
        if self.tok().kind == Kind::Int {
            let item = self.natural()?;
            if cap >= MENU_INDEX_LIMIT || index(&item) >= MENU_INDEX_LIMIT {
                return Err(literal(self.since(start), "a menu letter is cap:item, two naturals below 600"));
            }
            return Ok((cap, item));
        }
        let rotation = self.name()?;
        if cap >= 120 || !["spin", "unspin", "identity"].contains(&rotation.text.as_str()) {
            return Err(literal(
                self.since(start),
                "a menu letter is cap:item, two naturals below 600, or LX-8's fixture grip:rotation, a grip below \
                 120 and spin, unspin or identity",
            ));
        }
        Ok((cap, rotation.text))
    }

    /// Skips to the next declaration in column 1, taking at least one token.
    fn recover(&mut self, position: usize) {
        if self.pos <= position {
            self.take();
        }
        while self.tok().kind != Kind::Eof {
            let t = self.tok();
            if t.span.column == 1 && t.kind == Kind::Id && STARTS.contains(&t.text.as_str()) {
                break;
            }
            self.take();
        }
    }
}
