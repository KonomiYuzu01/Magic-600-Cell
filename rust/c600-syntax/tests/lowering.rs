//! LP2 plan section 8, acceptance check 7: the nine frozen exactness cases of M1's acceptance check 2, written in
//! source, give the frozen code, value type and fields through the checker, and each diagnostic maps to a source
//! range of its function (LG4.12). Also: base-type mismatches at their source text (LG2.1), imports by digest, and
//! what lowering refuses.

#[path = "../../c600-check/tests/support/mod.rs"]
mod support;

use c600_check::{check, TYPE_MISMATCH};
use c600_ir::{Callee, Expr as Ir};
use c600_syntax::{lower, parse, sha256_hex, LowerError, Lowered};
use support::{boundary_vector, fields, mismatch, Case};

const HEADER: &str = "edition \"6cs-draft-1\";\nmodule tool requires [];\n\n";

/// One module `tool` holding `declarations`.
fn tool(declarations: &str) -> String {
    format!("{HEADER}{declarations}")
}

fn lowered(sources: &[&str]) -> Result<Lowered, LowerError> {
    let files: Vec<_> = sources
        .iter()
        .map(|s| {
            let file = parse(s.as_bytes());
            assert!(file.diagnostics.is_empty(), "{s}\n{:#?}", file.diagnostics);
            file
        })
        .collect();
    lower(&files)
}

/// Each source gives exactly one diagnostic with its case's frozen fields. Through the source map, its function and
/// path name the offending source text, so the parser's paths and the checker's agree.
fn assert_cases(cases: Vec<Case>, sources: Vec<(String, &str)>) {
    assert_eq!(sources.len(), cases.len(), "one source per frozen case");
    for ((source, offending), case) in sources.iter().zip(&cases) {
        let lowered = lowered(&[source]).unwrap_or_else(|e| panic!("{}: {e}", case.site));
        let report = check(&lowered.program).unwrap_or_else(|e| panic!("{}: {e}", case.site));
        assert_eq!(report.diagnostics.len(), 1, "{}: {:#?}", case.site, report.diagnostics);
        let d = &report.diagnostics[0];
        let expected =
            (case.code.as_str(), Some(case.value_type.as_str()), case.display_control, case.qualifier_erasure);
        assert_eq!(fields(d), expected, "{}", case.site);
        let map = lowered.source(&d.module).expect("a source map per module");
        assert_eq!(map.digest, sha256_hex(source.as_bytes()));
        let span = map.span(&d.function, &d.path).unwrap_or_else(|| panic!("{}: no range for {}", case.site, d.path));
        let declaration = map.declarations[&d.function];
        assert!(declaration.start <= span.start && span.end <= declaration.end, "{}", case.site);
        assert_eq!(&source[span.start..span.end], *offending, "{}: path {}", case.site, d.path);
    }
}

/// Each `(declarations, offending, expected, supplied)` gives exactly one `LG2:type.mismatch`, in `f`, with the
/// expected and supplied types, and through the source map its path names the offending source text.
fn assert_mismatches(cases: &[(&str, &str, &str, &str)]) {
    for &(declarations, offending, expected, supplied) in cases {
        let source = tool(declarations);
        let lowered = lowered(&[&source]).unwrap_or_else(|e| panic!("{declarations}: {e}"));
        let report = check(&lowered.program).unwrap_or_else(|e| panic!("{declarations}: {e}"));
        assert_eq!(report.diagnostics.len(), 1, "{declarations}: {:#?}", report.diagnostics);
        let d = &report.diagnostics[0];
        let (code, function, path, expected_type, supplied_type) = mismatch(d);
        assert_eq!((code, function, expected_type, supplied_type), (TYPE_MISMATCH, "f", expected, supplied));
        let map = lowered.source(&d.module).expect("a source map per module");
        let span = map.span(function, path).unwrap_or_else(|| panic!("{declarations}: no range for {path}"));
        let declaration = map.declarations[function];
        assert!(declaration.start <= span.start && span.end <= declaration.end, "{declarations}");
        assert_eq!(&source[span.start..span.end], offending, "{declarations}: path {path}");
    }
}

