//! LP2 plan section 5, acceptance checks 3 to 5: LG2.22's admissible display results, the exactness parts of
//! CT-LG2-04 and CT-LG2-05, display purity (LG2.20), reports in IR order (LG4.3) and the contract refusal (LG4.1).
//! Each test names the part it covers. Hand lowerings are no evidence for the parser (milestone M7).

mod support;

use c600_check::{
    check, EFFECT_UNDECLARED, EXACTNESS_VIOLATION, EXACT_REQUIRED, IR_VERSION_UNSUPPORTED, TYPE_MISMATCH,
};
use c600_ir::build::*;
use c600_ir::{Capability, ExactRole, Expr, Function, Lit, Prim, QTy, Ty, FORM2_EXEC_CONTRACT};
use support::{codes, fields, less, mismatch, only, report, tint, tool, xy};

const V: &str = EXACTNESS_VIOLATION;
/// What a goal or an evidence sink expects, in a mismatch (PHS.52, 16 November 2026).
const PORTABLE: &str =
    "a type without Fn, Receipt, WordEqualityKey, TraceEqualityKey, CompressedWord, Generator or a type variable";

fn boolean(b: bool) -> Expr {
    Expr::Lit(Lit::Bool(b))
}

// Acceptance check 3 (LG2.22): pure display branches choosing two colours yield a display colour, and storing or
// exporting it with its qualifier is admitted.

/// CT-LG2-05 (part: LG2.22, a display branch yields a display value; not run: form 2, LG2.21).
#[test]
fn lg2_22_display_if_choosing_two_colours_yields_a_display_colour() {
    let shown = tool(vec![view("show", xy(), exact(Ty::Unit), publish_view(tint()))]);
    assert!(report(&shown).is_admitted());
    // It is a display colour: an exact colour result would let the approximate selection escape.
    let exact_result = tool(vec![helper("pick", xy(), exact(Ty::Colour), tint())]);
    assert_eq!(fields(&only(&exact_result)), (V, Some("Colour"), true, false));
}

/// CT-LG2-05 (part: LG2.22, storing a display value with its qualifier).
#[test]
fn lg2_22_storing_the_display_colour_with_its_qualifier_is_admitted() {
    let keep = |field| tool(vec![command("keep", xy(), exact(Ty::Unit), write_workspace("tint", field, tint()))]);
    assert!(report(&keep(display(Ty::Colour))).is_admitted());
    // CT-LG2-05, storage: an exact field is refused.
    assert_eq!(fields(&only(&keep(exact(Ty::Colour)))), (V, Some("Colour"), true, false));
}

/// CT-LG2-05 (part: LG2.22, exporting a display value with its qualifier).
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

/// CT-LG2-04 (part: the conversion names its precision, in IR; the row is whole with the next test and
/// `c600-syntax`'s `ct_lg2_04_conversions_in_source`).
#[test]
fn ct_lg2_04_exact_to_approximate_conversion_names_its_precision() {
    let widen = tool(vec![helper("widen", vec![param("x", exact(Ty::Nat))], exact(approx()), to_approx(var("x")))]);
    assert!(report(&widen).is_admitted());
    // The precision is part of the type: values of two precisions do not mix, and none narrows silently (LG2.1).
    let mixed = tool(vec![helper(
        "mix",
        xy(),
        exact(approx()),
        prim(Prim::ApproxAdd, vec![prim(Prim::ToApprox(53), vec![var("x")]), prim(Prim::ToApprox(24), vec![var("y")])]),
    )]);
    assert_eq!(mismatch(&only(&mixed)), (TYPE_MISMATCH, "mix", "1", "Approx<P53>", "Approx<P24>"));
    let narrowed =
        tool(vec![helper("narrow", vec![param("x", exact(Ty::Nat))], exact(Ty::Approx(24)), to_approx(var("x")))]);
    assert_eq!(mismatch(&only(&narrowed)), (TYPE_MISMATCH, "narrow", "root", "Approx<P24>", "Approx<P53>"));
}

/// CT-LG2-04 (part: direct and helper-mediated conversions back to exact, in IR).
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

