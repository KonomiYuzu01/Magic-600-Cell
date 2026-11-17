//! The built-in tool column of chapter 07's grant table in the solving profile ([`admit_built_in`], draft, LP2), the
//! class's rows (BND.57), the entry rule for previews and workspace changes (LG3.5, LG3.6) and the shown facts of an
//! entry that reads state-derived data (LB3.5, LP2 draft). Each test names the part of a conformance row it covers.
//! The two exceptions of the column (CT-BND-17, CT-BND-18) are not built.

mod support;

use std::collections::BTreeSet;

use c600_check::{
    admit, admit_built_in, check, Admission, Grant, Policy, Profile, Row, Separation, EFFECT_UNDECLARED, NOT_GRANTED,
};
use c600_ir::build::*;
use c600_ir::{Callee, Effect, EndTest, Expr, Function, Lit, Prim, Program, Ty};
use support::tool;

/// The row c600-tools' built-in tool lists: proposals to `operation.preview`.
const PREVIEW: &str = "operation.preview";

fn rows(rows: &[Row]) -> BTreeSet<Row> {
    rows.iter().cloned().collect()
}

fn preview_row() -> BTreeSet<Row> {
    rows(&[Row::Preview(PREVIEW.into())])
}

fn w() -> Vec<c600_ir::Param> {
    vec![param("w", exact(Ty::Word))]
}

/// `cmd go(w: Word) -> Receipt`, exported.
fn go(body: Expr) -> Function {
    command("go", w(), exact(Ty::Receipt), body).exported()
}

fn propose_to(word: Expr) -> Expr {
    propose(word, target(PREVIEW))
}

fn built_in(p: &Program, class: &BTreeSet<Row>, policy: &Policy) -> Admission {
    admit_built_in(p, "tool", "go", class, &Grant::none(), policy).unwrap_or_else(|e| panic!("{e}"))
}

/// The code and path of every diagnostic.
fn found(a: &Admission) -> Vec<(&'static str, &str)> {
    a.diagnostics.iter().map(|d| (d.code, d.path.as_str())).collect()
}

/// The code and path of every diagnostic of `p`'s built-in admission under `class` and the interim policy.
fn refusals(p: &Program, class: &BTreeSet<Row>) -> Vec<(&'static str, String)> {
    built_in(p, class, &Policy::interim()).diagnostics.iter().map(|d| (d.code, d.path.clone())).collect()
}

/// `LG3:capability.not_granted` at each of `paths`.
fn not_granted_at(paths: &[&str]) -> Vec<(&'static str, String)> {
    paths.iter().map(|p| (NOT_GRANTED, p.to_string())).collect()
}

/// CT-LG3-07 (part: the built-in column, a preview of the word the solver named) and CT-BND-15 (part: the entry's
/// rows lie within the rows its class lists, BND.57; not run: the class review and its record, LP3).
#[test]
fn a_built_in_previews_the_word_the_solver_named_to_a_listed_target() {
    let p = tool(vec![go(propose_to(var("w")))]);
    let a = built_in(&p, &preview_row(), &Policy::interim());
    assert!(a.is_admitted(), "{a:#?}");
    assert_eq!(a.separation, Separation { reads_state: false, prepares_previews: true });
    assert!(a.open_cells.is_empty());
    let report = check(&p).unwrap();
    assert_eq!(report.summary("tool", "go").unwrap().rows, preview_row());
    // The own column refuses the same entry: an own tool prepares no preview until the joint setting (BND.56).
    let own = admit(&p, "tool", "go", Profile::Solving, &Grant::none(), &Policy::interim()).unwrap();
    assert_eq!(own.codes(), [NOT_GRANTED]);
}

