//! The 600 Cell Script checker over the typed IR draft `c600-ir-draft-1` (LP2, milestone M1).
//!
//! The checker implements form 1 of the exactness contract (LG2.19, LG2.20, LG2.22), effect inference and the view
//! rules (LG3.1, LG3.2, LG3.4 to LG3.6), and capabilities, grants and capability separation (LG3.8, LG3.9, LG3.13)
//! with chapter 07's interim grant table. It reports chapter 02's frozen codes and no others.
//!
//! - A value whose base type does not fit its use is `LG2:type.mismatch` (LG2.1), with the expected and the supplied
//!   type in [`Mismatch`]. The check goes on with a value of unknown type, which fits every use and gives no finding of
//!   its own, so one fault gives one finding. The exactness codes apply where the base type fits (LG2.22).
//! - A fault with no source diagnostic is an [`IrError`]: a module or function defined twice, an unbound name, an
//!   unknown or unexported function, a wrong number of arguments, or effect inference that does not converge.
//!   Lowering refuses most of them before the checker sees them (`rust/README.md`).
//! - The flow rule (BND.33) stays off until LQ2 sets it, so the checker never emits `LG3:capability.flow_refused`
//!   (CT-BND-24, CT-ERR-15). [`separation`] computes how capability separation sees a program (BND.41).
//! - Static success never replaces the engine's run-time checks (LG3.10).

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use c600_ir::{
    Accounting, Callee, Capability, Effect, EffectBound, ExactRole, Expr, FnKind, FnTy, Lit, Prim, Program, QTy,
    ReceiptField, Ty, EXEC_CONTRACT, FORM2_EXEC_CONTRACT, IR_VERSION,
};

/// `LG2:type.mismatch` (LG2.1).
pub const TYPE_MISMATCH: &str = "LG2:type.mismatch";
/// `LG2:type.exactness_violation` (LG2.19, LG2.20, LG2.22).
pub const EXACTNESS_VIOLATION: &str = "LG2:type.exactness_violation";
/// `LG2:type.exact_required` (LG2.22).
pub const EXACT_REQUIRED: &str = "LG2:type.exact_required";
/// `LG3:effect.undeclared` (LG3.2, LG3.5, LG3.6).
pub const EFFECT_UNDECLARED: &str = "LG3:effect.undeclared";
/// `LG3:capability.not_granted` (LG3.8, LG3.9; chapter 07 section 8).
pub const NOT_GRANTED: &str = "LG3:capability.not_granted";
/// `LG4:input.ir_version_unsupported` (LG4.1).
pub const IR_VERSION_UNSUPPORTED: &str = "LG4:input.ir_version_unsupported";

/// Never granted in 1.0, whatever a manifest requests (LG3.8, LG3.9).
pub const NEVER_GRANTED: [Capability; 3] = [Capability::Network, Capability::Process, Capability::SessionFile];

/// One finding. An exactness finding carries the offending value's type and the two fields of the LP1 vectors; a
/// type mismatch carries the expected and the supplied type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub module: String,
    /// Empty for a finding about a module's manifest or the whole program.
    pub function: String,
    /// `root` or the dot-joined child indices of the offending expression in the function body; `manifest`, `entry`
    /// or `program` for a finding outside a body.
    pub path: String,
    /// The type in the vectors' notation (`Approx` for every precision).
    pub value_type: Option<String>,
    pub display_control: bool,
    pub qualifier_erasure: bool,
    /// The types of an `LG2:type.mismatch`; `None` for every other code.
    pub mismatch: Option<Mismatch>,
    pub note: String,
}

/// The data of an `LG2:type.mismatch` (chapter 13, `type` category): `expected` is the type the context requires and
/// `supplied` the type of the expression at the diagnostic's path. A literal pattern has no path of its own, so a
/// pattern that does not fit the scrutinee is reported at the match, with the scrutinee's type expected and the
/// pattern's supplied.
///
/// Types are base types in source notation, without the display qualifier or a function's effect bound, which a
/// base-type mismatch never involves. `_` stands for an unknown type or any precision, `..` for further record fields
/// and `A | B` for the choice an overloaded operation takes. The IR draft has no identity-tower levels, so the types
/// carry none (LG2.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mismatch {
    pub expected: String,
    pub supplied: String,
}

/// A fault that has no source diagnostic, such as an unbound name (see the crate documentation). A base-type mismatch
/// is `LG2:type.mismatch`, not an `IrError`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrError(pub String);

impl fmt::Display for IrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "malformed IR: {}", self.0)
    }
}

impl std::error::Error for IrError {}

/// The three profiles of LG3.9. `Solving` is the column of a solver's own, forked or imported tool; built-in tools
/// keep their reviewed classes and are not checked here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Solving,
    ToolAuthoring,
    Research,
}

/// The capabilities the host grants for one run (LG3.9).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Grant {
    pub capabilities: BTreeSet<Capability>,
}

impl Grant {
    pub fn none() -> Self {
        Grant::default()
    }

    pub fn of(capabilities: &[Capability]) -> Self {
        Grant { capabilities: capabilities.iter().copied().collect() }
    }
}

/// The boundary policy. LQ2 chooses it; until then [`Policy::interim`] enables no declassifier (BND.54). The flow
/// rule is no field here: it stays off until LQ2 sets it (BND.33, BND.41).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Policy {
    pub enabled_declassifiers: BTreeSet<String>,
}

impl Policy {
    pub fn interim() -> Self {
        Policy::default()
    }

    /// A policy that enables the named fixed declassifiers, for tests of the mechanism.
    pub fn enabling(ids: &[&str]) -> Self {
        Policy { enabled_declassifiers: ids.iter().map(|id| id.to_string()).collect() }
    }
}

/// A read of state-derived data.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StateRead {
    Snapshot,
    /// A preview outcome: its effect, protection verdict, end state, N or error (BND.42).
    Outcome,
    /// A call whose effect bound allows a snapshot read.
    Callable,
    /// A fixed declassifier of chapter 07 (LG3.11, BND.53).
    Declassifier(String),
}

/// What a function needs, inferred through its calls (LG3.2).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub effects: BTreeSet<Effect>,
    pub capabilities: BTreeSet<Capability>,
    /// Effect variables of callable parameters, not yet instantiated.
    pub vars: BTreeSet<String>,
    pub state_reads: BTreeSet<StateRead>,
    /// The exact goals, evidence and budgets the function supplies, with the supplied value's type. Approximate
    /// control may not steer any of them (LG2.20, CT-LG2-05).
    pub exact_sinks: BTreeSet<(ExactRole, String)>,
    /// Calls the checker could not resolve; they are neither exact nor pure (LG2.19, LG3.2).
    pub unresolved: BTreeSet<String>,
    /// The named functions, by module and function name, that evaluating this may call directly or through a
    /// function reference. Recursion control reads it (LG2.20).
    pub reaches: BTreeSet<(String, String)>,
}

impl Summary {
    fn add(&mut self, other: &Summary) {
        self.effects.extend(other.effects.iter().copied());
        self.capabilities.extend(other.capabilities.iter().copied());
        self.vars.extend(other.vars.iter().cloned());
        self.state_reads.extend(other.state_reads.iter().cloned());
        self.exact_sinks.extend(other.exact_sinks.iter().cloned());
        self.unresolved.extend(other.unresolved.iter().cloned());
        self.reaches.extend(other.reaches.iter().cloned());
    }

    /// Whether the summary has an effect, a capability requirement or an open effect variable.
    fn has_effect(&self) -> bool {
        !self.effects.is_empty() || !self.capabilities.is_empty() || !self.vars.is_empty()
    }

    fn of_bound(bound: &EffectBound) -> Summary {
        let mut s = Summary {
            effects: bound.effects.clone(),
            capabilities: bound.capabilities.clone(),
            vars: bound.vars.clone(),
            ..Summary::default()
        };
        if s.effects.contains(&Effect::ReadSnapshot) {
            s.state_reads.insert(StateRead::Callable);
        }
        s
    }

    /// The summary as a function type's effect bound.
    pub fn bound(&self) -> EffectBound {
        EffectBound { effects: self.effects.clone(), capabilities: self.capabilities.clone(), vars: self.vars.clone() }
    }

    /// Pure as a display body must be: no effect, capability, effect variable or exact sink (LG2.19, LG2.20).
    /// Unresolved calls are reported where they occur.
    pub fn is_pure(&self) -> bool {
        !self.has_effect() && self.exact_sinks.is_empty()
    }

    /// Whether the effects, capabilities and effect variables stay within a bound (LG3.2).
    pub fn within(&self, bound: &EffectBound) -> bool {
        self.effects.is_subset(&bound.effects)
            && self.capabilities.is_subset(&bound.capabilities)
            && self.vars.is_subset(&bound.vars)
    }
}

/// The checker's result for a whole program.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// In IR order (module, function, then the position in the body) and the same on every run (LG4.3).
    pub diagnostics: Vec<Diagnostic>,
    /// The inferred summary of every function, keyed by module and function name.
    pub summaries: BTreeMap<(String, String), Summary>,
}

impl Report {
    pub fn codes(&self) -> Vec<&'static str> {
        self.diagnostics.iter().map(|d| d.code).collect()
    }

    pub fn is_admitted(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn summary(&self, module: &str, function: &str) -> Option<&Summary> {
        self.summaries.get(&(module.to_string(), function.to_string()))
    }
}

/// How capability separation sees an entry (BND.39, BND.41). Refusing a program that does both is the flow rule,
/// which LQ2 sets (BND.33); under the interim table an own tool prepares no preview in a solving session at all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Separation {
    /// Reads state-derived data, preview outcomes included, other than through an enabled declassifier.
    pub reads_state: bool,
    pub prepares_previews: bool,
}

/// Classify a summary for capability separation. An unresolved call or an open effect variable counts as both.
pub fn separation(summary: &Summary, policy: &Policy) -> Separation {
    let unknown = !summary.unresolved.is_empty() || !summary.vars.is_empty();
    let reads_state = unknown
        || summary.state_reads.iter().any(|read| match read {
            StateRead::Declassifier(id) => !policy.enabled_declassifiers.contains(id),
            _ => true,
        });
    Separation { reads_state, prepares_previews: unknown || summary.effects.contains(&Effect::PreparePreview) }
}