/// CT-LG2-05 (part: branches under form 1; not run: form 2, which is on record and not built, LG2.21).
#[test]
fn ct_lg2_05_branches_cannot_steer_a_word_or_an_exact_goal() {
    let word_ = tool(vec![command(
        "go",
        xy(),
        exact(Ty::Receipt),
        propose(display_if(less(), word(&[1]), word(&[2])), target("t")),
    )]);
    let d = only(&word_);
    assert_eq!((d.path.as_str(), fields(&d)), ("0", (V, Some("Word"), true, false)));
    // Even an exact literal computed in a display branch cannot escape as an exact result (LG2.20).
    let goal = tool(vec![command(
        "go",
        xy(),
        exact(Ty::Unit),
        display_if(less(), use_exact(ExactRole::Goal, word(&[1])), unit()),
    )]);
    let d = only(&goal);
    assert_eq!((d.path.as_str(), fields(&d)), ("1", (V, Some("Word"), true, false)));
}

/// CT-LG2-05 (part: generics, form 1).
#[test]
fn ct_lg2_05_generic_substitution_keeps_the_qualifier_of_a_target() {
    let t = || exact(Ty::Var("T".into()));
    let p = tool(vec![
        helper("id", vec![param("v", t())], t(), var("v")),
        command(
            "go",
            xy(),
            exact(Ty::Receipt),
            propose(word(&[4]), call("id", vec![display_if(less(), target("a"), target("b"))])),
        ),
    ]);
    let d = only(&p);
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(&d)), ("go", "1", (V, Some("Target"), true, false)));
}

/// CT-LG2-05 (part: closures, form 1).
#[test]
fn ct_lg2_05_closures_cannot_steer_evidence() {
    // Display context propagates into a callee: a closure that builds exact evidence, called in a display branch.
    let p = tool(vec![command(
        "go",
        xy(),
        exact(Ty::Unit),
        let_(
            "f",
            closure(vec![], exact(Ty::Unit), use_exact(ExactRole::Evidence, word(&[5]))),
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

/// CT-LG2-05 (part: higher-order calls, form 1).
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
        let choice = if_(call("apply", vec![fn_ref("pick"), var("x"), var("y")]), word(&[1]), word(&[2]));
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
    // A parameter of another base type does not fit the slot (LG2.1).
    let words =
        helper("pick", vec![param("a", exact(Ty::Word)), param("b", exact(Ty::Word))], exact(Ty::Bool), boolean(true));
    let d = only(&steer(words));
    assert_eq!(mismatch(&d), (TYPE_MISMATCH, "go", "0.0", "fn(Nat, Nat) -> Bool", "fn(Word, Word) -> Bool"));
}

/// CT-LG2-05 (part: callable parameters, form 1).
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

/// CT-LG2-05 (part: generic results, form 1).
#[test]
fn ct_lg2_05_a_generic_result_keeps_the_qualifier_of_every_argument_it_may_return() {
    let t = || exact(Ty::Var("T".into()));
    let second = helper("second", vec![param("a", t()), param("b", t())], t(), var("b"));
    let steered = || display_if(less(), word(&[2]), word(&[3]));
    for args in [vec![word(&[1]), steered()], vec![steered(), word(&[1])]] {
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
    let chosen = propose(call("choose", vec![word(&[1]), word(&[2]), var("x"), var("y")]), target("t"));
    let p = tool(vec![choose(display(Ty::Var("T".into()))), command("go", xy(), exact(Ty::Receipt), chosen)]);
    let d = only(&p);
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(&d)), ("go", "0", (V, Some("Word"), false, true)));
}

/// CT-LG2-05 (part: display instances of generic bodies, form 1).
#[test]
fn ct_lg2_05_a_generic_body_with_a_display_instance_is_a_pure_display_body() {
    let t = || exact(Ty::Var("T".into()));
    let steered = || display_if(less(), word(&[1]), word(&[2]));
    let go = |callee: Function, arg: Expr| {
        let name = callee.name.clone();
        tool(vec![callee, command("go", xy(), exact(Ty::Unit), call(&name, vec![arg]))])
    };
    // An exact goal set from the argument. A value of a type variable may stand for a function or a receipt, so the
    // goal refuses it whatever the instance (PHS.52, 16 November 2026); a display instance also steers the goal.
    let aim = helper("aim", vec![param("v", t())], exact(Ty::Unit), use_exact(ExactRole::Goal, var("v")));
    assert_eq!(mismatch(&only(&go(aim.clone(), word(&[1])))), (TYPE_MISMATCH, "aim", "0", PORTABLE, "T"));
    let r = report(&go(aim, steered()));
    assert_eq!(r.codes(), [TYPE_MISMATCH, V]);
    let d = &r.diagnostics[1];
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(d)), ("go", "root", (V, Some("T"), true, false)));
    // An effect.
    let log =
        helper("log", vec![param("v", t())], exact(Ty::Unit), let_("r", propose(word(&[4]), target("t")), unit()));
    assert!(report(&go(log.clone(), word(&[1]))).is_admitted());
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

/// CT-LG2-05 (part: callables in records, form 1).
#[test]
fn ct_lg2_05_a_callable_in_a_record_keeps_its_exact_evidence() {
    let ev = helper("ev", vec![], exact(Ty::Unit), use_exact(ExactRole::Evidence, word(&[5])));
    let called = display_if(less(), call_value(field(var("r"), "f"), vec![]), unit());
    let p = tool(vec![ev, command("go", xy(), exact(Ty::Unit), let_("r", record(vec![("f", fn_ref("ev"))]), called))]);
    let r = report(&p);
    assert_eq!(r.diagnostics.len(), 1, "{:#?}", r.diagnostics);
    let d = &r.diagnostics[0];
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(d)), ("go", "1.1", (V, Some("Word"), true, false)));
    assert!(r.summary("tool", "go").unwrap().exact_sinks.contains(&(ExactRole::Evidence, "Word".to_string())));
}

