//! The declared-part round trip (LP4-7; chapter 04, EV3.5's `edit_declared_part` and EV3.7).
//!
//! Round trip is limited to parameters, layouts and defaults that their source explicitly declares editable. An edit
//! names the declaration, the source digest, a typed field and a proposed value, and returns source edits for checking:
//! the edited draft is checked again here, and its diagnostics and verdict come back with the edits. Nothing infers
//! code from a drawing, nothing is persisted, and no program runs (EV1.9): persisting the new version is the
//! workspace command's and activation's (EV1.11), and a new version inherits no evidence (LB6.3).
//!
//! DRAFT, which freezes with the syntax (PHS.49): the source syntax has no clause for an editable part, so a part is
//! declared by a directive comment on the lines directly above its declaration,
//!
//! ```text
//! # @editable layout columns title.size
//! fn filter_layout() -> record { columns: Nat, title: record { size: Nat } } effects [] {
//!     return record { columns = 3, title = record { size = 12 } };
//! }
//! ```
//!
//! naming the part's kind (`parameter`, `layout` or `default`) and its editable fields as dotted paths into the
//! result. With no field listed, the one field is `value`, the whole result. A declared part is a function with no
//! generics, parameters or effects whose body is `return` of a literal: an exact or approximate number, text, a
//! boolean, a literal word, or a record of these. A computed value, a command, a view, a field the directive does not
//! list and every declaration without a directive are not round-tripped: `EV3:input.round_trip_undeclared`. A value of
//! another type than the field's is `LG2:type.mismatch`, and a request for other bytes `EV1:stale.editor_source`.

use c600_syntax::ast::{Decl, DeclKind, Expr, ExprKind, Item, TriviaKind, TypeExpr};
use c600_syntax::{sha256_hex, type_text, ParsedFile, Span};

use crate::context::ModuleContext;
use crate::diagnose::{diagnose, Verdict};
use crate::document::{Document, SourceRange, Subject};
use crate::envelope::{code, Envelope, Form, NextAction, Reason, Service};
use crate::rename::Edit;

/// The directive's keyword, after the comment's `#`.
pub const DIRECTIVE: &str = "@editable";

/// The field that names a part's whole value.
pub const WHOLE: &str = "value";

/// What a declared part is (EV3.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PartKind {
    Parameter,
    Layout,
    Default,
}

impl PartKind {
    pub const ALL: [PartKind; 3] = [PartKind::Parameter, PartKind::Layout, PartKind::Default];

    /// The directive's spelling.
    pub fn name(self) -> &'static str {
        match self {
            PartKind::Parameter => "parameter",
            PartKind::Layout => "layout",
            PartKind::Default => "default",
        }
    }

    pub fn named(name: &str) -> Option<PartKind> {
        PartKind::ALL.into_iter().find(|k| k.name() == name)
    }
}

/// A typed value for a declared field, in the literal forms of the source (LG1.6, LG1.7). Integers and ratios are
/// canonical decimal text, never host numbers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PartValue {
    Bool(bool),
    /// An integer: `0` or an optional `-` and digits without a leading zero.
    Int(String),
    /// A ratio of canonical integers with a nonzero denominator.
    Rat {
        num: String,
        den: String,
    },
    /// The rational coefficients of 1 and √5.
    Q5 {
        one: (String, String),
        sqrt5: (String, String),
    },
    /// An approximate literal: its precision in bits (24, 53 or 113) and its finite decimal text.
    Approx {
        precision: u32,
        decimal: String,
    },
    Text(String),
    /// A literal word: signed letters of magnitude 1 to 1200.
    Word(Vec<i32>),
    /// A record, its fields in the declared type's order.
    Record(Vec<(String, PartValue)>),
}

