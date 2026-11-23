//! LB5.18, PS1.45 and PS1.46: the proof-obligation translator, as pure code.
//!
//! [`translate`] takes one typed [`Statement`] and the bytes its StatementId names, and returns the Lean 4 source of
//! PS1.45's `c600-proof-obligation-draft` export over vocabulary `c600-obligation-vocabulary-draft-1`: one theorem
//! whose hypotheses are the body's hypotheses, named `h0`, `h1` and so on, whose statement is the body's conclusion,
//! and whose proof is `by sorry`. An untranslatable statement refuses with `LB5:input.obligation_untranslatable` and
//! returns no text, so nothing can be written. The exact text follows the format packet's `obligations` vectors and
//! its reference reader (`docs/architecture/1.0/language/format-packet/check_vectors.py`, `obligation_result`).
//!
//! The translator reads and writes no file and starts no process; it has no network, no return path, no evidence
//! promotion and no commit gate (LB5.18, RTE.28, EVD.54). It reads no proof record and writes no evidence field:
//! `formal proof` keeps no writer (EVD.64). The `file` overlay row `research.obligation.export`, which writes the text
//! through a granted file capability and resolves a StatementId to its bytes, belongs to `c600-research`.
//!
//! # Versioned data
//!
//! - The symbol table, draft 1, is `rust/c600-evidence/obligation/symbols-draft-1.json`, compiled in as
//!   [`SYMBOLS_DRAFT_1`] and pinned by [`SYMBOLS_DRAFT_1_SHA256`]. Each row maps a `sym` or `pred` name to a Lean
//!   identifier with its arity, and each `named` sort to a Lean type.
//! - The translation prelude, draft 1, is `rust/c600-evidence/obligation/prelude-draft-1.lean`: Lean 4 source that
//!   declares every identifier the table names. It lives in the repository outside the shipped software (LB5.18): it is
//!   not compiled in, and the translator holds only its pin, [`PRELUDE_DRAFT_1_SHA256`]. No Lean runs in LP5 (owner,
//!   23 November 2026, LP5-2), so the prelude has not been elaborated by Lean.
//!
//! # Draft choices
//!
//! Where PS1.46 and the reference reader leave a detail open, this module takes a draft, named here. None of these is
//! a decision of the frozen text.
//!
//! - Table draft 1 holds exactly the rows of the format packet's fixture (`base`, `grade`, `member`, `orbit` and the
//!   sort `Piece`). The research vocabulary grows by a new table version with a new digest and a new prelude pin.
//! - Pins: the statement's content digest, the SHA-256 of the table's canonical bytes (the vectors' `table` form:
//!   `vocabulary`, `symbols`, `sorts`) and the prelude's SHA-256 when the table names a prelude. The vocabulary is
//!   carried beside them, as the vectors carry it.
//! - Order of checks, as the reference reader's: a named StatementId; the table's vocabulary; the body's bytes
//!   (unresolved bytes, read as no typed body); their canonical form (`PS1:input.noncanonical_value`,
//!   `PS1:resource_limit.persistence_size`); their digest against the StatementId (`PS1:input.digest_mismatch`); then
//!   the body, hypotheses before the conclusion. Exact integers refuse as the reference reader's `int_value` does.
//! - The reference reader ignores extra fields and fails without a code on a missing field or a name that is not text.
//!   Here every node of the body must have exactly its form's fields, and anything else refuses with
//!   `LB5:input.obligation_untranslatable`.
//! - A `forall` or `exists` name becomes a Lean binder, so it must be an ASCII identifier (a letter, then letters,
//!   digits or `_`) and not a Lean keyword. The table's Lean identifiers are dotted names of such parts.

pub mod json;

use std::collections::BTreeMap;

use crate::model::{Statement, StatementId};
use json::Json;

/// PS1.45: the export format.
pub const FORMAT: &str = "c600-proof-obligation-draft";
/// PS1.46: the vocabulary of a statement body.
pub const VOCABULARY: &str = "c600-obligation-vocabulary-draft-1";
/// LB5.18, PS1.46: the refusal of anything outside the vocabulary.
pub const UNTRANSLATABLE: &str = "LB5:input.obligation_untranslatable";
/// PS1.46: the body's bytes disagree with the StatementId's digest.
pub const DIGEST_MISMATCH: &str = "PS1:input.digest_mismatch";
/// PS1.3 to PS1.5: the body's bytes are not canonical.
pub const NONCANONICAL_VALUE: &str = "PS1:input.noncanonical_value";
/// PS1.10, PS1.11: the body exceeds a format limit.
pub const PERSISTENCE_SIZE: &str = "PS1:resource_limit.persistence_size";

