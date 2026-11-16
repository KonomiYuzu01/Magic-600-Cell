//! The built-in tool column of chapter 07's grant table in the solving profile ([`admit_built_in`], draft, LP2), the
//! class's rows (BND.57) and the entry rule for previews and workspace changes (LG3.5, LG3.6). Each test names the
//! part of a conformance row it covers. The two exceptions of the column (CT-BND-17, CT-BND-18) are not built.

mod support;

use std::collections::BTreeSet;

use c600_check::{
    admit, admit_built_in, check, Admission, Grant, Policy, Profile, Row, Separation, EFFECT_UNDECLARED, NOT_GRANTED,
};
use c600_ir::build::*;
use c600_ir::{Callee, Effect, Expr, Function, Program, Ty};
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