#[test]
fn a_base_type_mismatch_is_reported_at_its_source_text() {
    assert_mismatches(&[
        // a wrong operand
        (
            "fn f(t: Text) -> Nat effects [] {\n    let n = 1;\n    return exact_add(n, t);\n}\n",
            "t",
            "Nat | Int | Rat | Q5 | Budget",
            "Text",
        ),
        // a condition that is not Bool
        (
            "fn f(n: Nat) -> Word effects [] {\n    return if(exact_add(n, 1), word[+1], word[+2]);\n}\n",
            "exact_add(n, 1)",
            "Bool",
            "Nat",
        ),
        // a wrong return type
        ("fn f(x: Nat) -> Bool effects [] {\n    let y = exact_add(x, 1);\n    return y;\n}\n", "y", "Bool", "Nat"),
        // a wrong argument type
        (
            "fn g(n: Nat, w: Word) -> Word effects [] {\n    return w;\n}\n\n\
             fn f(x: Nat) -> Word effects [] {\n    return g(exact_add(x, 1), x);\n}\n",
            "x",
            "Word",
            "Nat",
        ),
        // a function value called with another number of arguments: the count is part of its type
        (
            "fn f() -> Nat effects [] {\n    let g = fn(x: Nat) -> Nat {\n        return x;\n    };\n    \
             return g(1, 2);\n}\n",
            "g",
            "fn(Nat, Nat) -> _",
            "fn(Nat) -> Nat",
        ),
    ]);
}

/// CT-LG2-05 (part: the frozen vector `direct-and-helper-condition` in source).
#[test]
fn direct_and_helper_condition_in_source() {
    let sources = vec![
        // ordinary if on approx_less(x,y) selecting literal words w1 or w2
        (
            tool(
                "command pick(x: Nat, y: Nat) -> Word effects [] {\n    \
                 return if(approx_less(approx53(x), approx53(y)), word[+1], word[+2]);\n}\n",
            ),
            "approx_less(approx53(x), approx53(y))",
        ),
        // helper returns approx_less(x,y) as ordinary Bool; exporting it does not change the refusal
        (
            tool("export fn less(x: Approx<P53>, y: Approx<P53>) -> Bool effects [] {\n    return approx_less(x, y);\n}\n"),
            "approx_less(x, y)",
        ),
        // closure captures approx_less(x,y) and returns exact budget 4 or 8
        (
            tool(
                "command budget(x: Nat, y: Nat) -> Budget effects [] {\n    \
                 let c = approx_less(approx53(x), approx53(y));\n    \
                 let f = fn() -> Budget {\n        return display_if(c, 4, 8);\n    };\n    \
                 return f();\n}\n",
            ),
            "fn() -> Budget {\n        return display_if(c, 4, 8);\n    }",
        ),
        // direct Approx<p> operand to exact addition
        (tool("fn add(x: Nat) -> Nat effects [] {\n    return exact_add(approx53(x), 1);\n}\n"), "approx53(x)"),
        // display-branch-selected Approx<p> result supplied to exact addition
        (
            tool(
                "fn add(x: Nat, y: Nat) -> Nat effects [] {\n    let a = approx53(x);\n    let b = approx53(y);\n    \
                 return exact_add(display_if(approx_less(a, b), a, b), 1);\n}\n",
            ),
            "display_if(approx_less(a, b), a, b)",
        ),
        // helper with Approx<p> input erases its display qualifier at an exact scalar return
        (tool("fn erase(x: Approx<P53>) -> Int effects [] {\n    return x;\n}\n"), "x"),
    ];
    assert_cases(boundary_vector("direct-and-helper-condition"), sources);
}

/// CT-LG2-05 (part: the frozen vector `display-accounting-no-exact-continuation` in source).
#[test]
fn display_accounting_no_exact_continuation_in_source() {
    let sources = vec![
        // exact code reads the host counter used.steps
        (
            tool("command next(n: Nat) -> Nat effects [budget] {\n    return exact_add(used_steps(), n);\n}\n"),
            "used_steps()",
        ),
        // exact code reclaims the unused one of the granted steps
        (
            tool("command reclaim() -> Unit effects [budget] {\n    return use_budget(unused_steps());\n}\n"),
            "unused_steps()",
        ),
        // exact code branches on KN7:budget.output
        (
            tool(
                "command branch() -> Word effects [budget] {\n    return match budget_status() {\n        \
                 tag(\"KN7:budget.output\") => word[+1];\n        _ => word[+2];\n    };\n}\n",
            ),
            "budget_status()",
        ),
    ];
    assert_cases(boundary_vector("display-accounting-no-exact-continuation"), sources);
}

