//! Stage modules in source (LG5.12, RPI.68; LP2 draft until the IR freeze, PHS.49). A stage module is a `.6cs` file of
//! the same syntax family whose header names its kind, its declared cost and its array inputs instead of a capability
//! list: `stage vertex lens ops 16 overdraw 1 inputs [knots: 4];`. It lowers to a module of kind stage, and the stage
//! operations (`approx_sub` to `approx_log2`, `exclude`) lower only there. The stage checker's rules are tested in
//! `c600-check`'s `stages.rs`.

use std::collections::BTreeMap;

use c600_check::{admit, admit_stage, check, Grant, Policy, Profile, StageLimits, STAGE_RULE_VIOLATION};
use c600_ir::{Expr as Ir, ModuleKind, Prim, StageDecl, StageKind, StageOp};
use c600_syntax::{format, lower, parse, sha256_hex, Lowered};

/// A vertex stage in canonical layout. With every weight 1 it counts 16 operations: `scale` 1, `exclude` 3, the loop
/// 3 × 2, the `display_if` 3, `approx_sqrt(approx_abs(t))` 2 and the last `index` 1.
const LENS: &str = concat!(
    "edition \"6cs-draft-1\";\n",
    "stage vertex lens ops 16 overdraw 1 inputs [knots: 4];\n",
    "\n",
    "fn scale(v: Approx<P24>) -> Approx<P24> effects [] {\n",
    "    return approx_mul(v, ~24\"0.5\");\n",
    "}\n",
    "\n",
    "export fn main(x: Approx<P24>, t: Approx<P24>, knots: List<Approx<P24>>) -> record { x: Approx<P24>, y: ",
    "Approx<P24>, z: Approx<P24> } effects [] {\n",
    "    let s = scale(x);\n",
    "    let c = exclude(approx_sub(x, index(knots, 1)), ~24\"0.25\");\n",
    "    let r = repeat(3, s, fn(acc: Approx<P24>) -> Approx<P24> {\n",
    "        return approx_add(acc, index(knots, 2));\n",
    "    });\n",
    "    return record { x = display_if(approx_less(t, ~24\"1\"), r, approx_neg(r)), y = approx_sqrt(approx_abs(t)), ",
    "z = index(knots, 3) };\n",
    "}\n",
);

/// A fragment stage with no array input and two exports: an entry and a helper.
const TINT: &str = concat!(
    "edition \"6cs-draft-1\";\n",
    "stage fragment tint ops 2 overdraw 2;\n",
    "\n",
    "export fn half(v: Approx<P24>) -> Approx<P24> effects [] {\n",
    "    return approx_div(v, ~24\"2\");\n",
    "}\n",
    "\n",
    "export fn main(u: Approx<P24>) -> record { b: Approx<P24>, g: Approx<P24>, r: Approx<P24> } effects [] {\n",
    "    return record { b = half(u), g = approx_cos(u), r = ~24\"0\" };\n",
    "}\n",
);

/// Every weight 1 and every tier enabled.
fn limits() -> StageLimits {
    let mut weights: BTreeMap<String, u64> = StageOp::ALL.iter().map(|op| (op.name().to_string(), 1)).collect();
    for name in ["approx_add", "approx_less", "approx_and", "approx_not", "display_if", "index"] {
        weights.insert(name.into(), 1);
    }
    StageLimits { tier: 3, weights, max_ops: 64, max_overdraw: 4, max_loop_count: 8, max_loop_depth: 2 }
}

fn lowered(sources: &[&str]) -> Lowered {
    let files: Vec<_> = sources
        .iter()
        .map(|s| {
            let file = parse(s.as_bytes());
            assert!(file.diagnostics.is_empty(), "{s}\n{:#?}", file.diagnostics);
            file
        })
        .collect();
    lower(&files).unwrap_or_else(|e| panic!("{sources:?}: {e}"))
}

/// The stage operations of a lowered module's functions, by name.
fn stage_ops(lowered: &Lowered, module: usize) -> Vec<StageOp> {
    let mut out = Vec::new();
    let mut pending: Vec<&Ir> = lowered.program.modules[module].functions.iter().map(|f| &f.body).collect();
    while let Some(e) = pending.pop() {
        if let Ir::Prim { op: Prim::Stage(s), .. } = e {
            out.push(*s);
        }
        pending.extend(e.children());
    }
    out.sort_by_key(|s| s.name());
    out
}

/// A tool module `tool` that imports `source` as `s` and whose exported `go() -> Unit` returns `result`.
fn tool(source: &str, statements: &str) -> String {
    format!(
        "edition \"6cs-draft-1\";\nmodule tool requires [];\nimport s \"{}\";\n\n\
         export fn go() -> Unit effects [] {{\n    {statements}\n}}\n",
        sha256_hex(source.as_bytes())
    )
}