/// CT-LG3-07 (part: the built-in column refuses a preview of a word the solver did not name).
#[test]
fn a_built_in_refuses_a_word_the_solver_did_not_name() {
    let class = preview_row();
    let literal = tool(vec![go(propose_to(word(&[1])))]);
    assert_eq!(found(&built_in(&literal, &class, &Policy::interim())), [(NOT_GRANTED, "root")]);
    let shadowed = tool(vec![go(let_("w", word(&[1]), propose_to(var("w"))))]);
    assert_eq!(found(&built_in(&shadowed, &class, &Policy::interim())), [(NOT_GRANTED, "1")]);
    let in_closure =
        tool(vec![go(call_value(closure(w(), exact(Ty::Receipt), propose_to(var("w"))), vec![word(&[1])]))]);
    assert_eq!(found(&built_in(&in_closure, &class, &Policy::interim())), [(NOT_GRANTED, "1.0")]);
    // A proposal in a callee is refused at the entry: the solver names a word to the entry.
    let prepare = helper("prepare", w(), exact(Ty::Receipt), propose_to(var("w")));
    let through_callee = tool(vec![prepare, go(call("prepare", vec![var("w")]))]);
    assert_eq!(found(&built_in(&through_callee, &class, &Policy::interim())), [(NOT_GRANTED, "entry")]);
}

/// CT-LG3-07 (part: the built-in column previews a menu word the solver named in the pinned menu's letters, LB1.8;
/// LP2 draft): a `MenuWord` parameter is a word the solver names, and a menu literal is not.
#[test]
fn a_built_in_previews_the_menu_word_the_solver_named() {
    let class = preview_row();
    let m = || vec![param("m", exact(Ty::MenuWord))];
    let named = tool(vec![command("go", m(), exact(Ty::Receipt), propose_to(var("m"))).exported()]);
    let a = built_in(&named, &class, &Policy::interim());
    assert!(a.is_admitted(), "{a:#?}");
    assert_eq!(a.separation, Separation { reads_state: false, prepares_previews: true });
    let literal = tool(vec![command("go", m(), exact(Ty::Receipt), propose_to(menu_word(&[[0, 1]]))).exported()]);
    assert_eq!(found(&built_in(&literal, &class, &Policy::interim())), [(NOT_GRANTED, "root")]);
    let built =
        tool(vec![command("go", m(), exact(Ty::Receipt), propose_to(prim(Prim::Inverse, vec![var("m")]))).exported()]);
    assert_eq!(found(&built_in(&built, &class, &Policy::interim())), [(NOT_GRANTED, "root")]);
}

/// CT-BND-15 (part: a row outside the class is refused, BND.57; not run: the class review, LP3).
#[test]
fn bnd_57_a_built_in_uses_only_the_rows_its_class_lists() {
    let other_target = tool(vec![go(propose(var("w"), target("operation.execute")))]);
    assert_eq!(found(&built_in(&other_target, &preview_row(), &Policy::interim())), [(NOT_GRANTED, "entry")]);
    let reads = command("go", w(), exact(Ty::Nat), read_snapshot("best", exact(Ty::Nat))).exported();
    let p = tool(vec![reads]);
    assert_eq!(found(&built_in(&p, &preview_row(), &Policy::interim())), [(NOT_GRANTED, "entry")]);
    let a = built_in(&p, &rows(&[Row::Read("best".into())]), &Policy::interim());
    assert!(a.is_admitted(), "{a:#?}");
    assert_eq!(a.separation, Separation { reads_state: true, prepares_previews: false });
    assert!(a.open_cells.is_empty(), "BND.59 is the own column's open cell");
    let shows = command("go", w(), exact(Ty::Unit), publish_view(read_snapshot("best", exact(Ty::Nat)))).exported();
    let p = tool(vec![shows]);
    assert_eq!(found(&built_in(&p, &rows(&[Row::Read("best".into())]), &Policy::interim())), [(NOT_GRANTED, "entry")]);
    assert!(built_in(&p, &rows(&[Row::Read("best".into()), Row::View]), &Policy::interim()).is_admitted());
}

