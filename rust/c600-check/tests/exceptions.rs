//! The built-in column's two exceptions (chapter 07 section 8, BND.1, BND.2, BND.69; [`admit_built_in`], LP2 draft):
//! the repair list at a cap the solver names (CT-BND-17) and the setup search for a target the solver names
//! (CT-BND-18). A class grants an exception by listing [`Row::RepairList`] or [`Row::SetupSearch`] for the query that
//! reads it: the IR draft has no `derive`, so the derived state is read as that engine row, for the cap or the target.
//! Each test names the part of a conformance row it covers. The engine rows themselves (the pairs in neutral order, the
//! setups that keep the buffers fixed) are the kernel's, and the library entries are LP3's.

mod support;

use std::collections::BTreeSet;

use c600_check::{
    admit_built_in, class_covers, Admission, Grant, Policy, Row, Separation, EFFECT_UNDECLARED, NOT_GRANTED,
};
use c600_ir::build::*;
use c600_ir::{ChildCaps, Expr, Lit, Param, Prim, Program, Ty};
use support::tool;

/// The engine row of the repair-list tests: M8's repair list at a cap (JI.12).
const REPAIR: &str = "repair_list";
/// The engine row of the setup-search tests: the setups for a target.
const SETUPS: &str = "setups";
/// The row c600-tools' built-in tool lists: proposals to `operation.preview`.
const PREVIEW: &str = "operation.preview";

fn rows(rows: &[Row]) -> BTreeSet<Row> {
    rows.iter().cloned().collect()
}

/// A repair row: the pair (x, d) with its y and z (BND.2).
fn pair() -> Ty {
    Ty::Record(vec![
        ("x".into(), exact(Ty::Nat)),
        ("d".into(), exact(Ty::Nat)),
        ("y".into(), exact(Ty::Word)),
        ("z".into(), exact(Ty::Word)),
    ])
}

fn pairs() -> Ty {
    Ty::List(Box::new(exact(pair())))
}

/// The pairs at the cap `arg`.
fn repair_at(arg: Expr) -> Expr {
    read_snapshot_with(REPAIR, arg, exact(pairs()))
}

/// The pairs at the cap the solver named.
fn named_list() -> Expr {
    repair_at(var("cap"))
}

fn cap() -> Vec<Param> {
    vec![param("cap", exact(Ty::Nat))]
}

/// `cmd go(cap: Nat) -> result`, exported: the solver names the cap.
fn at_cap(result: Ty, body: Expr) -> Program {
    tool(vec![command("go", cap(), exact(result), body).exported()])
}

/// A setup: a word.
fn setups() -> Ty {
    Ty::List(Box::new(exact(Ty::Word)))
}

/// The setups for the target `arg`.
fn setups_for(arg: Expr) -> Expr {
    read_snapshot_with(SETUPS, arg, exact(setups()))
}

/// `cmd go(goal: List<Nat>) -> result`, exported: the solver names the target, the pieces to bring home.
fn for_goal(result: Ty, body: Expr) -> Program {
    let goal = vec![param("goal", exact(Ty::List(Box::new(exact(Ty::Nat)))))];
    tool(vec![command("go", goal, exact(result), body).exported()])
}

fn built_in(p: &Program, class: &BTreeSet<Row>) -> Admission {
    admit_built_in(p, "tool", "go", class, &Grant::none(), &Policy::interim()).unwrap_or_else(|e| panic!("{e}"))
}

/// The code and path of every diagnostic of `p`'s built-in admission under `class` and the interim policy.
fn refusals(p: &Program, class: &BTreeSet<Row>) -> Vec<(&'static str, String)> {
    built_in(p, class).diagnostics.iter().map(|d| (d.code, d.path.clone())).collect()
}

/// `LG3:capability.not_granted` at each of `paths`.
fn not_granted_at(paths: &[&str]) -> Vec<(&'static str, String)> {
    paths.iter().map(|p| (NOT_GRANTED, p.to_string())).collect()
}

