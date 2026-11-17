//! The 600 Cell Script checker over the typed IR draft `c600-ir-draft-1` (LP2, milestone M1).
//!
//! The checker implements form 1 of the exactness contract (LG2.19, LG2.20, LG2.22), effect inference and the view
//! rules (LG3.1, LG3.2, LG3.4 to LG3.6), and capabilities, grants and capability separation (LG3.8, LG3.9, LG3.13)
//! with chapter 07's interim grant table. It reports chapter 02's frozen codes and, by the owner's decision below,
//! chapter 13's `LG3:capability.ambient_read`, and no others.
//!
//! - Owner decisions of 16 November 2026 under PHS.52 (`docs/wiki/decisions/owner-decisions-2026-11-16-phs52.md`): a
//!   clock or a network capability, and a file capability where the entry may read a file, that the run does not hold
//!   or that 1.0 never grants is `LG3:capability.ambient_read`; randomness, process and session-file access and a file
//!   that is only written stay `LG3:capability.not_granted` ([`Summary::ungranted`]). A call that resolves to no
//!   checked function is dynamic code, `LG3:capability.not_granted` at the call (DET.36, RTE.45). A goal or an
//!   evidence sink takes portable data only: a value whose type is or holds a callable, a receipt, a word equality key
//!   or a compressed word, or is a type variable, is `LG2:type.mismatch`.
//! - A value whose base type does not fit its use is `LG2:type.mismatch` (LG2.1), with the expected and the supplied
//!   type in [`Mismatch`]. The check goes on with a value of unknown type, which fits every use and gives no finding of
//!   its own, so one fault gives one finding. The exactness codes apply where the base type fits (LG2.22). A function
//!   value called with another number of arguments is a mismatch too: the count is part of its type.
//! - A fault with no source diagnostic is an [`IrError`]: a module or function defined twice, an unbound name, an
//!   unknown or unexported function, a named function called with a wrong number of arguments, or effect inference
//!   that does not converge. Lowering refuses all but the last before the checker sees them (`rust/README.md`).
//! - An end test's verdict (LB3.2; LP2 draft: `RetainedCheckpointVerdict`, `LabelCheckpointVerdict`,
//!   `StickerTestVerdict`) is used through a `match` on its outcomes `holds`, `fails` and `unestablished` only (LG2.9).
//!   A verdict in a boolean condition or given for a `Bool` at any depth, an exact equality of verdicts, a read of an
//!   end test's row declared `Bool`, and a match whose `_` arm takes `unestablished` together with another outcome are
//!   `LG2:type.verdict_required`. A read of an end test's row declared as any other type, another test's verdict
//!   included, is `LG2:type.mismatch`.
//! - The flow rule (BND.33) stays off until LQ2 sets it, so the checker never emits `LG3:capability.flow_refused`
//!   (CT-BND-24, CT-ERR-15). [`separation`] computes how capability separation sees a program (BND.41).
//! - Static success never replaces the engine's run-time checks (LG3.10).

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use c600_ir::{
    Accounting, Callee, Capability, Effect, EffectBound, EndTest, EndTestOutcome, ExactRole, Expr, FnKind, FnTy,
    Function, Lit, Prim, Program, QTy, ReceiptField, Ty, EXEC_CONTRACT, FORM2_EXEC_CONTRACT, IR_VERSION,
};

/// `LG2:type.mismatch` (LG2.1).
pub const TYPE_MISMATCH: &str = "LG2:type.mismatch";
/// `LG2:type.exactness_violation` (LG2.19, LG2.20, LG2.22).
pub const EXACTNESS_VIOLATION: &str = "LG2:type.exactness_violation";
/// `LG2:type.exact_required` (LG2.22).
pub const EXACT_REQUIRED: &str = "LG2:type.exact_required";
/// `LG2:type.word_key_not_executable` (LG2.13, LG2.17).
pub const WORD_KEY_NOT_EXECUTABLE: &str = "LG2:type.word_key_not_executable";
/// `LG2:type.verdict_required` (LG2.9; LP2 draft for LB3.2's end tests): an end test's verdict used as a boolean (in
/// a condition, for a `Bool`, read from its row as a `Bool`), compared by exact equality, or matched with a `_` arm
/// that takes `unestablished` together with another outcome.
pub const VERDICT_REQUIRED: &str = "LG2:type.verdict_required";
/// `LB1:input.incompatible_alphabet` (LB1.3, LB1.8; LP2 draft): a menu word where a Full word is expected, or the
/// reverse.
pub const INCOMPATIBLE_ALPHABET: &str = "LB1:input.incompatible_alphabet";
/// `LG3:effect.undeclared` (LG3.2, LG3.5, LG3.6).
pub const EFFECT_UNDECLARED: &str = "LG3:effect.undeclared";
/// `LG3:capability.not_granted` (LG3.8, LG3.9; chapter 07 section 8).
pub const NOT_GRANTED: &str = "LG3:capability.not_granted";
/// `LG3:capability.ambient_read` (INV-10; LX-3): a clock, file or network read requested without a grant (chapter 13;
/// owner decision of 16 November 2026 under PHS.52).
pub const AMBIENT_READ: &str = "LG3:capability.ambient_read";
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
    /// The types of an `LG2:type.mismatch`, and of a code that refines one where a type is expected: a word equality
    /// key for literal letters, a word of the other alphabet, an end test's verdict used as a boolean. `None` for
    /// every other finding.
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

/// The three profiles of LG3.9. In [`admit`], `Solving` is the column of a solver's own, forked or imported tool; a
/// built-in tool in the solving profile keeps its reviewed class and is admitted by [`admit_built_in`].
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

/// One row of chapter 07's grant table that a host-reaching operation uses, named by the operation's literal data:
/// the rows a built-in tool's class lists (BND.57). Draft naming (LP2): the IR draft has no class table, no `derive`
/// operation and no display plan, so a view change is one row, and the derived-state row arises only inside the two
/// exceptions, as [`Row::RepairList`] and [`Row::SetupSearch`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Row {
    /// A snapshot read of the query (snapshot read row).
    Read(String),
    /// A read of a preview outcome (BND.42).
    Outcome,
    /// A read through the fixed declassifier (LG3.11, BND.53).
    Declassified(String),
    /// A published view (view change row).
    View,
    /// A write of the workspace record (workspace change row).
    Workspace(String),
    /// A proposal to the literal target (prepare a preview row).
    Preview(String),
    /// A row the checker cannot name: a proposal whose target is not a literal, or a call of a callable known only by
    /// a bound that allows a snapshot read, a view or workspace change or a preview.
    Unknown,
    /// The derived-state row of the repair-list exception (BND.2; chapter 07 section 8, built-in column; LP2 draft):
    /// the pairs at a cap the solver names, read as the engine row `query` for the cap (the IR draft has no `derive`).
    /// Only a class lists it. A summary reports such a read as [`Row::Read`], which this row covers (see
    /// [`class_covers`]), and [`admit_built_in`] then checks every read of `query` under the exception's rules.
    RepairList(String),
    /// The derived-state row of the setup-search exception (BND.1; LP2 draft): the setups that keep the buffers fixed,
    /// for a target the solver names, read as the engine row `query` for the target. As [`Row::RepairList`].
    SetupSearch(String),
}

/// Whether the rows a built-in tool's class lists cover a row an entry may use (BND.57): the row itself, or for a read
/// of a query, the derived-state row of one of the two exceptions on that query ([`Row::RepairList`],
/// [`Row::SetupSearch`]). [`admit_built_in`] applies it to the entry's rows.
pub fn class_covers(class_rows: &BTreeSet<Row>, row: &Row) -> bool {
    let Row::Read(query) = row else { return class_rows.contains(row) };
    let exceptions = [Row::RepairList(query.clone()), Row::SetupSearch(query.clone())];
    class_rows.contains(row) || exceptions.iter().any(|x| class_rows.contains(x))
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
    /// The named functions, by module and function name, that evaluating this may call directly, through a function
    /// reference or through a callable it passes to a callee. Recursion control reads it (LG2.20).
    pub reaches: BTreeSet<(String, String)>,
    /// The grant-table rows that evaluating this may use (BND.57). Open effect variables and unresolved calls add none:
    /// they stay in `vars` and `unresolved`.
    pub rows: BTreeSet<Row>,
    /// Evaluating this may read a file: a file read, or a call of a callable whose bound holds the file capability.
    reads_file: bool,
    /// On a callable: it is a view, so whatever its effect variables take, it changes no workspace and prepares no
    /// preview (LG3.5, LG3.6). Needs that a caller accumulates do not inherit it.
    view: bool,
    /// On a callable: it computes or selects its result under approximate control (LG2.20, LG2.22), as a named
    /// function's control bit says. Needs that a caller accumulates do not inherit it.
    control: bool,
    /// The paths, in the function's own body, of the views that publish a value of a word-carrying type (LB3.5; LP2
    /// draft). Only a report's summary of a function holds them; needs that a caller accumulates do not inherit them.
    word_views: BTreeSet<String>,
    /// Evaluating this may spend a budget that exact inputs alone do not fix: a `display_if`, a display accounting
    /// read, a clock read, or a call of a function value, which the draft does not follow. No child scope's body may,
    /// since the child's outcome continues into exact computation (LG2.20, DET.60; LP2 draft).
    display_cost: bool,
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
        self.rows.extend(other.rows.iter().cloned());
        self.reads_file |= other.reads_file;
        self.display_cost |= other.display_cost;
    }

    /// The code that refuses the capability `cap` when the run does not hold it or 1.0 never grants it (LG3.8, LG3.9):
    /// `LG3:capability.ambient_read` for a clock or a network capability, and for a file capability when evaluating
    /// this may read a file; `LG3:capability.not_granted` otherwise (chapter 13; owner decision of 16 November 2026
    /// under PHS.52).
    pub fn ungranted(&self, cap: Capability) -> &'static str {
        ungranted(cap, self.reads_file)
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
            reads_file: bound.capabilities.contains(&Capability::File),
            ..Summary::default()
        };
        if s.effects.contains(&Effect::ReadSnapshot) {
            s.state_reads.insert(StateRead::Callable);
        }
        let rowed = [Effect::ReadSnapshot, Effect::ChangeView, Effect::ChangeWorkspace, Effect::PreparePreview];
        if rowed.iter().any(|e| s.effects.contains(e)) {
            s.rows.insert(Row::Unknown);
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
        let note = if NEVER_GRANTED.contains(cap) {
            format!("{cap:?} access is never granted in 1.0 (LG3.8)")
        } else if !grant.capabilities.contains(cap) {
            format!("the run grants no {cap:?} capability; a declaration or an import supplies none (LG3.9)")
        } else {
            continue;
        };
        let mut d = refuse(note);
        d.code = summary.ungranted(*cap);
        diagnostics.push(d);
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
        // A workspace change occurs only inside a command the solver invokes (LG3.5; chapter 07 section 8, own
        // column). A view entry's effect is reported at the view.
        let f =
            entry_fn(program, module, function).ok_or_else(|| IrError(format!("no function `{module}.{function}`")))?;
        if f.kind == FnKind::Helper && summary.effects.contains(&Effect::ChangeWorkspace) {
            let mut d = refuse(
                "the entry is no command, and a workspace change occurs only inside a command the solver invokes \
                 (LG3.5)"
                    .into(),
            );
            d.code = EFFECT_UNDECLARED;
            diagnostics.push(d);
        }
    }
    Ok(Admission { diagnostics, separation, open_cells })
}

fn entry_fn<'p>(program: &'p Program, module: &str, function: &str) -> Option<&'p Function> {
    program.modules.iter().filter(|m| m.name == module).flat_map(|m| &m.functions).find(|f| f.name == function)
}