/// CT-BND-15 (part: rows the checker cannot name are refused, BND.57).
#[test]
fn bnd_57_a_row_the_checker_cannot_name_is_refused() {
    let class = rows(&[Row::Read("best".into()), Row::Preview(PREVIEW.into())]);
    let unknown_target =
        command("go", vec![param("t", exact(Ty::Target))], exact(Ty::Receipt), propose(word(&[1]), var("t")));
    let p = tool(vec![unknown_target.exported()]);
    assert!(found(&built_in(&p, &class, &Policy::interim())).contains(&(NOT_GRANTED, "entry")));
    let reader = fn_ty(vec![], exact(Ty::Nat), bound(&[Effect::ReadSnapshot], &[]));
    let calls = command("go", vec![param("f", exact(reader))], exact(Ty::Nat), call_value(var("f"), vec![])).exported();
    assert_eq!(found(&built_in(&tool(vec![calls]), &class, &Policy::interim())), [(NOT_GRANTED, "entry")]);
    let dynamic =
        Expr::Call { callee: Callee::Unresolved { name: "eval".into(), result: exact(Ty::Nat) }, args: vec![] };
    let p = tool(vec![command("go", vec![], exact(Ty::Nat), dynamic).exported()]);
    let codes = built_in(&p, &class, &Policy::interim()).codes();
    assert_eq!(codes.last(), Some(&NOT_GRANTED), "{codes:?}");
}

/// CT-LG3-07 (part: the built-in column keeps capability separation, LG3.13, BND.39).
#[test]
fn a_built_in_never_both_reads_state_and_prepares_a_preview() {
    let both = go(let_("n", read_snapshot("best", exact(Ty::Nat)), propose_to(var("w"))));
    let class = rows(&[Row::Read("best".into()), Row::Preview(PREVIEW.into())]);
    let a = built_in(&tool(vec![both]), &class, &Policy::interim());
    assert_eq!(found(&a), [(NOT_GRANTED, "entry")]);
    assert_eq!(a.separation, Separation { reads_state: true, prepares_previews: true });
}

/// CT-BND-15 (part: the workspace-change row of the built-in column: a word only if the solver named it or an enabled
/// declassifier covers it). The gap LP3 recorded: an entry that reads state and writes a workspace record.
#[test]
fn a_built_in_writes_a_word_only_if_the_solver_named_it_or_a_declassifier_covers_it() {
    let keep =
        |value: Expr| command("go", w(), exact(Ty::Unit), write_workspace("kept", exact(Ty::Word), value)).exported();
    let class = rows(&[Row::Read("best_word".into()), Row::Workspace("kept".into())]);
    // c600-tools' `keep`: its class lists proposals only, so both rows are refused (BND.57), and the word read from
    // the state is not one the solver named ...
    let relay = tool(vec![keep(read_snapshot("best_word", exact(Ty::Word)))]);
    let refused = [(NOT_GRANTED, "entry"), (NOT_GRANTED, "entry"), (NOT_GRANTED, "root")];
    assert_eq!(found(&built_in(&relay, &preview_row(), &Policy::interim())), refused);
    // ... which stays refused with both rows listed.
    assert_eq!(found(&built_in(&relay, &class, &Policy::interim())), [(NOT_GRANTED, "root")]);
    assert!(built_in(&tool(vec![keep(var("w"))]), &class, &Policy::interim()).is_admitted());
    let declassified = tool(vec![keep(Expr::Declassified { id: "test.fixed".into(), ty: Ty::Word })]);
    let class = rows(&[Row::Declassified("test.fixed".into()), Row::Workspace("kept".into())]);
    assert_eq!(found(&built_in(&declassified, &class, &Policy::interim())), [(NOT_GRANTED, "root")]);
    assert!(built_in(&declassified, &class, &Policy::enabling(&["test.fixed"])).is_admitted());
    // A record that is no word needs no named word.
    let count = command("go", w(), exact(Ty::Unit), write_workspace("count", exact(Ty::Nat), nat(1))).exported();
    assert!(built_in(&tool(vec![count]), &rows(&[Row::Workspace("count".into())]), &Policy::interim()).is_admitted());
}