impl PartValue {
    /// The value's kind, as a reason names it.
    pub fn kind(&self) -> &'static str {
        match self {
            PartValue::Bool(_) => "Bool",
            PartValue::Int(_) => "Int",
            PartValue::Rat { .. } => "Rat",
            PartValue::Q5 { .. } => "Q5",
            PartValue::Approx { .. } => "Approx",
            PartValue::Text(_) => "Text",
            PartValue::Word(_) => "Word",
            PartValue::Record(_) => "record",
        }
    }

    /// The value's literal text in the source's canonical spelling.
    pub fn literal(&self) -> String {
        match self {
            PartValue::Bool(b) => b.to_string(),
            PartValue::Int(n) => n.clone(),
            PartValue::Rat { num, den } => format!("{num}/{den}"),
            PartValue::Q5 { one, sqrt5 } => format!("q5({}/{}, {}/{})", one.0, one.1, sqrt5.0, sqrt5.1),
            PartValue::Approx { precision, decimal } => format!("~{precision}{}", quote(decimal)),
            PartValue::Text(s) => quote(s),
            PartValue::Word(letters) => {
                format!("word[{}]", letters.iter().map(|l| format!("{l:+}")).collect::<Vec<_>>().join(", "))
            }
            PartValue::Record(fields) if fields.is_empty() => "record {}".to_owned(),
            PartValue::Record(fields) => format!(
                "record {{ {} }}",
                fields.iter().map(|(n, v)| format!("{n} = {}", v.literal())).collect::<Vec<_>>().join(", ")
            ),
        }
    }

    /// The value a literal expression holds; `None` for anything computed.
    fn of(expr: &Expr) -> Option<PartValue> {
        Some(match &expr.kind {
            ExprKind::Bool(b) => PartValue::Bool(*b),
            ExprKind::Int(n) => PartValue::Int(n.clone()),
            ExprKind::Rat { num, den } => PartValue::Rat { num: num.clone(), den: den.clone() },
            ExprKind::Q5 { one, sqrt5 } => PartValue::Q5 { one: one.clone(), sqrt5: sqrt5.clone() },
            ExprKind::Approx { precision, decimal } => {
                PartValue::Approx { precision: *precision, decimal: decimal.clone() }
            }
            ExprKind::Text(s) => PartValue::Text(s.clone()),
            ExprKind::Word(letters) => PartValue::Word(letters.clone()),
            ExprKind::Record(fields) => PartValue::Record(
                fields.iter().map(|(n, e)| Some((n.text.clone(), PartValue::of(e)?))).collect::<Option<Vec<_>>>()?,
            ),
            _ => return None,
        })
    }
}

/// One editable field of a declared part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredField {
    /// The dotted path, or [`WHOLE`].
    pub path: String,
    /// The field's declared type, in the source's canonical type text.
    pub ty: String,
    /// The value the source holds now.
    pub value: PartValue,
    /// The literal's range.
    pub range: SourceRange,
}

/// A part its source declares editable (EV3.7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredPart {
    pub declaration: String,
    pub kind: PartKind,
    /// The declaration's range.
    pub range: SourceRange,
    /// The directive comment's range.
    pub directive: SourceRange,
    pub fields: Vec<DeclaredField>,
}

/// Why a part is not round-tripped (`EV3:input.round_trip_undeclared`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Undeclared {
    /// No parsed declaration has this name.
    NoDeclaration,
    /// The declaration carries no `@editable` directive.
    NotEditable,
    /// The directive does not read: an unknown kind, a field path that is not a dotted name, a field listed twice, or
    /// two directives on one declaration.
    Malformed,
    /// The directive does not list the field; `declared` gives the fields it lists.
    FieldNotDeclared { declared: Vec<String> },
    /// The part is not a literal: a declaration with generics, parameters or effects, a command or view, a body that
    /// binds or computes, or a field whose value is computed. A drawn or computed part is never inverted into code.
    NotLiteral,
}

/// The answer to [`edit_declared_part`]: the source edits, the edited bytes' digest, and the edited draft checked
/// again under the same context.
#[derive(Clone, Debug)]
pub struct RoundTrip {
    pub subject: Subject,
    pub declaration: SourceRange,
    pub kind: PartKind,
    pub field: String,
    /// The edits, all under the request's digest.
    pub edits: Vec<Edit>,
    /// The SHA-256 of the bytes after the edits: the new version.
    pub source_digest: String,
    /// The edited draft's diagnostics.
    pub diagnostics: Vec<Envelope>,
    pub verdict: Verdict,
}