/// Check a program and admit one entry of a built-in tool for a run in the solving profile: the built-in column of
/// chapter 07 section 8 (draft, LP2; LP3 asked for it). `class_rows` are the rows the tool's reviewed class lists
/// (BND.37, BND.57); the IR has no manifest, so the caller supplies them. In the tool-authoring and research
/// profiles [`admit`] applies to every author.
///
/// The admission keeps the checker's diagnostics. It refuses with `LG3:effect.undeclared` at path `entry` an entry
/// that is no command and changes the workspace or prepares a preview (LG3.5, LG3.6; a view's own effect is reported
/// at the view), and with `LG3:capability.not_granted`:
///
/// - at `entry`: an entry that is not exported; a capability that is never granted (LG3.8) or that `grant` does not
///   hold (LG3.9), which is `LG3:capability.ambient_read` where [`Summary::ungranted`] says so; an entry that both
///   reads state-derived data and prepares previews (LG3.13, BND.39); a row the entry may use that `class_rows` does
///   not list, and a row the checker cannot name (`Row::Unknown`, an unresolved call or an open effect variable),
///   because a built-in tool's rows are listed by its class (BND.57); in a function the entry reaches, because the
///   solver names a word or a cap to the entry and never to a function it reaches: a preview, a word-carrying
///   workspace write, a snapshot read whose argument is not a literal, and, when the entry reads state-derived data,
///   a view that publishes a word-carrying value;
/// - at the proposal's or write's path: a proposal whose word, or a workspace write of a word-carrying type whose
///   value, is not a `Word` or `MenuWord` parameter of the entry as given, unshadowed (a word the solver named), or
///   for a write a read through a declassifier the policy enables (chapter 07 section 8, built-in column);
/// - shown facts (LB3.5, DT-J1 rule 6; LP2 draft): when the entry reads state-derived data, at the path of each
///   expression its body may return, if its result type is word-carrying, and of each view that publishes a
///   word-carrying value, unless it is such a parameter as given, so that no word the method derived or selected from
///   state leaves an inspector; and at a snapshot read's path, when its argument is not a parameter of the entry as
///   given, a literal, or a record or a collection built from those alone, so that nothing is read for an unnamed
///   cap or for a cap or a word the method computed.
///
/// A type is word-carrying when it is `Word`, a compressed word, a word equality key, a menu word, a type variable
/// (which may stand for a word), or a record, a list, a set, a map or a callable type holding one in a field, an
/// element, a key, a value, a parameter or the result. The rules are conservative: a callee that proposes, writes or
/// publishes the word the solver named is refused, as is a write, a view or a result of a record, a collection or a
/// closure built from named words, and a callee's snapshot read for its own parameter.
///
/// The column's two exceptions (BND.1, BND.2, BND.69; LP2 draft): a class that lists [`Row::RepairList`] or
/// [`Row::SetupSearch`] for a query may read that query's derived state, and every read of it, also one the class
/// lists as [`Row::Read`], follows the exception's rules, refused with `LG3:capability.not_granted`:
///
/// - named: the read is in the entry's own body, for a parameter of the entry as given, the cap or the target the
///   solver named (BND.2: the program does not choose the cap; BND.1; BND.64: no job for an unnamed cap). A read with
///   no argument or for any other argument is refused at the read, and a function the entry reaches that reads the
///   query is refused at `entry`;
/// - as given: the read's value is only delivered as read, at an expression the entry's body may return, as the value
///   of a view, or bound by `let` to a name used only so. Any other use is refused at the use: an operation, a call, a
///   field, a record or a collection holding it, a condition or a match on it, a write, a proposal, a closure's result.
///   So the list keeps the engine's neutral order and marks no row best, and no entry tests a list for emptiness to
///   skip to another cap (BND.2, BND.64); the solver chooses among the setups (BND.1). A branch of an `if`, a
///   `display_if` or a `match` delivers to the result only when its condition reads nothing but literals and the
///   entry's parameters as given: otherwise the program would choose the cap or the target, or whether to show the
///   value (BND.2, BND.64). A branch inside a view's or a `let`'s value is a use;
/// - shown facts: a value so delivered is the exception's output, so the shown-facts rule above does not refuse it.
///   Previews keep their rules: an entry that reads the list or the setups reads state and prepares no preview, and a
///   preview is of a word the solver named, in a command the solver invokes.
///
/// A class that lists both rows for one query is refused at `entry`: each exception reads a query of its own (BND.69).
///
/// Not built: the derived-state row outside the two exceptions (`derive`: the IR draft has no such operation, ONE.23),
/// the rest of rule 6's check of the facts a view shows (a selection that is not word-carrying, such as a cap the
/// method chose, is not told from a per-cap fact), and the review of the class against the entry (CT-BND-15). What the
/// exceptions also need lies outside the checker: the engine rows' neutral order and buffer-fixing setups, the empty
/// list's wording (BND.2), the shell's keys and the solver's edit of a setup, and a job that names its cap through
/// the command layer. BND.33's flow rule stays LQ2's. The open cells are left empty: they are the own column's.
pub fn admit_built_in(
    program: &Program,
    module: &str,
    function: &str,
    class_rows: &BTreeSet<Row>,
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
    let f = entry_fn(program, module, function).ok_or_else(|| IrError(format!("no function `{module}.{function}`")))?;
    let summary = report
        .summary(module, function)
        .cloned()
        .ok_or_else(|| IrError(format!("no function `{module}.{function}`")))?;
    let Report { mut diagnostics, summaries } = report;
    let mut refuse = |code: &'static str, path: &str, note: String| {
        diagnostics.push(Diagnostic {
            code,
            module: module.to_string(),
            function: function.to_string(),
            path: path.to_string(),
            value_type: None,
            display_control: false,
            qualifier_erasure: false,
            mismatch: None,
            note,
        })
    };
    if f.kind == FnKind::Helper {
        for (e, rule) in [(Effect::ChangeWorkspace, "LG3.5"), (Effect::PreparePreview, "LG3.6")] {
            if summary.effects.contains(&e) {
                let note = format!("the entry is no command, and a {e:?} effect occurs only inside a command the solver invokes ({rule})");
                refuse(EFFECT_UNDECLARED, "entry", note);
            }
        }
    }
    if !f.exported {
        refuse(NOT_GRANTED, "entry", "a run starts at an exported entry".into());
    }
    for cap in &summary.capabilities {
        if NEVER_GRANTED.contains(cap) {
            refuse(summary.ungranted(*cap), "entry", format!("{cap:?} access is never granted in 1.0 (LG3.8)"));
        } else if !grant.capabilities.contains(cap) {
            refuse(summary.ungranted(*cap), "entry", format!("the run grants no {cap:?} capability (LG3.9)"));
        }
    }
    let separation = separation(&summary, policy);
    if separation.reads_state && separation.prepares_previews {
        let note = "a program in the solving profile reads state-derived data or prepares previews, never both \
                    (capability separation, LG3.13, BND.39)";
        refuse(NOT_GRANTED, "entry", note.into());
    }
    for row in &summary.rows {
        if *row == Row::Unknown {
            let note = "the entry may use a row the checker cannot name, and a built-in tool's rows are listed by its \
                        class (BND.57)";
            refuse(NOT_GRANTED, "entry", note.into());
        } else if !class_covers(class_rows, row) {
            refuse(
                NOT_GRANTED,
                "entry",
                format!("the class lists no row {row:?}, and a built-in tool's rows are listed by its class (BND.57)"),
            );
        }
    }
    if !summary.unresolved.is_empty() || !summary.vars.is_empty() {
        let note = "an unresolved call or an open effect variable may use any row, and a built-in tool's rows are \
                    listed by its class (BND.57)";
        refuse(NOT_GRANTED, "entry", note.into());
    }
    // A view of a word-carrying value counts only when the entry reads state-derived data (LB3.5).
    let views = |m: &str, name: &str| -> BTreeSet<String> {
        match summaries.get(&(m.to_string(), name.to_string())) {
            Some(s) if separation.reads_state => s.word_views.clone(),
            _ => BTreeSet::new(),
        }
    };
    let exceptions = Exception::of(class_rows);
    for row in class_rows {
        if let Row::SetupSearch(query) = row {
            if class_rows.contains(&Row::RepairList(query.clone())) {
                let note = format!(
                    "the class lists `{query}` as both the repair list and the setup search, and each exception reads \
                     a query of its own (BND.69)"
                );
                refuse(NOT_GRANTED, "entry", note);
            }
        }
    }
    let none_delivered = BTreeSet::new();
    for (m, name) in &summary.reaches {
        let Some(callee) = entry_fn(program, m, name) else { continue };
        let callee_views = views(m, name);
        let named = Named {
            words: BTreeSet::new(),
            params: BTreeSet::new(),
            views: &callee_views,
            delivered: &none_delivered,
            exceptions: &exceptions,
            policy,
        };
        let found: BTreeSet<Unnamed> = unnamed(&callee.body, &named).into_iter().map(|(_, what)| what).collect();
        let verbs: Vec<&str> = found.iter().filter_map(|what| what.verb()).collect();
        if !verbs.is_empty() {
            refuse(
                NOT_GRANTED,
                "entry",
                format!(
                    "`{m}.{name}` {} a word, and the built-in column admits only a word the solver named to the entry \
                     (chapter 07 section 8, LB3.5)",
                    verbs.join(" or ")
                ),
            );
        }
        if found.contains(&Unnamed::ReadArgument) {
            refuse(
                NOT_GRANTED,
                "entry",
                format!(
                    "`{m}.{name}` reads a row for an argument other than a literal, and a built-in tool reads only for \
                     what the solver named to the entry (LB3.5: no prefetch for an unnamed cap)"
                ),
            );
        }
        if let Some((query, x)) = exception_read(&callee.body, &exceptions) {
            let (named, rule) = x.named();
            refuse(
                NOT_GRANTED,
                "entry",
                format!(
                    "`{m}.{name}` reads `{query}`, {}, and the built-in column reads it only in the entry, for the \
                     {named} the solver named to the entry ({rule}, BND.64)",
                    x.what()
                ),
            );
        }
    }
    let words: BTreeSet<&str> =
        f.params.iter().filter(|p| matches!(p.ty.ty, Ty::Word | Ty::MenuWord)).map(|p| p.name.as_str()).collect();
    let params: BTreeSet<&str> = f.params.iter().map(|p| p.name.as_str()).collect();
    let delivered = deliveries(&f.body, &params, &exceptions);
    for (path, note) in delivered.refusals {
        refuse(NOT_GRANTED, &path, note);
    }
    let entry_views = views(module, function);
    let named = Named {
        words: words.clone(),
        params,
        views: &entry_views,
        delivered: &delivered.views,
        exceptions: &exceptions,
        policy,
    };
    for (path, what) in unnamed(&f.body, &named) {
        refuse(NOT_GRANTED, &path, what.note().into());
    }
    if separation.reads_state && carries_word(&f.result.ty) {
        for path in unnamed_results(&f.body, &words) {
            if delivered.results.contains(&path) {
                continue;
            }
            let note = "the result of an entry that reads state-derived data, of a word-carrying type, other than a \
                        `Word` or `MenuWord` parameter of the entry as given: no word the method derived or selected \
                        from state leaves an inspector (LB3.5, DT-J1 rule 6)";
            refuse(NOT_GRANTED, &path, note.into());
        }
    }
    Ok(Admission { diagnostics, separation, open_cells: Vec::new() })
}

/// What the built-in column checks a body against (chapter 07 section 8; LB3.5): for the entry, the words and the
/// caps the solver named to it; for a function the entry reaches, nothing, since the solver names nothing to it.
struct Named<'a> {
    /// The entry's `Word` parameters and, LP2 draft, its `MenuWord` parameters, which the solver names in the pinned
    /// menu's letters.
    words: BTreeSet<&'a str>,
    /// The entry's parameters of every type.
    params: BTreeSet<&'a str>,
    /// The paths of the body's views that publish a word-carrying value, when the entry reads state-derived data.
    views: &'a BTreeSet<String>,
    /// The paths of the body's views that publish an exception's value as read ([`Deliveries::views`]).
    delivered: &'a BTreeSet<String>,
    /// The exceptions the class grants, by query: their reads follow the exceptions' rules ([`deliveries`]).
    exceptions: &'a BTreeMap<&'a str, Exception>,
    policy: &'a Policy,
}

/// A delivery or a request in a body that is not what the solver named (chapter 07 section 8, built-in column; LB3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Unnamed {
    /// A proposal of a word.
    Preview,
    /// A workspace write of a word-carrying value.
    Write,
    /// A view, at one of [`Named::views`], that publishes a word-carrying value.
    View,
    /// A snapshot read's argument.
    ReadArgument,
}

impl Unnamed {
    /// What a function the entry reaches does with a word, if this is a word's delivery.
    fn verb(self) -> Option<&'static str> {
        match self {
            Unnamed::Preview => Some("proposes"),
            Unnamed::Write => Some("writes"),
            Unnamed::View => Some("publishes"),
            Unnamed::ReadArgument => None,
        }
    }

    /// The refusal's note at the entry's own expression.
    fn note(self) -> &'static str {
        match self {
            Unnamed::Preview => {
                "a preview of a word the solver did not name: the built-in column takes a `Word` or `MenuWord` \
                 parameter of the entry as given (chapter 07 section 8)"
            }
            Unnamed::Write => {
                "a workspace write of a word the solver did not name: the built-in column takes a `Word` or \
                 `MenuWord` parameter of the entry as given, or a read through an enabled declassifier (chapter 07 \
                 section 8)"
            }
            Unnamed::View => {
                "a view of an entry that reads state-derived data publishes a word-carrying value other than a `Word` \
                 or `MenuWord` parameter of the entry as given: no word the method derived or selected from state \
                 leaves an inspector (LB3.5, DT-J1 rule 6)"
            }
            Unnamed::ReadArgument => {
                "a snapshot read for an argument the solver did not name: the built-in column reads for a parameter of \
                 the entry as given, a literal, or a record or a collection built from those alone (LB3.5: no prefetch \
                 for an unnamed cap, no query over a cap or a word the method computed)"
            }
        }
    }
}

