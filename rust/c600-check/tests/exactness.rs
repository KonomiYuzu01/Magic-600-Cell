//! LP2 plan section 5, acceptance checks 3 to 5: LG2.22's admissible display results, the exactness parts of
//! CT-LG2-04 and CT-LG2-05, display purity (LG2.20), reports in IR order (LG4.3) and the contract refusal (LG4.1).
//! Each test names the part it covers. Hand lowerings are no evidence for the parser (milestone M7).

mod support;

use c600_check::{check, EFFECT_UNDECLARED, EXACTNESS_VIOLATION, EXACT_REQUIRED, IR_VERSION_UNSUPPORTED};
use c600_ir::build::*;
use c600_ir::{Capability, ExactRole, Expr, Function, Lit, Prim, QTy, Ty, FORM2_EXEC_CONTRACT};
use support::{codes, fields, less, only, report, tint, tool, xy};

const V: &str = EXACTNESS_VIOLATION;

fn boolean(b: bool) -> Expr {
    Expr::Lit(Lit::Bool(b))
}

// Acceptance check 3 (LG2.22): pure display branches choosing two colours yield a display colour, and storing or
// exporting it with its qualifier is admitted.

#[test]
fn lg2_22_display_if_choosing_two_colours_yields_a_display_colour() {
    let shown = tool(vec![view("show", xy(), exact(Ty::Unit), publish_view(tint()))]);
    assert!(report(&shown).is_admitted());
    // It is a display colour: an exact colour result would let the approximate selection escape.
    let exact_result = tool(vec![helper("pick", xy(), exact(Ty::Colour), tint())]);
    assert_eq!(fields(&only(&exact_result)), (V, Some("Colour"), true, false));
}

#[test]
fn lg2_22_storing_the_display_colour_with_its_qualifier_is_admitted() {
    let keep = |field| tool(vec![command("keep", xy(), exact(Ty::Unit), write_workspace("tint", field, tint()))]);
    assert!(report(&keep(display(Ty::Colour))).is_admitted());
    // CT-LG2-05, storage: an exact field is refused.
    assert_eq!(fields(&only(&keep(exact(Ty::Colour)))), (V, Some("Colour"), true, false));
}

#[test]
fn lg2_22_exporting_the_display_colour_with_its_qualifier_is_admitted() {
    let lib = module("lib", vec![helper("tint", xy(), display(Ty::Colour), tint()).exported()]);
    let imported = || call_in("lib", "tint", vec![var("x"), var("y")]);
    let shown =
        program(vec![lib.clone(), module("app", vec![view("show", xy(), exact(Ty::Unit), publish_view(imported()))])]);
    assert!(report(&shown).is_admitted());
    // CT-LG2-05, module calls: the exported signature keeps the qualifier, so the importer cannot store it as exact.
    let stored = program(vec![
        lib,
        module(
            "app",
            vec![command("keep", xy(), exact(Ty::Unit), write_workspace("tint", exact(Ty::Colour), imported()))],
        ),
    ]);
    let d = only(&stored);
    assert_eq!((d.module.as_str(), fields(&d)), ("app", (V, Some("Colour"), false, true)));
    // A return annotation cannot erase the qualifier at the export either.
    let erased = program(vec![module("lib", vec![helper("tint", xy(), exact(Ty::Colour), tint()).exported()])]);
    assert_eq!(fields(&only(&erased)), (V, Some("Colour"), true, false));
}

// CT-LG2-04: explicit exact-to-approximate conversion names its precision; direct casts and helper-mediated
// approximate-to-exact conversions are refused.

#[test]
fn ct_lg2_04_exact_to_approximate_conversion_names_its_precision() {
    let widen = tool(vec![helper("widen", vec![param("x", exact(Ty::Nat))], exact(approx()), to_approx(var("x")))]);
    assert!(report(&widen).is_admitted());
    // The precision is part of the type: values of two precisions do not mix, and none narrows silently.
    let mixed = tool(vec![helper(
        "mix",
        xy(),
        exact(approx()),
        prim(Prim::ApproxAdd, vec![prim(Prim::ToApprox(53), vec![var("x")]), prim(Prim::ToApprox(24), vec![var("y")])]),
    )]);
    assert!(check(&mixed).is_err());
    let narrowed =
        tool(vec![helper("narrow", vec![param("x", exact(Ty::Nat))], exact(Ty::Approx(24)), to_approx(var("x")))]);
    assert!(check(&narrowed).is_err());
}