/// The symbol table, draft 1: canonical JSON in the vectors' `table` form.
pub const SYMBOLS_DRAFT_1: &[u8] = include_bytes!("../../obligation/symbols-draft-1.json");
/// The pin of [`SYMBOLS_DRAFT_1`]: the SHA-256 of its bytes.
pub const SYMBOLS_DRAFT_1_SHA256: &str = "9808840fa678be94f2a3c11cbbc89f671f1b3a681eaad280eb78c1001cf89933";
/// The pin of the translation prelude, draft 1 (`rust/c600-evidence/obligation/prelude-draft-1.lean`), which is not
/// compiled in: the SHA-256 of its bytes.
pub const PRELUDE_DRAFT_1_SHA256: &str = "b9d20ad081ce66289504bdc73ef57ca062f4d8d607cdd4b99a6883bac4922bdb";

/// Lean 4 keywords that have the shape of an identifier. A binder or a part of a Lean identifier is never one of
/// these.
const LEAN_KEYWORDS: [&str; 86] = [
    "Prop",
    "Sort",
    "Type",
    "abbrev",
    "at",
    "attribute",
    "axiom",
    "break",
    "builtin_initialize",
    "by",
    "calc",
    "catch",
    "class",
    "continue",
    "decreasing_by",
    "def",
    "deriving",
    "do",
    "elab",
    "elab_rules",
    "else",
    "end",
    "example",
    "exists",
    "export",
    "extends",
    "finally",
    "for",
    "forall",
    "from",
    "fun",
    "have",
    "hiding",
    "if",
    "import",
    "in",
    "include",
    "inductive",
    "infix",
    "infixl",
    "infixr",
    "initialize",
    "instance",
    "lemma",
    "let",
    "local",
    "macro",
    "macro_rules",
    "match",
    "mut",
    "mutual",
    "namespace",
    "nofun",
    "nomatch",
    "noncomputable",
    "nonrec",
    "notation",
    "omit",
    "opaque",
    "open",
    "partial",
    "postfix",
    "prefix",
    "private",
    "protected",
    "renaming",
    "return",
    "scoped",
    "section",
    "set_option",
    "show",
    "sorry",
    "structure",
    "suffices",
    "syntax",
    "termination_by",
    "then",
    "theorem",
    "this",
    "universe",
    "unless",
    "unsafe",
    "using",
    "variable",
    "where",
    "with",
];

/// Why a statement was not exported. Each reason has one code; tests read the code and the reason, never the detail
/// (ERR.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reason {
    /// The StatementId is a name: there is no body to translate (PS1.46).
    NamedStatement,
    /// The typed StatementId's bytes did not resolve: there is no typed body.
    NoBody,
    /// The symbol table belongs to another vocabulary.
    Vocabulary,
    /// The symbol table is not in its form.
    Table,
    /// A node of the body is outside the vocabulary's forms.
    Form,
    /// A `sym`, `pred` or `named` name is not in the symbol table.
    UnknownSymbol,
    /// A `sym` or `pred` has another number of arguments than its row's arity.
    Arity,
    /// A `var` is bound by no enclosing `forall` or `exists`.
    UnboundVariable,
    /// The body's bytes disagree with the StatementId's digest.
    Digest,
    /// The body's bytes are not canonical, or an exact integer is not canonical decimal text.
    Noncanonical,
    /// The body exceeds a format limit.
    Size,
}

impl Reason {
    /// The registered code of this reason (chapter 13).
    pub fn code(self) -> &'static str {
        match self {
            Reason::Digest => DIGEST_MISMATCH,
            Reason::Noncanonical => NONCANONICAL_VALUE,
            Reason::Size => PERSISTENCE_SIZE,
            _ => UNTRANSLATABLE,
        }
    }
}

/// A refusal of the export. No text accompanies it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObligationError {
    pub code: &'static str,
    pub reason: Reason,
    pub detail: String,
}

impl ObligationError {
    pub fn new(reason: Reason, detail: &str) -> ObligationError {
        ObligationError { code: reason.code(), reason, detail: detail.to_string() }
    }
}

impl std::fmt::Display for ObligationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for ObligationError {}

fn refuse<T>(reason: Reason, detail: &str) -> Result<T, ObligationError> {
    Err(ObligationError::new(reason, detail))
}

/// One row of the symbol table: the Lean identifier of the prelude and the arity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    pub lean: String,
    pub arity: usize,
}

