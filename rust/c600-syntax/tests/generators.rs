//! Generators as values in source (DET.61; LP2 draft until the IR freeze, PHS.49): `rng_init("a", ...)` starts a
//! generator on the run's seed, its stream path being the run's path followed by the named segments, `rng_draw(g)`
//! draws its next block as a natural, and `Generator` is a type. Both operations need the `random` effect and
//! capability, as `random_draw` does. A generator stays a bare value: no record, collection, closure capture or host
//! answer holds one, no type parameter takes one, no exact equality compares one and no host read takes one as its
//! argument; a declared type holds one only as a signature's own parameter or result, also a callable's. The run-time
//! half, its cursors and their rows, is `c600-run`'s `generators.rs`.

use c600_check::{admit, check, Grant, Policy, Profile, Report, EFFECT_UNDECLARED, NOT_GRANTED, TYPE_MISMATCH};
use c600_ir::{Capability, Expr as Ir, Prim, Program, Ty};
use c600_syntax::{lower, parse, Lowered};

const PROFILES: [Profile; 3] = [Profile::Solving, Profile::ToolAuthoring, Profile::Research];

/// The text of the checker's `PORTABLE` expectation, which a host read's argument must meet.
const PORTABLE: &str =
    "a type without Fn, Receipt, WordEqualityKey, TraceEqualityKey, CompressedWord, Generator or a type variable";

/// One module `tool` with the given capability list and declarations.
fn source(requires: &str, declarations: &str) -> String {
    format!("edition \"6cs-draft-1\";\nmodule tool requires [{requires}];\n\n{declarations}")
}

fn lowered_with(requires: &str, declarations: &str) -> Lowered {
    let text = source(requires, declarations);
    let file = parse(text.as_bytes());
    assert!(file.diagnostics.is_empty(), "{text}\n{:#?}", file.diagnostics);
    lower(&[file]).unwrap_or_else(|e| panic!("{declarations}: {e}"))
}

fn program(requires: &str, declarations: &str) -> Program {
    lowered_with(requires, declarations).program
}

fn report(declarations: &str) -> Report {
    check(&program("random, file", declarations)).unwrap_or_else(|e| panic!("{declarations}: {e}"))
}

fn admitted(declarations: &str) {
    let r = report(declarations);
    assert!(r.is_admitted(), "{declarations}: {:#?}", r.diagnostics);
}

/// The only finding's code, and the expected and supplied types of a mismatch.
fn only(declarations: &str) -> (&'static str, Option<(String, String)>) {
    let r = report(declarations);
    assert_eq!(r.diagnostics.len(), 1, "{declarations}: {:#?}", r.diagnostics);
    let d = &r.diagnostics[0];
    (d.code, d.mismatch.as_ref().map(|m| (m.expected.clone(), m.supplied.clone())))
}

/// The checker's refusal of the whole program, which lowers.
fn refused(declarations: &str) -> String {
    match check(&program("random, file", declarations)) {
        Ok(r) => panic!("{declarations}: checked, {:#?}", r.diagnostics),
        Err(e) => e.to_string(),
    }
}

/// Every expression of every function of a lowered program, depth first.
fn expressions(lowered: &Lowered) -> Vec<&Ir> {
    let mut out = Vec::new();
    let mut pending: Vec<&Ir> = lowered.program.modules.iter().flat_map(|m| &m.functions).map(|f| &f.body).collect();
    while let Some(e) = pending.pop() {
        out.push(e);
        pending.extend(e.children());
    }
    out
}

const MADE: &str =
    "fn f() -> Nat effects [random] {\n    let g = rng_init(\"a\", \"b\");\n    let h = rng_init();\n    \
    return exact_add(rng_draw(g), rng_draw(h));\n}\n\n\
    fn pass(g: Generator) -> Generator effects [random] {\n    let n = rng_draw(g);\n    return g;\n}\n\n\
    fn apply(c: fn(Generator) -> Nat effects [random], g: Generator) -> Nat effects [random] {\n    return c(g);\n}\n";

