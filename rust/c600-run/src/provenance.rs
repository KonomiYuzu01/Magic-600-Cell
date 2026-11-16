//! The provenance block, FP.17 wire version 1 (LP2, M3; chapter 07, BND.18 to BND.24).
//!
//! - [`Json`] is the JSON value model a block needs. [`canonical`] writes FP.5's bytes: keys in Unicode scalar order,
//!   compact separators, `encode`'s string escapes and one terminal line feed. [`parse`] reads JSON text strictly: no
//!   duplicate keys, integers within ±(2^53 − 1) with no fraction or exponent, no lone surrogates.
//! - [`validate`] applies FP.17's structural rules as the format packet's `check_vectors.py` does, and also checks the
//!   shapes of the pins and digest references that preview admission reads. A failure is
//!   `PS1:capability.provenance_incompatible`.
//! - [`read`] reads stored block bytes. Bytes that do not parse, are not canonical or fail FP.17 read `unknown`
//!   (BND.21): the caller keeps them (PS1.33) and never reads them as `own`.
//! - [`program`], [`manual`] and [`import`] build blocks. Only the host builds them (BND.22); the IR has no operation
//!   that sets a field. The runtime builds blocks in the capability-separation form only (LQ21); it reads all three.

use std::collections::BTreeMap;

use c600_digest::sha256;

use crate::encode;

/// FP.17's refusal of a block that cannot justify a use.
pub const INCOMPATIBLE: &str = "PS1:capability.provenance_incompatible";

const MAX_INT: i64 = 9_007_199_254_740_991;
const MAX_DEPTH: usize = 64;
const FIELDS: [&str; 9] =
    ["block", "author", "producer", "profile", "depends", "parts", "pick", "declassified", "legacy"];