/// A versioned symbol table (PS1.46). It is built only from JSON in the vectors' `table` form, so its digest always
/// names its rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolTable {
    vocabulary: String,
    symbols: BTreeMap<String, Symbol>,
    sorts: BTreeMap<String, String>,
    prelude: Option<String>,
    digest: String,
}

impl SymbolTable {
    /// Reads a table from JSON `{"vocabulary": text, "symbols": {name: {"lean": text, "arity": integer}}, "sorts":
    /// {name: text}}`. `prelude` is the SHA-256 of the prelude whose declarations the rows name, or `None` for a
    /// table with no pinned prelude, such as the format packet's fixture. A table outside this form refuses with
    /// `LB5:input.obligation_untranslatable`. Any vocabulary is read; [`translate`] refuses a table of another one.
    pub fn from_json(value: &Json, prelude: Option<&str>) -> Result<SymbolTable, ObligationError> {
        let table = |detail: &str| ObligationError::new(Reason::Table, detail);
        let fields = value.as_object().ok_or_else(|| table("the table is not an object"))?;
        if !exact(fields, &["sorts", "symbols", "vocabulary"]) {
            return refuse(Reason::Table, "the table has other fields than vocabulary, symbols and sorts");
        }
        let vocabulary = fields["vocabulary"].as_text().ok_or_else(|| table("the vocabulary is not text"))?;
        let mut symbols = BTreeMap::new();
        for (name, row) in fields["symbols"].as_object().ok_or_else(|| table("the symbols are not an object"))? {
            let row = row.as_object().ok_or_else(|| table("a symbol row is not an object"))?;
            if name.is_empty() || !exact(row, &["arity", "lean"]) {
                return refuse(Reason::Table, "a symbol row is not a name with lean and arity");
            }
            let lean = row["lean"].as_text().filter(|lean| lean_name(lean));
            let lean = lean.ok_or_else(|| table("a symbol row's lean is no Lean identifier"))?;
            let arity = match &row["arity"] {
                Json::Number(text) => json::canonical(&row["arity"]).ok().and_then(|_| text.parse::<usize>().ok()),
                _ => None,
            };
            let arity = arity.ok_or_else(|| table("a symbol row's arity is not a non-negative integer"))?;
            symbols.insert(name.clone(), Symbol { lean: lean.to_string(), arity });
        }
        let mut sorts = BTreeMap::new();
        for (name, lean) in fields["sorts"].as_object().ok_or_else(|| table("the sorts are not an object"))? {
            let lean = lean.as_text().filter(|lean| !name.is_empty() && lean_name(lean));
            let lean = lean.ok_or_else(|| table("a sort row is not a name with a Lean identifier"))?;
            sorts.insert(name.clone(), lean.to_string());
        }
        if let Some(pin) = prelude {
            if !(pin.len() == 64 && pin.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))) {
                return refuse(Reason::Table, "the prelude pin is not a SHA-256 in lower-case hexadecimal");
            }
        }
        let bytes = json::canonical(value).map_err(|_| table("the table has no canonical bytes"))?;
        Ok(SymbolTable {
            vocabulary: vocabulary.to_string(),
            symbols,
            sorts,
            prelude: prelude.map(str::to_string),
            digest: c600_digest::sha256_hex(&bytes),
        })
    }

    /// Reads a table from its canonical bytes.
    pub fn read(bytes: &[u8], prelude: Option<&str>) -> Result<SymbolTable, ObligationError> {
        SymbolTable::from_json(&json::read_canonical(bytes)?, prelude)
    }

    /// The symbol table, draft 1, with the prelude, draft 1, as its pinned prelude.
    pub fn draft_1() -> Result<SymbolTable, ObligationError> {
        SymbolTable::read(SYMBOLS_DRAFT_1, Some(PRELUDE_DRAFT_1_SHA256))
    }

    pub fn vocabulary(&self) -> &str {
        &self.vocabulary
    }

    pub fn symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    /// The Lean type of a `named` sort.
    pub fn sort(&self, name: &str) -> Option<&str> {
        self.sorts.get(name).map(String::as_str)
    }

    /// The SHA-256 of the table's canonical bytes: the export's table pin.
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// The pinned prelude's SHA-256, when the table names one.
    pub fn prelude(&self) -> Option<&str> {
        self.prelude.as_deref()
    }
}

/// The statement and translation dependency pins of an export (PS1.45).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pins {
    /// The statement's content digest (FP.19's typed StatementId).
    pub statement: String,
    /// The SHA-256 of the symbol table's canonical bytes.
    pub symbols: String,
    /// The SHA-256 of the prelude the table names, when it names one.
    pub prelude: Option<String>,
}

