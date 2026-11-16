//! The facts of chapter 06: grades, statements, scopes, keys, search records, producing jobs and leaves.
//!
//! Names follow chapter 06 sections 1 and 2 and the wire names of FP.19 and FP.20. Every enumeration that a decoded
//! record can carry has an `Other` variant: a value outside the versioned enumeration is kept as it was read and reads
//! as I1 (EVD.40). The library never invents a value the record did not state.

use std::collections::BTreeSet;

/// EVD.1, EVD.2: what a result rests on.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Basis {
    Theorem,
    Certificate,
    ExhaustiveComputation,
    ExactComputation,
    Sample,
    /// A decoded value outside edition 1's enumeration (I1).
    Other(String),
}

impl Basis {
    /// The five values of EVD.1.
    pub fn all() -> [Basis; 5] {
        [Basis::Theorem, Basis::Certificate, Basis::ExhaustiveComputation, Basis::ExactComputation, Basis::Sample]
    }

    /// FP.19's wire name.
    pub fn wire(&self) -> &str {
        match self {
            Basis::Theorem => "theorem",
            Basis::Certificate => "certificate",
            Basis::ExhaustiveComputation => "exhaustive_computation",
            Basis::ExactComputation => "exact_computation",
            Basis::Sample => "sample",
            Basis::Other(s) => s,
        }
    }

    /// Reads a wire name; an unknown name is kept as `Other` (I1 when read).
    pub fn from_wire(s: &str) -> Basis {
        match s {
            "theorem" => Basis::Theorem,
            "certificate" => Basis::Certificate,
            "exhaustive_computation" => Basis::ExhaustiveComputation,
            "exact_computation" => Basis::ExactComputation,
            "sample" => Basis::Sample,
            other => Basis::Other(other.to_string()),
        }
    }

    pub fn is_known(&self) -> bool {
        !matches!(self, Basis::Other(_))
    }
}

/// EVD.1, EVD.3: who confirmed a result.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Check {
    None,
    Producer,
    IndependentChecker,
    ModelReview,
    HumanReview,
    FormalProof,
    /// A decoded value outside edition 1's enumeration (I1).
    Other(String),
}

impl Check {
    /// The six values of EVD.1.
    pub fn all() -> [Check; 6] {
        [Check::None, Check::Producer, Check::IndependentChecker, Check::ModelReview, Check::HumanReview, Check::FormalProof]
    }

    /// FP.19's wire name.
    pub fn wire(&self) -> &str {
        match self {
            Check::None => "none",
            Check::Producer => "producer",
            Check::IndependentChecker => "independent_checker",
            Check::ModelReview => "model_review",
            Check::HumanReview => "human_review",
            Check::FormalProof => "formal_proof",
            Check::Other(s) => s,
        }
    }

    /// Reads a wire name; an unknown name is kept as `Other` (I1 when read).
    pub fn from_wire(s: &str) -> Check {
        match s {
            "none" => Check::None,
            "producer" => Check::Producer,
            "independent_checker" => Check::IndependentChecker,
            "model_review" => Check::ModelReview,
            "human_review" => Check::HumanReview,
            "formal_proof" => Check::FormalProof,
            other => Check::Other(other.to_string()),
        }
    }

    pub fn is_known(&self) -> bool {
        !matches!(self, Check::Other(_))
    }
}

/// EVD.19: a grade is a pair (basis, check).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Grade {
    pub basis: Basis,
    pub check: Check,
}

impl Grade {
    pub fn new(basis: Basis, check: Check) -> Grade {
        Grade { basis, check }
    }

    /// The 30 pairs of Basis x Check of edition 1.
    pub fn all() -> Vec<Grade> {
        let mut out = Vec::new();
        for b in Basis::all() {
            for k in Check::all() {
                out.push(Grade::new(b.clone(), k));
            }
        }
        out
    }

    pub fn is_known(&self) -> bool {
        self.basis.is_known() && self.check.is_known()
    }
}

