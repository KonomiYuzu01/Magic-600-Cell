//! LP2 plan section 5, acceptance check 4: effects (CT-LG3-01 to CT-LG3-03), capabilities and grants (CT-LG3-04) and
//! capability separation (CT-LG3-07), each test naming the part it covers. The run-time halves (one undoable action,
//! the fixed acknowledgement, saved words, workspace records across runs, research output) belong to M2 and M3.

mod support;

use c600_check::{
    admit, separation, Admission, Grant, Policy, Profile, Separation, AMBIENT_READ, EFFECT_UNDECLARED, NEVER_GRANTED,
    NOT_GRANTED,
};
use c600_ir::build::*;
use c600_ir::{Callee, Capability, Effect, Expr, FnKind, Function, Prim, Program, ReceiptField, Ty};
use support::{codes, report, tool};

const PROFILES: [Profile; 3] = [Profile::Solving, Profile::ToolAuthoring, Profile::Research];

fn effects(p: &Program, module: &str, function: &str) -> Vec<Effect> {
    report(p).summary(module, function).expect("function").effects.iter().copied().collect()
}

fn snapshot() -> Expr {
    read_snapshot("n", exact(Ty::Nat))
}

fn preview() -> Expr {
    propose(word(&[4]), target("t"))
}

fn unresolved(name: &str, result: Ty) -> Expr {
    Expr::Call { callee: Callee::Unresolved { name: name.into(), result: exact(result) }, args: vec![] }
}

/// `count() -> Nat`, reading the snapshot.
fn reader() -> Function {
    helper("count", vec![], exact(Ty::Nat), snapshot()).exported()
}

/// `prepare() -> Unit`, preparing a preview.
fn previewer() -> Function {
    helper("prepare", vec![], exact(Ty::Unit), let_("r", preview(), unit())).exported()
}

/// `apply(f: () -> R ! e) -> R`, polymorphic in the effect variable `e`.
fn apply(name: &str, result: Ty) -> Function {
    helper(
        name,
        vec![param("f", exact(fn_ty(vec![], exact(result.clone()), bound(&[], &["e"]))))],
        exact(result),
        call_value(var("f"), vec![]),
    )
}

fn entry(result: Ty, body: Expr) -> Function {
    command("go", vec![], exact(result), body)
}

fn admit_in(p: &Program, profile: Profile, grant: &Grant, policy: &Policy) -> Admission {
    admit(p, "tool", "go", profile, grant, policy).unwrap_or_else(|e| panic!("{e}"))
}

// CT-LG3-01: the function and import call graph preserves transitive effects and capability requirements, including
// higher-order and generic calls.

/// CT-LG3-01 (part: direct, transitive, higher-order and generic calls in IR; the row is whole with the other
/// `ct_lg3_01_*` tests here and `c600-syntax`'s `conformance_lg3.rs`).
#[test]
fn ct_lg3_01_calls_preserve_transitive_effects() {
    let t = || exact(Ty::Var("T".into()));
    let p = program(vec![
        module("lib", vec![reader()]),
        module(
            "tool",
            vec![
                helper("direct", vec![], exact(Ty::Nat), snapshot()),
                helper("transitive", vec![], exact(Ty::Nat), call("direct", vec![])),
                helper("imported", vec![], exact(Ty::Nat), call_in("lib", "count", vec![])),
                apply("apply", Ty::Nat),
                helper("higher_order", vec![], exact(Ty::Nat), call("apply", vec![fn_ref("direct")])),
                helper("id", vec![param("v", t())], t(), var("v")),
                helper("generic", vec![], exact(Ty::Nat), call_value(call("id", vec![fn_ref("direct")]), vec![])),
                helper("pure", vec![], exact(Ty::Nat), exact_add(nat(1), nat(2))),
            ],
        ),
    ]);
    assert!(report(&p).is_admitted());
    for f in ["direct", "transitive", "imported", "higher_order", "generic"] {
        assert_eq!(effects(&p, "tool", f), [Effect::ReadSnapshot], "{f}");
    }
    assert!(report(&p).summary("tool", "apply").unwrap().vars.contains("e"));
    assert!(report(&p).summary("tool", "pure").unwrap().is_pure());
}

