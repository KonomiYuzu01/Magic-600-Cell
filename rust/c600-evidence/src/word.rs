//! Sections 4 and 5: admission (EVD.40, EVD.41, EVD.42), the two standing passes (EVD.30), canonical summaries
//! (Theorem C) and the word function (EVD.32). Everything here is pure: it reads a graph and an edition and changes
//! nothing (EVD.54).

use std::collections::{BTreeMap, BTreeSet};

use crate::edition::Edition;
use crate::error::{EvidenceError, Inconsistency};
use crate::graph::{Graph, NegResolution, Node, NodeId};
use crate::model::{
    Basis, Check, Complete, Covered, Covers, Form, Grade, Guard, JobOutcome, Key, Leaf, LeafRole, Level, PremiseClass,
    Quantifier, SearchRecord, Statement, Stop,
};

/// The display word of EVD.32. The word encoding stays open (EVD.55); this enum is the library's draft.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WordKind {
    Refuted,
    FiniteInstance,
    Proved,
    Conditional,
    NotFoundWithinBudget,
    Incomplete,
    Open,
}

impl WordKind {
    pub fn text(&self) -> &'static str {
        match self {
            WordKind::Refuted => "refuted",
            WordKind::FiniteInstance => "finite instance",
            WordKind::Proved => "proved",
            WordKind::Conditional => "conditional",
            WordKind::NotFoundWithinBudget => "not found within budget",
            WordKind::Incomplete => "incomplete",
            WordKind::Open => "open",
        }
    }
}

/// EVD.1's claim field. Row 0's claim is none, so row 0 is an error, never a `Word`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Claim {
    Proved,
    Conditional,
    Refuted,
    Open,
}

/// Diagnostics a word carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Diagnostic {
    /// EVD.26: a pure child cycle is reachable from the statement's OR node.
    ChildCycle,
}

/// EVD.32's result: `Word = { word, claim, scope_phrase?, premises?, source_label? }`. Not a boolean (EVD.57): no
/// conversion to `bool` exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    /// The decision-list row, 1 to 7. Row 0 is returned as an error.
    pub row: u8,
    pub word: WordKind,
    pub claim: Claim,
    /// EVD.36: computed from `covers` and `guards`; every row carries it for the explanation.
    pub scope_phrase: String,
    /// The word cannot be shown without its phrase: row 3, and row 1 reached through EVD.42's finite guard.
    pub phrase_required: bool,
    /// Row 4: the premises the claim is conditional on.
    pub premises: Vec<Key>,
    /// Row 7: a recorded-only claim keeps its source label.
    pub source_label: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Word {
    /// A display form. Draft: EV5 owns teaching and EVD.55 leaves the encoding open.
    pub fn display(&self) -> String {
        if !self.phrase_required {
            return self.word.text().to_string();
        }
        if self.row == 3 {
            format!("{} {}", self.word.text(), self.scope_phrase)
        } else {
            format!("{}, {}", self.word.text(), self.scope_phrase)
        }
    }
}

/// Theorem C's alternatives: a = (L, Pi).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Alternative {
    pub grades: BTreeSet<Grade>,
    pub premises: BTreeSet<Key>,
}

impl Alternative {
    pub fn empty() -> Alternative {
        Alternative { grades: BTreeSet::new(), premises: BTreeSet::new() }
    }
}

/// The two standing passes and Ref for one key (EVD.30).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Standing {
    pub stands0: bool,
    pub stands1: bool,
    pub neg_stands0: bool,
    /// Prem: the premises in Reach(S).
    pub premises: BTreeSet<Key>,
    /// Ref = {p : stands0(neg p)}.
    pub refuted: BTreeSet<Key>,
}

/// EVD.36: the scope phrase from `covers` and `guards`. Draft wording, recorded in the crate documentation.
pub fn scope_phrase(covers: &Covers, guards: &[Guard]) -> String {
    let mut s = match covers {
        Covers::OneInstance => "on one instance".to_string(),
        Covers::SearchSet { family, bounds, .. } => format!("{bounds} of family {family} only"),
        Covers::AdmittedDomain { name, parameters } => {
            if parameters.is_empty() {
                format!("on {name}")
            } else {
                format!("on {name} ({parameters})")
            }
        }
        Covers::EveryReachable { puzzle } => format!("for every reachable state of {puzzle}"),
        Covers::Other(x) => format!("on {x}"),
    };
    for g in guards {
        s.push_str(", ");
        match &g.class {
            PremiseClass::ReachabilityPromise => s.push_str("for reachable inputs"),
            PremiseClass::OpenHypothesis => s.push_str(&format!("given {}", g.name)),
            PremiseClass::ModelAssumption => s.push_str(&format!("under {}", g.name)),
            PremiseClass::Other(_) => s.push_str(&g.name),
        }
    }
    s
}

fn computed_basis(b: &Basis) -> bool {
    matches!(b, Basis::Certificate | Basis::ExhaustiveComputation | Basis::ExactComputation)
}

