//! User stages as HLSL under `m600-stage-hlsl/1` (C9 U5a; RPI.68 to RPI.78; draft of 19 Nov 2026): the adapter's
//! binding, the generated text and the binary32 CPU reference, on stages that LP2's stage checker admitted. No device
//! runs here: the text is not compiled (U5b), and agreement with a device is never claimed.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use c600_check::{admit_stage, CheckedStage, StageLimits, STAGE_RULE_VIOLATION};
use c600_display::stage::{describe, take_up};
use c600_display::Form;
use c600_ir::build::*;
use c600_ir::{Prim, Program, QTy, StageKind, StageOp, Ty};
use c600_kernel::json::{parse, Value};
use c600_render::stage::{
    bind, evaluate, generate, parse_limits, record, Binding, Bound, Coverage, Evaluation, Input, Inputs,
};

/// Every weight 1 and every tier enabled.
fn limits() -> StageLimits {
    let mut weights: BTreeMap<String, u64> = StageOp::ALL.iter().map(|op| (op.name().to_string(), 1)).collect();
    for name in ["approx_add", "approx_less", "approx_and", "approx_not", "display_if", "index"] {
        weights.insert(name.into(), 1);
    }
    StageLimits { tier: 3, weights, max_ops: 256, max_overdraw: 8, max_loop_count: 8, max_loop_depth: 2 }
}

fn admitted_program(program: &Program, module: &str, limits: &StageLimits) -> CheckedStage {
    let admission = admit_stage(program, module, "main", limits).unwrap_or_else(|e| panic!("{e}"));
    assert!(admission.diagnostics.is_empty(), "{:#?}", admission.diagnostics);
    take_up(&admission).unwrap_or_else(|e| panic!("{e:#?}"));
    admission.stage.expect("a checked stage")
}

/// A stage module in source, admitted under `limits` with entry `main`.
fn admitted_with(source: &str, limits: &StageLimits) -> CheckedStage {
    let file = c600_syntax::parse(source.as_bytes());
    assert!(file.diagnostics.is_empty(), "{source}\n{:#?}", file.diagnostics);
    let lowered = c600_syntax::lower(std::slice::from_ref(&file)).unwrap_or_else(|e| panic!("{source}\n{e}"));
    let module = lowered.program.modules[0].name.clone();
    admitted_program(&lowered.program, &module, limits)
}

fn admitted(source: &str) -> CheckedStage {
    admitted_with(source, &limits())
}

fn hlsl(stage: &CheckedStage) -> String {
    generate(stage).unwrap_or_else(|e| panic!("{e:#?}")).hlsl
}

fn run(stage: &CheckedStage, inputs: Inputs, arrays: &[f32]) -> Evaluation {
    let binding = bind(stage).unwrap_or_else(|e| panic!("{e:#?}"));
    evaluate(stage, &binding, &inputs, arrays).unwrap_or_else(|e| panic!("{e}"))
}

fn point(value: [f32; 3]) -> Evaluation {
    Evaluation { coverage: Coverage::Point, value }
}

fn stopped(coverage: Coverage) -> Evaluation {
    Evaluation { coverage, value: [0.0; 3] }
}

/// The line index of the first line of `text` that contains `needle`.
fn line_of(text: &str, needle: &str) -> usize {
    text.lines().position(|l| l.contains(needle)).unwrap_or_else(|| panic!("no line holds `{needle}` in\n{text}"))
}

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(path)
}

/// A small fragment stage: a helper call, a `let`, a record and a lazy `display_if`.
const TINT: &str = concat!(
    "edition \"6cs-draft-1\";\n",
    "stage fragment tint ops 8 overdraw 1;\n",
    "\n",
    "fn half(v: Approx<P24>) -> Approx<P24> effects [] {\n",
    "    return approx_mul(v, ~24\"0.5\");\n",
    "}\n",
    "\n",
    "export fn main(shade: Approx<P24>, class: Approx<P24>) -> record { r: Approx<P24>, g: Approx<P24>, b: ",
    "Approx<P24> } effects [] {\n",
    "    let k = half(shade);\n",
    "    return record { r = k, g = approx_sub(~24\"1\", k), b = display_if(approx_less(class, ~24\"0.5\"), k, ",
    "~24\"0\") };\n",
    "}\n",
);

/// The generated text of [`TINT`], with its digest at `<digest>`.
const TINT_HLSL: &str = "\
// m600-stage-hlsl/1 fragment <digest>

struct m600_r0
{
    float m_r;
    float m_g;
    float m_b;
};

float m600_f0_0(float m600_a0, inout uint m600_cov)
{
    precise float m600_v0 = m600_a0 * asfloat(0x3F000000u);
    if (m600_cov == 0u && !isfinite(m600_v0)) m600_cov = 2u;
    return m600_v0;
}