/// CT-LG3-01 (part: recursive calls in IR).
#[test]
fn ct_lg3_01_recursion_reaches_its_fixed_point() {
    let n = || var("n");
    let p = tool(vec![
        command(
            "down",
            vec![param("n", exact(Ty::Nat))],
            exact(Ty::Unit),
            if_(
                prim(Prim::ExactLess, vec![n(), nat(1)]),
                write_workspace("k", exact(Ty::Nat), n()),
                call("again", vec![n()]),
            ),
        ),
        helper("again", vec![param("n", exact(Ty::Nat))], exact(Ty::Unit), call("down", vec![n()])),
    ]);
    assert!(report(&p).is_admitted());
    assert_eq!(effects(&p, "tool", "again"), [Effect::ChangeWorkspace]);
}

/// CT-LG3-01 (part: capability requirements across imports in IR).
#[test]
fn ct_lg3_01_capability_requirements_cross_imports_and_need_a_declaration() {
    // LG3.9: a module declares its required capabilities; an import supplies none.
    let lib =
        module(
            "lib",
            vec![helper("load", vec![], exact(Ty::Nat), Expr::FileRead { path: "data.txt".into(), ty: Ty::Nat })
                .exported()],
        )
        .declaring(&[Capability::File]);
    let app = || module("app", vec![command("go", vec![], exact(Ty::Nat), call_in("lib", "load", vec![]))]);
    let undeclared = report(&program(vec![lib.clone(), app()]));
    assert_eq!(undeclared.codes(), [EFFECT_UNDECLARED]);
    assert_eq!(undeclared.diagnostics[0].module, "app");
    assert!(undeclared.summary("app", "go").unwrap().capabilities.contains(&Capability::File));
    assert!(report(&program(vec![lib, app().declaring(&[Capability::File])])).is_admitted());
}

/// CT-LG3-01 (part: annotations, bounds and unresolved callables in IR).
#[test]
fn ct_lg3_01_annotations_bounds_and_unresolved_callables_cannot_erase_an_effect() {
    // LG3.2: an annotation restricts what checks; it cannot erase an inferred effect.
    let annotated =
        |b| tool(vec![reader(), helper("f", vec![], exact(Ty::Nat), call("count", vec![])).with_effects(b)]);
    assert_eq!(codes(&annotated(bound(&[], &[]))), [EFFECT_UNDECLARED]);
    assert!(report(&annotated(bound(&[Effect::ReadSnapshot], &[]))).is_admitted());
    // A pure-bounded callable parameter refuses an effectful argument at the call.
    let run = helper(
        "run",
        vec![param("f", exact(fn_ty(vec![], exact(Ty::Nat), bound(&[], &[]))))],
        exact(Ty::Nat),
        call_value(var("f"), vec![]),
    );
    let r = report(&tool(vec![reader(), run, helper("g", vec![], exact(Ty::Nat), call("run", vec![fn_ref("count")]))]));
    assert_eq!(r.codes(), [EFFECT_UNDECLARED]);
    assert_eq!((r.diagnostics[0].function.as_str(), r.diagnostics[0].path.as_str()), ("g", "0"));
    // An unresolved callable is never treated as pure: it is dynamic code (DET.36, RTE.45; PHS.52, 16 November 2026).
    assert_eq!(codes(&tool(vec![helper("f", vec![], exact(Ty::Nat), unresolved("mystery", Ty::Nat))])), [NOT_GRANTED]);
}

/// CT-LG3-01 (part: callables held in records, in IR).
#[test]
fn ct_lg3_01_a_record_keeps_the_effects_of_its_callables() {
    // `run(r: {f: () -> Unit ! b})` calls the callable in its record argument (pull request 149 review).
    let run = |b| {
        let field_ty = exact(Ty::Record(vec![("f".into(), exact(fn_ty(vec![], exact(Ty::Unit), b)))]));
        helper("run", vec![param("r", field_ty)], exact(Ty::Unit), call_value(field(var("r"), "f"), vec![]))
    };
    let go =
        |kind| function(kind, "go", vec![], exact(Ty::Unit), call("run", vec![record(vec![("f", fn_ref("prepare"))])]));
    // A pure-bounded field refuses an effectful callable at the call, as a pure-bounded parameter does.
    let pure = tool(vec![previewer(), run(bound(&[], &[])), go(FnKind::Command)]);
    let r = report(&pure);
    assert_eq!(r.codes(), [EFFECT_UNDECLARED]);
    assert_eq!((r.diagnostics[0].function.as_str(), r.diagnostics[0].path.as_str()), ("go", "0"));
    // An effect variable in a field is instantiated through the record (LG3.2), so a view cannot launder a preview.
    let open = tool(vec![previewer(), run(bound(&[], &["e"])), go(FnKind::Command)]);
    assert!(report(&open).is_admitted());
    assert_eq!(effects(&open, "tool", "go"), [Effect::PreparePreview]);
    assert_eq!(codes(&tool(vec![previewer(), run(bound(&[], &["e"])), go(FnKind::View)])), [EFFECT_UNDECLARED]);
}