/// CT-LG2-05 (part: module calls and budgets, form 1).
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

/// CT-LG2-05 (part: storage, form 1).
#[test]
fn ct_lg2_05_storage_keeps_the_qualifier() {
    // A saved display condition decoded as an exact goal (LG2.22): decoding retains the qualifier.
    let decoded = tool(vec![command(
        "go",
        vec![],
        exact(Ty::Unit),
        use_exact(ExactRole::Goal, read_snapshot("workspace.saved_condition", exact(Ty::ApproxBool))),
    )]);
    assert_eq!(fields(&only(&decoded)), (V, Some("ApproxBool"), false, false));
    // A word chosen by display_if, stored in an exact field.
    let stored = tool(vec![command(
        "keep",
        xy(),
        exact(Ty::Unit),
        write_workspace("word", exact(Ty::Word), display_if(less(), word(&[1]), word(&[2]))),
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

/// CT-LG2-05 (part: projection and matching, form 1).
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
            vec![(tag("a"), word(&[1]))],
            word(&[2]),
        ),
    )]);
    assert_eq!(fields(&only(&tagged)), (V, Some("Status"), true, false));
}

// LG2.20: ordinary control takes exact inputs only, and display branches and display bodies are pure.

/// CT-LG2-05 (part: LG2.20, ordinary control).
#[test]
fn lg2_20_ordinary_control_accepts_exact_inputs_only() {
    let short_circuit = tool(vec![command(
        "pick",
        xy(),
        exact(Ty::Word),
        if_(prim(Prim::And, vec![less(), boolean(true)]), word(&[1]), word(&[2])),
    )]);
    let d = only(&short_circuit);
    assert_eq!((d.path.as_str(), fields(&d)), ("0.0", (V, Some("ApproxBool"), false, false)));
    let exact_condition = tool(vec![command(
        "pick",
        xy(),
        exact(Ty::Word),
        if_(prim(Prim::ExactLess, vec![var("x"), var("y")]), word(&[1]), word(&[2])),
    )]);
    assert!(report(&exact_condition).is_admitted());
}

