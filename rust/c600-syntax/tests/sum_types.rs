//! A module's sum types in source (LG2.1; T1 drafts until the IR freeze, PHS.49): `type Shape { circle(Nat); empty(); }`
//! in column 1, exported with `export type`, its constructors called by name (`circle(3)`, `empty()`) or, from an
//! importing module, by the alias (`m.circle(3)`), the type named `Shape` or `m.Shape`, and a `match` on constructor
//! patterns with binders, `_` binders and an optional final `_` arm. The checker's half is `c600-check`'s `tower.rs`,
//! the conformance rows' source halves are `conformance_lg2.rs`.

use c600_check::{check, Report, TYPE_MISMATCH};
use c600_ir::{Expr as Ir, Pattern, Ty};
use c600_syntax::ast::{BindingKind, Item};
use c600_syntax::{format, lower, parse, sha256_hex, Lowered, ParsedFile};

const HEADER: &str = "edition \"6cs-draft-1\";\nmodule geo requires [];\n\n";

const SHAPE: &str =
    "export type Shape {\n    circle(Nat);\n    empty();\n    pair(Nat, record { a: Bool, b: Word });\n}\n";

fn complete(text: &str) -> ParsedFile {
    let file = parse(text.as_bytes());
    assert!(file.diagnostics.is_empty(), "{text}\n{:#?}", file.diagnostics);
    file
}

fn lowered(sources: &[&str]) -> Result<Lowered, String> {
    lower(&sources.iter().map(|s| complete(s)).collect::<Vec<_>>()).map_err(|e| e.to_string())
}

fn geo(declarations: &str) -> String {
    format!("{HEADER}{SHAPE}\n{declarations}")
}

fn report(sources: &[&str]) -> Report {
    let lowered = lowered(sources).unwrap_or_else(|e| panic!("{e}"));
    check(&lowered.program).unwrap_or_else(|e| panic!("{e}"))
}

fn refused(sources: &[&str]) -> String {
    match lowered(sources) {
        Ok(_) => panic!("lowered: {sources:#?}"),
        Err(e) => e,
    }
}

/// A module that imports `geo` as `m`.
fn client(geo: &str, body: &str) -> String {
    format!(
        "edition \"6cs-draft-1\";\nmodule client requires [];\nimport m \"{}\";\n\n{body}",
        sha256_hex(geo.as_bytes())
    )
}

/// A declaration parses to a type item, formats canonically, and lowers to the IR's declared type with its
/// constructors' payload types; constructors lower to constructions and a constructor match to a case whose patterns
/// keep their binders.
#[test]
fn a_sum_type_parses_formats_and_lowers() {
    let source =
        geo("export fn area(s: Shape) -> Nat effects [] {\n    return match s {\n        circle(r) => r;\n        \
         pair(n, _) => n;\n        _ => 0;\n    };\n}\n\nfn make() -> Shape effects [] {\n    return circle(3);\n}\n");
    let file = complete(&source);
    let types: Vec<&str> = file.types().map(|t| t.name.text.as_str()).collect();
    assert_eq!(types, ["Shape"]);
    assert!(matches!(&file.items[0], Item::Type(t) if t.exported && t.ctors.len() == 3));
    assert_eq!(format(&file).as_deref(), Some(source.as_str()), "the source is canonical");
    let lowered = lowered(&[&source]).unwrap();
    let module = &lowered.program.modules[0];
    let decl = module.type_decl("Shape").unwrap();
    assert!(decl.exported);
    let payloads: Vec<usize> = decl.ctors.iter().map(|c| c.payload.len()).collect();
    assert_eq!(payloads, [1, 0, 2]);
    assert_eq!(
        decl.ctors[2].payload[1].ty,
        Ty::Record(vec![
            ("a".into(), c600_ir::QTy { ty: Ty::Bool, display: false }),
            ("b".into(), c600_ir::QTy { ty: Ty::Word, display: false }),
        ])
    );
    let shape = Ty::Named { module: "geo".into(), name: "Shape".into() };
    assert_eq!(module.functions[0].params[0].ty.ty, shape);
    let Ir::Case { arms, default, .. } = &module.functions[0].body else { panic!("{:#?}", module.functions[0].body) };
    let patterns: Vec<&Pattern> = arms.iter().map(|(p, _)| p).collect();
    assert_eq!(patterns[0], &Pattern { ctor: "circle".into(), binders: vec!["r".into()] });
    assert_eq!(patterns[1], &Pattern { ctor: "pair".into(), binders: vec!["n".into(), "_".into()] });
    assert!(default.is_some());
    assert!(
        matches!(&module.functions[1].body, Ir::Construct { ty, ctor, args } if *ty == shape && ctor == "circle" && args.len() == 1)
    );
    // The source map keeps each arm's range: the case at the root, the scrutinee at 0, the arms at 1 and up.
    let map = lowered.source("geo").unwrap();
    let span = map.span("area", "2").unwrap();
    assert_eq!(&source[span.start..span.end], "n");
    assert!(check(&lowered.program).unwrap().is_admitted());
}