#[test]
fn an_import_names_a_module_by_the_digest_of_its_source() {
    let helpers = "edition \"6cs-draft-1\";\nmodule helpers requires [];\n\n\
                   export fn twice(x: Nat) -> Nat effects [] {\n    return exact_add(x, x);\n}\n";
    let client = format!(
        "edition \"6cs-draft-1\";\nmodule client requires [];\nimport h \"{}\";\n\n\
         fn four() -> Nat effects [] {{\n    let f = h.twice;\n    return exact_add(h.twice(2), f(0));\n}}\n",
        sha256_hex(helpers.as_bytes())
    );
    let lowered = lowered(&[helpers, &client]).unwrap();
    let report = check(&lowered.program).unwrap();
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let Ir::Let { value, body, .. } = &lowered.program.modules[1].functions[0].body else { panic!("a let") };
    assert_eq!(**value, Ir::FnRef { module: Some("helpers".into()), name: "twice".into() });
    let Ir::Prim { args, .. } = &**body else { panic!("exact_add") };
    assert!(matches!(&args[0], Ir::Call { callee: Callee::Fn { module: Some(m), .. }, .. } if m == "helpers"));
    // Without the imported file, or with a name it does not declare, nothing lowers.
    assert!(lowered_err(&[&client]).contains("no given source file"));
    let missing = client.replace("h.twice(2)", "h.thrice(2)");
    assert!(lowered_err(&[helpers, &missing]).contains("declares no function `thrice`"));
}

fn lowered_err(sources: &[&str]) -> String {
    match lowered(sources) {
        Ok(_) => panic!("lowered: {sources:?}"),
        Err(e) => e.to_string(),
    }
}

/// A lowering refusal's message and the text of `sources[i]` it points at.
fn refused_at<'s>(sources: &[&'s str], i: usize) -> (String, &'s str) {
    match lowered(sources) {
        Ok(_) => panic!("lowered: {sources:?}"),
        Err(e) => {
            let span = e.span.unwrap_or_else(|| panic!("no source range: {e}"));
            (e.message, &sources[i][span.start..span.end])
        }
    }
}

#[test]
fn a_call_of_a_module_function_gives_one_argument_per_parameter_and_an_import_needs_an_export() {
    // A program's own function, like a reserved operation, takes one argument per parameter.
    let own = tool(
        "fn g(n: Nat, w: Word) -> Word effects [] {\n    return w;\n}\n\n\
         fn f() -> Word effects [] {\n    return g(1);\n}\n",
    );
    assert_eq!(refused_at(&[&own], 0), ("`g` takes 2 arguments".into(), "g(1)"));
    let helpers = "edition \"6cs-draft-1\";\nmodule helpers requires [];\n\n\
                   export fn twice(x: Nat) -> Nat effects [] {\n    return exact_add(x, x);\n}\n\n\
                   fn secret(x: Nat) -> Nat effects [] {\n    return twice(x);\n}\n";
    let client = |body: &str| {
        format!(
            "edition \"6cs-draft-1\";\nmodule client requires [];\nimport h \"{}\";\n\n\
             fn f() -> Nat effects [] {{\n    {body}\n}}\n",
            sha256_hex(helpers.as_bytes())
        )
    };
    // An imported function likewise, named as the call writes it.
    let wrong = client("return h.twice(1, 2);");
    assert_eq!(refused_at(&[helpers, &wrong], 1), ("`h.twice` takes 1 argument".into(), "h.twice(1, 2)"));
    // Another module calls or names only what a module exports, as the checker requires; its own module calls it.
    for body in ["return h.secret(1);", "let s = h.secret;\n    return s(1);"] {
        let private = client(body);
        let refused = refused_at(&[helpers, &private], 1);
        assert_eq!(refused, ("module `helpers` does not export `secret`".into(), "secret"), "{body}");
    }
    let lowered = lowered(&[helpers, &client("return h.twice(1);")]).unwrap();
    assert!(check(&lowered.program).unwrap().diagnostics.is_empty());
}