/// CT-LG5-08 (part: a `.6cs` module of kind stage parses and formats in the same syntax family, lowers to a module of
/// kind stage with its declared kind, costs and array inputs, and reaches the stage checker, which admits it under
/// test limits; not run: module digests, locks and grants in LP3's `c600-module`).
#[test]
fn a_stage_module_shares_the_syntax_and_lowers_to_a_stage() {
    for source in [LENS, TINT] {
        let file = parse(source.as_bytes());
        assert!(file.diagnostics.is_empty(), "{source}\n{:#?}", file.diagnostics);
        assert_eq!(format(&file).as_deref(), Some(source));
    }
    let lens = lowered(&[LENS]);
    let module = &lens.program.modules[0];
    let decl = StageDecl { kind: StageKind::Vertex, ops: 16, overdraw: 1, arrays: vec![("knots".into(), 4)] };
    assert_eq!(module.kind, ModuleKind::Stage(decl));
    assert!(module.capabilities.is_empty());
    let ops = [StageOp::Abs, StageOp::Mul, StageOp::Neg, StageOp::Sqrt, StageOp::Sub, StageOp::Exclude];
    assert_eq!(stage_ops(&lens, 0), ops);
    assert_eq!(check(&lens.program).unwrap().diagnostics, vec![]);
    let admitted = admit_stage(&lens.program, "lens", "main", &limits()).unwrap();
    assert!(admitted.diagnostics.is_empty(), "{:#?}", admitted.diagnostics);
    assert_eq!(admitted.stage.expect("a checked stage").counted_ops(), 16);

    let tint = lowered(&[TINT]);
    let decl = StageDecl { kind: StageKind::Fragment, ops: 2, overdraw: 2, arrays: vec![] };
    assert_eq!(tint.program.modules[0].kind, ModuleKind::Stage(decl));
    let admitted = admit_stage(&tint.program, "tint", "main", &limits()).unwrap();
    assert_eq!(admitted.stage.expect("a checked stage").counted_ops(), 2);
    // `half` is exported but returns no colour: it is a helper, not an entry.
    let refused = admit_stage(&tint.program, "tint", "half", &limits()).unwrap();
    assert_eq!(refused.codes(), [STAGE_RULE_VIOLATION]);
    assert_eq!(refused.diagnostics[0].path, "entry");
}

/// CT-LG5-08 (part: a stage module introduces no fourth profile: in a program with a tool module, a stage that breaks
/// a rule is refused alike under each of the three profiles' admissions, and a tool module that imports a stage
/// module by digest cannot call it; not run: as above).
#[test]
fn a_stage_in_source_reaches_the_stage_checker_under_every_profile() {
    let clocked = LENS.replace("let s = scale(x);", "let s = approx_add(scale(x), read_clock());");
    let program = lowered(&[&clocked, &tool(&clocked, "return ();")]).program;
    for profile in [Profile::Solving, Profile::ToolAuthoring, Profile::Research] {
        let a = admit(&program, "tool", "go", profile, &Grant::none(), &Policy::interim()).unwrap();
        let got: Vec<_> = a.diagnostics.iter().map(|d| (d.code, d.module.as_str(), d.function.as_str())).collect();
        assert_eq!(got, [(STAGE_RULE_VIOLATION, "lens", "main")], "{profile:?}");
    }
    let clean = lowered(&[LENS, &tool(LENS, "return ();")]).program;
    for profile in [Profile::Solving, Profile::ToolAuthoring, Profile::Research] {
        let a = admit(&clean, "tool", "go", profile, &Grant::none(), &Policy::interim()).unwrap();
        assert!(a.diagnostics.is_empty(), "{profile:?}: {:#?}", a.diagnostics);
    }
    // A program calls no stage function, even an exported helper of a stage module.
    let calls = lowered(&[TINT, &tool(TINT, "let h = s.half(~24\"1\");\n    return ();")]).program;
    let r = check(&calls).unwrap();
    let got: Vec<_> = r.diagnostics.iter().map(|d| (d.code, d.module.as_str())).collect();
    assert_eq!(got, [(STAGE_RULE_VIOLATION, "tool")]);
}

/// A stage operation lowers only in a stage module, and a stage header names a kind, `ops` and `overdraw`.
#[test]
fn a_stage_operation_or_header_outside_its_place_is_refused() {
    let plain = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n\
                 fn f() -> Approx<P24> effects [] {\n    return approx_sqrt(~24\"2\");\n}\n";
    let file = parse(plain.as_bytes());
    assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
    let refused = lower(&[file]).map(|_| ()).unwrap_err().to_string();
    assert!(refused.contains("is a stage operation"), "{refused}");
    for header in [
        "stage pixel m ops 1 overdraw 1;",
        "stage vertex m overdraw 1;",
        "stage vertex m ops 1;",
        "stage vertex m ops 1 overdraw 1 inputs [k];",
        "stage vertex m ops -1 overdraw 1;",
        "stage fragment m ops 1 overdraw 1 requires [clock];",
    ] {
        let text = format!("edition \"6cs-draft-1\";\n{header}\n");
        assert!(!parse(text.as_bytes()).diagnostics.is_empty(), "{header}");
    }
}