/// The checker's report for a program and the run's refusals for one entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Admission {
    /// The checker's diagnostics, then the run's refusals at path `entry`.
    pub diagnostics: Vec<Diagnostic>,
    pub separation: Separation,
    /// The open cells of chapter 07's grant table that the entry touches. The checker fills none of them by
    /// inference, in either direction (BND.59); the caller must not read an admission as their answer.
    pub open_cells: Vec<&'static str>,
}

impl Admission {
    pub fn codes(&self) -> Vec<&'static str> {
        self.diagnostics.iter().map(|d| d.code).collect()
    }

    pub fn is_admitted(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Check a whole program.
pub fn check(program: &Program) -> Result<Report, IrError> {
    if program.ir_version != IR_VERSION || program.exec_contract != EXEC_CONTRACT {
        let note = if program.exec_contract == FORM2_EXEC_CONTRACT {
            format!(
                "form 2 ({FORM2_EXEC_CONTRACT}) stays on record and is not built; this checker advertises \
                 {IR_VERSION} with {EXEC_CONTRACT} (LG2.21, LG4.1)"
            )
        } else {
            format!(
                "the program asks for {} with {}; this checker advertises {IR_VERSION} with {EXEC_CONTRACT} (LG4.1)",
                program.ir_version, program.exec_contract
            )
        };
        let refusal = Diagnostic {
            code: IR_VERSION_UNSUPPORTED,
            module: String::new(),
            function: String::new(),
            path: "program".into(),
            value_type: None,
            display_control: false,
            qualifier_erasure: false,
            mismatch: None,
            note,
        };
        return Ok(Report { diagnostics: vec![refusal], summaries: BTreeMap::new() });
    }
    let mut checker = Checker::new(program)?;
    checker.infer()?;
    checker.emit()?;
    Ok(checker.report())
}

/// Check a program and admit one entry for a run under a profile, a grant and a policy (LG3.9, chapter 07 section 8).
pub fn admit(
    program: &Program,
    module: &str,
    function: &str,
    profile: Profile,
    grant: &Grant,
    policy: &Policy,
) -> Result<Admission, IrError> {
    let report = check(program)?;
    if report.diagnostics.iter().any(|d| d.code == IR_VERSION_UNSUPPORTED) {
        return Ok(Admission {
            diagnostics: report.diagnostics,
            separation: Separation::default(),
            open_cells: Vec::new(),
        });
    }
    let summary = report
        .summary(module, function)
        .cloned()
        .ok_or_else(|| IrError(format!("no function `{module}.{function}`")))?;
    let mut diagnostics = report.diagnostics;
    let refuse = |note: String| Diagnostic {
        code: NOT_GRANTED,
        module: module.to_string(),
        function: function.to_string(),
        path: "entry".into(),
        value_type: None,
        display_control: false,
        qualifier_erasure: false,
        mismatch: None,
        note,
    };
    for cap in &summary.capabilities {
        if NEVER_GRANTED.contains(cap) {
            diagnostics.push(refuse(format!("{cap:?} access is never granted in 1.0 (LG3.8)")));
        } else if !grant.capabilities.contains(cap) {
            diagnostics.push(refuse(format!(
                "the run grants no {cap:?} capability; a declaration or an import supplies none (LG3.9)"
            )));
        }
    }
    let separation = separation(&summary, policy);
    let mut open_cells = Vec::new();
    if profile == Profile::Solving {
        // An unresolved call or an open effect variable may prepare a preview, so it is refused as one.
        if separation.prepares_previews {
            let what = if summary.effects.contains(&Effect::PreparePreview) {
                "an own, forked or imported tool"
            } else {
                "an unresolved call or open effect variable may prepare a preview, and an own, forked or imported tool"
            };
            diagnostics.push(refuse(format!(
                "{what} prepares no preview in a solving session until the joint setting (chapter 07 section 8, \
                 BND.56; LG3.16)"
            )));
        }
        if separation.reads_state {
            // What an own tool may read in a solving session is open (BND.59).
            open_cells.push("BND.59");
        }
    }
    Ok(Admission { diagnostics, separation, open_cells })
}

/// A type in the vectors' notation.
pub fn type_name(ty: &Ty) -> String {
    match ty {
        Ty::Unit => "Unit".into(),
        Ty::Bool => "Bool".into(),
        Ty::Nat => "Nat".into(),
        Ty::Int => "Int".into(),
        Ty::Rat => "Rat".into(),
        Ty::Q5 => "Q5".into(),
        Ty::Text => "Text".into(),
        Ty::Word => "Word".into(),
        Ty::Target => "Target".into(),
        Ty::Budget => "Budget".into(),
        Ty::Status => "Status".into(),
        Ty::Colour => "Colour".into(),
        Ty::Approx(_) => "Approx".into(),
        Ty::ApproxBool => "ApproxBool".into(),
        Ty::Record(_) => "Record".into(),
        Ty::Fn(_) => "Fn".into(),
        Ty::Receipt => "Receipt".into(),
        Ty::Var(name) => name.clone(),
    }
}

/// A base type in source notation, as [`Mismatch`] writes it.
fn type_text(ty: &Ty) -> String {
    match ty {
        Ty::Approx(p) => format!("Approx<P{p}>"),
        Ty::Record(fields) if fields.is_empty() => "record {}".into(),
        Ty::Record(fields) => {
            let fields: Vec<String> = fields.iter().map(|(n, f)| format!("{n}: {}", type_text(&f.ty))).collect();
            format!("record {{ {} }}", fields.join(", "))
        }
        Ty::Fn(f) => {
            let params: Vec<String> = f.params.iter().map(|p| type_text(&p.ty)).collect();
            format!("fn({}) -> {}", params.join(", "), type_text(&f.result.ty))
        }
        other => type_name(other),
    }
}

/// The type of a value that replaces one whose base type does not fit its use. It fits every use and gets no check of
/// its own, so that one fault gives one finding. Source cannot write it: `_` is no name.
const UNKNOWN: &str = "_";

fn unknown() -> Ty {
    Ty::Var(UNKNOWN.into())
}

fn is_unknown(ty: &Ty) -> bool {
    matches!(ty, Ty::Var(name) if name == UNKNOWN)
}

/// The type with the type variable `name` replaced by `with` at any depth.
fn replace_var(ty: &Ty, name: &str, with: &Ty) -> Ty {
    let q = |q: &QTy| QTy { ty: replace_var(&q.ty, name, with), display: q.display };
    match ty {
        Ty::Var(n) if n == name => with.clone(),
        Ty::Record(fields) => Ty::Record(fields.iter().map(|(n, f)| (n.clone(), q(f))).collect()),
        Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
            params: f.params.iter().map(q).collect(),
            result: q(&f.result),
            effects: f.effects.clone(),
        })),
        other => other.clone(),
    }
}

/// The path, below a body, of the expression it returns: the rest of its innermost `let`.
fn returned(body: &Expr) -> Vec<u32> {
    let mut path = Vec::new();
    let mut e = body;
    while let Expr::Let { body, .. } = e {
        path.push(1);
        e = body;
    }
    path
}

// ---------------------------------------------------------------------------------------------------------------
// Qualifiers

/// The exactness qualifier the checker tracks for a value (LG2.19).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Qual {
    Exact,
    /// A direct approximate scalar, `Approx<p>`.
    Approx,
    /// A display condition, `ApproxBool`.
    Cond,
    /// Display-qualified; `control` when computed or selected under approximate control (LG2.20).
    Display {
        control: bool,
    },
}

impl Qual {
    fn join(self, other: Qual) -> Qual {
        match (self, other) {
            (Qual::Display { control: a }, Qual::Display { control: b }) => Qual::Display { control: a || b },
            (Qual::Display { control }, _) | (_, Qual::Display { control }) => Qual::Display { control },
            (Qual::Cond, _) | (_, Qual::Cond) => Qual::Cond,
            (Qual::Approx, _) | (_, Qual::Approx) => Qual::Approx,
            (Qual::Exact, Qual::Exact) => Qual::Exact,
        }
    }

    /// A container with an element of this qualifier is display-qualified as a whole (LG2.19).
    fn container(self) -> Qual {
        match self {
            Qual::Exact => Qual::Exact,
            Qual::Display { control } => Qual::Display { control },
            Qual::Approx | Qual::Cond => Qual::Display { control: false },
        }
    }
}

/// The qualifier of an `ApproxBool` computed from inputs of qualifier `q`.
fn cond_of(q: Qual) -> Qual {
    match q {
        Qual::Display { control } => Qual::Display { control },
        _ => Qual::Cond,
    }
}

/// The qualifier a declared type gives its values. `Approx<p>` and `ApproxBool` are display by definition.
fn declared_qual(q: &QTy) -> Qual {
    match &q.ty {
        Ty::Approx(_) => Qual::Approx,
        Ty::ApproxBool => Qual::Cond,
        _ if q.display => Qual::Display { control: false },
        Ty::Record(fields) => fields.iter().fold(Qual::Exact, |acc, (_, f)| acc.join(declared_qual(f).container())),
        _ => Qual::Exact,
    }
}

/// A slot that takes exact values only; a record slot does when all its fields do, and `Unit` carries no value. A
/// callee's type variable takes any argument and keeps its qualifier (LG2.19). A function's `own` type variable, in
/// its result or a field it stores, is exact relative to its arguments: the body may not add a qualifier there.
fn exact_slot(slot: &QTy, own: bool) -> bool {
    if slot.display {
        return false;
    }
    match &slot.ty {
        Ty::Unit | Ty::Approx(_) | Ty::ApproxBool => false,
        Ty::Var(_) => own,
        Ty::Record(fields) => fields.iter().all(|(_, f)| f.ty == Ty::Unit || exact_slot(f, own)),
        _ => true,
    }
}

/// Whether a value of declared type `value` keeps every qualifier in a slot of declared type `slot`, at any depth.
fn admits(slot: &QTy, value: &QTy, own: bool) -> bool {
    if exact_slot(slot, own) && declared_qual(value) != Qual::Exact {
        return false;
    }
    match (&slot.ty, &value.ty) {
        (Ty::Fn(s), Ty::Fn(t)) => signature_keeps(s, t, own),
        (Ty::Record(s), Ty::Record(t)) => s.iter().zip(t).all(|((_, sf), (_, tf))| admits(sf, tf, own)),
        _ => true,
    }
}

/// Whether a callable of type `t` keeps every qualifier in a slot of type `s` (LG2.19): its result is no more
/// qualified than the slot promises, and each parameter takes what the slot passes. A parameter's type variables
/// are the callable's own and take anything.
fn signature_keeps(s: &FnTy, t: &FnTy, own: bool) -> bool {
    s.params.iter().zip(&t.params).all(|(sp, tp)| admits(tp, sp, false)) && admits(&s.result, &t.result, own)
}