/// The proposals, word-carrying workspace writes, views and snapshot reads in `body` that `named` does not allow, with
/// their paths, in body order. A word the solver named is one of `named.words` as given, a parameter that no `let` or
/// closure parameter shadows; a write may also store a read through a declassifier the policy enables, and a view may
/// publish an exception's value as read. A read's argument may be one of `named.params` as given, a literal, or a
/// record or a collection built from those alone; a read of an exception's query is [`deliveries`]' to check.
fn unnamed(body: &Expr, named: &Named) -> Vec<(String, Unnamed)> {
    fn word(e: &Expr, named: &Named, bound: &[String]) -> bool {
        matches!(e, Expr::Var(v) if named.words.contains(v.as_str()) && !bound.contains(v))
    }
    fn given(e: &Expr, named: &Named, bound: &[String]) -> bool {
        match e {
            Expr::Lit(_) => true,
            Expr::Var(v) => named.params.contains(v.as_str()) && !bound.contains(v),
            Expr::Record(fields) => fields.iter().all(|(_, field)| given(field, named, bound)),
            Expr::Collection { items, .. } => items.iter().all(|item| given(item, named, bound)),
            _ => false,
        }
    }
    fn walk(e: &Expr, named: &Named, path: &mut Vec<u32>, bound: &mut Vec<String>, out: &mut Vec<(String, Unnamed)>) {
        let found = match e {
            Expr::Propose { word: w, .. } if !word(w, named, bound) => Some(Unnamed::Preview),
            Expr::WriteWorkspace { ty, value, .. } if carries_word(&ty.ty) => {
                let declassified = matches!(&**value, Expr::Declassified { id, .. }
                    if named.policy.enabled_declassifiers.contains(id));
                (!word(value, named, bound) && !declassified).then_some(Unnamed::Write)
            }
            Expr::PublishView { value } if !named.views.is_empty() && !word(value, named, bound) => {
                let at = path_text(path);
                (named.views.contains(&at) && !named.delivered.contains(&at)).then_some(Unnamed::View)
            }
            Expr::ReadSnapshotWith { query, arg, .. }
                if !named.exceptions.contains_key(query.as_str()) && !given(arg, named, bound) =>
            {
                Some(Unnamed::ReadArgument)
            }
            _ => None,
        };
        if let Some(found) = found {
            out.push((path_text(path), found));
        }
        let (scoped, names): (usize, Vec<String>) = match e {
            Expr::Let { name, .. } => (1, vec![name.clone()]),
            Expr::Closure { params, .. } => (0, params.iter().map(|p| p.name.clone()).collect()),
            Expr::Child { status, .. } => (1, status.iter().cloned().collect()),
            Expr::Repeat { acc, .. } => (2, vec![acc.clone()]),
            _ => (usize::MAX, Vec::new()),
        };
        for (i, child) in e.children().into_iter().enumerate() {
            let enters = i == scoped;
            if enters {
                bound.extend(names.iter().cloned());
            }
            path.push(i as u32);
            walk(child, named, path, bound, out);
            path.pop();
            if enters {
                bound.truncate(bound.len() - names.len());
            }
        }
    }
    let mut out = Vec::new();
    walk(body, named, &mut Vec::new(), &mut Vec::new(), &mut out);
    out
}

/// The paths of the expressions `body` may return that are not one of `words` as given, unshadowed (LB3.5): the rest
/// of each `let`, both branches of an `if` or a `display_if`, every arm of a `match`, its `_` arm included, a child
/// scope's body and stop branch, and a loop's initial value and body.
fn unnamed_results(body: &Expr, words: &BTreeSet<&str>) -> Vec<String> {
    fn walk(e: &Expr, words: &BTreeSet<&str>, path: &mut Vec<u32>, bound: &mut Vec<String>, out: &mut Vec<String>) {
        match e {
            Expr::Let { name, body, .. } => {
                bound.push(name.clone());
                path.push(1);
                walk(body, words, path, bound, out);
                path.pop();
                bound.pop();
            }
            Expr::If { then, els, .. } | Expr::DisplayIf { then, els, .. } => {
                for (i, branch) in [(1, then), (2, els)] {
                    path.push(i);
                    walk(branch, words, path, bound, out);
                    path.pop();
                }
            }
            Expr::Match { arms, default, .. } => {
                for (i, arm) in arms.iter().map(|(_, arm)| arm).chain([&**default]).enumerate() {
                    path.push(i as u32 + 1);
                    walk(arm, words, path, bound, out);
                    path.pop();
                }
            }
            Expr::Child { body, status, otherwise, .. } => {
                path.push(0);
                walk(body, words, path, bound, out);
                path.pop();
                let depth = bound.len();
                bound.extend(status.iter().cloned());
                path.push(1);
                walk(otherwise, words, path, bound, out);
                path.pop();
                bound.truncate(depth);
            }
            // The loop's value is its initial value or its body's last one. The value so far is no word as given,
            // since the body may have computed it.
            Expr::Repeat { init, acc, body, .. } => {
                path.push(1);
                walk(init, words, path, bound, out);
                path.pop();
                bound.push(acc.clone());
                path.push(2);
                walk(body, words, path, bound, out);
                path.pop();
                bound.pop();
            }
            Expr::Var(v) if words.contains(v.as_str()) && !bound.contains(v) => {}
            _ => out.push(path_text(path)),
        }
    }
    let mut out = Vec::new();
    walk(body, words, &mut Vec::new(), &mut Vec::new(), &mut out);
    out
}

/// One of the built-in column's two exceptions (chapter 07 section 8, BND.69), by the class row that grants it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Exception {
    /// BND.2: the pairs at a cap the solver names ([`Row::RepairList`]).
    RepairList,
    /// BND.1: the setups for a target the solver names ([`Row::SetupSearch`]).
    SetupSearch,
}

impl Exception {
    /// The exceptions a class grants, by the query each reads.
    fn of(class_rows: &BTreeSet<Row>) -> BTreeMap<&str, Exception> {
        class_rows
            .iter()
            .filter_map(|row| match row {
                Row::RepairList(query) => Some((query.as_str(), Exception::RepairList)),
                Row::SetupSearch(query) => Some((query.as_str(), Exception::SetupSearch)),
                _ => None,
            })
            .collect()
    }

    fn what(self) -> &'static str {
        match self {
            Exception::RepairList => "the repair list",
            Exception::SetupSearch => "the setup search",
        }
    }

    /// What the solver names to it, and the rule that says so.
    fn named(self) -> (&'static str, &'static str) {
        match self {
            Exception::RepairList => ("cap", "BND.2"),
            Exception::SetupSearch => ("target", "BND.1"),
        }
    }

    /// The refusal's note at a read that is not for what the solver named to the entry.
    fn unnamed_note(self) -> String {
        let (named, rule) = self.named();
        format!(
            "a read of {} for a {named} the solver did not name: the built-in column reads it only in the entry, for \
             the {named} the solver named, a parameter of the entry as given ({rule}, BND.64)",
            self.what()
        )
    }

    /// The refusal's note at a delivery where the program chooses whether it runs: under a branch whose condition
    /// reads more than what the solver named, after a short-circuit operand that does, in a loop's body whose count
    /// does, in a closure's body or in a child scope.
    fn steered_note(self) -> String {
        let (named, rule) = self.named();
        format!(
            "{} delivered where the program chooses whether it runs: where a branch's condition, an earlier \
             short-circuit operand or a loop's count reads more than literals and the entry's parameters as given, in \
             a closure's body or in a child scope: the program would choose the {named} or whether to show it, and \
             the solver names the {named} ({rule}, BND.64)",
            self.what()
        )
    }

    /// The refusal's note at a use of its value other than its delivery as read.
    fn used_note(self) -> String {
        let so = match self {
            Exception::RepairList => {
                "the list keeps its neutral order, marks no pair best and skips no cap (BND.2, BND.64)"
            }
            Exception::SetupSearch => "the solver chooses among the setups (BND.1, BND.64)",
        };
        format!(
            "{} used other than delivered as read: the entry returns it, publishes it as a view or binds it with \
             `let` for that, so {so}",
            self.what()
        )
    }
}

/// The first read in `body`, in body order, of a query one of `exceptions` reads, with its exception.
fn exception_read<'e>(body: &'e Expr, exceptions: &BTreeMap<&str, Exception>) -> Option<(&'e str, Exception)> {
    let here = match body {
        Expr::ReadSnapshot { query, .. } | Expr::ReadSnapshotWith { query, .. } => {
            exceptions.get(query.as_str()).map(|&x| (query.as_str(), x))
        }
        _ => None,
    };
    here.or_else(|| body.children().into_iter().find_map(|child| exception_read(child, exceptions)))
}

/// Where an entry's body delivers an exception's value as read, and the refusals of the exceptions' rules
/// ([`admit_built_in`]).
#[derive(Default)]
struct Deliveries {
    /// The paths of the views that publish an exception's value, which the exceptions' rules check.
    views: BTreeSet<String>,
    /// The paths of the expressions the body may return that are an exception's value, which the exceptions' rules
    /// check; a read for what the solver did not name is refused at the read, once.
    results: BTreeSet<String>,
    /// The refusals, with their paths, in body order.
    refusals: Vec<(String, String)>,
}

/// Where an expression's value goes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    /// The entry's body may return it.
    Result,
    /// A view publishes it.
    View,
    /// A `let` binds it.
    Bound,
    /// Anything else: an operation, a call, a field, a record or a collection, a condition, a write, a proposal or a
    /// closure's result.
    Other,
}

