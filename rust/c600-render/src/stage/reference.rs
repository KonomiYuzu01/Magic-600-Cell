//! The stage's CPU reference (RPI.74; `m600-stage-hlsl/1`, draft of 19 Nov 2026): the same checked definition
//! evaluated in Rust's `f32`, each operation rounded to binary32 (`+ - * /` and `f32::sqrt` correctly rounded; `sin`,
//! `cos`, `exp2` and `log2` from Rust's `f32` methods, which tier 3's open error rule does not pin), in the generated
//! text's order: left to right, postorder, with no reassociation and no fused multiply-add.
//!
//! The coverage rule is the text's: a nonfinite bound input or array element at the entry, and a nonfinite result of
//! any arithmetic operation (an overflow, a division by zero), give `nonfinite` (2); `exclude(v, r)` with
//! `abs(v) <= r` gives `singular_neighbourhood` (1). The first event wins, so evaluation stops there; the text runs on
//! and keeps the code, which gives the same answer. `display_if` evaluates its condition and the taken branch only.
//! A nonzero code gives the value `(0, 0, 0)`. Agreement with a device is recorded, never a proof (RPI.35).

use c600_check::CheckedStage;
use c600_ir::{Expr, Lit, Module, Prim, StageKind, StageOp};

use super::{callee, literal, nat, Binding, Bound, Input};

/// The host's inputs to one evaluation. A vertex stage reads `x`, `y`, `z`, `w` and `t`; a fragment stage `class`,
/// `shade`, `x`, `y`, `z` and `t`; each only where its entry takes it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Inputs {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
    pub t: f32,
    pub class: f32,
    pub shade: f32,
}

impl Inputs {
    pub fn get(&self, input: Input) -> f32 {
        match input {
            Input::X => self.x,
            Input::Y => self.y,
            Input::Z => self.z,
            Input::W => self.w,
            Input::T => self.t,
            Input::Class => self.class,
            Input::Shade => self.shade,
        }
    }
}

/// LX-7's coverage codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Coverage {
    /// 0: a finite point or colour.
    Point,
    /// 1: `exclude` met its neighbourhood.
    SingularNeighbourhood,
    /// 2: a nonfinite input, an overflow or a division by zero.
    Nonfinite,
}

impl Coverage {
    pub fn code(self) -> u32 {
        match self {
            Coverage::Point => 0,
            Coverage::SingularNeighbourhood => 1,
            Coverage::Nonfinite => 2,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Coverage::Point => "point",
            Coverage::SingularNeighbourhood => "singular_neighbourhood",
            Coverage::Nonfinite => "nonfinite",
        }
    }
}

/// One evaluation: the coverage code and the position (`x`, `y`, `z`) or colour (`r`, `g`, `b`), which is
/// `(0, 0, 0)` unless the code is 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Evaluation {
    pub coverage: Coverage,
    pub value: [f32; 3],
}

impl Evaluation {
    fn stopped(coverage: Coverage) -> Evaluation {
        Evaluation { coverage, value: [0.0; 3] }
    }
}