/// EVD.9: the outer quantifier of a mixed statement.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Quantifier {
    Forall,
    Exists,
}

/// EVD.9, EVD.13: statement form decides what can refute a claim.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Form {
    Finite,
    Universal,
    Existence,
    Mixed(Quantifier),
    /// A decoded value outside edition 1's enumeration (I1).
    Other(String),
}

/// Section 2: a stage-qualified, commit-pinned name, or the digest of a typed statement (FP.19).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StatementId {
    Named { stage: String, commit: String, name: String },
    Typed { content: String },
}

impl StatementId {
    pub fn named(stage: &str, commit: &str, name: &str) -> StatementId {
        StatementId::Named { stage: stage.to_string(), commit: commit.to_string(), name: name.to_string() }
    }

    pub fn typed(content: &str) -> StatementId {
        StatementId::Typed { content: content.to_string() }
    }
}

/// KN1.7: the identity levels of a puzzle. They form a partial order, not a chain.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Pose,
    StickerClass,
    Labels,
    RegionClass,
    Position,
    Coset,
    TreeData,
    Shape,
    /// A decoded value outside edition 1's enumeration (I1).
    Other(String),
}

impl Level {
    /// The direct maps of KN1.7, from a finer level to a coarser one. Labels stand beside the sticker class (the map
    /// has no kernel on the lattice) and position beside the region class ("as the region class"), so both pairs are
    /// listed in both directions. Draft reading, recorded in the crate documentation.
    fn coarser_neighbours(&self) -> &'static [Level] {
        match self {
            Level::Pose => &[Level::StickerClass],
            Level::StickerClass => &[Level::Labels, Level::RegionClass],
            Level::Labels => &[Level::StickerClass, Level::Position],
            Level::RegionClass => &[Level::Coset, Level::Shape, Level::Position],
            Level::Position => &[Level::RegionClass],
            Level::Coset => &[Level::TreeData],
            Level::TreeData | Level::Shape | Level::Other(_) => &[],
        }
    }

    /// EVD.11: true when `self` is at least as fine as `other`, that is when a chain of KN1.7 maps leads from `self`
    /// to `other`. An `Other` level is comparable with nothing but itself.
    pub fn at_least_as_fine_as(&self, other: &Level) -> bool {
        if self == other {
            return true;
        }
        let mut seen: Vec<Level> = vec![self.clone()];
        let mut stack: Vec<Level> = vec![self.clone()];
        while let Some(l) = stack.pop() {
            for n in l.coarser_neighbours() {
                if n == other {
                    return true;
                }
                if !seen.contains(n) {
                    seen.push(n.clone());
                    stack.push(n.clone());
                }
            }
        }
        false
    }
}

/// FP.8's digest roles, plus the KN1.7 level a state digest names (EVD.11, KN1.8).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DigestRole {
    Poses,
    Labels,
    /// KN1.8: optional, and no such digest exists yet. Kept so a scope can name it.
    StickerClass,
    Bytes,
    Content,
    Source,
    Other(String),
}

/// A digest that names its level or role (FP.8, KN1.8). Two digests are identical only when role and value agree.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DigestRef {
    pub role: DigestRole,
    pub sha256: String,
}

impl DigestRef {
    pub fn new(role: DigestRole, sha256: &str) -> DigestRef {
        DigestRef { role, sha256: sha256.to_string() }
    }

    pub fn poses(sha256: &str) -> DigestRef {
        DigestRef::new(DigestRole::Poses, sha256)
    }

    pub fn labels(sha256: &str) -> DigestRef {
        DigestRef::new(DigestRole::Labels, sha256)
    }

    pub fn source(sha256: &str) -> DigestRef {
        DigestRef::new(DigestRole::Source, sha256)
    }

    /// A source or module digest of pinned source bytes: SHA-256 over the bytes, as `c600-syntax`'s `parse` sets a
    /// module digest (FP.8 role `source`).
    pub fn of_source_bytes(bytes: &[u8]) -> DigestRef {
        DigestRef::new(DigestRole::Source, &c600_digest::sha256_hex(bytes))
    }