#[test]
fn ct_lg2_04_direct_and_helper_conversions_back_to_exact_are_refused() {
    // A direct cast is LG2.22's direct scalar case.
    let cast = tool(vec![helper(
        "cast",
        vec![param("x", exact(Ty::Nat))],
        exact(Ty::Int),
        prim(Prim::ToExact, vec![to_approx(var("x"))]),
    )]);
    assert_eq!(fields(&only(&cast)), (EXACT_REQUIRED, Some("Approx"), false, false));
    // A helper erases the qualifier at its own boundary, wherever it is called from.
    let via_helper = tool(vec![
        helper("erase", vec![param("a", exact(approx()))], exact(Ty::Int), var("a")),
        command("use", vec![param("x", exact(Ty::Nat))], exact(Ty::Int), call("erase", vec![to_approx(var("x"))])),
    ]);
    let d = only(&via_helper);
    assert_eq!((d.function.as_str(), fields(&d)), ("erase", (V, Some("Approx"), false, true)));
    // So does a closure.
    let via_closure = tool(vec![command(
        "use",
        vec![param("x", exact(Ty::Nat))],
        exact(Ty::Int),
        call_value(closure(vec![param("a", exact(approx()))], exact(Ty::Int), var("a")), vec![to_approx(var("x"))]),
    )]);
    assert_eq!(fields(&only(&via_closure)), (V, Some("Approx"), false, true));
}

// CT-LG2-05: approximate comparisons cannot steer a word, target, exact goal, evidence or budget through branches,
// generics, closures, storage or module calls (form 1; form 2 is not built).

#[test]
fn ct_lg2_05_branches_cannot_steer_a_word_or_an_exact_goal() {
    let word_ = tool(vec![command(
        "go",
        xy(),
        exact(Ty::Receipt),
        propose(display_if(less(), word("w1"), word("w2")), target("t")),
    )]);
    let d = only(&word_);
    assert_eq!((d.path.as_str(), fields(&d)), ("0", (V, Some("Word"), true, false)));
    // Even an exact literal computed in a display branch cannot escape as an exact result (LG2.20).
    let goal = tool(vec![command(
        "go",
        xy(),
        exact(Ty::Unit),
        display_if(less(), use_exact(ExactRole::Goal, word("w1")), unit()),
    )]);
    let d = only(&goal);
    assert_eq!((d.path.as_str(), fields(&d)), ("1", (V, Some("Word"), true, false)));
}

#[test]
fn ct_lg2_05_generic_substitution_keeps_the_qualifier_of_a_target() {
    let t = || exact(Ty::Var("T".into()));
    let p = tool(vec![
        helper("id", vec![param("v", t())], t(), var("v")),
        command(
            "go",
            xy(),
            exact(Ty::Receipt),
            propose(word("w"), call("id", vec![display_if(less(), target("a"), target("b"))])),
        ),
    ]);
    let d = only(&p);
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(&d)), ("go", "1", (V, Some("Target"), true, false)));
}

#[test]
fn ct_lg2_05_closures_cannot_steer_evidence() {
    // Display context propagates into a callee: a closure that builds exact evidence, called in a display branch.
    let p = tool(vec![command(
        "go",
        xy(),
        exact(Ty::Unit),
        let_(
            "f",
            closure(vec![], exact(Ty::Unit), use_exact(ExactRole::Evidence, word("e1"))),
            display_if(less(), call_value(var("f"), vec![]), unit()),
        ),
    )]);
    let d = only(&p);
    assert_eq!((d.path.as_str(), fields(&d)), ("1.1", (V, Some("Word"), true, false)));
    // A closure capturing a display condition is display-qualified; it cannot pass as an exact callable.
    let apply = helper(
        "apply",
        vec![param("f", exact(fn_ty(vec![], exact(Ty::Bool), bound(&[], &[]))))],
        exact(Ty::Bool),
        call_value(var("f"), vec![]),
    );
    let captured = closure(vec![], display(Ty::Bool), display_if(var("c"), boolean(true), boolean(false)));
    let p = tool(vec![apply, command("go", xy(), exact(Ty::Bool), let_("c", less(), call("apply", vec![captured])))]);
    let d = only(&p);
    assert_eq!((d.path.as_str(), fields(&d)), ("1.0", (V, Some("Fn"), false, true)));
}

