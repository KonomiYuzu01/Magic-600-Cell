//! Fixtures shared by the editor services' tests.

#![allow(dead_code)]

use c600_editor::{Document, ModelIdentity, SourceRange, Units};

pub const HEADER: &str = "edition \"6cs-draft-1\";\nmodule tool requires [];\n\n";

/// A module `tool` with these declarations.
pub fn tool(declarations: &str) -> String {
    format!("{HEADER}{declarations}")
}

pub fn open(source: &str) -> Document {
    Document::open(source.as_bytes().to_vec(), Units::Utf8)
}

/// The byte offset of the `n`th occurrence (from 0) of `needle` in `source`, plus `shift`.
pub fn at(source: &str, needle: &str, n: usize, shift: usize) -> usize {
    source.match_indices(needle).nth(n).unwrap_or_else(|| panic!("{needle} #{n}")).0 + shift
}

/// The text a range selects in `source`.
pub fn text<'a>(source: &'a str, range: &SourceRange) -> &'a str {
    &source[range.start..range.end]
}

pub fn full() -> ModelIdentity {
    ModelIdentity {
        puzzle: "full".into(),
        model_id: "1111111111111111111111111111111111111111111111111111111111111111".into(),
        menu_id: None,
        menu_order: None,
        contract_rev: None,
    }
}

pub fn jumble(menu_order: &str) -> ModelIdentity {
    ModelIdentity {
        puzzle: "jumble".into(),
        model_id: "2222222222222222222222222222222222222222222222222222222222222222".into(),
        menu_id: Some("3333333333333333333333333333333333333333333333333333333333333333".into()),
        menu_order: Some(menu_order.into()),
        contract_rev: Some("1".into()),
    }
}

/// A dependency module `lib` with an exported `double`, a private `helper` and a `twice` that calls `double`.
pub const LIB: &str = "edition \"6cs-draft-1\";\nmodule lib requires [];\n\n\
export fn double(n: Nat) -> Nat effects [] {\n    return exact_add(n, n);\n}\n\n\
fn helper(n: Nat) -> Nat effects [] {\n    return n;\n}\n\n\
export fn twice(n: Nat) -> Nat effects [] {\n    return double(double(n));\n}\n";

/// A module importing `lib` by its digest.
pub fn importer(lib_digest: &str, body: &str) -> String {
    format!("edition \"6cs-draft-1\";\nmodule tool requires [];\nimport lib \"{lib_digest}\";\n\n{body}")
}