// CT-LG3-02: no program effect can commit or obtain a token; a helper cannot hide that request under another effect.

/// Exhaustive on purpose: an added commit effect would have to be named here (LG3.1, LG3.4).
fn family(e: Effect) -> &'static str {
    match e {
        Effect::ReadSnapshot => "read snapshot",
        Effect::ChangeView => "change view",
        Effect::ChangeWorkspace => "change workspace",
        Effect::PreparePreview => "prepare preview",
        Effect::GrantedFile => "granted file",
        Effect::SeededRandomness => "seeded randomness",
        Effect::Budgets => "budgets",
    }
}

/// CT-LG3-02 (part: no effect commits or obtains a token, in IR; the overlay rows are in `c600-command`'s
/// `tests/layer.rs` and `tests/served_rows.rs`).
#[test]
fn ct_lg3_02_no_effect_commits_or_obtains_a_token() {
    let all = [
        Effect::ReadSnapshot,
        Effect::ChangeView,
        Effect::ChangeWorkspace,
        Effect::PreparePreview,
        Effect::GrantedFile,
        Effect::SeededRandomness,
        Effect::Budgets,
    ];
    assert!(all.iter().all(|e| !family(*e).contains("commit") && !family(*e).contains("token")));
    // A preview returns a receipt, never a token, and has only the preview effect.
    let p = tool(vec![entry(Ty::Receipt, preview())]);
    assert!(report(&p).is_admitted());
    assert_eq!(effects(&p, "tool", "go"), [Effect::PreparePreview]);
    // The only way to name a commit is an unresolved callable, which is refused.
    assert_eq!(codes(&tool(vec![entry(Ty::Unit, unresolved("commit", Ty::Unit))])), [NOT_GRANTED]);
    // A helper cannot hide a preview under another effect.
    assert_eq!(codes(&tool(vec![previewer().with_effects(bound(&[Effect::ChangeView], &[]))])), [EFFECT_UNDECLARED]);
}

// CT-LG3-03: views refuse workspace and preview effects through direct and indirect calls. The one undoable action of
// an invoked workspace change is the command layer's (`c600-command`).

/// CT-LG3-03 (part: the static half in IR; the undoable action runs in `c600-command`'s `job_output.rs` and
/// `filter_apply.rs`).
#[test]
fn ct_lg3_03_views_refuse_workspace_and_preview_effects() {
    let in_view = |body: Expr, mut functions: Vec<Function>| {
        functions.push(view("v", vec![], exact(Ty::Unit), body));
        codes(&tool(functions))
    };
    assert_eq!(in_view(write_workspace("k", exact(Ty::Nat), nat(1)), vec![]), [EFFECT_UNDECLARED]);
    assert_eq!(in_view(let_("r", preview(), unit()), vec![]), [EFFECT_UNDECLARED]);
    assert_eq!(in_view(call("prepare", vec![]), vec![previewer()]), [EFFECT_UNDECLARED]);
    let through_argument = call("apply", vec![fn_ref("prepare")]);
    assert_eq!(in_view(through_argument, vec![previewer(), apply("apply", Ty::Unit)]), [EFFECT_UNDECLARED]);
    assert!(in_view(publish_view(snapshot()), vec![]).is_empty());
}

// CT-LG3-04: undeclared or ungranted file, clock and randomness requests fail; network, process and session-file
// access fail even when requested in a manifest.

fn request(cap: Capability) -> Expr {
    match cap {
        Capability::File => Expr::FileRead { path: "data.txt".into(), ty: Ty::Nat },
        Capability::Clock => Expr::ReadClock,
        Capability::Randomness => Expr::RandomDraw,
        other => panic!("no operation requests {other:?}"),
    }
}

/// The declared result of a request: a clock reading is display-qualified, since seconds are a display unit only
/// (LG2.6, DET.18).
fn result_of(cap: Capability) -> c600_ir::QTy {
    match cap {
        Capability::Clock => display(Ty::Nat),
        _ => exact(Ty::Nat),
    }
}