fn is_integer(ty: &Ty) -> bool {
    matches!(ty, Ty::Nat | Ty::Int | Ty::Budget)
}

/// The exact numbers: integers, rationals and Q(√5) values (LG1.6). Nat and Int widen to Rat and Rat to Q5, as
/// Z ⊂ Q ⊂ Q(√5); a budget is a cost and mixes with integers only (LG2.6).
fn is_exact_number(ty: &Ty) -> bool {
    is_integer(ty) || matches!(ty, Ty::Rat | Ty::Q5)
}

/// Whether a value of type `value` may enter a slot of type `slot`. An approximate value at an exact slot fits here
/// and is refused by the exactness rules, not as a type mismatch.
fn fits(slot: &Ty, value: &Ty) -> bool {
    if is_unknown(value) {
        return true;
    }
    match (slot, value) {
        (Ty::Var(_), _) => true,
        (Ty::Approx(p), Ty::Approx(q)) => p == q,
        (slot, Ty::Approx(_)) if is_exact_number(slot) => true,
        (Ty::Bool, Ty::ApproxBool) => true,
        (Ty::Int | Ty::Budget, Ty::Nat) => true,
        (Ty::Rat, Ty::Nat | Ty::Int) | (Ty::Q5, Ty::Nat | Ty::Int | Ty::Rat) => true,
        (Ty::Record(a), Ty::Record(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|((na, qa), (nb, qb))| na == nb && fits(&qa.ty, &qb.ty))
        }
        // A callable's parameter takes what the slot passes. The checker does not unify type variables (M1).
        (Ty::Fn(a), Ty::Fn(b)) => {
            a.params.len() == b.params.len()
                && a.params.iter().zip(&b.params).all(|(pa, pb)| matches!(pa.ty, Ty::Var(_)) || fits(&pb.ty, &pa.ty))
                && fits(&a.result.ty, &b.result.ty)
        }
        (a, b) => a == b,
    }
}

/// The common type of two values, such as two branches or two arguments bound to one type variable: a record widens
/// field by field, and a callable keeps either result's qualifier and either effect bound (LG2.19, LG3.2). With an
/// unknown type it is unknown.
fn widen(a: &Ty, b: &Ty) -> Option<Ty> {
    if is_unknown(a) || is_unknown(b) {
        return Some(unknown());
    }
    if a == b {
        return Some(a.clone());
    }
    match (a, b) {
        (Ty::Nat, Ty::Int) | (Ty::Int, Ty::Nat) => Some(Ty::Int),
        (Ty::Nat, Ty::Budget) | (Ty::Budget, Ty::Nat) => Some(Ty::Budget),
        (Ty::Nat | Ty::Int, Ty::Rat) | (Ty::Rat, Ty::Nat | Ty::Int) => Some(Ty::Rat),
        (Ty::Nat | Ty::Int | Ty::Rat, Ty::Q5) | (Ty::Q5, Ty::Nat | Ty::Int | Ty::Rat) => Some(Ty::Q5),
        (Ty::Record(x), Ty::Record(y)) if x.len() == y.len() => x
            .iter()
            .zip(y)
            .map(|((nx, qx), (ny, qy))| if nx == ny { Some((nx.clone(), widen_q(qx, qy)?)) } else { None })
            .collect::<Option<Vec<_>>>()
            .map(Ty::Record),
        (Ty::Fn(x), Ty::Fn(y)) if x.params == y.params => {
            let mut effects = x.effects.clone();
            effects.effects.extend(y.effects.effects.iter().copied());
            effects.capabilities.extend(y.effects.capabilities.iter().copied());
            effects.vars.extend(y.effects.vars.iter().cloned());
            let result = widen_q(&x.result, &y.result)?;
            Some(Ty::Fn(Box::new(FnTy { params: x.params.clone(), result, effects })))
        }
        _ => None,
    }
}

fn widen_q(a: &QTy, b: &QTy) -> Option<QTy> {
    Some(QTy { ty: widen(&a.ty, &b.ty)?, display: a.display || b.display })
}

/// The effect variables a type mentions at any depth.
fn effect_vars(q: &QTy, out: &mut BTreeSet<String>) {
    match &q.ty {
        Ty::Fn(f) => {
            out.extend(f.effects.vars.iter().cloned());
            for p in &f.params {
                effect_vars(p, out);
            }
            effect_vars(&f.result, out);
        }
        Ty::Record(fields) => {
            for (_, f) in fields {
                effect_vars(f, out);
            }
        }
        _ => {}
    }
}

/// The type with its effect variables renamed by `map`.
fn rename_vars(q: &QTy, map: &BTreeMap<String, String>) -> QTy {
    let ty = match &q.ty {
        Ty::Fn(f) => {
            let mut effects = f.effects.clone();
            effects.vars = f.effects.vars.iter().map(|v| map.get(v).unwrap_or(v).clone()).collect();
            Ty::Fn(Box::new(FnTy {
                params: f.params.iter().map(|p| rename_vars(p, map)).collect(),
                result: rename_vars(&f.result, map),
                effects,
            }))
        }
        Ty::Record(fields) => Ty::Record(fields.iter().map(|(n, f)| (n.clone(), rename_vars(f, map))).collect()),
        other => other.clone(),
    };
    QTy { ty, display: q.display }
}

fn lit_type(lit: &Lit) -> Ty {
    match lit {
        Lit::Unit => Ty::Unit,
        Lit::Bool(_) => Ty::Bool,
        Lit::Nat(_) => Ty::Nat,
        Lit::Int(_) => Ty::Int,
        Lit::Rat { .. } => Ty::Rat,
        Lit::Q5 { .. } => Ty::Q5,
        Lit::Approx { precision, .. } => Ty::Approx(*precision),
        Lit::Text(_) => Ty::Text,
        Lit::Word(_) => Ty::Word,
        Lit::Target(_) => Ty::Target,
        Lit::Tag(_) => Ty::Status,
        Lit::Colour(_) => Ty::Colour,
    }
}

/// One operation's own needs.
fn needs(effects: &[Effect], capabilities: &[Capability]) -> Summary {
    Summary {
        effects: effects.iter().copied().collect(),
        capabilities: capabilities.iter().copied().collect(),
        ..Summary::default()
    }
}

fn describe(s: &Summary) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.extend(s.effects.iter().map(|e| format!("{e:?}")));
    parts.extend(s.capabilities.iter().map(|c| format!("the {c:?} capability")));
    parts.extend(s.vars.iter().map(|v| format!("the effect variable {v}")));
    parts.extend(s.exact_sinks.iter().map(|(role, ty)| format!("an exact {role:?} of type {ty}")));
    if parts.is_empty() {
        "nothing".into()
    } else {
        parts.join(", ")
    }
}

fn path_text(path: &[u32]) -> String {
    if path.is_empty() {
        "root".into()
    } else {
        path.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(".")
    }
}

/// The names an expression uses but does not bind.
fn free_names<'a>(e: &'a Expr, bound: &mut Vec<&'a str>, out: &mut BTreeSet<&'a str>) {
    match e {
        Expr::Var(name) => {
            if !bound.contains(&name.as_str()) {
                out.insert(name);
            }
        }
        Expr::Let { name, value, body } => {
            free_names(value, bound, out);
            bound.push(name);
            free_names(body, bound, out);
            bound.pop();
        }
        Expr::Closure { params, body, .. } => {
            let depth = bound.len();
            bound.extend(params.iter().map(|p| p.name.as_str()));
            free_names(body, bound, out);
            bound.truncate(depth);
        }
        other => {
            for child in other.children() {
                free_names(child, bound, out);
            }
        }
    }
}