    /// The KN1.7 level a state digest names; `None` for roles that are not a state level (bytes, content, source).
    pub fn level(&self) -> Option<Level> {
        match self.role {
            DigestRole::Poses => Some(Level::Pose),
            DigestRole::Labels => Some(Level::Labels),
            DigestRole::StickerClass => Some(Level::StickerClass),
            _ => None,
        }
    }
}

/// Section 2: what a scope covers.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Covers {
    OneInstance,
    SearchSet { family: String, alphabet: String, bounds: String },
    AdmittedDomain { name: String, parameters: String },
    EveryReachable { puzzle: String },
    /// A decoded value outside edition 1's enumeration (I1).
    Other(String),
}

impl Covers {
    pub fn search_set(family: &str, alphabet: &str, bounds: &str) -> Covers {
        Covers::SearchSet { family: family.to_string(), alphabet: alphabet.to_string(), bounds: bounds.to_string() }
    }

    pub fn admitted_domain(name: &str, parameters: &str) -> Covers {
        Covers::AdmittedDomain { name: name.to_string(), parameters: parameters.to_string() }
    }

    pub fn every_reachable(puzzle: &str) -> Covers {
        Covers::EveryReachable { puzzle: puzzle.to_string() }
    }

    /// EVD.32 rows 2 and 3: one finite instance or a declared search set.
    pub fn is_finite_kind(&self) -> bool {
        matches!(self, Covers::OneInstance | Covers::SearchSet { .. })
    }
}

/// Section 2: every field is immutable and none is revision-bound context (EVD.10, EVD.12).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Scope {
    pub covers: Covers,
    /// The KN0.1 identity block, kept as its wire text.
    pub identity: String,
    pub contract: String,
    pub input: Option<DigestRef>,
    pub sources: Vec<DigestRef>,
}

impl Scope {
    pub fn new(covers: Covers, identity: &str, contract: &str, input: Option<DigestRef>) -> Scope {
        Scope { covers, identity: identity.to_string(), contract: contract.to_string(), input, sources: Vec::new() }
    }

    pub fn with_sources(mut self, sources: Vec<DigestRef>) -> Scope {
        self.sources = sources;
        self
    }
}

/// EVD.7: a statement ID with its scope. Everything that discharges, refutes or transports works on keys.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key {
    pub statement: StatementId,
    pub scope: Scope,
}

impl Key {
    pub fn new(statement: StatementId, scope: Scope) -> Key {
        Key { statement, scope }
    }
}

/// Section 2: a premise class, which is also the class of a guard (EVD.36).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PremiseClass {
    ReachabilityPromise,
    OpenHypothesis,
    ModelAssumption,
    /// A decoded value outside edition 1's enumeration (I1).
    Other(String),
}

/// EVD.36: a hypothesis the statement's own text holds.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Guard {
    pub class: PremiseClass,
    pub name: String,
}

/// Section 2: the statement facts. `form` and `subject` are optional facts: the label table has no producer for
/// `form` (section 13), and EVD.41 says how an absent form reads.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Statement {
    pub id: StatementId,
    pub form: Option<Form>,
    pub negation: Option<StatementId>,
    pub subject: Option<Level>,
    pub premises: Vec<String>,
    pub guards: Vec<Guard>,
    /// EVD.32 row 7: the source label a recorded-only claim keeps.
    pub source_label: Option<String>,
    /// Recorded references to other keys, such as the original of a fork (LB6.3). Data only: never an edge.
    pub references: Vec<Key>,
}

impl Statement {
    pub fn new(id: StatementId) -> Statement {
        Statement {
            id,
            form: None,
            negation: None,
            subject: None,
            premises: Vec::new(),
            guards: Vec::new(),
            source_label: None,
            references: Vec::new(),
        }
    }

    pub fn form(mut self, form: Form) -> Statement {
        self.form = Some(form);
        self
    }

