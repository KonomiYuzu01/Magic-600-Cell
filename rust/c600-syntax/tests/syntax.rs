//! LP2 plan section 8, acceptance checks 2, 3, 5 and 6: editions (CT-LG1-01), pinned source bytes (CT-LG1-02,
//! CT-ERR-05), recovery (CT-LG1-04) on candidate A's LX-8 drafts with the product header, and exact literals
//! (CT-LG1-05).

#[path = "../../c600-check/tests/support/mod.rs"]
mod support;

use c600_ir::{Expr as Ir, Lit};
use c600_syntax::ast::ExprKind;
use c600_syntax::code::*;
use c600_syntax::{lower, parse, sha256_hex, ParsedFile, Span};
use support::json::Json;

/// LX-8's surface experiment, read in place.
const LX8: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../work/experiments/language-lx/lx8-surfaces");
/// CT-LG1-02's fixture and its SHA-256 as the checkout holds it (`* -text`, LF line ends).
const PINNED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/canonical.6cs");
const PINNED_SHA256: &str = "53a5e2d303ee5d89245935d4f4dfe78c86de2bf3d57711d9f0d860a4a3ded0c2";

fn codes(file: &ParsedFile) -> Vec<&'static str> {
    file.diagnostics.iter().map(|d| d.code).collect()
}

/// LX-8's draft with the product's edition header in place of LX-8's.
fn product(bytes: &[u8]) -> Vec<u8> {
    let (old, new) = (b"edition \"lx8-0\";".as_slice(), b"edition \"6cs-draft-1\";".as_slice());
    match bytes.windows(old.len()).position(|w| w == old) {
        Some(i) => [&bytes[..i], new, &bytes[i + old.len()..]].concat(),
        None => bytes.to_vec(),
    }
}

/// A span's line and column agree with the bytes before it.
fn located(data: &[u8], span: Span, what: &str) {
    assert!(span.start <= span.end && span.end <= data.len(), "{what}: span outside the input");
    let before = std::str::from_utf8(&data[..span.start]).expect("a span starts on a character boundary");
    assert_eq!(span.line, before.matches('\n').count() + 1, "{what}: line");
    assert_eq!(span.column, before.rsplit('\n').next().unwrap().chars().count() + 1, "{what}: column");
}

fn manifest() -> Json {
    let text = std::fs::read_to_string(format!("{LX8}/fixtures/MANIFEST.json")).expect("LX-8 manifest");
    Json::parse(&text).expect("the manifest parses")
}

fn text<'a>(row: &'a Json, key: &str) -> &'a str {
    row.get(key).and_then(Json::str).unwrap_or_else(|| panic!("manifest row without `{key}`"))
}

fn texts<'a>(row: &'a Json, key: &str) -> Vec<&'a str> {
    row.get(key).and_then(Json::arr).expect(key).iter().map(|v| v.str().expect(key)).collect()
}

/// Candidate A's rows of a manifest list, each with its bytes, checked against the manifest's digest.
fn candidate_a(list: &str) -> Vec<(Json, Vec<u8>)> {
    let manifest = manifest();
    let rows = manifest.get(list).and_then(Json::arr).expect(list).to_vec();
    rows.into_iter()
        .filter(|row| text(row, "candidate") == "candidate-a")
        .map(|row| {
            let bytes = std::fs::read(format!("{LX8}/{}", text(&row, "path"))).expect("LX-8 draft");
            assert_eq!(sha256_hex(&bytes), text(&row, "sha256"), "{}: the LX-8 draft changed", text(&row, "id"));
            (row, bytes)
        })
        .collect()
}

/// CT-LG1-01 (part: a missing header in one file; the other files of a program and the mixed-edition clause are in
/// `conformance_lg1.rs`): the file is refused at an insertion point and nothing after it is read.
#[test]
fn ct_lg1_01_a_missing_header_is_refused_at_an_insertion_point() {
    let file = parse(b"module m requires [];\n\nfn f() -> Int effects [] {\n    return 1;\n}\n");
    assert_eq!(codes(&file), [EDITION_MISSING]);
    let span = file.diagnostics[0].span.unwrap();
    assert_eq!((span.start, span.end, span.line, span.column), (0, 0, 1, 1));
    assert!(file.module.is_none() && file.items.is_empty() && file.holes.is_empty());
}