/// The parts the document's source declares editable, in source order. A declaration with a directive that is not a
/// literal part is left out.
pub fn declared_parts(document: &Document, digest: &str) -> Result<Vec<DeclaredPart>, Envelope> {
    let text = document.expect(digest, Service::EditDeclaredPart)?;
    let file = c600_syntax::parse(text.as_bytes());
    let mut parts = Vec::new();
    for (decl, directives) in declarations(&file, text) {
        if let Ok(part) = part(decl, &directives, text, digest) {
            parts.push(part);
        }
    }
    Ok(parts)
}

/// `edit_declared_part(source, declaration, field, value)` (EV3.5, EV3.7): the edits that set one declared field to
/// `value`, with the edited draft's check under `context`. The field is a dotted path from the directive, or
/// [`WHOLE`] for the whole value.
pub fn edit_declared_part(
    document: &Document,
    digest: &str,
    declaration: &str,
    field: &str,
    value: &PartValue,
    context: &ModuleContext,
) -> Result<RoundTrip, Envelope> {
    let text = document.expect(digest, Service::EditDeclaredPart)?;
    let subject = document.subject(&context.edition, Some(context.digest()));
    let file = c600_syntax::parse(text.as_bytes());
    let undeclared = |why: Undeclared| undeclared(declaration, field, why);
    let (decl, directives) = declarations(&file, text)
        .into_iter()
        .find(|(d, _)| d.name.text == declaration)
        .ok_or_else(|| undeclared(Undeclared::NoDeclaration))?;
    let part = part(decl, &directives, text, digest).map_err(undeclared)?;
    let Some(target) = part.fields.iter().find(|f| f.path == field) else {
        let declared = part.fields.iter().map(|f| f.path.clone()).collect();
        return Err(undeclared(Undeclared::FieldNotDeclared { declared }));
    };
    let ty = field_type(&decl.result, field).ok_or_else(|| undeclared(Undeclared::NotLiteral))?;
    // A refusal carries no span (EV1.12, as the other services' refusals): the reason names the declaration and the
    // field, and `declared_parts` gives its range.
    if let Err((expected, supplied)) = conforms(ty, value) {
        return Err(Envelope::new(
            code::TYPE_MISMATCH,
            Form::Refusal,
            Service::EditDeclaredPart,
            Reason::ValueType {
                declaration: declaration.to_owned(),
                field: field.to_owned(),
                expected: expected.clone(),
                supplied,
            },
            NextAction::SupplyValue { expected },
        )
        .saying("the proposed value does not have the field's declared type"));
    }

    let edits = vec![Edit { range: target.range.clone(), text: value.literal() }];
    let mut edited = String::with_capacity(text.len());
    edited.push_str(&text[..target.range.start]);
    edited.push_str(&edits[0].text);
    edited.push_str(&text[target.range.end..]);
    let source_digest = sha256_hex(edited.as_bytes());
    let after = Document::open(edited.into_bytes(), document.units());
    let diagnosis = diagnose(&after, &source_digest, context)?;
    Ok(RoundTrip {
        subject,
        declaration: part.range,
        kind: part.kind,
        field: field.to_owned(),
        edits,
        source_digest,
        diagnostics: diagnosis.diagnostics,
        verdict: diagnosis.verdict,
    })
}

fn undeclared(declaration: &str, field: &str, why: Undeclared) -> Envelope {
    Envelope::new(
        code::ROUND_TRIP_UNDECLARED,
        Form::Refusal,
        Service::EditDeclaredPart,
        Reason::RoundTripUndeclared { declaration: declaration.to_owned(), field: field.to_owned(), why },
        NextAction::EditSource,
    )
    .saying("the source does not declare this field editable")
}