/// Evaluate `stage` under its `binding` (from [`super::bind`]) for one set of inputs. `arrays` is the stage's buffer:
/// the declared arrays in declaration order, [`Binding::total`] values. An error names a buffer of another length or
/// a binding that is not the stage's.
pub fn evaluate(
    stage: &CheckedStage,
    binding: &Binding,
    inputs: &Inputs,
    arrays: &[f32],
) -> Result<Evaluation, String> {
    if u64::try_from(arrays.len()).ok() != Some(binding.total()) {
        return Err(format!("the stage's buffer holds {} values, and {} were given", binding.total(), arrays.len()));
    }
    let defs = stage.definition();
    let (mi, fi) = binding.entry;
    let entry = defs.get(mi).and_then(|m| m.functions.get(fi));
    if binding.kind != stage.kind() || entry.is_none_or(|f| f.params.len() != binding.params.len()) {
        return Err("the binding is not this stage's".into());
    }
    let mut nonfinite = arrays.iter().any(|v| !v.is_finite());
    let mut args = Vec::new();
    for p in &binding.params {
        args.push(match p.bound {
            Bound::Input(input) => {
                let v = inputs.get(input);
                nonfinite |= !v.is_finite();
                V::F(v)
            }
            Bound::Array(k) => V::Array(binding.arrays[k].offset),
        });
    }
    if nonfinite {
        return Ok(Evaluation::stopped(Coverage::Nonfinite));
    }
    let names = match binding.kind {
        StageKind::Vertex => ["x", "y", "z"],
        StageKind::Fragment => ["r", "g", "b"],
    };
    match (Run { defs, arrays }).call(mi, fi, args) {
        Ok(V::Rec(fields)) => {
            let mut value = [0.0; 3];
            for (slot, name) in value.iter_mut().zip(names) {
                match fields.iter().find(|(n, _)| n == name) {
                    Some((_, V::F(v))) => *slot = *v,
                    _ => return Err(format!("the entry's value has no binary32 field `{name}`")),
                }
            }
            Ok(Evaluation { coverage: Coverage::Point, value })
        }
        Ok(_) => Err("the entry's value is no record".into()),
        Err(Stop::Event(coverage)) => Ok(Evaluation::stopped(coverage)),
        Err(Stop::Broken(why)) => Err(why),
    }
}

/// A value of the evaluation.
#[derive(Clone, Debug, PartialEq)]
enum V {
    F(f32),
    B(bool),
    Unit,
    /// Fields in written order.
    Rec(Vec<(String, V)>),
    /// A declared array input, at this offset in the buffer.
    Array(u64),
}

/// Why an evaluation stopped: a coverage event, or a form the checked definition cannot hold.
enum Stop {
    Event(Coverage),
    Broken(String),
}

type R<T> = Result<T, Stop>;

fn broken<T>(why: &str) -> R<T> {
    Err(Stop::Broken(why.into()))
}

/// An arithmetic result, under the coverage rule.
fn arith(v: f32) -> R<V> {
    if v.is_finite() {
        Ok(V::F(v))
    } else {
        Err(Stop::Event(Coverage::Nonfinite))
    }
}

struct Run<'a> {
    defs: &'a [Module],
    arrays: &'a [f32],
}

