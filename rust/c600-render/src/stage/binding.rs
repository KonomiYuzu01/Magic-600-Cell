//! The adapter's parameter binding (`binding/1`, draft; interface contract `m600-stage-hlsl/1`). The binding's records
//! go to LP3 main; nothing in c600-display changes for it.
//!
//! - The entry's parameters bind by name. A vertex stage's binary32 parameters are drawn from `x`, `y`, `z`, `w` (the
//!   4D point after the view's rotation, `Q * world` in `geometry.hlsl`'s `projectWorld`) and `t` (frame time in
//!   seconds); a fragment stage's from `class` (the colour class, `labels[slot] / 433`), `shade` (the built-in
//!   lighting factor of `psMain`), `x`, `y`, `z` (the projected 3D position) and `t`. A stage may omit any of them.
//! - The declared arrays bind in declaration order to one `StructuredBuffer<float>`, `m600_vertex_arrays` at `t20` or
//!   `m600_fragment_arrays` at `t21`, each at the sum of the earlier lengths.
//!
//! Every refusal is `LG2:type.stage_rule_violation` in form `diagnostic` (draft), naming the module, the function and
//! the parameter: a parameter of another name, a scalar named like a declared array, a parameter that is no binary32
//! scalar and no declared array, a name bound twice, and (draft) an array parameter of a reached function other than
//! the entry. The stage checker lets a call pass an operand to a callee's array parameter; an array is bound only to
//! the entry, so such a stage is refused here.

use c600_check::CheckedStage;
use c600_display::{codes, Error, Form};
use c600_ir::{StageKind, Ty};

use super::{is_array, reached};

/// One of the host's inputs to a stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Input {
    X,
    Y,
    Z,
    W,
    T,
    Class,
    Shade,
}

impl Input {
    /// The parameter name that binds to the input.
    pub fn name(self) -> &'static str {
        match self {
            Input::X => "x",
            Input::Y => "y",
            Input::Z => "z",
            Input::W => "w",
            Input::T => "t",
            Input::Class => "class",
            Input::Shade => "shade",
        }
    }

    /// The fixed entry's parameter that carries the input (`class` is a keyword of HLSL).
    pub fn hlsl(self) -> &'static str {
        match self {
            Input::Class => "class_",
            other => other.name(),
        }
    }

    /// The inputs of a stage kind, in the fixed entry's parameter order.
    pub fn of(kind: StageKind) -> &'static [Input] {
        match kind {
            StageKind::Vertex => &[Input::X, Input::Y, Input::Z, Input::W, Input::T],
            StageKind::Fragment => &[Input::Class, Input::Shade, Input::X, Input::Y, Input::Z, Input::T],
        }
    }
}

/// What an entry parameter binds to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bound {
    Input(Input),
    /// The declared array of this position in [`Binding::arrays`].
    Array(usize),
}

/// One entry parameter and its binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundParam {
    pub name: String,
    pub bound: Bound,
}

/// A declared array's place in the stage's buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArrayInput {
    pub name: String,
    /// The sum of the lengths of the arrays declared before it.
    pub offset: u64,
    pub length: u64,
}

/// The binding of a checked stage's entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub kind: StageKind,
    /// The entry's parameters, in order.
    pub params: Vec<BoundParam>,
    /// The declared arrays, in declaration order.
    pub arrays: Vec<ArrayInput>,
    /// The entry as (module, function) indices in the checked definition.
    pub(crate) entry: (usize, usize),
    /// The functions the entry reaches, callees before callers.
    pub(crate) reached: Vec<(usize, usize)>,
}

impl Binding {
    /// The structured buffer of the stage's arrays.
    pub fn buffer(&self) -> &'static str {
        match self.kind {
            StageKind::Vertex => "m600_vertex_arrays",
            StageKind::Fragment => "m600_fragment_arrays",
        }
    }

    /// The buffer's register.
    pub fn register(&self) -> &'static str {
        match self.kind {
            StageKind::Vertex => "t20",
            StageKind::Fragment => "t21",
        }
    }

    /// The buffer's length: the sum of the declared lengths.
    pub fn total(&self) -> u64 {
        self.arrays.iter().map(|a| a.length).sum()
    }
}