const FORMS: [&str; 3] = ["flow_static", "flow_runtime", "capability_separation"];
const AUTHORS: [&str; 5] = ["built_in", "own", "forked_built_in", "imported", "unknown"];
const PROFILES: [&str; 4] = ["none", "solving", "research", "tool_authoring"];
const ATOMS: [&str; 4] = ["state", "computed", "research", "unknown"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i64),
    Text(String),
    List(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    pub fn object<const N: usize>(fields: [(&str, Json); N]) -> Json {
        Json::Object(fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    pub fn text(s: &str) -> Json {
        Json::Text(s.into())
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(m) => m.get(key),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Json::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Whether this is an object with exactly these keys.
    fn has_exactly(&self, keys: &[&str]) -> bool {
        matches!(self, Json::Object(m) if m.len() == keys.len() && keys.iter().all(|k| m.contains_key(*k)))
    }

    fn field_text(&self, key: &str) -> &str {
        self.get(key).and_then(Json::as_text).unwrap_or("")
    }
}

/// FP.5's canonical bytes of a JSON value.
pub fn canonical(value: &Json) -> Vec<u8> {
    let mut out = Vec::new();
    write(value, &mut out);
    out.push(b'\n');
    out
}

fn write(value: &Json, out: &mut Vec<u8>) {
    match value {
        Json::Null => out.extend_from_slice(b"null"),
        Json::Bool(b) => out.extend_from_slice(if *b { b"true" } else { b"false" }),
        Json::Int(n) => out.extend_from_slice(n.to_string().as_bytes()),
        Json::Text(s) => encode::string(s, out),
        Json::List(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write(item, out);
            }
            out.push(b']');
        }
        Json::Object(map) => {
            out.push(b'{');
            for (i, (key, item)) in map.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                encode::string(key, out);
                out.push(b':');
                write(item, out);
            }
            out.push(b'}');
        }
    }
}

/// Reads JSON text strictly; insignificant whitespace is allowed.
pub fn parse(bytes: &[u8]) -> Option<Json> {
    std::str::from_utf8(bytes).ok()?;
    let mut p = Parser { s: bytes, i: 0 };
    let value = p.value(0)?;
    p.ws();
    (p.i == bytes.len()).then_some(value)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while matches!(self.s.get(self.i), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn eat(&mut self, b: u8) -> bool {
        self.ws();
        let found = self.s.get(self.i) == Some(&b);
        self.i += found as usize;
        found
    }

    fn literal(&mut self, word: &[u8], value: Json) -> Option<Json> {
        self.s[self.i..].starts_with(word).then(|| {
            self.i += word.len();
            value
        })
    }

    /// `depth` counts the containers around the value. FP.9's `json_depth` counts the containers on the deepest path,
    /// so a container at depth 64 would be the 65th.
    fn value(&mut self, depth: usize) -> Option<Json> {
        self.ws();
        match *self.s.get(self.i)? {
            b'[' | b'{' if depth >= MAX_DEPTH => None,
            b'n' => self.literal(b"null", Json::Null),
            b't' => self.literal(b"true", Json::Bool(true)),
            b'f' => self.literal(b"false", Json::Bool(false)),
            b'"' => self.string().map(Json::Text),
            b'-' | b'0'..=b'9' => self.number(),
            b'[' => {
                self.i += 1;
                let mut items = Vec::new();
                if !self.eat(b']') {
                    loop {
                        items.push(self.value(depth + 1)?);
                        if self.eat(b']') {
                            break;
                        }
                        if !self.eat(b',') {
                            return None;
                        }
                    }
                }
                Some(Json::List(items))
            }
            b'{' => {
                self.i += 1;
                let mut map = BTreeMap::new();
                if !self.eat(b'}') {
                    loop {
                        self.ws();
                        if self.s.get(self.i) != Some(&b'"') {
                            return None;
                        }
                        let key = self.string()?;
                        if !self.eat(b':') {
                            return None;
                        }
                        let item = self.value(depth + 1)?;
                        if map.insert(key, item).is_some() {
                            return None;
                        }
                        if self.eat(b'}') {
                            break;
                        }
                        if !self.eat(b',') {
                            return None;
                        }
                    }
                }
                Some(Json::Object(map))
            }
            _ => None,
        }
    }

    fn number(&mut self) -> Option<Json> {
        let start = self.i;
        self.i += (self.s[self.i] == b'-') as usize;
        let digits = self.i;
        while matches!(self.s.get(self.i), Some(b'0'..=b'9')) {
            self.i += 1;
        }
        let d = &self.s[digits..self.i];
        if d.is_empty() || (d.len() > 1 && d[0] == b'0') || d.len() > 16 {
            return None;
        }
        if matches!(self.s.get(self.i), Some(b'.' | b'e' | b'E')) {
            return None;
        }
        let n: i64 = std::str::from_utf8(&self.s[start..self.i]).ok()?.parse().ok()?;
        (n.abs() <= MAX_INT).then_some(Json::Int(n))
    }

    fn hex4(&mut self) -> Option<u32> {
        let h = self.s.get(self.i..self.i + 4)?;
        self.i += 4;
        if !h.iter().all(u8::is_ascii_hexdigit) {
            return None;
        }
        u32::from_str_radix(std::str::from_utf8(h).ok()?, 16).ok()
    }

    fn string(&mut self) -> Option<String> {
        self.i += 1;
        let mut out = String::new();
        loop {
            match *self.s.get(self.i)? {
                b'"' => {
                    self.i += 1;
                    return Some(out);
                }
                b'\\' => {
                    let escape = *self.s.get(self.i + 1)?;
                    self.i += 2;
                    match escape {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let high = self.hex4()?;
                            let scalar = if (0xD800..0xDC00).contains(&high) {
                                if !self.s[self.i..].starts_with(b"\\u") {
                                    return None;
                                }
                                self.i += 2;
                                let low = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&low) {
                                    return None;
                                }
                                0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00)
                            } else {
                                high
                            };
                            out.push(char::from_u32(scalar)?);
                        }
                        _ => return None,
                    }
                }
                0..=0x1f => return None,
                _ => {
                    // A run of plain bytes ends at an ASCII delimiter, so it is a whole UTF-8 sequence.
                    let start = self.i;
                    while let Some(&c) = self.s.get(self.i) {
                        if c == b'"' || c == b'\\' || c < 0x20 {
                            break;
                        }
                        self.i += 1;
                    }
                    out.push_str(std::str::from_utf8(&self.s[start..self.i]).ok()?);
                }
            }
        }
    }
}

fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_digest_ref(v: &Json, role: &str) -> bool {
    v.has_exactly(&["role", "sha256"]) && v.field_text("role") == role && is_hex64(v.field_text("sha256"))
}

fn is_pin(v: &Json) -> bool {
    v.has_exactly(&["version", "source"])
        && v.get("version").and_then(Json::as_text).is_some()
        && v.get("source").is_some_and(|s| is_digest_ref(s, "source"))
}

/// FP.17's structural rules.
pub fn validate(block: &Json) -> Result<(), &'static str> {
    if well_formed(block) {
        Ok(())
    } else {
        Err(INCOMPATIBLE)
    }
}