m600_r0 m600_f0_1(float m600_a0, float m600_a1, inout uint m600_cov)
{
    precise float m600_v0 = m600_f0_0(m600_a0, m600_cov);
    precise float m600_v1 = m600_v0;
    precise float m600_v2 = asfloat(0x3F800000u) - m600_v1;
    if (m600_cov == 0u && !isfinite(m600_v2)) m600_cov = 2u;
    bool m600_v3 = m600_a1 < asfloat(0x3F000000u);
    precise float m600_v4;
    if (m600_v3)
    {
        m600_v4 = m600_v1;
    }
    else
    {
        m600_v4 = asfloat(0x00000000u);
    }
    m600_r0 m600_v5;
    m600_v5.m_r = m600_v1;
    m600_v5.m_g = m600_v2;
    m600_v5.m_b = m600_v4;
    return m600_v5;
}

float3 m600_fragment(float class_, float shade, float x, float y, float z, float t)
{
    uint m600_cov = 0u;
    if (m600_cov == 0u && !isfinite(shade)) m600_cov = 2u;
    if (m600_cov == 0u && !isfinite(class_)) m600_cov = 2u;
    m600_r0 m600_p = m600_f0_1(shade, class_, m600_cov);
    float3 m600_rgb = float3(0.0f, 0.0f, 0.0f);
    if (m600_cov == 0u)
    {
        m600_rgb = float3(m600_p.m_r, m600_p.m_g, m600_p.m_b);
    }
    return m600_rgb;
}
";

/// RPI.78 (part: the same checked stage and generator version give the same bytes): the golden text of a small stage,
/// byte for byte, from two runs and from two admissions of the same source; its header names the kind and
/// c600-display's digest.
#[test]
fn a_small_stage_gives_its_golden_text() {
    let stage = admitted(TINT);
    let first = generate(&stage).unwrap();
    let second = generate(&stage).unwrap();
    assert_eq!(first.hlsl, second.hlsl);
    assert_eq!(first.hlsl, hlsl(&admitted(TINT)));
    let digest = describe(&stage).digest;
    assert_eq!(first.digest, digest);
    assert_eq!(first.hlsl.lines().next().unwrap(), format!("// m600-stage-hlsl/1 fragment {digest}"));
    assert_eq!(first.hlsl.replacen(&digest, "<digest>", 1), TINT_HLSL);
    assert!(!first.hlsl.contains('\r'));
    // The colour of the CPU reference: `k = shade * 0.5`.
    let inputs = Inputs { shade: 0.75, class: 0.25, ..Inputs::default() };
    assert_eq!(run(&stage, inputs, &[]), point([0.375, 0.625, 0.375]));
    let inputs = Inputs { shade: 0.75, class: 0.75, ..Inputs::default() };
    assert_eq!(run(&stage, inputs, &[]), point([0.375, 0.625, 0.0]));
}

/// A vertex stage of every construct of the stage language: helper calls (one returning a record), records and
/// fields, `let`, `index` of two declared arrays, `exclude`, `repeat`, a lazy `display_if`, `approx_and`,
/// `approx_not`, `approx_less`, `approx_add` and every stage operation.
const EVERY: &str = concat!(
    "edition \"6cs-draft-1\";\n",
    "stage vertex every ops 64 overdraw 1 inputs [knots: 3, gains: 2];\n",
    "\n",
    "fn half(v: Approx<P24>) -> Approx<P24> effects [] {\n",
    "    return approx_mul(v, ~24\"0.5\");\n",
    "}\n",
    "\n",
    "fn pair(a: Approx<P24>, b: Approx<P24>) -> record { lo: Approx<P24>, hi: Approx<P24> } effects [] {\n",
    "    return record { lo = approx_neg(a), hi = approx_abs(b) };\n",
    "}\n",
    "\n",
    "export fn main(x: Approx<P24>, y: Approx<P24>, t: Approx<P24>, knots: List<Approx<P24>>, gains: ",
    "List<Approx<P24>>) -> record { x: Approx<P24>, y: Approx<P24>, z: Approx<P24> } effects [] {\n",
    "    let h = half(x);\n",
    "    let p = pair(h, approx_sub(y, index(gains, 1)));\n",
    "    let cut = exclude(approx_sub(x, index(knots, 2)), ~24\"0.25\");\n",
    "    let r = repeat(3, p.lo, fn(acc: Approx<P24>) -> Approx<P24> {\n",
    "        return approx_add(acc, index(knots, 0));\n",
    "    });\n",
    "    let ok = approx_and(approx_less(t, ~24\"1\"), approx_not(approx_less(y, ~24\"0\")));\n",
    "    return record { x = display_if(ok, approx_div(r, p.hi), approx_sqrt(p.hi)), y = approx_add(approx_sin(t), ",
    "approx_cos(t)), z = approx_log2(approx_exp2(h)) };\n",
    "}\n",
);

