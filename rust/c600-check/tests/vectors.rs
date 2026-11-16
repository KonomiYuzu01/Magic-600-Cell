//! LP2 plan section 5, acceptance check 2: the frozen exactness vectors `direct-and-helper-condition` (LG2.22) and
//! `display-accounting-no-exact-continuation` (LG2.20), read from `vectors.json` and lowered by hand. A hand lowering
//! is no evidence for the parser (milestone M7).

mod support;

use c600_ir::build::*;
use c600_ir::{Accounting, ExactRole, Expr, Program, Ty};
use support::json::Json;
use support::{boundary_vector, fields, less, report, tool, xy, Case};

/// Each lowering yields exactly one diagnostic, with the vector's code, value type and both fields.
fn assert_cases(programs: Vec<Program>, cases: Vec<Case>) {
    assert_eq!(programs.len(), cases.len(), "one lowering per frozen case");
    for (p, case) in programs.iter().zip(&cases) {
        let r = report(p);
        assert_eq!(r.diagnostics.len(), 1, "{}: {:#?}", case.site, r.diagnostics);
        assert_eq!(
            fields(&r.diagnostics[0]),
            (case.code.as_str(), Some(case.value_type.as_str()), case.display_control, case.qualifier_erasure),
            "{}",
            case.site
        );
    }
}

/// CT-LG2-05 (part: the frozen vector `direct-and-helper-condition`, lowered by hand).
#[test]
fn direct_and_helper_condition() {
    let lowerings = vec![
        // ordinary if on approx_less(x,y) selecting literal words w1 or w2
        tool(vec![command("pick", xy(), exact(Ty::Word), if_(less(), word(&[1]), word(&[2])))]),
        // helper returns approx_less(x,y) as ordinary Bool; exporting it does not change the refusal
        tool(vec![helper(
            "less",
            vec![param("x", exact(approx())), param("y", exact(approx()))],
            exact(Ty::Bool),
            approx_less(var("x"), var("y")),
        )
        .exported()]),
        // closure captures approx_less(x,y) and returns exact budget 4 or 8
        tool(vec![command(
            "budget",
            xy(),
            exact(Ty::Budget),
            let_(
                "c",
                less(),
                let_(
                    "f",
                    closure(vec![], exact(Ty::Budget), display_if(var("c"), nat(4), nat(8))),
                    call_value(var("f"), vec![]),
                ),
            ),
        )]),
        // direct Approx<p> operand to exact addition
        tool(vec![helper(
            "add",
            vec![param("x", exact(Ty::Nat))],
            exact(Ty::Nat),
            exact_add(to_approx(var("x")), nat(1)),
        )]),
        // display-branch-selected Approx<p> result supplied to exact addition
        tool(vec![helper(
            "add",
            xy(),
            exact(Ty::Nat),
            let_(
                "a",
                to_approx(var("x")),
                let_(
                    "b",
                    to_approx(var("y")),
                    exact_add(display_if(approx_less(var("a"), var("b")), var("a"), var("b")), nat(1)),
                ),
            ),
        )]),
        // helper with Approx<p> input erases its display qualifier at an exact scalar return
        tool(vec![helper("erase", vec![param("x", exact(approx()))], exact(Ty::Int), var("x"))]),
    ];
    assert_cases(lowerings, boundary_vector("direct-and-helper-condition"));
}

/// CT-LG2-05 (part: the frozen vector `display-accounting-no-exact-continuation`, lowered by hand).
#[test]
fn display_accounting_no_exact_continuation() {
    // The accounting observation is the same under either display form; form 2 itself is not built (LG2.21).
    let lowerings = vec![
        // exact code reads the host counter used.steps
        tool(vec![command(
            "next",
            vec![param("n", exact(Ty::Nat))],
            exact(Ty::Nat),
            exact_add(Expr::Accounting(Accounting::UsedSteps), var("n")),
        )]),
        // exact code reclaims the unused one of the granted steps
        tool(vec![command(
            "reclaim",
            vec![],
            exact(Ty::Unit),
            use_exact(ExactRole::Budget, Expr::Accounting(Accounting::UnusedSteps)),
        )]),
        // exact code branches on KN7:budget.output
        tool(vec![command(
            "branch",
            vec![],
            exact(Ty::Word),
            match_(Expr::Accounting(Accounting::Status), vec![(tag("KN7:budget.output"), word(&[1]))], word(&[2])),
        )]),
    ];
    assert_cases(lowerings, boundary_vector("display-accounting-no-exact-continuation"));
}

#[test]
fn the_vector_reader_handles_escapes_and_rejects_bad_input() {
    let v = Json::parse(r#"{"a": [true, null, -1.5e2], "b": "x\"é😀"}"#).unwrap();
    assert_eq!(v.get("a").and_then(Json::arr).map(<[Json]>::len), Some(3));
    assert_eq!(v.get("b").and_then(Json::str), Some("x\"\u{e9}\u{1f600}"));
    assert!(Json::parse("[1,]").is_err());
    assert!(Json::parse("{} x").is_err());
    assert!(Json::parse(r#""\ud83d""#).is_err());
}