/// CT-LG2-05 (part: LG2.20, pure display branches and bodies).
#[test]
fn lg2_20_display_branches_and_display_bodies_are_pure() {
    // A state read inside a display branch.
    let read = tool(vec![view(
        "peek",
        xy(),
        exact(Ty::Unit),
        publish_view(display_if(less(), read_snapshot("tint", exact(Ty::Colour)), colour("blue"))),
    )]);
    let d = only(&read);
    assert_eq!((d.code, d.path.as_str()), (EFFECT_UNDECLARED, "0.1"));
    // A preview prepared by a callee of a display branch: display context propagates into callees.
    let preview = tool(vec![
        helper("prepare", vec![], exact(Ty::Unit), let_("r", propose(word(&[4]), target("t")), unit())),
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
fn lg2_1_a_base_type_mismatch_is_a_diagnostic_not_an_error() {
    // An ordinary if takes a Bool condition, and display_if an ApproxBool one (LG2.20).
    let mismatched = tool(vec![helper("f", vec![], exact(Ty::Word), if_(nat(1), word(&[6]), word(&[7])))]);
    assert_eq!(mismatch(&only(&mismatched)), (TYPE_MISMATCH, "f", "0", "Bool", "Nat"));
    let exact_condition = tool(vec![helper("f", vec![], display(Ty::Nat), display_if(boolean(true), nat(1), nat(2)))]);
    assert_eq!(mismatch(&only(&exact_condition)), (TYPE_MISMATCH, "f", "0", "ApproxBool", "Bool"));
}

#[test]
fn faults_without_a_source_diagnostic_are_errors() {
    // An unbound name, a function defined twice, a call of an unexported function and a named function called with a
    // wrong number of arguments have no code in chapters 02 and 13. Lowering refuses each of them before the checker
    // sees them. A function value called with another number of arguments is a mismatch (`mismatch.rs`).
    assert!(check(&tool(vec![helper("f", vec![], exact(Ty::Nat), var("z"))])).is_err());
    let twice = tool(vec![helper("f", vec![], exact(Ty::Nat), nat(1)), helper("f", vec![], exact(Ty::Nat), nat(2))]);
    assert!(check(&twice).is_err());
    let private = program(vec![
        module("lib", vec![helper("f", vec![], exact(Ty::Nat), nat(1))]),
        module("app", vec![helper("g", vec![], exact(Ty::Nat), call_in("lib", "f", vec![]))]),
    ]);
    assert!(check(&private).is_err());
    let arity = tool(vec![
        helper("f", vec![], exact(Ty::Nat), nat(1)),
        helper("g", vec![], exact(Ty::Nat), call("f", vec![nat(1)])),
    ]);
    assert!(check(&arity).is_err());
}

// The Fable attack on the M1 checker (owner decision, 15 November 2026): each program below slipped past the checker
// before pull request 149 fixed it. The leads were confirmed by these tests; the fixes are the checker's.

/// CT-LG2-05 (part: type variables bound through callable results, form 1).
#[test]
fn ct_lg2_05_a_type_variable_bound_through_a_callable_result_keeps_its_qualifier() {
    let t = || exact(Ty::Var("T".into()));
    let u = || exact(Ty::Var("U".into()));
    let steered = || closure(vec![], display(Ty::Word), display_if(less(), word(&[1]), word(&[2])));
    let pipe = helper(
        "pipe",
        vec![
            param("f", exact(fn_ty(vec![], t(), bound(&[], &["e"])))),
            param("g", exact(fn_ty(vec![t()], u(), bound(&[], &["d"])))),
        ],
        u(),
        call_value(var("g"), vec![call_value(var("f"), vec![])]),
    );
    let same = closure(vec![param("w", exact(Ty::Word))], exact(Ty::Word), var("w"));
    let sent = propose(call("pipe", vec![steered(), same]), target("t"));
    let d = only(&tool(vec![pipe, command("go", xy(), exact(Ty::Receipt), sent)]));
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(&d)), ("go", "0", (V, Some("Word"), false, true)));
    // The same binding reaches an exact goal inside the callee: the call runs a display instance of it. The goal also
    // refuses a value of a type variable (PHS.52, 16 November 2026).
    let aim = helper(
        "aim",
        vec![param("f", exact(fn_ty(vec![], t(), bound(&[], &["e"]))))],
        exact(Ty::Unit),
        use_exact(ExactRole::Goal, call_value(var("f"), vec![])),
    );
    let r = report(&tool(vec![aim, command("go", xy(), exact(Ty::Unit), call("aim", vec![steered()]))]));
    assert_eq!(r.codes(), [TYPE_MISMATCH, V]);
    assert_eq!(mismatch(&r.diagnostics[0]), (TYPE_MISMATCH, "aim", "0", PORTABLE, "T"));
    let d = &r.diagnostics[1];
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(d)), ("go", "root", (V, Some("T"), true, false)));
}

