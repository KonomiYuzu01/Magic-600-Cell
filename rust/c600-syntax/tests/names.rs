//! The reserved names a host reads for its completion list (LX-4): `keywords()` and `builtins()` are the lists
//! lowering itself checks, so a name they give is one a program cannot bind.

use std::collections::BTreeSet;

use c600_syntax::{builtins, constructors, indexed_operations, keywords, lower, parse};

const HEADER: &str = "edition \"6cs-draft-1\";\nmodule tool requires [];\n\n";

#[test]
fn the_reserved_name_lists_are_whole_and_distinct() {
    let (keywords, builtins) = (keywords(), builtins());
    assert_eq!(keywords.len(), 10);
    assert_eq!(builtins.len(), 99);
    for name in ["fn", "let", "return", "match"] {
        assert!(keywords.contains(&name), "{name}");
    }
    // The identity tower's rows without a static index are reserved (T1 drafts); its operations with one
    // (`indexed_operations()`) and the built-in constructors (`constructors()`) are not: a call with type arguments
    // names them.
    for name in ["if", "exact_add", "exact_less", "approx53", "exclude", "q5_parts", "q5_of", "digest", "derive"] {
        assert!(builtins.contains(&name), "{name}");
    }
    for name in indexed_operations().iter().chain(constructors()) {
        assert!(!builtins.contains(name) && !keywords.contains(name), "{name}");
    }
    assert_eq!(indexed_operations(), ["snapshot", "project", "compare", "check", "imply", "to_cost"]);
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

/// A program may bind the name of an identity tower operation with a static index: a call with type arguments names
/// the operation, a call without names the program's function (T1 drafts).
#[test]
fn an_indexed_operation_is_named_by_its_type_arguments() {
    let source = format!(
        "{HEADER}fn check(n: Nat) -> Bool effects [] {{\n    return exact_less(n, 5);\n}}\n\nfn f(s: State<Jumbling>) \
         -> Bool effects [snapshot] {{\n    let compare = check(4);\n    let h = check<exact_pose>(s);\n    return \
         compare;\n}}\n"
    );
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
    let lowered = lower(&[file]).unwrap_or_else(|e| panic!("{e}"));
    let report = c600_check::check(&lowered.program).unwrap();
    assert!(report.is_admitted(), "{:#?}", report.diagnostics);
    // Unbound, the name names the operation, which says what it takes, called or not.
    for (body, why) in [
        ("let h = check(s);", "`check` takes one type argument, a goal"),
        ("let h = check;", "`check` is not bound; the tower's operation `check` is called with its static index"),
    ] {
        let unbound =
            format!("{HEADER}fn f(s: State<Full>) -> Bool effects [snapshot] {{\n    {body}\n    return true;\n}}\n");
        let error = lower(&[parse(unbound.as_bytes())]).err().expect("refused").to_string();
        assert!(error.contains(why), "{body}: {error}");
    }
}