// CT-LG2-05, higher-order calls: a slot's signature cannot erase a callable's result or parameter qualifier
// (LG2.19, closure row; pull request 149 review).

#[test]
fn ct_lg2_05_higher_order_calls_keep_the_result_qualifier_of_a_callable() {
    let pair = || vec![exact(Ty::Nat), exact(Ty::Nat)];
    let apply = || {
        helper(
            "apply",
            vec![
                param("f", exact(fn_ty(pair(), exact(Ty::Bool), bound(&[], &[])))),
                param("x", exact(Ty::Nat)),
                param("y", exact(Ty::Nat)),
            ],
            exact(Ty::Bool),
            call_value(var("f"), vec![var("x"), var("y")]),
        )
    };
    let steer = |pick: Function| {
        let choice = if_(call("apply", vec![fn_ref("pick"), var("x"), var("y")]), word("w1"), word("w2"));
        tool(vec![pick, apply(), command("go", xy(), exact(Ty::Word), choice)])
    };
    // LG2.22's display pick and an approximate condition cannot pass for an exact boolean.
    let display_pick = helper("pick", xy(), display(Ty::Bool), display_if(less(), boolean(true), boolean(false)));
    let condition = helper("pick", xy(), exact(Ty::ApproxBool), less());
    for pick in [display_pick, condition] {
        let d = only(&steer(pick));
        assert_eq!((d.function.as_str(), d.path.as_str(), fields(&d)), ("go", "0.0", (V, Some("Fn"), false, true)));
    }
    let exact_pick = helper("pick", xy(), exact(Ty::Bool), prim(Prim::ExactLess, vec![var("x"), var("y")]));
    assert!(report(&steer(exact_pick)).is_admitted());
    // A parameter of another base type is malformed IR.
    let words =
        helper("pick", vec![param("a", exact(Ty::Word)), param("b", exact(Ty::Word))], exact(Ty::Bool), boolean(true));
    assert!(check(&steer(words)).is_err());
}

#[test]
fn ct_lg2_05_a_callable_cannot_take_a_display_argument_as_exact() {
    // The slot passes a display value; a callable with an exact parameter would store it as exact.
    let apply = helper(
        "apply",
        vec![
            param("f", exact(fn_ty(vec![display(Ty::Nat)], exact(Ty::Unit), bound(&[], &["e"])))),
            param("x", exact(Ty::Nat)),
            param("y", exact(Ty::Nat)),
        ],
        exact(Ty::Unit),
        call_value(var("f"), vec![display_if(less(), var("x"), var("y"))]),
    );
    let go = || command("go", xy(), exact(Ty::Unit), call("apply", vec![fn_ref("store"), var("x"), var("y")]));
    let store = helper(
        "store",
        vec![param("n", exact(Ty::Nat))],
        exact(Ty::Unit),
        write_workspace("k", exact(Ty::Nat), var("n")),
    );
    let d = only(&tool(vec![apply.clone(), store, go()]));
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(&d)), ("go", "0", (V, Some("Fn"), false, true)));
    let shown = helper("store", vec![param("n", display(Ty::Nat))], exact(Ty::Unit), unit());
    assert!(report(&tool(vec![apply, shown, go()])).is_admitted());
}

// CT-LG2-05, generics: a generic result keeps the qualifier of every argument it may return, a selection inside a
// generic body needs a display result, and a display instance runs a pure display body (LG2.19; pull request 149
// review).