fn well_formed(b: &Json) -> bool {
    if !b.has_exactly(&FIELDS) {
        return false;
    }
    let field = |k: &str| b.get(k).expect("a checked field");
    let meta = field("block");
    if !meta.has_exactly(&["version", "form", "contract", "checker"]) || meta.get("version") != Some(&Json::Int(1)) {
        return false;
    }
    let form = meta.field_text("form");
    let checker = meta.get("checker").expect("a checked field");
    if !FORMS.contains(&form) || !is_pin(meta.get("contract").expect("a checked field")) {
        return false;
    }
    if !(checker == &Json::Null || is_pin(checker)) {
        return false;
    }
    let author = b.field_text("author");
    let producer = field("producer");
    let kind = producer.field_text("kind");
    let profile = b.field_text("profile");
    if !AUTHORS.contains(&author) || !PROFILES.contains(&profile) || (profile == "none") != (kind != "program") {
        return false;
    }
    let legacy = field("legacy");
    let producer_ok = match kind {
        "manual" => producer.has_exactly(&["kind"]),
        "import" => {
            producer.has_exactly(&["kind", "origin"])
                && producer.get("origin").is_some_and(|o| is_digest_ref(o, "bytes"))
                && legacy.get("source") == producer.get("origin")
        }
        "program" => {
            producer.has_exactly(&["kind", "module", "entry"])
                && producer.get("module").is_some_and(|m| is_digest_ref(m, "source"))
                && producer.get("entry").and_then(Json::as_text).is_some()
                && (form != "flow_static" || checker != &Json::Null)
        }
        _ => false,
    };
    if !producer_ok {
        return false;
    }
    let pick = field("pick");
    if !(pick == &Json::Null || pick.has_exactly(&["record", "chosen"])) {
        return false;
    }
    let depends = field("depends");
    let depends_ok = if depends == &Json::object([("kind", Json::text("not_recorded"))]) {
        kind == "import" || author == "unknown"
    } else if form == "capability_separation" {
        depends.has_exactly(&["read_state"]) && matches!(depends.field_text("read_state"), "yes" | "no")
    } else {
        match depends.get("labels") {
            Some(Json::List(labels)) if depends.has_exactly(&["labels"]) => {
                let texts: Option<Vec<&str>> = labels.iter().map(Json::as_text).collect();
                texts.is_some_and(|t| t.iter().all(|a| ATOMS.contains(a)) && t.windows(2).all(|w| w[0] < w[1]))
            }
            _ => false,
        }
    };
    if !depends_ok {
        return false;
    }
    let parts_ok = matches!(field("parts"), Json::List(parts) if parts.iter().all(|p| is_digest_ref(p, "content")));
    let legacy_ok = legacy == &Json::Null || legacy.has_exactly(&["source", "labels", "missing"]);
    parts_ok && matches!(field("declassified"), Json::List(_)) && legacy_ok
}

/// Reads stored block bytes. Anything that is not a canonical FP.17 block reads `unknown` (BND.21).
pub fn read(bytes: &[u8]) -> Result<Json, &'static str> {
    let block = parse(bytes).ok_or(INCOMPATIBLE)?;
    if canonical(&block) != bytes {
        return Err(INCOMPATIBLE);
    }
    validate(&block)?;
    Ok(block)
}

/// The lowercase hex of a digest.
pub fn hex(digest: &[u8; 32]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<[u8; 32]> {
    if !is_hex64(s) {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(out)
}

pub fn digest_ref(role: &str, digest: &[u8; 32]) -> Json {
    Json::object([("role", Json::text(role)), ("sha256", Json::Text(hex(digest)))])
}

/// A contract or checker pin: `{version, source: DigestRef(source)}` (FP.11).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pin {
    pub version: String,
    pub source: [u8; 32],
}

impl Pin {
    /// The pin of a contract's or checker's source bytes.
    pub fn of(version: &str, source: &[u8]) -> Pin {
        Pin { version: version.into(), source: sha256(source) }
    }

    pub fn json(&self) -> Json {
        Json::object([("version", Json::text(&self.version)), ("source", digest_ref("source", &self.source))])
    }
}

/// The author classes of BND.18, with `unknown` for a missing or unreadable record (BND.21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Author {
    BuiltIn,
    Own,
    ForkedBuiltIn,
    Imported,
    Unknown,
}

impl Author {
    pub fn name(self) -> &'static str {
        match self {
            Author::BuiltIn => "built_in",
            Author::Own => "own",
            Author::ForkedBuiltIn => "forked_built_in",
            Author::Imported => "imported",
            Author::Unknown => "unknown",
        }
    }
}

/// The run profiles (LG3.9, BND.17).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Solving,
    ToolAuthoring,
    Research,
}

impl Profile {
    pub fn name(self) -> &'static str {
        match self {
            Profile::Solving => "solving",
            Profile::ToolAuthoring => "tool_authoring",
            Profile::Research => "research",
        }
    }
}

/// What the host knows about the program whose output a block describes.
#[derive(Clone, Debug)]
pub struct Producer {
    pub contract: Pin,
    pub checker: Option<Pin>,
    pub author: Author,
    /// The SHA-256 of the producing module's source bytes.
    pub module: [u8; 32],
    pub entry: String,
    pub profile: Profile,
}