/// The outcome of the leaf's own producing job: the EVD.61 binding when it states one, else the search record's.
pub fn effective_job(q: &Leaf) -> Option<JobOutcome> {
    match (&q.producing_job, &q.search) {
        (Some(b), _) if b.outcome != JobOutcome::NotStated => Some(b.outcome.clone()),
        (_, Some(r)) => Some(r.job.clone()),
        (Some(b), None) => Some(b.outcome.clone()),
        (None, None) => None,
    }
}

struct Analysis<'g> {
    g: &'g Graph,
    ed: &'g Edition,
    s_root: Option<NodeId>,
    n_root: Option<NodeId>,
    reach: Vec<NodeId>,
    in_reach: Vec<bool>,
    roots: Vec<NodeId>,
    args: BTreeMap<NodeId, Vec<Vec<NodeId>>>,
    contexts: BTreeMap<NodeId, Vec<NodeId>>,
    live: BTreeSet<NodeId>,
    v0: Vec<bool>,
    v1: Vec<bool>,
    premises: BTreeSet<Key>,
    refuted: BTreeSet<Key>,
    findings: Vec<(Inconsistency, String)>,
}

impl<'g> Analysis<'g> {
    fn new(g: &'g Graph, key: &Key, ed: &'g Edition) -> Analysis<'g> {
        let s_root = g.root_of(key);
        let n_root = match g.neg_root(key) {
            NegResolution::One(n) => Some(n),
            _ => None,
        };
        let reach: Vec<NodeId> = g.reach(key).into_iter().collect();
        let mut in_reach = vec![false; g.len()];
        for id in &reach {
            in_reach[*id] = true;
        }
        let mut a = Analysis {
            g,
            ed,
            s_root,
            n_root,
            reach,
            in_reach,
            roots: Vec::new(),
            args: BTreeMap::new(),
            contexts: BTreeMap::new(),
            live: BTreeSet::new(),
            v0: vec![false; g.len()],
            v1: vec![false; g.len()],
            premises: BTreeSet::new(),
            refuted: BTreeSet::new(),
            findings: Vec::new(),
        };
        if g.neg_root(key) == NegResolution::Ambiguous {
            a.findings.push((Inconsistency::I2, "the negation of the statement does not resolve to one key".into()));
        }
        for id in a.reach.clone() {
            if let Node::Or(_) = &g.nodes()[id] {
                a.roots.push(id);
                let args = g.arguments(id);
                for arg in &args {
                    for l in arg {
                        let ctx = a.contexts.entry(*l).or_default();
                        if !ctx.contains(&id) {
                            ctx.push(id);
                        }
                    }
                }
                a.args.insert(id, args);
            }
        }
        a
    }

    fn key(&self, root: NodeId) -> &'g Key {
        match &self.g.nodes()[root] {
            Node::Or(or) => &or.key,
            _ => unreachable!("roots are OR nodes"),
        }
    }

    fn statement(&self, root: NodeId) -> Option<&'g Statement> {
        self.g.statement(&self.key(root).statement)
    }

    fn form(&self, root: NodeId) -> Option<&'g Form> {
        self.statement(root).and_then(|s| s.form.as_ref())
    }

    fn leaf(&self, id: NodeId) -> &'g Leaf {
        match &self.g.nodes()[id] {
            Node::Leaf(q) => q,
            _ => unreachable!("leaf ids name leaves"),
        }
    }

    fn ctx(&self, leaf: NodeId) -> Vec<NodeId> {
        self.contexts.get(&leaf).cloned().unwrap_or_default()
    }

    fn found(&mut self, case: Inconsistency, detail: String) {
        self.findings.push((case, detail));
    }

    fn take_error(&mut self) -> Option<EvidenceError> {
        if self.findings.is_empty() {
            return None;
        }
        let cases = self.findings.iter().map(|(c, _)| *c).collect();
        let detail = self.findings.iter().map(|(c, d)| format!("{c:?}: {d}")).collect::<Vec<_>>().join("; ");
        self.findings.clear();
        Some(EvidenceError::inconsistent(cases, &detail))
    }

    /// The covered set I7 (a) compares, after a transport lemma's checked relation in the same argument (EVD.40).
    fn effective_covered(&self, leaf: NodeId, root: NodeId) -> Option<Covered> {
        let q = self.leaf(leaf);
        let c = q.covered.clone()?;
        for arg in self.args.get(&root).map(|v| v.as_slice()).unwrap_or(&[]) {
            if !arg.contains(&leaf) {
                continue;
            }
            for t in arg {
                if let LeafRole::Transport(rel) = &self.leaf(*t).role {
                    if *t != leaf && c.input.as_ref() == Some(&rel.from) {
                        return Some(Covered { covers: c.covers.clone(), input: Some(rel.to.clone()) });
                    }
                }
            }
        }
        Some(c)
    }

    fn check_search_values(&mut self, r: &SearchRecord, at: &str) {
        if let Complete::Other(v) = &r.complete {
            self.found(Inconsistency::I1, format!("{at}: search complete value {v:?} outside the enumeration"));
        }
        if !r.job.is_known() {
            self.found(Inconsistency::I1, format!("{at}: search job value outside the enumeration"));
        }
    }

    /// EVD.40's admission conditions that need no liveness: I1, I2, I3, I5, I7 and I8.
    fn static_checks(&mut self) {
        for id in self.reach.clone() {
            match &self.g.nodes()[id] {
                Node::Or(or) => {
                    let at = format!("root {id}");
                    if let Covers::Other(v) = &or.key.scope.covers {
                        self.found(Inconsistency::I1, format!("{at}: covers {v:?} outside the enumeration"));
                    }
                    if self.g.neg_root(&or.key) == NegResolution::Ambiguous {
                        self.found(Inconsistency::I2, format!("{at}: negation does not resolve to one key"));
                    }
                    if let Some(st) = self.g.statement(&or.key.statement) {
                        if let Some(Form::Other(v)) = &st.form {
                            self.found(Inconsistency::I1, format!("{at}: form {v:?} outside the enumeration"));
                        }
                        if let Some(Level::Other(v)) = &st.subject {
                            self.found(Inconsistency::I1, format!("{at}: subject level {v:?} outside the enumeration"));
                        }
                        for gd in &st.guards {
                            if let PremiseClass::Other(v) = &gd.class {
                                self.found(
                                    Inconsistency::I1,
                                    format!("{at}: guard class {v:?} outside the enumeration"),
                                );
                            }
                        }
                        if let Some(n) = &st.negation {
                            if let Some(nst) = self.g.statement(n) {
                                if nst.negation.as_ref() != Some(&st.id) {
                                    self.found(Inconsistency::I2, format!("{at}: the negation pair is not mutual"));
                                }
                            }
                        }
                        if let (Some(subject), Some(input)) = (&st.subject, &or.key.scope.input) {
                            if let Some(level) = input.level() {
                                if !level.at_least_as_fine_as(subject) {
                                    self.found(
                                        Inconsistency::I3,
                                        format!(
                                            "{at}: scope digest level {level:?} is coarser than subject {subject:?}"
                                        ),
                                    );
                                }
                            }
                        }
                        if st.form == Some(Form::Finite) && or.key.scope.covers != Covers::OneInstance {
                            self.found(
                                Inconsistency::I7,
                                format!("{at}: a finite statement covers more than one instance"),
                            );
                        }
                    }
                    for r in or.searches.clone() {
                        self.check_search_values(&r, &at);
                    }
                    for j in &or.jobs {
                        if !j.outcome.is_known() {
                            self.found(Inconsistency::I1, format!("{at}: job outcome outside the enumeration"));
                        }
                    }
                }
                Node::And(children) => {
                    if children.is_empty() {
                        self.found(Inconsistency::I8, format!("AND node {id} has no children"));
                    }
                }
                Node::Premise(p) => {
                    if let PremiseClass::Other(v) = &p.class {
                        self.found(Inconsistency::I1, format!("premise {id}: class {v:?} outside the enumeration"));
                    }
                    if let Some(by) = &p.discharged_by {
                        if *by != p.key {
                            self.found(
                                Inconsistency::I2,
                                format!("premise {id}: a discharge edge between different keys"),
                            );
                        }
                    }
                }
                Node::Leaf(_) => self.leaf_static(id),
            }
        }
    }

    fn leaf_static(&mut self, id: NodeId) {
        let q = self.leaf(id);
        let at = format!("leaf {id}");
        if self.ed.grade_is_invalid(&q.grade) {
            self.found(Inconsistency::I1, format!("{at}: grade outside the edition or listed as invalid"));
        }
        for m in &q.membership {
            if self.ed.grade_is_invalid(&m.check) {
                self.found(Inconsistency::I1, format!("{at}: membership grade outside the edition or invalid"));
            }
        }
        for c in &q.checks {
            if let crate::model::CheckMode::Other(v) = &c.mode {
                self.found(Inconsistency::I1, format!("{at}: check mode {v:?} outside the enumeration"));
            }
        }
        if let Some(c) = &q.covered {
            if let Covers::Other(v) = &c.covers {
                self.found(Inconsistency::I1, format!("{at}: covered {v:?} outside the enumeration"));
            }
        }
        if let Some(r) = &q.search {
            self.check_search_values(r, &at);
        }
        if let Some(b) = &q.producing_job {
            if !b.outcome.is_known() {
                self.found(Inconsistency::I1, format!("{at}: producing job outcome outside the enumeration"));
            }
            if let Some(r) = &q.search {
                if b.outcome != JobOutcome::NotStated && r.job != JobOutcome::NotStated && b.outcome != r.job {
                    self.found(Inconsistency::I1, format!("{at}: the producing job and the search record disagree"));
                }
            }
        }
        let ctx = self.ctx(id);
        for r in &ctx {
            if q.identity != self.key(*r).scope.identity {
                self.found(Inconsistency::I2, format!("{at}: identity fields differ from the statement's scope"));
            }
        }
        // I5: a negative or exhaustive leaf on an incomplete search, and a job-produced negative whose own job did
        // not complete. Draft: a negative leaf is a leaf in an argument of a statement of form universal.
        let negative = ctx.iter().any(|r| self.form(*r) == Some(&Form::Universal));
        let exhaustive = q.grade.basis == Basis::ExhaustiveComputation;
        let job = effective_job(q);
        if negative || exhaustive {
            if let Some(r) = &q.search {
                match r.complete {
                    Complete::No => self
                        .found(Inconsistency::I5, format!("{at}: negative or exhaustive leaf on an incomplete search")),
                    Complete::Yes if job.as_ref().is_some_and(|o| o.did_not_complete()) => {
                        self.found(Inconsistency::I5, format!("{at}: complete = yes with a job that did not complete"))
                    }
                    _ => {}
                }
            }
        }
        if negative {
            if let Some(b) = &q.producing_job {
                if b.outcome.did_not_complete() {
                    self.found(Inconsistency::I5, format!("{at}: a negative from a job that did not complete"));
                }
            }
        }
        // I7 (b)
        if matches!(q.grade.basis, Basis::ExhaustiveComputation | Basis::ExactComputation) {
            if let Some(c) = &q.covered {
                if !c.covers.is_finite_kind() {
                    self.found(
                        Inconsistency::I7,
                        format!("{at}: computed leaf covers neither one instance nor a search set"),
                    );
                }
            }
        }
        // I7 (a)
        if computed_basis(&q.grade.basis) {
            for r in &ctx {
                if matches!(self.form(*r), Some(Form::Universal) | Some(Form::Mixed(Quantifier::Forall))) {
                    if let Some(c) = self.effective_covered(id, *r) {
                        if c != Covered::of_scope(&self.key(*r).scope) {
                            self.found(
                                Inconsistency::I7,
                                format!("{at}: covered set not identical to the universal statement's scope"),
                            );
                        }
                    }
                }
            }
        }
    }

    /// EVD.41 and EVD.31: which leaves are live.
    fn compute_live(&mut self) {
        for id in self.reach.clone() {
            if let Node::Leaf(_) = &self.g.nodes()[id] {
                if self.leaf_is_live(id) {
                    self.live.insert(id);
                }
            }
        }
    }

    fn leaf_is_live(&self, id: NodeId) -> bool {
        let q = self.leaf(id);
        if q.failed {
            return false;
        }
        let own_not_run = match (&q.producing_job, &q.search) {
            (Some(b), _) if b.outcome == JobOutcome::NotRun => true,
            (Some(b), Some(r)) if b.outcome == JobOutcome::NotStated && r.job == JobOutcome::NotRun => true,
            (None, Some(r)) if r.job == JobOutcome::NotRun => true,
            _ => false,
        };
        if own_not_run {
            return false;
        }
        let ctx = self.ctx(id);
        let negative = ctx.iter().any(|r| self.form(*r) == Some(&Form::Universal));
        let exhaustive = q.grade.basis == Basis::ExhaustiveComputation;
        if (negative || exhaustive) && q.search.is_some() && effective_job(q) == Some(JobOutcome::NotStated) {
            return false;
        }
        if computed_basis(&q.grade.basis) {
            for r in &ctx {
                let form = self.form(*r);
                let read_by_i7a = matches!(form, None | Some(Form::Universal) | Some(Form::Mixed(Quantifier::Forall)));
                if read_by_i7a && q.covered.is_none() {
                    return false;
                }
                if form.is_none() {
                    if let Some(c) = self.effective_covered(id, *r) {
                        if c != Covered::of_scope(&self.key(*r).scope) {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    fn is_proof_or_complete_certificate(q: &Leaf) -> bool {
        q.grade.basis == Basis::Theorem || (q.grade.basis == Basis::Certificate && q.complete_certificate)
    }

    /// EVD.42's admission guard for refuting a plain existence statement E by a completed exhaustive search of
    /// exactly its declared finite set. EVD.59 computes an exhaustive leaf's `covered` from its SearchRecord's
    /// declared set, so that declared set must equal E's `covers` as well as the leaf's stored `covered` set: a
    /// stored set copied from the statement does not stand in for a different declared set (section 9, E3).
    fn exhaustive_guard(&self, q: &Leaf, e: &Key) -> bool {
        let Some(st) = self.g.statement(&e.statement) else {
            return false;
        };
        let Some(r) = &q.search else {
            return false;
        };
        st.form == Some(Form::Existence)
            && matches!(&e.scope.covers, Covers::SearchSet { family, alphabet, bounds }
                if *family == r.declared.family && *alphabet == r.declared.alphabet && *bounds == r.declared.bounds)
            && q.grade.basis == Basis::ExhaustiveComputation
            && r.target == e.statement
            && q.covered.as_ref() == Some(&Covered::of_scope(&e.scope))
            && q.identity == e.scope.identity
            && q.key.scope.contract == e.scope.contract
            && r.complete == Complete::Yes
            && r.stop == Stop::Closed
            && r.job == JobOutcome::Completed
            && q.producing_job.as_ref().is_none_or(|b| b.outcome == JobOutcome::Completed)
    }

    fn complete_negative(q: &Leaf) -> bool {
        Self::is_proof_or_complete_certificate(q)
            || (q.grade.basis == Basis::ExhaustiveComputation
                && q.search.as_ref().is_some_and(|r| {
                    r.complete == Complete::Yes && r.stop == Stop::Closed && r.job == JobOutcome::Completed
                }))
    }

    /// The pairs (X, root of neg X) in Reach(S). X need not have an OR node of its own: a statement with no support
    /// can still be offered a refutation, so a root N in Reach whose negated statement has no root pairs with the key
    /// (negated statement, N's scope).
    fn refutation_pairs(&self) -> Vec<(Key, NodeId)> {
        let mut out = Vec::new();
        for r in &self.roots {
            let k = self.key(*r);
            match self.g.neg_root(k) {
                NegResolution::One(n) if self.in_reach[n] => out.push((k.clone(), n)),
                NegResolution::Absent => {
                    if let Some(neg) = self.g.statement(&k.statement).and_then(|s| s.negation.clone()) {
                        out.push((Key { statement: neg, scope: k.scope.clone() }, *r));
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// I6 (EVD.42) on live leaves only. Every argument of neg X that holds a live leaf of a basis other than sample is
    /// an offered refutation of X. Draft: an argument whose live leaves are all samples offers no refutation, because
    /// a sample does not entail its statement (EVD.2).
    fn refutation_checks(&mut self) {
        for (xkey_owned, n) in self.refutation_pairs() {
            let xkey = &xkey_owned;
            let x = format!("{:?}", xkey.statement);
            let form = self.g.statement(&xkey.statement).and_then(|s| s.form.clone());
            let args = self.args.get(&n).cloned().unwrap_or_default();
            for arg in args {
                let live: Vec<&Leaf> = arg.iter().filter(|l| self.live.contains(l)).map(|l| self.leaf(*l)).collect();
                if live.iter().all(|q| q.grade.basis == Basis::Sample) {
                    continue;
                }
                let ok = match &form {
                    Some(Form::Universal) => live.iter().any(|q| {
                        q.grade.basis == Basis::Theorem
                            || (q.identity == xkey.scope.identity
                                && q.membership
                                    .iter()
                                    .any(|m| m.key == *xkey && m.check.check != Check::None && m.check.is_known()))
                    }),
                    Some(Form::Existence) => {
                        let mut has = false;
                        let mut broken = false;
                        for q in &live {
                            if q.grade.basis == Basis::ExhaustiveComputation {
                                if self.exhaustive_guard(q, xkey) {
                                    has = true;
                                } else {
                                    broken = true;
                                }
                            } else if Self::is_proof_or_complete_certificate(q) {
                                has = true;
                            }
                        }
                        has && !broken
                    }
                    Some(Form::Mixed(Quantifier::Exists)) => {
                        live.iter().any(|q| Self::is_proof_or_complete_certificate(q))
                    }
                    Some(Form::Mixed(Quantifier::Forall)) => {
                        let witness = live.iter().position(|q| q.role == LeafRole::Witness);
                        match witness {
                            Some(w) => live.iter().enumerate().any(|(i, q)| i != w && Self::complete_negative(q)),
                            None => false,
                        }
                    }
                    _ => true,
                };
                if !ok {
                    self.found(Inconsistency::I6, format!("an offered refutation of {x} fails EVD.42's conditions"));
                }
            }
        }
    }

    /// One pass of EVD.22's least solution at the scenario (distrusted grades, assumed premises).
    fn eval(&self, distrusted: &dyn Fn(&Grade) -> bool, assumed: &BTreeSet<Key>) -> Vec<bool> {
        let mut v = vec![false; self.g.len()];
        loop {
            let mut changed = false;
            for id in &self.reach {
                if v[*id] {
                    continue;
                }
                let nv = match &self.g.nodes()[*id] {
                    Node::Leaf(q) => self.live.contains(id) && !distrusted(&q.grade),
                    Node::Premise(p) => assumed.contains(&p.key) || self.g.root_of(&p.key).is_some_and(|r| v[r]),
                    Node::And(children) => children.iter().all(|c| v[*c]),
                    Node::Or(or) => or.children.iter().any(|c| v[*c]),
                };
                if nv {
                    v[*id] = true;
                    changed = true;
                }
            }
            if !changed {
                return v;
            }
        }
    }

    /// EVD.30: omega0 = (P minus U, empty), Ref, omega1 = (P minus U, Prem minus Ref).
    fn passes(&mut self) {
        let ed = self.ed;
        let distrusted = |g: &Grade| !ed.passes(g);
        self.v0 = self.eval(&distrusted, &BTreeSet::new());
        let mut prem = BTreeSet::new();
        for id in &self.reach {
            if let Node::Premise(p) = &self.g.nodes()[*id] {
                prem.insert(p.key.clone());
            }
        }
        let mut refuted = BTreeSet::new();
        for p in &prem {
            if let NegResolution::One(n) = self.g.neg_root(p) {
                if self.v0[n] {
                    refuted.insert(p.clone());
                }
            }
        }
        let t1: BTreeSet<Key> = prem.difference(&refuted).cloned().collect();
        self.v1 = self.eval(&distrusted, &t1);
        self.premises = prem;
        self.refuted = refuted;
    }

    /// I4: stands0(K) and stands0(neg K) for a pair in Reach(S).
    fn i4(&mut self) {
        for x in self.roots.clone() {
            if let NegResolution::One(n) = self.g.neg_root(self.key(x)) {
                if self.in_reach[n] && self.v0[x] && self.v0[n] {
                    self.found(Inconsistency::I4, format!("root {x} and its negation both stand"));
                }
            }
        }
    }

    fn pair_roots(&self) -> Vec<NodeId> {
        self.s_root.into_iter().chain(self.n_root).collect()
    }

    /// Search records for S or neg S, each with the outcome of its job.
    fn pair_searches(&self) -> Vec<(SearchRecord, JobOutcome)> {
        let mut out = Vec::new();
        for r in self.pair_roots() {
            if let Node::Or(or) = &self.g.nodes()[r] {
                for s in &or.searches {
                    out.push((s.clone(), s.job.clone()));
                }
            }
            for arg in self.args.get(&r).cloned().unwrap_or_default() {
                for l in arg {
                    let q = self.leaf(l);
                    if let Some(s) = &q.search {
                        out.push((s.clone(), effective_job(q).unwrap_or(JobOutcome::NotStated)));
                    }
                }
            }
        }
        out
    }

    /// EVD.32 row 5.
    fn row5(&self) -> bool {
        self.pair_searches().iter().any(|(r, job)| {
            r.complete == Complete::No
                && *job != JobOutcome::NotRun
                && matches!(r.stop, Stop::DeterministicBudget | Stop::DeclaredLimit)
        })
    }

    /// EVD.32 row 6. Draft: a search job cut off reads row 6 when its stop is `other(reason)`, the wall-clock,
    /// resource or product-ceiling stop of EVD.35 and ERR.16; cut off by a deterministic budget or a declared limit it
    /// is row 5's, and cut off with stop closed it matches no row of its own.
    fn row6(&self) -> bool {
        let interrupted = |o: &JobOutcome| matches!(o, JobOutcome::Cancelled | JobOutcome::Error);
        for (r, job) in self.pair_searches() {
            if interrupted(&job) || (job == JobOutcome::CutOff && matches!(r.stop, Stop::Other(_))) {
                return true;
            }
        }
        for r in self.pair_roots() {
            if let Node::Or(or) = &self.g.nodes()[r] {
                if or.jobs.iter().any(|j| interrupted(&j.outcome) || j.outcome == JobOutcome::CutOff) {
                    return true;
                }
            }
            for arg in self.args.get(&r).cloned().unwrap_or_default() {
                for l in arg {
                    let q = self.leaf(l);
                    if q.search.is_none() {
                        if let Some(b) = &q.producing_job {
                            if interrupted(&b.outcome) || b.outcome == JobOutcome::CutOff {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// EVD.42: neg S stands at omega0 through an OR child that holds a live exhaustive-computation leaf, which I6 has
    /// admitted through the finite exhaustive guard.
    fn refuted_through_guard(&self) -> bool {
        let Some(n) = self.n_root else {
            return false;
        };
        let Node::Or(or) = &self.g.nodes()[n] else {
            return false;
        };
        let args = self.args.get(&n).cloned().unwrap_or_default();
        or.children.iter().zip(args).any(|(c, arg)| {
            self.v0[*c]
                && arg
                    .iter()
                    .any(|l| self.live.contains(l) && self.leaf(*l).grade.basis == Basis::ExhaustiveComputation)
        })
    }

    fn has_child_cycle(&self) -> bool {
        let Some(start) = self.s_root else {
            return false;
        };
        let children = |id: NodeId| -> Vec<NodeId> {
            match &self.g.nodes()[id] {
                Node::Or(or) => or.children.clone(),
                Node::And(c) => c.clone(),
                _ => Vec::new(),
            }
        };
        let mut color = vec![0u8; self.g.len()];
        let mut stack = vec![(start, 0usize)];
        color[start] = 1;
        while let Some((id, idx)) = stack.pop() {
            let ch = children(id);
            if idx < ch.len() {
                stack.push((id, idx + 1));
                let c = ch[idx];
                if color[c] == 1 {
                    return true;
                }
                if color[c] == 0 {
                    color[c] = 1;
                    stack.push((c, 0));
                }
            } else {
                color[id] = 2;
            }
        }
        false
    }

    /// Row 4's named premises. Draft: the premises of the alternatives satisfied at omega1 that do not already hold at
    /// omega0 through their own claim.
    fn named_premises(&self) -> Vec<Key> {
        let Some(s) = self.s_root else {
            return Vec::new();
        };
        let live = |id: NodeId| self.live.contains(&id);
        let sums = compute_summaries(self.g, self.ed, &self.reach, &|id| match &self.g.nodes()[id] {
            Node::Leaf(_) => live(id),
            _ => true,
        });
        let t1: BTreeSet<Key> = self.premises.difference(&self.refuted).cloned().collect();
        let mut out = BTreeSet::new();
        for alt in sums.get(&s).cloned().unwrap_or_default() {
            if alt.grades.iter().all(|g| self.ed.passes(g)) && alt.premises.is_subset(&t1) {
                for p in &alt.premises {
                    let holds0 = self.g.root_of(p).is_some_and(|r| self.v0[r]);
                    if !holds0 {
                        out.insert(p.clone());
                    }
                }
            }
        }
        out.into_iter().collect()
    }
}

fn analyse<'g>(g: &'g Graph, key: &Key, ed: &'g Edition) -> Result<Analysis<'g>, EvidenceError> {
    let mut a = Analysis::new(g, key, ed);
    a.static_checks();
    if let Some(e) = a.take_error() {
        return Err(e);
    }
    a.compute_live();
    a.refutation_checks();
    if let Some(e) = a.take_error() {
        return Err(e);
    }
    a.passes();
    a.i4();
    if let Some(e) = a.take_error() {
        return Err(e);
    }
    Ok(a)
}

/// EVD.32: `word(statement: Key, tables: EvidenceEdition) -> Word`. Effect pure. Row 0 is returned as the error
/// `EVD:evidence.inconsistent_fields` or `EVD:evidence.negative_without_completion`.
pub fn word(g: &Graph, key: &Key, ed: &Edition) -> Result<Word, EvidenceError> {
    let a = analyse(g, key, ed)?;
    let st = g.statement(&key.statement);
    let guards: &[Guard] = st.map(|s| s.guards.as_slice()).unwrap_or(&[]);
    let s0 = a.s_root.is_some_and(|r| a.v0[r]);
    let s1 = a.s_root.is_some_and(|r| a.v1[r]);
    let n0 = a.n_root.is_some_and(|n| a.v0[n]);
    let mut w = Word {
        row: 7,
        word: WordKind::Open,
        claim: Claim::Open,
        scope_phrase: scope_phrase(&key.scope.covers, guards),
        phrase_required: false,
        premises: Vec::new(),
        source_label: None,
        diagnostics: Vec::new(),
    };
    if a.has_child_cycle() {
        w.diagnostics.push(Diagnostic::ChildCycle);
    }
    if n0 {
        w.row = 1;
        w.word = WordKind::Refuted;
        w.claim = Claim::Refuted;
        // EVD.42: row 1 reached through the finite exhaustive guard prints its scope phrase, as row 3 does.
        w.phrase_required = st.and_then(|s| s.form.as_ref()) == Some(&Form::Existence)
            && matches!(key.scope.covers, Covers::SearchSet { .. })
            && a.refuted_through_guard();
    } else if s0 && key.scope.covers.is_finite_kind() {
        w.row = 2;
        w.word = WordKind::FiniteInstance;
        w.claim = Claim::Proved;
    } else if s0 {
        w.row = 3;
        w.word = WordKind::Proved;
        w.claim = Claim::Proved;
        w.phrase_required = true;
    } else if s1 {
        w.row = 4;
        w.word = WordKind::Conditional;
        w.claim = Claim::Conditional;
        w.premises = a.named_premises();
    } else if a.row5() {
        w.row = 5;
        w.word = WordKind::NotFoundWithinBudget;
    } else if a.row6() {
        w.row = 6;
        w.word = WordKind::Incomplete;
    } else {
        w.source_label = st.and_then(|s| s.source_label.clone());
    }
    Ok(w)
}

/// EVD.30: the two passes and Ref for `key`, after admission.
pub fn standing(g: &Graph, key: &Key, ed: &Edition) -> Result<Standing, EvidenceError> {
    let a = analyse(g, key, ed)?;
    Ok(Standing {
        stands0: a.s_root.is_some_and(|r| a.v0[r]),
        stands1: a.s_root.is_some_and(|r| a.v1[r]),
        neg_stands0: a.n_root.is_some_and(|n| a.v0[n]),
        premises: a.premises.clone(),
        refuted: a.refuted.clone(),
    })
}

/// EVD.22: whether the scenario (D, T) lies in val(S), after admission. `distrusted` must be a down-set.
pub fn val_at(
    g: &Graph,
    key: &Key,
    ed: &Edition,
    distrusted: &BTreeSet<Grade>,
    assumed: &BTreeSet<Key>,
) -> Result<bool, EvidenceError> {
    let a = analyse(g, key, ed)?;
    let v = a.eval(&|gr: &Grade| distrusted.contains(gr), assumed);
    Ok(a.s_root.is_some_and(|r| v[r]))
}

/// Theorem C's canonical summary of S, after admission, over Reach(S) with EVD.41's inert leaves withheld.
pub fn summary(g: &Graph, key: &Key, ed: &Edition) -> Result<Vec<Alternative>, EvidenceError> {
    let a = analyse(g, key, ed)?;
    let Some(s) = a.s_root else {
        return Ok(Vec::new());
    };
    let sums = compute_summaries(g, ed, &a.reach, &|id| match &g.nodes()[id] {
        Node::Leaf(_) => a.live.contains(&id),
        _ => true,
    });
    Ok(sums.get(&s).cloned().unwrap_or_default())
}

/// Summaries of every node in `order`, evaluated in that order until the least fixed point (Kleene iteration).
/// `order` must hold every node it references. Every leaf reads as live except failed leaves and leaves whose own
/// producing job was not run; the context rules of EVD.41 apply only through `summary` and `word`.
pub fn summaries_in_order(g: &Graph, ed: &Edition, order: &[NodeId]) -> BTreeMap<NodeId, Vec<Alternative>> {
    compute_summaries(g, ed, order, &|id| match &g.nodes()[id] {
        Node::Leaf(q) => !q.failed && !q.producing_job.as_ref().is_some_and(|b| b.outcome == JobOutcome::NotRun),
        _ => true,
    })
}

fn compute_summaries(
    g: &Graph,
    ed: &Edition,
    order: &[NodeId],
    live: &dyn Fn(NodeId) -> bool,
) -> BTreeMap<NodeId, Vec<Alternative>> {
    let mut s: BTreeMap<NodeId, Vec<Alternative>> = order.iter().map(|i| (*i, Vec::new())).collect();
    loop {
        let mut changed = false;
        for id in order {
            let new = match &g.nodes()[*id] {
                Node::Leaf(q) => {
                    if live(*id) {
                        vec![Alternative { grades: [q.grade.clone()].into_iter().collect(), premises: BTreeSet::new() }]
                    } else {
                        Vec::new()
                    }
                }
                Node::Premise(p) => {
                    let mut v =
                        vec![Alternative { grades: BTreeSet::new(), premises: [p.key.clone()].into_iter().collect() }];
                    if let Some(r) = g.root_of(&p.key) {
                        v.extend(s.get(&r).cloned().unwrap_or_default());
                    }
                    reduce(ed, v)
                }
                Node::And(children) => {
                    let mut acc = vec![Alternative::empty()];
                    for c in children {
                        let cs = s.get(c).cloned().unwrap_or_default();
                        let mut next = Vec::new();
                        for a in &acc {
                            for b in &cs {
                                next.push(Alternative {
                                    grades: a.grades.union(&b.grades).cloned().collect(),
                                    premises: a.premises.union(&b.premises).cloned().collect(),
                                });
                            }
                        }
                        acc = reduce(ed, next);
                    }
                    acc
                }
                Node::Or(or) => {
                    let mut v = Vec::new();
                    for c in &or.children {
                        v.extend(s.get(c).cloned().unwrap_or_default());
                    }
                    reduce(ed, v)
                }
            };
            if s.get(id) != Some(&new) {
                s.insert(*id, new);
                changed = true;
            }
        }
        if !changed {
            return s;
        }
    }
}

/// Theorem B: a dominates b (Sat(b) is inside Sat(a)) exactly when Pi_a is inside Pi_b and L_a is inside the up-set of
/// L_b.
pub fn dominates(ed: &Edition, a: &Alternative, b: &Alternative) -> bool {
    a.premises.is_subset(&b.premises) && a.grades.iter().all(|x| b.grades.iter().any(|y| ed.le(y, x)))
}

/// Theorem A and C: replace each L by its minimal elements and delete dominated alternatives.
pub fn reduce(ed: &Edition, alts: Vec<Alternative>) -> Vec<Alternative> {
    let mut v: Vec<Alternative> = alts
        .into_iter()
        .map(|a| {
            let min = a.grades.iter().filter(|x| !a.grades.iter().any(|y| y != *x && ed.le(y, x))).cloned().collect();
            Alternative { grades: min, premises: a.premises }
        })
        .collect();
    v.sort();
    v.dedup();
    let mut keep = Vec::new();
    for (i, b) in v.iter().enumerate() {
        let dominated =
            v.iter().enumerate().any(|(j, a)| j != i && dominates(ed, a, b) && (!dominates(ed, b, a) || j < i));
        if !dominated {
            keep.push(b.clone());
        }
    }
    keep
}

/// EVD.17, EVD.58: a discharge of `premise` by the claim of `offered`. Identity is by key: another key is refused
/// with `EVD:evidence.premise_unmet` until a bridge lemma gives the premise's own key a standing claim, and so is the
/// identical key when its claim does not stand at omega0.
pub fn discharge(g: &Graph, premise: &Key, offered: &Key, ed: &Edition) -> Result<(), EvidenceError> {
    if premise != offered {
        return Err(EvidenceError::premise_unmet("a premise is discharged only by a claim with the identical key"));
    }
    match standing(g, offered, ed) {
        Ok(s) if s.stands0 => Ok(()),
        _ => Err(EvidenceError::premise_unmet("no claim of the identical key holds the premise")),
    }
}