/// CT-BND-15 (part: the workspace-change row of the built-in column for a write at a type variable, which may stand
/// for a word; review finding F1). A generic helper `put<T>(x: T)` that writes `x` is refused at the entry whatever the
/// entry passes. With the word the solver named the refusal is conservative: the checker does not follow a named word
/// into a callee, as for a proposal in a callee.
#[test]
fn a_workspace_write_at_a_type_variable_may_carry_a_word() {
    let t = || exact(Ty::Var("T".into()));
    let put = || helper("put", vec![param("x", t())], exact(Ty::Unit), write_workspace("kept", t(), var("x")));
    let go = |arg: Expr| command("go", w(), exact(Ty::Unit), call("put", vec![arg])).exported();
    let kept = rows(&[Row::Workspace("kept".into())]);
    let literal = tool(vec![put(), go(word(&[1]))]);
    assert_eq!(found(&built_in(&literal, &kept, &Policy::interim())), [(NOT_GRANTED, "entry")]);
    let read = tool(vec![put(), go(read_snapshot("best_word", exact(Ty::Word)))]);
    let read_and_kept = rows(&[Row::Read("best_word".into()), Row::Workspace("kept".into())]);
    assert_eq!(found(&built_in(&read, &read_and_kept, &Policy::interim())), [(NOT_GRANTED, "entry")]);
    // The control: the word the solver named, passed through the generic helper, is refused conservatively.
    let named = tool(vec![put(), go(var("w"))]);
    assert_eq!(found(&built_in(&named, &kept, &Policy::interim())), [(NOT_GRANTED, "entry")]);
    // At the entry itself a write at a type variable takes the word the solver named and nothing else.
    let params = || vec![param("w", exact(Ty::Word)), param("x", t())];
    let keep = |value: Expr| command("go", params(), exact(Ty::Unit), write_workspace("kept", t(), value)).exported();
    assert_eq!(found(&built_in(&tool(vec![keep(var("x"))]), &kept, &Policy::interim())), [(NOT_GRANTED, "root")]);
    assert!(built_in(&tool(vec![keep(var("w"))]), &kept, &Policy::interim()).is_admitted());
}

/// CT-BND-15 (part: the workspace-change row of the built-in column for a write of a callable whose type holds a word
/// or a type variable; review finding F1, "any type that contains one"). A stored callable that returns a word holds
/// it, so its write is refused at the write; conservatively also when the closure returns the word the solver named.
/// A callable whose type holds neither is written.
#[test]
fn a_workspace_write_of_a_callable_may_carry_a_word() {
    let kept = rows(&[Row::Workspace("kept".into())]);
    let params = || vec![param("w", exact(Ty::Word)), param("x", exact(Ty::Var("T".into())))];
    let keep = |result: Ty, value: Expr| {
        let ty = exact(fn_ty(vec![], exact(result.clone()), bound(&[], &[])));
        let write = write_workspace("kept", ty, closure(vec![], exact(result), value));
        command("go", params(), exact(Ty::Unit), write).exported()
    };
    let refused = [(NOT_GRANTED, "root")];
    assert_eq!(found(&built_in(&tool(vec![keep(Ty::Word, word(&[1]))]), &kept, &Policy::interim())), refused);
    assert_eq!(found(&built_in(&tool(vec![keep(Ty::Var("T".into()), var("x"))]), &kept, &Policy::interim())), refused);
    assert_eq!(found(&built_in(&tool(vec![keep(Ty::Word, var("w"))]), &kept, &Policy::interim())), refused);
    assert!(built_in(&tool(vec![keep(Ty::Nat, nat(1))]), &kept, &Policy::interim()).is_admitted());
}

/// CT-BND-15 (part: the workspace-change row of the built-in column for a write of a collection, a compressed word, a
/// word equality key or a trace equality key, LP2 drafts). A list, set or map with a word among its elements, keys or
/// values holds the word, and a compressed word or a key holds its letters, so their write is refused at the write;
/// conservatively also when they are built from the word the solver named. A collection whose type holds no word is
/// written.
#[test]
fn a_workspace_write_of_a_collection_or_compressed_word_may_carry_a_word() {
    let kept = rows(&[Row::Workspace("kept".into())]);
    let keep =
        |ty: Ty, value: Expr| command("go", w(), exact(Ty::Unit), write_workspace("kept", exact(ty), value)).exported();
    let words = || Box::new(exact(Ty::Word));
    let refused = [(NOT_GRANTED, "root")];
    let cases = [
        (Ty::List(words()), list(exact(Ty::Word), vec![word(&[1])])),
        (Ty::List(words()), list(exact(Ty::Word), vec![var("w")])),
        (Ty::Set(words()), set(exact(Ty::Word), vec![var("w")])),
        (Ty::Map(Box::new(exact(Ty::Nat)), words()), map(exact(Ty::Nat), exact(Ty::Word), vec![(nat(1), var("w"))])),
        (Ty::Map(words(), Box::new(exact(Ty::Nat))), map(exact(Ty::Word), exact(Ty::Nat), vec![(var("w"), nat(1))])),
        (Ty::CompressedWord, prim(Prim::Compress, vec![var("w")])),
        (Ty::WordEqualityKey, prim(Prim::WordKey, vec![var("w")])),
        (Ty::TraceEqualityKey, prim(Prim::TraceKey, vec![var("w")])),
    ];
    for (ty, value) in cases {
        let p = tool(vec![keep(ty.clone(), value)]);
        assert_eq!(found(&built_in(&p, &kept, &Policy::interim())), refused, "{ty:?}");
    }
    let counts = list(exact(Ty::Nat), vec![nat(1)]);
    assert!(built_in(&tool(vec![keep(Ty::List(Box::new(exact(Ty::Nat))), counts)]), &kept, &Policy::interim())
        .is_admitted());
}