#[test]
fn ct_lg2_05_a_generic_result_keeps_the_qualifier_of_every_argument_it_may_return() {
    let t = || exact(Ty::Var("T".into()));
    let second = helper("second", vec![param("a", t()), param("b", t())], t(), var("b"));
    let steered = || display_if(less(), word("w2"), word("w3"));
    for args in [vec![word("w1"), steered()], vec![steered(), word("w1")]] {
        let go = command("go", xy(), exact(Ty::Receipt), propose(call("second", args), target("t")));
        let d = only(&tool(vec![second.clone(), go]));
        assert_eq!((d.function.as_str(), d.path.as_str(), fields(&d)), ("go", "0", (V, Some("Word"), true, false)));
    }
    let choose = |result: QTy| {
        let params = vec![param("a", t()), param("b", t()), param("x", exact(Ty::Nat)), param("y", exact(Ty::Nat))];
        helper("choose", params, result, display_if(less(), var("a"), var("b")))
    };
    let d = only(&tool(vec![choose(t())]));
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(&d)), ("choose", "root", (V, Some("T"), true, false)));
    let chosen = propose(call("choose", vec![word("w1"), word("w2"), var("x"), var("y")]), target("t"));
    let p = tool(vec![choose(display(Ty::Var("T".into()))), command("go", xy(), exact(Ty::Receipt), chosen)]);
    let d = only(&p);
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(&d)), ("go", "0", (V, Some("Word"), false, true)));
}

#[test]
fn ct_lg2_05_a_generic_body_with_a_display_instance_is_a_pure_display_body() {
    let t = || exact(Ty::Var("T".into()));
    let steered = || display_if(less(), word("w1"), word("w2"));
    let go = |callee: Function, arg: Expr| {
        let name = callee.name.clone();
        tool(vec![callee, command("go", xy(), exact(Ty::Unit), call(&name, vec![arg]))])
    };
    // An exact goal set from the argument.
    let aim = helper("aim", vec![param("v", t())], exact(Ty::Unit), use_exact(ExactRole::Goal, var("v")));
    assert!(report(&go(aim.clone(), word("w1"))).is_admitted());
    let d = only(&go(aim, steered()));
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(&d)), ("go", "root", (V, Some("T"), true, false)));
    // An effect.
    let log = helper("log", vec![param("v", t())], exact(Ty::Unit), let_("r", propose(word("w"), target("t")), unit()));
    assert!(report(&go(log.clone(), word("w1"))).is_admitted());
    let d = only(&go(log, steered()));
    assert_eq!((d.code, d.function.as_str(), d.path.as_str()), (EFFECT_UNDECLARED, "go", "root"));
    // A result of a concrete type is display-qualified too.
    let size = helper("size", vec![param("v", t())], exact(Ty::Nat), nat(1));
    let budget = |arg| use_exact(ExactRole::Budget, call("size", vec![arg]));
    let p = tool(vec![size, command("go", xy(), exact(Ty::Unit), budget(steered()))]);
    assert_eq!(fields(&only(&p)), (V, Some("Nat"), true, false));
}

// CT-LG2-05, aggregates: a callable keeps its exact sinks through a record (LG2.19, aggregate row; LG2.20; pull
// request 149 review).

#[test]
fn ct_lg2_05_a_callable_in_a_record_keeps_its_exact_evidence() {
    let ev = helper("ev", vec![], exact(Ty::Unit), use_exact(ExactRole::Evidence, word("e1")));
    let called = display_if(less(), call_value(field(var("r"), "f"), vec![]), unit());
    let p = tool(vec![ev, command("go", xy(), exact(Ty::Unit), let_("r", record(vec![("f", fn_ref("ev"))]), called))]);
    let r = report(&p);
    assert_eq!(r.diagnostics.len(), 1, "{:#?}", r.diagnostics);
    let d = &r.diagnostics[0];
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(d)), ("go", "1.1", (V, Some("Word"), true, false)));
    assert!(r.summary("tool", "go").unwrap().exact_sinks.contains(&(ExactRole::Evidence, "Word".to_string())));
}