/// The exceptions' rules over an entry's body ([`admit_built_in`]): each read of an exception's query is for one of
/// `params` as given, and its value is only delivered as read. A `let` binds the value under its name until another
/// `let`, a closure parameter, a child's status or a loop's value so far shadows the name; a branch of an `if`, a
/// `display_if` or a `match` delivers it only to the result; a closure's result, a child's and a loop's are no
/// delivery. Nothing is delivered where the program chooses whether it runs: under a branch whose condition reads more
/// than literals and `params` as given, wherever the branch is, in a short-circuit operand after such an operand, in a
/// loop's body whose count reads more, in a closure's body, which runs when its caller chooses, and in a child scope,
/// whose budget chooses whether its body finishes or its stop branch runs.
fn deliveries(body: &Expr, params: &BTreeSet<&str>, exceptions: &BTreeMap<&str, Exception>) -> Deliveries {
    struct Walk<'a> {
        params: &'a BTreeSet<&'a str>,
        exceptions: &'a BTreeMap<&'a str, Exception>,
        /// The names `let`, closure parameters, a child's status and a loop's value so far bind, innermost last, with
        /// the exception whose value a name holds as read.
        scope: Vec<(&'a str, Option<Exception>)>,
        /// How many of the constructs around the current expression choose whether it runs: branches whose condition
        /// reads more than literals and `params` as given, short-circuit operands after such an operand, loop bodies
        /// whose count reads more, closure bodies and child scopes.
        steered: u32,
        path: Vec<u32>,
        out: Deliveries,
    }

    impl<'a> Walk<'a> {
        /// Check `e` at `place`; the exception whose value `e` is, as read.
        fn walk(&mut self, e: &'a Expr, place: Place) -> Option<Exception> {
            let read = match e {
                Expr::ReadSnapshot { query, .. } => Some((query, None)),
                Expr::ReadSnapshotWith { query, arg, .. } => Some((query, Some(&**arg))),
                _ => None,
            };
            if let Some((query, arg)) = read {
                if let Some(&x) = self.exceptions.get(query.as_str()) {
                    if let Some(arg) = arg {
                        self.child(0, arg, Place::Other);
                    }
                    let named = matches!(arg, Some(Expr::Var(v)) if self.params.contains(v.as_str()) && !self.binds(v));
                    if named {
                        self.deliver(x, place);
                    } else {
                        self.refuse(x.unnamed_note());
                        // Refused here once: the shown-facts rule does not report the same result again.
                        if place == Place::Result {
                            self.out.results.insert(path_text(&self.path));
                        }
                    }
                    return Some(x);
                }
            }
            match e {
                Expr::Var(v) => {
                    let held = self.scope.iter().rev().find(|(name, _)| *name == v.as_str()).and_then(|&(_, x)| x);
                    if let Some(x) = held {
                        self.deliver(x, place);
                    }
                    held
                }
                Expr::Let { name, value, body } => {
                    let held = self.child(0, value, Place::Bound);
                    self.scope.push((name.as_str(), held));
                    let out = self.child(1, body, place);
                    self.scope.pop();
                    out
                }
                Expr::If { cond, then, els } | Expr::DisplayIf { cond, then, els } => {
                    self.child(0, cond, Place::Other);
                    let (branch, steers) = self.branch(cond, place);
                    self.steered += steers;
                    self.child(1, then, branch);
                    self.child(2, els, branch);
                    self.steered -= steers;
                    None
                }
                Expr::Match { scrutinee, arms, default } => {
                    self.child(0, scrutinee, Place::Other);
                    let (branch, steers) = self.branch(scrutinee, place);
                    self.steered += steers;
                    for (i, arm) in arms.iter().map(|(_, arm)| arm).chain([&**default]).enumerate() {
                        self.child(i as u32 + 1, arm, branch);
                    }
                    self.steered -= steers;
                    None
                }
                Expr::PublishView { value } => {
                    if self.child(0, value, Place::View).is_some() {
                        self.out.views.insert(path_text(&self.path));
                    }
                    None
                }
                Expr::Closure { params, body, .. } => {
                    // Its caller chooses when, and whether, the body runs.
                    let depth = self.scope.len();
                    self.scope.extend(params.iter().map(|p| (p.name.as_str(), None)));
                    self.steered += 1;
                    self.child(0, body, Place::Other);
                    self.steered -= 1;
                    self.scope.truncate(depth);
                    None
                }
                Expr::Child { body, status, otherwise, .. } => {
                    // Its budget chooses whether the body finishes or the stop branch runs (DET.60).
                    self.steered += 1;
                    self.child(0, body, Place::Other);
                    let depth = self.scope.len();
                    self.scope.extend(status.as_deref().map(|s| (s, None)));
                    self.child(1, otherwise, Place::Other);
                    self.scope.truncate(depth);
                    self.steered -= 1;
                    None
                }
                Expr::Repeat { count, init, acc, body, .. } => {
                    // Its count chooses whether, and how often, the body runs (DET.55). The loop's value, its initial
                    // value or the body's last result, is no delivery, and the value so far shadows `acc`.
                    self.child(0, count, Place::Other);
                    self.child(1, init, Place::Other);
                    let steers = u32::from(!self.by_solver(count));
                    self.scope.push((acc.as_str(), None));
                    self.steered += steers;
                    self.child(2, body, Place::Other);
                    self.steered -= steers;
                    self.scope.pop();
                    None
                }
                Expr::Prim { op: Prim::And | Prim::Or, args } => {
                    // A short-circuit operand runs only when the operands before it leave the result open (LG2.20).
                    for (i, arg) in args.iter().enumerate() {
                        let steers = u32::from(!args[..i].iter().all(|before| self.by_solver(before)));
                        self.steered += steers;
                        self.child(i as u32, arg, Place::Other);
                        self.steered -= steers;
                    }
                    None
                }
                _ => {
                    for (i, child) in e.children().into_iter().enumerate() {
                        self.child(i as u32, child, Place::Other);
                    }
                    None
                }
            }
        }

        fn child(&mut self, i: u32, e: &'a Expr, place: Place) -> Option<Exception> {
            self.path.push(i);
            let out = self.walk(e, place);
            self.path.pop();
            out
        }

        fn binds(&self, name: &str) -> bool {
            self.scope.iter().any(|(bound, _)| *bound == name)
        }

        /// Where the branches of a condition `cond` at `place` deliver, and whether `cond` steers them: a branch
        /// delivers only to the result, and its condition chooses which branch runs unless it reads nothing but
        /// literals and `params` as given, wherever the branch is.
        fn branch(&self, cond: &Expr, place: Place) -> (Place, u32) {
            let steers = u32::from(!self.by_solver(cond));
            match place {
                Place::Result => (Place::Result, steers),
                _ => (Place::Other, steers),
            }
        }

        /// Whether `e` reads nothing but literals and `params` as given, through primitive operations and fields. A
        /// draw reads a generator's stream, which the solver does not give.
        fn by_solver(&self, e: &Expr) -> bool {
            match e {
                Expr::Lit(_) => true,
                Expr::Var(v) => self.params.contains(v.as_str()) && !self.binds(v),
                Expr::Prim { op, args } => *op != Prim::RngDraw && args.iter().all(|arg| self.by_solver(arg)),
                Expr::Field { value, .. } => self.by_solver(value),
                _ => false,
            }
        }

        fn deliver(&mut self, x: Exception, place: Place) {
            match place {
                Place::Result | Place::View if self.steered > 0 => {
                    self.refuse(x.steered_note());
                    // Refused here once: the shown-facts rule does not report the same result again.
                    if place == Place::Result {
                        self.out.results.insert(path_text(&self.path));
                    }
                }
                Place::Result => {
                    self.out.results.insert(path_text(&self.path));
                }
                Place::View | Place::Bound => {}
                Place::Other => self.refuse(x.used_note()),
            }
        }

        fn refuse(&mut self, note: String) {
            self.out.refusals.push((path_text(&self.path), note));
        }
    }

    let mut walk =
        Walk { params, exceptions, scope: Vec::new(), steered: 0, path: Vec::new(), out: Deliveries::default() };
    if !exceptions.is_empty() {
        walk.walk(body, Place::Result);
    }
    walk.out
}

/// Whether a value of a type may hold a word, itself, in a record field, in a collection's elements, keys or values or
/// in a callable's parameters or result, at any depth. A type variable, the unknown type included, may stand for a
/// word, so it counts as one; a compressed word, a word equality key, a trace equality key and a menu word hold a
/// word's letters (LP2 drafts). A callable counts by its parameters too, conservatively: one that takes a callback may
/// hand the callback a word it holds. Every type is named, so a type added later is a compile error here rather than a
/// silent "holds no word".
fn carries_word(ty: &Ty) -> bool {
    match ty {
        Ty::Word | Ty::CompressedWord | Ty::WordEqualityKey | Ty::TraceEqualityKey | Ty::MenuWord | Ty::Var(_) => true,
        Ty::Record(fields) => fields.iter().any(|(_, f)| carries_word(&f.ty)),
        Ty::Fn(f) => carries_word(&f.result.ty) || f.params.iter().any(|p| carries_word(&p.ty)),
        Ty::List(_) | Ty::Set(_) | Ty::Map(..) => ty.elements().into_iter().any(|e| carries_word(&e.ty)),
        Ty::Unit
        | Ty::Bool
        | Ty::Nat
        | Ty::Int
        | Ty::Rat
        | Ty::Q5
        | Ty::Text
        | Ty::Target
        | Ty::Budget
        | Ty::Status
        | Ty::Colour
        | Ty::Approx(_)
        | Ty::ApproxBool
        | Ty::Receipt
        | Ty::Generator
        | Ty::EndTest(_) => false,
    }
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
        Ty::List(_) => "List".into(),
        Ty::WordEqualityKey => "WordEqualityKey".into(),
        Ty::TraceEqualityKey => "TraceEqualityKey".into(),
        Ty::Set(_) => "Set".into(),
        Ty::Map(..) => "Map".into(),
        Ty::CompressedWord => "CompressedWord".into(),
        Ty::MenuWord => "MenuWord".into(),
        Ty::Generator => "Generator".into(),
        Ty::EndTest(test) => test.type_name().into(),
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
        Ty::List(e) => format!("List<{}>", type_text(&e.ty)),
        Ty::Set(e) => format!("Set<{}>", type_text(&e.ty)),
        Ty::Map(k, v) => format!("Map<{}, {}>", type_text(&k.ty), type_text(&v.ty)),
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
        c @ (Ty::List(_) | Ty::Set(_) | Ty::Map(..)) => c.with_elements(c.elements().into_iter().map(q).collect()),
        other => other.clone(),
    }
}

/// The type with every menu word read as a Full word, at any depth: the type it would have if the two alphabets
/// were one (LB1.8; LP2 draft). A mismatch that this removes is one of alphabets only.
fn one_alphabet(ty: &Ty) -> Ty {
    let q = |q: &QTy| QTy { ty: one_alphabet(&q.ty), display: q.display };
    match ty {
        Ty::MenuWord => Ty::Word,
        Ty::Record(fields) => Ty::Record(fields.iter().map(|(n, f)| (n.clone(), q(f))).collect()),
        Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
            params: f.params.iter().map(q).collect(),
            result: q(&f.result),
            effects: f.effects.clone(),
        })),
        c @ (Ty::List(_) | Ty::Set(_) | Ty::Map(..)) => c.with_elements(c.elements().into_iter().map(q).collect()),
        other => other.clone(),
    }
}

/// The type with every end test's verdict read as a `Bool`, at any depth: the type it would have if a verdict were a
/// boolean (LG2.9; LP2 draft). A mismatch that this removes is one of a verdict used as a boolean.
fn verdicts_as_bool(ty: &Ty) -> Ty {
    let q = |q: &QTy| QTy { ty: verdicts_as_bool(&q.ty), display: q.display };
    match ty {
        Ty::EndTest(_) => Ty::Bool,
        Ty::Record(fields) => Ty::Record(fields.iter().map(|(n, f)| (n.clone(), q(f))).collect()),
        Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
            params: f.params.iter().map(q).collect(),
            result: q(&f.result),
            effects: f.effects.clone(),
        })),
        c @ (Ty::List(_) | Ty::Set(_) | Ty::Map(..)) => c.with_elements(c.elements().into_iter().map(q).collect()),
        other => other.clone(),
    }
}

/// Whether a type is or holds an end test's verdict in a record or a collection (LP2 draft).
fn holds_verdict(ty: &Ty) -> bool {
    match ty {
        Ty::EndTest(_) => true,
        Ty::Record(fields) => fields.iter().any(|(_, f)| holds_verdict(&f.ty)),
        other => other.elements().into_iter().any(|e| holds_verdict(&e.ty)),
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
        c @ (Ty::List(_) | Ty::Set(_) | Ty::Map(..)) => {
            c.elements().into_iter().fold(Qual::Exact, |acc, e| acc.join(declared_qual(e).container()))
        }
        _ => Qual::Exact,
    }
}

/// Whether a set element or a map key may have this type (LG2.20; LP2 draft): exact and without the display qualifier
/// at any depth, and without a callable, a receipt, `Unit`, a type variable, a compressed word, a generator or an end
/// test's verdict, so that its canonical key order (DET.9) is defined and a verdict is only ever matched (LG2.9).
fn key_type(q: &QTy) -> bool {
    !q.display
        && match &q.ty {
            Ty::Unit
            | Ty::Approx(_)
            | Ty::ApproxBool
            | Ty::Fn(_)
            | Ty::Receipt
            | Ty::Var(_)
            | Ty::CompressedWord
            | Ty::Generator
            | Ty::EndTest(_) => false,
            Ty::Record(fields) => fields.iter().all(|(_, f)| key_type(f)),
            other => other.elements().into_iter().all(key_type),
        }
}

/// Projection's qualifier rule (LG2.19): an element of a container of qualifier `container`, declared with qualifier
/// `element`. A display-qualified container lends no exact element; an element under approximate control stays so.
fn project(container: Qual, element: Qual) -> Qual {
    match container {
        Qual::Exact => element,
        Qual::Display { control: true } => element.join(Qual::Display { control: true }),
        _ => match element {
            Qual::Approx | Qual::Cond => element,
            q => q.join(Qual::Display { control: false }),
        },
    }
}

/// Whether a type holds a callable at any depth. A collection does not (LP2 draft): its elements carry no callable
/// needs (LG3.2).
fn holds_callable(ty: &Ty) -> bool {
    match ty {
        Ty::Fn(_) => true,
        Ty::Record(fields) => fields.iter().any(|(_, f)| holds_callable(&f.ty)),
        other => other.elements().into_iter().any(|e| holds_callable(&e.ty)),
    }
}

/// What a goal or an evidence sink takes, in a mismatch's `expected`.
const PORTABLE: &str =
    "a type without Fn, Receipt, WordEqualityKey, TraceEqualityKey, CompressedWord, Generator or a type variable";

/// Whether a goal or an evidence sink takes a value of this type (owner decision of 16 November 2026 under PHS.52):
/// portable data with a canonical encoding, so no callable, receipt, word equality key (LG2.13), trace equality key
/// (LB1.10), compressed word (DET.59) or generator (DET.61) at any depth, and no type variable, which may stand for one
/// of them (the checker does not unify type variables, M1). An unknown type fits. Whether an approximate value may
/// enter is the exactness rules' question (LG2.22).
fn portable(ty: &Ty) -> bool {
    match ty {
        _ if is_unknown(ty) => true,
        Ty::Fn(_) | Ty::Receipt | Ty::Var(_) | Ty::WordEqualityKey | Ty::TraceEqualityKey | Ty::CompressedWord => false,
        Ty::Generator => false,
        Ty::Record(fields) => fields.iter().all(|(_, f)| portable(&f.ty)),
        other => other.elements().into_iter().all(|e| portable(&e.ty)),
    }
}