fn meta(contract: &Pin, checker: Option<&Pin>) -> Json {
    Json::object([
        ("version", Json::Int(1)),
        ("form", Json::text("capability_separation")),
        ("contract", contract.json()),
        ("checker", checker.map_or(Json::Null, Pin::json)),
    ])
}

fn read_state(yes: bool) -> Json {
    Json::object([("read_state", Json::text(if yes { "yes" } else { "no" }))])
}

/// The block of a program's output (BND.22, BND.24): the run's facts and the blocks it took in, by content digest.
pub fn program(producer: &Producer, reads_state: bool, parts: &[[u8; 32]]) -> Json {
    Json::object([
        ("block", meta(&producer.contract, producer.checker.as_ref())),
        ("author", Json::text(producer.author.name())),
        (
            "producer",
            Json::object([
                ("kind", Json::text("program")),
                ("module", digest_ref("source", &producer.module)),
                ("entry", Json::text(&producer.entry)),
            ]),
        ),
        ("profile", Json::text(producer.profile.name())),
        ("depends", read_state(reads_state)),
        ("parts", Json::List(parts.iter().map(|d| digest_ref("content", d)).collect())),
        ("pick", Json::Null),
        ("declassified", Json::List(Vec::new())),
        ("legacy", Json::Null),
    ])
}

/// A word the solver typed (BND.23): `own`, `manual`, no profile, `read_state: no`.
pub fn manual(contract: &Pin) -> Json {
    Json::object([
        ("block", meta(contract, None)),
        ("author", Json::text("own")),
        ("producer", Json::object([("kind", Json::text("manual"))])),
        ("profile", Json::text("none")),
        ("depends", read_state(false)),
        ("parts", Json::List(Vec::new())),
        ("pick", Json::Null),
        ("declassified", Json::List(Vec::new())),
        ("legacy", Json::Null),
    ])
}

/// An imported 0.4 macro (BND.23, FP.18): `own`, `import` of the retained source bytes, no profile, `not_recorded`.
pub fn import(contract: &Pin, source: &[u8]) -> Json {
    let origin = digest_ref("bytes", &sha256(source));
    let missing = ["producing_run", "dependency_history"].map(Json::text);
    Json::object([
        ("block", meta(contract, None)),
        ("author", Json::text("own")),
        ("producer", Json::object([("kind", Json::text("import")), ("origin", origin.clone())])),
        ("profile", Json::text("none")),
        ("depends", Json::object([("kind", Json::text("not_recorded"))])),
        ("parts", Json::List(Vec::new())),
        ("pick", Json::Null),
        ("declassified", Json::List(Vec::new())),
        (
            "legacy",
            Json::object([
                ("source", origin),
                ("labels", Json::List(Vec::new())),
                ("missing", Json::List(missing.into())),
            ]),
        ),
    ])
}

/// Read accessors for a block that passed [`validate`].
pub mod field {
    use super::{unhex, Json, Pin};

    pub fn form(b: &Json) -> &str {
        b.get("block").map_or("", |m| m.field_text("form"))
    }

    pub fn author(b: &Json) -> &str {
        b.field_text("author")
    }

    pub fn profile(b: &Json) -> &str {
        b.field_text("profile")
    }

    pub fn producer(b: &Json) -> &str {
        b.get("producer").map_or("", |p| p.field_text("kind"))
    }

    pub fn has_pick(b: &Json) -> bool {
        b.get("pick").is_some_and(|p| p != &Json::Null)
    }

    fn pin(v: Option<&Json>) -> Option<Pin> {
        let v = v?;
        let source = unhex(v.get("source")?.field_text("sha256"))?;
        Some(Pin { version: v.get("version")?.as_text()?.into(), source })
    }

    pub fn contract(b: &Json) -> Option<Pin> {
        pin(b.get("block")?.get("contract"))
    }

    pub fn checker(b: &Json) -> Option<Pin> {
        pin(b.get("block")?.get("checker"))
    }

    /// The content digests of the parts, in order.
    pub fn parts(b: &Json) -> Vec<[u8; 32]> {
        match b.get("parts") {
            Some(Json::List(parts)) => parts.iter().filter_map(|p| unhex(p.field_text("sha256"))).collect(),
            _ => Vec::new(),
        }
    }

    /// Whether the block may record a dependence on the state. Only `read_state: no` and an empty label set say it
    /// does not; `not_recorded` never reads as `no` (FP.17).
    pub fn may_read_state(b: &Json) -> bool {
        match b.get("depends") {
            Some(d) if d.get("read_state").is_some() => d.field_text("read_state") != "no",
            Some(d) => !matches!(d.get("labels"), Some(Json::List(l)) if l.is_empty()),
            None => true,
        }
    }
}