/// CT-LG3-04 (part: file, clock and randomness in IR; the row is whole with the source and run-time halves).
#[test]
fn ct_lg3_04_file_clock_and_randomness_need_a_declaration_and_a_grant() {
    for cap in [Capability::File, Capability::Clock, Capability::Randomness] {
        let declaring = |caps: &[Capability]| {
            program(vec![module("tool", vec![command("go", vec![], result_of(cap), request(cap))]).declaring(caps)])
        };
        // A clock or a file read without a grant is an ambient read (PHS.52, 16 November 2026).
        let refused = if cap == Capability::Randomness { NOT_GRANTED } else { AMBIENT_READ };
        for profile in PROFILES {
            let run = |p: Program, grant: Grant| admit_in(&p, profile, &grant, &Policy::interim()).codes();
            let case = format!("{cap:?} in {profile:?}");
            assert_eq!(run(declaring(&[]), Grant::none()), [EFFECT_UNDECLARED, refused], "{case}");
            assert_eq!(run(declaring(&[]), Grant::of(&[cap])), [EFFECT_UNDECLARED], "{case}");
            assert_eq!(run(declaring(&[cap]), Grant::none()), [refused], "{case}");
            assert!(run(declaring(&[cap]), Grant::of(&[cap])).is_empty(), "{case}");
        }
    }
}

/// CT-LG3-04 (part: the code of an ungranted file capability; owner decision of 16 November 2026 under PHS.52). An
/// entry that may read a file, itself or through a callable whose bound holds the file capability, is refused as an
/// ambient read; an entry that only writes a file is refused as authority not granted.
#[test]
fn phs52_an_ungranted_file_is_an_ambient_read_only_where_it_may_be_read() {
    let write = Expr::FileWrite { path: "out.txt".into(), value: Box::new(nat(1)) };
    let file = c600_ir::EffectBound { capabilities: [Capability::File].into(), ..bound(&[Effect::GrantedFile], &[]) };
    let reader = fn_ty(vec![], exact(Ty::Nat), file);
    let through = command("go", vec![param("f", exact(reader))], exact(Ty::Nat), call_value(var("f"), vec![]));
    for (go, refused) in [
        (entry(Ty::Unit, write.clone()), NOT_GRANTED),
        (entry(Ty::Nat, request(Capability::File)), AMBIENT_READ),
        (entry(Ty::Unit, let_("n", request(Capability::File), write)), AMBIENT_READ),
        (through, AMBIENT_READ),
    ] {
        let p = program(vec![module("tool", vec![go]).declaring(&[Capability::File])]);
        for profile in PROFILES {
            assert_eq!(admit_in(&p, profile, &Grant::none(), &Policy::interim()).codes(), [refused], "{profile:?}");
        }
    }
}

/// CT-LG3-04 (part: network, process and session files in IR). A manifest's network request is an ambient read
/// (PHS.52, 16 November 2026).
#[test]
fn ct_lg3_04_network_process_and_session_files_are_never_granted() {
    for cap in NEVER_GRANTED {
        let p = program(vec![module("tool", vec![entry(Ty::Nat, nat(1))]).declaring(&[cap])]);
        let refused = if cap == Capability::Network { AMBIENT_READ } else { NOT_GRANTED };
        for profile in PROFILES {
            let a = admit_in(&p, profile, &Grant::of(&NEVER_GRANTED), &Policy::interim());
            assert_eq!(a.codes(), [refused], "{cap:?} in {profile:?}");
            assert_eq!(a.diagnostics[0].path, "manifest");
        }
    }
}

/// CT-LG3-04 (part: a clock read needs the clock capability and has no effect family, LG3.8).
#[test]
fn lg3_8_a_clock_read_is_a_requirement_without_an_effect_family() {
    let go = command("go", vec![], display(Ty::Nat), Expr::ReadClock);
    let p = program(vec![module("tool", vec![go]).declaring(&[Capability::Clock])]);
    let s = report(&p).summary("tool", "go").unwrap().clone();
    assert!(s.effects.is_empty());
    assert_eq!(s.capabilities.into_iter().collect::<Vec<_>>(), [Capability::Clock]);
}

// CT-LG3-07, M1 part: in the solving profile a program that prepares previews reads no state and no preview outcome,
// and helpers and other modules do not launder this. Under chapter 07's interim table an own tool prepares no preview
// in a solving session at all, so that refusal is `LG3:capability.not_granted`; the flow rule waits for LQ2.