/// LG3.5 and LG3.6 at the entry: a run whose entry is no command changes no workspace and prepares no preview. The
/// LP3 finding: an exported plain `fn` that proposes was admitted on the built-in path, which kept only `check`.
#[test]
fn lg3_6_an_exported_plain_fn_prepares_no_preview() {
    let plain = helper("go", w(), exact(Ty::Receipt), propose_to(var("w"))).exported();
    let p = tool(vec![plain]);
    // The finding confirmed: the static check admits it, because a command may call such a helper.
    assert!(check(&p).unwrap().is_admitted());
    let a = built_in(&p, &preview_row(), &Policy::interim());
    assert_eq!(found(&a), [(EFFECT_UNDECLARED, "entry")]);
    let writes = helper("go", w(), exact(Ty::Unit), write_workspace("kept", exact(Ty::Word), var("w"))).exported();
    let class = rows(&[Row::Workspace("kept".into())]);
    assert_eq!(
        found(&built_in(&tool(vec![writes.clone()]), &class, &Policy::interim())),
        [(EFFECT_UNDECLARED, "entry")]
    );
    // The own column in the solving profile: the workspace cell is "inside a command the solver invokes" (LG3.5).
    let own = admit(&tool(vec![writes]), "tool", "go", Profile::Solving, &Grant::none(), &Policy::interim()).unwrap();
    assert_eq!(found(&own), [(EFFECT_UNDECLARED, "entry")]);
    // In tool authoring and research `propose` goes to a capture (BND.60); the frozen text does not say whether such
    // a run needs a command entry, so `admit` keeps admitting it there.
    for profile in [Profile::ToolAuthoring, Profile::Research] {
        let a = admit(&p, "tool", "go", profile, &Grant::none(), &Policy::interim()).unwrap();
        assert!(a.is_admitted(), "{profile:?}: {a:#?}");
    }
}

/// A run starts at an exported entry; a built-in tool's capabilities follow the grant (LG3.8, LG3.9).
#[test]
fn a_built_in_entry_is_exported() {
    let hidden = command("go", w(), exact(Ty::Receipt), propose_to(var("w")));
    assert_eq!(found(&built_in(&tool(vec![hidden]), &preview_row(), &Policy::interim())), [(NOT_GRANTED, "entry")]);
}