/// Each parsed declaration with the directive comments between the previous item (or the header) and its start.
fn declarations<'f>(file: &'f ParsedFile, text: &str) -> Vec<(&'f Decl, Vec<Span>)> {
    let mut from = file.module.as_ref().map_or(0, |m| m.loc.0.end);
    let mut found = Vec::new();
    for item in &file.items {
        let span = match item {
            Item::Import(i) => i.loc.0,
            Item::Decl(d) => d.loc.0,
        };
        if let Item::Decl(d) = item {
            let directives = file
                .trivia
                .iter()
                .filter(|t| t.kind == TriviaKind::Comment && t.span.start >= from && t.span.end <= span.start)
                .filter(|t| is_directive(&text[t.span.start..t.span.end]))
                .map(|t| t.span)
                .collect();
            found.push((&**d, directives));
        }
        from = span.end;
    }
    found
}

/// The words after `#` and the directive keyword, if the comment is a directive.
fn directive_words(comment: &str) -> Option<Vec<&str>> {
    let mut words = comment.strip_prefix('#')?.split_whitespace();
    (words.next() == Some(DIRECTIVE)).then(|| words.collect())
}

fn is_directive(comment: &str) -> bool {
    directive_words(comment).is_some()
}

/// The declared part of a declaration, or why it is none.
fn part(decl: &Decl, directives: &[Span], text: &str, digest: &str) -> Result<DeclaredPart, Undeclared> {
    let directive = match directives {
        [] => return Err(Undeclared::NotEditable),
        [one] => *one,
        _ => return Err(Undeclared::Malformed),
    };
    let words = directive_words(&text[directive.start..directive.end]).ok_or(Undeclared::Malformed)?;
    let (kind, paths) = words.split_first().ok_or(Undeclared::Malformed)?;
    let kind = PartKind::named(kind).ok_or(Undeclared::Malformed)?;
    let mut paths: Vec<&str> = paths.to_vec();
    if paths.iter().any(|p| !is_path(p)) {
        return Err(Undeclared::Malformed);
    }
    let mut sorted = paths.clone();
    sorted.sort_unstable();
    if sorted.windows(2).any(|w| w[0] == w[1]) {
        return Err(Undeclared::Malformed);
    }
    if paths.is_empty() {
        paths.push(WHOLE);
    }
    let literal_shape = decl.kind == DeclKind::Fn
        && decl.generics.is_empty()
        && decl.params.is_empty()
        && decl.effects.is_empty()
        && decl.charge.is_none()
        && decl.body.lets.is_empty();
    if !literal_shape {
        return Err(Undeclared::NotLiteral);
    }
    let mut fields = Vec::new();
    for path in paths {
        let ty = field_type(&decl.result, path).ok_or(Undeclared::NotLiteral)?;
        let expr = field_expr(&decl.body.result, path).ok_or(Undeclared::NotLiteral)?;
        let value = PartValue::of(expr).ok_or(Undeclared::NotLiteral)?;
        let span = expr.loc.0;
        fields.push(DeclaredField {
            path: path.to_owned(),
            ty: type_text(ty),
            value,
            range: SourceRange::new(digest, span.start, span.end),
        });
    }
    let span = decl.loc.0;
    Ok(DeclaredPart {
        declaration: decl.name.text.clone(),
        kind,
        range: SourceRange::new(digest, span.start, span.end),
        directive: SourceRange::new(digest, directive.start, directive.end),
        fields,
    })
}

/// A dotted path of names, or [`WHOLE`].
fn is_path(path: &str) -> bool {
    path.split('.').all(|n| {
        let mut chars = n.chars();
        chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

/// The declared type a path names inside a result type.
fn field_type<'t>(ty: &'t TypeExpr, path: &str) -> Option<&'t TypeExpr> {
    if path == WHOLE {
        return Some(ty);
    }
    let mut at = ty;
    for name in path.split('.') {
        let TypeExpr::Record { fields, .. } = at else { return None };
        at = &fields.iter().find(|(n, _)| n.text == name)?.1;
    }
    Some(at)
}

/// The expression a path names inside a returned literal.
fn field_expr<'e>(expr: &'e Expr, path: &str) -> Option<&'e Expr> {
    if path == WHOLE {
        return Some(expr);
    }
    let mut at = expr;
    for name in path.split('.') {
        let ExprKind::Record(fields) = &at.kind else { return None };
        at = &fields.iter().find(|(n, _)| n.text == name)?.1;
    }
    Some(at)
}

