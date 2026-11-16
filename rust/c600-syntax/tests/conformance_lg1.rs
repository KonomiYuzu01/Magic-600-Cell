//! Conformance test for chapter 02's LG1 row CT-LG1-01 across the files of one program (LP2). The single-file cases
//! are in `syntax.rs`.

use c600_check::check;
use c600_syntax::code::{EDITION_MISSING, EDITION_UNSUPPORTED};
use c600_syntax::{lower, parse, sha256_hex, EDITION};

/// A module `helpers` that `client` imports by digest.
fn helpers() -> String {
    format!(
        "edition \"{EDITION}\";\nmodule helpers requires [];\n\n\
         export fn one() -> Nat effects [] {{\n    return 1;\n}}\n"
    )
}

/// A module `client` with the given header line, importing `helpers` by the digest of its source.
fn client(header: &str, helpers: &str) -> String {
    format!(
        "{header}module client requires [];\nimport h \"{}\";\n\n\
         fn f() -> Nat effects [] {{\n    return h.one();\n}}\n",
        sha256_hex(helpers.as_bytes())
    )
}

/// CT-LG1-01 (part: the per-file half across the files of one program; not run: "mixed supported dependency editions
/// keep their meanings", not applicable while `6cs-draft-1` is the only supported edition): each file carries its own
/// edition. A file without one, or with another, is refused by itself, without trying another grammar; the other
/// file's header does not apply to it, and the set does not lower.
#[test]
fn ct_lg1_01_each_file_of_a_program_needs_its_own_supported_edition() {
    let helpers = helpers();
    let good = client(&format!("edition \"{EDITION}\";\n"), &helpers);
    let files = [parse(helpers.as_bytes()), parse(good.as_bytes())];
    assert!(files.iter().all(|f| f.diagnostics.is_empty()));
    let lowered = lower(&files).unwrap_or_else(|e| panic!("{e}"));
    assert!(check(&lowered.program).unwrap().diagnostics.is_empty());

    for (header, code) in [("", EDITION_MISSING), ("edition \"6cs-draft-2\";\n", EDITION_UNSUPPORTED)] {
        let source = client(header, &helpers);
        let file = parse(source.as_bytes());
        let codes: Vec<_> = file.diagnostics.iter().map(|d| d.code).collect();
        assert_eq!(codes, [code], "{header:?}");
        assert!(file.module.is_none() && file.items.is_empty() && file.holes.is_empty(), "nothing else is read");
        let helper_file = parse(helpers.as_bytes());
        assert!(helper_file.diagnostics.is_empty(), "the other file keeps its own edition");
        assert!(lower(&[helper_file, file]).is_err(), "{header:?}: the set does not lower");
    }
}