/// CT-LB3-07 (part: the checker half of "no recommendation leaks through the inspector entries", LB3.5 and DT-J1 rule
/// 6; LP2 draft: an entry of the built-in column that reads state-derived data returns no word but the one the solver
/// named; not run: a selection that is no word, such as a cap the method chose, which the checker does not tell from a
/// per-cap fact, and the inspector library entries, which are LP3's).
///
/// The result of such an entry, when its type is word-carrying, is refused at each expression the body may return that
/// is not a `Word` or `MenuWord` parameter of the entry as given: the read itself, the rest of a `let`, a branch of an
/// `if`, an arm of a `match` and a record that holds the word. The word the solver named passes, also after a read, and
/// so does a number read from state. An entry that reads nothing shows no fact from state, so the rule leaves it alone.
#[test]
fn lb3_5_a_reading_built_in_returns_no_word_but_the_one_the_solver_named() {
    let best = || read_snapshot("best_word", exact(Ty::Word));
    let go = |result: Ty, body: Expr| tool(vec![command("go", w(), exact(result), body).exported()]);
    let class = rows(&[Row::Read("best_word".into()), Row::Read("best".into())]);
    assert_eq!(refusals(&go(Ty::Word, best()), &class), not_granted_at(&["root"]));
    assert_eq!(refusals(&go(Ty::Word, let_("x", best(), var("x"))), &class), not_granted_at(&["1"]));
    let either = if_(Expr::Lit(Lit::Bool(true)), var("w"), best());
    assert_eq!(refusals(&go(Ty::Word, either), &class), not_granted_at(&["2"]));
    let picked = match_(read_snapshot("best", exact(Ty::Nat)), vec![(Lit::Nat("1".into()), best())], var("w"));
    assert_eq!(refusals(&go(Ty::Word, picked), &class), not_granted_at(&["1"]));
    let fallback = match_(read_snapshot("best", exact(Ty::Nat)), vec![(Lit::Nat("1".into()), var("w"))], best());
    assert_eq!(refusals(&go(Ty::Word, fallback), &class), not_granted_at(&["2"]));
    // Under the name of the solver's word, or in a record, a word read from state is still not the one named.
    assert_eq!(refusals(&go(Ty::Word, let_("w", best(), var("w"))), &class), not_granted_at(&["1"]));
    let held = Ty::Record(vec![("next".into(), exact(Ty::Word))]);
    assert_eq!(refusals(&go(held, record(vec![("next", best())])), &class), not_granted_at(&["root"]));

    let after_a_read = go(Ty::Word, let_("n", read_snapshot("best", exact(Ty::Nat)), var("w")));
    assert!(built_in(&after_a_read, &class, &Policy::interim()).is_admitted());
    assert!(built_in(&go(Ty::Nat, read_snapshot("best", exact(Ty::Nat))), &class, &Policy::interim()).is_admitted());
    let m = vec![param("m", exact(Ty::MenuWord))];
    let menu = command("go", m, exact(Ty::MenuWord), let_("n", read_snapshot("best", exact(Ty::Nat)), var("m")));
    assert!(built_in(&tool(vec![menu.exported()]), &class, &Policy::interim()).is_admitted());
    assert!(built_in(&go(Ty::Word, word(&[1])), &BTreeSet::new(), &Policy::interim()).is_admitted());
}

/// CT-LB3-07 (part: the checker half of "no recommendation leaks through the inspector entries" for views, LB3.5 and
/// DT-J1 rule 6; LP2 draft: an entry of the built-in column that reads state-derived data publishes no word-carrying
/// view but of the word the solver named, also through a function it reaches; not run: a selection that is no word,
/// as above).
///
/// A view of a word read from state, directly, through a `let` or in a list with the solver's word, is refused at the
/// view. A function the entry reaches that publishes a word is refused at the entry, conservatively also when it
/// publishes the word the solver named: the solver names a word to the entry, not to the function. A view of the word
/// the solver named, or of a number read from state, passes, and an entry that reads nothing shows no fact from state.
#[test]
fn lb3_5_a_reading_built_in_shows_no_word_but_the_one_the_solver_named() {
    let best = || read_snapshot("best_word", exact(Ty::Word));
    let shows = |body: Expr| command("go", w(), exact(Ty::Unit), body).exported();
    let class = rows(&[Row::Read("best_word".into()), Row::Read("best".into()), Row::View]);
    assert_eq!(refusals(&tool(vec![shows(publish_view(best()))]), &class), not_granted_at(&["root"]));
    let named_later = let_("x", best(), publish_view(var("x")));
    assert_eq!(refusals(&tool(vec![shows(named_later)]), &class), not_granted_at(&["1"]));
    let listed = publish_view(list(exact(Ty::Word), vec![var("w"), best()]));
    assert_eq!(refusals(&tool(vec![shows(listed)]), &class), not_granted_at(&["root"]));
    let then_named = let_("n", read_snapshot("best", exact(Ty::Nat)), publish_view(var("w")));
    assert!(built_in(&tool(vec![shows(then_named)]), &class, &Policy::interim()).is_admitted());
    let count = publish_view(read_snapshot("best", exact(Ty::Nat)));
    assert!(built_in(&tool(vec![shows(count)]), &class, &Policy::interim()).is_admitted());
    let built = publish_view(word(&[1]));
    assert!(built_in(&tool(vec![shows(built)]), &rows(&[Row::View]), &Policy::interim()).is_admitted());

    let show = || helper("show", vec![param("x", exact(Ty::Word))], exact(Ty::Unit), publish_view(var("x")));
    let reads_then_calls = let_("n", read_snapshot("best", exact(Ty::Nat)), call("show", vec![var("w")]));
    assert_eq!(refusals(&tool(vec![show(), shows(reads_then_calls)]), &class), not_granted_at(&["entry"]));
    let calls = tool(vec![show(), shows(call("show", vec![var("w")]))]);
    assert!(built_in(&calls, &rows(&[Row::View]), &Policy::interim()).is_admitted());
}