/// CT-LG2-01 (part: guard; matches over other types than verdicts wait for the domain families), CT-LG2-02 (part:
/// guard; puzzle and level indices wait for M4), CT-LG2-08 (part: guard; LX-8's fixture menu letters do not lower)
/// and CT-LG2-09 (part: guard; verdicts wait for M4): each construct is refused at lowering and names what it waits
/// for or what to write.
#[test]
fn lowering_refuses_what_the_ir_draft_cannot_hold_yet_and_names_what_it_waits_for() {
    let refusals = [
        ("fn f() -> Int effects [] charge [12, 0, 0, 0] {\n    return 1;\n}\n", "LG4.8"),
        ("command c() -> Unit effects [] row \"x\" kind k contexts [] permits [] {\n    return ();\n}\n", "LP3"),
        ("fn f(v: Nat) -> Text effects [] {\n    return match v {\n        yes(p) => \"y\";\n        _ => \"n\";\n    };\n}\n", "M4"),
        ("fn f() -> Word effects [] {\n    return menu[2:spin];\n}\n", "cap:item"),
        ("fn f(w: Word<Full,Retained>) -> Int effects [] {\n    return 1;\n}\n", "M4"),
        ("fn f(v: Verdict) -> Int effects [] {\n    return 1;\n}\n", "M4"),
    ];
    for (declaration, waits_for) in refusals {
        let message = lowered_err(&[&tool(declaration)]);
        assert!(message.contains(waits_for), "{declaration}: {message}");
    }
}

#[test]
fn lowering_refuses_unbound_reserved_and_shadowing_names() {
    let refusals = [
        ("fn f() -> Nat effects [] {\n    return y;\n}\n", "not bound"),
        ("fn f() -> Nat effects [] {\n    let if = 1;\n    return 1;\n}\n", "reserved"),
        ("fn f(x: Nat) -> Nat effects [] {\n    let x = 1;\n    return x;\n}\n", "shadow"),
        ("fn f() -> Nat effects [] {\n    return f;\n}\n\nfn f() -> Nat effects [] {\n    return 1;\n}\n", "twice"),
        ("fn exact_add() -> Nat effects [] {\n    return 1;\n}\n", "reserved"),
        ("fn f() -> Nat effects [] {\n    return approx53;\n}\n", "call it"),
        ("fn g(x: Nat) -> Nat effects [] {\n    return x;\n}\n\nfn f() -> Nat effects [] {\n    return g<Nat>(1);\n}\n", "type arguments"),
        ("fn f(x: Nat) -> Nat effects [] {\n    return match x {\n        1 => 2;\n    };\n}\n", "`_` arm"),
        ("fn f(x: Nat) -> Nat effects [] {\n    return match x {\n        _ => 1;\n        2 => 2;\n        _ => 3;\n    };\n}\n", "last arm"),
        ("fn f() -> Nat effects [] {\n    return if(true, 1);\n}\n", "3 arguments"),
        ("fn f() -> Nat effects [] {\n    return read_snapshot(\"q\");\n}\n", "one type argument"),
        ("fn f(x: Text) -> Colour effects [] {\n    return colour(x);\n}\n", "literal string"),
        ("fn f() -> Nat effects [teleport] {\n    return 1;\n}\n", "not an effect"),
        ("fn f() -> Nat effects [view, view] {\n    return 1;\n}\n", "twice"),
        ("fn f() -> record { a: Nat, a: Nat } effects [] {\n    return record { a = 1 };\n}\n", "twice"),
        ("fn f() -> Display<Display<Nat>> effects [] {\n    return 1;\n}\n", "nest"),
        ("fn f() -> Approx<P64> effects [] {\n    return 1;\n}\n", "P53"),
        ("fn f<Nat>() -> Nat effects [] {\n    return 1;\n}\n", "generic"),
    ];
    for (declaration, why) in refusals {
        let message = lowered_err(&[&tool(declaration)]);
        assert!(message.contains(why), "{declaration}: {message}");
    }
    // `requires` lists capabilities; an effect belongs to a declaration (LG3.8, LG3.9).
    let message = lowered_err(&["edition \"6cs-draft-1\";\nmodule m requires [workspace];\n"]);
    assert!(message.contains("is an effect"), "{message}");
}

