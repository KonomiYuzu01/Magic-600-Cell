//! LP2 plan section 8, acceptance checks 4 and 8: the canonical layout (CT-LG1-03) and candidate A's seventeen LX-8
//! fixtures with the product header.

use c600_syntax::{format, lower, parse, sha256_hex, ParsedFile};

const LX8: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../work/experiments/language-lx/lx8-surfaces");
/// The expected canonical layout. CT-LG1-02 pins its SHA-256, so every host compares against the same bytes.
const PINNED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/canonical.6cs");

/// The pinned program written by hand: other spacing, comments, a tab, non-canonical literals and parentheses.
const UNFORMATTED: [&str; 17] = [
    "edition   \"6cs-draft-1\" ;",
    "module  pinned requires [ ];",
    "# the formatter drops comments",
    "export fn   choose ( x : Nat , y:Nat ) -> Word effects [ ] {",
    "  let c = approx_less( approx53(x), approx53(y) ) ; # and trailing ones",
    "  let f = fn( ) -> Display< Colour > { return display_if(c, colour(\"red\"), colour(\"blue\")); };",
    "  return match exact_less(x, y) { true => word[+1, -2]; _ => word[ 3 ]; } ;",
    "}",
    "fn apply < T , E > ( g : fn ( T ) -> T effects [ E ] , v : T ) -> T effects [E] { return g(v); }",
    "fn values() -> record { ratio : Rat , phi : Q5 , scale : Approx<P53> , label : Text } effects [] {",
    "\tlet r = record { ratio = 3/-4, phi = q5(+1/2, 1/+2), scale = ~53\"0.125\", label = \"\\u03bb \\\"quoted\\\"\\n\" };",
    "\treturn (r);",
    "}",
    "",
    "",
    "fn ratio() -> Rat effects [] { return (values()).ratio; }",
    "",
];

fn complete(bytes: &[u8]) -> ParsedFile {
    let file = parse(bytes);
    assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
    file
}

#[test]
fn ct_lg1_03_formatting_is_canonical_idempotent_and_keeps_the_program() {
    let source = UNFORMATTED.join("\n");
    let parsed = complete(source.as_bytes());
    let formatted = format(&parsed).expect("a complete parse formats");
    let pinned = std::fs::read_to_string(PINNED).expect("canonical.6cs");
    assert_eq!(formatted, pinned);
    let reparsed = complete(formatted.as_bytes());
    assert!(reparsed.same_program(&parsed));
    assert_eq!(format(&reparsed).as_deref(), Some(formatted.as_str()));
    let (a, b) = (lower(&[parsed]).unwrap(), lower(&[reparsed]).unwrap());
    assert_eq!(a.program, b.program);
    assert_ne!(a.sources, b.sources, "the source maps follow the text");
}

#[test]
fn ct_lg1_03_a_draft_with_a_hole_has_no_layout() {
    let file =
        parse(b"edition \"6cs-draft-1\";\nmodule m requires [];\n\nfn f() -> Int effects [] {\n    return $;\n}\n");
    assert!(!file.holes.is_empty());
    assert_eq!(format(&file), None);
}

#[test]
fn candidate_a_fixtures_parse_and_format_idempotently() {
    let dir = format!("{LX8}/candidate-a/fixtures");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("candidate A fixtures")
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".lxa8"))
        .collect();
    names.sort();
    assert_eq!(names.len(), 17);
    let manifest = std::fs::read_to_string(format!("{LX8}/fixtures/MANIFEST.json")).unwrap();
    for name in names {
        let raw = std::fs::read(format!("{dir}/{name}")).unwrap();
        // The bytes are the ones LX-8 recorded (and a check of this crate's SHA-256).
        assert!(manifest.contains(&sha256_hex(&raw)), "{name}: the fixture changed");
        let text = String::from_utf8(raw).unwrap();
        assert!(text.starts_with("edition \"lx8-0\";\n"), "{name}");
        let source = text.replacen("edition \"lx8-0\";", "edition \"6cs-draft-1\";", 1);
        let parsed = complete(source.as_bytes());
        let formatted = format(&parsed).unwrap();
        let reparsed = complete(formatted.as_bytes());
        assert!(reparsed.same_program(&parsed), "{name}");
        assert_eq!(format(&reparsed).as_deref(), Some(formatted.as_str()), "{name}");
    }
}