/// CT-LG1-01 (part: an unsupported edition in one file; the other files of a program and the mixed-edition clause
/// are in `conformance_lg1.rs`): the file is refused at its edition without trying another grammar.
#[test]
fn ct_lg1_01_another_edition_is_refused_without_trying_another_grammar() {
    for edition in ["6cs-draft-2", "lx8-0", ""] {
        let source = format!(
            "edition \"{edition}\";\nmodule m requires [];\n\nfn f() -> Int effects [] {{\n    return 1;\n}}\n"
        );
        let file = parse(source.as_bytes());
        assert_eq!(codes(&file), [EDITION_UNSUPPORTED], "{edition}");
        let span = file.diagnostics[0].span.unwrap();
        assert_eq!(&source[span.start..span.end], format!("\"{edition}\""));
        // Nothing after the header is read: no module, no declaration, no recovery.
        assert_eq!(file.edition.as_deref(), Some(edition));
        assert!(file.module.is_none() && file.items.is_empty() && file.holes.is_empty());
        assert!(lower(&[file]).is_err());
    }
}

#[test]
fn ct_lg1_02_the_pinned_fixture_has_its_digest() {
    let bytes = std::fs::read(PINNED).expect("canonical.6cs");
    let file = parse(&bytes);
    assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(file.digest.as_deref(), Some(PINNED_SHA256));
}

#[test]
fn ct_lg1_02_crlf_and_a_byte_order_mark_are_refused() {
    let bytes = std::fs::read(PINNED).expect("canonical.6cs");
    let first_newline = bytes.iter().position(|&b| b == b'\n').unwrap();
    let crlf: Vec<u8> = bytes.iter().flat_map(|&b| if b == b'\n' { vec![b'\r', b'\n'] } else { vec![b] }).collect();
    let bom = [&[0xef, 0xbb, 0xbf][..], &bytes].concat();
    for (input, start, end) in [(crlf, first_newline, first_newline + 1), (bom, 0, 3)] {
        let file = parse(&input);
        assert_eq!(codes(&file), [INVALID_ENCODING]);
        let d = &file.diagnostics[0];
        assert_eq!(d.span, None);
        let raw = d.raw.as_ref().unwrap();
        assert_eq!((raw.digest.as_str(), raw.start, raw.end), (sha256_hex(&input).as_str(), start, end));
        assert!(file.digest.is_none() && file.items.is_empty());
    }
}

#[test]
fn ct_err_05_malformed_utf8_gives_the_raw_digest_and_interval() {
    for (input, start, end) in [(&[0x61, 0xff, 0x62][..], 1, 2), (&[0x61, 0xf0, 0x9f, 0x98][..], 1, 4)] {
        let file = parse(input);
        assert_eq!(codes(&file), [INVALID_ENCODING]);
        let d = &file.diagnostics[0];
        let raw = d.raw.as_ref().unwrap();
        assert_eq!((d.span, raw.digest.clone(), raw.start, raw.end), (None, sha256_hex(input), start, end));
    }
}

#[test]
fn ct_lg1_04_candidate_a_recovery_drafts() {
    let rows = candidate_a("recovery");
    assert_eq!(rows.len(), 14, "candidate A has fourteen text drafts");
    for (row, raw) in rows {
        let id = text(&row, "id");
        let data = product(&raw);
        let first = parse(&data);
        assert_eq!(first, parse(&data), "{id}: unstable recovery");
        assert_eq!(codes(&first), texts(&row, "codes"), "{id}");
        let recovered: Vec<&str> = first.declarations().map(|d| d.name.text.as_str()).collect();
        assert_eq!(recovered, texts(&row, "recovered"), "{id}");
        for d in first.declarations() {
            assert!(
                d.body.lets.is_empty() && d.body.result.kind == ExprKind::Int("99".into()),
                "{id}: meaning changed"
            );
        }
        for d in &first.diagnostics {
            located(&data, d.span.unwrap(), id);
        }
        for hole in &first.holes {
            located(&data, *hole, id);
        }
        // A hole never lowers, whatever its diagnostic says.
        assert!(lower(std::slice::from_ref(&first)).is_err(), "{id}: a draft with a hole lowered");
        let mut reworded = first.clone();
        for d in &mut reworded.diagnostics {
            d.message = "Alternate wording for this same diagnostic.".into();
        }
        assert!(lower(&[reworded]).is_err(), "{id}: reworded");
        // A prefix moves every location and changes no code.
        let prefix = "# shifted λ\n".as_bytes();
        let shifted = parse(&[prefix, &data].concat());
        assert_eq!(codes(&shifted), codes(&first), "{id}: shifted");
        for (a, b) in first.diagnostics.iter().zip(&shifted.diagnostics) {
            let (a, b) = (a.span.unwrap(), b.span.unwrap());
            let moved = (a.start + prefix.len(), a.end + prefix.len(), a.line + 1, a.column);
            assert_eq!((b.start, b.end, b.line, b.column), moved, "{id}: shifted location");
        }
        if id == "zero-denominator" {
            let span = first.diagnostics[0].span.unwrap();
            assert_eq!(&data[span.start..span.end], b"1/0");
            // Two independent errors both recover, and the declaration after them survives.
            let hole = first.holes[0];
            let damaged = &data[hole.start..hole.end];
            let renamed = String::from_utf8(damaged.to_vec()).unwrap().replace("broken", "also_broken");
            let twice = [&data[..hole.start], damaged, renamed.as_bytes(), &data[hole.end..]].concat();
            let both = parse(&twice);
            assert_eq!(codes(&both), [codes(&first), codes(&first)].concat());
            assert_eq!(both.declarations().map(|d| d.name.text.as_str()).collect::<Vec<_>>(), ["recovered"]);
            assert!(lower(&[both]).is_err());
        }
    }
}

