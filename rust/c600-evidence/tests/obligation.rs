//! LP5 piece R9: the proof-obligation translator (LB5.18; PS1.45, PS1.46; RTE.28; EVD.47, EVD.64).
//!
//! The format packet's `obligations` vectors are read in place from
//! `docs/architecture/1.0/language/format-packet/vectors.json`; expected text and codes come from those vectors and
//! from PS1.46's forms, never from the translator's own output. No Lean runs (owner, 23 November 2026, LP5-2). The
//! `file` overlay row `research.obligation.export`, its file grant and the file it writes belong to `c600-research`, so
//! each test names that half as not run. Evidence kind: source.

use std::path::{Path, PathBuf};

use c600_evidence::obligation::json::{self, Json};
use c600_evidence::obligation::{self, Obligation, ObligationError, Reason, SymbolTable};
use c600_evidence::{check_write, Basis, Check, Citation, Form, Grade, Statement, StatementId, Writer};

/// LB5.18, PS1.46 and chapter 13's registry.
const UNTRANSLATABLE: &str = "LB5:input.obligation_untranslatable";
const DIGEST_MISMATCH: &str = "PS1:input.digest_mismatch";
const NONCANONICAL_VALUE: &str = "PS1:input.noncanonical_value";
const PERSISTENCE_SIZE: &str = "PS1:resource_limit.persistence_size";
/// EVD.64, EVD.65.
const FIELD_NOT_WRITABLE: &str = "EVD:capability.field_not_writable";

/// The module's sources, for the scans.
const MODULE: [(&str, &str); 2] =
    [("mod.rs", include_str!("../src/obligation/mod.rs")), ("json.rs", include_str!("../src/obligation/json.rs"))];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repository() -> PathBuf {
    crate_dir().join("..").join("..")
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The `obligations` vectors of the format packet.
fn vectors() -> Vec<Json> {
    let path = repository().join("docs/architecture/1.0/language/format-packet/vectors.json");
    let all = json::parse(&read(&path)).expect("vectors.json is JSON");
    all.get("obligations").and_then(Json::as_array).expect("an obligations group").to_vec()
}

fn text<'a>(value: &'a Json, key: &str) -> &'a str {
    value.get(key).and_then(Json::as_text).unwrap_or_else(|| panic!("{key} is text"))
}

/// A vector's statement, as chapter 06's typed `Statement`.
fn statement_of(given: &Json) -> Statement {
    let statement = given.get("statement").expect("a statement");
    let id = statement.get("id").expect("an id");
    let id = match text(id, "kind") {
        "typed" => {
            let content = id.get("content").expect("a content digest");
            assert_eq!(text(content, "role"), "content");
            StatementId::typed(text(content, "sha256"))
        }
        "named" => StatementId::named(text(id, "stage"), text(id, "commit"), text(id, "name")),
        other => panic!("an id of kind {other}"),
    };
    let mut out = Statement::new(id);
    if statement.get("form").map(|form| text(form, "kind")) == Some("universal") {
        out = out.form(Form::Universal);
    }
    out
}

fn canonical(value: &Json) -> Vec<u8> {
    json::canonical(value).expect("canonical bytes")
}

fn draft_table() -> SymbolTable {
    SymbolTable::draft_1().expect("the draft 1 table reads")
}

/// A typed statement whose StatementId names `body`, a JSON text, with the body's canonical bytes.
fn typed(body: &str) -> (Statement, Vec<u8>) {
    let bytes = canonical(&json::parse(body.as_bytes()).expect("the body is JSON"));
    (Statement::new(StatementId::typed(&c600_digest::sha256_hex(&bytes))), bytes)
}

fn export(body: &str) -> Result<Obligation, ObligationError> {
    let (statement, bytes) = typed(body);
    obligation::translate(&statement, Some(&bytes), &draft_table())
}

/// A refused export: its error. A refusal carries no text, so nothing can be written.
fn refused(result: Result<Obligation, ObligationError>, label: &str) -> ObligationError {
    match result {
        Ok(out) => panic!("{label}: exported {:?}", out.source),
        Err(error) => error,
    }
}

