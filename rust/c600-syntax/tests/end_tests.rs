//! LB3.2's end tests' verdicts in source (LP2 draft until the IR freeze, PHS.49). Each end test answers a verdict of
//! its own type, `RetainedCheckpointVerdict`, `LabelCheckpointVerdict` or `StickerTestVerdict`, read from the engine
//! row of its test and used through a `match` on its outcomes `holds`, `fails` and `unestablished` only (LG2.9). The
//! end tests' bare names stay chapter 02's goal names (LG2.10), which wait for the IR's goals. The checker refuses a
//! verdict used as a boolean, an equality of verdicts, a read of an end test's row declared `Bool` and a `_` arm that
//! takes `unestablished` together with another outcome (`LG2:type.verdict_required`), and one test's verdict where
//! another's is expected (`LG2:type.mismatch`). The run-time halves are `c600-run`'s `end_tests.rs`.

use c600_check::{check, Report, TYPE_MISMATCH, VERDICT_REQUIRED};
use c600_ir::{EndTest, Ty};
use c600_syntax::{lower, parse, Lowered, ParsedFile};

const HEADER: &str = "edition \"6cs-draft-1\";\nmodule tool requires [];\n\n";

fn complete(text: &str) -> ParsedFile {
    let file = parse(text.as_bytes());
    assert!(file.diagnostics.is_empty(), "{text}\n{:#?}", file.diagnostics);
    file
}

fn lowered(declarations: &str) -> Result<Lowered, String> {
    lower(&[complete(&format!("{HEADER}{declarations}"))]).map_err(|e| e.to_string())
}