fn reads_then_previews() -> Program {
    tool(vec![entry(Ty::Unit, let_("n", snapshot(), let_("r", preview(), unit())))])
}

fn reads_an_outcome() -> Program {
    tool(vec![entry(Ty::Nat, read_receipt(preview(), ReceiptField::N))])
}

fn launders_through_a_module() -> Program {
    program(vec![
        module("probe", vec![reader()]),
        module(
            "tool",
            vec![entry(Ty::Unit, let_("n", call_in("probe", "count", vec![]), let_("r", preview(), unit())))],
        ),
    ])
}

fn previews_only() -> Program {
    tool(vec![entry(Ty::Text, read_receipt(preview(), ReceiptField::Id))])
}

fn declassified_then_previews() -> Program {
    let fixed = Expr::Declassified { id: "test.fixed".into(), ty: Ty::Nat };
    tool(vec![entry(Ty::Unit, let_("k", fixed, let_("r", preview(), unit())))])
}

fn reads_only() -> Program {
    tool(vec![entry(Ty::Unit, publish_view(snapshot()))])
}

#[test]
fn ct_lg3_07_no_own_tool_preview_in_a_solving_session_before_the_joint_setting() {
    let cases = [
        ("reads state, then previews", reads_then_previews(), true),
        ("reads a preview outcome", reads_an_outcome(), true),
        ("launders a read through a second module", launders_through_a_module(), true),
        ("previews only", previews_only(), false),
    ];
    for (name, p, reads_state) in cases {
        assert!(report(&p).is_admitted(), "{name}: the program itself checks");
        let a = admit_in(&p, Profile::Solving, &Grant::none(), &Policy::interim());
        assert_eq!(a.codes(), [NOT_GRANTED], "{name}");
        assert_eq!(a.diagnostics[0].path, "entry", "{name}");
        assert_eq!(a.separation, Separation { reads_state, prepares_previews: true }, "{name}");
        // Tool authoring and research turn a proposal into a capture (BND.60); ONE.32 stands there (BND.42).
        for profile in [Profile::ToolAuthoring, Profile::Research] {
            let a = admit_in(&p, profile, &Grant::none(), &Policy::interim());
            assert!(a.is_admitted() && a.open_cells.is_empty(), "{name} in {profile:?}");
        }
    }
}

#[test]
fn ct_lg3_07_only_enabled_fixed_declassifiers_cross_the_boundary() {
    let p = declassified_then_previews();
    let summary = report(&p).summary("tool", "go").unwrap().clone();
    // The interim policy enables none (BND.54); `test.fixed` stands in for a fixed declassifier of chapter 07.
    assert_eq!(separation(&summary, &Policy::interim()), Separation { reads_state: true, prepares_previews: true });
    let enabled = Policy::enabling(&["test.fixed"]);
    assert_eq!(separation(&summary, &enabled), Separation { reads_state: false, prepares_previews: true });
    assert!(separation(&summary, &Policy::enabling(&["test.other"])).reads_state);
    // The interim table still bars an own tool's preview in a solving session, declassifier or not.
    assert_eq!(admit_in(&p, Profile::Solving, &Grant::none(), &enabled).codes(), [NOT_GRANTED]);
}

#[test]
fn bnd_59_what_an_own_tool_may_read_in_a_solving_session_stays_open() {
    // The checker raises no refusal for the open cell and reports it instead of filling it.
    let a = admit_in(&reads_only(), Profile::Solving, &Grant::none(), &Policy::interim());
    assert!(a.is_admitted());
    assert_eq!(a.open_cells, ["BND.59"]);
    assert_eq!(a.separation, Separation { reads_state: true, prepares_previews: false });
}

#[test]
fn ct_bnd_24_flow_refused_is_never_emitted_before_lq2() {
    // CT-BND-24 and CT-ERR-15: `LG3:capability.flow_refused` is not emitted before LQ2 sets the flow rule.
    let programs = [
        reads_then_previews(),
        reads_an_outcome(),
        launders_through_a_module(),
        previews_only(),
        declassified_then_previews(),
        reads_only(),
    ];
    for p in &programs {
        for profile in PROFILES {
            for policy in [Policy::interim(), Policy::enabling(&["test.fixed"])] {
                let a = admit_in(p, profile, &Grant::none(), &policy);
                assert!(a.diagnostics.iter().all(|d| d.code != "LG3:capability.flow_refused"), "{a:#?}");
            }
        }
    }
}

