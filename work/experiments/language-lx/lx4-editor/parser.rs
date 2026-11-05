// LX-8 lexer and candidate-a parser port. b/c adapters validate the original
// broken-draft corpus only; candidate-a is the sole measured surface.
use crate::json::{self, obj, J};
use crate::{Int, sha256};

#[derive(Clone)]
struct Token { kind: &'static str, text: String, start: usize, end: usize, line: usize, column: usize }
impl Token {
    fn span(&self) -> J { obj(&[("start", self.start.into()), ("end", self.end.into()), ("line", self.line.into()), ("column", self.column.into())]) }
    fn error(&self, message: &str, code: &str) -> J {
        obj(&[("code", format!("LG1:syntax.{code}").into()), ("message", message.into()), ("source", self.span())])
    }
}
fn id_start(c: u8) -> bool { c.is_ascii_alphabetic() || c == b'_' }
fn digit_end(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && b[i].is_ascii_digit() { i += 1; } i
}
fn signed_end(b: &[u8], mut i: usize) -> usize {
    if b.get(i) == Some(&b'+') || b.get(i) == Some(&b'-') { i += 1; }
    let end = digit_end(b, i); if end == i { 0 } else { end }
}
fn exponent_end(b: &[u8], i: usize) -> usize {
    if b.get(i) == Some(&b'e') || b.get(i) == Some(&b'E') {
        let end = signed_end(b, i + 1); if end != 0 { return end; }
    }
    i
}
fn decimal_text(s: &str) -> bool {
    let b = s.as_bytes(); let mut i = signed_end(b, 0);
    if i == 0 { return false; }
    if b.get(i) == Some(&b'.') {
        let end = digit_end(b, i + 1); if end == i + 1 { return false; } i = end;
    }
    exponent_end(b, i) == b.len()
}
fn lex(s: &str, layout: bool) -> Vec<Token> {
    let b = s.as_bytes(); let mut i = 0; let mut line = 1; let mut column = 1; let mut out = vec![];
    while i < b.len() {
        let start = i; let mut kind = "BAD"; i += s[start..].chars().next().unwrap().len_utf8();
        match b[start] {
            b' ' | b'\t' => {
                while i < b.len() && (b[i] == b' ' || b[i] == b'\t') { i += 1; }
                kind = if layout && b[start..i].contains(&b'\t') { "BAD" } else { "SPACE" };
            }
            b'#' => { while i < b.len() && b[i] != b'\n' { i += 1; } kind = "COMMENT"; }
            b'\n' => kind = "NL",
            b'"' => {
                let mut j = i;
                while j < b.len() && b[j] != b'\n' {
                    if b[j] == b'"' { i = j + 1; kind = "STRING"; break; }
                    if b[j] == b'\\' {
                        j += 1;
                        if j == b.len() || b[j] == b'\n' { break; }
                    }
                    j += s[j..].chars().next().unwrap().len_utf8();
                }
            }
            c if c.is_ascii_digit() || ((c == b'+' || c == b'-') && b.get(i).is_some_and(u8::is_ascii_digit)) => {
                i = signed_end(b, start); kind = "INT";
                if b.get(i) == Some(&b'/') && signed_end(b, i + 1) != 0 { i = signed_end(b, i + 1); kind = "RAT"; }
                else if b.get(i) == Some(&b'.') { i = exponent_end(b, digit_end(b, i + 1)); kind = "BAD_NUMBER"; }
            }
            c if id_start(c) => { while i < b.len() && (id_start(b[i]) || b[i].is_ascii_digit()) { i += 1; } kind = "ID"; }
            _ => {
                if ["->", "=>", ":=", "<-", "|>"].iter().any(|p| s[start..].starts_with(p)) { i = start + 2; kind = "PUNCT"; }
                else if b"{}()[],;:<>!=@~|".contains(&b[start]) { kind = "PUNCT"; }
            }
        }
        if kind != "SPACE" && kind != "COMMENT" && (kind != "NL" || layout) {
            out.push(Token { kind, text: s[start..i].into(), start, end: i, line, column });
        }
        if kind == "NL" { line += 1; column = 1; }
        else { column += s[start..i].chars().count(); }
    }
    out.push(Token { kind: "EOF", text: String::new(), start: i, end: i, line, column }); out
}
type P<T> = Result<T, J>;
struct Parser { tokens: Vec<Token>, pos: usize, surface: char }
impl Parser {
    fn t(&self) -> &Token { &self.tokens[self.pos] }
    fn take(&mut self) -> Token { let t = self.t().clone(); if t.kind != "EOF" { self.pos += 1; } t }
    fn at(&self, s: &str) -> bool { self.t().text == s }
    fn accept(&mut self, s: &str) -> bool { if self.at(s) { self.take(); true } else { false } }
    fn err(&self, message: &str) -> J { self.t().error(message, "unexpected_token") }
    fn expect(&mut self, s: &str) -> P<Token> {
        if !self.at(s) { return Err(self.err(&format!("expected '{s}'"))); } Ok(self.take())
    }
    fn name(&mut self) -> P<String> {
        if self.t().kind != "ID" { return Err(self.err("expected an identifier")); } Ok(self.take().text)
    }
    fn string(&mut self) -> P<J> {
        if self.t().kind != "STRING" { return Err(self.err("expected a quoted string")); }
        let t = self.take(); json::parse(&t.text).map_err(|_| t.error("invalid string literal", "invalid_literal"))
    }
    fn natural(&mut self) -> P<J> {
        let t = self.t().clone();
        if t.kind != "INT" { return Err(t.error("expected an exact nonnegative integer", "invalid_literal")); }
        self.take(); let n = Int::parse(&t.text);
        if n.sign < 0 { return Err(t.error("negative count", "invalid_literal")); } Ok(n.decimal().into())
    }
    fn separated<F>(&mut self, close: &str, mut item: F) -> P<Vec<J>> where F: FnMut(&mut Self) -> P<J> {
        let mut values = vec![];
        if !self.at(close) {
            values.push(item(self)?);
            while self.accept(",") { values.push(item(self)?); }
        }
        self.expect(close)?; Ok(values)
    }
    fn names(&mut self) -> P<J> { self.expect("[")?; Ok(self.separated("]", |p| Ok(p.name()?.into()))?.into()) }
    fn charges(&mut self) -> P<J> {
        self.expect("[")?; let v = self.separated("]", Self::natural)?;
        if v.len() != 4 { return Err(self.err("charge needs steps, kernel, memory, output")); }
        Ok(obj(&[("steps", v[0].clone()), ("kernel", v[1].clone()), ("memory", v[2].clone()), ("output", v[3].clone())]))
    }
    fn type_name(&mut self) -> P<String> {
        let mut name = self.name()?;
        if self.accept("<") {
            let a = self.separated(">", |p| Ok(p.type_name()?.into()))?;
            if a.is_empty() { return Err(self.err("empty type argument list")); }
            name += &format!("<{}>", a.iter().map(J::s).collect::<Vec<_>>().join(","));
        }
        Ok(name)
    }
    fn params(&mut self) -> P<J> {
        self.expect("(")?;
        Ok(self.separated(")", |p| { let n = p.name()?; p.expect(":")?; Ok(obj(&[("name", n.into()), ("type", p.type_name()?.into())])) })?.into())
    }
    fn node(&self, start: &Token, kind: &str, fields: &[(&str, J)]) -> J {
        let mut n = obj(fields); let mut span = start.span();
        span.set("end", self.tokens[self.pos.saturating_sub(1)].end.into());
        n.set("kind", kind.into()); n.set("source", span); n
    }
    fn atom(&mut self) -> P<J> {
        let t = self.t().clone();
        match t.kind {
            "INT" => { self.take(); return Ok(self.node(&t, "int", &[("value", Int::parse(&t.text).decimal().into())])); }
            "RAT" => {
                self.take(); let (a, b) = t.text.split_once('/').unwrap(); let a = Int::parse(a); let b = Int::parse(b);
                if b.sign == 0 { return Err(t.error("zero denominator", "invalid_literal")); }
                let mut g = a.gcd(&b); g.sign = b.sign;
                return Ok(self.node(&t, "rat", &[("value", vec![a.div(&g).decimal().into(), b.div(&g).decimal().into()].into())]));
            }
            "STRING" => { let v = self.string()?; return Ok(self.node(&t, "text", &[("value", v)])); }
            "BAD_NUMBER" => return Err(t.error("approximation needs a precision marker", "invalid_literal")),
            _ => (),
        }
        if self.accept("~") {
            let precision = self.natural()?; let value = self.string()?;
            if !["24", "53", "113"].contains(&precision.s()) || !decimal_text(value.s()) {
                return Err(t.error("invalid approximate precision or decimal", "invalid_literal"));
            }
            return Ok(self.node(&t, "approx", &[("value", obj(&[("decimal", value), ("precision_bits", precision)]))]));
        }
        if self.accept("q5") {
            self.expect("(")?; let a = self.atom()?; self.expect(",")?; let b = self.atom()?; self.expect(")")?;
            if a["kind"].s() != "rat" || b["kind"].s() != "rat" { return Err(t.error("q5 needs two rational coefficients", "invalid_literal")); }
            return Ok(self.node(&t, "q5", &[("value", obj(&[("one", a["value"].clone()), ("sqrt5", b["value"].clone())]))]));
        }
        if self.at("word") || self.at("menu") {
            let kind = self.take().text; self.expect("[")?;
            let v = self.separated("]", |p| {
                let start = p.t().clone();
                if kind == "word" {
                    let n = if start.kind == "INT" { start.text.parse::<i64>().unwrap_or(0) } else { 0 };
                    if !(1..=1200).contains(&n.unsigned_abs()) { return Err(start.error("retained generator outside 1..1200", "invalid_literal")); }
                    p.take(); return Ok(n.to_string().into());
                }
                let grip = p.natural()?; p.expect(":")?; let rotation = p.name()?;
                if grip.s().parse::<u32>().unwrap_or(120) >= 120 || !["spin", "unspin", "identity"].contains(&rotation.as_str()) {
                    return Err(start.error("unknown fixture menu letter", "invalid_literal"));
                }
                Ok(obj(&[("grip", grip), ("rotation", rotation.into())]))
            })?;
            return Ok(self.node(&t, &kind, &[("value", v.into())]));
        }
        if self.accept("(") { let n = self.expression()?; self.expect(")")?; return Ok(n); }
        let name = self.name()?;
        if self.accept("(") {
            let args = self.separated(")", Self::expression)?;
            return Ok(self.node(&t, "call", &[("name", name.into()), ("args", args.into())]));
        }
        Ok(self.node(&t, "ref", &[("name", name.into())]))
    }
    fn arm(&mut self) -> P<J> {
        let start = self.t().clone(); let tag = self.name()?; self.expect("(")?; let binder = self.name()?;
        self.expect(")")?; self.expect("=>")?;
        Ok(obj(&[("tag", tag.into()), ("binder", binder.into()), ("body", self.expression()?), ("source", start.span())]))
    }
    fn blanks(&mut self) { while self.t().kind == "NL" { self.take(); } }
    fn line_end(&mut self) -> P<()> {
        if self.t().kind != "NL" { return Err(self.err("expected end of line")); } self.blanks(); Ok(())
    }
    fn indent(&self, column: usize) -> P<()> {
        if self.t().column != column { Err(self.err(&format!("expected column {column}"))) } else { Ok(()) }
    }
    fn expression(&mut self) -> P<J> {
        let start = self.t().clone();
        let keyword = match self.surface { 'a' => "match", 'b' => "choose", _ => "case" };
        let mut node = if self.accept(keyword) {
            let subject = self.atom()?; let mut arms = vec![];
            if self.surface == 'a' {
                self.expect("{")?;
                while !self.at("}") { arms.push(self.arm()?); self.expect(";")?; } self.expect("}")?;
            } else if self.surface == 'b' {
                self.expect(":")?; self.line_end()?;
                while self.t().column == 9 {
                    arms.push(self.arm()?);
                    if self.t().kind != "NL" { return Err(self.err("expected case line end")); }
                    let pos = self.pos; self.blanks();
                    if self.t().column != 9 { self.pos = pos; break; }
                }
                if arms.is_empty() { return Err(self.err("expected indented cases")); }
            } else {
                self.expect("of")?; arms.push(self.arm()?);
                while self.accept("|") { arms.push(self.arm()?); } self.expect("end")?;
            }
            self.node(&start, "match", &[("subject", subject), ("arms", arms.into())])
        } else { self.atom()? };
        if self.surface == 'c' {
            while self.accept("|>") {
                let name = self.name()?; self.expect("(")?;
                let mut args = vec![node]; args.extend(self.separated(")", Self::expression)?);
                node = self.node(&start, "call", &[("name", name.into()), ("args", args.into())]);
            }
        }
        Ok(node)
    }
    fn header(&mut self) -> P<J> {
        if self.surface == 'c' { self.expect("@")?; }
        self.expect("edition")?;
        if self.surface == 'b' { self.expect(":")?; }
        if self.surface == 'c' { self.expect("(")?; }
        let value = self.string()?;
        if self.surface == 'c' { self.expect(")")?; }
        if self.surface == 'b' { self.line_end()?; } else { self.expect(";")?; }
        Ok(value)
    }
    fn module(&mut self) -> P<(J, J)> {
        if self.surface == 'c' { self.expect("@")?; }
        self.expect("module")?;
        if self.surface == 'c' { self.expect("(")?; }
        let name = self.name()?;
        self.expect(if self.surface == 'c' { "," } else { "requires" })?;
        let effects = self.names()?;
        if self.surface == 'c' { self.expect(")")?; }
        if self.surface == 'b' { self.line_end()?; } else { self.expect(";")?; }
        Ok((name.into(), effects))
    }
    fn metadata(&mut self) -> P<J> {
        self.expect("row")?; let row = self.string()?; self.expect("kind")?; let kind = self.name()?;
        self.expect("contexts")?; let contexts = self.names()?; self.expect("permits")?; let permits = self.names()?;
        Ok(obj(&[("row", row), ("kind", kind.into()), ("contexts", contexts), ("permits", permits)]))
    }
    fn declaration(&mut self) -> P<J> {
        let start = self.t().clone(); let surface = self.surface;
        let import = match surface { 'a' => "import", 'b' => "use", _ => "@" };
        if self.accept(import) {
            if surface == 'c' { self.expect("use")?; self.expect("(")?; }
            let name = self.name()?;
            if surface == 'b' { self.expect("from")?; } else if surface == 'c' { self.expect(",")?; }
            let digest = self.string()?;
            if surface == 'c' { self.expect(")")?; }
            if surface == 'b' { self.line_end()?; } else { self.expect(";")?; }
            return Ok(self.node(&start, "import", &[("name", name.into()), ("digest", digest)]));
        }
        let mut kind = self.name()?;
        let allowed = match surface { 'a' => ["fn", "command", "view"], 'b' => ["function", "command", "view"], _ => ["fn", "cmd", "view"] };
        if !allowed.contains(&kind.as_str()) { self.expect(&format!("{}, {} or {}", allowed[0], allowed[1], allowed[2]))?; }
        let name = self.name()?; let params = self.params()?;
        self.expect(match surface { 'a' => "->", 'b' => "returns", _ => ":" })?;
        let result = self.type_name()?;
        self.expect(match surface { 'a' => "effects", 'b' => "using", _ => "!" })?;
        let effects = self.names()?; self.expect(if surface == 'c' { "@" } else { "charge" })?;
        let charges = self.charges()?; let mut metadata = J::Null;
        if kind == "command" { metadata = self.metadata()?; }
        else if kind == "cmd" {
            self.expect("with")?; self.expect("(")?; let row = self.string()?; self.expect(",")?;
            let k = self.name()?; self.expect(",")?; let contexts = self.names()?; self.expect(",")?;
            let permits = self.names()?; self.expect(")")?;
            metadata = obj(&[("row", row), ("kind", k.into()), ("contexts", contexts), ("permits", permits)]);
        }
        self.expect(match surface { 'a' => "{", 'b' => ":", _ => "=" })?;
        if surface == 'b' { self.line_end()?; }
        let mut bindings = vec![];
        loop {
            if surface == 'b' { if self.at("yield") { break; } self.indent(5)?; }
            else if !self.accept(if surface == 'a' { "let" } else { "bind" }) { break; }
            let binding = self.name()?; self.expect(match surface { 'a' => "=", 'b' => ":=", _ => "<-" })?;
            bindings.push(obj(&[("name", binding.into()), ("value", self.expression()?)]));
            if surface == 'b' { self.line_end()?; } else { self.expect(if surface == 'a' { ";" } else { "in" })?; }
        }
        if surface == 'a' { self.expect("return")?; }
        else if surface == 'b' { self.indent(5)?; self.expect("yield")?; }
        let body = self.expression()?;
        if surface == 'b' { self.line_end()?; } else { self.expect(";")?; }
        if surface == 'a' { self.expect("}")?; }
        if kind == "fn" { kind = "function".into(); } else if kind == "cmd" { kind = "command".into(); }
        Ok(self.node(&start, &kind, &[("name", name.into()), ("params", params), ("result", result.into()),
            ("effects", effects), ("charges", charges), ("metadata", metadata), ("bindings", bindings.into()), ("body", body)]))
    }
    fn declaration_start(&self) -> bool {
        let starts = match self.surface { 'a' => ["fn", "command", "view", "import"], 'b' => ["function", "command", "view", "use"], _ => ["fn", "cmd", "view", "@"] };
        self.t().column == 1 && starts.contains(&self.t().text.as_str())
    }
}

pub fn parse(data: &[u8], surface: char) -> J {
    let mut result = obj(&[("edition", J::Null), ("module", J::Null), ("requires", vec![].into()),
        ("imports", vec![].into()), ("declarations", vec![].into()), ("diagnostics", vec![].into()),
        ("holes", vec![].into()), ("digest", sha256(data).into())]);
    let source = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(e) => {
            let at = e.valid_up_to(); let prefix = std::str::from_utf8(&data[..at]).unwrap();
            let t = Token { kind: "BAD", text: String::new(), start: at, end: at + e.error_len().unwrap_or(data.len() - at),
                line: prefix.bytes().filter(|c| *c == b'\n').count() + 1, column: prefix.rsplit('\n').next().unwrap().chars().count() + 1 };
            result.push("diagnostics", t.error("source is not UTF-8", "invalid_encoding")); return result;
        }
    };
    if source.contains('\r') || source.starts_with('\u{feff}') {
        let at = source.find('\r').unwrap_or(0); let prefix = &source[..at];
        let t = Token { kind: "BAD", text: String::new(), start: at, end: at + 1,
            line: prefix.bytes().filter(|c| *c == b'\n').count() + 1, column: prefix.rsplit('\n').next().unwrap().chars().count() + 1 };
        result.push("diagnostics", t.error("source must be LF UTF-8 without BOM", "invalid_encoding")); return result;
    }
    let mut p = Parser { tokens: lex(source, surface == 'b'), pos: 0, surface }; p.blanks();
    let header = (|| -> P<()> {
        let t = p.t().clone();
        let present = if surface == 'c' { p.at("@") && p.tokens.get(p.pos + 1).is_some_and(|t| t.text == "edition") } else { p.at("edition") };
        if !present { return Err(t.error("edition header required", "edition_missing")); }
        let edition = p.header()?; result.set("edition", edition.clone());
        if edition.s() != "lx8-0" { return Err(t.error("unsupported edition", "edition_unsupported")); }
        let (name, effects) = p.module()?; result.set("module", name); result.set("requires", effects); Ok(())
    })();
    if let Err(error) = header { result.push("diagnostics", error); return result; }
    while p.t().kind != "EOF" {
        p.blanks(); if p.t().kind == "EOF" { break; }
        let pos = p.pos; let start = p.t().clone();
        let declaration = if start.column != 1 { Err(p.err("declaration must start in column 1")) } else { p.declaration() };
        match declaration {
            Ok(d) => result.push(if d["kind"].s() == "import" { "imports" } else { "declarations" }, d),
            Err(error) => {
                result.push("diagnostics", error);
                if p.pos <= pos { p.take(); }
                while p.t().kind != "EOF" && !p.declaration_start() { p.take(); }
                let mut span = start.span(); span.set("end", p.t().start.into());
                result.push("holes", obj(&[("kind", "hole".into()), ("source", span.clone())]));
                result.push("diagnostics", obj(&[("code", "LG1:syntax.recovery_gap".into()),
                    ("message", "declaration omitted; draft cannot lower".into()), ("source", span)]));
            }
        }
    }
    result
}
