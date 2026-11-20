//! The reserved names a host reads for its completion list (LX-4): `keywords()` and `builtins()` are the lists
//! lowering itself checks, so a name they give is one a program cannot bind.

use std::collections::BTreeSet;

use c600_syntax::{builtins, keywords, lower, parse};

const HEADER: &str = "edition \"6cs-draft-1\";\nmodule tool requires [];\n\n";

#[test]
fn the_reserved_name_lists_are_whole_and_distinct() {
    let (keywords, builtins) = (keywords(), builtins());
    assert_eq!(keywords.len(), 10);
    assert_eq!(builtins.len(), 93);
    for name in ["fn", "let", "return", "match"] {
        assert!(keywords.contains(&name), "{name}");
    }
    for name in ["if", "exact_add", "exact_less", "approx53", "exclude", "q5_parts", "q5_of"] {
        assert!(builtins.contains(&name), "{name}");
    }
    let (k, b): (BTreeSet<_>, BTreeSet<_>) = (keywords.iter().collect(), builtins.iter().collect());
    assert_eq!(k.len(), keywords.len(), "no keyword twice");
    assert_eq!(b.len(), builtins.len(), "no operation twice");
    assert!(k.is_disjoint(&b));
}

/// Every name `builtins()` gives is refused as a function's name by lowering, so the public list is the one lowering
/// checks.
#[test]
fn lowering_refuses_every_builtin_as_a_name() {
    for name in builtins() {
        let source = format!("{HEADER}fn {name}() -> Nat effects [] {{\n    return 1;\n}}\n");
        let file = parse(source.as_bytes());
        assert!(file.diagnostics.is_empty(), "{name}: {:#?}", file.diagnostics);
        match lower(&[file]) {
            Ok(_) => panic!("`{name}` lowered as a function's name"),
            Err(error) => assert!(error.to_string().contains("reserved"), "{name}: {error}"),
        }
    }
}