// The Fable attack on the M1 checker (owner decision, 15 November 2026): each program below slipped past the
// effect inference before pull request 149 fixed it.

/// CT-LG3-01 (part: generic results that may return a record of callables).
#[test]
fn ct_lg3_01_a_generic_result_keeps_the_callables_of_every_record_it_may_return() {
    let t = || exact(Ty::Var("T".into()));
    let holder = |b| exact(Ty::Record(vec![("f".into(), exact(fn_ty(vec![], exact(Ty::Unit), b)))]));
    let nothing = closure(vec![], exact(Ty::Unit), unit());
    let quiet = helper("quiet", vec![], holder(bound(&[], &[])), record(vec![("f", nothing)]));
    let loud =
        helper("loud", vec![], holder(bound(&[Effect::PreparePreview], &[])), record(vec![("f", fn_ref("prepare"))]));
    let second = helper("second", vec![param("a", t()), param("b", t())], t(), var("b"));
    let picked = call("second", vec![call("quiet", vec![]), call("loud", vec![])]);
    let p = tool(vec![previewer(), quiet, loud, second, entry(Ty::Unit, call_value(field(picked, "f"), vec![]))]);
    assert_eq!(effects(&p, "tool", "go"), [Effect::PreparePreview]);
    let a = admit_in(&p, Profile::Solving, &Grant::none(), &Policy::interim());
    assert_eq!(a.codes(), [NOT_GRANTED]);
    assert_eq!(a.separation, Separation { reads_state: false, prepares_previews: true });
}

/// CT-LG3-01 (part: closures over callables with effect variables).
#[test]
fn ct_lg3_01_a_closure_parameter_does_not_capture_the_effect_variable_of_a_captured_callable() {
    let poly = || exact(fn_ty(vec![], exact(Ty::Unit), bound(&[], &["e"])));
    let run = || {
        let both = closure(
            vec![param("g", poly())],
            exact(Ty::Unit),
            let_("u", call_value(var("f"), vec![]), call_value(var("g"), vec![])),
        );
        let nothing = closure(vec![], exact(Ty::Unit), unit());
        helper("run", vec![param("f", poly())], exact(Ty::Unit), call_value(both, vec![nothing]))
    };
    let go = |f: &str| entry(Ty::Unit, call("run", vec![fn_ref(f)]));
    let previews = tool(vec![previewer(), run(), go("prepare")]);
    assert_eq!(effects(&previews, "tool", "go"), [Effect::PreparePreview]);
    let a = admit_in(&previews, Profile::Solving, &Grant::none(), &Policy::interim());
    assert_eq!(a.codes(), [NOT_GRANTED]);
    assert_eq!(a.separation, Separation { reads_state: false, prepares_previews: true });
    let peek = helper("peek", vec![], exact(Ty::Unit), let_("n", snapshot(), unit()));
    let reads = tool(vec![peek, run(), go("peek")]);
    let a = admit_in(&reads, Profile::Solving, &Grant::none(), &Policy::interim());
    assert_eq!(a.separation, Separation { reads_state: true, prepares_previews: false });
    assert_eq!(a.open_cells, ["BND.59"]);
}

/// CT-LG3-01 (part: effect variables in result types).
#[test]
fn ct_lg3_01_a_call_instantiates_the_effect_variable_of_its_result_type() {
    let poly = || exact(fn_ty(vec![], exact(Ty::Unit), bound(&[], &["e"])));
    let same = || helper("same", vec![param("f", poly())], poly(), var("f"));
    let handed_back = || call_value(call("same", vec![fn_ref("prepare")]), vec![]);
    let direct = tool(vec![previewer(), same(), entry(Ty::Unit, handed_back())]);
    assert_eq!(effects(&direct, "tool", "go"), [Effect::PreparePreview]);
    let wash = helper("wash", vec![param("g", poly())], exact(Ty::Unit), handed_back());
    let nothing = closure(vec![], exact(Ty::Unit), unit());
    let washed = tool(vec![previewer(), same(), wash, entry(Ty::Unit, call("wash", vec![nothing]))]);
    assert_eq!(effects(&washed, "tool", "go"), [Effect::PreparePreview]);
    let a = admit_in(&washed, Profile::Solving, &Grant::none(), &Policy::interim());
    assert_eq!(a.codes(), [NOT_GRANTED]);
    assert_eq!(a.separation, Separation { reads_state: false, prepares_previews: true });
}