impl Run<'_> {
    fn call(&self, mi: usize, fi: usize, args: Vec<V>) -> R<V> {
        let Some(f) = self.defs.get(mi).and_then(|m| m.functions.get(fi)) else { return broken("no such function") };
        let mut env: Vec<(String, V)> = f.params.iter().map(|p| p.name.clone()).zip(args).collect();
        self.expr(mi, &f.body, &mut env)
    }

    fn f32(&self, mi: usize, e: &Expr, env: &mut Vec<(String, V)>) -> R<f32> {
        match self.expr(mi, e, env)? {
            V::F(v) => Ok(v),
            _ => broken("a binary32 operand expected"),
        }
    }

    fn expr(&self, mi: usize, e: &Expr, env: &mut Vec<(String, V)>) -> R<V> {
        match e {
            Expr::Lit(Lit::Unit) => Ok(V::Unit),
            Expr::Lit(_) => literal(e).map(V::F).map_or_else(|| broken("a literal that is no binary32 value"), Ok),
            Expr::Var(name) => match env.iter().rev().find(|(n, _)| n == name) {
                Some((_, v)) => Ok(v.clone()),
                None => broken("an unbound name"),
            },
            Expr::Let { name, value, body } => {
                let v = self.expr(mi, value, env)?;
                env.push((name.clone(), v));
                let out = self.expr(mi, body, env);
                env.pop();
                out
            }
            Expr::DisplayIf { cond, then, els } => match self.expr(mi, cond, env)? {
                V::B(true) => self.expr(mi, then, env),
                V::B(false) => self.expr(mi, els, env),
                _ => broken("a `display_if` condition that is no boolean"),
            },
            Expr::Call { callee: c, args } => {
                let Some((mj, fj)) = callee(self.defs, mi, c) else { return broken("a call of no function") };
                let mut vals = Vec::new();
                for a in args {
                    vals.push(self.expr(mi, a, env)?);
                }
                self.call(mj, fj, vals)
            }
            Expr::Prim { op, args } => self.prim(mi, *op, args, env),
            Expr::Record(fields) => {
                let mut out = Vec::new();
                for (name, e) in fields {
                    out.push((name.clone(), self.expr(mi, e, env)?));
                }
                Ok(V::Rec(out))
            }
            Expr::Field { value, field } => match self.expr(mi, value, env)? {
                V::Rec(fields) => match fields.into_iter().find(|(n, _)| n == field) {
                    Some((_, v)) => Ok(v),
                    None => broken("no such field"),
                },
                _ => broken("a field of no record"),
            },
            Expr::Repeat { count, init, acc, body, .. } => {
                let Some(n) = nat(count) else { return broken("a loop count that is no literal") };
                let mut value = self.expr(mi, init, env)?;
                for _ in 0..n {
                    env.push((acc.clone(), value));
                    let next = self.expr(mi, body, env);
                    env.pop();
                    value = next?;
                }
                Ok(value)
            }
            _ => broken("an expression outside the stage language"),
        }
    }

    fn prim(&self, mi: usize, op: Prim, args: &[Expr], env: &mut Vec<(String, V)>) -> R<V> {
        match op {
            Prim::Index => {
                let Some(Expr::Var(name)) = args.first() else { return broken("an index of no named array") };
                let Some((_, V::Array(offset))) = env.iter().rev().find(|(n, _)| n == name) else {
                    return broken("an index of no declared array");
                };
                let Some(i) = args.get(1).and_then(nat) else { return broken("an index at no literal position") };
                let at = offset.checked_add(i).and_then(|at| usize::try_from(at).ok());
                match at.and_then(|at| self.arrays.get(at)) {
                    Some(v) => Ok(V::F(*v)),
                    None => broken("an index outside the buffer"),
                }
            }
            Prim::Stage(StageOp::Exclude) => {
                let Some(e) = args.first() else { return broken("an exclusion of no value") };
                let v = self.f32(mi, e, env)?;
                let Some(r) = args.get(1).and_then(literal) else { return broken("an exclusion radius of no literal") };
                if v.abs() <= r {
                    Err(Stop::Event(Coverage::SingularNeighbourhood))
                } else {
                    Ok(V::Unit)
                }
            }
            _ => {
                let mut vals = Vec::new();
                for a in args {
                    vals.push(self.expr(mi, a, env)?);
                }
                match (op, vals.as_slice()) {
                    (Prim::ApproxAdd, [V::F(a), V::F(b)]) => arith(a + b),
                    (Prim::Stage(StageOp::Sub), [V::F(a), V::F(b)]) => arith(a - b),
                    (Prim::Stage(StageOp::Mul), [V::F(a), V::F(b)]) => arith(a * b),
                    (Prim::Stage(StageOp::Div), [V::F(a), V::F(b)]) => arith(a / b),
                    (Prim::Stage(StageOp::Neg), [V::F(a)]) => arith(-a),
                    (Prim::Stage(StageOp::Abs), [V::F(a)]) => arith(a.abs()),
                    (Prim::Stage(StageOp::Sqrt), [V::F(a)]) => arith(a.sqrt()),
                    (Prim::Stage(StageOp::Sin), [V::F(a)]) => arith(a.sin()),
                    (Prim::Stage(StageOp::Cos), [V::F(a)]) => arith(a.cos()),
                    (Prim::Stage(StageOp::Exp2), [V::F(a)]) => arith(a.exp2()),
                    (Prim::Stage(StageOp::Log2), [V::F(a)]) => arith(a.log2()),
                    (Prim::ApproxLess, [V::F(a), V::F(b)]) => Ok(V::B(a < b)),
                    (Prim::ApproxAnd, [V::B(a), V::B(b)]) => Ok(V::B(*a && *b)),
                    (Prim::ApproxNot, [V::B(a)]) => Ok(V::B(!a)),
                    _ => broken("an operation outside the stage's table"),
                }
            }
        }
    }
}