/// The CPU reference of [`EVERY`] in Rust's `f32`, written out in the stage's order.
fn every_by_hand(x: f32, y: f32, t: f32, knots: [f32; 3], gains: [f32; 2]) -> [f32; 3] {
    let h = x * 0.5;
    let (lo, hi) = (-h, (y - gains[1]).abs());
    let mut r = lo;
    for _ in 0..3 {
        r += knots[0];
    }
    let ok = t < 1.0 && y >= 0.0;
    [if ok { r / hi } else { hi.sqrt() }, t.sin() + t.cos(), h.exp2().log2()]
}

/// Every construct reaches the text in the contract's forms, and the CPU reference evaluates the same definition.
#[test]
fn every_construct_reaches_the_text_and_the_reference() {
    let stage = admitted(EVERY);
    let text = hlsl(&stage);
    for needle in [
        "StructuredBuffer<float> m600_vertex_arrays : register(t20);",
        "struct m600_r0\n{\n    float m_lo;\n    float m_hi;\n};",
        "struct m600_r1\n{\n    float m_x;\n    float m_y;\n    float m_z;\n};",
        "\nfloat m600_f0_0(float m600_a0, inout uint m600_cov)",
        "m600_r0 m600_f0_1(float m600_a0, float m600_a1, inout uint m600_cov)",
        "m600_r1 m600_f0_2(float m600_a0, float m600_a1, float m600_a2, inout uint m600_cov)",
        "= m600_vertex_arrays[4u];",
        "= m600_vertex_arrays[2u];",
        "= m600_vertex_arrays[0u];",
        "abs(m600_",
        ") <= asfloat(0x3E800000u)) m600_cov = 1u;",
        "[unroll] for (uint m600_i0 = 0u; m600_i0 < 3u; ++m600_i0)",
        "= -m600_",
        "= abs(m600_",
        "= sqrt(m600_",
        "= sin(m600_",
        "= cos(m600_",
        "= exp2(m600_",
        "= log2(m600_",
        " && m600_",
        "= !m600_",
        ".m_lo;",
        ".m_hi;",
        "uint m600_vertex(float x, float y, float z, float w, float t, out float3 position)",
        "for (uint m600_j = 0u; m600_j < 5u; ++m600_j)",
        "m600_r1 m600_p = m600_f0_2(x, y, t, m600_cov);",
    ] {
        assert!(text.contains(needle), "no `{needle}` in\n{text}");
    }
    // Every arithmetic result is followed by its coverage check.
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let arithmetic =
            [" + ", " - ", " * ", " / ", "= -", "= abs(", "= sqrt(", "= sin(", "= cos(", "= exp2(", "= log2("];
        if line.trim_start().starts_with("precise float") && arithmetic.iter().any(|a| line.contains(a)) {
            let v = line.split_whitespace().nth(2).unwrap();
            assert_eq!(lines[i + 1].trim(), format!("if (m600_cov == 0u && !isfinite({v})) m600_cov = 2u;"));
        }
    }

    let knots = [0.5, 0.5, 4.0];
    let gains = [9.0, 0.5];
    let arrays = [knots.as_slice(), gains.as_slice()].concat();
    for (x, y, t) in [(1.5, 2.0, 0.5), (1.5, 2.0, 2.0), (-3.0, 0.75, 0.25), (1.5, -1.0, 0.0)] {
        let inputs = Inputs { x, y, t, ..Inputs::default() };
        assert_eq!(run(&stage, inputs, &arrays), point(every_by_hand(x, y, t, knots, gains)), "{inputs:?}");
    }
    // `x - knots[2]` within 0.25 of zero.
    let inputs = Inputs { x: 4.125, y: 2.0, t: 0.5, ..Inputs::default() };
    assert_eq!(run(&stage, inputs, &arrays), stopped(Coverage::SingularNeighbourhood));
}

/// A program the source cannot write (a binding does not shadow another in source): a `let` that shadows another,
/// and a helper of result `Unit` whose `exclude` reaches the caller's coverage through `inout`.
fn shadowing() -> Program {
    let guard = helper(
        "guard",
        vec![param("v", f32())],
        exact(Ty::Unit),
        stage_op(StageOp::Exclude, vec![var("v"), f32_lit("0.25")]),
    );
    let body = let_(
        "a",
        stage_op(StageOp::Mul, vec![var("x"), f32_lit("2")]),
        let_(
            "a",
            prim(Prim::ApproxAdd, vec![var("a"), var("a")]),
            let_(
                "g",
                call("guard", vec![var("x")]),
                c600_ir::build::record(vec![("x", var("a")), ("y", var("x")), ("z", f32_lit("0"))]),
            ),
        ),
    );
    let position: QTy = exact(Ty::Record(["x", "y", "z"].iter().map(|n| (n.to_string(), f32())).collect()));
    let main = helper("main", vec![param("x", f32())], position, body).exported();
    program(vec![stage("shade", StageKind::Vertex, 16, 1, vec![guard, main])])
}