/// Whether a type is or holds a generator in a record field or a collection (LP2 draft; DET.61). A generator stays a
/// bare value, so that each owner holds its own cursor: no record, collection, closure capture or type variable holds
/// one, since copying any of them would share the cursor instead of copying it. A callable's signature holds no value.
fn holds_generator(ty: &Ty) -> bool {
    match ty {
        Ty::Generator => true,
        Ty::Record(fields) => fields.iter().any(|(_, f)| holds_generator(&f.ty)),
        other => other.elements().into_iter().any(|e| holds_generator(&e.ty)),
    }
}

/// Whether a declared type puts a generator where a value would share its cursor (LP2 draft; DET.61): in a record
/// field or a collection, also inside a callable's parameter or result types. With `alone`, a generator may stand as
/// the type itself, as a signature's parameter or result; without it, as for data a host answers with, it may not.
fn shares_generator(ty: &Ty, alone: bool) -> bool {
    match ty {
        Ty::Generator => !alone,
        Ty::Fn(f) => f.params.iter().any(|p| shares_generator(&p.ty, true)) || shares_generator(&f.result.ty, true),
        Ty::Record(fields) => fields.iter().any(|(_, f)| shares_generator(&f.ty, false)),
        other => other.elements().into_iter().any(|e| shares_generator(&e.ty, false)),
    }
}

/// Whether a type is or holds a compressed word in a record or a collection (LP2 draft).
fn holds_compressed(ty: &Ty) -> bool {
    match ty {
        Ty::CompressedWord => true,
        Ty::Record(fields) => fields.iter().any(|(_, f)| holds_compressed(&f.ty)),
        other => other.elements().into_iter().any(|e| holds_compressed(&e.ty)),
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
        Ty::List(_) | Ty::Set(_) | Ty::Map(..) => slot.ty.elements().into_iter().all(|e| exact_slot(e, own)),
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
        (s, t) if s.same_collection(t) => s.elements().into_iter().zip(t.elements()).all(|(s, t)| admits(s, t, own)),
        _ => true,
    }
}

/// Whether a callable of type `t` keeps every qualifier in a slot of type `s` (LG2.19): its result is no more
/// qualified than the slot promises, and each parameter takes what the slot passes. A parameter's type variables
/// are the callable's own and take anything.
fn signature_keeps(s: &FnTy, t: &FnTy, own: bool) -> bool {
    s.params.iter().zip(&t.params).all(|(sp, tp)| admits(tp, sp, false)) && admits(&s.result, &t.result, own)
}