/// CT-LB3-07 (part: the checker half of "named-word checks answer only their named word" and "no next-cap skip",
/// LB3.5's "no prefetch for an unnamed cap"; LP2 draft: an entry of the built-in column reads a row for an argument
/// the solver named to it, a literal, or a record or a collection of those; not run: the inspector library entries,
/// which are LP3's, and the facts themselves, which are the kernel's).
///
/// A read for the solver's word or cap, for a literal, or for a record or a list built from those passes, and so does
/// an end test's verdict read for the named word (LB3.2). A read for a cap the method computed, under its own name or
/// the solver's, for a value read from state, or for a word the method built, is refused at the read. A function the
/// entry reaches that reads for its own parameter is refused at the entry, since the solver names nothing to it; one
/// that reads for a literal passes.
#[test]
fn lb3_5_a_built_in_reads_only_for_what_the_solver_named() {
    let params = || vec![param("w", exact(Ty::Word)), param("cap", exact(Ty::Nat))];
    let go = |result: Ty, body: Expr| command("go", params(), exact(result), body).exported();
    let protection = |arg: Expr| read_snapshot_with("protection", arg, exact(Ty::Nat));
    let class = rows(&[Row::Read("protection".into()), Row::Read("next_cap".into())]);
    let given = [
        var("w"),
        var("cap"),
        nat(7),
        record(vec![("cap", var("cap")), ("word", var("w"))]),
        list(exact(Ty::Nat), vec![var("cap"), nat(1)]),
    ];
    for arg in given {
        let shown = format!("{arg:?}");
        let a = built_in(&tool(vec![go(Ty::Nat, protection(arg))]), &class, &Policy::interim());
        assert!(a.is_admitted(), "{shown}: {a:#?}");
    }
    let label = EndTest::LabelCheckpoint;
    let check_named = |arg: Expr| {
        let read = read_snapshot_with(label.row(), arg, exact(Ty::EndTest(label)));
        tool(vec![go(Ty::EndTest(label), read)])
    };
    let checks = rows(&[Row::Read(label.row().into())]);
    assert!(built_in(&check_named(var("w")), &checks, &Policy::interim()).is_admitted());
    let inverse = prim(Prim::Inverse, vec![var("w")]);
    assert_eq!(refusals(&check_named(inverse), &checks), not_granted_at(&["root"]));

    let next = protection(exact_add(var("cap"), nat(1)));
    assert_eq!(refusals(&tool(vec![go(Ty::Nat, next)]), &class), not_granted_at(&["root"]));
    let renamed = let_("cap", exact_add(var("cap"), nat(1)), protection(var("cap")));
    assert_eq!(refusals(&tool(vec![go(Ty::Nat, renamed)]), &class), not_granted_at(&["1"]));
    let fetched = protection(read_snapshot("next_cap", exact(Ty::Nat)));
    assert_eq!(refusals(&tool(vec![go(Ty::Nat, fetched)]), &class), not_granted_at(&["root"]));

    let own = helper("protected", vec![param("c", exact(Ty::Nat))], exact(Ty::Nat), protection(var("c")));
    let through = tool(vec![own, go(Ty::Nat, call("protected", vec![var("cap")]))]);
    assert_eq!(refusals(&through, &class), not_granted_at(&["entry"]));
    let literal = helper("seventh", vec![], exact(Ty::Nat), protection(nat(7)));
    let fixed = tool(vec![literal, go(Ty::Nat, call("seventh", vec![]))]);
    assert!(built_in(&fixed, &class, &Policy::interim()).is_admitted());
}