fn body(conclusion: &str) -> String {
    format!(r#"{{"hypotheses":[],"conclusion":{conclusion}}}"#)
}

/// Every form of PS1.46's vocabulary, each with the Lean text written from PS1.46 and the reference reader's
/// rendering rules.
const SUPPORTED: [(&str, &str); 8] = [
    (r#"{"hypotheses":[],"conclusion":{"kind":"true"}}"#, "theorem obligation : True := by sorry\n"),
    (r#"{"hypotheses":[],"conclusion":{"kind":"false"}}"#, "theorem obligation : False := by sorry\n"),
    (
        r#"{"hypotheses":[{"kind":"eq","a":{"kind":"nat","value":"0"},"b":{"kind":"nat","value":"12"}}],
            "conclusion":{"kind":"lt","a":{"kind":"int","value":"-3"},"b":{"kind":"int","value":"4"}}}"#,
        "theorem obligation (h0 : (0 = 12)) : ((-3 : Int) < (4 : Int)) := by sorry\n",
    ),
    (
        r#"{"hypotheses":[],"conclusion":{"kind":"le","a":{"kind":"rat","n":"1","d":"2"},
            "b":{"kind":"rat","n":"-3","d":"4"}}}"#,
        "theorem obligation : (((1 : Rat) / 2) ≤ ((-3 : Rat) / 4)) := by sorry\n",
    ),
    (
        r#"{"hypotheses":[{"kind":"not","p":{"kind":"true"}},{"kind":"and","p":{"kind":"true"},"q":{"kind":"false"}}],
            "conclusion":{"kind":"or","p":{"kind":"false"},"q":{"kind":"implies","p":{"kind":"true"},"q":{"kind":"true"}}}}"#,
        "theorem obligation (h0 : (¬ True)) (h1 : (True ∧ False)) : (False ∨ (True → True)) := by sorry\n",
    ),
    (
        r#"{"hypotheses":[],"conclusion":{"kind":"exists","name":"y","sort":{"kind":"Nat"},
            "p":{"kind":"eq","a":{"kind":"var","name":"y"},"b":{"kind":"nat","value":"7"}}}}"#,
        "theorem obligation : (∃ y : Nat, (y = 7)) := by sorry\n",
    ),
    (
        r#"{"hypotheses":[],"conclusion":{"kind":"forall","name":"a","sort":{"kind":"Int"},
            "p":{"kind":"exists","name":"b","sort":{"kind":"Rat"},
                 "p":{"kind":"lt","a":{"kind":"var","name":"a"},"b":{"kind":"var","name":"b"}}}}}"#,
        "theorem obligation : (∀ a : Int, (∃ b : Rat, (a < b))) := by sorry\n",
    ),
    (
        r#"{"hypotheses":[{"kind":"pred","name":"member","args":[{"kind":"sym","name":"orbit","args":[]},
                                                               {"kind":"sym","name":"base","args":[]}]}],
            "conclusion":{"kind":"forall","name":"x","sort":{"kind":"named","name":"Piece"},
            "p":{"kind":"eq","a":{"kind":"sym","name":"grade","args":[{"kind":"var","name":"x"}]},
                 "b":{"kind":"sym","name":"grade","args":[{"kind":"sym","name":"base","args":[]}]}}}}"#,
        "theorem obligation (h0 : (C600.member C600.orbit C600.base)) : \
         (∀ x : C600.Piece, ((C600.grade x) = (C600.grade C600.base))) := by sorry\n",
    ),
];

/// The tokens an export may hold: the theorem's frame, PS1.46's marks and sorts, numerals, hypothesis names, the
/// table's Lean identifiers and the body's own bound names.
fn tokens_are_vocabulary(source: &str, table_lean: &[&str], bound: &[String]) -> Result<(), String> {
    let frame = ["theorem", "obligation", ":", ":=", "by", "sorry", "True", "False", "Nat", "Int", "Rat", "/"];
    let marks = ["=", "≤", "<", "¬", "∧", "∨", "→", "∀", "∃"];
    let spaced: String = source.chars().map(|c| if matches!(c, '(' | ')' | ',') { ' ' } else { c }).collect();
    for token in spaced.split_whitespace() {
        let numeral = {
            let digits = token.strip_prefix('-').unwrap_or(token);
            !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
        };
        let hypothesis =
            token.strip_prefix('h').is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
        let known = frame.contains(&token)
            || marks.contains(&token)
            || table_lean.contains(&token)
            || bound.iter().any(|b| b == token);
        if !(numeral || hypothesis || known) {
            return Err(format!("{token:?} is outside the vocabulary"));
        }
    }
    Ok(())
}

/// The names a body binds with `forall` or `exists`.
fn bound_names(value: &Json, out: &mut Vec<String>) {
    match value {
        Json::Object(fields) => {
            if matches!(fields.get("kind").and_then(Json::as_text), Some("forall" | "exists")) {
                if let Some(name) = fields.get("name").and_then(Json::as_text) {
                    out.push(name.to_string());
                }
            }
            fields.values().for_each(|child| bound_names(child, out));
        }
        Json::Array(items) => items.iter().for_each(|child| bound_names(child, out)),
        _ => {}
    }
}

// --- CT-PS1-26 ------------------------------------------------------------------------------------------------