/// A binding refusal (draft code and form). Its path is `entry` for an entry parameter and `binding` for another.
fn refusal(module: &str, function: &str, path: &str, parameter: &str, summary: String) -> Error {
    Error::new(codes::STAGE_RULE_VIOLATION, summary)
        .in_form(Form::Diagnostic)
        .with("module", module)
        .with("function", function)
        .with("path", path)
        .with("parameter", parameter)
}

/// The binding of `stage`'s entry, or every binding refusal in parameter order, then the reached functions' array
/// parameters in emission order.
pub fn bind(stage: &CheckedStage) -> Result<Binding, Vec<Error>> {
    let defs = stage.definition();
    let kind = stage.kind();
    let module = stage.module();
    let mi = defs.iter().position(|m| m.name == module);
    let fi = mi.and_then(|mi| defs[mi].functions.iter().position(|f| f.name == stage.entry()));
    let (Some(mi), Some(fi)) = (mi, fi) else {
        let summary = format!("the checked definition holds no entry `{module}.{}`", stage.entry());
        return Err(vec![refusal(module, stage.entry(), "entry", "", summary)]);
    };
    let mut offset = 0u64;
    let arrays: Vec<ArrayInput> = stage
        .declaration()
        .arrays
        .iter()
        .map(|(name, length)| {
            let a = ArrayInput { name: name.clone(), offset, length: *length };
            offset = offset.saturating_add(*length);
            a
        })
        .collect();
    let entry = &defs[mi].functions[fi];
    let names: Vec<&str> = Input::of(kind).iter().map(|i| i.name()).collect();
    let mut errors = Vec::new();
    let mut params: Vec<BoundParam> = Vec::new();
    for p in &entry.params {
        let refuse = |summary: String| refusal(module, &entry.name, "entry", &p.name, summary);
        if params.iter().any(|b| b.name == p.name) {
            errors.push(refuse(format!("the parameter `{}` is bound twice", p.name)));
            continue;
        }
        let array = arrays.iter().position(|a| a.name == p.name);
        let bound = if is_array(&p.ty) {
            match array {
                Some(k) => Bound::Array(k),
                None => {
                    errors.push(refuse(format!("the array parameter `{}` names no declared array input", p.name)));
                    continue;
                }
            }
        } else if p.ty.ty != Ty::Approx(24) {
            errors.push(refuse(format!(
                "the parameter `{}` is no binary32 scalar and no declared array: the host supplies binary32 inputs \
                 only",
                p.name
            )));
            continue;
        } else if array.is_some() {
            errors.push(refuse(format!("the scalar parameter `{}` is named like a declared array input", p.name)));
            continue;
        } else {
            match Input::of(kind).iter().find(|i| i.name() == p.name) {
                Some(i) => Bound::Input(*i),
                None => {
                    errors.push(refuse(format!(
                        "the parameter `{}` binds to no input of a {} stage, which are {}",
                        p.name,
                        kind.name(),
                        names.join(", ")
                    )));
                    continue;
                }
            }
        };
        params.push(BoundParam { name: p.name.clone(), bound });
    }
    let order = reached(defs, (mi, fi));
    for &(mj, fj) in &order {
        if (mj, fj) == (mi, fi) {
            continue;
        }
        let f = &defs[mj].functions[fj];
        for p in f.params.iter().filter(|p| is_array(&p.ty)) {
            errors.push(refusal(
                &defs[mj].name,
                &f.name,
                "binding",
                &p.name,
                format!(
                    "`{}` takes the array parameter `{}`, and the host binds arrays to the entry only (draft)",
                    f.name, p.name
                ),
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Binding { kind, params, arrays, entry: (mi, fi), reached: order })
}