/// The contract's "unique names (shadowing renamed)" and a `void` helper (draft: `Unit` is erased).
#[test]
fn a_shadowing_let_gets_a_fresh_local_and_a_unit_helper_is_void() {
    let stage = admitted_program(&shadowing(), "shade", &limits());
    let text = hlsl(&stage);
    for needle in [
        "void m600_f0_0(float m600_a0, inout uint m600_cov)",
        "    if (m600_cov == 0u && abs(m600_a0) <= asfloat(0x3E800000u)) m600_cov = 1u;\n}",
        "    precise float m600_v0 = m600_a0 * asfloat(0x40000000u);",
        "    precise float m600_v1 = m600_v0;",
        "    precise float m600_v2 = m600_v1 + m600_v1;",
        "    precise float m600_v3 = m600_v2;",
        "    m600_f0_0(m600_a0, m600_cov);",
        "    m600_v4.m_x = m600_v3;",
    ] {
        assert!(text.contains(needle), "no `{needle}` in\n{text}");
    }
    assert_eq!(run(&stage, Inputs { x: 1.5, ..Inputs::default() }, &[]), point([6.0, 1.5, 0.0]));
    assert_eq!(run(&stage, Inputs { x: -0.25, ..Inputs::default() }, &[]), stopped(Coverage::SingularNeighbourhood));
}

/// A vertex stage of `x / y` and `x * x`.
const DIVIDE: &str = concat!(
    "edition \"6cs-draft-1\";\n",
    "stage vertex divide ops 8 overdraw 1;\n",
    "\n",
    "export fn main(x: Approx<P24>, y: Approx<P24>) -> record { x: Approx<P24>, y: Approx<P24>, z: Approx<P24> } ",
    "effects [] {\n",
    "    return record { x = approx_div(x, y), y = approx_mul(x, x), z = y };\n",
    "}\n",
);

/// LX-7's coverage rule in the CPU reference: a division by zero, an overflow and a nonfinite input give 2.
#[test]
fn a_nonfinite_result_or_input_gives_code_2() {
    let stage = admitted(DIVIDE);
    let at = |x: f32, y: f32| run(&stage, Inputs { x, y, ..Inputs::default() }, &[]);
    assert_eq!(at(3.0, 2.0), point([1.5, 9.0, 2.0]));
    assert_eq!(at(1.0, 0.0), stopped(Coverage::Nonfinite));
    assert_eq!(at(0.0, 0.0), stopped(Coverage::Nonfinite));
    assert_eq!(at(1e30, 1.0), stopped(Coverage::Nonfinite));
    assert_eq!(at(f32::INFINITY, 1.0), stopped(Coverage::Nonfinite));
    assert_eq!(at(1.0, f32::NAN), stopped(Coverage::Nonfinite));
    // An input the stage does not take is not checked (draft).
    let unbound = Inputs { x: 3.0, y: 2.0, w: f32::NAN, t: f32::INFINITY, ..Inputs::default() };
    assert_eq!(run(&stage, unbound, &[]), point([1.5, 9.0, 2.0]));
    let text = hlsl(&stage);
    assert!(!text.contains("isfinite(w)") && !text.contains("isfinite(t)"), "{text}");
    assert_eq!(Coverage::Point.code(), 0);
    assert_eq!(Coverage::SingularNeighbourhood.code(), 1);
    assert_eq!(Coverage::Nonfinite.code(), 2);
}

fn ordered(first: &str, second: &str) -> String {
    concat!(
        "edition \"6cs-draft-1\";\n",
        "stage vertex order ops 8 overdraw 1;\n",
        "\n",
        "export fn main(x: Approx<P24>, y: Approx<P24>) -> record { x: Approx<P24>, y: Approx<P24>, z: Approx<P24> } ",
        "effects [] {\n",
        "    let a = FIRST;\n",
        "    let b = SECOND;\n",
        "    return record { x = x, y = y, z = x };\n",
        "}\n",
    )
    .replace("FIRST", first)
    .replace("SECOND", second)
}

/// The first event wins, in the text's order: a division by zero before an `exclude` gives 2, after it 1.
#[test]
fn the_first_event_wins() {
    let divide = "approx_div(x, y)";
    let cut = "exclude(x, ~24\"0.25\")";
    let zero = Inputs::default();
    let stage = admitted(&ordered(divide, cut));
    assert_eq!(run(&stage, zero, &[]), stopped(Coverage::Nonfinite));
    let text = hlsl(&stage);
    assert!(line_of(&text, " / ") < line_of(&text, "m600_cov = 1u;"), "{text}");
    let stage = admitted(&ordered(cut, divide));
    assert_eq!(run(&stage, zero, &[]), stopped(Coverage::SingularNeighbourhood));
    let text = hlsl(&stage);
    assert!(line_of(&text, "m600_cov = 1u;") < line_of(&text, " / "), "{text}");
    // Neither event: a point.
    assert_eq!(run(&stage, Inputs { x: 1.0, y: 2.0, ..zero }, &[]), point([1.0, 2.0, 1.0]));
}