/// CT-BND-17 (part: the checker half: an inspector of the built-in column reads the repair list for the cap the solver
/// named to the entry, as given, and returns it or publishes it as read, also under a `let`, though its rows hold
/// words; LP2 draft; not run: the pairs and their neutral order, x in menu order then d, which are the kernel's, and
/// the library entry, LP3's).
///
/// The class's [`Row::RepairList`] covers the read (BND.57), and the list is the exception's output, so the shown-facts
/// rule (LB3.5) leaves its words alone. Listed only as [`Row::Read`], the same entries are refused there.
#[test]
fn bnd_2_the_repair_list_reads_for_the_cap_the_solver_named() {
    let class = rows(&[Row::RepairList(REPAIR.into()), Row::View]);
    assert!(class_covers(&class, &Row::Read(REPAIR.into())));
    assert!(!class_covers(&class, &Row::Read("protection".into())));
    let returned = at_cap(pairs(), named_list());
    let a = built_in(&returned, &class);
    assert!(a.is_admitted(), "{a:#?}");
    assert_eq!(a.separation, Separation { reads_state: true, prepares_previews: false });
    assert!(a.open_cells.is_empty());
    let bound = at_cap(pairs(), let_("pairs", named_list(), var("pairs")));
    assert!(built_in(&bound, &class).is_admitted());
    let shown = at_cap(Ty::Unit, publish_view(named_list()));
    assert!(built_in(&shown, &class).is_admitted());
    let shown_bound = at_cap(Ty::Unit, let_("pairs", named_list(), publish_view(var("pairs"))));
    assert!(built_in(&shown_bound, &class).is_admitted());
    // Listed as a plain read as well, the read still follows the exception's rules.
    let both = rows(&[Row::Read(REPAIR.into()), Row::RepairList(REPAIR.into())]);
    assert!(built_in(&returned, &both).is_admitted());
    assert_eq!(refusals(&at_cap(pairs(), repair_at(nat(3))), &both), not_granted_at(&["root"]));
    // Without the exception, the pairs' words are words the method derived from state (LB3.5).
    let plain = rows(&[Row::Read(REPAIR.into()), Row::View]);
    assert_eq!(refusals(&returned, &plain), not_granted_at(&["root"]));
    assert_eq!(refusals(&shown, &plain), not_granted_at(&["root"]));
}

/// CT-BND-17 (part: the checker half: the repair list runs only for a named cap, and no entry reads it for a cap the
/// method chose, the next cap included, so no entry skips to another cap (BND.2: the program does not choose the cap;
/// BND.64: no job for an unnamed cap); LP2 draft; not run: a job that names its cap through the command layer, KN5,
/// and the shell's keys, LP4).
///
/// A read with no cap, for a literal cap, for a cap the method computed, under its own name or the solver's, for a cap
/// read from state, for a record of the cap, or for a closure's own parameter is refused at the read, once. A function
/// the entry reaches that reads the list, for a literal cap too, is refused at the entry; what it returns is then a
/// word from state, which the shown-facts rule refuses.
#[test]
fn bnd_2_the_repair_list_runs_only_for_a_named_cap() {
    let class = rows(&[Row::RepairList(REPAIR.into()), Row::Read("next_cap".into())]);
    let refused_at_the_read = [
        read_snapshot(REPAIR, exact(pairs())),
        repair_at(nat(3)),
        repair_at(exact_add(var("cap"), nat(1))),
        repair_at(read_snapshot("next_cap", exact(Ty::Nat))),
        repair_at(record(vec![("cap", var("cap"))])),
    ];
    for body in refused_at_the_read {
        let shown = format!("{body:?}");
        assert_eq!(refusals(&at_cap(pairs(), body), &class), not_granted_at(&["root"]), "{shown}");
    }
    let next = let_("cap", exact_add(var("cap"), nat(1)), named_list());
    assert_eq!(refusals(&at_cap(pairs(), next), &class), not_granted_at(&["1"]));
    let own = closure(cap(), exact(pairs()), named_list());
    let in_closure = at_cap(pairs(), call_value(own, vec![var("cap")]));
    assert_eq!(refusals(&in_closure, &class), not_granted_at(&["1.0", "root"]));

    let pairs_at = || helper("pairs_at", vec![param("c", exact(Ty::Nat))], exact(pairs()), repair_at(var("c")));
    let through =
        tool(vec![pairs_at(), command("go", cap(), exact(pairs()), call("pairs_at", vec![var("cap")])).exported()]);
    assert_eq!(refusals(&through, &class), not_granted_at(&["entry", "root"]));
    let by_ref = call_value(fn_ref("pairs_at"), vec![var("cap")]);
    let referenced = tool(vec![pairs_at(), command("go", cap(), exact(pairs()), by_ref).exported()]);
    assert_eq!(refusals(&referenced, &class), not_granted_at(&["entry", "root"]));
    let third = helper("third", vec![], exact(pairs()), repair_at(nat(3)));
    let fixed = tool(vec![third, command("go", cap(), exact(pairs()), call("third", vec![])).exported()]);
    assert_eq!(refusals(&fixed, &class), not_granted_at(&["entry", "root"]));
}