/// CT-PS1-26 (part: the translator; not run: the export through a granted file capability, which is
/// `c600-research`'s `research.obligation.export` row): PS1.45 and PS1.46. The format packet's supported typed
/// statement exports its exact Lean 4 text, with `by sorry`, the format, the vocabulary and the statement and table
/// pins; the unknown symbol, the wrong arity and the named statement refuse with
/// `LB5:input.obligation_untranslatable`, and the body whose bytes differ from the StatementId's digest refuses with
/// `PS1:input.digest_mismatch`. Each refusal returns no text.
#[test]
fn ct_ps1_26_format_packet_vectors_translate_byte_for_byte() {
    let vectors = vectors();
    let ids: Vec<&str> = vectors.iter().map(|v| text(v, "id")).collect();
    assert_eq!(ids, ["supported-statement", "unknown-symbol", "wrong-arity", "named-statement", "body-digest-differs"]);
    let mut supported = 0;
    for vector in &vectors {
        let id = text(vector, "id");
        assert_eq!(text(vector, "clause"), "PS1.46", "{id}");
        let given = vector.get("given").expect("given");
        let statement = statement_of(given);
        let table_json = given.get("table").expect("a table");
        let table = SymbolTable::from_json(table_json, None).expect("the vector's table reads");
        let bytes = canonical(given.get("body").expect("a body"));
        let result = obligation::translate(&statement, Some(&bytes), &table);
        match vector.get("expected").expect("expected") {
            Json::Text(code) => {
                let error = refused(result, id);
                assert_eq!(error.code, code.as_str(), "{id}");
                let reason = match id {
                    "unknown-symbol" => Reason::UnknownSymbol,
                    "wrong-arity" => Reason::Arity,
                    "named-statement" => Reason::NamedStatement,
                    "body-digest-differs" => Reason::Digest,
                    other => panic!("an unexpected refusal vector {other}"),
                };
                assert_eq!(error.reason, reason, "{id}");
            }
            expected => {
                supported += 1;
                let out = result.unwrap_or_else(|e| panic!("{id}: {e}"));
                assert_eq!(out.format, text(expected, "format"), "{id}");
                assert_eq!(out.vocabulary, text(expected, "vocabulary"), "{id}");
                assert_eq!(out.source.as_bytes(), text(expected, "source").as_bytes(), "{id}");
                assert_eq!(out.sha256, text(expected, "sha256"), "{id}");
                assert_eq!(out.sha256, c600_digest::sha256_hex(out.source.as_bytes()), "{id}");
                assert!(out.source.ends_with(" := by sorry\n") && !out.source.contains('\r'), "{id}");
                let StatementId::Typed { content } = &statement.id else { panic!("{id}: a typed statement") };
                assert_eq!(&out.pins.statement, content, "{id}");
                assert_eq!(out.pins.symbols, c600_digest::sha256_hex(&canonical(table_json)), "{id}");
                assert_eq!(out.pins.prelude, None, "{id}: the fixture table names no prelude");
                // The shipped table, draft 1, is the vector's table, so the shipped export is the same text with the
                // prelude pinned.
                assert_eq!(canonical(table_json), obligation::SYMBOLS_DRAFT_1, "{id}");
                let shipped = obligation::translate(&statement, Some(&bytes), &draft_table()).expect("shipped table");
                assert_eq!((&shipped.source, &shipped.sha256), (&out.source, &out.sha256), "{id}");
                assert_eq!(shipped.pins.symbols, obligation::SYMBOLS_DRAFT_1_SHA256, "{id}");
                assert_eq!(shipped.pins.prelude.as_deref(), Some(obligation::PRELUDE_DRAFT_1_SHA256), "{id}");
            }
        }
    }
    assert_eq!(supported, 1);
}

/// PS1.46: the pins name the versioned data. The table's pin is the SHA-256 of its file, the file is the canonical
/// bytes of its rows, and the prelude's pin is the SHA-256 of the prelude file in the repository.
#[test]
fn ps1_46_versioned_table_and_prelude_pins_hold() {
    let table = draft_table();
    assert_eq!(c600_digest::sha256_hex(obligation::SYMBOLS_DRAFT_1), obligation::SYMBOLS_DRAFT_1_SHA256);
    assert_eq!(table.digest(), obligation::SYMBOLS_DRAFT_1_SHA256);
    assert_eq!(table.vocabulary(), obligation::VOCABULARY);
    assert_eq!(table.prelude(), Some(obligation::PRELUDE_DRAFT_1_SHA256));
    let on_disk = read(&crate_dir().join("obligation/symbols-draft-1.json"));
    assert_eq!(on_disk, obligation::SYMBOLS_DRAFT_1);
    assert!(json::read_canonical(&on_disk).is_ok(), "the table file is canonical");
    let prelude = read(&crate_dir().join("obligation/prelude-draft-1.lean"));
    assert_eq!(c600_digest::sha256_hex(&prelude), obligation::PRELUDE_DRAFT_1_SHA256);
    assert!(!prelude.contains(&b'\r'), "the prelude is LF text");
    let prelude = String::from_utf8(prelude).expect("the prelude is UTF-8");
    // Every Lean identifier of the table is declared by the prelude, with an arrow per argument.
    for (name, arity) in [("base", 0), ("grade", 1), ("member", 2), ("orbit", 0)] {
        let row = table.symbol(name).expect("a draft 1 row");
        assert_eq!(row.arity, arity, "{name}");
        let short = row.lean.strip_prefix("C600.").expect("a C600 identifier");
        let line = prelude.lines().find(|l| l.starts_with(&format!("axiom {short} : "))).expect("a declaration");
        assert_eq!(line.matches('→').count(), arity, "{name}");
    }
    let sort = table.sort("Piece").expect("the Piece sort");
    assert!(prelude.lines().any(|l| l == format!("axiom {} : Type", sort.trim_start_matches("C600."))));
    assert!(prelude.contains("namespace C600") && prelude.contains("end C600"));
}