/// The layout prints a type one constructor per line, nullary constructors with `()`, a constructor pattern's
/// binders separated by `, `, and a literal pattern that would read as a constructor pattern in parentheses.
#[test]
fn the_layout_of_types_and_patterns_is_canonical() {
    let messy = format!(
        "{HEADER}type  Shape{{circle( Nat ) ;empty( );}}\nfn f(s: Shape) -> Nat effects [] {{ return match s \
         {{ circle( r ) => r; empty( ) => 0; }}; }}\n"
    );
    let file = complete(&messy);
    let text = format(&file).unwrap();
    let canonical = format!(
        "{HEADER}type Shape {{\n    circle(Nat);\n    empty();\n}}\n\nfn f(s: Shape) -> Nat effects [] {{\n    return \
         match s {{\n        circle(r) => r;\n        empty() => 0;\n    }};\n}}\n"
    );
    assert_eq!(text, canonical);
    assert_eq!(format(&complete(&text)).as_deref(), Some(text.as_str()));
    assert!(complete(&text).same_program(&file));
    // A literal pattern that would read as a constructor pattern keeps its parentheses.
    let literal = format!("{HEADER}fn f(x: Status) -> Nat effects [] {{\n    return match x {{\n        (f()) => 1;\n        _ => 0;\n    }};\n}}\n");
    let file = complete(&literal);
    assert_eq!(format(&file).as_deref(), Some(literal.as_str()));
    // A type without a constructor has a layout, and lowering refuses it.
    let never = format!("{HEADER}type Never {{}}\n");
    assert_eq!(format(&complete(&never)).as_deref(), Some(never.as_str()));
    assert!(refused(&[&never]).contains("declares no constructor"));
}

/// Another module names an exported type as `m.Shape` and calls its constructors as `m.circle(3)`; it names no
/// unexported type or constructor, and a pattern names the constructor alone.
#[test]
fn an_imported_type_is_named_through_its_alias() {
    let geo = geo("");
    let uses = client(
        &geo,
        "fn f(s: m.Shape) -> Nat effects [] {\n    return match s {\n        circle(r) => r;\n        _ => 0;\n    };\n}\n\n\
         fn g() -> m.Shape effects [] {\n    return m.empty();\n}\n",
    );
    assert_eq!(format(&complete(&uses)).as_deref(), Some(uses.as_str()));
    let r = report(&[&geo, &uses]);
    assert!(r.is_admitted(), "{:#?}", r.diagnostics);
    let hidden = geo.replace("export type", "type");
    for body in [
        "fn f(s: m.Shape) -> Nat effects [] {\n    return 1;\n}\n",
        "fn f() -> Nat effects [] {\n    let s = m.empty();\n    return 1;\n}\n",
    ] {
        let message = refused(&[&hidden, &client(&hidden, body)]);
        assert!(message.contains("does not export the type `Shape`"), "{body}: {message}");
    }
    for (body, why) in [
        ("fn f(s: m.Square) -> Nat effects [] {\n    return 1;\n}\n", "declares no type `Square`"),
        ("fn f(s: n.Shape) -> Nat effects [] {\n    return 1;\n}\n", "`n` names no imported module"),
        ("fn f(s: Shape) -> Nat effects [] {\n    return 1;\n}\n", "unknown type `Shape`"),
        ("fn f() -> Nat effects [] {\n    let c = m.circle;\n    return 1;\n}\n", "is a constructor of module"),
        ("fn f(s: m.Shape<Nat>) -> Nat effects [] {\n    return 1;\n}\n", "takes no type arguments"),
    ] {
        let message = refused(&[&geo, &client(&geo, body)]);
        assert!(message.contains(why), "{body}: {message}");
    }
}