#[test]
fn ct_lg1_04_candidate_a_byte_drafts() {
    let rows = candidate_a("byte_recovery");
    assert_eq!(rows.len(), 2, "candidate A has two byte drafts");
    for (row, json) in rows {
        let id = text(&row, "id");
        let hex = Json::parse(std::str::from_utf8(&json).unwrap()).unwrap();
        let hex = hex.get("source_hex").and_then(Json::str).expect("source_hex").to_string();
        let raw: Vec<u8> = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
        let data = product(&raw);
        let file = parse(&data);
        assert_eq!(codes(&file), texts(&row, "codes"), "{id}");
        let raw_context = file.diagnostics[0].raw.as_ref().unwrap();
        let start = match id {
            "invalid-utf8" => data.len() - 1,
            _ => data.iter().position(|&b| b == b'\r').unwrap(),
        };
        assert_eq!((raw_context.start, raw_context.end), (start, start + 1), "{id}");
        assert_eq!(raw_context.digest, sha256_hex(&data));
        assert!(file.declarations().next().is_none() && lower(&[file]).is_err());
    }
}

/// The let values of `values()`, in order.
fn let_values(source: &str) -> Vec<Lit> {
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
    let lowered = lower(&[file]).unwrap_or_else(|e| panic!("{e}"));
    let mut body = &lowered.program.modules[0].functions[0].body;
    let mut out = Vec::new();
    while let Ir::Let { value, body: rest, .. } = body {
        match &**value {
            Ir::Lit(lit) => out.push(lit.clone()),
            other => panic!("not a literal: {other:?}"),
        }
        body = &**rest;
    }
    out
}

#[test]
fn ct_lg1_05_exact_literals_stay_exact_and_approximations_keep_their_kind() {
    let big = format!("1{}", "0".repeat(120));
    let source = format!(
        "edition \"6cs-draft-1\";\nmodule m requires [];\n\nfn values() -> Unit effects [] {{\n    let a = +000{big};\n    \
         let b = -{big};\n    let c = -6/8;\n    let d = 3/-4;\n    let e = q5(1/2, -1/3);\n    let f = ~53\"0.125\";\n    \
         let g = 1/8;\n    return ();\n}}\n"
    );
    let pair = |n: &str, d: &str| (n.to_string(), d.to_string());
    assert_eq!(
        let_values(&source),
        [
            Lit::Nat(big.clone()),
            Lit::Int(format!("-{big}")),
            Lit::Rat { num: "-6".into(), den: "8".into() },
            Lit::Rat { num: "-3".into(), den: "4".into() },
            Lit::Q5 { one: pair("1", "2"), sqrt5: pair("-1", "3") },
            Lit::Approx { decimal: "0.125".into(), precision: 53 },
            Lit::Rat { num: "1".into(), den: "8".into() },
        ]
    );
}

#[test]
fn ct_lg1_05_a_zero_denominator_an_unmarked_decimal_and_a_bad_approximation_are_refused() {
    for (literal, offending) in [
        ("1/0", "1/0"),
        ("q5(1/2, 1/0)", "1/0"),
        ("1.5", "1.5"),
        ("~53\"NaN\"", "~53\"NaN\""),
        ("~53\"1e\"", "~53\"1e\""),
        ("~17\"0.5\"", "~17\"0.5\""),
        ("word[0]", "0"),
        ("word[+1201]", "+1201"),
    ] {
        let source = format!("edition \"6cs-draft-1\";\nmodule m requires [];\n\nfn f() -> Int effects [] {{\n    return {literal};\n}}\n");
        let file = parse(source.as_bytes());
        assert_eq!(codes(&file), [INVALID_LITERAL, RECOVERY_GAP], "{literal}");
        let span = file.diagnostics[0].span.unwrap();
        assert_eq!(&source[span.start..span.end], offending, "{literal}");
    }
}