    pub fn negation(mut self, negation: &StatementId) -> Statement {
        self.negation = Some(negation.clone());
        self
    }

    pub fn subject(mut self, level: Level) -> Statement {
        self.subject = Some(level);
        self
    }

    pub fn guard(mut self, class: PremiseClass, name: &str) -> Statement {
        self.guards.push(Guard { class, name: name.to_string() });
        self
    }

    pub fn label(mut self, label: &str) -> Statement {
        self.source_label = Some(label.to_string());
        self
    }

    pub fn reference(mut self, key: &Key) -> Statement {
        self.references.push(key.clone());
        self
    }
}

/// SearchRecord.declared.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Declared {
    pub family: String,
    pub alphabet: String,
    pub bounds: String,
}

/// SearchRecord.complete.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Complete {
    Yes,
    No,
    Other(String),
}

/// SearchRecord.stop.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stop {
    Closed,
    DeterministicBudget,
    DeclaredLimit,
    /// `other(reason)`: a wall-clock, resource or product-ceiling stop, or a stop by a person (EVD.35, ERR.16).
    Other(String),
}

/// KN7.3's job outcomes with EVD.47's two import values.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JobOutcome {
    Completed,
    Cancelled,
    CutOff,
    Error,
    NotRun,
    NotStated,
    /// A decoded value outside edition 1's enumeration (I1).
    Other(String),
}

impl JobOutcome {
    pub fn is_known(&self) -> bool {
        !matches!(self, JobOutcome::Other(_))
    }

    /// I5's list: cancelled, cut off, error or not run.
    pub fn did_not_complete(&self) -> bool {
        matches!(self, JobOutcome::Cancelled | JobOutcome::CutOff | JobOutcome::Error | JobOutcome::NotRun)
    }
}

/// Section 2's SearchRecord. `target` is the existence statement shared by the pair {E, neg E}.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SearchRecord {
    pub target: StatementId,
    pub declared: Declared,
    pub complete: Complete,
    pub stop: Stop,
    pub job: JobOutcome,
}

impl SearchRecord {
    pub fn new(target: &StatementId, declared: &Covers, complete: Complete, stop: Stop, job: JobOutcome) -> SearchRecord {
        let declared = match declared {
            Covers::SearchSet { family, alphabet, bounds } => {
                Declared { family: family.clone(), alphabet: alphabet.clone(), bounds: bounds.clone() }
            }
            other => Declared { family: format!("{other:?}"), alphabet: String::new(), bounds: String::new() },
        };
        SearchRecord { target: target.clone(), declared, complete, stop, job }
    }
}

/// EVD.61: binds one leaf to the job that produced it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProducingJob {
    /// Digest of the captured source bytes.
    pub source: String,
    /// Exact pointer into the captured source.
    pub pointer: String,
    pub id: String,
    pub outcome: JobOutcome,
}

impl ProducingJob {
    pub fn new(id: &str, outcome: JobOutcome) -> ProducingJob {
        ProducingJob { source: String::new(), pointer: String::new(), id: id.to_string(), outcome }
    }
}

/// Section 2's ScopeMembership: the checked fact that an instance lies in a key's scope.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Membership {
    pub key: Key,
    pub member: String,
    pub check: Grade,
}

/// EVD.4 and EVD.47: what a check covered. No word reads it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CheckMode {
    ReusedExactTransport,
    FreshExactAudit,
    ActualReplay,
    /// EVD.47's import value "stated: not replayed".
    StatedNotReplayed,
    Other(String),
}

/// EVD.13 and FP.19's CheckFact, without the report blob and result value.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CheckFact {
    /// Tool digest (EVD.28).
    pub tool: String,
    /// The declared common base (EVD.28).
    pub common_base: Vec<String>,
    pub kind: String,
    pub mode: CheckMode,
    /// A transport edge: the node of a transport lemma leaf. P13 never sets it (EVD.52).
    pub transport: Option<usize>,
}

/// Section 2's Covered, computed from the leaf's own record (EVD.59).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Covered {
    pub covers: Covers,
    pub input: Option<DigestRef>,
}