/// Whether a value has a declared type; otherwise the type's text and what was supplied: the value's kind, or
/// `malformed <kind>` for a value no literal of its kind spells.
fn conforms(ty: &TypeExpr, value: &PartValue) -> Result<(), (String, String)> {
    let expected = type_text(ty);
    if !well_formed(value) {
        return Err((expected, format!("malformed {}", value.kind())));
    }
    let ok = match (ty, value) {
        (TypeExpr::Record { fields, .. }, PartValue::Record(values)) => {
            fields.len() == values.len()
                && fields.iter().zip(values).all(|((n, t), (m, v))| n.text == *m && conforms(t, v).is_ok())
        }
        (TypeExpr::Named { name, args, .. }, _) => match (name.text.as_str(), args.as_slice(), value) {
            ("Bool", [], PartValue::Bool(_))
            | ("Int", [], PartValue::Int(_))
            | ("Rat", [], PartValue::Rat { .. })
            | ("Q5", [], PartValue::Q5 { .. })
            | ("Text", [], PartValue::Text(_))
            | ("Word", [], PartValue::Word(_)) => true,
            ("Nat", [], PartValue::Int(n)) => !n.starts_with('-'),
            ("Approx", [TypeExpr::Named { name: p, args: none, .. }], PartValue::Approx { precision, .. }) => {
                none.is_empty() && p.text == format!("P{precision}")
            }
            _ => false,
        },
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err((expected, value.kind().to_owned()))
    }
}

/// Whether some literal of the value's kind spells it.
fn well_formed(value: &PartValue) -> bool {
    match value {
        PartValue::Bool(_) | PartValue::Text(_) => true,
        PartValue::Int(n) => canonical_int(n),
        PartValue::Rat { num, den } => ratio(num, den),
        PartValue::Q5 { one, sqrt5 } => ratio(&one.0, &one.1) && ratio(&sqrt5.0, &sqrt5.1),
        PartValue::Approx { precision, decimal } => matches!(precision, 24 | 53 | 113) && decimal_text(decimal),
        PartValue::Word(letters) => letters.iter().all(|l| (1..=1200).contains(&l.unsigned_abs())),
        PartValue::Record(fields) => {
            let mut names: Vec<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
            names.sort_unstable();
            names.windows(2).all(|w| w[0] != w[1])
                && fields.iter().all(|(n, v)| is_path(n) && !n.contains('.') && well_formed(v))
        }
    }
}

/// `0` or an optional `-` and digits with no leading zero.
fn canonical_int(n: &str) -> bool {
    let digits = n.strip_prefix('-').unwrap_or(n);
    !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && (digits == "0" && !n.starts_with('-') || !digits.starts_with('0'))
}

/// Canonical integers with a nonzero, unsigned denominator.
fn ratio(num: &str, den: &str) -> bool {
    canonical_int(num) && canonical_int(den) && !den.starts_with('-') && den != "0"
}

/// A finite decimal as the lexer reads one: an optional sign, digits, an optional fraction and an optional exponent.
fn decimal_text(d: &str) -> bool {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let signed = |s: &str| digits(s.strip_prefix(['+', '-']).unwrap_or(s));
    let (mantissa, exponent) = d.split_once(['e', 'E']).map_or((d, None), |(m, e)| (m, Some(e)));
    let (whole, fraction) = mantissa.split_once('.').map_or((mantissa, None), |(w, f)| (w, Some(f)));
    signed(whole) && fraction.is_none_or(digits) && exponent.is_none_or(signed)
}

/// A text literal with the lexer's escapes.
fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