/// DET.61 (LP2 draft): `rng_init` lowers to the IR's generator start with its segments in written order, none or
/// more, and `rng_draw` to its draw; `Generator` is a type of a signature's parameters and results, also a callable's.
/// The program is admitted.
#[test]
fn rng_init_and_rng_draw_lower_to_the_generator_operations() {
    let lowered = lowered_with("random", MADE);
    let mut paths: Vec<Vec<String>> = expressions(&lowered)
        .into_iter()
        .filter_map(|e| match e {
            Ir::RngInit { path } => Some(path.clone()),
            _ => None,
        })
        .collect();
    paths.sort();
    assert_eq!(paths, [vec![], vec!["a".to_string(), "b".to_string()]]);
    let draws = expressions(&lowered).iter().filter(|e| matches!(e, Ir::Prim { op: Prim::RngDraw, .. })).count();
    assert_eq!(draws, 3);
    let functions = &lowered.program.modules[0].functions;
    assert_eq!((&functions[1].params[0].ty.ty, &functions[1].result.ty), (&Ty::Generator, &Ty::Generator));
    let Ty::Fn(callable) = &functions[2].params[0].ty.ty else { panic!("a callable parameter") };
    assert_eq!((&callable.params[0].ty, &callable.result.ty), (&Ty::Generator, &Ty::Nat));
    admitted(MADE);
}

/// DET.61 (LP2 draft): `rng_init` and `rng_draw` need the `random` effect, declared by the function and required by
/// its module, and the randomness grant, as `random_draw` does (CT-LG3-04's table): each missing part gives its own
/// finding, in every profile.
#[test]
fn the_generator_operations_need_the_random_effect_and_grant() {
    let declarations = [
        "fn f() -> Generator effects [random] {\n    return rng_init(\"a\");\n}\n",
        "fn f(g: Generator) -> Nat effects [random] {\n    return rng_draw(g);\n}\n",
    ];
    for declaration in declarations {
        for profile in PROFILES {
            let run = |requires: &str, grant: &Grant| {
                let p = program(requires, declaration);
                admit(&p, "tool", "f", profile, grant, &Policy::interim()).unwrap().codes()
            };
            let case = format!("{declaration} in {profile:?}");
            let granted = Grant::of(&[Capability::Randomness]);
            assert_eq!(run("", &Grant::none()), [EFFECT_UNDECLARED, NOT_GRANTED], "{case}");
            assert_eq!(run("", &granted), [EFFECT_UNDECLARED], "{case}");
            assert_eq!(run("random", &Grant::none()), [NOT_GRANTED], "{case}");
            assert!(run("random", &granted).is_empty(), "{case}");
        }
        let undeclared = declaration.replace("effects [random]", "effects []");
        assert_eq!(only(&undeclared).0, EFFECT_UNDECLARED, "{undeclared}");
    }
}