/// A vertex stage whose `x` divides only in one branch and whose `y` excludes only in the other.
const LAZY: &str = concat!(
    "edition \"6cs-draft-1\";\n",
    "stage vertex lazy ops 16 overdraw 1;\n",
    "\n",
    "fn guarded(v: Approx<P24>) -> Approx<P24> effects [] {\n",
    "    let c = exclude(v, ~24\"0.5\");\n",
    "    return v;\n",
    "}\n",
    "\n",
    "export fn main(x: Approx<P24>, y: Approx<P24>) -> record { x: Approx<P24>, y: Approx<P24>, z: Approx<P24> } ",
    "effects [] {\n",
    "    return record { x = display_if(approx_less(x, ~24\"0\"), approx_div(~24\"1\", y), x), ",
    "y = display_if(approx_less(~24\"0\", x), y, guarded(y)), z = x };\n",
    "}\n",
);

/// `display_if` is lazy: an event in the untaken branch does not count, and in the text the branch's statements stand
/// inside its `if` or `else`.
#[test]
fn an_event_in_an_untaken_branch_does_not_count() {
    let stage = admitted(LAZY);
    let at = |x: f32, y: f32| run(&stage, Inputs { x, y, ..Inputs::default() }, &[]);
    // Neither the division nor the exclusion runs.
    assert_eq!(at(1.0, 0.0), point([1.0, 0.0, 1.0]));
    assert_eq!(at(-1.0, 0.0), stopped(Coverage::Nonfinite));
    assert_eq!(at(-1.0, 4.0), point([0.25, 4.0, -1.0]));
    assert_eq!(at(-1.0, 0.25), stopped(Coverage::SingularNeighbourhood));
    assert_eq!(at(0.0, 0.0), stopped(Coverage::SingularNeighbourhood));
    let text = hlsl(&stage);
    let open = line_of(&text, "    if (m600_v");
    let divide = line_of(&text, " / ");
    let otherwise = line_of(&text, "    else");
    assert!(open < divide && divide < otherwise, "{text}");
    assert!(text.lines().nth(divide).unwrap().starts_with("        precise float"), "{text}");
}

/// `approx_and` evaluates both operands, as the interpreter does (draft: the text computes each operand as a statement
/// before `&&`), so an event in its second operand counts though the first is false.
#[test]
fn approx_and_evaluates_both_operands() {
    let strict = concat!(
        "edition \"6cs-draft-1\";\n",
        "stage vertex strict ops 16 overdraw 1;\n",
        "\n",
        "export fn main(x: Approx<P24>, y: Approx<P24>) -> record { x: Approx<P24>, y: Approx<P24>, z: Approx<P24> } ",
        "effects [] {\n",
        "    let both = approx_and(approx_less(x, ~24\"0\"), approx_less(approx_div(~24\"1\", y), ~24\"0\"));\n",
        "    return record { x = display_if(both, x, y), y = y, z = x };\n",
        "}\n",
    );
    let stage = admitted(strict);
    let at = |x: f32, y: f32| run(&stage, Inputs { x, y, ..Inputs::default() }, &[]);
    assert_eq!(at(1.0, 0.0), stopped(Coverage::Nonfinite));
    assert_eq!(at(1.0, 2.0), point([2.0, 2.0, 1.0]));
    assert_eq!(at(-1.0, -2.0), point([-1.0, -2.0, -1.0]));
    let text = hlsl(&stage);
    assert!(line_of(&text, " / ") < line_of(&text, " && "), "{text}");
}

/// The binding refusals of a stage the checker admitted, as `(parameter, path)`.
fn refusals(stage: &CheckedStage) -> Vec<(String, String)> {
    let errors = bind(stage).expect_err("a binding refusal");
    assert_eq!(generate(stage).expect_err("no text").len(), errors.len());
    errors
        .iter()
        .map(|e| {
            assert_eq!(e.code, STAGE_RULE_VIOLATION);
            assert_eq!(e.form, Form::Diagnostic);
            (e.context["parameter"].clone(), e.context["path"].clone())
        })
        .collect()
}

fn refused_at(parameter: &str, path: &str) -> Vec<(String, String)> {
    vec![(parameter.to_string(), path.to_string())]
}

/// A one-function stage `probe` of `kind` whose entry `main(params) -> result` returns `body`.
fn entry(kind: &str, header: &str, params: &str, result: &str, body: &str) -> String {
    format!(
        "edition \"6cs-draft-1\";\nstage {kind} probe ops 16 overdraw 1{header};\n\nexport fn main({params}) -> \
         {result} effects [] {{\n    return {body};\n}}\n"
    )
}

const POSITION: &str = "record { x: Approx<P24>, y: Approx<P24>, z: Approx<P24> }";
const COLOUR: &str = "record { r: Approx<P24>, g: Approx<P24>, b: Approx<P24> }";