impl Covered {
    pub fn new(covers: Covers, input: Option<DigestRef>) -> Covered {
        Covered { covers, input }
    }

    /// The covered set a statement's own scope names.
    pub fn of_scope(scope: &Scope) -> Covered {
        Covered { covers: scope.covers.clone(), input: scope.input.clone() }
    }
}

/// KN1.16, EVD.18: the checked relation Z = X.gamma a transport lemma names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Relation {
    pub from: DigestRef,
    pub to: DigestRef,
    pub gauge: String,
}

/// What a leaf does in its argument. Draft (crate documentation): chapter 06 names witnesses and transport lemmas
/// but stores no role field, so the library keeps the role beside the leaf.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LeafRole {
    Support,
    /// The witness x of a mixed(forall) refutation (EVD.42).
    Witness,
    /// The transport lemma leaf of a reused leaf (EVD.4, EVD.18).
    Transport(Relation),
}

/// EVD.15 and FP.20: one piece of support with its grade.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Leaf {
    /// The key the leaf was recorded for (FP.20).
    pub key: Key,
    pub grade: Grade,
    /// The identity fields compared with the statement's scope (I2): the KN0.1 identity block.
    pub identity: String,
    /// Leaf identity for independence (EVD.28): record, certificate and tool digests.
    pub support: Vec<String>,
    pub covered: Option<Covered>,
    pub checks: Vec<CheckFact>,
    pub membership: Vec<Membership>,
    pub search: Option<SearchRecord>,
    pub producing_job: Option<ProducingJob>,
    pub role: LeafRole,
    /// Draft: a certificate whose checker states it complete for its stated inputs (EVD.42 "complete certificate").
    pub complete_certificate: bool,
    /// EVD.31: a failed check. The leaf is kept for display and is never live.
    pub failed: bool,
}

impl Leaf {
    /// A leaf for `key` with the key's identity block, no covered set and no job.
    pub fn new(key: &Key, grade: Grade) -> Leaf {
        Leaf {
            key: key.clone(),
            grade,
            identity: key.scope.identity.clone(),
            support: Vec::new(),
            covered: None,
            checks: Vec::new(),
            membership: Vec::new(),
            search: None,
            producing_job: None,
            role: LeafRole::Support,
            complete_certificate: false,
            failed: false,
        }
    }

    pub fn covered(mut self, covered: Covered) -> Leaf {
        self.covered = Some(covered);
        self
    }

    pub fn search(mut self, search: SearchRecord) -> Leaf {
        self.search = Some(search);
        self
    }

    pub fn job(mut self, job: ProducingJob) -> Leaf {
        self.producing_job = Some(job);
        self
    }

    pub fn member_of(mut self, key: &Key, member: &str, check: Grade) -> Leaf {
        self.membership.push(Membership { key: key.clone(), member: member.to_string(), check });
        self
    }

    pub fn witness(mut self) -> Leaf {
        self.role = LeafRole::Witness;
        self
    }

    pub fn transport(mut self, relation: Relation) -> Leaf {
        self.role = LeafRole::Transport(relation);
        self
    }

    pub fn complete(mut self) -> Leaf {
        self.complete_certificate = true;
        self
    }

    pub fn identity(mut self, identity: &str) -> Leaf {
        self.identity = identity.to_string();
        self
    }

    pub fn support(mut self, digest: &str) -> Leaf {
        self.support.push(digest.to_string());
        self
    }

    pub fn check_fact(mut self, fact: CheckFact) -> Leaf {
        self.checks.push(fact);
        self
    }
}

/// Section 2's Premise. A recorded discharge edge to a different key is I2.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Premise {
    pub key: Key,
    pub class: PremiseClass,
    pub discharged_by: Option<Key>,
}

/// The grades of a set, as a sorted set.
pub fn grade_set(grades: &[Grade]) -> BTreeSet<Grade> {
    grades.iter().cloned().collect()
}