/// CT-BND-17 (part: the checker half: the program does not choose the cap, also not among caps the solver named, and
/// not whether to show the list, so a branch delivers the list only when its condition reads nothing but literals and
/// the entry's parameters as given; LP2 draft; not run: a job that names its cap through the command layer, KN5).
///
/// A condition that reads state or names a `let` binding steers its branches, and the list each delivers is refused
/// there, once; a condition on the solver's own arguments chooses nothing. A branch inside a view's or a `let`'s value
/// is a use. A view of the list is refused wherever the program chooses whether it runs: under a steered branch at any
/// depth, after a steered short-circuit operand, in a closure's body and in a child scope. An entry that calls itself
/// names a cap to itself, so it is refused at the entry, also when it only publishes the list.
#[test]
fn bnd_2_the_program_does_not_choose_the_cap() {
    let class = rows(&[Row::RepairList(REPAIR.into()), Row::Read("goal_state".into()), Row::View]);
    let two = || vec![param("a", exact(Ty::Nat)), param("b", exact(Ty::Nat)), param("flag", exact(Ty::Bool))];
    let either = |result: Ty, body: Expr| tool(vec![command("go", two(), exact(result), body).exported()]);
    let state = || read_snapshot("goal_state", exact(Ty::Bool));
    let at_a = || repair_at(var("a"));
    let at_b = || repair_at(var("b"));
    assert!(built_in(&either(pairs(), if_(var("flag"), at_a(), at_b())), &class).is_admitted());
    let less = prim(Prim::ExactLess, vec![var("a"), var("b")]);
    assert!(built_in(&either(pairs(), if_(less, at_a(), at_b())), &class).is_admitted());

    let by_state = if_(state(), at_a(), at_b());
    assert_eq!(refusals(&either(pairs(), by_state), &class), not_granted_at(&["1", "2"]));
    let or_none = if_(state(), at_a(), list(exact(pair()), vec![]));
    assert_eq!(refusals(&either(pairs(), or_none), &class), not_granted_at(&["1", "2"]));
    let held = let_("l", at_a(), let_("m", at_b(), if_(state(), var("l"), var("m"))));
    assert_eq!(refusals(&either(pairs(), held), &class), not_granted_at(&["1.1.1", "1.1.2"]));
    let shown = if_(state(), publish_view(at_a()), publish_view(at_b()));
    assert_eq!(refusals(&either(Ty::Unit, shown), &class), not_granted_at(&["1.0", "2.0"]));
    let renamed = let_("f", var("flag"), if_(var("f"), at_a(), at_b()));
    assert_eq!(refusals(&either(pairs(), renamed), &class), not_granted_at(&["1.1", "1.2"]));

    let in_view = publish_view(if_(var("flag"), at_a(), at_b()));
    assert_eq!(refusals(&either(Ty::Unit, in_view), &class), not_granted_at(&["0.1", "0.2", "root"]));
    let in_let = let_("l", if_(var("flag"), at_a(), at_b()), var("l"));
    assert_eq!(refusals(&either(pairs(), in_let), &class), not_granted_at(&["0.1", "0.2", "1"]));

    // Whether a view runs is the program's choice under a branch that state steers, wherever the branch is, after a
    // short-circuit operand that state steers, in a closure's body, which runs when its caller chooses, and in a child
    // scope, whose budget chooses whether its body finishes or its stop branch runs.
    let view_either = |cond: Expr| let_("v", if_(cond, publish_view(at_a()), publish_view(at_b())), var("v"));
    assert_eq!(refusals(&either(Ty::Unit, view_either(state())), &class), not_granted_at(&["0.1.0", "0.2.0"]));
    assert!(built_in(&either(Ty::Unit, view_either(var("flag"))), &class).is_admitted());
    let view_after = |op: Prim, first: Expr| {
        let shown = let_("u", publish_view(at_a()), Expr::Lit(Lit::Bool(true)));
        let_("t", prim(op, vec![first, shown]), unit())
    };
    assert_eq!(refusals(&either(Ty::Unit, view_after(Prim::And, state())), &class), not_granted_at(&["0.1.0.0"]));
    assert_eq!(refusals(&either(Ty::Unit, view_after(Prim::Or, state())), &class), not_granted_at(&["0.1.0.0"]));
    assert!(built_in(&either(Ty::Unit, view_after(Prim::And, var("flag"))), &class).is_admitted());
    let nested = |first: Expr| {
        let shown = let_("u", publish_view(at_a()), Expr::Lit(Lit::Bool(true)));
        let_("t", prim(Prim::And, vec![var("flag"), prim(Prim::Or, vec![first, shown])]), unit())
    };
    assert_eq!(refusals(&either(Ty::Unit, nested(state())), &class), not_granted_at(&["0.1.1.0.0"]));
    assert!(built_in(&either(Ty::Unit, nested(var("flag"))), &class).is_admitted());
    // A draw from a generator the entry takes reads the stream, which the solver does not give, so it steers. The
    // column refuses the draw itself too (no class row grants randomness); the delivery's refusal is at "1".
    let drawn = vec![param("a", exact(Ty::Nat)), param("g", exact(Ty::Generator))];
    let by_draw = if_(prim(Prim::ExactLess, vec![rng_draw(var("g")), nat(5)]), at_a(), list(exact(pair()), vec![]));
    let by_draw = tool(vec![command("go", drawn, exact(pairs()), by_draw).exported()]);
    let mut expected = vec![(EFFECT_UNDECLARED, "root".to_string())];
    expected.extend(not_granted_at(&["entry", "1", "2"]));
    assert_eq!(refusals(&by_draw, &class), expected);
    let later = let_("f", closure(vec![], exact(Ty::Unit), publish_view(at_a())), call_value(var("f"), vec![]));
    assert_eq!(refusals(&either(Ty::Unit, later), &class), not_granted_at(&["0.0.0"]));
    let caps = ChildCaps { steps: Some(100), kernel_cost: None, memory: None, output: None };
    let in_child = child(caps, exact(Ty::Unit), publish_view(at_a()), None, publish_view(at_b()));
    assert_eq!(refusals(&either(Ty::Unit, in_child), &class), not_granted_at(&["0.0", "1.0"]));

    let below = prim(Prim::ExactLess, vec![var("cap"), nat(3)]);
    let deeper = if_(below, call("go", vec![exact_add(var("cap"), nat(1))]), publish_view(named_list()));
    // Refused twice at the entry: the call publishes a word from state, and it reads the list (LB3.5, BND.2).
    assert_eq!(refusals(&at_cap(Ty::Unit, deeper), &class), not_granted_at(&["entry", "entry"]));
}