/// Goal and evidence sinks take portable data (owner decision of 16 November 2026 under PHS.52): a value whose type
/// is or holds a callable, a receipt, a word equality key or a compressed word is a type mismatch at the value, so
/// none reaches the host; an approximate value keeps its exactness code (LG2.22).
#[test]
fn phs52_goal_and_evidence_sinks_take_portable_data() {
    let callee = helper("h", vec![], exact(Ty::Nat), nat(1));
    let receipt = || propose(word(&[1]), target("t"));
    for role in [ExactRole::Goal, ExactRole::Evidence] {
        let sink =
            |value: Expr| tool(vec![callee.clone(), command("go", xy(), exact(Ty::Unit), use_exact(role, value))]);
        for value in [
            word(&[1]),
            record(vec![("n", nat(1)), ("w", word(&[2]))]),
            list(exact(Ty::Word), vec![word(&[1]), word(&[2])]),
        ] {
            assert!(report(&sink(value)).is_admitted(), "{role:?}");
        }
        for (value, supplied) in [
            (fn_ref("h"), "fn() -> Nat"),
            (receipt(), "Receipt"),
            (prim(Prim::WordKey, vec![word(&[1])]), "WordEqualityKey"),
            (prim(Prim::TraceKey, vec![word(&[1])]), "TraceEqualityKey"),
            (prim(Prim::Compress, vec![word(&[1])]), "CompressedWord"),
            (record(vec![("f", fn_ref("h"))]), "record { f: fn() -> Nat }"),
            (list(exact(Ty::Receipt), vec![receipt()]), "List<Receipt>"),
        ] {
            assert_eq!(mismatch(&only(&sink(value))), (TYPE_MISMATCH, "go", "0", PORTABLE, supplied), "{role:?}");
        }
        let approximate = only(&sink(Expr::Lit(Lit::Approx { decimal: "0.5".into(), precision: 53 })));
        assert_eq!(fields(&approximate), (V, Some("Approx"), false, true), "{role:?}");
    }
}

/// CT-LG2-05 (part: generic results of callables, form 1).
#[test]
fn ct_lg2_05_a_generic_result_keeps_the_result_qualifier_of_every_callable_it_may_return() {
    let t = || exact(Ty::Var("T".into()));
    let second = helper("second", vec![param("a", t()), param("b", t())], t(), var("b"));
    let fixed = || closure(vec![], exact(Ty::Word), word(&[1]));
    let steered = || closure(vec![], display(Ty::Word), display_if(less(), word(&[2]), word(&[3])));
    for args in [vec![fixed(), steered()], vec![steered(), fixed()]] {
        let chosen = call_value(call("second", args), vec![]);
        let d =
            only(&tool(vec![second.clone(), command("go", xy(), exact(Ty::Receipt), propose(chosen, target("t")))]));
        assert_eq!((d.function.as_str(), d.path.as_str(), fields(&d)), ("go", "0", (V, Some("Word"), false, true)));
    }
}

/// CT-LG2-05 (part: closures returned by display instances, form 1).
#[test]
fn ct_lg2_05_a_display_instance_cannot_hand_back_a_closure_that_previews_its_argument() {
    let t = || exact(Ty::Var("T".into()));
    let previews = || bound(&[c600_ir::Effect::PreparePreview], &[]);
    let later = helper(
        "later",
        vec![param("v", t()), param("k", exact(fn_ty(vec![t()], exact(Ty::Unit), previews())))],
        exact(fn_ty(vec![], exact(Ty::Unit), previews())),
        closure(vec![], exact(Ty::Unit), call_value(var("k"), vec![var("v")])),
    );
    let send = || {
        closure(vec![param("w", exact(Ty::Word))], exact(Ty::Unit), let_("r", propose(var("w"), target("t")), unit()))
    };
    let go = |arg: Expr| command("go", xy(), exact(Ty::Unit), call_value(call("later", vec![arg, send()]), vec![]));
    assert!(report(&tool(vec![later.clone(), go(word(&[1]))])).is_admitted());
    // A steered argument makes the handed-back callable display-qualified: calling it is a display region.
    let d = only(&tool(vec![later, go(display_if(less(), word(&[1]), word(&[2])))]));
    assert_eq!((d.function.as_str(), d.path.as_str(), d.code), ("go", "root", EFFECT_UNDECLARED));
}