/// The typed fields an expression's workspace writes store, by record.
fn writes<'a>(e: &'a Expr, out: &mut BTreeMap<&'a str, Vec<&'a QTy>>) {
    if let Expr::WriteWorkspace { record, ty, .. } = e {
        out.entry(record.as_str()).or_default().push(ty);
    }
    for child in e.children() {
        writes(child, out);
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Values and the walk

#[derive(Clone, Debug)]
struct Val {
    ty: Ty,
    qual: Qual,
    /// What calling this value needs, when it is a callable.
    callable: Option<Summary>,
    /// A record's field values in field order, callables included, when known; otherwise the declared field types
    /// give them.
    fields: Option<Vec<Val>>,
}

impl Val {
    fn new(ty: Ty, qual: Qual) -> Val {
        Val { ty, qual, callable: None, fields: None }
    }

    fn exact(ty: Ty) -> Val {
        Val::new(ty, Qual::Exact)
    }

    /// The value that replaces one whose base type does not fit its use: of unknown type, exact, with no callable and
    /// no fields, so that it gives no finding of its own.
    fn unknown() -> Val {
        Val::exact(unknown())
    }

    /// A value known only by its declared type. A callable's needs are then its bound: whoever supplied it was
    /// checked against that bound.
    fn declared(q: &QTy) -> Val {
        if is_unknown(&q.ty) {
            return Val::unknown();
        }
        let callable = match &q.ty {
            Ty::Fn(f) => Some(Summary::of_bound(&f.effects)),
            _ => None,
        };
        Val { ty: q.ty.clone(), qual: declared_qual(q), callable, fields: None }
    }

    /// Field `i` of a record value as the record holds it, before projection's qualifier rule.
    fn field(&self, i: usize) -> Val {
        match (&self.fields, &self.ty) {
            (_, ty) if is_unknown(ty) => Val::unknown(),
            (Some(fields), _) => fields[i].clone(),
            (None, Ty::Record(declared)) => Val::declared(&declared[i].1),
            _ => unreachable!("a field of a {:?} value", self.ty),
        }
    }

    /// What display branches select is selected under approximate control at every depth, and a callable selected
    /// so is unknown (LG2.20).
    fn steered(self) -> Val {
        if is_unknown(&self.ty) {
            return self;
        }
        let fields = match &self.ty {
            Ty::Record(declared) => Some((0..declared.len()).map(|i| self.field(i).steered()).collect()),
            _ => None,
        };
        Val { ty: self.ty, qual: Qual::Display { control: true }, callable: None, fields }
    }
}

/// One of two values: the type is the common type, the qualifiers join, a callable needs what either needs, and
/// record fields merge one by one. `None` when the types have no common type; unknown with an unknown value.
fn merge(a: Val, b: Val) -> Option<Val> {
    if is_unknown(&a.ty) || is_unknown(&b.ty) {
        return Some(Val::unknown());
    }
    let ty = widen(&a.ty, &b.ty)?;
    let fields = match &ty {
        Ty::Record(x) if a.fields.is_some() || b.fields.is_some() => {
            Some((0..x.len()).map(|i| merge(a.field(i), b.field(i))).collect::<Option<Vec<_>>>()?)
        }
        _ => None,
    };
    let callable = match (a.callable, b.callable) {
        (Some(mut x), Some(y)) => {
            x.add(&y);
            Some(x)
        }
        _ => None,
    };
    Some(Val { ty, qual: a.qual.join(b.qual), callable, fields })
}

/// What the arguments bind the callee's type variables to where a result can come from: a parameter or record field
/// of a variable's type, and a callable's result (LG2.19: substitution preserves the qualifier recursively).
fn bindings(slot: &QTy, v: &Val, out: &mut Vec<(String, Val)>) {
    match &slot.ty {
        Ty::Var(name) => out.push((name.clone(), v.clone())),
        Ty::Record(fields) => {
            for (i, (_, f)) in fields.iter().enumerate() {
                bindings(f, &v.field(i), out);
            }
        }
        Ty::Fn(s) => {
            if let Ty::Fn(t) = &v.ty {
                let mut result = Val::declared(&t.result);
                result.qual = result.qual.join(v.qual.container());
                bindings(&s.result, &result, out);
            }
        }
        _ => {}
    }
}

/// The qualifier a display instance adds to a call (LG2.19): a display value bound to a type variable of a parameter
/// that is not itself a display input, directly, in a record field or as a callable's result, makes the callee's
/// body a display body. `except` leaves out the result's own type variable, whose qualifier the result keeps as it
/// is.
fn instance(params: &[QTy], vals: &[Val], except: Option<&str>) -> Qual {
    let mut bound = Vec::new();
    for (p, v) in params.iter().zip(vals).filter(|(p, _)| declared_qual(p) == Qual::Exact) {
        bindings(p, v, &mut bound);
    }
    bound.iter().filter(|(n, _)| Some(n.as_str()) != except).fold(Qual::Exact, |q, (_, v)| q.join(v.qual.container()))
}

/// An argument that binds the type variable of a call's result to a type with no common type with an earlier binding.
struct Conflict {
    arg: usize,
    var: String,
    /// The argument's slot with the variable replaced by the earlier binding's type.
    expected: String,
    supplied: String,
}

/// The value of a call. A generic result keeps the qualifier, callable and fields of every argument bound to its
/// type variable; a display instance, a display-qualified callable and a callee whose body selects its result under
/// approximate control (`control`) give a display-qualified result (LG2.19, LG2.20). Values of incompatible types
/// bound to one type variable are a [`Conflict`] (the checker does not unify type variables, M1).
fn result_val(result: &QTy, params: &[QTy], vals: &[Val], callee: Qual, control: bool) -> Result<Val, Conflict> {
    let own = match &result.ty {
        Ty::Var(name) => Some(name.as_str()),
        _ => None,
    };
    let mut returned: Option<Val> = None;
    for (arg, (p, v)) in params.iter().zip(vals).enumerate() {
        let mut bound = Vec::new();
        bindings(p, v, &mut bound);
        for (var, b) in bound.into_iter().filter(|(n, _)| Some(n.as_str()) == own) {
            returned = Some(match returned {
                None => b,
                Some(r) => {
                    let earlier = r.ty.clone();
                    merge(r, b).ok_or_else(|| Conflict {
                        arg,
                        expected: type_text(&replace_var(&p.ty, &var, &earlier)),
                        supplied: type_text(&v.ty),
                        var,
                    })?
                }
            });
        }
    }
    let mut v = returned.unwrap_or_else(|| instantiated(result, params, vals));
    if is_unknown(&v.ty) {
        return Ok(v);
    }
    if result.display {
        v.qual = v.qual.join(Qual::Display { control: false });
    }
    if control {
        v.qual = v.qual.join(Qual::Display { control: true });
    }
    v.qual = v.qual.join(instance(params, vals, own)).join(callee.container());
    Ok(v)
}

/// A value of the declared type `q` that a call returns: each callable in it, itself or in a record field, needs its
/// bound with the call's effect variables instantiated (LG3.2).
fn instantiated(q: &QTy, params: &[QTy], vals: &[Val]) -> Val {
    match &q.ty {
        Ty::Fn(f) => {
            let callable = substitute(&Summary::of_bound(&f.effects), params, vals);
            let ty = Ty::Fn(Box::new(FnTy {
                params: f.params.clone(),
                result: f.result.clone(),
                effects: callable.bound(),
            }));
            Val { ty, qual: declared_qual(q), callable: Some(callable), fields: None }
        }
        Ty::Record(fields) => {
            let field_vals: Vec<Val> = fields.iter().map(|(_, f)| instantiated(f, params, vals)).collect();
            let tys = fields
                .iter()
                .zip(&field_vals)
                .map(|((n, f), v)| (n.clone(), QTy { ty: v.ty.clone(), display: f.display }))
                .collect();
            Val { ty: Ty::Record(tys), qual: declared_qual(q), callable: None, fields: Some(field_vals) }
        }
        _ => Val::declared(q),
    }
}

/// The callee's needs with its effect variables replaced by the needs of the callables passed for them, as
/// arguments or record fields at any depth (LG3.2).
fn substitute(callee: &Summary, params: &[QTy], vals: &[Val]) -> Summary {
    fn bind(slot: &QTy, v: &Val, var: &str, out: &mut Summary, bound_here: &mut bool) {
        match &slot.ty {
            Ty::Fn(f) if f.effects.vars.contains(var) => {
                *bound_here = true;
                match &v.callable {
                    Some(callable) => out.add(callable),
                    None => {
                        out.unresolved.insert(format!("<argument for {var}>"));
                    }
                }
            }
            Ty::Record(fields) => {
                for (i, (_, f)) in fields.iter().enumerate() {
                    bind(f, &v.field(i), var, out, bound_here);
                }
            }
            _ => {}
        }
    }
    let mut out = callee.clone();
    out.vars.clear();
    for var in &callee.vars {
        let mut bound_here = false;
        for (slot, v) in params.iter().zip(vals) {
            bind(slot, v, var, &mut out, &mut bound_here);
        }
        if !bound_here {
            out.vars.insert(var.clone());
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Site {
    /// An ordinary condition, match, short-circuit boolean or loop bound (LG2.20).
    Control,
    /// An exact numeric operand or a budget (LG2.22).
    Operand,
    /// An attempted conversion back to exact (LG2.22).
    Convert,
    /// A function, closure, aggregate, storage, export or preview boundary (LG2.19).
    Boundary,
}

#[derive(Default)]
struct Env {
    vars: Vec<(String, Val)>,
}

impl Env {
    fn get(&self, name: &str) -> Option<&Val> {
        self.vars.iter().rev().find(|(n, _)| n == name).map(|(_, v)| v)
    }
}

struct Found {
    key: (usize, Option<usize>, Vec<u32>),
    diagnostic: Diagnostic,
}

struct Checker<'p> {
    program: &'p Program,
    modules: BTreeMap<&'p str, usize>,
    functions: Vec<BTreeMap<&'p str, usize>>,
    table: Vec<Vec<Summary>>,
    /// Whether each function's body computes or selects its result under approximate control (LG2.20, LG2.22).
    controls: Vec<Vec<bool>>,
    /// The typed fields the program's own workspace writes store, by record (LG2.19, storage row).
    stored: BTreeMap<&'p str, Vec<&'p QTy>>,
    emitting: bool,
    /// How many display contexts enclose the current expression: display branches, and the bodies of functions and
    /// closures with an approximate or display input or capture (LG2.19, LG2.20).
    display: u32,
    /// How many display branches of the current body enclose the current expression; a closure's body runs when it
    /// is called and starts at zero (LG2.20).
    steer: u32,
    cur: (usize, usize),
    path: Vec<u32>,
    found: Vec<Found>,
}

impl<'p> Checker<'p> {
    fn new(program: &'p Program) -> Result<Self, IrError> {
        let mut modules = BTreeMap::new();
        let mut functions = Vec::with_capacity(program.modules.len());
        for (mi, module) in program.modules.iter().enumerate() {
            if modules.insert(module.name.as_str(), mi).is_some() {
                return Err(IrError(format!("module `{}` is defined twice", module.name)));
            }
            let mut names = BTreeMap::new();
            for (fi, f) in module.functions.iter().enumerate() {
                if names.insert(f.name.as_str(), fi).is_some() {
                    return Err(IrError(format!("function `{}.{}` is defined twice", module.name, f.name)));
                }
            }
            functions.push(names);
        }
        let table = program.modules.iter().map(|m| vec![Summary::default(); m.functions.len()]).collect();
        let controls = program.modules.iter().map(|m| vec![false; m.functions.len()]).collect();
        let mut stored = BTreeMap::new();
        for f in program.modules.iter().flat_map(|m| &m.functions) {
            writes(&f.body, &mut stored);
        }
        Ok(Checker {
            program,
            modules,
            functions,
            table,
            controls,
            stored,
            emitting: false,
            display: 0,
            steer: 0,
            cur: (0, 0),
            path: Vec::new(),
            found: Vec::new(),
        })
    }

    /// Infer every function's needs to the least fixed point, recursion included (LG3.2).
    fn infer(&mut self) -> Result<(), IrError> {
        let count: usize = self.table.iter().map(Vec::len).sum();
        for _ in 0..count * 64 + 64 {
            let mut changed = false;
            for mi in 0..self.table.len() {
                for fi in 0..self.table[mi].len() {
                    let (s, control) = self.function(mi, fi)?;
                    if s != self.table[mi][fi] || control != self.controls[mi][fi] {
                        self.table[mi][fi] = s;
                        self.controls[mi][fi] = control;
                        changed = true;
                    }
                }
            }
            if !changed {
                return Ok(());
            }
        }
        Err(IrError("effect inference did not converge".into()))
    }

    fn emit(&mut self) -> Result<(), IrError> {
        self.emitting = true;
        let program = self.program;
        for (mi, module) in program.modules.iter().enumerate() {
            for cap in NEVER_GRANTED {
                if module.capabilities.contains(&cap) {
                    self.found.push(Found {
                        key: (mi, None, Vec::new()),
                        diagnostic: Diagnostic {
                            code: NOT_GRANTED,
                            module: module.name.clone(),
                            function: String::new(),
                            path: "manifest".into(),
                            value_type: None,
                            display_control: false,
                            qualifier_erasure: false,
                            mismatch: None,
                            note: format!(
                                "{cap:?} access is never granted in 1.0, even when a manifest requests it (LG3.8)"
                            ),
                        },
                    });
                }
            }
            for fi in 0..module.functions.len() {
                let (s, control) = self.function(mi, fi)?;
                if s != self.table[mi][fi] || control != self.controls[mi][fi] {
                    return Err(IrError("effect inference stopped before its fixed point".into()));
                }
            }
        }
        self.found.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(())
    }

    fn report(self) -> Report {
        let program = self.program;
        let mut summaries = BTreeMap::new();
        for (mi, module) in program.modules.iter().enumerate() {
            for (fi, f) in module.functions.iter().enumerate() {
                summaries.insert((module.name.clone(), f.name.clone()), self.table[mi][fi].clone());
            }
        }
        Report { diagnostics: self.found.into_iter().map(|f| f.diagnostic).collect(), summaries }
    }

    fn err(&self, message: String) -> IrError {
        let program = self.program;
        let (mi, fi) = self.cur;
        let place = match program.modules.get(mi).and_then(|m| m.functions.get(fi).map(|f| (m, f))) {
            Some((m, f)) => format!("{}.{} at {}", m.name, f.name, path_text(&self.path)),
            None => "the program".into(),
        };
        IrError(format!("{place}: {message}"))
    }

    /// Record a finding at the current position. `value` is the offending value's type name with the vectors'
    /// display-control and qualifier-erasure fields.
    fn push(&mut self, code: &'static str, value: Option<(String, bool, bool)>, note: String) {
        self.record(code, value, None, note);
    }

    /// `LG2:type.mismatch` at the current position (LG2.1).
    fn mismatch(&mut self, expected: String, supplied: String, what: &str) {
        let note = format!("{what}: {supplied} where {expected} is expected (LG2.1)");
        self.record(TYPE_MISMATCH, None, Some(Mismatch { expected, supplied }), note);
    }

    /// A value at the current position whose base type does not fit its use. Its exactness is not checked: where
    /// both would apply, the base type comes first. The caller goes on with [`Val::unknown`] in its place.
    fn misfit(&mut self, expected: &str, v: &Val, what: &str) {
        self.mismatch(expected.to_string(), type_text(&v.ty), what);
    }

    fn record(
        &mut self,
        code: &'static str,
        value: Option<(String, bool, bool)>,
        mismatch: Option<Mismatch>,
        note: String,
    ) {
        if !self.emitting {
            return;
        }
        let program = self.program;
        let (mi, fi) = self.cur;
        let module = &program.modules[mi];
        let (value_type, display_control, qualifier_erasure) = match value {
            Some((ty, control, erasure)) => (Some(ty), control, erasure),
            None => (None, false, false),
        };
        self.found.push(Found {
            key: (mi, Some(fi), self.path.clone()),
            diagnostic: Diagnostic {
                code,
                module: module.name.clone(),
                function: module.functions[fi].name.clone(),
                path: path_text(&self.path),
                value_type,
                display_control,
                qualifier_erasure,
                mismatch,
                note,
            },
        });
    }

    fn exactness(&mut self, code: &'static str, ty: &Ty, control: bool, erasure: bool, note: String) {
        self.push(code, Some((type_name(ty), control, erasure)), note);
    }

    fn effect(&mut self, note: String) {
        self.push(EFFECT_UNDECLARED, None, note);
    }

    fn at<T>(&mut self, i: usize, f: impl FnOnce(&mut Self) -> T) -> T {
        self.path.push(i as u32);
        let r = f(self);
        self.path.pop();
        r
    }

    /// Run `f` at the relative path `rel` below the current position.
    fn within<T>(&mut self, rel: &[u32], f: impl FnOnce(&mut Self) -> T) -> T {
        let depth = self.path.len();
        self.path.extend_from_slice(rel);
        let r = f(self);
        self.path.truncate(depth);
        r
    }

    fn in_display<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        self.display += 1;
        let r = f(self);
        self.display -= 1;
        r
    }

    /// One operation's own needs where a display context requires purity, so that each fault is reported once, at
    /// its operation or call (LG2.19, LG2.20). An effect is outside the context's allowed effect set; an exact sink
    /// would let approximate control steer an exact value.
    fn leaf(&mut self, own: &Summary, what: &str) {
        if self.display == 0 {
            return;
        }
        if own.has_effect() {
            let mut effects = own.clone();
            effects.exact_sinks.clear();
            self.effect(format!(
                "{what} needs {}, but a display branch or display body is pure (LG2.19, LG2.20)",
                describe(&effects)
            ));
        }
        if let Some((role, ty)) = own.exact_sinks.iter().next() {
            let note = format!("{what} supplies an exact {role:?} under approximate control (LG2.20, CT-LG2-05)");
            self.push(EXACTNESS_VIOLATION, Some((ty.clone(), true, false)), note);
        }
    }

    fn resolve(&self, module: Option<&str>, name: &str) -> Result<(usize, usize), IrError> {
        let mi = match module {
            None => self.cur.0,
            Some(m) => *self.modules.get(m).ok_or_else(|| self.err(format!("unknown module `{m}`")))?,
        };
        let fi = *self.functions[mi].get(name).ok_or_else(|| self.err(format!("unknown function `{name}`")))?;
        let program = self.program;
        if mi != self.cur.0 && !program.modules[mi].functions[fi].exported {
            return Err(self.err(format!("`{}.{name}` is not exported", program.modules[mi].name)));
        }
        Ok((mi, fi))
    }

    /// An exact-use site. LG2.22's precedence: a display condition, a value under approximate control or an erased
    /// qualifier is an exactness violation; a direct approximate scalar at an exact operand or a conversion back to
    /// exact is `exact_required`. A value of unknown type gets no check.
    fn exact_use(&mut self, site: Site, v: &Val, what: &str) {
        if is_unknown(&v.ty) {
            return;
        }
        match v.qual {
            Qual::Exact => {}
            Qual::Cond => self.exactness(
                EXACTNESS_VIOLATION,
                &v.ty,
                false,
                false,
                format!("{what}: a display condition at an exact use (LG2.19, LG2.20)"),
            ),
            Qual::Display { control: true } => self.exactness(
                EXACTNESS_VIOLATION,
                &v.ty,
                true,
                false,
                format!("{what}: a value computed or selected under approximate control (LG2.20, LG2.22)"),
            ),
            Qual::Display { control: false } => self.exactness(
                EXACTNESS_VIOLATION,
                &v.ty,
                false,
                true,
                format!("{what}: this would erase the display qualifier (LG2.19, LG2.22)"),
            ),
            Qual::Approx => match site {
                Site::Operand | Site::Convert => self.exactness(
                    EXACT_REQUIRED,
                    &v.ty,
                    false,
                    false,
                    format!("{what}: a direct approximate scalar where an exact value is required (LG2.22)"),
                ),
                Site::Control | Site::Boundary => self.exactness(
                    EXACTNESS_VIOLATION,
                    &v.ty,
                    false,
                    true,
                    format!("{what}: this would erase the approximate qualifier (LG2.19, LG2.22)"),
                ),
            },
        }
    }

    /// A value entering a typed slot: an argument, or with `own` a result or a stored field, whose type variables are
    /// the enclosing function's own.
    fn slot(&mut self, slot: &QTy, v: &Val, what: &str, own: bool) {
        self.callables(slot, v, what, own);
        self.qualifiers(slot, v, what, own);
    }

    /// Every callable in the value, itself or in a record field at any depth, stays within its slot's bound. An
    /// argument's effect variables are the callee's own and take whatever the callable needs; in a result or a stored
    /// field they are the enclosing function's and bound the callable like any effect (LG3.2).
    fn callables(&mut self, slot: &QTy, v: &Val, what: &str, own: bool) {
        if is_unknown(&v.ty) {
            return;
        }
        match &slot.ty {
            Ty::Fn(fty) => match &v.callable {
                None => self.effect(format!("{what}: an unknown callable is neither exact nor pure (LG2.19, LG3.2)")),
                // A bound admits no exact sink: whoever calls the callable may do so under approximate control
                // (LG2.20).
                Some(callable)
                    if (own || fty.effects.vars.is_empty())
                        && !(callable.within(&fty.effects) && callable.exact_sinks.is_empty()) =>
                {
                    self.effect(format!("{what}: the callable needs {}, beyond its bound (LG3.2)", describe(callable)))
                }
                Some(_) => {}
            },
            Ty::Record(fields) => {
                for (i, (name, f)) in fields.iter().enumerate() {
                    self.callables(f, &v.field(i), &format!("{what}, field `{name}`"), own);
                }
            }
            _ => {}
        }
    }

    /// The value keeps its qualifiers in the slot: as a whole, in a callable's signature and in each record field.
    /// A value refused as a whole is reported once.
    fn qualifiers(&mut self, slot: &QTy, v: &Val, what: &str, own: bool) {
        if is_unknown(&v.ty) {
            return;
        }
        if exact_slot(slot, own) && v.qual != Qual::Exact {
            self.exact_use(Site::Boundary, v, what);
            return;
        }
        match (&slot.ty, &v.ty) {
            (Ty::Fn(s), Ty::Fn(t)) if !signature_keeps(s, t, own) => self.exactness(
                EXACTNESS_VIOLATION,
                &v.ty,
                false,
                true,
                format!("{what}: the slot's signature would erase the qualifier of the callable's result or a parameter (LG2.19)"),
            ),
            (Ty::Record(fields), Ty::Record(_)) => {
                for (i, (name, f)) in fields.iter().enumerate() {
                    self.qualifiers(f, &v.field(i), &format!("{what}, field `{name}`"), own);
                }
            }
            _ => {}
        }
    }

    /// The arguments enter their slots. An argument whose base type does not fit is reported and replaced by
    /// [`Val::unknown`].
    fn arguments(&mut self, params: &[QTy], vals: &mut [Val], what: &str) -> Result<(), IrError> {
        if params.len() != vals.len() {
            return Err(self.err(format!("{what} takes {} arguments, not {}", params.len(), vals.len())));
        }
        for (i, (slot, v)) in params.iter().zip(vals.iter_mut()).enumerate() {
            let arg = format!("argument {i} of {what}");
            if fits(&slot.ty, &v.ty) {
                self.at(i, |c| c.slot(slot, v, &arg, false));
            } else {
                self.at(i, |c| c.misfit(&type_text(&slot.ty), v, &arg));
                *v = Val::unknown();
            }
        }
        Ok(())
    }

    /// A call: the arguments enter their slots, the callee's needs are instantiated and checked against the context,
    /// and the result follows `result_val` (LG2.19, LG3.2). `summary` is `None` for an unknown callable; `callee` is
    /// the callable's qualifier, and `control` says that the callee selects its result under approximate control.
    #[allow(clippy::too_many_arguments)]
    fn call(
        &mut self,
        params: &[QTy],
        result: &QTy,
        summary: Option<&Summary>,
        vals: &mut [Val],
        callee: Qual,
        control: bool,
        what: &str,
    ) -> Result<(Val, Summary), IrError> {
        self.arguments(params, vals, what)?;
        let vals: &[Val] = vals;
        let needs = match summary {
            Some(summary) => {
                let needs = substitute(summary, params, vals);
                let call = format!("the call of {what}");
                if callee != Qual::Exact {
                    // A display-qualified callable runs a display body, which is pure (LG2.19, LG2.20).
                    let call = format!("{call}, a display-qualified callable,");
                    self.in_display(|c| c.leaf(&needs, &call));
                } else if instance(params, vals, None) != Qual::Exact {
                    // A display value for a generic parameter makes the callee's body a display body, which is pure
                    // (LG2.19, LG2.20).
                    let call = format!("{call}, whose type variable takes a display value,");
                    self.in_display(|c| c.leaf(&needs, &call));
                } else {
                    self.leaf(&needs, &call);
                }
                if self.steer > 0 && needs.reaches.contains(&self.current()) {
                    self.exactness(
                        EXACTNESS_VIOLATION,
                        &result.ty,
                        true,
                        false,
                        format!("{call} recurses under approximate control; approximate data cannot bound recursion (LG2.20)"),
                    );
                }
                needs
            }
            None => {
                self.effect("a call of an unknown callable is neither exact nor pure (LG2.19, LG3.2)".into());
                let mut needs = Summary::default();
                needs.unresolved.insert("<function value>".into());
                needs
            }
        };
        let v = match result_val(result, params, vals, callee, control) {
            Ok(v) => v,
            Err(Conflict { arg, var, expected, supplied }) => {
                let note = format!(
                    "argument {arg} of {what} binds the result's type variable {var} to a type that has no common \
                     type with an earlier binding"
                );
                self.at(arg, |c| c.mismatch(expected, supplied, &note));
                Val::unknown()
            }
        };
        Ok((v, needs))
    }

    /// The current function's module and name.
    fn current(&self) -> (String, String) {
        let program = self.program;
        let module = &program.modules[self.cur.0];
        (module.name.clone(), module.functions[self.cur.1].name.clone())
    }

    /// A function's or closure's result, whose returned expression is at `at` below the current position, and
    /// whether its base type fits. An approximate or display input requires a display-qualified result (LG2.19).
    fn result(&mut self, slot: &QTy, v: &Val, display_input: Option<&Ty>, what: &str, at: &[u32]) -> bool {
        if !fits(&slot.ty, &v.ty) {
            self.within(at, |c| c.misfit(&type_text(&slot.ty), v, what));
            return false;
        }
        if is_unknown(&v.ty) {
            return true;
        }
        if exact_slot(slot, true) && v.qual == Qual::Exact {
            if let Some(input) = display_input {
                self.exactness(
                    EXACTNESS_VIOLATION,
                    input,
                    false,
                    true,
                    format!("{what}: an approximate or display input requires a display-qualified result (LG2.19)"),
                );
            }
        }
        self.slot(slot, v, what, true);
        true
    }

    /// The value of two branches or arms. When `b`, at child `at`, has no common type with `a`, it is reported and
    /// the value is unknown.
    fn join(&mut self, a: Val, b: Val, at: usize, what: &str) -> Val {
        let (ta, tb) = (a.ty.clone(), b.ty.clone());
        merge(a, b).unwrap_or_else(|| {
            self.at(at, |c| c.mismatch(type_text(&ta), type_text(&tb), what));
            Val::unknown()
        })
    }

    /// A function's needs, and whether its body computes or selects its result under approximate control.
    fn function(&mut self, mi: usize, fi: usize) -> Result<(Summary, bool), IrError> {
        let program = self.program;
        let module = &program.modules[mi];
        let f = &module.functions[fi];
        self.cur = (mi, fi);
        self.path.clear();
        let mut env = Env::default();
        for p in &f.params {
            env.vars.push((p.name.clone(), Val::declared(&p.ty)));
        }
        let display_input = f.params.iter().find(|p| declared_qual(&p.ty) != Qual::Exact).map(|p| &p.ty.ty);
        // A function with an approximate or display input has a pure display body (LG2.19).
        self.display = u32::from(display_input.is_some());
        self.steer = 0;
        let walked = self.expr(&f.body, &mut env);
        self.display = 0;
        let (v, s) = walked?;
        let fit = self.result(&f.result, &v, display_input, "the result", &returned(&f.body));
        // An exact result slot refuses a steered value above; callers see no second finding. A display-qualified
        // result shows callers its qualifier, so a later exact use erases it (LG2.22). The bit carries the control
        // only through a result whose declared type cannot show it, such as `Approx<p>`: its exact use is then a
        // value selected under approximate control, not a direct approximate scalar (`exact_required`).
        let control =
            fit && !f.result.display && !exact_slot(&f.result, true) && v.qual == (Qual::Display { control: true });
        if f.kind == FnKind::View {
            for e in [Effect::ChangeWorkspace, Effect::PreparePreview] {
                if s.effects.contains(&e) {
                    self.effect(format!("a view has no {e:?} effect, also through callees (LG3.5, LG3.6)"));
                }
            }
        }
        if let Some(annotation) = &f.effects {
            if !s.within(annotation) {
                self.effect(format!(
                    "the inferred needs ({}) exceed the annotation, which cannot erase them (LG3.2)",
                    describe(&s)
                ));
            }
        }
        let undeclared: Vec<String> =
            s.capabilities.difference(&module.capabilities).map(|c| format!("{c:?}")).collect();
        if !undeclared.is_empty() {
            self.effect(format!("module `{}` does not declare {} (LG3.8, LG3.9)", module.name, undeclared.join(", ")));
        }
        Ok((s, control))
    }

    fn expr(&mut self, e: &'p Expr, env: &mut Env) -> Result<(Val, Summary), IrError> {
        match e {
            // An approximate literal is display data from the start, like an explicit conversion (LG1.7, LG2.19).
            Expr::Lit(Lit::Approx { precision, .. }) => {
                Ok((Val::new(Ty::Approx(*precision), Qual::Approx), Summary::default()))
            }
            Expr::Lit(lit) => Ok((Val::exact(lit_type(lit)), Summary::default())),
            Expr::Var(name) => match env.get(name) {
                Some(v) => Ok((v.clone(), Summary::default())),
                None => Err(self.err(format!("unbound name `{name}`"))),
            },
            Expr::FnRef { module, name } => {
                let (mi, fi) = self.resolve(module.as_deref(), name)?;
                let program = self.program;
                let f = &program.modules[mi].functions[fi];
                let mut summary = self.table[mi][fi].clone();
                summary.reaches.insert((program.modules[mi].name.clone(), f.name.clone()));
                let ty = Ty::Fn(Box::new(FnTy {
                    params: f.params.iter().map(|p| p.ty.clone()).collect(),
                    result: f.result.clone(),
                    effects: summary.bound(),
                }));
                Ok((Val { ty, qual: Qual::Exact, callable: Some(summary), fields: None }, Summary::default()))
            }
            Expr::Let { name, value, body } => {
                let (v, mut s) = self.at(0, |c| c.expr(value, env))?;
                env.vars.push((name.clone(), v));
                let r = self.at(1, |c| c.expr(body, env));
                env.vars.pop();
                let (bv, bs) = r?;
                s.add(&bs);
                Ok((bv, s))
            }
            Expr::If { cond, then, els } => {
                let (cv, mut s) = self.at(0, |c| c.expr(cond, env))?;
                if fits(&Ty::Bool, &cv.ty) {
                    self.at(0, |c| c.exact_use(Site::Control, &cv, "an ordinary if condition"));
                } else {
                    self.at(0, |c| c.misfit("Bool", &cv, "an if condition"));
                }
                let (tv, ts) = self.at(1, |c| c.expr(then, env))?;
                let (ev, es) = self.at(2, |c| c.expr(els, env))?;
                s.add(&ts);
                s.add(&es);
                Ok((self.join(tv, ev, 2, "the else branch of an if"), s))
            }
            Expr::Match { scrutinee, arms, default } => {
                let (sv, mut s) = self.at(0, |c| c.expr(scrutinee, env))?;
                self.at(0, |c| c.exact_use(Site::Control, &sv, "a match scrutinee"));
                let mut out: Option<Val> = None;
                for (i, (lit, arm)) in arms.iter().enumerate() {
                    // A pattern has no path of its own, so the match is the offending range; a pattern compares with
                    // the scrutinee as an exact equality does.
                    let pattern = lit_type(lit);
                    if !fits(&pattern, &sv.ty) && !fits(&sv.ty, &pattern) {
                        let what = format!("the pattern of arm {i} of a match");
                        self.mismatch(type_text(&sv.ty), type_text(&pattern), &what);
                    }
                    let (av, a_s) = self.at(i + 1, |c| c.expr(arm, env))?;
                    s.add(&a_s);
                    out = Some(match out {
                        None => av,
                        Some(o) => self.join(o, av, i + 1, &format!("arm {i} of a match")),
                    });
                }
                let (dv, ds) = self.at(arms.len() + 1, |c| c.expr(default, env))?;
                s.add(&ds);
                let v = match out {
                    None => dv,
                    Some(o) => self.join(o, dv, arms.len() + 1, "the `_` arm of a match"),
                };
                Ok((v, s))
            }
            Expr::DisplayIf { cond, then, els } => {
                let (cv, mut s) = self.at(0, |c| c.expr(cond, env))?;
                if cv.ty != Ty::ApproxBool && !is_unknown(&cv.ty) {
                    self.at(0, |c| c.misfit("ApproxBool", &cv, "a display_if condition"));
                }
                // The branches are lazy pure display expressions under approximate control (LG2.20).
                self.steer += 1;
                let then = self.at(1, |c| c.in_display(|c| c.expr(then, env)));
                let els = self.at(2, |c| c.in_display(|c| c.expr(els, env)));
                self.steer -= 1;
                let ((tv, ts), (ev, es)) = (then?, els?);
                let v = self.join(tv, ev, 2, "the else branch of a display_if");
                s.add(&ts);
                s.add(&es);
                Ok((v.steered(), s))
            }
            Expr::Call { callee, args } => {
                let mut s = Summary::default();
                let mut vals: Vec<Val> = Vec::with_capacity(args.len());
                for (i, a) in args.iter().enumerate() {
                    let (v, a_s) = self.at(i, |c| c.expr(a, env))?;
                    s.add(&a_s);
                    vals.push(v);
                }
                match callee {
                    Callee::Fn { module, name } => {
                        let (mi, fi) = self.resolve(module.as_deref(), name)?;
                        let program = self.program;
                        let f = &program.modules[mi].functions[fi];
                        let params: Vec<QTy> = f.params.iter().map(|p| p.ty.clone()).collect();
                        let mut summary = self.table[mi][fi].clone();
                        summary.reaches.insert((program.modules[mi].name.clone(), f.name.clone()));
                        let control = self.controls[mi][fi];
                        let what = format!("`{name}`");
                        let (v, needs) =
                            self.call(&params, &f.result, Some(&summary), &mut vals, Qual::Exact, control, &what)?;
                        s.add(&needs);
                        Ok((v, s))
                    }
                    Callee::Value(fe) => {
                        let (fv, fs) = self.at(args.len(), |c| c.expr(fe, env))?;
                        s.add(&fs);
                        let fty = match &fv.ty {
                            Ty::Fn(fty) => (**fty).clone(),
                            other => {
                                if !is_unknown(other) {
                                    let params: Vec<String> = vals.iter().map(|v| type_text(&v.ty)).collect();
                                    let expected = format!("fn({}) -> _", params.join(", "));
                                    self.at(args.len(), |c| c.misfit(&expected, &fv, "a called value"));
                                }
                                // What the call needs is unknown (LG3.2).
                                s.unresolved.insert("<a value of unknown type>".into());
                                return Ok((Val::unknown(), s));
                            }
                        };
                        let callable = fv.callable.as_ref();
                        let (v, needs) = self.call(
                            &fty.params,
                            &fty.result,
                            callable,
                            &mut vals,
                            fv.qual,
                            false,
                            "a function value",
                        )?;
                        s.add(&needs);
                        Ok((v, s))
                    }
                    Callee::Unresolved { name, result } => {
                        self.effect(format!(
                            "the unresolved callable `{name}` is neither exact nor pure (LG2.19, LG3.2)"
                        ));
                        s.unresolved.insert(name.clone());
                        Ok((Val::declared(result), s))
                    }
                }
            }
            Expr::Closure { params, result, body } => {
                let mut free = BTreeSet::new();
                let mut bound: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
                free_names(body, &mut bound, &mut free);
                let mut inner = Env::default();
                for name in free {
                    match env.get(name) {
                        Some(v) => inner.vars.push((name.to_string(), v.clone())),
                        None => return Err(self.err(format!("unbound name `{name}` in a closure"))),
                    }
                }
                // The effect variables of the closure's parameters are its own: a call instantiates them, apart from
                // those of the callables it captures, so they get names of their own (LG3.2).
                let mut own = BTreeSet::new();
                for p in params {
                    effect_vars(&p.ty, &mut own);
                }
                let (module, function) = self.current();
                let place = format!("{module}.{function}@{}", path_text(&self.path));
                let renamed: BTreeMap<String, String> =
                    own.into_iter().map(|v| (v.clone(), format!("{v}'{place}"))).collect();
                let param_tys: Vec<QTy> = params.iter().map(|p| rename_vars(&p.ty, &renamed)).collect();
                let result = rename_vars(result, &renamed);
                // A closure capturing display data is display-qualified as a whole (LG2.19).
                let captured = inner.vars.iter().fold(Qual::Exact, |q, (_, v)| q.join(v.qual.container()));
                let display_input = param_tys
                    .iter()
                    .find(|t| declared_qual(t) != Qual::Exact)
                    .map(|t| t.ty.clone())
                    .or_else(|| inner.vars.iter().find(|(_, v)| v.qual != Qual::Exact).map(|(_, v)| v.ty.clone()));
                for (p, t) in params.iter().zip(&param_tys) {
                    inner.vars.push((p.name.clone(), Val::declared(t)));
                }
                // An approximate or display input or capture makes the body a pure display body (LG2.19). The body
                // runs when the closure is called, and the call is checked where it happens.
                let steer = std::mem::replace(&mut self.steer, 0);
                let walked = if display_input.is_some() {
                    self.at(0, |c| c.in_display(|c| c.expr(body, &mut inner)))
                } else {
                    self.at(0, |c| c.expr(body, &mut inner))
                };
                self.steer = steer;
                let (bv, bs) = walked?;
                let mut at = vec![0];
                at.extend(returned(body));
                self.result(&result, &bv, display_input.as_ref(), "the closure's result", &at);
                // With an exact result slot a display capture was refused above; the value carries no second finding.
                let qual = if exact_slot(&result, true) { Qual::Exact } else { captured };
                let ty = Ty::Fn(Box::new(FnTy { params: param_tys, result, effects: bs.bound() }));
                Ok((Val { ty, qual, callable: Some(bs), fields: None }, Summary::default()))
            }
            Expr::Prim { op, args } => {
                let mut s = Summary::default();
                let mut vals = Vec::with_capacity(args.len());
                for (i, a) in args.iter().enumerate() {
                    let (v, a_s) = self.at(i, |c| c.expr(a, env))?;
                    s.add(&a_s);
                    vals.push(v);
                }
                Ok((self.prim(*op, &vals)?, s))
            }
            Expr::Record(fields) => {
                let mut s = Summary::default();
                let mut tys = Vec::with_capacity(fields.len());
                let mut vals = Vec::with_capacity(fields.len());
                let mut qual = Qual::Exact;
                for (i, (name, fe)) in fields.iter().enumerate() {
                    let (v, f_s) = self.at(i, |c| c.expr(fe, env))?;
                    s.add(&f_s);
                    qual = qual.join(v.qual.container());
                    let display = v.qual != Qual::Exact && !matches!(v.ty, Ty::Approx(_) | Ty::ApproxBool);
                    tys.push((name.clone(), QTy { ty: v.ty.clone(), display }));
                    vals.push(v);
                }
                // The record keeps its field values, so that a callable keeps its needs through it (LG3.2).
                Ok((Val { ty: Ty::Record(tys), qual, callable: None, fields: Some(vals) }, s))
            }
            Expr::Field { value, field } => {
                let (rv, s) = self.at(0, |c| c.expr(value, env))?;
                let index = match &rv.ty {
                    Ty::Record(fields) => fields.iter().position(|(n, _)| n == field),
                    _ => None,
                };
                let Some(index) = index else {
                    if !is_unknown(&rv.ty) {
                        let expected = format!("record {{ {field}: _, .. }}");
                        self.at(0, |c| c.misfit(&expected, &rv, &format!("a read of field `{field}`")));
                    }
                    return Ok((Val::unknown(), s));
                };
                let mut v = rv.field(index);
                // Projection cannot extract an exact result from a display-qualified aggregate (LG2.19).
                v.qual = match rv.qual {
                    Qual::Exact => v.qual,
                    Qual::Display { control: true } => v.qual.join(Qual::Display { control: true }),
                    _ => match v.qual {
                        Qual::Approx | Qual::Cond => v.qual,
                        q => q.join(Qual::Display { control: false }),
                    },
                };
                Ok((v, s))
            }
            Expr::ReadSnapshot { query, ty } => {
                let mut s = Summary::default();
                let mut own = needs(&[Effect::ReadSnapshot], &[]);
                own.state_reads.insert(StateRead::Snapshot);
                self.operation(&mut s, own, "a snapshot read");
                let mut v = Val::declared(ty);
                let stored = self.stored.get(query.as_str()).cloned().unwrap_or_default();
                let mut reported = false;
                for field in stored {
                    if !fits(&ty.ty, &field.ty) {
                        // The read is reported once, against the first write it does not fit.
                        if !reported {
                            let what = format!("a read of `{query}`, which a workspace write of the program stores");
                            self.mismatch(type_text(&field.ty), type_text(&ty.ty), &what);
                            reported = true;
                        }
                        continue;
                    }
                    // Decoding retains the qualifier the program stores; a read declared with a weaker one would
                    // erase it at the storage boundary (LG2.19).
                    if !admits(ty, field, false) {
                        v.qual = v.qual.join(declared_qual(field).container());
                    }
                }
                Ok((v, s))
            }
            Expr::PublishView { value } => {
                let (_, mut s) = self.at(0, |c| c.expr(value, env))?;
                self.operation(&mut s, needs(&[Effect::ChangeView], &[]), "publish_view");
                Ok((Val::exact(Ty::Unit), s))
            }
            Expr::WriteWorkspace { ty, value, .. } => {
                let (v, mut s) = self.at(0, |c| c.expr(value, env))?;
                if fits(&ty.ty, &v.ty) {
                    // Display values may be stored only in typed display fields (LG2.19).
                    self.at(0, |c| c.slot(ty, &v, "a workspace field", true));
                } else {
                    self.at(0, |c| c.misfit(&type_text(&ty.ty), &v, "a workspace field"));
                }
                self.operation(&mut s, needs(&[Effect::ChangeWorkspace], &[]), "a workspace write");
                Ok((Val::exact(Ty::Unit), s))
            }
            Expr::Propose { word, target } => {
                let (wv, mut s) = self.at(0, |c| c.expr(word, env))?;
                let (tv, t_s) = self.at(1, |c| c.expr(target, env))?;
                s.add(&t_s);
                for (i, (v, ty, what)) in [(&wv, Ty::Word, "a preview's word"), (&tv, Ty::Target, "a preview's target")]
                    .into_iter()
                    .enumerate()
                {
                    if fits(&ty, &v.ty) {
                        self.at(i, |c| c.exact_use(Site::Boundary, v, what));
                    } else {
                        self.at(i, |c| c.misfit(&type_text(&ty), v, what));
                    }
                }
                self.operation(&mut s, needs(&[Effect::PreparePreview], &[]), "a preview");
                Ok((Val::exact(Ty::Receipt), s))
            }
            Expr::ReadReceipt { receipt, field } => {
                let (rv, mut s) = self.at(0, |c| c.expr(receipt, env))?;
                if !fits(&Ty::Receipt, &rv.ty) {
                    self.at(0, |c| c.misfit("Receipt", &rv, "a receipt field read"));
                }
                let ty = match field {
                    ReceiptField::Id | ReceiptField::Base | ReceiptField::Effect | ReceiptField::EndState => Ty::Text,
                    ReceiptField::Protection | ReceiptField::Error => Ty::Status,
                    ReceiptField::N => Ty::Nat,
                };
                if !matches!(field, ReceiptField::Id | ReceiptField::Base) {
                    // A preview's outcome is state-derived data (BND.42).
                    let mut own = needs(&[Effect::ReadSnapshot], &[]);
                    own.state_reads.insert(StateRead::Outcome);
                    self.operation(&mut s, own, "a preview outcome read");
                }
                let qual = if rv.ty == Ty::Receipt { rv.qual } else { Qual::Exact };
                Ok((Val::new(ty, qual), s))
            }
            Expr::UseExact { role, value } => {
                let (v, mut s) = self.at(0, |c| c.expr(value, env))?;
                let what = format!("an exact {role:?}");
                let (site, fit) = match role {
                    ExactRole::Budget => {
                        (Site::Operand, is_integer(&v.ty) || matches!(v.ty, Ty::Approx(_)) || is_unknown(&v.ty))
                    }
                    ExactRole::Goal | ExactRole::Evidence => (Site::Boundary, true),
                };
                if !fit {
                    self.at(0, |c| c.misfit("Nat | Int | Budget", &v, &what));
                }
                if self.display > 0 {
                    // Even an exact literal computed in a display context cannot escape as an exact result (LG2.20).
                    // This is a fault of the place, whatever the value's type.
                    let note = format!("{what} set under approximate control (LG2.20, CT-LG2-05)");
                    self.push(EXACTNESS_VIOLATION, Some((type_name(&v.ty), true, false)), note);
                } else if fit {
                    self.at(0, |c| c.exact_use(site, &v, &what));
                }
                s.exact_sinks.insert((*role, type_name(&v.ty)));
                Ok((Val::exact(Ty::Unit), s))
            }
            Expr::Accounting(a) => {
                // Display accounting is a host observation with no exact continuation (LG2.20).
                let ty = match a {
                    Accounting::UsedSteps => Ty::Nat,
                    Accounting::UnusedSteps => Ty::Budget,
                    Accounting::Status => Ty::Status,
                };
                let mut s = Summary::default();
                self.operation(&mut s, needs(&[Effect::Budgets], &[]), "a display accounting read");
                Ok((Val::new(ty, Qual::Display { control: true }), s))
            }
            Expr::Declassified { id, ty } => {
                let mut s = Summary::default();
                let mut own = needs(&[Effect::ReadSnapshot], &[]);
                own.state_reads.insert(StateRead::Declassifier(id.clone()));
                self.operation(&mut s, own, "a declassified read");
                Ok((Val::declared(&QTy { ty: ty.clone(), display: false }), s))
            }
            Expr::FileRead { ty, .. } => {
                let mut s = Summary::default();
                self.operation(&mut s, needs(&[Effect::GrantedFile], &[Capability::File]), "a file read");
                Ok((Val::declared(&QTy { ty: ty.clone(), display: false }), s))
            }
            Expr::FileWrite { value, .. } => {
                let (v, mut s) = self.at(0, |c| c.expr(value, env))?;
                // Untyped file bytes are no typed display field (LG2.19).
                self.at(0, |c| c.exact_use(Site::Boundary, &v, "a file write"));
                self.operation(&mut s, needs(&[Effect::GrantedFile], &[Capability::File]), "a file write");
                Ok((Val::exact(Ty::Unit), s))
            }
            Expr::RandomDraw => {
                let mut s = Summary::default();
                let own = needs(&[Effect::SeededRandomness], &[Capability::Randomness]);
                self.operation(&mut s, own, "a random draw");
                Ok((Val::exact(Ty::Nat), s))
            }
            Expr::ReadClock => {
                // The clock is a declared requirement without an effect family (LG3.8).
                let mut s = Summary::default();
                self.operation(&mut s, needs(&[], &[Capability::Clock]), "a clock read");
                Ok((Val::exact(Ty::Nat), s))
            }
        }
    }

    /// Add one operation's own needs after checking them against an enclosing display context.
    fn operation(&mut self, s: &mut Summary, own: Summary, what: &str) {
        self.leaf(&own, what);
        s.add(&own);
    }

    fn prim(&mut self, op: Prim, vals: &[Val]) -> Result<Val, IrError> {
        let arity = match op {
            Prim::Not | Prim::ToApprox(_) | Prim::ToExact | Prim::ApproxNot => 1,
            _ => 2,
        };
        if vals.len() != arity {
            return Err(self.err(format!("{op:?} takes {arity} operands, not {}", vals.len())));
        }
        let what = format!("an operand of {op:?}");
        match op {
            Prim::ExactAdd | Prim::ExactLess => {
                // The result widens along Nat, Int, Rat and Q5; a budget is a cost and mixes with integers only, so an
                // operand that mixes it with a rational or a Q(√5) value does not fit (LG2.6).
                let mut exact: Vec<&Ty> = Vec::new();
                let mut unknown_operand = false;
                for (i, v) in vals.iter().enumerate() {
                    let budget = exact.contains(&&Ty::Budget);
                    let field = exact.iter().any(|t| matches!(t, Ty::Rat | Ty::Q5));
                    let fit = match &v.ty {
                        Ty::Budget => !field,
                        Ty::Rat | Ty::Q5 => !budget,
                        t => is_exact_number(t) || matches!(t, Ty::Approx(_)) || is_unknown(t),
                    };
                    if !fit {
                        let expected = if budget {
                            "Nat | Int | Budget"
                        } else if field {
                            "Nat | Int | Rat | Q5"
                        } else {
                            "Nat | Int | Rat | Q5 | Budget"
                        };
                        self.at(i, |c| c.misfit(expected, v, &what));
                        unknown_operand = true;
                        continue;
                    }
                    unknown_operand |= is_unknown(&v.ty);
                    if is_exact_number(&v.ty) {
                        exact.push(&v.ty);
                    }
                    self.at(i, |c| c.exact_use(Site::Operand, v, &what));
                }
                if op == Prim::ExactLess {
                    return Ok(Val::exact(Ty::Bool));
                }
                if unknown_operand {
                    return Ok(Val::unknown());
                }
                let ty = if exact.contains(&&Ty::Q5) {
                    Ty::Q5
                } else if exact.contains(&&Ty::Rat) {
                    Ty::Rat
                } else if exact.is_empty() || exact.iter().any(|t| **t == Ty::Int) {
                    Ty::Int
                } else if exact.iter().any(|t| **t == Ty::Budget) {
                    Ty::Budget
                } else {
                    Ty::Nat
                };
                Ok(Val::exact(ty))
            }
            Prim::ExactEq => {
                let (a, b) = (&vals[0], &vals[1]);
                let what = "an operand of an exact equality";
                self.at(0, |c| c.exact_use(Site::Operand, a, what));
                if fits(&a.ty, &b.ty) || fits(&b.ty, &a.ty) {
                    self.at(1, |c| c.exact_use(Site::Operand, b, what));
                } else {
                    self.at(1, |c| c.misfit(&type_text(&a.ty), b, what));
                }
                Ok(Val::exact(Ty::Bool))
            }
            Prim::And | Prim::Or | Prim::Not => {
                for (i, v) in vals.iter().enumerate() {
                    if fits(&Ty::Bool, &v.ty) {
                        self.at(i, |c| c.exact_use(Site::Control, v, "a short-circuit exact boolean"));
                    } else {
                        self.at(i, |c| c.misfit("Bool", v, &what));
                    }
                }
                Ok(Val::exact(Ty::Bool))
            }
            Prim::ToApprox(p) => {
                let v = &vals[0];
                if is_exact_number(&v.ty) {
                    return Ok(Val::new(Ty::Approx(p), Qual::Approx.join(v.qual)));
                }
                if !is_unknown(&v.ty) {
                    self.at(0, |c| c.misfit("Nat | Int | Rat | Q5 | Budget", v, "an approximate conversion"));
                }
                Ok(Val::new(Ty::Approx(p), Qual::Approx))
            }
            Prim::ToExact => {
                let v = &vals[0];
                let what = "a conversion back to exact";
                if is_integer(&v.ty) || matches!(v.ty, Ty::Approx(_)) || is_unknown(&v.ty) {
                    self.at(0, |c| c.exact_use(Site::Convert, v, what));
                } else {
                    self.at(0, |c| c.misfit("Approx<_> | Nat | Int | Budget", v, what));
                }
                Ok(Val::exact(Ty::Int))
            }
            Prim::ApproxAdd | Prim::ApproxLess => {
                // The first operand of an approximate type fixes the precision.
                let mut precision: Option<u32> = None;
                let mut q = Qual::Approx;
                let mut unknown_operand = false;
                for (i, v) in vals.iter().enumerate() {
                    match (&v.ty, precision) {
                        (Ty::Approx(p), None) => precision = Some(*p),
                        (Ty::Approx(p), Some(first)) if *p == first => {}
                        (t, _) if is_unknown(t) => {
                            unknown_operand = true;
                            continue;
                        }
                        _ => {
                            let expected =
                                precision.map_or_else(|| "Approx<_>".to_string(), |p| format!("Approx<P{p}>"));
                            self.at(i, |c| c.misfit(&expected, v, &what));
                            unknown_operand = true;
                            continue;
                        }
                    }
                    q = q.join(v.qual);
                }
                Ok(match (op, precision) {
                    (Prim::ApproxLess, _) => Val::new(Ty::ApproxBool, cond_of(q)),
                    (_, Some(p)) if !unknown_operand => Val::new(Ty::Approx(p), q),
                    _ => Val::unknown(),
                })
            }
            Prim::ApproxAnd | Prim::ApproxNot => {
                let mut q = Qual::Cond;
                for (i, v) in vals.iter().enumerate() {
                    if v.ty == Ty::ApproxBool {
                        q = q.join(v.qual);
                    } else if !is_unknown(&v.ty) {
                        self.at(i, |c| c.misfit("ApproxBool", v, &what));
                    }
                }
                Ok(Val::new(Ty::ApproxBool, cond_of(q)))
            }
        }
    }
}