/// CT-BND-17 (part: the checker half: a loop's count chooses whether, and how often, its body runs (DET.55), so a view
/// of the list in a loop's body is refused unless the count reads nothing but literals and the entry's parameters as
/// given; LP2 draft).
///
/// The loop's value so far shadows a parameter of the same name, so a read for it is for a cap the method computed.
/// The loop's value is no delivery: a loop that holds the list uses it.
#[test]
fn bnd_2_a_loop_count_chooses_whether_its_body_runs() {
    let class = rows(&[Row::RepairList(REPAIR.into()), Row::Read("rounds".into()), Row::View]);
    let two = || vec![param("a", exact(Ty::Nat)), param("n", exact(Ty::Nat))];
    let go = |result: Ty, body: Expr| tool(vec![command("go", two(), exact(result), body).exported()]);
    let rounds = || read_snapshot("rounds", exact(Ty::Nat));
    let shown = || let_("v", publish_view(repair_at(var("a"))), var("u"));
    let looped = |count: Expr| go(Ty::Unit, repeat(count, unit(), "u", exact(Ty::Unit), shown()));
    assert_eq!(refusals(&looped(rounds()), &class), not_granted_at(&["2.0.0"]));
    assert_eq!(refusals(&looped(exact_add(var("n"), rounds())), &class), not_granted_at(&["2.0.0"]));
    for count in [nat(2), var("n"), exact_add(var("n"), nat(1))] {
        let at = format!("{count:?}");
        assert!(built_in(&looped(count), &class).is_admitted(), "{at}");
    }
    // A count that names a `let` binding, or an outer loop's value so far, steers; a count is read before its loop
    // binds the value so far, so it reads the solver's parameter of the same name.
    let bound = let_("k", var("n"), repeat(var("k"), unit(), "u", exact(Ty::Unit), shown()));
    assert_eq!(refusals(&go(Ty::Unit, bound), &class), not_granted_at(&["1.2.0.0"]));
    let inner = repeat(var("u"), unit(), "z", exact(Ty::Unit), let_("v", publish_view(repair_at(var("a"))), var("z")));
    let outer = repeat(nat(2), nat(1), "u", exact(Ty::Nat), let_("w", inner, var("u")));
    assert_eq!(refusals(&go(Ty::Nat, outer), &class), not_granted_at(&["2.0.2.0.0"]));
    let own = repeat(var("n"), unit(), "n", exact(Ty::Unit), let_("v", publish_view(repair_at(var("a"))), var("n")));
    assert!(built_in(&go(Ty::Unit, own), &class).is_admitted());
    let shadowed = let_("v", publish_view(repair_at(var("a"))), var("a"));
    let shadowed = go(Ty::Nat, repeat(nat(1), var("a"), "a", exact(Ty::Nat), shadowed));
    assert_eq!(refusals(&shadowed, &class), not_granted_at(&["2.0.0"]));
    // Refused as a use at the initial value, and by the shown-facts rule at both of the loop's possible results, the
    // initial value and the body's last one, which hold words from state (LB3.5).
    let held = go(pairs(), repeat(nat(1), repair_at(var("a")), "l", exact(pairs()), var("l")));
    assert_eq!(refusals(&held, &class), not_granted_at(&["1", "1", "2"]));
}