fn report(declarations: &str) -> Report {
    let lowered = lowered(declarations).unwrap_or_else(|e| panic!("{declarations}: {e}"));
    check(&lowered.program).unwrap_or_else(|e| panic!("{declarations}: {e}"))
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

fn types(expected: &str, supplied: &str) -> Option<(String, String)> {
    Some((expected.to_string(), supplied.to_string()))
}

/// `fn f(v: LabelCheckpointVerdict) -> {result}` returning `body`.
fn over_verdict(result: &str, body: &str) -> String {
    format!("fn f(v: LabelCheckpointVerdict) -> {result} effects [] {{\n    return {body};\n}}\n")
}

/// A match on the label checkpoint's verdict `v` with `arms`, each `pattern => value;`, and a `_` arm.
fn matching(arms: &[&str]) -> String {
    let arms: String = arms.iter().map(|a| format!("        {a}\n")).collect();
    over_verdict("Nat", &format!("match v {{\n{arms}        _ => 0;\n    }}"))
}

/// LB3.2 (LP2 draft): each end test's verdict is a type of its own, named for its test, also as a list's element, a
/// record's field and a map's value.
#[test]
fn each_end_test_answers_a_verdict_type_of_its_own() {
    for test in EndTest::ALL {
        let name = test.type_name();
        let declarations = format!(
            "fn f(v: {name}, l: List<{name}>, m: Map<Nat, {name}>) -> record {{ v: {name}, l: List<{name}> }} \
             effects [] {{\n    return record {{ v = v, l = l }};\n}}\n"
        );
        let lowered = lowered(&declarations).unwrap_or_else(|e| panic!("{e}"));
        let f = &lowered.program.modules[0].functions[0];
        assert_eq!(f.params[0].ty.ty, Ty::EndTest(test));
        assert_eq!(f.params[1].ty.ty.elements()[0].ty, Ty::EndTest(test));
        assert_eq!(f.params[2].ty.ty.elements()[1].ty, Ty::EndTest(test));
        admitted(&declarations);
    }
}

/// LG2.20 and LG2.9 (LP2 draft): a verdict is no set element and no map key, alone or in a record, so no collection
/// orders or compares verdicts.
#[test]
fn a_verdict_is_no_set_element_or_map_key() {
    for declaration in [
        "fn f(s: Set<StickerTestVerdict>) -> Nat effects [] {\n    return 1;\n}\n",
        "fn f(m: Map<LabelCheckpointVerdict, Nat>) -> Nat effects [] {\n    return 1;\n}\n",
        "fn f(s: Set<record { r: RetainedCheckpointVerdict }>) -> Nat effects [] {\n    return 1;\n}\n",
        "fn f(v: LabelCheckpointVerdict) -> Nat effects [] {\n    return length(set<LabelCheckpointVerdict>(v));\n}\n",
    ] {
        let message = match lowered(declaration) {
            Ok(_) => panic!("lowered: {declaration}"),
            Err(e) => e,
        };
        assert!(message.contains("an end test's verdict"), "{declaration}: {message}");
    }
}

/// CT-LG2-09 (part: unknown cannot pass as false, for LB3.2's end tests' verdicts (LP2 draft): a verdict is never a
/// boolean, at any depth, never compared, and its `unestablished` is never taken together with another outcome; not
/// run: an off-lattice projection passing as unknown and a goal weakening its level, which wait for the IR's domain
/// families of partial projections and goals, and KN0.5's general verdict, which waits for M4).
///
/// A verdict as a condition, an operand of `and`, `or` or `not`, an argument for a `Bool` parameter, a function's
/// `Bool` result, or an element of a list of booleans is `LG2:type.verdict_required` with the boolean type expected
/// and the verdict's type supplied, so `unestablished` never reads as `fails`. So is an exact equality whose operand
/// is or holds a verdict, which would make a boolean of it with `unestablished` one value among the others.
#[test]
fn ct_lg2_09_a_verdict_is_never_a_boolean() {
    let verdict = "LabelCheckpointVerdict";
    for (result, body) in
        [("Nat", "if(v, 1, 2)"), ("Bool", "and(v, true)"), ("Bool", "or(true, v)"), ("Bool", "not(v)"), ("Bool", "v")]
    {
        assert_eq!(only(&over_verdict(result, body)), (VERDICT_REQUIRED, types("Bool", verdict)), "{body}");
    }
    let shown = over_verdict("Display<Nat>", "display_if(v, 1, 2)");
    assert_eq!(only(&shown), (VERDICT_REQUIRED, types("ApproxBool", verdict)));
    let argument = format!("fn g(b: Bool) -> Nat effects [] {{\n    return 1;\n}}\n\n{}", over_verdict("Nat", "g(v)"));
    assert_eq!(only(&argument), (VERDICT_REQUIRED, types("Bool", verdict)));
    let element = over_verdict("List<Bool>", "list<Bool>(true, v)");
    assert_eq!(only(&element), (VERDICT_REQUIRED, types("Bool", verdict)));
    let nested = over_verdict("List<Bool>", "list<LabelCheckpointVerdict>(v)");
    assert_eq!(only(&nested), (VERDICT_REQUIRED, types("List<Bool>", "List<LabelCheckpointVerdict>")));
    let field = over_verdict("record { b: Bool }", "record { b = v }");
    assert_eq!(only(&field).0, VERDICT_REQUIRED);

    for body in [
        "exact_eq(v, v)",
        "exact_eq(list<LabelCheckpointVerdict>(v), list<LabelCheckpointVerdict>())",
        "exact_eq(record { v = v }, record { v = v })",
    ] {
        assert_eq!(only(&over_verdict("Bool", body)), (VERDICT_REQUIRED, None), "{body}");
    }
}

/// CT-LG2-09 (part: a boolean never stands for a verdict, LP2 draft; not run: as above). A boolean argument for a
/// verdict parameter, a boolean result where a verdict is declared, and a join of a verdict and a boolean, in either
/// order, are `LG2:type.verdict_required`: a boolean has no `unestablished`.
#[test]
fn ct_lg2_09_a_boolean_never_stands_for_a_verdict() {
    let verdict = "LabelCheckpointVerdict";
    let g = "fn g(v: LabelCheckpointVerdict) -> Nat effects [] {\n    return 1;\n}\n\n";
    let argument = format!("{g}fn f() -> Nat effects [] {{\n    return g(true);\n}}\n");
    assert_eq!(only(&argument), (VERDICT_REQUIRED, types(verdict, "Bool")));
    let result = "fn f() -> LabelCheckpointVerdict effects [] {\n    return true;\n}\n";
    assert_eq!(only(result), (VERDICT_REQUIRED, types(verdict, "Bool")));
    for (branches, expected, supplied) in [("v, true", verdict, "Bool"), ("true, v", "Bool", verdict)] {
        let join = format!(
            "fn f(v: LabelCheckpointVerdict, c: Bool) -> Nat effects [] {{\n    let x = if(c, {branches});\n    \
             return 1;\n}}\n"
        );
        assert_eq!(only(&join), (VERDICT_REQUIRED, types(expected, supplied)), "{branches}");
    }
}

/// CT-LB3-01 (part: only an end test's own row answers its verdict, LB3.2; LP2 draft; not run here: the kernel's facts,
/// in `c600-jumble-tool/tests/lb3.rs`). A read of another row that declares a verdict type is `LG2:type.mismatch`, with
/// or without an argument, so no row's answer passes for a test's verdict; a list of verdicts from another row stays
/// the row's own declaration.
#[test]
fn ct_lb3_01_only_an_end_tests_row_answers_its_verdict() {
    for read in [
        "read_snapshot<StickerTestVerdict>(\"protection\", w)",
        "read_snapshot<LabelCheckpointVerdict>(\"protection\")",
        "read_snapshot<RetainedCheckpointVerdict>(\"label_checkpoint\", w)",
    ] {
        let program = format!("fn f(w: Word) -> Nat effects [snapshot] {{\n    let v = {read};\n    return 1;\n}}\n");
        assert_eq!(only(&program).0, TYPE_MISMATCH, "{read}");
    }
    admitted(
        "fn f(w: Word) -> Nat effects [snapshot] {\n    \
         let v = read_snapshot<List<StickerTestVerdict>>(\"sticker_tests\", w);\n    return 1;\n}\n",
    );
}

/// CT-LG2-01 (part: a match over the verdict constructors with a `_` arm admits the arms that name an outcome and takes
/// the remaining outcomes in its `_` arm; not run: matches over other algebraic types, and model, level and mode
/// indices).
/// CT-LG2-09 (part: unknown cannot pass as false: a match's `_` arm on an end test's verdict takes `unestablished`
/// alone or not at all; LP2 draft).
///
/// A `_` arm that takes `unestablished` together with `holds` or `fails`, or with both, would read unknown as one of
/// them: the match is `LG2:type.verdict_required`. Naming `unestablished` in an arm of its own, or both `holds` and
/// `fails`, is admitted, so a `_` arm may still take the two established outcomes together.
#[test]
fn ct_lg2_09_unestablished_never_falls_together_with_another_outcome() {
    for arms in [&[][..], &["tag(\"holds\") => 1;"], &["tag(\"fails\") => 1;"]] {
        let declaration = matching(arms);
        assert_eq!(only(&declaration), (VERDICT_REQUIRED, None), "{declaration}");
    }
    for arms in [
        &["tag(\"holds\") => 1;", "tag(\"fails\") => 2;"][..],
        &["tag(\"unestablished\") => 1;"],
        &["tag(\"fails\") => 1;", "tag(\"unestablished\") => 2;"],
        &["tag(\"holds\") => 1;", "tag(\"fails\") => 2;", "tag(\"unestablished\") => 3;"],
    ] {
        admitted(&matching(arms));
    }
}

/// CT-LG2-01 (part: a match over the verdict constructors with `holds`, `fails` and `unestablished` arms and no `_` arm
/// is refused at lowering with the message that a match ends with a `_` arm; not run: matches over other algebraic
/// types, and model, level and mode indices).
#[test]
fn a_verdict_match_without_a_wildcard_arm_does_not_lower() {
    let arms = "tag(\"holds\") => 1;\n        tag(\"fails\") => 2;\n        tag(\"unestablished\") => 3;\n";
    let declaration = over_verdict("Nat", &format!("match v {{\n        {arms}    }}"));
    match lowered(&declaration) {
        Err(message) => assert!(message.contains("a match ends with a `_` arm"), "{message}"),
        Ok(_) => panic!("a match over every verdict constructor lowered without a `_` arm: {declaration}"),
    }
}

/// CT-LG2-09 (part: unknown cannot pass as false: a verdict's patterns are its outcomes' tags; LP2 draft). A boolean
/// pattern is `LG2:type.verdict_required`; another status tag or another literal is `LG2:type.mismatch`, with the
/// verdict's type expected and the pattern's supplied.
#[test]
fn ct_lg2_09_a_verdicts_patterns_are_its_outcomes() {
    let verdict = "LabelCheckpointVerdict";
    let boolean = matching(&["true => 1;", "tag(\"unestablished\") => 2;"]);
    assert_eq!(only(&boolean), (VERDICT_REQUIRED, types(verdict, "Bool")));
    let status = matching(&["tag(\"open\") => 1;", "tag(\"unestablished\") => 2;"]);
    assert_eq!(only(&status), (TYPE_MISMATCH, types(verdict, "Status")));
    let number = matching(&["1 => 1;", "tag(\"unestablished\") => 2;"]);
    assert_eq!(only(&number), (TYPE_MISMATCH, types(verdict, "Nat")));
}

/// CT-LG2-09 (part: unknown cannot pass as false through a type parameter; LP2 draft). A type parameter may stand for
/// a verdict, so its match on an outcome's tag is held to the same rule; a match of a type parameter on other tags is
/// a status's and is not.
#[test]
fn ct_lg2_09_a_type_parameter_matched_on_an_outcome_is_held_to_the_rule() {
    let generic = |arms: &str| {
        format!("fn f<T>(x: T) -> Nat effects [] {{\n    return match x {{\n{arms}        _ => 0;\n    }};\n}}\n")
    };
    assert_eq!(only(&generic("        tag(\"holds\") => 1;\n")), (VERDICT_REQUIRED, None));
    admitted(&generic("        tag(\"holds\") => 1;\n        tag(\"fails\") => 2;\n"));
    admitted(&generic("        tag(\"open\") => 1;\n"));
}

/// CT-LB3-01 (part: the checker half of "separate fixtures distinguish retained checkpoint, label checkpoint and sticker
/// test" and of "an imported table without reachability stays unestablished", LB3.2; LP2 draft: each end test's row
/// answers its own verdict type, which no boolean, no other test's verdict and no status stands for, and its
/// `unestablished` is matched apart; not run: the library entries over the three rows, which are LP3's, and "a reachable
/// nonzero-ν lattice state fails the label checkpoint", which is the kernel's; the kernel half runs in
/// `c600-jumble-tool`'s `lb3.rs`).
///
/// A read of an end test's row, with or without an argument, is admitted with its own verdict type. Declared `Bool` it
/// is `LG2:type.verdict_required`; declared as another test's verdict or as a status it is `LG2:type.mismatch`, with the
/// row's verdict type expected: a retained checkpoint implies a label checkpoint as a kernel fact, not as a conversion
/// between the types. One test's verdict given for another's, and a status tag given for a verdict, are refused as
/// mismatches, so no program makes a verdict of its own.
#[test]
fn ct_lb3_01_each_end_test_row_answers_its_own_verdict() {
    for test in EndTest::ALL {
        let (row, name) = (test.row(), test.type_name());
        let other = EndTest::ALL.into_iter().find(|t| *t != test).expect("three tests").type_name();
        for arg in ["", ", x"] {
            let read = |ty: &str| {
                format!(
                    "fn f(x: Word) -> {ty} effects [snapshot] {{\n    return read_snapshot<{ty}>(\"{row}\"{arg});\n}}\n"
                )
            };
            admitted(&read(name));
            assert_eq!(only(&read("Bool")), (VERDICT_REQUIRED, types(name, "Bool")), "{row}{arg}");
            assert_eq!(only(&read(other)), (TYPE_MISMATCH, types(name, other)), "{row}{arg}");
            assert_eq!(only(&read("Status")), (TYPE_MISMATCH, types(name, "Status")), "{row}{arg}");
        }
    }
    let callee = "fn g(r: RetainedCheckpointVerdict) -> Nat effects [] {\n    return 1;\n}\n\n";
    let swapped = format!("{callee}{}", over_verdict("Nat", "g(v)"));
    assert_eq!(only(&swapped), (TYPE_MISMATCH, types("RetainedCheckpointVerdict", "LabelCheckpointVerdict")));
    let forged = format!("{callee}fn f() -> Nat effects [] {{\n    return g(tag(\"holds\"));\n}}\n");
    assert_eq!(only(&forged), (TYPE_MISMATCH, types("RetainedCheckpointVerdict", "Status")));
}