/// CT-LG3-01 (part: effects, capabilities and effect variables reach the IR's bound from source).
#[test]
fn effects_and_capabilities_lower_to_the_ir_bound() {
    let source = "edition \"6cs-draft-1\";\nmodule m requires [file, clock];\n\n\
                  fn f<E>(g: fn() -> Nat effects [E]) -> Nat effects [snapshot, file, clock, E] {\n    return g();\n}\n";
    let lowered = lowered(&[source]).unwrap();
    let module = &lowered.program.modules[0];
    use c600_ir::{Capability, Effect};
    assert_eq!(module.capabilities.iter().copied().collect::<Vec<_>>(), [Capability::File, Capability::Clock]);
    let bound = module.functions[0].effects.as_ref().unwrap();
    assert_eq!(bound.effects.iter().copied().collect::<Vec<_>>(), [Effect::ReadSnapshot, Effect::GrantedFile]);
    assert_eq!(bound.capabilities.iter().copied().collect::<Vec<_>>(), [Capability::File, Capability::Clock]);
    assert_eq!(bound.vars.iter().map(String::as_str).collect::<Vec<_>>(), ["E"]);
}

/// CT-LG2-06 (part: a budget mixes with integers only; not run: puzzle and unit indices, which the IR draft does not
/// have).
#[test]
fn exact_arithmetic_takes_rationals_and_q5_values() {
    // Z ⊂ Q ⊂ Q(√5): exact addition, ordering and equality take rationals and Q(√5) values, and integers widen to
    // them in operands, branches, arguments and results (LG1.6).
    let accepted = [
        "fn f() -> Rat effects [] {\n    return exact_add(1/2, 1/3);\n}\n",
        "fn f(x: Rat) -> Rat effects [] {\n    return exact_add(x, 1);\n}\n",
        "fn f(x: Rat) -> Q5 effects [] {\n    return exact_add(q5(1/2, 1/2), x);\n}\n",
        "fn f(x: Q5, n: Int) -> Bool effects [] {\n    return exact_less(x, n);\n}\n",
        "fn f(x: Rat) -> Bool effects [] {\n    return exact_eq(x, 2);\n}\n",
        "fn f(c: Bool) -> Q5 effects [] {\n    return if(c, 1/2, q5(0/1, 1/1));\n}\n",
        "fn half(x: Rat) -> Rat effects [] {\n    return x;\n}\n\nfn f() -> Q5 effects [] {\n    return half(3);\n}\n",
    ];
    for declaration in accepted {
        let lowered = lowered(&[&tool(declaration)]).unwrap();
        let report = check(&lowered.program).unwrap_or_else(|e| panic!("{declaration}: {e}"));
        assert!(report.diagnostics.is_empty(), "{declaration}: {:#?}", report.diagnostics);
    }
    // A wider result does not fit a narrower slot, and a budget is a cost that mixes with integers only (LG2.6). Each
    // is a type mismatch (LG2.1).
    assert_mismatches(&[
        ("fn f() -> Int effects [] {\n    return exact_add(1/2, 1);\n}\n", "exact_add(1/2, 1)", "Int", "Rat"),
        (
            "fn f(b: Budget) -> Bool effects [] {\n    return exact_less(b, 1/2);\n}\n",
            "1/2",
            "Nat | Int | Budget",
            "Rat",
        ),
        (
            "fn f(b: Budget) -> Q5 effects [] {\n    return exact_add(q5(1/1, 0/1), b);\n}\n",
            "b",
            "Nat | Int | Rat | Q5",
            "Budget",
        ),
    ]);
    // An approximate operand at a rational exact addition stays an exactness refusal, not malformed IR.
    let source = tool("fn f(x: Rat) -> Rat effects [] {\n    return exact_add(approx53(x), 1/2);\n}\n");
    let lowered = lowered(&[&source]).unwrap();
    let report = check(&lowered.program).unwrap();
    assert_eq!(report.diagnostics.len(), 1, "{:#?}", report.diagnostics);
    assert_eq!(fields(&report.diagnostics[0]).0, "LG2:type.exact_required");
}