/// `binding/1`: a parameter of another name, an input of the other kind, a scalar named like a declared array and an
/// array parameter of a function other than the entry are refused with `LG2:type.stage_rule_violation`.
#[test]
fn the_binding_refuses_what_the_host_cannot_supply() {
    let stage = admitted(&entry("vertex", "", "u: Approx<P24>", POSITION, "record { x = u, y = u, z = u }"));
    assert_eq!(refusals(&stage), refused_at("u", "entry"));
    let stage = admitted(&entry("fragment", "", "w: Approx<P24>", COLOUR, "record { r = w, g = w, b = w }"));
    assert_eq!(refusals(&stage), refused_at("w", "entry"));
    let stage = admitted(&entry(
        "fragment",
        "",
        "class_: Approx<P24>",
        COLOUR,
        "record { r = class_, g = class_, b = class_ }",
    ));
    assert_eq!(refusals(&stage), refused_at("class_", "entry"));
    // A scalar named like a declared array: the checker already refuses it (RPI.69), so it never reaches the binding,
    // whose own refusal of it stays as a guard.
    let scalar =
        entry("vertex", " inputs [k: 2]", "x: Approx<P24>, k: Approx<P24>", POSITION, "record { x = x, y = k, z = x }");
    let file = c600_syntax::parse(scalar.as_bytes());
    let lowered = c600_syntax::lower(std::slice::from_ref(&file)).unwrap();
    let admission = admit_stage(&lowered.program, "probe", "main", &limits()).unwrap();
    assert!(admission.stage.is_none());
    assert!(admission.diagnostics.iter().all(|d| d.code == STAGE_RULE_VIOLATION), "{:#?}", admission.diagnostics);

    // The checker lets a call pass a scalar to a helper's array parameter; the host binds arrays to the entry only.
    let gap = concat!(
        "edition \"6cs-draft-1\";\n",
        "stage vertex gap ops 16 overdraw 1 inputs [a: 2];\n",
        "\n",
        "export fn pick(a: List<Approx<P24>>) -> Approx<P24> effects [] {\n",
        "    return index(a, 1);\n",
        "}\n",
        "\n",
        "export fn main(x: Approx<P24>, a: List<Approx<P24>>) -> record { x: Approx<P24>, y: Approx<P24>, z: ",
        "Approx<P24> } effects [] {\n",
        "    return record { x = pick(x), y = index(a, 0), z = x };\n",
        "}\n",
    );
    assert_eq!(refusals(&admitted(gap)), refused_at("a", "binding"));
}

/// A binding that holds: inputs in parameter order, arrays in declaration order at the sum of the earlier lengths.
#[test]
fn the_binding_places_inputs_and_arrays() {
    let stage = admitted(&entry(
        "vertex",
        " inputs [p: 2, q: 3]",
        "t: Approx<P24>, q: List<Approx<P24>>, w: Approx<P24>, p: List<Approx<P24>>",
        POSITION,
        "record { x = index(q, 2), y = index(p, 1), z = approx_add(t, w) }",
    ));
    let binding: Binding = bind(&stage).unwrap();
    let bound: Vec<Bound> = binding.params.iter().map(|p| p.bound).collect();
    assert_eq!(bound, [Bound::Input(Input::T), Bound::Array(1), Bound::Input(Input::W), Bound::Array(0)]);
    let arrays: Vec<(&str, u64, u64)> = binding.arrays.iter().map(|a| (a.name.as_str(), a.offset, a.length)).collect();
    assert_eq!(arrays, [("p", 0, 2), ("q", 2, 3)]);
    assert_eq!((binding.buffer(), binding.register(), binding.total()), ("m600_vertex_arrays", "t20", 5));
    let text = hlsl(&stage);
    assert!(text.contains("= m600_vertex_arrays[4u];") && text.contains("= m600_vertex_arrays[1u];"), "{text}");
    assert!(text.contains("m600_f0_0(t, w, m600_cov);"), "{text}");
    let inputs = Inputs { t: 0.5, w: 0.25, ..Inputs::default() };
    assert_eq!(run(&stage, inputs, &[1.0, 2.0, 3.0, 4.0, 5.0]), point([5.0, 2.0, 0.75]));
    assert_eq!(run(&stage, inputs, &[1.0, 2.0, 3.0, f32::NAN, 5.0]), stopped(Coverage::Nonfinite));
    assert!(evaluate(&stage, &binding, &inputs, &[1.0; 4]).is_err());
    // A fragment stage without arrays declares no buffer.
    let stage = admitted(TINT);
    assert!(!hlsl(&stage).contains("StructuredBuffer"));
    let binding = bind(&stage).unwrap();
    assert_eq!((binding.buffer(), binding.register(), binding.total()), ("m600_fragment_arrays", "t21", 0));
}