#[test]
fn ct_lg2_05_module_calls_cannot_steer_a_budget() {
    let lib = module(
        "lib",
        vec![helper(
            "spend",
            vec![param("b", exact(Ty::Budget))],
            exact(Ty::Unit),
            use_exact(ExactRole::Budget, var("b")),
        )
        .exported()],
    );
    let app = module(
        "app",
        vec![command("go", xy(), exact(Ty::Unit), call_in("lib", "spend", vec![display_if(less(), nat(4), nat(8))]))],
    );
    let d = only(&program(vec![lib, app]));
    assert_eq!((d.module.as_str(), d.path.as_str(), fields(&d)), ("app", "0", (V, Some("Nat"), true, false)));
}

#[test]
fn ct_lg2_05_storage_keeps_the_qualifier() {
    // A saved display condition decoded as an exact goal (LG2.22): decoding retains the qualifier.
    let decoded = tool(vec![command(
        "go",
        vec![],
        exact(Ty::Unit),
        use_exact(ExactRole::Goal, read_snapshot("workspace.saved_condition", Ty::ApproxBool)),
    )]);
    assert_eq!(fields(&only(&decoded)), (V, Some("ApproxBool"), false, false));
    // A word chosen by display_if, stored in an exact field.
    let stored = tool(vec![command(
        "keep",
        xy(),
        exact(Ty::Unit),
        write_workspace("word", exact(Ty::Word), display_if(less(), word("w1"), word("w2"))),
    )]);
    assert_eq!(fields(&only(&stored)), (V, Some("Word"), true, false));
    // Untyped file bytes are no typed display field (LG2.19, storage row).
    let filed = program(vec![module(
        "tool",
        vec![command(
            "keep",
            xy(),
            exact(Ty::Unit),
            Expr::FileWrite { path: "tint.txt".into(), value: Box::new(tint()) },
        )],
    )
    .declaring(&[Capability::File])]);
    assert_eq!(fields(&only(&filed)), (V, Some("Colour"), true, false));
}

#[test]
fn ct_lg2_05_projection_and_matching_cannot_extract_an_exact_result() {
    // Box<ApproxBool> projected to an exact boolean (LG2.22).
    let boxed = tool(vec![helper("unbox", xy(), exact(Ty::Bool), field(record(vec![("value", less())]), "value"))]);
    assert_eq!(fields(&only(&boxed)), (V, Some("ApproxBool"), false, false));
    // An exact field of a display aggregate stays display-qualified (LG2.19, aggregate row).
    let mixed = tool(vec![command(
        "go",
        xy(),
        exact(Ty::Unit),
        use_exact(ExactRole::Budget, field(record(vec![("flag", less()), ("n", nat(4))]), "n")),
    )]);
    assert_eq!(fields(&only(&mixed)), (V, Some("Nat"), false, true));
    // A tag chosen by display_if makes its aggregate display as a whole; matching on it is refused.
    let tagged = tool(vec![command(
        "go",
        xy(),
        exact(Ty::Word),
        match_(
            field(record(vec![("tag", display_if(less(), Expr::Lit(tag("a")), Expr::Lit(tag("b"))))]), "tag"),
            vec![(tag("a"), word("w1"))],
            word("w2"),
        ),
    )]);
    assert_eq!(fields(&only(&tagged)), (V, Some("Status"), true, false));
}

// LG2.20: ordinary control takes exact inputs only, and display branches and display bodies are pure.

#[test]
fn lg2_20_ordinary_control_accepts_exact_inputs_only() {
    let short_circuit = tool(vec![command(
        "pick",
        xy(),
        exact(Ty::Word),
        if_(prim(Prim::And, vec![less(), boolean(true)]), word("w1"), word("w2")),
    )]);
    let d = only(&short_circuit);
    assert_eq!((d.path.as_str(), fields(&d)), ("0.0", (V, Some("ApproxBool"), false, false)));
    let exact_condition = tool(vec![command(
        "pick",
        xy(),
        exact(Ty::Word),
        if_(prim(Prim::ExactLess, vec![var("x"), var("y")]), word("w1"), word("w2")),
    )]);
    assert!(report(&exact_condition).is_admitted());
}