/// CT-BND-17 (part: the checker half: the list keeps the engine's neutral order and marks no row best, no entry tests
/// a list for emptiness to read another cap, and the list makes no twist, since an entry only delivers the list as
/// read; LP2 draft; not run: the empty list's wording, BND.2, the library's and the display's, and the neutral order
/// itself, the kernel's).
///
/// A use of the list other than its delivery as read is refused at the use: a row picked by its index, a count, a test
/// for emptiness that reads the next cap, a record, a list or a call that holds it, a workspace write, a closure's
/// result, and a proposal. A word that such a use returns is also refused by the shown-facts rule, and an entry that
/// reads the list and proposes by capability separation.
#[test]
fn bnd_2_the_repair_list_is_delivered_as_read() {
    let class = rows(&[Row::RepairList(REPAIR.into()), Row::Workspace("kept".into())]);
    let first = prim(Prim::Index, vec![named_list(), nat(0)]);
    assert_eq!(refusals(&at_cap(pair(), first), &class), not_granted_at(&["0", "root"]));
    let count = prim(Prim::Length, vec![named_list()]);
    assert_eq!(refusals(&at_cap(Ty::Nat, count), &class), not_granted_at(&["0"]));
    let empty = prim(Prim::ExactEq, vec![prim(Prim::Length, vec![named_list()]), nat(0)]);
    let skip = if_(empty, repair_at(exact_add(var("cap"), nat(1))), named_list());
    assert_eq!(refusals(&at_cap(pairs(), skip), &class), not_granted_at(&["0.0.0", "1", "2"]));

    let held = Ty::Record(vec![("pairs".into(), exact(pairs()))]);
    assert_eq!(refusals(&at_cap(held, record(vec![("pairs", named_list())])), &class), not_granted_at(&["0", "root"]));
    let listed = list(exact(pairs()), vec![named_list()]);
    let lists = Ty::List(Box::new(exact(pairs())));
    assert_eq!(refusals(&at_cap(lists, listed), &class), not_granted_at(&["0", "root"]));
    let keep = helper("keep", vec![param("l", exact(pairs()))], exact(pairs()), var("l"));
    let passed = tool(vec![keep, command("go", cap(), exact(pairs()), call("keep", vec![named_list()])).exported()]);
    assert_eq!(refusals(&passed, &class), not_granted_at(&["0", "root"]));
    let write = write_workspace("kept", exact(pairs()), named_list());
    assert_eq!(refusals(&at_cap(Ty::Unit, write), &class), not_granted_at(&["0", "root"]));
    let later = let_("f", closure(vec![], exact(pairs()), named_list()), call_value(var("f"), vec![]));
    assert_eq!(refusals(&at_cap(pairs(), later), &class), not_granted_at(&["0.0", "1"]));
    let bound_then_counted = let_("pairs", named_list(), prim(Prim::Length, vec![var("pairs")]));
    assert_eq!(refusals(&at_cap(Ty::Nat, bound_then_counted), &class), not_granted_at(&["1.0"]));

    let twists = rows(&[Row::RepairList(REPAIR.into()), Row::Preview(PREVIEW.into())]);
    let y = field(prim(Prim::Index, vec![named_list(), nat(0)]), "y");
    let proposes = at_cap(Ty::Receipt, propose(y, target(PREVIEW)));
    assert_eq!(refusals(&proposes, &twists), not_granted_at(&["entry", "0.0.0", "root"]));
}