/// One exported obligation: PS1.45's `c600-proof-obligation-draft` with its Lean 4 source (UTF-8, LF) and the
/// SHA-256 of the source bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Obligation {
    pub format: &'static str,
    pub vocabulary: &'static str,
    pub source: String,
    pub sha256: String,
    pub pins: Pins,
}

/// LB5.18, PS1.45, PS1.46: the Lean 4 source of one typed statement. `body` is the bytes the statement's typed
/// StatementId names, as the caller resolved them, or `None` when they did not resolve. A refusal returns no text.
pub fn translate(
    statement: &Statement,
    body: Option<&[u8]>,
    table: &SymbolTable,
) -> Result<Obligation, ObligationError> {
    let content = match &statement.id {
        StatementId::Named { .. } => return refuse(Reason::NamedStatement, "a statement named by ID has no body"),
        StatementId::Typed { content } => content,
    };
    if table.vocabulary != VOCABULARY {
        return refuse(Reason::Vocabulary, "the symbol table belongs to another vocabulary");
    }
    let bytes = match body {
        Some(bytes) => bytes,
        None => return refuse(Reason::NoBody, "the typed statement's bytes did not resolve"),
    };
    let value = json::read_canonical(bytes)?;
    if c600_digest::sha256_hex(bytes) != *content {
        return refuse(Reason::Digest, "the body's bytes disagree with the statement's content digest");
    }
    let fields = value.as_object().filter(|fields| exact(fields, &["conclusion", "hypotheses"]));
    let fields =
        fields.ok_or_else(|| ObligationError::new(Reason::Form, "the body is not hypotheses and conclusion"))?;
    let hypotheses = fields["hypotheses"]
        .as_array()
        .ok_or_else(|| ObligationError::new(Reason::Form, "the hypotheses are not a list"))?;
    let mut bound = Vec::new();
    let mut source = String::from("theorem obligation");
    for (index, hypothesis) in hypotheses.iter().enumerate() {
        source.push_str(&format!(" (h{index} : {})", prop(hypothesis, table, &mut bound)?));
    }
    source.push_str(&format!(" : {} := by sorry\n", prop(&fields["conclusion"], table, &mut bound)?));
    Ok(Obligation {
        format: FORMAT,
        vocabulary: VOCABULARY,
        sha256: c600_digest::sha256_hex(source.as_bytes()),
        source,
        pins: Pins { statement: content.clone(), symbols: table.digest.clone(), prelude: table.prelude.clone() },
    })
}

/// Whether an object has exactly these fields.
fn exact(fields: &BTreeMap<String, Json>, names: &[&str]) -> bool {
    fields.len() == names.len() && names.iter().all(|name| fields.contains_key(*name))
}

fn identifier(part: &str) -> bool {
    let mut chars = part.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !LEAN_KEYWORDS.contains(&part)
}

/// The names the output itself writes outside the table: the constants and sorts of PS1.46's forms.
const OUTPUT_NAMES: [&str; 5] = ["False", "Int", "Nat", "Rat", "True"];

/// Whether a binder named `name` would hide a name the output uses: one of [`OUTPUT_NAMES`], or the first part of a
/// symbol's or a sort's Lean identifier in the table (a binder `C600` would turn `C600.member` into a field access).
fn shadows(table: &SymbolTable, name: &str) -> bool {
    OUTPUT_NAMES.contains(&name)
        || table
            .symbols
            .values()
            .map(|row| row.lean.as_str())
            .chain(table.sorts.values().map(String::as_str))
            .any(|lean| lean.split('.').next() == Some(name))
}

/// A Lean identifier of the table: dotted parts, each an identifier.
fn lean_name(name: &str) -> bool {
    name.split('.').all(identifier)
}

/// A node of the body with exactly the fields of its form.
fn node<'a>(value: &'a Json, names: &[&str]) -> Result<&'a BTreeMap<String, Json>, ObligationError> {
    match value.as_object() {
        Some(fields) if exact(fields, names) => Ok(fields),
        _ => refuse(Reason::Form, "a node does not have exactly its form's fields"),
    }
}

fn kind(value: &Json) -> Result<&str, ObligationError> {
    match value.get("kind").and_then(Json::as_text) {
        Some(kind) => Ok(kind),
        None => refuse(Reason::Form, "a node has no kind"),
    }
}