// --- CT-LB5-11 ------------------------------------------------------------------------------------------------

/// CT-LB5-11 (part: the translator half; not run: the file grant that `research.obligation.export` requires and that
/// a refused export writes no file, which are `c600-research`'s): LB5.18. The translator writes only the restricted
/// versioned vocabulary: each of PS1.46's forms gives its exact Lean text, and every token of every export is the
/// theorem's frame, a mark or sort of the vocabulary, a numeral, a hypothesis name, a Lean identifier of the table or a
/// name the body binds. A statement named by ID, unresolved bytes, a table of another vocabulary, an unknown symbol, a
/// wrong arity, an unbound variable and every form outside the vocabulary refuse with
/// `LB5:input.obligation_untranslatable` and return no text. The prelude and Lean stay outside the shipped software:
/// the prelude is not compiled in and the crate has no Lean dependency.
#[test]
fn ct_lb5_11_translator_writes_only_the_vocabulary() {
    let table = draft_table();
    let lean: Vec<&str> = ["base", "grade", "member", "orbit"]
        .iter()
        .map(|name| table.symbol(name).expect("a row").lean.as_str())
        .chain([table.sort("Piece").expect("a sort")])
        .collect();
    for (text, expected) in SUPPORTED {
        let out = export(text).unwrap_or_else(|e| panic!("{text}: {e}"));
        assert_eq!(out.source, expected);
        let mut bound = Vec::new();
        bound_names(&json::parse(text.as_bytes()).expect("JSON"), &mut bound);
        tokens_are_vocabulary(&out.source, &lean, &bound).unwrap_or_else(|e| panic!("{expected}: {e}"));
    }
    // The token check is not vacuous.
    assert!(tokens_are_vocabulary("theorem obligation : (#eval 1) := by sorry\n", &lean, &[]).is_err());

    let pred = |name: &str, args: &str| format!(r#"{{"kind":"pred","name":"{name}","args":[{args}]}}"#);
    let orbit = r#"{"kind":"sym","name":"orbit","args":[]}"#;
    let nat0 = r#"{"kind":"nat","value":"0"}"#;
    let eq = |a: &str, b: &str| format!(r#"{{"kind":"eq","a":{a},"b":{b}}}"#);
    let forall =
        |name: &str, sort: &str, p: &str| format!(r#"{{"kind":"forall","name":"{name}","sort":{sort},"p":{p}}}"#);
    let nat_sort = r#"{"kind":"Nat"}"#;
    let cases: Vec<(String, Reason)> = vec![
        // Unknown symbols: a predicate, a term symbol and a named sort outside the table.
        (body(&pred("absent", "")), Reason::UnknownSymbol),
        (body(&eq(r#"{"kind":"sym","name":"nothing","args":[]}"#, nat0)), Reason::UnknownSymbol),
        (body(&forall("x", r#"{"kind":"named","name":"Orbit"}"#, r#"{"kind":"true"}"#)), Reason::UnknownSymbol),
        // Wrong arities.
        (body(&pred("member", orbit)), Reason::Arity),
        (body(&pred("member", &[orbit, orbit, orbit].join(","))), Reason::Arity),
        (body(&eq(r#"{"kind":"sym","name":"grade","args":[]}"#, nat0)), Reason::Arity),
        (body(&eq(r#"{"kind":"sym","name":"base","args":[{"kind":"nat","value":"1"}]}"#, nat0)), Reason::Arity),
        // Unbound variables, including one bound only in another hypothesis.
        (body(&eq(r#"{"kind":"var","name":"x"}"#, nat0)), Reason::UnboundVariable),
        (
            format!(
                r#"{{"hypotheses":[{}],"conclusion":{}}}"#,
                forall("x", nat_sort, r#"{"kind":"true"}"#),
                eq(r#"{"kind":"var","name":"x"}"#, nat0)
            ),
            Reason::UnboundVariable,
        ),
        // Forms outside the vocabulary.
        (body(r#"{"kind":"iff","p":{"kind":"true"},"q":{"kind":"true"}}"#), Reason::Form),
        (body(&eq(r#"{"kind":"real","value":"1"}"#, nat0)), Reason::Form),
        (body(&forall("x", r#"{"kind":"Bool"}"#, r#"{"kind":"true"}"#)), Reason::Form),
        (body(r#"{"kind":"true","note":"extra"}"#), Reason::Form),
        (body(r#"{"kind":"eq","a":{"kind":"nat","value":"0"}}"#), Reason::Form),
        (body(r#"{"kind":1}"#), Reason::Form),
        (body(r#"{"p":{"kind":"true"}}"#), Reason::Form),
        (body(r#""true""#), Reason::Form),
        (body(&pred("member", "").replace(r#""args":[]"#, r#""args":{}"#)), Reason::Form),
        (r#"{"hypotheses":[],"conclusion":{"kind":"true"},"proof":"rfl"}"#.to_string(), Reason::Form),
        (r#"{"hypotheses":{},"conclusion":{"kind":"true"}}"#.to_string(), Reason::Form),
        (r#"{"conclusion":{"kind":"true"}}"#.to_string(), Reason::Form),
        (r#"[]"#.to_string(), Reason::Form),
        (body(&eq(r#"{"kind":"nat","value":"-1"}"#, nat0)), Reason::Form),
        (body(&eq(r#"{"kind":"rat","n":"1","d":"0"}"#, nat0)), Reason::Form),
        (body(&eq(r#"{"kind":"rat","n":"1","d":"-2"}"#, nat0)), Reason::Form),
        // A bound name becomes a Lean binder: keywords and other text are outside the vocabulary.
        (body(&forall("fun", nat_sort, r#"{"kind":"true"}"#)), Reason::Form),
        (body(&forall("sorry", nat_sort, r#"{"kind":"true"}"#)), Reason::Form),
        (body(&forall("x y", nat_sort, r#"{"kind":"true"}"#)), Reason::Form),
        (body(&forall("1x", nat_sort, r#"{"kind":"true"}"#)), Reason::Form),
        (body(&forall("x) (h : False", nat_sort, r#"{"kind":"true"}"#)), Reason::Form),
        (body(&forall("", nat_sort, r#"{"kind":"true"}"#)), Reason::Form),
        (body(&forall("with", nat_sort, r#"{"kind":"true"}"#)), Reason::Form),
        (body(&forall("universe", nat_sort, r#"{"kind":"true"}"#)), Reason::Form),
        // A binder may not hide a name the output writes: PS1.46's constants and sorts, or a table identifier's root.
        (body(&forall("True", nat_sort, r#"{"kind":"true"}"#)), Reason::Form),
        (body(&forall("Nat", nat_sort, r#"{"kind":"true"}"#)), Reason::Form),
        (body(&forall("C600", nat_sort, r#"{"kind":"true"}"#)), Reason::Form),
    ];
    for (text, reason) in &cases {
        let error = refused(export(text), text);
        assert_eq!((error.code, error.reason), (UNTRANSLATABLE, *reason), "{text}");
    }

    // A statement named by ID has no body to translate, whatever bytes come with it.
    let (_, bytes) = typed(SUPPORTED[0].0);
    let named = Statement::new(StatementId::named("stage-14", "fixture", "fixture statement"));
    for body in [Some(bytes.as_slice()), None] {
        let error = refused(obligation::translate(&named, body, &table), "named");
        assert_eq!((error.code, error.reason), (UNTRANSLATABLE, Reason::NamedStatement));
    }
    // Bytes that did not resolve: no typed body.
    let (statement, _) = typed(SUPPORTED[0].0);
    let error = refused(obligation::translate(&statement, None, &table), "unresolved");
    assert_eq!((error.code, error.reason), (UNTRANSLATABLE, Reason::NoBody));
    // A table of another vocabulary.
    let other = String::from_utf8(obligation::SYMBOLS_DRAFT_1.to_vec()).expect("UTF-8").replace("draft-1", "draft-2");
    let other = SymbolTable::read(other.as_bytes(), None).expect("a table of another vocabulary reads");
    let error = refused(obligation::translate(&statement, Some(&bytes), &other), "vocabulary");
    assert_eq!((error.code, error.reason), (UNTRANSLATABLE, Reason::Vocabulary));
    // A table outside its form never reaches a translation.
    for table in [
        r#"{"vocabulary":"c600-obligation-vocabulary-draft-1","symbols":{}}"#,
        r#"{"vocabulary":"c600-obligation-vocabulary-draft-1","symbols":{"f":{"lean":"C600.f","arity":-1}},"sorts":{}}"#,
        r#"{"vocabulary":"c600-obligation-vocabulary-draft-1","symbols":{"f":{"lean":"C600.f x","arity":1}},"sorts":{}}"#,
        r#"{"vocabulary":"c600-obligation-vocabulary-draft-1","symbols":{},"sorts":{"S":"C600.end"}}"#,
    ] {
        let error = SymbolTable::from_json(&json::parse(table.as_bytes()).expect("JSON"), None).expect_err(table);
        assert_eq!((error.code, error.reason), (UNTRANSLATABLE, Reason::Table), "{table}");
    }

    // The prelude stays outside the shipped software: no source of the module compiles it in, it is not under src/,
    // and the crate has no Lean dependency or build script.
    for (name, source) in MODULE {
        for line in source.lines().filter(|l| l.contains("include_str!") || l.contains("include_bytes!")) {
            assert!(!line.contains(".lean"), "{name} compiles the prelude in: {line}");
        }
    }
    assert!(crate_dir().join("obligation/prelude-draft-1.lean").is_file());
    let manifest = String::from_utf8(read(&crate_dir().join("Cargo.toml"))).expect("UTF-8");
    let dependencies: Vec<&str> = manifest
        .split("[dependencies]")
        .nth(1)
        .unwrap_or("")
        .lines()
        .take_while(|l| !l.trim_start().starts_with('['))
        .filter_map(|l| l.split('=').next().map(str::trim).filter(|n| !n.is_empty() && !n.starts_with('#')))
        .collect();
    assert_eq!(dependencies, ["c600-digest"]);
    assert!(!manifest.to_lowercase().contains("lean"), "no Lean dependency");
    assert!(!crate_dir().join("build.rs").exists(), "no build script");
}

/// PS1.46 and the reference reader's `int_value`: an exact integer of the body that is not canonical decimal text
/// refuses with `PS1:input.noncanonical_value`, and one with more digits than the format packet's limit with
/// `PS1:resource_limit.persistence_size`; body bytes that are not canonical refuse with
/// `PS1:input.noncanonical_value` even when the StatementId names them, and canonical bytes that the StatementId does
/// not name refuse with `PS1:input.digest_mismatch`. The checks run in the reference reader's order.
#[test]
fn ps1_46_body_bytes_and_integers_refuse_with_their_codes() {
    let nat0 = r#"{"kind":"nat","value":"0"}"#;
    for value in [r#""007""#, r#""-0""#, r#""+1""#, r#""1.0""#, r#""""#, "7"] {
        let text = body(&format!(r#"{{"kind":"eq","a":{{"kind":"nat","value":{value}}},"b":{nat0}}}"#));
        let error = refused(export(&text), &text);
        assert_eq!((error.code, error.reason), (NONCANONICAL_VALUE, Reason::Noncanonical), "{text}");
    }
    let long = "9".repeat(json::INTEGER_DIGITS);
    let fits = body(&format!(r#"{{"kind":"eq","a":{{"kind":"int","value":"-{long}"}},"b":{nat0}}}"#));
    assert!(export(&fits).is_ok(), "the digit limit itself is admitted");
    let over = body(&format!(r#"{{"kind":"eq","a":{{"kind":"int","value":"{long}9"}},"b":{nat0}}}"#));
    let error = refused(export(&over), "over the digit limit");
    assert_eq!((error.code, error.reason), (PERSISTENCE_SIZE, Reason::Size));

    let table = draft_table();
    let pretty = b"{\"conclusion\": {\"kind\": \"true\"}, \"hypotheses\": []}\n".to_vec();
    let statement = Statement::new(StatementId::typed(&c600_digest::sha256_hex(&pretty)));
    let error = refused(obligation::translate(&statement, Some(&pretty), &table), "pretty bytes");
    assert_eq!((error.code, error.reason), (NONCANONICAL_VALUE, Reason::Noncanonical));

    let (statement, _) = typed(SUPPORTED[0].0);
    let (_, other) = typed(SUPPORTED[1].0);
    let error = refused(obligation::translate(&statement, Some(&other), &table), "another body");
    assert_eq!((error.code, error.reason), (DIGEST_MISMATCH, Reason::Digest));
    // The vocabulary check comes before the body's bytes, and the named check before both.
    let other_vocabulary =
        String::from_utf8(obligation::SYMBOLS_DRAFT_1.to_vec()).expect("UTF-8").replace("draft-1", "draft-9");
    let other_vocabulary = SymbolTable::read(other_vocabulary.as_bytes(), None).expect("reads");
    let error = refused(obligation::translate(&statement, Some(&other), &other_vocabulary), "order");
    assert_eq!(error.reason, Reason::Vocabulary);
    // The same statement and bytes give the same export on every call.
    let (statement, bytes) = typed(SUPPORTED[7].0);
    let first = obligation::translate(&statement, Some(&bytes), &table).expect("exports");
    assert_eq!(obligation::translate(&statement, Some(&bytes), &table).expect("exports"), first);
}

/// The reader of canonical bytes refuses exactly what the format packet's reference reader refuses: whitespace,
/// unsorted or repeated keys, a missing final line feed, fractions, exponents, `-0`, leading zeros, integers beyond
/// 2^53 - 1, escapes canonical bytes do not use, lone surrogates and bytes that are not UTF-8 give
/// `PS1:input.noncanonical_value`; more than 64 nested containers give `PS1:resource_limit.persistence_size`.
#[test]
fn canonical_json_reads_as_the_reference_reader() {
    let value = json::parse(" {\"b\" : 1,\n \"a\":[true,null,\"\u{e9}\\u0001\\n\\/\\ud83d\\ude00\",-0]} ".as_bytes())
        .expect("JSON text");
    let bytes = canonical(&value);
    assert_eq!(bytes, "{\"a\":[true,null,\"\u{e9}\\u0001\\n/\u{1f600}\",0],\"b\":1}\n".as_bytes());
    assert_eq!(json::read_canonical(&bytes).expect("canonical"), json::parse(&bytes).expect("JSON"));
    let safe = b"[9007199254740991,-9007199254740991]\n";
    assert!(json::read_canonical(safe).is_ok());
    for raw in [
        &b"{\"a\":1}"[..],
        b"{\"a\": 1}\n",
        b"{\"b\":1,\"a\":2}\n",
        b"{\"a\":1,\"a\":1}\n",
        b"[1.0]\n",
        b"[1e2]\n",
        b"[-0]\n",
        b"[01]\n",
        b"[9007199254740992]\n",
        b"[-9007199254740992]\n",
        b"[\"\\u00e9\"]\n",
        b"[\"\\u001F\"]\n",
        b"[\"\\/\"]\n",
        b"[\"\\ud800\"]\n",
        b"[\"\\udc00\"]\n",
        b"[\"\x01\"]\n",
        b"[\"\xff\"]\n",
        b"[NaN]\n",
        b"\xef\xbb\xbf[]\n",
        b"[]\n\n",
        b"",
    ] {
        let error = json::read_canonical(raw).expect_err(&String::from_utf8_lossy(raw));
        assert_eq!((error.code, error.reason), (NONCANONICAL_VALUE, Reason::Noncanonical), "{raw:?}");
    }
    let nest = |n: usize| format!("{}{}\n", "[".repeat(n), "]".repeat(n)).into_bytes();
    assert!(json::read_canonical(&nest(json::JSON_DEPTH)).is_ok());
    let error = json::read_canonical(&nest(json::JSON_DEPTH + 1)).expect_err("one container too deep");
    assert_eq!((error.code, error.reason), (PERSISTENCE_SIZE, Reason::Size));
}

// --- CT-RTE-23 ------------------------------------------------------------------------------------------------

/// CT-RTE-23 (part: as far as the pure translator shows it; not run: LB5.18's file grant and the `file` row's command
/// path, which are `c600-research`'s): RTE.28 and EVD.64. The export is PS1.45's `c600-proof-obligation-draft`; the
/// crate has no Lean dependency; the module reaches nothing in the crate but `Statement` and `StatementId`, so it reads
/// no proof record, defines no import row or check mode and writes no evidence field; no public function takes a
/// mutable reference, so an export changes nothing it is given and promotes no claim; and `formal proof` keeps no
/// writer: every writer's attempt is refused with `EVD:capability.field_not_writable`.
#[test]
fn ct_rte_23_export_promotes_nothing_and_has_no_return_path() {
    assert_eq!(obligation::FORMAT, "c600-proof-obligation-draft");
    assert_eq!(obligation::VOCABULARY, "c600-obligation-vocabulary-draft-1");
    let (statement, bytes) = typed(SUPPORTED[0].0);
    let out = obligation::translate(&statement, Some(&bytes), &draft_table()).expect("exports");
    assert_eq!(out.format, obligation::FORMAT);

    let uses: Vec<(&str, &str)> = MODULE
        .iter()
        .flat_map(|(name, source)| source.lines().filter(|l| l.starts_with("use ")).map(move |l| (*name, l)))
        .collect();
    assert_eq!(
        uses,
        [
            ("mod.rs", "use std::collections::BTreeMap;"),
            ("mod.rs", "use crate::model::{Statement, StatementId};"),
            ("mod.rs", "use json::Json;"),
            ("json.rs", "use std::collections::BTreeMap;"),
            ("json.rs", "use super::{ObligationError, Reason};"),
        ]
    );
    for (name, source) in MODULE {
        for banned in [
            "Graph",
            "Leaf",
            "Grade",
            "CheckFact",
            "CheckMode",
            "CheckReport",
            "Writer",
            "Citation",
            "Basis",
            "Check::",
            "admit_check_report",
            "check_write",
            "crate::sink",
            "crate::graph",
            "crate::word",
            "crate::import",
            "crate::edition",
        ] {
            assert!(!source.contains(banned), "{name} names {banned:?}");
        }
        for line in source.lines().filter(|l| l.contains("pub fn")) {
            assert!(!line.contains("&mut"), "{name}: a public function takes a mutable reference: {line}");
        }
    }
    // No check mode was added, and formal proof has no writer in 1.0.
    assert_eq!(Check::all().len(), 6);
    assert!(Check::all().contains(&Check::FormalProof));
    for writer in [Writer::Kernel, Writer::Host, Writer::Import, Writer::UserProgram] {
        for basis in Basis::all() {
            let citation = Citation::theorem("stage 19 record", &"a".repeat(40)).with_review(&"b".repeat(64));
            let error = check_write(writer, &Grade::new(basis.clone(), Check::FormalProof), &citation)
                .expect_err("formal proof has no writer");
            assert_eq!(error.code, FIELD_NOT_WRITABLE, "{writer:?} {basis:?}");
        }
    }
}

// --- CT-ERR-19 ------------------------------------------------------------------------------------------------

/// The cells of chapter 13's registry row for `code`, read in place.
fn registry_row(code: &str) -> Vec<String> {
    let path = repository().join("docs/architecture/1.0/language/architecture/13-errors.md");
    let text = String::from_utf8(read(&path)).expect("UTF-8");
    let prefix = format!("| `{code}` |");
    let rows: Vec<&str> = text.lines().filter(|l| l.starts_with(&prefix)).collect();
    assert_eq!(rows.len(), 1, "one registry row for {code}");
    rows[0].trim_matches('|').split(" | ").map(|cell| cell.trim().to_string()).collect()
}

/// CT-ERR-19 (part: the codes the translator and the writer check emit, with their chapter 13 rows read in place; not
/// run: that a refused `research.obligation.export` writes no file, which is `c600-research`'s row): LB5.18, EVD.64,
/// EVD.65 and ERR.14. An untranslatable obligation refuses with `LB5:input.obligation_untranslatable` and returns no
/// text; an unauthorized evidence field refuses with `EVD:capability.field_not_writable` and adds no leaf (the
/// writer check's whole table runs as CT-EVD-26 to CT-EVD-29 in `tests/sinks.rs`). Each code keeps its owning
/// chapter (03 for LB5, 06 for EVD), the refusal form and its category's retry, `after_user_change` for input and
/// capability (ERR.14).
#[test]
fn ct_err_19_refusals_keep_their_codes_chapters_and_retry() {
    let error = refused(export(&body(r#"{"kind":"pred","name":"absent","args":[]}"#)), "untranslatable");
    assert_eq!(error.code, UNTRANSLATABLE);
    assert_eq!(obligation::UNTRANSLATABLE, UNTRANSLATABLE);
    let written =
        check_write(Writer::UserProgram, &Grade::new(Basis::Certificate, Check::IndependentChecker), &Citation::none());
    assert_eq!(written.expect_err("an unauthorized field").code, FIELD_NOT_WRITABLE);
    assert_eq!(c600_evidence::error::FIELD_NOT_WRITABLE, FIELD_NOT_WRITABLE);

    for (code, category, chapter, rule) in
        [(UNTRANSLATABLE, "input", "03", "LB5.18"), (FIELD_NOT_WRITABLE, "capability", "06", "EVD.64")]
    {
        let (source, rest) = code.split_once(':').expect("a prefixed code");
        assert_eq!(rest.split_once('.').expect("a category").0, category, "{code}");
        assert!(matches!(source, "LB5" | "EVD"), "{code}");
        let row = registry_row(code);
        assert_eq!(row.len(), 6, "{code}: {row:?}");
        assert_eq!(row[1], "refusal", "{code}");
        assert_eq!(row[3], "`after_user_change`", "{code}");
        assert!(row[4].contains(rule), "{code}: {}", row[4]);
        assert!(row[5].starts_with(&format!("{chapter} ")), "{code}: {}", row[5]);
    }
}

// --- CT-EVD-20 for the obligation module ----------------------------------------------------------------------

/// CT-EVD-20 (part: the obligation module folder, which `tests/sinks.rs` also scans with the crate's other modules):
/// EVD.54. The module names no file, clock, process, network, environment,
/// thread or shared mutable state, and its folder holds exactly the two scanned sources.
#[test]
fn ct_evd_20_obligation_module_names_no_effect() {
    let dir = crate_dir().join("src").join("obligation");
    let mut found: Vec<String> = std::fs::read_dir(&dir)
        .expect("the module folder")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();
    assert_eq!(found, ["json.rs", "mod.rs"], "the scans must list every file of the module");
    for (name, source) in MODULE {
        for banned in [
            "std::fs",
            "std::io",
            "std::net",
            "std::process",
            "std::env",
            "std::time",
            "std::thread",
            "std::sync",
            "SystemTime",
            "Instant::",
            "File::",
            "Command::",
            "TcpStream",
            "env!(",
            "env::",
            "print!(",
            "println!(",
            "eprintln!(",
            "static mut",
            "thread_local!",
            "RefCell",
            "Cell<",
            "unsafe {",
            "unsafe fn",
            "for bool",
            "PartialEq<bool>",
            "Into<bool>",
            "Not for",
            "fn grant",
            "fn permit",
            "fn commit",
            "fn token",
            "fn preview",
            "fn capability",
            "fn authori",
        ] {
            assert!(!source.contains(banned), "{name} names {banned:?}");
        }
    }
}
