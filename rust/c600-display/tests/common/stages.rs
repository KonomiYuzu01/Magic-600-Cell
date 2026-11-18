//! Stage fixtures for CT-RPI-32 and CT-RM-09: stage modules built with `c600_ir::build`, admitted by LP2's stage
//! checker (`c600_check::admit_stage`) and taken up by the display (`stage::take_up`). The limits are test values:
//! RM.38's weights and the fixed checker bounds come from measurement (RPI.79).

use std::collections::BTreeMap;

use c600_check::{admit_stage, StageAdmission, StageLimits};
use c600_display::stage::{self, Chain, CheckedStage};
use c600_ir::build::{exact, f32, f32_lit, helper, param, program, record, stage as stage_module, stage_op, var};
use c600_ir::{Expr, Module, Program, QTy, StageKind, StageOp, Ty};

/// Every weight 1 and every tier enabled, with bounds wide enough for the stages here.
pub fn stage_limits() -> StageLimits {
    let mut weights: BTreeMap<String, u64> = StageOp::ALL.iter().map(|op| (op.name().to_string(), 1)).collect();
    for name in ["approx_add", "approx_less", "approx_and", "approx_not", "display_if", "index"] {
        weights.insert(name.into(), 1);
    }
    StageLimits { tier: 3, weights, max_ops: 64, max_overdraw: 4, max_loop_count: 8, max_loop_depth: 2 }
}

pub fn zero() -> Expr {
    f32_lit("0")
}

/// A record type of binary32 fields named `names`.
pub fn record_ty(names: &[&str]) -> QTy {
    exact(Ty::Record(names.iter().map(|n| (n.to_string(), f32())).collect()))
}

/// A record of the fields `names`: `first` for the first field and zero for the others.
pub fn record_of(names: &[&str], first: Expr) -> Expr {
    record(names.iter().enumerate().map(|(i, &n)| (n, if i == 0 { first.clone() } else { zero() })).collect())
}

/// The position `{ x: x * 0.5, y: t, z: 0 }`.
pub fn position() -> Expr {
    record(vec![("x", stage_op(StageOp::Mul, vec![var("x"), f32_lit("0.5")])), ("y", var("t")), ("z", zero())])
}

/// A stage module `name` of `kind` whose exported entry `main(x, t)` returns `body` of type `result`. It declares
/// four operations and `overdraw` 1.
pub fn stage_with(name: &str, kind: StageKind, result: QTy, body: Expr) -> Module {
    let main = helper("main", vec![param("x", f32()), param("t", f32())], result, body).exported();
    stage_module(name, kind, 4, 1, vec![main])
}

/// The vertex stage `v`, whose entry returns [`position`].
pub fn vertex() -> Module {
    stage_with("v", StageKind::Vertex, record_ty(&["x", "y", "z"]), position())
}

/// A fragment stage `name` whose entry returns the opaque colour `{ r: t, g: 0, b: 0 }`.
pub fn fragment(name: &str) -> Module {
    stage_with(name, StageKind::Fragment, record_ty(&["r", "g", "b"]), record_of(&["r", "g", "b"], var("t")))
}

/// The stage checker's answer for `module.main` of `p` under [`stage_limits`].
pub fn admission(p: &Program, module: &str) -> StageAdmission {
    admit_stage(p, module, "main", &stage_limits()).unwrap_or_else(|e| panic!("{e}"))
}

/// The display's take-up of `module.main` of `p`, which the checker admits.
pub fn admitted(p: &Program, module: &str) -> CheckedStage {
    stage::take_up(&admission(p, module)).unwrap_or_else(|e| panic!("refused: {e:#?}"))
}

/// The chain of the vertex stage `v` and the fragment stage `f`, each admitted and taken up.
pub fn user_chain() -> Chain {
    let p = program(vec![vertex(), fragment("f")]);
    Chain::User(vec![admitted(&p, "v"), admitted(&p, "f")])
}