fn prop(value: &Json, table: &SymbolTable, bound: &mut Vec<String>) -> Result<String, ObligationError> {
    let kind = kind(value)?;
    match kind {
        "true" | "false" => {
            node(value, &["kind"])?;
            Ok(if kind == "true" { "True" } else { "False" }.to_string())
        }
        "eq" | "le" | "lt" => {
            let fields = node(value, &["a", "b", "kind"])?;
            let mark = match kind {
                "eq" => "=",
                "le" => "≤",
                _ => "<",
            };
            let a = term(&fields["a"], table, bound)?;
            Ok(format!("({a} {mark} {})", term(&fields["b"], table, bound)?))
        }
        "not" => Ok(format!("(¬ {})", prop(&node(value, &["kind", "p"])?["p"], table, bound)?)),
        "and" | "or" | "implies" => {
            let fields = node(value, &["kind", "p", "q"])?;
            let mark = match kind {
                "and" => "∧",
                "or" => "∨",
                _ => "→",
            };
            let p = prop(&fields["p"], table, bound)?;
            Ok(format!("({p} {mark} {})", prop(&fields["q"], table, bound)?))
        }
        "forall" | "exists" => {
            let fields = node(value, &["kind", "name", "p", "sort"])?;
            let mark = if kind == "forall" { "∀" } else { "∃" };
            let name = match fields["name"].as_text() {
                Some(name) if identifier(name) && !shadows(table, name) => name,
                _ => return refuse(Reason::Form, "a bound name is no identifier Lean accepts as a binder"),
            };
            bound.push(name.to_string());
            let inner = prop(&fields["p"], table, bound);
            bound.pop();
            let inner = inner?;
            Ok(format!("({mark} {name} : {}, {inner})", sort(&fields["sort"], table)?))
        }
        "pred" => application(node(value, &["args", "kind", "name"])?, table, bound),
        _ => refuse(Reason::Form, "a proposition is outside the vocabulary"),
    }
}

fn term(value: &Json, table: &SymbolTable, bound: &mut Vec<String>) -> Result<String, ObligationError> {
    match kind(value)? {
        "var" => {
            let fields = node(value, &["kind", "name"])?;
            match fields["name"].as_text() {
                Some(name) if bound.iter().any(|b| b == name) => Ok(name.to_string()),
                _ => refuse(Reason::UnboundVariable, "a variable is bound by no enclosing quantifier"),
            }
        }
        "nat" => {
            let (text, negative) = json::decimal(&node(value, &["kind", "value"])?["value"])?;
            if negative {
                return refuse(Reason::Form, "a natural number is negative");
            }
            Ok(text.to_string())
        }
        "int" => Ok(format!("({} : Int)", json::decimal(&node(value, &["kind", "value"])?["value"])?.0)),
        "rat" => {
            let fields = node(value, &["d", "kind", "n"])?;
            let (d, negative) = json::decimal(&fields["d"])?;
            if negative || d == "0" {
                return refuse(Reason::Form, "a rational's denominator is not positive");
            }
            Ok(format!("(({} : Rat) / {d})", json::decimal(&fields["n"])?.0))
        }
        "sym" => application(node(value, &["args", "kind", "name"])?, table, bound),
        _ => refuse(Reason::Form, "a term is outside the vocabulary"),
    }
}

fn sort(value: &Json, table: &SymbolTable) -> Result<String, ObligationError> {
    match kind(value)? {
        kind @ ("Nat" | "Int" | "Rat") => {
            node(value, &["kind"])?;
            Ok(kind.to_string())
        }
        "named" => match node(value, &["kind", "name"])?["name"].as_text().and_then(|name| table.sort(name)) {
            Some(lean) => Ok(lean.to_string()),
            None => refuse(Reason::UnknownSymbol, "a named sort is not in the symbol table"),
        },
        _ => refuse(Reason::Form, "a sort is outside the vocabulary"),
    }
}

/// A `sym` or `pred`: its row's Lean identifier, applied to its arguments in order.
fn application(
    fields: &BTreeMap<String, Json>,
    table: &SymbolTable,
    bound: &mut Vec<String>,
) -> Result<String, ObligationError> {
    let row = match fields["name"].as_text().and_then(|name| table.symbol(name)) {
        Some(row) => row,
        None => return refuse(Reason::UnknownSymbol, "a symbol is not in the symbol table"),
    };
    let args = match fields["args"].as_array() {
        Some(args) => args,
        None => return refuse(Reason::Form, "a symbol's arguments are not a list"),
    };
    if args.len() != row.arity {
        return refuse(Reason::Arity, "a symbol has another number of arguments than its arity");
    }
    if args.is_empty() {
        return Ok(row.lean.clone());
    }
    let mut text = format!("({}", row.lean);
    for arg in args {
        text.push(' ');
        text.push_str(&term(arg, table, bound)?);
    }
    text.push(')');
    Ok(text)
}