/// CT-LG2-05 (part: LG2.20 with captured effect variables).
#[test]
fn lg2_20_a_captured_effect_variable_cannot_hide_exact_evidence_from_a_display_branch() {
    let poly = || exact(fn_ty(vec![], exact(Ty::Unit), bound(&[], &["e"])));
    let both = closure(
        vec![param("g", poly())],
        exact(Ty::Unit),
        let_("u", call_value(var("f"), vec![]), call_value(var("g"), vec![])),
    );
    let nothing = closure(vec![], exact(Ty::Unit), unit());
    let run = helper("run", vec![param("f", poly())], exact(Ty::Unit), call_value(both, vec![nothing]));
    let ev = helper("ev", vec![], exact(Ty::Unit), use_exact(ExactRole::Evidence, word(&[5])));
    let go = command("go", xy(), exact(Ty::Unit), display_if(less(), call("run", vec![fn_ref("ev")]), unit()));
    let d = only(&tool(vec![ev, run, go]));
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(&d)), ("go", "1", (V, Some("Word"), true, false)));
}

/// CT-LG2-05 (part: stored display fields, form 1).
#[test]
fn ct_lg2_05_a_stored_display_field_is_not_decoded_as_exact() {
    let keep = command("keep", xy(), exact(Ty::Unit), write_workspace("tint", display(Ty::Colour), tint()));
    let chosen =
        match_(read_snapshot("tint", exact(Ty::Colour)), vec![(Lit::Colour("red".into()), word(&[1]))], word(&[2]));
    let d = only(&tool(vec![keep, command("go", vec![], exact(Ty::Receipt), propose(chosen, target("t")))]));
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(&d)), ("go", "0.0", (V, Some("Colour"), false, true)));
}

/// CT-LG2-05 (part: LG2.20, approximate data cannot bound recursion).
#[test]
fn lg2_20_approximate_data_cannot_bound_recursion() {
    let ab = vec![param("a", exact(approx())), param("b", exact(approx()))];
    let again = call("count", vec![prim(Prim::ApproxAdd, vec![var("a"), var("a")]), var("b")]);
    let count = helper("count", ab, display(Ty::Nat), display_if(approx_less(var("a"), var("b")), again, nat(0)));
    let d = only(&tool(vec![count]));
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(&d)), ("count", "1", (V, Some("Nat"), true, false)));
}

/// CT-LG2-05 (part: LG2.22's precedence through a helper).
#[test]
fn lg2_22_an_approx_selected_under_approximate_control_keeps_its_precedence_through_a_helper() {
    let pick = helper("pick", xy(), exact(approx()), display_if(less(), to_approx(var("x")), to_approx(var("y"))));
    let add = helper("add", xy(), exact(Ty::Nat), exact_add(call("pick", vec![var("x"), var("y")]), nat(1)));
    let d = only(&tool(vec![pick, add]));
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(&d)), ("add", "0", (V, Some("Approx"), true, false)));
}

/// CT-LG2-05 (part: LG2.20 for the LP2 draft `assert`, DET.55): an assertion decides whether the run goes on, so its
/// predicate is exact control, as an ordinary if condition is. An approximate or a display-qualified boolean is
/// refused as the condition would be, and an exact comparison is admitted.
#[test]
fn lg2_20_an_assertion_takes_an_exact_predicate_as_an_if_does() {
    let shown = || display_if(less(), Expr::Lit(Lit::Bool(true)), Expr::Lit(Lit::Bool(false)));
    let predicates: [fn() -> Expr; 2] = [less, shown];
    for predicate in predicates {
        let assertion = only(&tool(vec![helper("f", xy(), exact(Ty::Unit), prim(Prim::Assert, vec![predicate()]))]));
        let condition = only(&tool(vec![helper("f", xy(), exact(Ty::Unit), if_(predicate(), unit(), unit()))]));
        assert_eq!(assertion.code, V, "{assertion:#?}");
        assert_eq!((fields(&assertion), assertion.path.as_str()), (fields(&condition), condition.path.as_str()));
    }
    let n = vec![param("n", exact(Ty::Nat))];
    let held = prim(Prim::Assert, vec![prim(Prim::ExactLess, vec![var("n"), nat(5)])]);
    assert!(report(&tool(vec![helper("f", n, exact(Ty::Unit), held)])).is_admitted());
}