/// A type's name is no built-in type or index, a module declares a type once, a constructor's name is unique among
/// the module's constructors and functions and no reserved word, and a payload holds no function or type variable;
/// an exported function's types are exported (the checker's IR fault).
#[test]
fn a_types_names_and_payloads_are_checked() {
    for (declarations, why) in [
        ("type State {\n    s();\n}\n", "names a built-in type"),
        ("type Full {\n    s();\n}\n", "names a built-in type"),
        ("type Nat {\n    s();\n}\n", "names a built-in type"),
        ("type A {\n    a();\n}\n\ntype A {\n    b();\n}\n", "declared twice"),
        ("type A {\n    a();\n}\n\ntype B {\n    a();\n}\n", "declared twice"),
        ("type A {\n    if();\n}\n", "reserved"),
        ("type A {\n    a();\n}\n\nfn a() -> Nat effects [] {\n    return 1;\n}\n", "names a constructor"),
        ("type A {\n    a(T);\n}\n", "unknown type `T`"),
        ("type A {\n    a(fn(Nat) -> Nat effects []);\n}\n", "holds a function"),
        ("type A {\n    a(List<fn(Nat) -> Nat effects []>);\n}\n", "holds no function"),
        ("type A {\n    a();\n}\n\nfn f<A>(x: A) -> Nat effects [] {\n    return 1;\n}\n", "generic"),
        ("type A {\n    a();\n}\n\nfn f(a: Nat) -> Nat effects [] {\n    return 1;\n}\n", "constructor"),
        ("type A {\n    a();\n}\n\nfn f() -> Nat effects [] {\n    let x = a;\n    return 1;\n}\n", "call it"),
        ("type A {\n    a();\n}\n\nfn f() -> A effects [] {\n    return a<A>();\n}\n", "type arguments"),
    ] {
        let message = refused(&[&format!("{HEADER}{declarations}")]);
        assert!(message.contains(why), "{declarations}: {message}");
    }
    // A sum type's value holds no generator: the checker's IR fault, as for a record or a collection.
    let generator = lowered(&[&format!("{HEADER}type A {{\n    a(Generator);\n}}\n")]).unwrap();
    assert!(check(&generator.program).is_err());
    // An exported function names exported types only.
    let private = format!("{HEADER}type A {{\n    a();\n}}\n\nexport fn f() -> A effects [] {{\n    return a();\n}}\n");
    assert!(check(&lowered(&[&private]).unwrap().program).is_err());
    let public = private.replace("type A", "export type A");
    assert!(check(&lowered(&[&public]).unwrap().program).unwrap().is_admitted());
    // Types and functions are apart, and a constructor may share a built-in constructor's name: a pattern resolves by
    // the scrutinee's type, a call with a type argument names the built-in one.
    let shared = format!(
        "{HEADER}type Answer {{\n    yes();\n    no(Nat);\n}}\n\nfn f(a: Answer, v: Partial<Nat>) -> Nat effects [] {{\n    \
         let x = match a {{\n        yes() => 1;\n        no(n) => n;\n    }};\n    let t = total<Partial<Nat>>(x);\n    \
         return match v {{\n        total(n) => n;\n        _ => x;\n    }};\n}}\n"
    );
    assert!(report(&[&shared]).is_admitted());
    let wrong = shared.replace("yes() => 1;", "yes(e) => 1;");
    assert_eq!(report(&[&wrong]).codes(), [TYPE_MISMATCH]);
    // So may a tower operation with a static index: a call without type arguments constructs.
    let probe = format!(
        "{HEADER}type Probe {{\n    check(Nat);\n}}\n\nfn f(s: State<Jumbling>) -> Probe effects [snapshot] {{\n    let h = \
         check<exact_pose>(s);\n    return check(1);\n}}\n"
    );
    assert!(report(&[&probe]).is_admitted());
}

/// A constructor pattern binds its names in its arm only, `_` binds nothing, and a binder does not shadow a name in
/// scope; a recovered declaration keeps a constructor pattern's binders (EV1.2).
#[test]
fn a_patterns_binders_are_scoped_to_their_arm() {
    let shadow = geo("fn f(r: Nat, s: Shape) -> Nat effects [] {\n    return match s {\n        circle(r) => r;\n        _ => 0;\n    };\n}\n");
    assert!(refused(&[&shadow]).contains("does not shadow"));
    let twice = geo("fn f(s: Shape) -> Nat effects [] {\n    return match s {\n        pair(n, n) => n;\n        _ => 0;\n    };\n}\n");
    assert!(refused(&[&twice]).contains("does not shadow"));
    let outside = geo("fn f(s: Shape) -> Nat effects [] {\n    return match s {\n        circle(r) => r;\n        _ => r;\n    };\n}\n");
    assert!(refused(&[&outside]).contains("`r` is not bound"));
    let wildcard = geo("fn f(s: Shape) -> Nat effects [] {\n    return match s {\n        pair(_, _) => 1;\n        _ => 0;\n    };\n}\n");
    assert!(report(&[&wildcard]).is_admitted());
    let late = geo(
        "fn f(s: Shape) -> Nat effects [] {\n    return match s {\n        _ => 0;\n        empty() => 1;\n    };\n}\n",
    );
    assert!(refused(&[&late]).contains("only the last arm"));
    // A broken declaration keeps the binders the parser read, `_` excepted.
    let broken =
        format!("{HEADER}fn f(s: Nat) -> Nat effects [] {{\n    return match s {{\n        pair(a, _) => $;\n}}\n");
    let file = parse(broken.as_bytes());
    assert_eq!(file.recovered.len(), 1);
    let bindings: Vec<(&str, BindingKind)> =
        file.recovered[0].bindings.iter().map(|b| (b.name.text.as_str(), b.kind)).collect();
    assert_eq!(bindings, [("s", BindingKind::Parameter), ("a", BindingKind::MatchBinder)]);
    // Recovery resumes at a type declaration in column 1.
    let resumed = format!("{HEADER}fn f() -> Nat effects [] {{\n    return $;\n}}\n\ntype A {{\n    a();\n}}\n");
    let file = parse(resumed.as_bytes());
    assert_eq!(file.holes.len(), 1);
    assert_eq!(file.types().count(), 1);
}