#[test]
fn lg2_20_display_branches_and_display_bodies_are_pure() {
    // A state read inside a display branch.
    let read = tool(vec![view(
        "peek",
        xy(),
        exact(Ty::Unit),
        publish_view(display_if(less(), read_snapshot("tint", Ty::Colour), colour("blue"))),
    )]);
    let d = only(&read);
    assert_eq!((d.code, d.path.as_str()), (EFFECT_UNDECLARED, "0.1"));
    // A preview prepared by a callee of a display branch: display context propagates into callees.
    let preview = tool(vec![
        helper("prepare", vec![], exact(Ty::Unit), let_("r", propose(word("w"), target("t")), unit())),
        command("go", xy(), exact(Ty::Unit), display_if(less(), call("prepare", vec![]), unit())),
    ]);
    let d = only(&preview);
    assert_eq!((d.code, d.function.as_str(), d.path.as_str()), (EFFECT_UNDECLARED, "go", "1"));
    // A capability request in the body of a function with an approximate input.
    let clock = program(vec![module(
        "tool",
        vec![helper("f", vec![param("a", exact(approx()))], display(Ty::Nat), Expr::ReadClock)],
    )
    .declaring(&[Capability::Clock])]);
    assert_eq!(codes(&clock), [EFFECT_UNDECLARED]);
    // Exact arithmetic inside a display branch is allowed; its result stays display.
    let arithmetic =
        tool(vec![helper("f", xy(), display(Ty::Nat), display_if(less(), exact_add(var("x"), nat(1)), var("y")))]);
    assert!(report(&arithmetic).is_admitted());
}

// LG4.1 and LG4.3.

#[test]
fn lg4_3_reports_list_diagnostics_in_ir_order_and_repeat() {
    // Modules in IR order, not by name; positions in body order.
    let p = program(vec![
        module("zeta", vec![helper("b", xy(), exact(Ty::Int), exact_add(to_approx(var("x")), to_approx(var("y"))))]),
        module("alpha", vec![helper("a", vec![param("v", exact(approx()))], exact(Ty::Int), var("v"))]),
    ]);
    let first = report(&p);
    let places: Vec<(&str, &str, &str)> =
        first.diagnostics.iter().map(|d| (d.module.as_str(), d.function.as_str(), d.path.as_str())).collect();
    assert_eq!(places, [("zeta", "b", "0"), ("zeta", "b", "1"), ("alpha", "a", "root")]);
    assert_eq!(first.codes(), [EXACT_REQUIRED, EXACT_REQUIRED, V]);
    for _ in 0..5 {
        assert_eq!(report(&p.clone()), first);
    }
}

#[test]
fn lg4_1_a_program_pinned_to_another_contract_is_refused_whole() {
    let body = || tool(vec![helper("f", xy(), exact(Ty::Int), to_approx(var("x")))]);
    let mut form2 = body();
    form2.exec_contract = FORM2_EXEC_CONTRACT.into();
    let r = report(&form2);
    assert_eq!(r.codes(), [IR_VERSION_UNSUPPORTED]);
    assert_eq!(r.diagnostics[0].path, "program");
    let mut older = body();
    older.ir_version = "c600-ir-draft-0".into();
    assert_eq!(codes(&older), [IR_VERSION_UNSUPPORTED]);
}

#[test]
fn malformed_ir_is_an_error_not_a_diagnostic() {
    // Chapter 02 has no source code for a base-type mismatch: the lowering is at fault.
    let mismatched = tool(vec![helper("f", vec![], exact(Ty::Word), if_(nat(1), word("a"), word("b")))]);
    assert!(check(&mismatched).is_err());
    assert!(check(&tool(vec![helper("f", vec![], exact(Ty::Nat), var("z"))])).is_err());
    let twice = tool(vec![helper("f", vec![], exact(Ty::Nat), nat(1)), helper("f", vec![], exact(Ty::Nat), nat(2))]);
    assert!(check(&twice).is_err());
    let private = program(vec![
        module("lib", vec![helper("f", vec![], exact(Ty::Nat), nat(1))]),
        module("app", vec![helper("g", vec![], exact(Ty::Nat), call_in("lib", "f", vec![]))]),
    ]);
    assert!(check(&private).is_err());
    let exact_condition = tool(vec![helper("f", vec![], display(Ty::Nat), display_if(boolean(true), nat(1), nat(2)))]);
    assert!(check(&exact_condition).is_err());
}