/// A storage read's value `v`, declared as `slot`, of a value the program stores as `stored`: a type variable of the
/// declared type stands for the stored type and keeps its qualifier, at any depth of a record, since decoding retains
/// the qualifier (LG2.19). A record holding such a field is display-qualified as a whole.
fn decoded(v: Val, slot: &QTy, stored: &QTy) -> Val {
    match (&slot.ty, &stored.ty) {
        (Ty::Var(_), _) => Val { qual: v.qual.join(declared_qual(stored)), ..v },
        (Ty::Record(s), Ty::Record(t)) if s.len() == t.len() => {
            let fields: Vec<Val> =
                s.iter().zip(t).enumerate().map(|(i, ((_, sf), (_, tf)))| decoded(v.field(i), sf, tf)).collect();
            let qual = fields.iter().fold(v.qual, |q, f| q.join(f.qual.container()));
            Val { qual, fields: Some(fields), ..v }
        }
        _ => v,
    }
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
        // A type variable takes no generator, which stays a bare value (LP2 draft; DET.61); the unknown type takes one.
        (Ty::Var(name), value) => name == UNKNOWN || !holds_generator(value),
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
        (a, b) if a.same_collection(b) => a.elements().into_iter().zip(b.elements()).all(|(x, y)| fits(&x.ty, &y.ty)),
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
        (x, y) if x.same_collection(y) => {
            let parts = x.elements().into_iter().zip(y.elements()).map(|(p, q)| widen_q(p, q));
            Some(x.with_elements(parts.collect::<Option<Vec<_>>>()?))
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
        other => {
            for e in other.elements() {
                effect_vars(e, out);
            }
        }
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
        c @ (Ty::List(_) | Ty::Set(_) | Ty::Map(..)) => {
            c.with_elements(c.elements().into_iter().map(|e| rename_vars(e, map)).collect())
        }
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
        Lit::MenuWord(_) => Ty::MenuWord,
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

/// [`Summary::ungranted`], given whether evaluation may read a file.
fn ungranted(cap: Capability, reads_file: bool) -> &'static str {
    match cap {
        Capability::Clock | Capability::Network => AMBIENT_READ,
        Capability::File if reads_file => AMBIENT_READ,
        Capability::File | Capability::Randomness | Capability::Process | Capability::SessionFile => NOT_GRANTED,
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
        Expr::Child { body, status, otherwise, .. } => {
            free_names(body, bound, out);
            let depth = bound.len();
            bound.extend(status.as_deref());
            free_names(otherwise, bound, out);
            bound.truncate(depth);
        }
        Expr::Repeat { count, init, acc, body, .. } => {
            free_names(count, bound, out);
            free_names(init, bound, out);
            bound.push(acc);
            free_names(body, bound, out);
            bound.pop();
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

    /// Element type `i` of a collection value ([`Ty::elements`]) under projection's qualifier rule (LG2.19).
    fn element(&self, i: usize) -> Val {
        let Some(declared) = self.ty.elements().get(i).map(|e| (*e).clone()) else {
            return Val::unknown();
        };
        let mut v = Val::declared(&declared);
        v.qual = project(self.qual, v.qual);
        v
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
            // Either callable may be called: a view's rule and a control bit hold for the merged one.
            x.view |= y.view;
            x.control |= y.control;
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
        c if c.same_collection(&v.ty) => {
            for (i, e) in c.elements().into_iter().enumerate() {
                bindings(e, &v.element(i), out);
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

/// The named functions that the callables in a value, itself or in a record field at any depth, may call (LG2.20).
fn reached(v: &Val, out: &mut BTreeSet<(String, String)>) {
    if let Some(callable) = &v.callable {
        out.extend(callable.reaches.iter().cloned());
    }
    if let Some(fields) = &v.fields {
        for f in fields {
            reached(f, out);
        }
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
    /// The paths of the views that publish a word-carrying value, by module and function index, found while
    /// emitting ([`Summary::word_views`]).
    word_views: BTreeMap<(usize, usize), BTreeSet<String>>,
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
            word_views: BTreeMap::new(),
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
                            // A manifest's network request is an ambient read (chapter 13; PHS.52).
                            code: ungranted(cap, false),
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
                let mut summary = self.table[mi][fi].clone();
                summary.word_views = self.word_views.get(&(mi, fi)).cloned().unwrap_or_default();
                summaries.insert((module.name.clone(), f.name.clone()), summary);
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
        // A word equality key where literal letters are required has a code of its own (LG2.13, CT-LG2-11), and so
        // has a trace equality key (LB1.10; LP2 draft).
        if (supplied == "WordEqualityKey" || supplied == "TraceEqualityKey") && expected == "Word" {
            let note = format!("{what}: a {supplied} is never literal letters (LG2.13, LG2.17, LB1.10)");
            return self.record(WORD_KEY_NOT_EXECUTABLE, None, Some(Mismatch { expected, supplied }), note);
        }
        // A Full word and a menu word are words of two alphabets, which no operation mixes (LB1.8; LP2 draft).
        if matches!((expected.as_str(), supplied.as_str()), ("Word", "MenuWord") | ("MenuWord", "Word")) {
            let note = format!("{what}: a {supplied} is not a word of the {expected} alphabet (LB1.8)");
            return self.record(INCOMPATIBLE_ALPHABET, None, Some(Mismatch { expected, supplied }), note);
        }
        // An end test's verdict is matched on its outcomes and never used as a boolean (LG2.9; LP2 draft).
        if matches!(expected.as_str(), "Bool" | "ApproxBool") && EndTest::of_type_name(&supplied).is_some() {
            let note = format!(
                "{what}: a {supplied} verdict is matched on its outcomes and never used as a boolean, so \
                 `unestablished` never reads as `fails` (LG2.9)"
            );
            return self.record(VERDICT_REQUIRED, None, Some(Mismatch { expected, supplied }), note);
        }
        // A boolean where a verdict is expected has no `unestablished` (LG2.9; LP2 draft).
        if EndTest::of_type_name(&expected).is_some() && matches!(supplied.as_str(), "Bool" | "ApproxBool") {
            let note = format!(
                "{what}: a {supplied} where a {expected} verdict is expected, which a boolean never stands for: it has \
                 no `unestablished` (LG2.9)"
            );
            return self.record(VERDICT_REQUIRED, None, Some(Mismatch { expected, supplied }), note);
        }
        let note = format!("{what}: {supplied} where {expected} is expected (LG2.1)");
        self.record(TYPE_MISMATCH, None, Some(Mismatch { expected, supplied }), note);
    }

    /// A value at the current position whose base type does not fit its use. Its exactness is not checked: where
    /// both would apply, the base type comes first. The caller goes on with [`Val::unknown`] in its place.
    fn misfit(&mut self, expected: &str, v: &Val, what: &str) {
        self.mismatch(expected.to_string(), type_text(&v.ty), what);
    }

    /// A value whose type does not fit a slot of type `slot`, as [`Checker::misfit`] reports it.
    fn misfit_slot(&mut self, slot: &Ty, v: &Val, what: &str) {
        self.mismatch_of(slot, &v.ty, fits, what);
    }

    /// A mismatch between two known types, which do not `agree` (the test that found it). When they would agree with
    /// every menu word read as a Full word, a menu word stands where a Full word is expected or the reverse, at some
    /// depth, as in a list of menu words given for a list of Full words: the mismatch is one of alphabets only, and
    /// its code is `LB1:input.incompatible_alphabet` (LB1.8; LP2 draft). When they would agree with every end test's
    /// verdict supplied read as a `Bool`, a verdict stands where a boolean is expected, at some depth, and when they
    /// would agree with every verdict expected read as a `Bool`, a boolean stands where a verdict is expected: the
    /// code is `LG2:type.verdict_required` (LG2.9; LP2 draft), whichever branch of a join comes first. Otherwise it is
    /// [`Checker::mismatch`]'s.
    fn mismatch_of(&mut self, expected: &Ty, supplied: &Ty, agree: fn(&Ty, &Ty) -> bool, what: &str) {
        let (one_expected, one_supplied) = (one_alphabet(expected), one_alphabet(supplied));
        let (expected_text, supplied_text) = (type_text(expected), type_text(supplied));
        let alphabets = &one_expected != expected || &one_supplied != supplied;
        if alphabets && agree(&one_expected, &one_supplied) {
            let note = format!(
                "{what}: {supplied_text} where {expected_text} is expected: a Full word and a menu word are words of \
                 two alphabets (LB1.8)"
            );
            let mismatch = Mismatch { expected: expected_text, supplied: supplied_text };
            return self.record(INCOMPATIBLE_ALPHABET, None, Some(mismatch), note);
        }
        let (supplied_as_bool, expected_as_bool) = (verdicts_as_bool(supplied), verdicts_as_bool(expected));
        if (&supplied_as_bool != supplied && agree(expected, &supplied_as_bool))
            || (&expected_as_bool != expected && agree(&expected_as_bool, supplied))
        {
            let note = format!(
                "{what}: {supplied_text} where {expected_text} is expected: an end test's verdict is matched on its \
                 outcomes, never used as a boolean, and a boolean never stands for it (LG2.9)"
            );
            let mismatch = Mismatch { expected: expected_text, supplied: supplied_text };
            return self.record(VERDICT_REQUIRED, None, Some(mismatch), note);
        }
        self.mismatch(expected_text, supplied_text, what);
    }

    /// A read of an end test's row declares that test's verdict (LB3.2; LP2 draft). A `Bool` would let `unestablished`
    /// read as `fails` and is `LG2:type.verdict_required` (LG2.9); any other type, another test's verdict included, is
    /// `LG2:type.mismatch`: one test's verdict never stands for another's. A read of another row that declares a
    /// verdict is `LG2:type.mismatch` too: only the test's own row answers its verdict.
    fn end_test_read(&mut self, query: &str, ty: &QTy) {
        let Some(test) = EndTest::of_row(query) else {
            if let Ty::EndTest(declared) = &ty.ty {
                let note = format!(
                    "a read of `{query}` declared {}: only the end test's own row `{}` answers that verdict (LB3.2)",
                    declared.type_name(),
                    declared.row()
                );
                self.record(TYPE_MISMATCH, None, None, note);
            }
            return;
        };
        if ty.ty == Ty::EndTest(test) {
            return;
        }
        let mismatch = Mismatch { expected: test.type_name().into(), supplied: type_text(&ty.ty) };
        let (expected, supplied) = (&mismatch.expected, &mismatch.supplied);
        if matches!(ty.ty, Ty::Bool | Ty::ApproxBool) {
            let note = format!(
                "a read of `{query}` declared {supplied}: the end test answers a {expected} verdict, matched on its \
                 outcomes and never used as a boolean (LB3.2, LG2.9)"
            );
            self.record(VERDICT_REQUIRED, None, Some(mismatch), note);
        } else {
            let note = format!(
                "a read of `{query}` declared {supplied}: the end test answers a {expected} verdict, and one test's \
                 verdict never stands for another's (LB3.2)"
            );
            self.record(TYPE_MISMATCH, None, Some(mismatch), note);
        }
    }

    // The end-test rules below run inside `expr` and `prim` but live apart from them: `expr` recurses once per nested
    // expression, and every local of its arms is part of each recursion's stack frame in a debug build.

    /// The pattern of a match arm on an end test's verdict is the tag of one of its outcomes (LG2.9; LP2 draft): a
    /// boolean pattern is `LG2:type.verdict_required`, any other tag or literal `LG2:type.mismatch`.
    fn verdict_pattern(&mut self, test: EndTest, lit: &Lit, pattern: &Ty, what: &str) {
        let expected = test.type_name().to_string();
        match lit {
            Lit::Tag(t) if EndTestOutcome::of_tag(t).is_some() => {}
            Lit::Bool(_) => {
                let note = format!(
                    "{what}: a boolean pattern for a {expected} verdict, which is matched on its outcomes and never \
                     used as a boolean (LG2.9)"
                );
                let mismatch = Mismatch { expected, supplied: "Bool".into() };
                self.record(VERDICT_REQUIRED, None, Some(mismatch), note);
            }
            Lit::Tag(t) => {
                let note = format!(
                    "{what}: tag(\"{t}\") is no outcome of a {expected} verdict, which is `holds`, `fails` or \
                     `unestablished` (LG2.9)"
                );
                let mismatch = Mismatch { expected, supplied: type_text(pattern) };
                self.record(TYPE_MISMATCH, None, Some(mismatch), note);
            }
            _ => self.mismatch(expected, type_text(pattern), what),
        }
    }

    /// The `_` arm of a match on an end test's verdict takes `unestablished` alone, never together with another
    /// outcome: unknown never reads as no, or as yes (LG2.9; LP2 draft). A type variable may stand for a verdict, so a
    /// match of one with an outcome's tag among its patterns is held to the same rule.
    fn verdict_default_arm(&mut self, scrutinee: &Ty, arms: &[(Lit, Expr)]) {
        let outcome = |lit: &Lit| match lit {
            Lit::Tag(t) => EndTestOutcome::of_tag(t),
            _ => None,
        };
        let whose = match scrutinee {
            Ty::EndTest(test) => test.type_name(),
            Ty::Var(_) if !is_unknown(scrutinee) && arms.iter().any(|(lit, _)| outcome(lit).is_some()) => {
                "type variable's"
            }
            _ => return,
        };
        let named: BTreeSet<EndTestOutcome> = arms.iter().filter_map(|(lit, _)| outcome(lit)).collect();
        let taken: Vec<&str> =
            EndTestOutcome::ALL.into_iter().filter(|o| !named.contains(o)).map(EndTestOutcome::tag).collect();
        if taken.len() > 1 && taken.contains(&EndTestOutcome::Unestablished.tag()) {
            let note = format!(
                "the `_` arm of a match on a {whose} verdict takes `{}` together: name `unestablished` in an arm of its \
                 own, or both `holds` and `fails`, so unknown never reads as no (LG2.9)",
                taken.join("` and `")
            );
            self.record(VERDICT_REQUIRED, None, None, note);
        }
    }

    /// While emitting, a view that publishes a word-carrying value is recorded by its path: the built-in column asks
    /// whether it publishes a word the solver named (LB3.5; LP2 draft; [`Summary::word_views`]).
    fn note_word_view(&mut self, ty: &Ty) {
        if self.emitting && carries_word(ty) {
            let path = path_text(&self.path);
            self.word_views.entry(self.cur).or_default().insert(path);
        }
    }

    /// An end test's verdict is matched on its outcomes (LG2.9; LP2 draft): an exact equality would make a boolean of
    /// it, with `unestablished` one value among the others. The refusal is recorded at the first operand that holds a
    /// verdict, and the answer says whether one did. A type variable's operand may hold a verdict that only the run
    /// sees; `c600-run` refuses that equality before its step.
    fn verdict_equality(&mut self, vals: &[Val]) -> bool {
        let Some(i) = vals.iter().position(|v| holds_verdict(&v.ty)) else {
            return false;
        };
        let note = format!(
            "an operand of an exact equality holds a {} verdict, which is matched on its outcomes and never compared \
             (LG2.9)",
            type_text(&vals[i].ty)
        );
        self.at(i, |c| c.record(VERDICT_REQUIRED, None, None, note));
        true
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
                self.at(i, |c| c.misfit_slot(&slot.ty, v, &arg));
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
                let mut needs = substitute(summary, params, vals);
                let call = format!("the call of {what}");
                if summary.view {
                    // A view's effect variable takes what the call site passes; the view still has no workspace or
                    // preview effect. An effect of the view's own body is reported at the view.
                    for e in [Effect::ChangeWorkspace, Effect::PreparePreview] {
                        if needs.effects.contains(&e) && !summary.effects.contains(&e) {
                            self.effect(format!(
                                "{call}: an effect variable of the view takes a {e:?} effect, and a view has none, \
                                 also through callees (LG3.5, LG3.6)"
                            ));
                        }
                    }
                }
                // The callee may call any callable it is passed, whatever the slot's bound: recursion through it is
                // recursion (LG2.20).
                for v in vals.iter() {
                    reached(v, &mut needs.reaches);
                }
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
            self.within(at, |c| c.misfit_slot(&slot.ty, v, what));
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
            self.at(at, |c| c.mismatch_of(&ta, &tb, |a, b| widen(a, b).is_some(), what));
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
        for p in &f.params {
            self.bare(&p.ty.ty, true, "a parameter")?;
        }
        self.bare(&f.result.ty, true, "the result")?;
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

    /// Refuses a declared type that would share a generator's cursor ([`shares_generator`]; LP2 draft, DET.61).
    fn bare(&self, ty: &Ty, alone: bool, what: &str) -> Result<(), IrError> {
        if shares_generator(ty, alone) {
            let note = "a generator stays a bare value, so no record, collection or host answer holds one";
            return Err(self.err(format!("{what} of type {}: {note} (LP2 draft; DET.61)", type_text(ty))));
        }
        Ok(())
    }

    /// Refuses a value that holds a generator where a view or a file receives it ([`holds_generator`]; LP2 draft,
    /// DET.61): it has no canonical form, so the checker refuses it before the run would stop at its encoding.
    fn sink(&self, ty: &Ty, what: &str) -> Result<(), IrError> {
        if holds_generator(ty) {
            let note = "a generator stays a bare value, so no view or file receives one";
            return Err(self.err(format!("{what} of type {}: {note} (LP2 draft; DET.61)", type_text(ty))));
        }
        Ok(())
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
                summary.view = f.kind == FnKind::View;
                summary.control = self.controls[mi][fi];
                let ty = Ty::Fn(Box::new(FnTy {
                    params: f.params.iter().map(|p| p.ty.clone()).collect(),
                    result: f.result.clone(),
                    effects: summary.bound(),
                }));
                Ok((Val { ty, qual: Qual::Exact, callable: Some(summary), fields: None }, Summary::default()))
            }
            // A chain of lets is checked in a loop, as nested `at(1, ..)` calls would check it, so a long straight-line
            // body does not deepen the recursion: an unoptimised build gives each level of `expr` a large frame.
            Expr::Let { .. } => {
                let (depth, bound) = (self.path.len(), env.vars.len());
                let mut node = e;
                let mut values: Option<Summary> = None;
                let r = loop {
                    let Expr::Let { name, value, body } = node else { break self.expr(node, env) };
                    match self.at(0, |c| c.expr(value, env)) {
                        Ok((v, s)) => {
                            match &mut values {
                                None => values = Some(s),
                                Some(all) => all.add(&s),
                            }
                            env.vars.push((name.clone(), v));
                        }
                        Err(err) => break Err(err),
                    }
                    self.path.push(1);
                    node = &**body;
                };
                self.path.truncate(depth);
                env.vars.truncate(bound);
                let (bv, bs) = r?;
                let mut s = values.expect("a let chain binds at least one value");
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
                    let what = format!("the pattern of arm {i} of a match");
                    if let Ty::EndTest(test) = sv.ty {
                        // An end test's verdict is matched on the tags of its outcomes (LG2.9; LP2 draft).
                        self.verdict_pattern(test, lit, &pattern, &what);
                    } else if !fits(&pattern, &sv.ty) && !fits(&sv.ty, &pattern) {
                        self.mismatch_of(&sv.ty, &pattern, |a, b| fits(a, b) || fits(b, a), &what);
                    } else if let Lit::Approx { .. } = lit {
                        // An approximate literal is never read as exact (LG1.7), so it is no operand of that equality.
                        self.exact_use(Site::Operand, &Val::new(pattern, Qual::Approx), &what);
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
                self.verdict_default_arm(&sv.ty, arms);
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
                // Only the selected branch is charged (DET.64).
                s.display_cost = true;
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
                        summary.view = f.kind == FnKind::View;
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
                            // A function value's number of parameters is part of its type.
                            Ty::Fn(fty) if fty.params.len() == vals.len() => (**fty).clone(),
                            other => {
                                if !is_unknown(other) {
                                    let params: Vec<String> = vals.iter().map(|v| type_text(&v.ty)).collect();
                                    let expected = format!("fn({}) -> _", params.join(", "));
                                    self.at(args.len(), |c| c.misfit(&expected, &fv, "a called value"));
                                }
                                // What the call needs is unknown (LG3.2).
                                s.unresolved.insert("<a value that does not fit its call>".into());
                                return Ok((Val::unknown(), s));
                            }
                        };
                        let callable = fv.callable.as_ref();
                        // A function value of a named function or closure keeps its control bit; a callable known
                        // only by its declared type has none the checker can see (LG2.20, LG2.22).
                        let control = callable.is_some_and(|c| c.control);
                        let (v, needs) = self.call(
                            &fty.params,
                            &fty.result,
                            callable,
                            &mut vals,
                            fv.qual,
                            control,
                            "a function value",
                        )?;
                        s.add(&needs);
                        s.display_cost = true;
                        Ok((v, s))
                    }
                    Callee::Unresolved { name, result } => {
                        // Dynamic code (owner decision of 16 November 2026 under PHS.52).
                        let note = format!(
                            "the unresolved callable `{name}` is dynamic code, which no grant admits (DET.36, RTE.45); \
                             it is neither exact nor pure (LG2.19, LG3.2)"
                        );
                        self.push(NOT_GRANTED, None, note);
                        s.unresolved.insert(name.clone());
                        Ok((Val::declared(result), s))
                    }
                }
            }
            Expr::Closure { params, result, body } => {
                for p in params {
                    self.bare(&p.ty.ty, true, "a closure's parameter")?;
                }
                self.bare(&result.ty, true, "a closure's result")?;
                let mut free = BTreeSet::new();
                let mut bound: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
                free_names(body, &mut bound, &mut free);
                let mut inner = Env::default();
                for name in free {
                    match env.get(name) {
                        Some(v) if holds_generator(&v.ty) => {
                            let note = "a closure captures no generator, which stays a bare value (LP2 draft; DET.61)";
                            return Err(self.err(format!("`{name}`: {note}")));
                        }
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
                let (bv, mut bs) = walked?;
                let mut at = vec![0];
                at.extend(returned(body));
                let fit = self.result(&result, &bv, display_input.as_ref(), "the closure's result", &at);
                // The closure's control bit, as a named function's (LG2.20, LG2.22).
                bs.control = fit
                    && !result.display
                    && !exact_slot(&result, true)
                    && bv.qual == (Qual::Display { control: true });
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
                if *op == Prim::RngDraw {
                    // A draw advances a seeded stream, as a random draw does (DET.37, DET.61; LP2 draft).
                    let own = needs(&[Effect::SeededRandomness], &[Capability::Randomness]);
                    self.operation(&mut s, own, "a generator draw");
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
                    if holds_generator(&v.ty) {
                        let note = "a record holds no generator, which stays a bare value (LP2 draft; DET.61)";
                        return Err(self.err(format!("field `{name}`: {note}")));
                    }
                    s.add(&f_s);
                    qual = qual.join(v.qual.container());
                    let display = v.qual != Qual::Exact && !matches!(v.ty, Ty::Approx(_) | Ty::ApproxBool);
                    tys.push((name.clone(), QTy { ty: v.ty.clone(), display }));
                    vals.push(v);
                }
                // The record keeps its field values, so that a callable keeps its needs through it (LG3.2).
                Ok((Val { ty: Ty::Record(tys), qual, callable: None, fields: Some(vals) }, s))
            }
            Expr::Collection { ty, items } => {
                let elements: Vec<QTy> = ty.elements().into_iter().cloned().collect();
                if elements.is_empty() {
                    return Err(self.err(format!("a collection of type {}", type_text(ty))));
                }
                if elements.iter().any(|e| holds_callable(&e.ty)) {
                    return Err(self.err(format!("{}: a collection holds no callable (LP2 draft)", type_text(ty))));
                }
                self.bare(ty, true, "a collection")?;
                // A set's elements and a map's keys are exact, so that their canonical key order is defined (LG2.20).
                let keyed = matches!(ty, Ty::Set(_) | Ty::Map(..));
                if keyed && !key_type(&elements[0]) {
                    return Err(self.err(format!("{}: a set element or a map key is exact (LG2.20)", type_text(ty))));
                }
                if matches!(ty, Ty::Map(..)) && items.len() % 2 == 1 {
                    return Err(self.err(format!("{}: a map's items alternate keys and values", type_text(ty))));
                }
                let mut s = Summary::default();
                // A container with a display element is display-qualified as a whole (LG2.19).
                let mut qual = declared_qual(&QTy { ty: ty.clone(), display: false });
                for (i, item) in items.iter().enumerate() {
                    let (v, i_s) = self.at(i, |c| c.expr(item, env))?;
                    s.add(&i_s);
                    let (element, what, key) = match ty {
                        Ty::List(_) => (&elements[0], "a list element", false),
                        Ty::Set(_) => (&elements[0], "a set element", true),
                        _ if i % 2 == 0 => (&elements[0], "a map key", true),
                        _ => (&elements[1], "a map value", false),
                    };
                    if !fits(&element.ty, &v.ty) {
                        self.at(i, |c| c.misfit_slot(&element.ty, &v, what));
                    }
                    if key {
                        // An exact collection index accepts exact inputs only (LG2.20).
                        self.at(i, |c| c.exact_use(Site::Control, &v, what));
                    }
                    qual = qual.join(v.qual.container());
                }
                Ok((Val::new(ty.clone(), qual), s))
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
                self.bare(&ty.ty, false, "a snapshot read")?;
                let mut s = Summary::default();
                let mut own = needs(&[Effect::ReadSnapshot], &[]);
                own.state_reads.insert(StateRead::Snapshot);
                own.rows.insert(Row::Read(query.clone()));
                self.operation(&mut s, own, "a snapshot read");
                self.end_test_read(query, ty);
                let mut v = Val::declared(ty);
                let stored = self.stored.get(query.as_str()).cloned().unwrap_or_default();
                let mut reported = false;
                for field in stored {
                    if !fits(&ty.ty, &field.ty) {
                        // The read is reported once, against the first write it does not fit.
                        if !reported {
                            let what = format!("a read of `{query}`, which a workspace write of the program stores");
                            self.mismatch_of(&field.ty, &ty.ty, |stored, read| fits(read, stored), &what);
                            reported = true;
                        }
                        continue;
                    }
                    // Decoding retains the qualifier the program stores; a read declared with a weaker one would
                    // erase it at the storage boundary (LG2.19).
                    if !admits(ty, field, false) {
                        v.qual = v.qual.join(declared_qual(field).container());
                    }
                    v = decoded(v, ty, field);
                }
                Ok((v, s))
            }
            Expr::ReadSnapshotWith { query, arg, ty } => {
                self.bare(&ty.ty, false, "a snapshot read")?;
                let (av, mut s) = self.at(0, |c| c.expr(arg, env))?;
                // The argument crosses to the host as its canonical bytes (DET.59): portable data, as a goal or
                // evidence takes (owner decision of 16 November 2026 under PHS.52), with no qualifier to erase
                // (LG2.19, LG2.20). An approximation would otherwise reach an exact read's answer.
                let what = format!("the argument of a read of `{query}`");
                if portable(&av.ty) {
                    self.at(0, |c| c.exact_use(Site::Boundary, &av, &what));
                } else {
                    self.at(0, |c| c.misfit(PORTABLE, &av, &what));
                }
                let mut own = needs(&[Effect::ReadSnapshot], &[]);
                own.state_reads.insert(StateRead::Snapshot);
                own.rows.insert(Row::Read(query.clone()));
                self.operation(&mut s, own, "a snapshot read");
                self.end_test_read(query, ty);
                // An engine row, never a workspace record the program stores, so the declared type stands.
                Ok((Val::declared(ty), s))
            }
            Expr::PublishView { value } => {
                let (v, mut s) = self.at(0, |c| c.expr(value, env))?;
                self.sink(&v.ty, "a published view")?;
                self.note_word_view(&v.ty);
                let mut own = needs(&[Effect::ChangeView], &[]);
                own.rows.insert(Row::View);
                self.operation(&mut s, own, "publish_view");
                Ok((Val::exact(Ty::Unit), s))
            }
            Expr::WriteWorkspace { record, ty, value } => {
                self.bare(&ty.ty, false, "a workspace field")?;
                let (v, mut s) = self.at(0, |c| c.expr(value, env))?;
                if fits(&ty.ty, &v.ty) {
                    // Display values may be stored only in typed display fields (LG2.19).
                    self.at(0, |c| c.slot(ty, &v, "a workspace field", true));
                } else {
                    self.at(0, |c| c.misfit_slot(&ty.ty, &v, "a workspace field"));
                }
                let mut own = needs(&[Effect::ChangeWorkspace], &[]);
                own.rows.insert(Row::Workspace(record.clone()));
                self.operation(&mut s, own, "a workspace write");
                Ok((Val::exact(Ty::Unit), s))
            }
            Expr::Propose { word, target } => {
                let (wv, mut s) = self.at(0, |c| c.expr(word, env))?;
                let (tv, t_s) = self.at(1, |c| c.expr(target, env))?;
                s.add(&t_s);
                // A menu word is proposed in its own alphabet (LB1.8; LP2 draft).
                let word_ty = if wv.ty == Ty::MenuWord { Ty::MenuWord } else { Ty::Word };
                for (i, (v, ty, what)) in [(&wv, word_ty, "a preview's word"), (&tv, Ty::Target, "a preview's target")]
                    .into_iter()
                    .enumerate()
                {
                    if fits(&ty, &v.ty) {
                        self.at(i, |c| c.exact_use(Site::Boundary, v, what));
                    } else {
                        self.at(i, |c| c.misfit_slot(&ty, v, what));
                    }
                }
                let mut own = needs(&[Effect::PreparePreview], &[]);
                own.rows.insert(match &**target {
                    Expr::Lit(Lit::Target(t)) => Row::Preview(t.clone()),
                    _ => Row::Unknown,
                });
                self.operation(&mut s, own, "a preview");
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
                    own.rows.insert(Row::Outcome);
                    self.operation(&mut s, own, "a preview outcome read");
                }
                let qual = if rv.ty == Ty::Receipt { rv.qual } else { Qual::Exact };
                Ok((Val::new(ty, qual), s))
            }
            Expr::UseExact { role, value } => {
                let (v, mut s) = self.at(0, |c| c.expr(value, env))?;
                let what = format!("an exact {role:?}");
                let (site, fit, expected) = match role {
                    ExactRole::Budget => (
                        Site::Operand,
                        is_integer(&v.ty) || matches!(v.ty, Ty::Approx(_)) || is_unknown(&v.ty),
                        "Nat | Int | Budget",
                    ),
                    ExactRole::Goal | ExactRole::Evidence => (Site::Boundary, portable(&v.ty), PORTABLE),
                };
                if !fit {
                    self.at(0, |c| c.misfit(expected, &v, &what));
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
                s.display_cost = true;
                Ok((Val::new(ty, Qual::Display { control: true }), s))
            }
            Expr::Declassified { id, ty } => {
                self.bare(ty, false, "a declassified read")?;
                let mut s = Summary::default();
                let mut own = needs(&[Effect::ReadSnapshot], &[]);
                own.state_reads.insert(StateRead::Declassifier(id.clone()));
                own.rows.insert(Row::Declassified(id.clone()));
                self.operation(&mut s, own, "a declassified read");
                Ok((Val::declared(&QTy { ty: ty.clone(), display: false }), s))
            }
            Expr::FileRead { ty, .. } => {
                self.bare(ty, false, "a file read")?;
                let mut s = Summary::default();
                let mut own = needs(&[Effect::GrantedFile], &[Capability::File]);
                own.reads_file = true;
                self.operation(&mut s, own, "a file read");
                Ok((Val::declared(&QTy { ty: ty.clone(), display: false }), s))
            }
            Expr::FileWrite { value, .. } => {
                let (v, mut s) = self.at(0, |c| c.expr(value, env))?;
                self.sink(&v.ty, "a file write")?;
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
            Expr::RngInit { .. } => {
                // A generator on the run's sealed seed and stream path (DET.37, DET.61; LP2 draft).
                let mut s = Summary::default();
                let own = needs(&[Effect::SeededRandomness], &[Capability::Randomness]);
                self.operation(&mut s, own, "a generator's start");
                Ok((Val::exact(Ty::Generator), s))
            }
            Expr::ReadClock => {
                // The clock is a declared requirement without an effect family (LG3.8). Seconds are a display unit
                // only (LG2.6, DET.18), so a reading is display-qualified: a clock grant cannot make it a deterministic
                // budget, an exact decision or an exact sink's value (LG3.8).
                let mut s = Summary::default();
                self.operation(&mut s, needs(&[], &[Capability::Clock]), "a clock read");
                s.display_cost = true;
                Ok((Val::new(Ty::Nat, Qual::Display { control: false }), s))
            }
            Expr::Child { result, body, status, otherwise, .. } => {
                self.bare(&result.ty, true, "a child scope's result")?;
                // The body borrows the names in scope (DET.60). Display data would let approximate control, a budget
                // observation or the clock fix its budget use, and so the child's outcome, which continues into exact
                // computation (LG2.20).
                let mut free = BTreeSet::new();
                free_names(body, &mut Vec::new(), &mut free);
                for name in free {
                    if let Some(v) = env.get(name).filter(|v| matches!(v.qual, Qual::Display { .. })) {
                        let note = format!(
                            "`{name}`: a child scope's body borrows display data, and the child's outcome continues \
                             into exact computation (LG2.20, DET.60; LP2 draft)"
                        );
                        let ty = v.ty.clone();
                        self.at(0, |c| c.exactness(EXACTNESS_VIOLATION, &ty, true, false, note));
                    }
                }
                let (bv, mut s) = self.at(0, |c| c.expr(body, env))?;
                // Reported once, at the innermost child scope whose body holds it.
                if std::mem::take(&mut s.display_cost) {
                    let note =
                        "a child scope's body evaluates display, reads display accounting or the clock, or calls \
                                a function value, so exact inputs alone would not fix its budget use and the child's \
                                outcome, which continues into exact computation (LG2.20, DET.60; LP2 draft)";
                    self.at(0, |c| c.exactness(EXACTNESS_VIOLATION, &result.ty, true, false, note.into()));
                }
                let mut at = vec![0];
                at.extend(returned(body));
                self.result(result, &bv, None, "a child scope's body", &at);
                // The stop branch runs in the parent, with the stop's outcome bound as an exact status when it names
                // one (DET.60).
                let depth = env.vars.len();
                env.vars.extend(status.iter().map(|status| (status.clone(), Val::exact(Ty::Status))));
                let walked = self.at(1, |c| c.expr(otherwise, env));
                env.vars.truncate(depth);
                let (ov, os) = walked?;
                let mut at = vec![1];
                at.extend(returned(otherwise));
                self.result(result, &ov, None, "a child scope's stop branch", &at);
                s.add(&os);
                self.operation(&mut s, needs(&[Effect::Budgets], &[]), "a child scope");
                Ok((Val::declared(result), s))
            }
            Expr::Repeat { count, init, acc, result, body } => {
                self.bare(&result.ty, true, "a loop's value")?;
                // The count bounds the iteration, so approximate data cannot (LG2.20, DET.55; LP2 draft).
                let (cv, mut s) = self.at(0, |c| c.expr(count, env))?;
                if fits(&Ty::Nat, &cv.ty) {
                    self.at(0, |c| c.exact_use(Site::Control, &cv, "a loop bound"));
                } else {
                    self.at(0, |c| c.misfit("Nat", &cv, "a loop bound"));
                }
                let (iv, is) = self.at(1, |c| c.expr(init, env))?;
                let mut at = vec![1];
                at.extend(returned(init));
                self.result(result, &iv, None, "a loop's initial value", &at);
                s.add(&is);
                // Each iteration sees the value so far by its declared type, as a function sees a parameter.
                env.vars.push((acc.clone(), Val::declared(result)));
                let walked = self.at(2, |c| c.expr(body, env));
                env.vars.pop();
                let (bv, bs) = walked?;
                let mut at = vec![2];
                at.extend(returned(body));
                self.result(result, &bv, None, "a loop's body", &at);
                s.add(&bs);
                Ok((Val::declared(result), s))
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
            Prim::Not | Prim::ToApprox(_) | Prim::ToExact | Prim::ApproxNot | Prim::Assert | Prim::RngDraw => 1,
            Prim::Length | Prim::Inverse | Prim::WordKey | Prim::TraceKey | Prim::Compress => 1,
            Prim::Elements(_) | Prim::Keys(_) | Prim::Entries(_) => 1,
            Prim::InsertEntry | Prim::Expand => 3,
            _ => 2,
        };
        if vals.len() != arity {
            return Err(self.err(format!("{op:?} takes {arity} operands, not {}", vals.len())));
        }
        let what = format!("an operand of {op:?}");
        match op {
            Prim::ExactAdd | Prim::ExactLess | Prim::ExactMul => {
                // The result widens along Nat, Int, Rat and Q5; a budget is a cost and mixes with integers only, so an
                // operand that mixes it with a rational or a Q(√5) value does not fit (LG2.6). A product of costs is no
                // cost, so a product takes no budget at all (LP2 draft).
                let costs = op != Prim::ExactMul;
                let mut exact: Vec<&Ty> = Vec::new();
                let mut unknown_operand = false;
                for (i, v) in vals.iter().enumerate() {
                    let budget = exact.contains(&&Ty::Budget);
                    let field = exact.iter().any(|t| matches!(t, Ty::Rat | Ty::Q5));
                    let fit = match &v.ty {
                        Ty::Budget => costs && !field,
                        Ty::Rat | Ty::Q5 => !budget,
                        t => is_exact_number(t) || matches!(t, Ty::Approx(_)) || is_unknown(t),
                    };
                    if !fit {
                        let expected = if budget {
                            "Nat | Int | Budget"
                        } else if field || !costs {
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
                // A compressed word has no exact equality in the LP2 draft: two programs of one expansion differ as
                // values, so a structural answer is no word equality (LG2.16).
                if let Some(i) = vals.iter().position(|v| holds_compressed(&v.ty)) {
                    let note =
                        "an operand of an exact equality, which a compressed word does not have (compare letters)";
                    self.at(i, |c| c.misfit("a type without CompressedWord", &vals[i], note));
                    return Ok(Val::exact(Ty::Bool));
                }
                // A generator has no equality: it is a stream's cursor, not data (LP2 draft; DET.61).
                if let Some(i) = vals.iter().position(|v| holds_generator(&v.ty)) {
                    let note = "an operand of an exact equality, which a generator does not have";
                    self.at(i, |c| c.misfit("a type without Generator", &vals[i], note));
                    return Ok(Val::exact(Ty::Bool));
                }
                if self.verdict_equality(vals) {
                    return Ok(Val::exact(Ty::Bool));
                }
                self.at(0, |c| c.exact_use(Site::Operand, a, what));
                if fits(&a.ty, &b.ty) || fits(&b.ty, &a.ty) {
                    self.at(1, |c| c.exact_use(Site::Operand, b, what));
                } else {
                    self.at(1, |c| c.mismatch_of(&a.ty, &b.ty, |a, b| fits(a, b) || fits(b, a), what));
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
            Prim::Assert => {
                // A pointwise assertion decides whether the run goes on, so its predicate is exact control, as an
                // ordinary condition is (LG2.20; DET.55): an approximate or display-qualified boolean cannot stop a
                // run, and an end test's verdict is no boolean (LG2.9).
                let v = &vals[0];
                if fits(&Ty::Bool, &v.ty) {
                    self.at(0, |c| c.exact_use(Site::Control, v, "a pointwise assertion's predicate"));
                } else {
                    self.at(0, |c| c.misfit("Bool", v, "a pointwise assertion's predicate"));
                }
                Ok(Val::exact(Ty::Unit))
            }
            Prim::RngDraw => {
                // The next block of a generator's stream (DET.61; LP2 draft). The generator is exact data: one chosen
                // under approximate control or display-qualified draws no exact value.
                let v = &vals[0];
                if fits(&Ty::Generator, &v.ty) {
                    self.at(0, |c| c.exact_use(Site::Operand, v, "the generator of a draw"));
                } else {
                    self.at(0, |c| c.misfit("Generator", v, "the generator of a draw"));
                }
                Ok(Val::exact(Ty::Nat))
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
            Prim::Length => {
                let v = &vals[0];
                let measured = matches!(
                    v.ty,
                    Ty::List(_) | Ty::Set(_) | Ty::Map(..) | Ty::Word | Ty::CompressedWord | Ty::MenuWord
                );
                if !measured && !is_unknown(&v.ty) {
                    let wanted = "List<_> | Set<_> | Map<_, _> | Word | CompressedWord | MenuWord";
                    self.at(0, |c| c.misfit(wanted, v, "a length"));
                }
                // A length cannot extract an exact result from a display-qualified value (LG2.19).
                Ok(Val::new(Ty::Nat, project(v.qual, Qual::Exact)))
            }
            Prim::Index => {
                let (list, i) = (&vals[0], &vals[1]);
                // An exact collection index accepts exact inputs only (LG2.20).
                if fits(&Ty::Nat, &i.ty) {
                    self.at(1, |c| c.exact_use(Site::Control, i, "an exact collection index"));
                } else {
                    self.at(1, |c| c.misfit("Nat", i, "an exact collection index"));
                }
                match &list.ty {
                    Ty::List(_) => Ok(list.element(0)),
                    t => {
                        if !is_unknown(t) {
                            self.at(0, |c| c.misfit("List<_>", list, "an indexed list"));
                        }
                        Ok(Val::unknown())
                    }
                }
            }
            Prim::Push => {
                let (list, x) = (&vals[0], &vals[1]);
                let Ty::List(element) = &list.ty else {
                    if !is_unknown(&list.ty) {
                        self.at(0, |c| c.misfit("List<_>", list, "a push"));
                    }
                    return Ok(Val::unknown());
                };
                if !fits(&element.ty, &x.ty) {
                    self.at(1, |c| c.misfit_slot(&element.ty, x, "a pushed element"));
                }
                Ok(Val::new(list.ty.clone(), list.qual.join(x.qual.container())))
            }
            Prim::Concat => {
                let (a, b) = (&vals[0], &vals[1]);
                let ty = match (&a.ty, &b.ty) {
                    (x, y) if is_unknown(x) || is_unknown(y) => return Ok(Val::unknown()),
                    (Ty::Word, _) | (Ty::List(_), _) | (Ty::CompressedWord, _) | (Ty::MenuWord, _) => {
                        let common = if fits(&a.ty, &b.ty) { widen(&a.ty, &b.ty) } else { None };
                        if common.is_none() {
                            let agree = |a: &Ty, b: &Ty| fits(a, b) && widen(a, b).is_some();
                            self.at(1, |c| c.mismatch_of(&a.ty, &b.ty, agree, "a concatenation"));
                            return Ok(Val::unknown());
                        }
                        common
                    }
                    _ => {
                        self.at(0, |c| c.misfit("Word | List<_> | CompressedWord | MenuWord", a, "a concatenation"));
                        return Ok(Val::unknown());
                    }
                };
                Ok(Val::new(ty.unwrap_or_else(unknown), a.qual.join(b.qual)))
            }
            Prim::Inverse | Prim::Conjugate | Prim::Commutator | Prim::Compose | Prim::WordKey | Prim::TraceKey => {
                // One well-formed word domain for every word operation (LG2.15): Full words, or menu words of the
                // run's pinned alphabet (LB1.8; LP2 draft), never a mix. The first operand of a known type sets the
                // domain; a word key and a trace key take Full words only (the menu's trace key waits for its
                // relation, LB1.10).
                let mut domain = matches!(op, Prim::WordKey | Prim::TraceKey).then_some(Ty::Word);
                let mut q = Qual::Exact;
                let mut unknown_operand = false;
                for (i, v) in vals.iter().enumerate() {
                    if is_unknown(&v.ty) {
                        unknown_operand = true;
                    } else {
                        let wanted =
                            domain.get_or_insert_with(|| if v.ty == Ty::MenuWord { Ty::MenuWord } else { Ty::Word });
                        if v.ty != *wanted {
                            let wanted = type_text(wanted);
                            self.at(i, |c| c.misfit(&wanted, v, &what));
                            unknown_operand = true;
                        }
                    }
                    q = q.join(v.qual);
                }
                let ty = match op {
                    Prim::WordKey => Ty::WordEqualityKey,
                    Prim::TraceKey => Ty::TraceEqualityKey,
                    _ => domain.unwrap_or(Ty::Word),
                };
                Ok(if unknown_operand { Val::unknown() } else { Val::new(ty, q) })
            }
            Prim::Contains | Prim::Insert | Prim::InsertEntry | Prim::Lookup => {
                let coll = &vals[0];
                let (key, value) = match (&coll.ty, op) {
                    (Ty::Set(e), Prim::Contains | Prim::Insert) => (&**e, None),
                    (Ty::Map(k, v), Prim::Contains | Prim::InsertEntry | Prim::Lookup) => (&**k, Some(&**v)),
                    (t, _) => {
                        if !is_unknown(t) {
                            let wanted = match op {
                                Prim::Contains => "Set<_> | Map<_, _>",
                                Prim::Insert => "Set<_>",
                                _ => "Map<_, _>",
                            };
                            self.at(0, |c| c.misfit(wanted, coll, &what));
                        }
                        return Ok(Val::unknown());
                    }
                };
                let k = &vals[1];
                if !fits(&key.ty, &k.ty) {
                    self.at(1, |c| c.misfit_slot(&key.ty, k, "a collection key"));
                }
                // An exact collection index accepts exact inputs only (LG2.20).
                self.at(1, |c| c.exact_use(Site::Control, k, "an exact collection key"));
                match (op, value) {
                    // Membership cannot extract an exact result from a display-qualified collection (LG2.19).
                    (Prim::Contains, _) => Ok(Val::new(Ty::Bool, project(coll.qual, Qual::Exact))),
                    (Prim::Lookup, _) => Ok(coll.element(1)),
                    (Prim::InsertEntry, Some(value)) => {
                        let v = &vals[2];
                        if !fits(&value.ty, &v.ty) {
                            self.at(2, |c| c.misfit_slot(&value.ty, v, "a map value"));
                        }
                        Ok(Val::new(coll.ty.clone(), coll.qual.join(k.qual.container()).join(v.qual.container())))
                    }
                    _ => Ok(Val::new(coll.ty.clone(), coll.qual.join(k.qual.container()))),
                }
            }
            Prim::Elements(_) | Prim::Keys(_) | Prim::Entries(_) => {
                // An ordered result names its order in the IR (LG4.3).
                let coll = &vals[0];
                let element = match (&coll.ty, op) {
                    (Ty::Set(e), Prim::Elements(_)) => (**e).clone(),
                    (Ty::Map(k, _), Prim::Keys(_)) => (**k).clone(),
                    (Ty::Map(k, v), Prim::Entries(_)) => {
                        let fields = vec![("key".to_string(), (**k).clone()), ("value".to_string(), (**v).clone())];
                        QTy { ty: Ty::Record(fields), display: false }
                    }
                    (t, _) => {
                        if !is_unknown(t) {
                            let wanted = if matches!(op, Prim::Elements(_)) { "Set<_>" } else { "Map<_, _>" };
                            self.at(0, |c| c.misfit(wanted, coll, &what));
                        }
                        return Ok(Val::unknown());
                    }
                };
                // The list keeps the collection's qualifier (LG2.19).
                Ok(Val::new(Ty::List(Box::new(element)), coll.qual))
            }
            Prim::Compress => {
                let w = &vals[0];
                if is_unknown(&w.ty) {
                    return Ok(Val::unknown());
                }
                if w.ty != Ty::Word {
                    self.at(0, |c| c.misfit("Word", w, "a compressed word's letters"));
                    return Ok(Val::unknown());
                }
                Ok(Val::new(Ty::CompressedWord, w.qual))
            }
            Prim::Expand => {
                let c = &vals[0];
                // Exact positions, like an exact collection index, accept exact inputs only (LG2.20).
                for (i, what) in [(1, "an exact word position"), (2, "an exact letter count")] {
                    let v = &vals[i];
                    if fits(&Ty::Nat, &v.ty) {
                        self.at(i, |s| s.exact_use(Site::Control, v, what));
                    } else {
                        self.at(i, |s| s.misfit("Nat", v, what));
                    }
                }
                if c.ty != Ty::CompressedWord && !is_unknown(&c.ty) {
                    self.at(0, |s| s.misfit("CompressedWord", c, "an expanded word"));
                    return Ok(Val::unknown());
                }
                // An expansion cannot extract exact letters from a display-qualified word (LG2.19).
                Ok(Val::new(Ty::Word, project(c.qual, Qual::Exact)))
            }
        }
    }
}
