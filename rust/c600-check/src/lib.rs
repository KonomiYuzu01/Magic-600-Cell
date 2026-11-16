//! The 600 Cell Script checker over the typed IR draft `c600-ir-draft-1` (LP2, milestone M1).
//!
//! The checker implements form 1 of the exactness contract (LG2.19, LG2.20, LG2.22), effect inference and the view
//! rules (LG3.1, LG3.2, LG3.4 to LG3.6), and capabilities, grants and capability separation (LG3.8, LG3.9, LG3.13)
//! with chapter 07's interim grant table. It reports chapter 02's frozen codes and no others.
//!
//! - Malformed IR, such as a base-type mismatch or an unbound name, is an [`IrError`]: the lowering is at fault, and
//!   chapter 02 has no source diagnostic for it.
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

/// One finding. An exactness finding carries the offending value's type and the two fields of the LP1 vectors.
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
    pub note: String,
}

/// Malformed IR: the lowering is at fault, not the program.
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
}

impl Summary {
    fn add(&mut self, other: &Summary) {
        self.effects.extend(other.effects.iter().copied());
        self.capabilities.extend(other.capabilities.iter().copied());
        self.vars.extend(other.vars.iter().cloned());
        self.state_reads.extend(other.state_reads.iter().cloned());
        self.exact_sinks.extend(other.exact_sinks.iter().cloned());
        self.unresolved.extend(other.unresolved.iter().cloned());
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
        if summary.effects.contains(&Effect::PreparePreview) {
            diagnostics.push(refuse(
                "an own, forked or imported tool prepares no preview in a solving session until the joint setting \
                 (chapter 07 section 8, BND.56; LG3.16)"
                    .into(),
            ));
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

/// Whether a type mentions a type variable.
fn generic(ty: &Ty) -> bool {
    match ty {
        Ty::Var(_) => true,
        Ty::Record(fields) => fields.iter().any(|(_, f)| generic(&f.ty)),
        Ty::Fn(f) => f.params.iter().any(|p| generic(&p.ty)) || generic(&f.result.ty),
        _ => false,
    }
}

fn is_numeric(ty: &Ty) -> bool {
    matches!(ty, Ty::Nat | Ty::Int | Ty::Budget)
}

/// Whether a value of type `value` may enter a slot of type `slot`. An approximate value at an exact slot fits here
/// and is refused by the exactness rules, not as malformed IR.
fn fits(slot: &Ty, value: &Ty) -> bool {
    match (slot, value) {
        (Ty::Var(_), _) => true,
        (Ty::Approx(p), Ty::Approx(q)) => p == q,
        (Ty::Nat | Ty::Int | Ty::Budget, Ty::Approx(_)) => true,
        (Ty::Bool, Ty::ApproxBool) => true,
        (Ty::Int | Ty::Budget, Ty::Nat) => true,
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

/// The common type of two branches.
fn widen(a: &Ty, b: &Ty) -> Option<Ty> {
    if a == b {
        return Some(a.clone());
    }
    match (a, b) {
        (Ty::Nat, Ty::Int) | (Ty::Int, Ty::Nat) => Some(Ty::Int),
        (Ty::Nat, Ty::Budget) | (Ty::Budget, Ty::Nat) => Some(Ty::Budget),
        (Ty::Fn(x), Ty::Fn(y)) if x.params == y.params && x.result == y.result => {
            let mut effects = x.effects.clone();
            effects.effects.extend(y.effects.effects.iter().copied());
            effects.capabilities.extend(y.effects.capabilities.iter().copied());
            effects.vars.extend(y.effects.vars.iter().cloned());
            Some(Ty::Fn(Box::new(FnTy { params: x.params.clone(), result: x.result.clone(), effects })))
        }
        _ => None,
    }
}

fn lit_type(lit: &Lit) -> Ty {
    match lit {
        Lit::Unit => Ty::Unit,
        Lit::Bool(_) => Ty::Bool,
        Lit::Nat(_) => Ty::Nat,
        Lit::Int(_) => Ty::Int,
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
            for child in children(other) {
                free_names(child, bound, out);
            }
        }
    }
}

/// The child expressions in path order.
fn children(e: &Expr) -> Vec<&Expr> {
    match e {
        Expr::Lit(_)
        | Expr::Var(_)
        | Expr::FnRef { .. }
        | Expr::ReadSnapshot { .. }
        | Expr::Accounting(_)
        | Expr::Declassified { .. }
        | Expr::FileRead { .. }
        | Expr::RandomDraw
        | Expr::ReadClock => Vec::new(),
        Expr::Let { value, body, .. } => vec![&**value, &**body],
        Expr::If { cond, then, els } | Expr::DisplayIf { cond, then, els } => vec![&**cond, &**then, &**els],
        Expr::Match { scrutinee, arms, default } => {
            let mut out = vec![&**scrutinee];
            out.extend(arms.iter().map(|(_, arm)| arm));
            out.push(&**default);
            out
        }
        Expr::Call { callee, args } => {
            let mut out: Vec<&Expr> = args.iter().collect();
            if let Callee::Value(f) = callee {
                out.push(&**f);
            }
            out
        }
        Expr::Closure { body, .. } => vec![&**body],
        Expr::Prim { args, .. } => args.iter().collect(),
        Expr::Record(fields) => fields.iter().map(|(_, f)| f).collect(),
        Expr::Field { value, .. }
        | Expr::PublishView { value }
        | Expr::WriteWorkspace { value, .. }
        | Expr::UseExact { value, .. }
        | Expr::FileWrite { value, .. } => vec![&**value],
        Expr::Propose { word, target } => vec![&**word, &**target],
        Expr::ReadReceipt { receipt, .. } => vec![&**receipt],
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

    /// A value known only by its declared type. A callable's needs are then its bound: whoever supplied it was
    /// checked against that bound.
    fn declared(q: &QTy) -> Val {
        let callable = match &q.ty {
            Ty::Fn(f) => Some(Summary::of_bound(&f.effects)),
            _ => None,
        };
        Val { ty: q.ty.clone(), qual: declared_qual(q), callable, fields: None }
    }

    /// Field `i` of a record value as the record holds it, before projection's qualifier rule.
    fn field(&self, i: usize) -> Val {
        match (&self.fields, &self.ty) {
            (Some(fields), _) => fields[i].clone(),
            (None, Ty::Record(declared)) => Val::declared(&declared[i].1),
            _ => unreachable!("a field of a {:?} value", self.ty),
        }
    }

    /// What display branches select is selected under approximate control at every depth, and a callable selected
    /// so is unknown (LG2.20).
    fn steered(self) -> Val {
        let fields = match &self.ty {
            Ty::Record(declared) => Some((0..declared.len()).map(|i| self.field(i).steered()).collect()),
            _ => None,
        };
        Val { ty: self.ty, qual: Qual::Display { control: true }, callable: None, fields }
    }
}

/// One of two values: the qualifiers join, a callable needs what either needs, and record fields merge one by one.
/// The type is the common type, or the first value's where the checker cannot tell (it does not unify type
/// variables in M1).
fn merge(a: Val, b: Val) -> Val {
    let ty = widen(&a.ty, &b.ty).unwrap_or_else(|| a.ty.clone());
    let fields = match (&a.ty, &b.ty) {
        (Ty::Record(x), Ty::Record(y)) if x.len() == y.len() && (a.fields.is_some() || b.fields.is_some()) => {
            Some((0..x.len()).map(|i| merge(a.field(i), b.field(i))).collect())
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
    Val { ty, qual: a.qual.join(b.qual), callable, fields }
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

/// The qualifier a display instance adds to a call (LG2.19): a display value for a generic parameter that is not
/// itself a display input makes the callee's body a display body. `except` leaves out the parameters of the
/// result's own type variable, whose qualifier the result keeps as it is.
fn instance(params: &[QTy], vals: &[Val], except: Option<&str>) -> Qual {
    params
        .iter()
        .zip(vals)
        .filter(|(p, _)| !matches!(p.ty, Ty::Fn(_)) && generic(&p.ty) && declared_qual(p) == Qual::Exact)
        .filter(|(p, _)| !matches!((&p.ty, except), (Ty::Var(n), Some(e)) if n == e))
        .fold(Qual::Exact, |q, (_, v)| q.join(v.qual.container()))
}

/// The value of a call. A generic result keeps the qualifier, callable and fields of every argument bound to its
/// type variable; a display instance and a display-qualified callable give a display-qualified result (LG2.19).
fn result_val(result: &QTy, params: &[QTy], vals: &[Val], callee: Qual) -> Val {
    let own = match &result.ty {
        Ty::Var(name) => Some(name.as_str()),
        _ => None,
    };
    let mut bound = Vec::new();
    for (p, v) in params.iter().zip(vals) {
        bindings(p, v, &mut bound);
    }
    let returned = own.and_then(|name| bound.into_iter().filter(|(n, _)| n == name).map(|(_, v)| v).reduce(merge));
    let mut v = returned.unwrap_or_else(|| Val::declared(result));
    if result.display {
        v.qual = v.qual.join(Qual::Display { control: false });
    }
    v.qual = v.qual.join(instance(params, vals, own)).join(callee.container());
    v
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
    emitting: bool,
    /// How many display contexts enclose the current expression: display branches, and the bodies of functions and
    /// closures with an approximate or display input or capture (LG2.19, LG2.20).
    display: u32,
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
        Ok(Checker {
            program,
            modules,
            functions,
            table,
            emitting: false,
            display: 0,
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
                    let s = self.function(mi, fi)?;
                    if s != self.table[mi][fi] {
                        self.table[mi][fi] = s;
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
                            note: format!(
                                "{cap:?} access is never granted in 1.0, even when a manifest requests it (LG3.8)"
                            ),
                        },
                    });
                }
            }
            for fi in 0..module.functions.len() {
                let s = self.function(mi, fi)?;
                if s != self.table[mi][fi] {
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
    /// exact is `exact_required`.
    fn exact_use(&mut self, site: Site, v: &Val, what: &str) {
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
        self.callables(slot, v, what);
        self.qualifiers(slot, v, what, own);
    }

    /// Every callable in the value, itself or in a record field at any depth, stays within its slot's bound.
    fn callables(&mut self, slot: &QTy, v: &Val, what: &str) {
        match &slot.ty {
            Ty::Fn(fty) => match &v.callable {
                None => self.effect(format!("{what}: an unknown callable is neither exact nor pure (LG2.19, LG3.2)")),
                // A bound without effect variables admits no exact sink: the callee may call it under approximate
                // control (LG2.20).
                Some(callable)
                    if fty.effects.vars.is_empty()
                        && !(callable.within(&fty.effects) && callable.exact_sinks.is_empty()) =>
                {
                    self.effect(format!("{what}: the callable needs {}, beyond its bound (LG3.2)", describe(callable)))
                }
                Some(_) => {}
            },
            Ty::Record(fields) => {
                for (i, (name, f)) in fields.iter().enumerate() {
                    self.callables(f, &v.field(i), &format!("{what}, field `{name}`"));
                }
            }
            _ => {}
        }
    }

    /// The value keeps its qualifiers in the slot: as a whole, in a callable's signature and in each record field.
    /// A value refused as a whole is reported once.
    fn qualifiers(&mut self, slot: &QTy, v: &Val, what: &str, own: bool) {
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

    fn arguments(&mut self, params: &[QTy], vals: &[Val], what: &str) -> Result<(), IrError> {
        if params.len() != vals.len() {
            return Err(self.err(format!("{what} takes {} arguments, not {}", params.len(), vals.len())));
        }
        for (i, (slot, v)) in params.iter().zip(vals).enumerate() {
            if !fits(&slot.ty, &v.ty) {
                return Err(self.err(format!("argument {i} of {what}: {:?} does not fit {:?}", v.ty, slot.ty)));
            }
            self.at(i, |c| c.slot(slot, v, &format!("argument {i} of {what}"), false));
        }
        Ok(())
    }

    /// A call: the arguments enter their slots, the callee's needs are instantiated and checked against the context,
    /// and the result follows `result_val` (LG2.19, LG3.2). `summary` is `None` for an unknown callable.
    fn call(
        &mut self,
        params: &[QTy],
        result: &QTy,
        summary: Option<&Summary>,
        vals: &[Val],
        callee: Qual,
        what: &str,
    ) -> Result<(Val, Summary), IrError> {
        self.arguments(params, vals, what)?;
        let needs = match summary {
            Some(summary) => {
                let needs = substitute(summary, params, vals);
                let call = format!("the call of {what}");
                if instance(params, vals, None) == Qual::Exact {
                    self.leaf(&needs, &call);
                } else {
                    // A display value for a generic parameter makes the callee's body a display body, which is pure
                    // (LG2.19, LG2.20).
                    let call = format!("{call}, whose type variable takes a display value,");
                    self.in_display(|c| c.leaf(&needs, &call));
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
        Ok((result_val(result, params, vals, callee), needs))
    }

    /// A function's or closure's result. An approximate or display input requires a display-qualified result (LG2.19).
    fn result(&mut self, slot: &QTy, v: &Val, display_input: Option<&Ty>, what: &str) -> Result<(), IrError> {
        if !fits(&slot.ty, &v.ty) {
            return Err(self.err(format!("{what}: {:?} does not fit {:?}", v.ty, slot.ty)));
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
        Ok(())
    }

    fn join(&self, a: Val, b: Val, what: &str) -> Result<Val, IrError> {
        if widen(&a.ty, &b.ty).is_none() {
            return Err(self.err(format!("{what} have types {:?} and {:?}", a.ty, b.ty)));
        }
        Ok(merge(a, b))
    }

    fn function(&mut self, mi: usize, fi: usize) -> Result<Summary, IrError> {
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
        let walked = self.expr(&f.body, &mut env);
        self.display = 0;
        let (v, s) = walked?;
        self.result(&f.result, &v, display_input, "the result")?;
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
        Ok(s)
    }

    fn expr(&mut self, e: &'p Expr, env: &mut Env) -> Result<(Val, Summary), IrError> {
        match e {
            Expr::Lit(lit) => Ok((Val::exact(lit_type(lit)), Summary::default())),
            Expr::Var(name) => match env.get(name) {
                Some(v) => Ok((v.clone(), Summary::default())),
                None => Err(self.err(format!("unbound name `{name}`"))),
            },
            Expr::FnRef { module, name } => {
                let (mi, fi) = self.resolve(module.as_deref(), name)?;
                let program = self.program;
                let f = &program.modules[mi].functions[fi];
                let summary = self.table[mi][fi].clone();
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
                if !matches!(cv.ty, Ty::Bool | Ty::ApproxBool) {
                    return Err(self.err(format!("an if condition of type {:?}", cv.ty)));
                }
                self.at(0, |c| c.exact_use(Site::Control, &cv, "an ordinary if condition"));
                let (tv, ts) = self.at(1, |c| c.expr(then, env))?;
                let (ev, es) = self.at(2, |c| c.expr(els, env))?;
                s.add(&ts);
                s.add(&es);
                Ok((self.join(tv, ev, "the branches of an if")?, s))
            }
            Expr::Match { scrutinee, arms, default } => {
                let (sv, mut s) = self.at(0, |c| c.expr(scrutinee, env))?;
                self.at(0, |c| c.exact_use(Site::Control, &sv, "a match scrutinee"));
                let mut out: Option<Val> = None;
                for (i, (lit, arm)) in arms.iter().enumerate() {
                    if !fits(&lit_type(lit), &sv.ty) {
                        return Err(self.err(format!("a {:?} pattern on a {:?} scrutinee", lit_type(lit), sv.ty)));
                    }
                    let (av, a_s) = self.at(i + 1, |c| c.expr(arm, env))?;
                    s.add(&a_s);
                    out = Some(match out {
                        None => av,
                        Some(o) => self.join(o, av, "the arms of a match")?,
                    });
                }
                let (dv, ds) = self.at(arms.len() + 1, |c| c.expr(default, env))?;
                s.add(&ds);
                let v = match out {
                    None => dv,
                    Some(o) => self.join(o, dv, "the arms of a match")?,
                };
                Ok((v, s))
            }
            Expr::DisplayIf { cond, then, els } => {
                let (cv, mut s) = self.at(0, |c| c.expr(cond, env))?;
                if cv.ty != Ty::ApproxBool {
                    return Err(self.err(format!("display_if takes an ApproxBool condition, not {:?} (LG2.20)", cv.ty)));
                }
                // The branches are lazy pure display expressions (LG2.20).
                let (tv, ts) = self.at(1, |c| c.in_display(|c| c.expr(then, env)))?;
                let (ev, es) = self.at(2, |c| c.in_display(|c| c.expr(els, env)))?;
                let v = self.join(tv, ev, "display branches (LG2.20)")?;
                s.add(&ts);
                s.add(&es);
                Ok((v.steered(), s))
            }
            Expr::Call { callee, args } => {
                let mut s = Summary::default();
                let mut vals = Vec::with_capacity(args.len());
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
                        let summary = self.table[mi][fi].clone();
                        let what = format!("`{name}`");
                        let (v, needs) = self.call(&params, &f.result, Some(&summary), &vals, Qual::Exact, &what)?;
                        s.add(&needs);
                        Ok((v, s))
                    }
                    Callee::Value(fe) => {
                        let (fv, fs) = self.at(args.len(), |c| c.expr(fe, env))?;
                        s.add(&fs);
                        let fty = match &fv.ty {
                            Ty::Fn(fty) => (**fty).clone(),
                            other => return Err(self.err(format!("a call of a {other:?} value"))),
                        };
                        let callable = fv.callable.as_ref();
                        let (v, needs) =
                            self.call(&fty.params, &fty.result, callable, &vals, fv.qual, "a function value")?;
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
                // A closure capturing display data is display-qualified as a whole (LG2.19).
                let captured = inner.vars.iter().fold(Qual::Exact, |q, (_, v)| q.join(v.qual.container()));
                let display_input = params
                    .iter()
                    .find(|p| declared_qual(&p.ty) != Qual::Exact)
                    .map(|p| p.ty.ty.clone())
                    .or_else(|| inner.vars.iter().find(|(_, v)| v.qual != Qual::Exact).map(|(_, v)| v.ty.clone()));
                for p in params {
                    inner.vars.push((p.name.clone(), Val::declared(&p.ty)));
                }
                // An approximate or display input or capture makes the body a pure display body (LG2.19).
                let (bv, bs) = if display_input.is_some() {
                    self.at(0, |c| c.in_display(|c| c.expr(body, &mut inner)))?
                } else {
                    self.at(0, |c| c.expr(body, &mut inner))?
                };
                self.result(result, &bv, display_input.as_ref(), "the closure's result")?;
                // With an exact result slot a display capture was refused above; the value carries no second finding.
                let qual = if exact_slot(result, true) { Qual::Exact } else { captured };
                let ty = Ty::Fn(Box::new(FnTy {
                    params: params.iter().map(|p| p.ty.clone()).collect(),
                    result: result.clone(),
                    effects: bs.bound(),
                }));
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
                let index = index.ok_or_else(|| self.err(format!("no field `{field}` on a {:?}", rv.ty)))?;
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
            Expr::ReadSnapshot { ty, .. } => {
                let mut s = Summary::default();
                let mut own = needs(&[Effect::ReadSnapshot], &[]);
                own.state_reads.insert(StateRead::Snapshot);
                self.operation(&mut s, own, "a snapshot read");
                Ok((Val::declared(&QTy { ty: ty.clone(), display: false }), s))
            }
            Expr::PublishView { value } => {
                let (_, mut s) = self.at(0, |c| c.expr(value, env))?;
                self.operation(&mut s, needs(&[Effect::ChangeView], &[]), "publish_view");
                Ok((Val::exact(Ty::Unit), s))
            }
            Expr::WriteWorkspace { ty, value, .. } => {
                let (v, mut s) = self.at(0, |c| c.expr(value, env))?;
                if !fits(&ty.ty, &v.ty) {
                    return Err(self.err(format!("a workspace field of type {:?} given a {:?}", ty.ty, v.ty)));
                }
                // Display values may be stored only in typed display fields (LG2.19).
                self.at(0, |c| c.slot(ty, &v, "a workspace field", true));
                self.operation(&mut s, needs(&[Effect::ChangeWorkspace], &[]), "a workspace write");
                Ok((Val::exact(Ty::Unit), s))
            }
            Expr::Propose { word, target } => {
                let (wv, mut s) = self.at(0, |c| c.expr(word, env))?;
                let (tv, t_s) = self.at(1, |c| c.expr(target, env))?;
                s.add(&t_s);
                if wv.ty != Ty::Word || tv.ty != Ty::Target {
                    return Err(self.err(format!("propose takes a Word and a Target, not {:?} and {:?}", wv.ty, tv.ty)));
                }
                self.at(0, |c| c.exact_use(Site::Boundary, &wv, "a preview's word"));
                self.at(1, |c| c.exact_use(Site::Boundary, &tv, "a preview's target"));
                self.operation(&mut s, needs(&[Effect::PreparePreview], &[]), "a preview");
                Ok((Val::exact(Ty::Receipt), s))
            }
            Expr::ReadReceipt { receipt, field } => {
                let (rv, mut s) = self.at(0, |c| c.expr(receipt, env))?;
                if rv.ty != Ty::Receipt {
                    return Err(self.err(format!("a receipt field read on a {:?}", rv.ty)));
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
                Ok((Val::new(ty, rv.qual), s))
            }
            Expr::UseExact { role, value } => {
                let (v, mut s) = self.at(0, |c| c.expr(value, env))?;
                let site = match role {
                    ExactRole::Budget => {
                        if !is_numeric(&v.ty) && !matches!(v.ty, Ty::Approx(_)) {
                            return Err(self.err(format!("a budget of type {:?}", v.ty)));
                        }
                        Site::Operand
                    }
                    ExactRole::Goal | ExactRole::Evidence => Site::Boundary,
                };
                let what = format!("an exact {role:?}");
                if self.display > 0 {
                    // Even an exact literal computed in a display context cannot escape as an exact result (LG2.20).
                    let note = format!("{what} set under approximate control (LG2.20, CT-LG2-05)");
                    self.push(EXACTNESS_VIOLATION, Some((type_name(&v.ty), true, false)), note);
                } else {
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
        match op {
            Prim::ExactAdd | Prim::ExactLess => {
                for (i, v) in vals.iter().enumerate() {
                    if !is_numeric(&v.ty) && !matches!(v.ty, Ty::Approx(_)) {
                        return Err(self.err(format!("{op:?} on a {:?}", v.ty)));
                    }
                    self.at(i, |c| c.exact_use(Site::Operand, v, &format!("an operand of {op:?}")));
                }
                if op == Prim::ExactLess {
                    return Ok(Val::exact(Ty::Bool));
                }
                let exact: Vec<&Ty> = vals.iter().map(|v| &v.ty).filter(|t| is_numeric(t)).collect();
                let ty = if exact.is_empty() || exact.iter().any(|t| **t == Ty::Int) {
                    Ty::Int
                } else if exact.iter().any(|t| **t == Ty::Budget) {
                    Ty::Budget
                } else {
                    Ty::Nat
                };
                Ok(Val::exact(ty))
            }
            Prim::ExactEq => {
                if !fits(&vals[0].ty, &vals[1].ty) && !fits(&vals[1].ty, &vals[0].ty) {
                    return Err(self.err(format!("an equality of {:?} and {:?}", vals[0].ty, vals[1].ty)));
                }
                for (i, v) in vals.iter().enumerate() {
                    self.at(i, |c| c.exact_use(Site::Operand, v, "an operand of an exact equality"));
                }
                Ok(Val::exact(Ty::Bool))
            }
            Prim::And | Prim::Or | Prim::Not => {
                for (i, v) in vals.iter().enumerate() {
                    if !matches!(v.ty, Ty::Bool | Ty::ApproxBool) {
                        return Err(self.err(format!("{op:?} on a {:?}", v.ty)));
                    }
                    self.at(i, |c| c.exact_use(Site::Control, v, "a short-circuit exact boolean"));
                }
                Ok(Val::exact(Ty::Bool))
            }
            Prim::ToApprox(p) => {
                let v = &vals[0];
                if !is_numeric(&v.ty) {
                    return Err(self.err(format!("an approximate conversion of a {:?}", v.ty)));
                }
                Ok(Val::new(Ty::Approx(p), Qual::Approx.join(v.qual)))
            }
            Prim::ToExact => {
                let v = &vals[0];
                if !is_numeric(&v.ty) && !matches!(v.ty, Ty::Approx(_)) {
                    return Err(self.err(format!("an exact conversion of a {:?}", v.ty)));
                }
                self.at(0, |c| c.exact_use(Site::Convert, v, "a conversion back to exact"));
                Ok(Val::exact(Ty::Int))
            }
            Prim::ApproxAdd | Prim::ApproxLess => {
                let p = match (&vals[0].ty, &vals[1].ty) {
                    (Ty::Approx(p), Ty::Approx(q)) if p == q => *p,
                    (a, b) => {
                        return Err(self.err(format!("{op:?} takes two Approx of one precision, not {a:?} and {b:?}")))
                    }
                };
                let q = vals[0].qual.join(vals[1].qual);
                Ok(if op == Prim::ApproxAdd {
                    Val::new(Ty::Approx(p), q)
                } else {
                    Val::new(Ty::ApproxBool, cond_of(q))
                })
            }
            Prim::ApproxAnd | Prim::ApproxNot => {
                let mut q = Qual::Cond;
                for v in vals {
                    if v.ty != Ty::ApproxBool {
                        return Err(self.err(format!("{op:?} on a {:?}", v.ty)));
                    }
                    q = q.join(v.qual);
                }
                Ok(Val::new(Ty::ApproxBool, cond_of(q)))
            }
        }
    }
}