fn gate(name: &str) -> String {
    std::fs::read_to_string(repo(&format!("renderer/acceptance/stages/{name}")))
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn gate_limits() -> StageLimits {
    parse_limits(gate("limits.json").as_bytes()).unwrap_or_else(|e| panic!("{e}"))
}

/// The renderer gate's stage pair admits under its declared limits and generates. The vertex stage is the built-in
/// projection `p = d4 * xyz / (d4 - w)` with `exclude` near the pole; the fragment stage uses tier-1 operations only.
#[test]
fn the_gate_pair_admits_and_generates() {
    let limits = gate_limits();
    assert_eq!(limits.tier, 1);
    assert_eq!(limits.weights.len(), 12);
    assert!(limits.weights.values().all(|w| *w == 1));
    assert_eq!((limits.max_ops, limits.max_overdraw, limits.max_loop_count, limits.max_loop_depth), (256, 108, 8, 2));

    let vertex = admitted_with(&gate("vertex.6cs"), &limits);
    assert_eq!(vertex.kind(), StageKind::Vertex);
    assert_eq!((vertex.counted_ops(), vertex.declaration().ops, vertex.declaration().overdraw), (9, 9, 108));
    let text = hlsl(&vertex);
    assert!(text.starts_with("// m600-stage-hlsl/1 vertex "));
    assert!(text.contains("abs(m600_v3) <= asfloat(0x39800000u)) m600_cov = 1u;"), "{text}");
    let d4 = 3.0f32;
    for (x, y, z, w) in [(1.0f32, 2.0f32, 0.5f32, 1.0f32), (-0.75, 0.125, 2.0, -2.5), (0.3, -0.7, 0.1, 0.9)] {
        let den = d4 - w;
        let expected = [d4 * x / den, d4 * y / den, d4 * z / den];
        assert_eq!(run(&vertex, Inputs { x, y, z, w, ..Inputs::default() }, &[d4]), point(expected));
    }
    let pole = Inputs { x: 1.0, y: 1.0, z: 1.0, w: d4, ..Inputs::default() };
    assert_eq!(run(&vertex, pole, &[d4]), stopped(Coverage::SingularNeighbourhood));
    let near = Inputs { w: d4 - 1.0 / 4096.0, ..pole };
    assert_eq!(run(&vertex, near, &[d4]), stopped(Coverage::SingularNeighbourhood));
    let outside = Inputs { w: d4 - 0.001, ..pole };
    assert_eq!(run(&vertex, outside, &[d4]).coverage, Coverage::Point);
    assert_eq!(run(&vertex, pole, &[f32::INFINITY]), stopped(Coverage::Nonfinite));

    let fragment = admitted_with(&gate("fragment.6cs"), &limits);
    assert_eq!(fragment.kind(), StageKind::Fragment);
    assert_eq!((fragment.counted_ops(), fragment.declaration().ops, fragment.declaration().overdraw), (6, 6, 108));
    let text = hlsl(&fragment);
    assert!(text.starts_with("// m600-stage-hlsl/1 fragment "));
    assert!(!text.contains("StructuredBuffer"));
    let (class, shade) = (0.5f32, 0.8f32);
    let warm = 0.25 + class * 0.75;
    let expected = [warm * shade, 0.5 * shade, (1.0 - warm) * shade];
    assert_eq!(run(&fragment, Inputs { class, shade, ..Inputs::default() }, &[]), point(expected));

    // A tier-2 operation is over the gate's tier.
    let tier2 =
        entry("fragment", "", "shade: Approx<P24>", COLOUR, "record { r = approx_sqrt(shade), g = shade, b = shade }");
    let file = c600_syntax::parse(tier2.as_bytes());
    let lowered = c600_syntax::lower(std::slice::from_ref(&file)).unwrap();
    let refused = admit_stage(&lowered.program, "probe", "main", &limits).unwrap();
    assert!(refused.stage.is_none() && !refused.diagnostics.is_empty());
}

/// The limits file refuses an unknown member, an unknown weight, a negative or fractional number, a missing member
/// and a note that is no string.
#[test]
fn the_limits_file_is_strict() {
    let good = gate("limits.json");
    assert!(parse_limits(good.as_bytes()).is_ok());
    let bad = [
        good.replace("\"max_ops\"", "\"max_opps\""),
        good.replace("\"approx_abs\"", "\"approx_abz\""),
        good.replace("\"max_overdraw\": 108", "\"max_overdraw\": -108"),
        good.replace("\"max_loop_depth\": 2", "\"max_loop_depth\": 2.5"),
        good.replace("\"tier\": 1,", ""),
        good.replace("\"tier\": 1,", "\"tier\": 1, \"note\": 1,").replacen("\"note\": \"", "\"other\": \"", 1),
        "[]".to_string(),
        "{".to_string(),
    ];
    for text in bad {
        assert!(parse_limits(text.as_bytes()).is_err(), "{text}");
    }
}

/// The record beside the text: the kind, digest, module, entry, declared ops, overdraw and arrays, the binding, the
/// versions and the SHA-256 of the text's bytes, in canonical JSON.
#[test]
fn the_record_names_the_text() {
    let stage = admitted_with(&gate("vertex.6cs"), &gate_limits());
    let generated = generate(&stage).unwrap();
    let text = record(&stage, &generated);
    assert!(text.ends_with('\n') && !text.contains(' '), "{text}");
    let value = parse(text.as_bytes()).unwrap();
    assert_eq!(value.canonical(), text.trim_end());
    let s = |key: &str| value.get(key).and_then(Value::as_str).unwrap().to_string();
    assert_eq!(s("contract"), "m600-stage-hlsl/1");
    assert_eq!(s("generator"), "c600-render-stage-hlsl/1");
    assert_eq!(s("kind"), "vertex");
    assert_eq!(s("module"), "chart");
    assert_eq!(s("entry"), "main");
    assert_eq!(s("digest"), describe(&stage).digest);
    assert_eq!(s("hlsl_sha256"), c600_digest::sha256_hex(generated.hlsl.as_bytes()));
    assert_eq!(value.get("ops").and_then(Value::as_i64), Some(9));
    assert_eq!(value.get("overdraw").and_then(Value::as_i64), Some(108));
    let arrays = value.get("arrays").and_then(Value::as_array).unwrap();
    assert_eq!(arrays.len(), 1);
    assert_eq!(arrays[0].canonical(), r#"{"length":1,"name":"d4","offset":0}"#);
    let binding = value.get("binding").unwrap();
    assert_eq!(binding.get("version").and_then(Value::as_str), Some("binding/1"));
    assert_eq!(binding.get("buffer").and_then(Value::as_str), Some("m600_vertex_arrays"));
    assert_eq!(binding.get("register").and_then(Value::as_str), Some("t20"));
    let params = binding.get("params").and_then(Value::as_array).unwrap();
    let params: Vec<String> = params.iter().map(Value::canonical).collect();
    let expected = [
        r#"{"input":"x","name":"x"}"#,
        r#"{"input":"y","name":"y"}"#,
        r#"{"input":"z","name":"z"}"#,
        r#"{"input":"w","name":"w"}"#,
        r#"{"array":"d4","name":"d4"}"#,
    ];
    assert_eq!(params, expected);
}

fn bin(args: &[&OsStr]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_c600-stage-hlsl")).args(args).output().expect("the bin runs")
}

fn os(text: &str) -> &OsStr {
    OsStr::new(text)
}

/// The bin writes `<out>/<name>.hlsl` and `<out>/<name>.json` for the gate pair, the same bytes as the module; a
/// refused stage exits 1 and prints each diagnostic with its code; a usage error exits 2.
#[test]
fn the_bin_writes_the_gate_pair() {
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("stage_hlsl_bin_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let stages = repo("renderer/acceptance/stages");
    let limits = stages.join("limits.json");
    for name in ["vertex", "fragment"] {
        let source = stages.join(format!("{name}.6cs"));
        let args =
            [os("--source"), source.as_os_str(), os("--limits"), limits.as_os_str(), os("--out"), out.as_os_str()];
        let run = bin(&args);
        assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
        let stage = admitted_with(&gate(&format!("{name}.6cs")), &gate_limits());
        let generated = generate(&stage).unwrap();
        assert_eq!(std::fs::read(out.join(format!("{name}.hlsl"))).unwrap(), generated.hlsl.as_bytes());
        assert_eq!(std::fs::read(out.join(format!("{name}.json"))).unwrap(), record(&stage, &generated).as_bytes());
    }

    // A binding refusal.
    let refused = out.join("refused.6cs");
    std::fs::write(&refused, entry("vertex", "", "u: Approx<P24>", POSITION, "record { x = u, y = u, z = u }"))
        .unwrap();
    let args = [os("--source"), refused.as_os_str(), os("--limits"), limits.as_os_str(), os("--out"), out.as_os_str()];
    let run = bin(&args);
    assert_eq!(run.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains("LG2:type.stage_rule_violation: ") && stderr.contains("parameter=u"), "{stderr}");
    assert!(!out.join("refused.hlsl").exists());

    // A checker refusal: a tier-2 operation under the gate's tier 1.
    let checker = out.join("checker.6cs");
    let tier2 =
        entry("fragment", "", "shade: Approx<P24>", COLOUR, "record { r = approx_sqrt(shade), g = shade, b = shade }");
    std::fs::write(&checker, tier2).unwrap();
    let args = [os("--source"), checker.as_os_str(), os("--limits"), limits.as_os_str(), os("--out"), out.as_os_str()];
    let run = bin(&args);
    assert_eq!(run.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.lines().all(|l| l.starts_with("LG")), "{stderr}");

    // A parse finding.
    let broken = out.join("broken.6cs");
    std::fs::write(&broken, "stage vertex").unwrap();
    let args = [os("--source"), broken.as_os_str(), os("--limits"), limits.as_os_str(), os("--out"), out.as_os_str()];
    assert_eq!(bin(&args).status.code(), Some(1));

    // A usage error.
    assert_eq!(bin(&[os("--source"), refused.as_os_str()]).status.code(), Some(2));
}