/// DET.61 (LP2 draft): a generator stays a bare value, so that every cursor has one owner and only DET.61's alias
/// copies it. The checker refuses the whole program when a record literal, a collection or a closure's capture would
/// hold one, and when a declared type would: a parameter or a result whose type holds one inside a record or a
/// collection, also a callable's or a closure's, and a host read or write of a type that holds one at all. A view or a
/// file that would receive one refuses it too.
#[test]
fn a_generator_stays_a_bare_value() {
    let wrap = |effects: &str, body: &str| format!("fn f() -> Nat effects [{effects}] {{\n    {body}\n}}\n");
    let cases = [
        (
            wrap("random", "let r = record { g = rng_init(\"a\") };\n    return 1;"),
            "field `g`: a record holds no generator",
        ),
        (
            wrap("random", "let l = list<Generator>(rng_init(\"a\"));\n    return 1;"),
            "a collection of type List<Generator>: a generator stays a bare value",
        ),
        (
            wrap("random", "let g = rng_init(\"a\");\n    let c = fn() -> Nat {\n        return rng_draw(g);\n    };\n    return c();"),
            "`g`: a closure captures no generator",
        ),
        (
            "fn f(l: List<Generator>) -> Nat effects [] {\n    return 1;\n}\n".to_string(),
            "a parameter of type List<Generator>: a generator stays a bare value",
        ),
        (
            "fn f(g: Generator) -> record { g: Generator } effects [] {\n    return record { g = g };\n}\n".to_string(),
            "the result of type record { g: Generator }: a generator stays a bare value",
        ),
        (
            "fn f(c: fn(Set<Generator>) -> Nat effects []) -> Nat effects [] {\n    return 1;\n}\n".to_string(),
            "a parameter of type fn(Set<Generator>) -> Nat: a generator stays a bare value",
        ),
        (
            wrap("", "let c = fn(m: Map<Nat, Generator>) -> Nat {\n        return 1;\n    };\n    return 1;"),
            "a closure's parameter of type Map<Nat, Generator>: a generator stays a bare value",
        ),
        (
            wrap("snapshot", "let g = read_snapshot<Generator>(\"g\");\n    return 1;"),
            "a snapshot read of type Generator: a generator stays a bare value",
        ),
        (
            wrap("snapshot", "let g = read_snapshot<record { g: Generator }>(\"g\", 1);\n    return 1;"),
            "a snapshot read of type record { g: Generator }: a generator stays a bare value",
        ),
        (
            wrap("random, workspace", "let k = write_workspace<Generator>(\"k\", rng_init(\"a\"));\n    return 1;"),
            "a workspace field of type Generator: a generator stays a bare value",
        ),
        (
            wrap("file", "let g = file_read<Generator>(\"g\");\n    return 1;"),
            "a file read of type Generator: a generator stays a bare value",
        ),
        (
            wrap("random, view", "let g = rng_init(\"a\");\n    let u = publish_view(g);\n    return 1;"),
            "a published view of type Generator: a generator stays a bare value",
        ),
        (
            wrap("random, file", "let g = rng_init(\"a\");\n    let u = file_write(\"g\", g);\n    return 1;"),
            "a file write of type Generator: a generator stays a bare value",
        ),
    ];
    for (declarations, expected) in cases {
        let message = refused(&declarations);
        assert!(message.contains(expected), "{declarations}: {message}");
    }
}

/// DET.61 (LP2 draft): no exact equality compares a generator, no type parameter takes one, no host read takes one
/// as its argument, `rng_draw` draws from a generator only, and a generator is no natural. Each is one type mismatch.
#[test]
fn a_generator_has_no_equality_type_parameter_or_canonical_form() {
    let cases = [
        (
            "fn f() -> Bool effects [random] {\n    let g = rng_init(\"a\");\n    return exact_eq(g, g);\n}\n",
            ("a type without Generator", "Generator"),
        ),
        (
            "fn id<T>(x: T) -> T effects [] {\n    return x;\n}\n\n\
             fn f() -> Nat effects [random] {\n    let g = id(rng_init(\"a\"));\n    return 1;\n}\n",
            ("T", "Generator"),
        ),
        (
            "fn f() -> Nat effects [random, snapshot] {\n    return read_snapshot<Nat>(\"n\", rng_init(\"a\"));\n}\n",
            (PORTABLE, "Generator"),
        ),
        ("fn f() -> Nat effects [random] {\n    return rng_draw(1);\n}\n", ("Generator", "Nat")),
        (
            "fn n(x: Nat) -> Nat effects [] {\n    return x;\n}\n\n\
             fn f() -> Nat effects [random] {\n    return n(rng_init(\"a\"));\n}\n",
            ("Nat", "Generator"),
        ),
    ];
    for (declarations, (expected, supplied)) in cases {
        let (code, mismatch) = only(declarations);
        assert_eq!(code, TYPE_MISMATCH, "{declarations}");
        assert_eq!(mismatch, Some((expected.to_string(), supplied.to_string())), "{declarations}");
    }
}