/// CT-BND-17 (part: the exception covers its own query only; LP2 draft): a class's exception row covers a read of its
/// query and no other row, and the shown-facts rule still refuses a word read from another row beside the list. A class
/// that lists one query as both exceptions is refused. A class without exception rows keeps the column's rules
/// unchanged (`built_in.rs`).
#[test]
fn bnd_69_an_exception_covers_its_own_query_only() {
    let class = rows(&[Row::RepairList(REPAIR.into()), Row::Read("best_word".into())]);
    let other = at_cap(Ty::Nat, read_snapshot_with("protection", var("cap"), exact(Ty::Nat)));
    assert_eq!(refusals(&other, &class), not_granted_at(&["entry"]));
    let best = at_cap(Ty::Word, let_("pairs", named_list(), read_snapshot("best_word", exact(Ty::Word))));
    assert_eq!(refusals(&best, &class), not_granted_at(&["1"]));
    let search = at_cap(setups(), setups_for(var("cap")));
    assert_eq!(refusals(&search, &class), not_granted_at(&["entry", "root"]));
    let doubled = rows(&[Row::RepairList(REPAIR.into()), Row::SetupSearch(REPAIR.into())]);
    assert_eq!(refusals(&at_cap(pairs(), named_list()), &doubled), not_granted_at(&["entry"]));
}

/// CT-BND-18 (part: the checker half: setup search runs only for the target the solver named to the entry, its setups
/// are only delivered as read, so the solver chooses among them, and nothing previews without the solver's action: an
/// entry that reads the setups prepares no preview, and a preview is of a word the solver named, in a command the
/// solver invokes; LP2 draft; not run: that the setups keep the buffers fixed, which the engine row answers, the
/// solver's edit of a setup, LP4's, and the setup-search library entry, LP3's).
#[test]
fn bnd_1_setup_search_runs_only_for_the_target_the_solver_named() {
    let class = rows(&[Row::SetupSearch(SETUPS.into()), Row::View]);
    let returned = for_goal(setups(), setups_for(var("goal")));
    let a = built_in(&returned, &class);
    assert!(a.is_admitted(), "{a:#?}");
    assert_eq!(a.separation, Separation { reads_state: true, prepares_previews: false });
    assert!(built_in(&for_goal(Ty::Unit, publish_view(setups_for(var("goal")))), &class).is_admitted());

    let literal = setups_for(list(exact(Ty::Nat), vec![nat(1)]));
    let grown = setups_for(prim(Prim::Push, vec![var("goal"), nat(1)]));
    for body in [literal, grown, read_snapshot(SETUPS, exact(setups()))] {
        let shown = format!("{body:?}");
        assert_eq!(refusals(&for_goal(setups(), body), &class), not_granted_at(&["root"]), "{shown}");
    }
    let goal = || vec![param("g", exact(Ty::List(Box::new(exact(Ty::Nat)))))];
    let search = helper("search", goal(), exact(setups()), setups_for(var("g")));
    let entry = command("go", goal(), exact(setups()), call("search", vec![var("g")])).exported();
    assert_eq!(refusals(&tool(vec![search, entry]), &class), not_granted_at(&["entry", "root"]));

    // The solver chooses: a setup picked by its index is refused at the read.
    let first = || prim(Prim::Index, vec![setups_for(var("goal")), nat(0)]);
    assert_eq!(refusals(&for_goal(Ty::Word, first()), &class), not_granted_at(&["0", "root"]));
    // Nothing previews without the solver's action: an entry that reads the setups prepares no preview ...
    let previews = rows(&[Row::SetupSearch(SETUPS.into()), Row::Preview(PREVIEW.into())]);
    let proposes = for_goal(Ty::Receipt, propose(first(), target(PREVIEW)));
    assert_eq!(refusals(&proposes, &previews), not_granted_at(&["entry", "0.0", "root"]));
    // ... and the solver previews a setup, edited or not, by naming it to a command.
    let named = command(
        "go",
        vec![param("setup", exact(Ty::Word))],
        exact(Ty::Receipt),
        propose(var("setup"), target(PREVIEW)),
    );
    assert!(built_in(&tool(vec![named.exported()]), &rows(&[Row::Preview(PREVIEW.into())])).is_admitted());
}
