//! FR-B08's typed claim bindings and LB5.14's availability preflight (chapter 03 LB5.14 to LB5.17; the LP5 plan's
//! piece R1 and selection 5; the owner's LP5-4 of 23 November 2026).
//!
//! # The edition
//!
//! Edition 1 (`data/claim-bindings-1.json`) has one row per claim group of the frozen claim matrix
//! (`docs/architecture/1.0/language/claim-matrix/matrix.json`, all 510), written from the matrix's anchors by
//! `data/claim_bindings.py`. A row ([`Binding`]) holds:
//!
//! - the group's typed statement ([`TypedStatement`]): the claim its record states at its pointers, on that one record.
//!   Its ID is the SHA-256 of the statement's canonical content (FP.19's typed form), which binds the family, the
//!   record's path and bytes, the pointers, the form and the covered set, so it is injective on these meanings (EVD.8);
//!   its scope covers one instance with the record's bytes as input, and its scope phrase is EVD.36's;
//! - its stored inputs by digest (the matrix's `required_files`, and for stage 18's two chains the stored stop table,
//!   step members and end table), and the inputs a check rebuilds instead of reading ([`Derived`]: stage 18's
//!   intermediate start tables, rebuilt by exact replay under the checking sub-checker's identity, owner's LP5-3);
//! - the KN6 research-archive sub-checkers that read it, by ID (such as `s14.t3p` or `s19.frontier`), with how its
//!   record enters them ([`CheckedAs`]), or none. A sub-checker named [`CheckedAs::ScopeOnly`] reads the record but
//!   checks no claim of the group (it answers KN6's `not_applicable` with the record's own bounds), so it is no
//!   independent checker of the group;
//! - the matrix's own blockers word for word (`matrix_reasons`, `missing`, `historically_incomplete`) and what the
//!   binding resolves (`resolved`): the typed binding resolves `claim_unmapped`, and the rebuilt start tables resolve
//!   stage 18's `dependency_missing`; nothing else is resolved by data;
//! - LB5.17's captured string where the group carries one (q11's `PASS_SHORTENED_THREE_BLOCK_WITNESS_AND_Q11_DEPTH3`),
//!   word for word.
//!
//! The edition is embedded, and its SHA-256 is pinned in [`EDITION_1_SHA256`], so an edit to the data is a new
//! edition. [`Bindings::compare_with_matrix`] checks an edition against the matrix's bytes as granted.
//!
//! # Preflight
//!
//! [`Bindings::check_availability`] is LB5.14's `check_availability`: `ready(checkers, inputs)` or
//! `unavailable(reasons, dependencies)`. It is preflight data, not a `CheckReport`: it reads the binding and the
//! [`Host`]'s answers (which KN6 checkers are registered, and whether each stored input's bytes are held with their
//! pinned digest), runs no check and adds no evidence leaf (EVD.46). `ready` is not a pass; `unavailable` is neither a
//! fail nor a negative and refutes nothing (LB5.14, CX.11). A claim with no row is `unavailable([claim_unmapped], [])`.
//! The answer for a bound group:
//!
//! - the matrix's reasons, less those the binding resolves;
//! - `no_independent_checker` stays unless the row names checkers that check its claim (not [`CheckedAs::ScopeOnly`])
//!   and the host registers every one of them (a missing checker is never named with an invented digest); a group the
//!   matrix answers `unsupported_claim` gets no checker reason, since no check of that kind is supported;
//! - a stored input the host does not hold adds `dependency_missing`, and one with other bytes `dependency_mismatch`;
//! - `historical_check_incomplete` is never resolved: an import or a newer check does not complete a historical gate.
//!
//! The library's `research_preflight.check_availability` (c600-library) states the same rule as language code over
//! host-supplied data.
//!
//! Nothing here reads a file, walks a checkout, runs a process or calls an engine.

use std::collections::{BTreeMap, BTreeSet};

use c600_evidence::{Covers, DigestRef, DigestRole, Form, Key, Scope, Statement, StatementId};
use c600_kernel::json::{object, parse, Value};

use crate::ArchiveError;

/// Edition 1's bytes, embedded so that the edition's content digest is fixed by the build.
pub const EDITION_1: &[u8] = include_bytes!("../data/claim-bindings-1.json");

/// The SHA-256 of [`EDITION_1`]: a change to the data is a new edition, and this pin moves with it.
pub const EDITION_1_SHA256: &str = "313059bdf413b796c94c943b357585f4ad97fc91e978599a88cd33d40da926a0";

/// The frozen claim matrix's format, which edition 1 was written from.
pub const MATRIX_FORMAT: &str = "fr-b08-claim-matrix-draft/1";

/// The format of the typed statement content whose SHA-256 is a row's statement ID.
pub const STATEMENT_FORMAT: &str = "c600-claim-statement/1";

/// LB5.14's reason atoms. They are values, not error codes (CX.11), and their order is LB5.14's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reason {
    ClaimUnmapped,
    DependencyMissing,
    DependencyMismatch,
    NoIndependentChecker,
    UnsupportedClaim,
    HistoricalCheckIncomplete,
}

impl Reason {
    /// Every reason, in LB5.14's order.
    pub const ALL: [Reason; 6] = [
        Reason::ClaimUnmapped,
        Reason::DependencyMissing,
        Reason::DependencyMismatch,
        Reason::NoIndependentChecker,
        Reason::UnsupportedClaim,
        Reason::HistoricalCheckIncomplete,
    ];

    pub fn word(&self) -> &'static str {
        match self {
            Reason::ClaimUnmapped => "claim_unmapped",
            Reason::DependencyMissing => "dependency_missing",
            Reason::DependencyMismatch => "dependency_mismatch",
            Reason::NoIndependentChecker => "no_independent_checker",
            Reason::UnsupportedClaim => "unsupported_claim",
            Reason::HistoricalCheckIncomplete => "historical_check_incomplete",
        }
    }

    pub fn from_word(word: &str) -> Option<Reason> {
        Reason::ALL.into_iter().find(|r| r.word() == word)
    }
}

/// CX.10's data status of a group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataStatus {
    SelfContained,
    NeedsInput,
    RecordOnly,
}

impl DataStatus {
    pub fn word(&self) -> &'static str {
        match self {
            DataStatus::SelfContained => "self_contained",
            DataStatus::NeedsInput => "needs_input",
            DataStatus::RecordOnly => "record_only",
        }
    }

    fn from_word(word: &str) -> Option<DataStatus> {
        [DataStatus::SelfContained, DataStatus::NeedsInput, DataStatus::RecordOnly]
            .into_iter()
            .find(|d| d.word() == word)
    }
}

/// How a group's record enters the check of its named checkers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckedAs {
    /// The record is the checker's certificate.
    Certificate,
    /// The record is an input of the check whose certificate is the row's `via` record (stage 18's step members and
    /// lattice run members).
    Input,
    /// Stage 19: the certificate states the table's `IMPORT-MANIFEST.json` entry (the row's `via`), and the record is
    /// the table.
    Table,
    /// The sub-checker reads the record but checks no claim of the group: it answers KN6's `not_applicable` with the
    /// record's own bounds (no KN6 checker of a search's coverage exists), or, for stage 16's reduced transport at
    /// 7307, checks only the recorded agreement with the unreduced case. The group keeps `no_independent_checker`.
    ScopeOnly,
}

impl CheckedAs {
    pub fn word(&self) -> &'static str {
        match self {
            CheckedAs::Certificate => "certificate",
            CheckedAs::Input => "input",
            CheckedAs::Table => "table",
            CheckedAs::ScopeOnly => "scope_only",
        }
    }

    fn from_word(word: &str) -> Option<CheckedAs> {
        [CheckedAs::Certificate, CheckedAs::Input, CheckedAs::Table, CheckedAs::ScopeOnly]
            .into_iter()
            .find(|c| c.word() == word)
    }

    /// Whether a checker named this way checks the group's claim.
    pub fn checks_the_claim(&self) -> bool {
        !matches!(self, CheckedAs::ScopeOnly)
    }
}

/// A file the edition cites: its repository-relative path, byte count and the SHA-256 of its bytes (Git blob bytes;
/// the matrix's pin where the matrix pins it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePin {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

/// A stored input of a row: a file of the edition's catalogue and the role it plays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputBinding {
    pub file: usize,
    pub role: String,
}

/// An input a check rebuilds rather than reads: stage 18's intermediate start tables, each the state the chain's
/// stored words reach from the stored stop table, with the state digest the record states for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Derived {
    pub pointer: String,
    pub role: String,
    pub digest: String,
    pub rule: String,
}

/// One of the matrix's missing inputs, its name word for word, with the pointers of the record's own declared
/// references that it names (q11: its successor cache).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Missing {
    pub name: String,
    pub cites: Vec<String>,
}

/// A captured string, word for word, at its pointer in the group's record (LB5.17).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub pointer: String,
    pub text: String,
}

/// A row's typed statement: the claim the group's record states at its pointers, on that one record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedStatement {
    /// SHA-256 of the canonical statement content (see [`Bindings::statement_content`]).
    pub id: String,
    /// `finite` for a mathematical group; none for a record-only anchor, which asserts no mathematical result.
    pub form: Option<Form>,
    /// The record's bytes, the scope's input.
    pub input: String,
}

/// One claim group's binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    /// The group's position in the matrix's `groups`.
    pub index: usize,
    /// `family:path#pointers`, unique in the edition.
    pub group: String,
    pub family: String,
    pub stage: u8,
    /// The group's record, in the edition's catalogue.
    pub source: usize,
    /// The matrix's anchors, word for word.
    pub anchors: Vec<String>,
    /// The anchors, or the matrix's leaves for a group with no anchor (the T3P chains' `/stop`, q11).
    pub pointers: Vec<String>,
    pub data_status: DataStatus,
    pub statement: TypedStatement,
    pub scope_phrase: String,
    pub inputs: Vec<InputBinding>,
    pub derived: Vec<Derived>,
    pub via: Option<usize>,
    pub checkers: Vec<String>,
    pub checked_as: Option<CheckedAs>,
    /// The matrix's preflight reasons for the group, word for word.
    pub matrix_reasons: Vec<Reason>,
    pub missing: Vec<Missing>,
    pub historically_incomplete: Vec<String>,
    /// The matrix reasons this binding resolves, each with the name of its basis in the edition's `resolutions`.
    pub resolved: BTreeMap<Reason, String>,
    pub label: Option<Label>,
}

impl Binding {
    /// The checkers that check the group's claim: the named ones, unless they read it [`CheckedAs::ScopeOnly`].
    pub fn claim_checkers(&self) -> &[String] {
        match self.checked_as {
            Some(how) if how.checks_the_claim() => &self.checkers,
            _ => &[],
        }
    }
}

/// A claim family's row of the matrix, kept for display: its assertion, limits and the planned checker's name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Family {
    pub id: String,
    pub stage: u8,
    pub asserts: String,
    pub limits: String,
    pub planned_checker: String,
}

/// The matrix an edition was written from: its path, format, group count and the SHA-256 of its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixPin {
    pub path: String,
    pub format: String,
    pub groups: usize,
    pub sha256: String,
}

/// A versioned edition of claim bindings.
#[derive(Clone, Debug)]
pub struct Bindings {
    pub edition: String,
    /// SHA-256 of the edition's bytes.
    pub sha256: String,
    pub matrix: MatrixPin,
    /// The KN0.1 identity block of the jumbling model the archive's research ran on, as canonical text.
    pub identity: String,
    /// The state-contract revision of that identity.
    pub contract: String,
    /// The bases a row's `resolved` may name, with their text.
    pub resolutions: BTreeMap<String, String>,
    pub families: Vec<Family>,
    pub files: Vec<FilePin>,
    rows: Vec<Binding>,
    by_group: BTreeMap<String, usize>,
}

/// Whether the host holds a stored input's bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputState {
    /// Held, with the pinned SHA-256.
    Present,
    /// Not held.
    Missing,
    /// Held with other bytes.
    Mismatch,
}

/// What preflight asks the host. The host answers from its KN6 registry and the files granted to its import; nothing
/// here decides either.
pub trait Host {
    /// The source digest of the registered KN6 checker with this ID, when one is registered that meets KN6.2 and
    /// KN6.5; `None` otherwise.
    fn checker(&self, id: &str) -> Option<String>;
    /// Whether the host holds this stored input with its pinned SHA-256.
    fn input(&self, file: &FilePin) -> InputState;
}

/// A checker a `ready` answer names, with its source digest as the host reported it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadyChecker {
    pub id: String,
    pub digest: String,
}

/// An input a `ready` answer names: a stored file by digest, or a derived table by the state digest it must reach.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadyInput {
    Stored { role: String, path: String, sha256: String },
    Derived { role: String, pointer: String, digest: String },
}

/// A known blocker of an `unavailable` answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dependency {
    /// A missing input the matrix names, never with a digest, and the pointers of the record's own declared
    /// references it names.
    MissingInput { source: String, name: String, cites: Vec<String> },
    /// A stored input the host does not hold, or holds with other bytes.
    Input { path: String, sha256: String, state: InputState },
    /// A historical gate the archive did not finish, at its pointer in the record (stage 18's gate 3).
    HistoricalGate { source: String, pointer: String },
}

/// LB5.14's answer. Neither variant is a check verdict or an evidence leaf.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Availability {
    Ready { checkers: Vec<ReadyChecker>, inputs: Vec<ReadyInput> },
    Unavailable { reasons: Vec<Reason>, dependencies: Vec<Dependency> },
}

impl Availability {
    pub fn is_ready(&self) -> bool {
        matches!(self, Availability::Ready { .. })
    }

    /// The reasons of an `unavailable` answer; none for `ready`.
    pub fn reasons(&self) -> &[Reason] {
        match self {
            Availability::Ready { .. } => &[],
            Availability::Unavailable { reasons, .. } => reasons,
        }
    }

    pub fn dependencies(&self) -> &[Dependency] {
        match self {
            Availability::Ready { .. } => &[],
            Availability::Unavailable { dependencies, .. } => dependencies,
        }
    }
}

impl Bindings {
    /// Edition 1, embedded in this crate, with its pinned digest.
    pub fn edition_1() -> Bindings {
        let bindings = Bindings::parse(EDITION_1).expect("the embedded claim bindings edition 1 is valid");
        assert_eq!(bindings.sha256, EDITION_1_SHA256, "the embedded edition is the pinned edition 1");
        bindings
    }

    /// Reads an edition. Every row is read in its exact form and checked on its own: its statement ID is its content's
    /// digest, a record-only anchor has no form and no checker, and it resolves only what an edition may resolve (the
    /// typed binding resolves `claim_unmapped`; stage 18's rebuilt start tables, recorded as derived, resolve
    /// `dependency_missing` for a composed or tail-replay group whose checker checks it), each among the matrix's
    /// reasons for the group. A departure refuses the edition.
    pub fn parse(bytes: &[u8]) -> Result<Bindings, ArchiveError> {
        let v = parse(bytes).map_err(|_| bad("the claim bindings are not strict JSON"))?;
        exact_keys(
            &v,
            &[
                "edition",
                "families",
                "files",
                "format",
                "identity",
                "matrix",
                "note",
                "reasons",
                "resolutions",
                "rows",
                "statement_format",
                "version",
            ],
            "the edition",
        )?;
        if text(&v, "format")? != "c600-claim-bindings" || v.get("version") != Some(&Value::Int(1)) {
            return Err(bad("not a claim bindings edition of version 1"));
        }
        if text(&v, "statement_format")? != STATEMENT_FORMAT {
            return Err(bad("the edition names another statement format"));
        }
        let reasons = texts(field(&v, "reasons")?)?;
        if reasons != Reason::ALL.map(|r| r.word()) {
            return Err(bad("the edition's reasons are not LB5.14's, in its order"));
        }
        let identity = field(&v, "identity")?;
        crate::check_identity(identity)?;
        let contract = text(identity, "contract_rev")?;
        let m = field(&v, "matrix")?;
        exact_keys(m, &["format", "groups", "path", "sha256"], "the matrix pin")?;
        let matrix = MatrixPin {
            path: text(m, "path")?,
            format: text(m, "format")?,
            groups: count(m, "groups")?,
            sha256: digest(m, "sha256")?,
        };
        let mut resolutions = BTreeMap::new();
        for (name, value) in field(&v, "resolutions")?.as_object().ok_or(bad("resolutions is not an object"))? {
            resolutions.insert(name.clone(), value.as_str().ok_or(bad("a resolution is not text"))?.to_string());
        }
        let mut families = Vec::new();
        for f in array(&v, "families")? {
            exact_keys(f, &["asserts", "id", "limits", "planned_checker", "stage"], "a family")?;
            families.push(Family {
                id: text(f, "id")?,
                stage: stage(f)?,
                asserts: text(f, "asserts")?,
                limits: text(f, "limits")?,
                planned_checker: text(f, "planned_checker")?,
            });
        }
        let mut files = Vec::new();
        for f in array(&v, "files")? {
            exact_keys(f, &["bytes", "path", "sha256"], "a file")?;
            files.push(FilePin {
                path: text(f, "path")?,
                sha256: digest(f, "sha256")?,
                bytes: count(f, "bytes")? as u64,
            });
        }
        if files.windows(2).any(|w| w[0].path >= w[1].path) {
            return Err(bad("the files are not in path order, once each"));
        }
        let mut rows = Vec::new();
        let mut by_group = BTreeMap::new();
        for (i, r) in array(&v, "rows")?.iter().enumerate() {
            let row = read_row(r, &files, &resolutions)?;
            check_row(&row, &files)?;
            if row.index != i {
                return Err(bad("the rows are not in the matrix's group order"));
            }
            if by_group.insert(row.group.clone(), i).is_some() {
                return Err(bad("two rows bind the same group"));
            }
            rows.push(row);
        }
        if rows.len() != matrix.groups {
            return Err(bad("the edition does not have one row per matrix group"));
        }
        Ok(Bindings {
            edition: text(&v, "edition")?,
            sha256: c600_digest::sha256_hex(bytes),
            matrix,
            identity: identity.canonical(),
            contract,
            resolutions,
            families,
            files,
            rows,
            by_group,
        })
    }

    pub fn rows(&self) -> &[Binding] {
        &self.rows
    }

    /// The row binding this group ID.
    pub fn find(&self, group: &str) -> Option<&Binding> {
        self.by_group.get(group).map(|&i| &self.rows[i])
    }

    pub fn file(&self, index: usize) -> &FilePin {
        &self.files[index]
    }

    pub fn family(&self, id: &str) -> Option<&Family> {
        self.families.iter().find(|f| f.id == id)
    }

    /// The canonical statement content whose SHA-256 is the row's statement ID.
    pub fn statement_content(&self, row: &Binding) -> String {
        statement_content(row, self.file(row.source))
    }

    /// The row's statement as chapter 06's type, with its captured label as the source label a recorded-only claim
    /// keeps (EVD.32 row 7).
    pub fn statement(&self, row: &Binding) -> Statement {
        let mut s = Statement::new(StatementId::typed(&row.statement.id));
        if let Some(form) = &row.statement.form {
            s = s.form(form.clone());
        }
        if let Some(label) = &row.label {
            s = s.label(&label.text);
        }
        s
    }

    /// The row's key: its statement ID with its scope, one instance with the record's bytes as input, under the
    /// jumbling model's identity and contract.
    pub fn key(&self, row: &Binding) -> Key {
        let input = DigestRef::new(DigestRole::Bytes, &row.statement.input);
        Key::new(
            StatementId::typed(&row.statement.id),
            Scope::new(Covers::OneInstance, &self.identity, &self.contract, Some(input)),
        )
    }

    /// LB5.14's preflight for a claim group by its ID. A claim with no row is `unavailable([claim_unmapped], [])`.
    pub fn check_availability(&self, group: &str, host: &dyn Host) -> Availability {
        match self.find(group) {
            Some(row) => self.availability(row, host),
            None => Availability::Unavailable { reasons: vec![Reason::ClaimUnmapped], dependencies: Vec::new() },
        }
    }

    /// LB5.14's preflight for one row (see the module documentation for the rule).
    pub fn availability(&self, row: &Binding, host: &dyn Host) -> Availability {
        let mut reasons: BTreeSet<Reason> =
            row.matrix_reasons.iter().filter(|r| !row.resolved.contains_key(r)).copied().collect();
        let mut checkers = Vec::new();
        if !reasons.contains(&Reason::UnsupportedClaim) {
            let named = row.claim_checkers();
            let found: Option<Vec<ReadyChecker>> = if named.is_empty() {
                None
            } else {
                named.iter().map(|id| host.checker(id).map(|digest| ReadyChecker { id: id.clone(), digest })).collect()
            };
            match found {
                Some(found) => {
                    reasons.remove(&Reason::NoIndependentChecker);
                    checkers = found;
                }
                None => {
                    reasons.insert(Reason::NoIndependentChecker);
                }
            }
        }
        let mut dependencies = Vec::new();
        let mut inputs = Vec::new();
        for (file, role) in self.stored(row) {
            let pin = self.file(file);
            match host.input(pin) {
                InputState::Present => inputs.push(ReadyInput::Stored {
                    role: role.to_string(),
                    path: pin.path.clone(),
                    sha256: pin.sha256.clone(),
                }),
                state => {
                    reasons.insert(match state {
                        InputState::Missing => Reason::DependencyMissing,
                        _ => Reason::DependencyMismatch,
                    });
                    dependencies.push(Dependency::Input { path: pin.path.clone(), sha256: pin.sha256.clone(), state });
                }
            }
        }
        for d in &row.derived {
            inputs.push(ReadyInput::Derived {
                role: d.role.clone(),
                pointer: d.pointer.clone(),
                digest: d.digest.clone(),
            });
        }
        let source = &self.file(row.source).path;
        if !row.resolved.contains_key(&Reason::DependencyMissing) {
            for m in &row.missing {
                dependencies.push(Dependency::MissingInput {
                    source: source.clone(),
                    name: m.name.clone(),
                    cites: m.cites.clone(),
                });
            }
        }
        if reasons.contains(&Reason::HistoricalCheckIncomplete) {
            for pointer in &row.historically_incomplete {
                dependencies.push(Dependency::HistoricalGate { source: source.clone(), pointer: pointer.clone() });
            }
        }
        if reasons.is_empty() {
            Availability::Ready { checkers, inputs }
        } else {
            Availability::Unavailable { reasons: reasons.into_iter().collect(), dependencies }
        }
    }

    /// The stored files a row's check reads, once each: its record, its inputs and its `via` record.
    pub fn stored<'b>(&self, row: &'b Binding) -> Vec<(usize, &'b str)> {
        let mut out: Vec<(usize, &str)> = vec![(row.source, "claim_record")];
        out.extend(row.inputs.iter().map(|i| (i.file, i.role.as_str())));
        if let Some(via) = row.via {
            out.push((via, "via"));
        }
        let mut seen = BTreeSet::new();
        out.retain(|(file, _)| seen.insert(*file));
        out
    }

    /// Compares the edition with the matrix's bytes as granted: the pinned digest and format; one row per group, in
    /// order, with the matrix's family, record and digest, anchors, data status, preflight reasons, missing inputs and
    /// unfinished gates; every required file bound with its pinned digest; each statement ID recomputed; and every
    /// resolution one the edition may make. Returns the problems found, empty when the edition is the matrix's.
    pub fn compare_with_matrix(&self, matrix: &[u8]) -> Result<Vec<String>, ArchiveError> {
        let m = parse(matrix).map_err(|_| bad("the matrix is not strict JSON"))?;
        let mut problems = Vec::new();
        if c600_digest::sha256_hex(matrix) != self.matrix.sha256 {
            problems.push("the matrix's bytes are not the ones the edition pins".to_string());
        }
        if m.get("format").and_then(Value::as_str) != Some(self.matrix.format.as_str())
            || self.matrix.format != MATRIX_FORMAT
        {
            problems.push("the matrix's format is not the pinned one".to_string());
        }
        let files: Vec<(String, String)> = array(&m, "files")?
            .iter()
            .map(|f| Ok((text(f, "path")?, text(f, "sha256")?)))
            .collect::<Result<_, ArchiveError>>()?;
        let file = |v: &Value| -> Result<&(String, String), ArchiveError> {
            v.as_i64().and_then(|i| files.get(i as usize)).ok_or(bad("a matrix file index is out of range"))
        };
        let groups = array(&m, "groups")?;
        if groups.len() != self.rows.len() || groups.len() != self.matrix.groups {
            problems.push(format!("the matrix has {} groups and the edition {} rows", groups.len(), self.rows.len()));
        }
        let fams: BTreeSet<&str> = array(&m, "families")?.iter().filter_map(|f| f.get("id")?.as_str()).collect();
        let ours: BTreeSet<&str> = self.families.iter().map(|f| f.id.as_str()).collect();
        if fams != ours {
            problems.push("the edition's families are not the matrix's".to_string());
        }
        for (g, row) in groups.iter().zip(&self.rows) {
            let at = &row.group;
            let (path, sha) = file(field(g, "source")?)?;
            let source = self.file(row.source);
            if (&source.path, &source.sha256) != (path, sha) {
                problems.push(format!("{at}: another record than the matrix's"));
            }
            if text(g, "family")? != row.family || row.stage.to_string() != row.family[..2] {
                problems.push(format!("{at}: another family than the matrix's"));
            }
            let anchors = texts(field(g, "anchors")?)?;
            if anchors != row.anchors {
                problems.push(format!("{at}: other anchors than the matrix's"));
            }
            let pointers = if anchors.is_empty() { texts(field(g, "leaves")?)? } else { anchors };
            if pointers != row.pointers {
                problems.push(format!("{at}: other pointers than the matrix's anchors or leaves"));
            }
            if text(g, "data_status")? != row.data_status.word() {
                problems.push(format!("{at}: another data status than the matrix's"));
            }
            let outcome = field(g, "outcome")?;
            let reasons = texts(field(outcome, "reasons")?)?;
            let ours: Vec<&str> = row.matrix_reasons.iter().map(Reason::word).collect();
            if reasons != ours || text(outcome, "kind")? != "unavailable" {
                problems.push(format!("{at}: other blockers than the matrix's"));
            }
            let missing = array(g, "missing_inputs")?;
            let names: Vec<String> = missing.iter().map(|x| text(x, "name")).collect::<Result<_, _>>()?;
            let ours: Vec<String> = row.missing.iter().map(|x| x.name.clone()).collect();
            if names != ours {
                problems.push(format!("{at}: other missing inputs than the matrix's"));
            }
            for x in missing {
                if x.get("present") != Some(&Value::Bool(false)) || x.get("digest") != Some(&Value::Null) {
                    problems.push(format!("{at}: a missing input the matrix gives bytes for"));
                }
                if file(field(x, "source")?)?.0 != source.path {
                    problems.push(format!("{at}: a missing input of another record"));
                }
            }
            if texts(field(g, "historically_incomplete")?)? != row.historically_incomplete {
                problems.push(format!("{at}: other unfinished gates than the matrix's"));
            }
            for f in field(g, "required_files")?.as_array().ok_or(bad("required_files is not a list"))? {
                let (path, sha) = file(f)?;
                let bound = self.stored(row).iter().any(|(i, _)| {
                    let pin = self.file(*i);
                    (&pin.path, &pin.sha256) == (path, sha)
                });
                if !bound {
                    problems.push(format!("{at}: the required file {path} is not bound with its pin"));
                }
            }
            if c600_digest::sha256_hex(self.statement_content(row).as_bytes()) != row.statement.id
                || row.statement.input != source.sha256
            {
                problems.push(format!("{at}: the statement ID is not its content's digest"));
            }
            let record_only = row.data_status == DataStatus::RecordOnly;
            if record_only != row.statement.form.is_none() || (record_only && !row.checkers.is_empty()) {
                problems.push(format!("{at}: a record-only anchor with a form or a checker"));
            }
            for (reason, basis) in &row.resolved {
                if !may_resolve(row, *reason, basis) || !row.matrix_reasons.contains(reason) {
                    problems.push(format!("{at}: resolves {} without a basis the edition may use", reason.word()));
                }
            }
            if row.matrix_reasons.contains(&Reason::ClaimUnmapped) && !row.resolved.contains_key(&Reason::ClaimUnmapped)
            {
                problems.push(format!("{at}: a bound group that keeps claim_unmapped"));
            }
        }
        Ok(problems)
    }
}

/// The canonical statement content of a row whose record is `source`.
fn statement_content(row: &Binding, source: &FilePin) -> String {
    let form = match &row.statement.form {
        Some(Form::Finite) => Value::String("finite".into()),
        None => Value::Null,
        Some(_) => Value::String("other".into()),
    };
    object([
        ("covers", Value::String("one_instance".into())),
        ("family", Value::String(row.family.clone())),
        ("form", form),
        ("pointers", Value::Array(row.pointers.iter().map(|p| Value::String(p.clone())).collect())),
        (
            "source",
            object([("path", Value::String(source.path.clone())), ("sha256", Value::String(source.sha256.clone()))]),
        ),
        ("statement", Value::String(STATEMENT_FORMAT.into())),
    ])
    .canonical()
}

/// Whether an edition may resolve `reason` in `row` on `basis`: the typed binding resolves `claim_unmapped`, and
/// stage 18's rebuilt start tables resolve `dependency_missing` for a composed or tail-replay group whose named checker
/// checks it and which records them as derived (owner's LP5-3). Nothing else is resolved by data.
fn may_resolve(row: &Binding, reason: Reason, basis: &str) -> bool {
    match reason {
        Reason::ClaimUnmapped => basis == "typed_binding",
        Reason::DependencyMissing => {
            basis == "rebuilt_start_tables"
                && matches!(row.family.as_str(), "18/composed" | "18/tail_replay")
                && !row.claim_checkers().is_empty()
                && !row.derived.is_empty()
        }
        _ => false,
    }
}

/// A row's own consistency: its statement ID, form and stage, and what it resolves.
fn check_row(row: &Binding, files: &[FilePin]) -> Result<(), ArchiveError> {
    let source = &files[row.source];
    if c600_digest::sha256_hex(statement_content(row, source).as_bytes()) != row.statement.id
        || row.statement.input != source.sha256
    {
        return Err(bad("a row's statement ID is not its content's digest"));
    }
    if row.family.get(..3) != Some(&format!("{}/", row.stage)) {
        return Err(bad("a row's stage is not its family's"));
    }
    let record_only = row.data_status == DataStatus::RecordOnly;
    if record_only != row.statement.form.is_none() || (record_only && !row.checkers.is_empty()) {
        return Err(bad("a record-only anchor with a form or a checker"));
    }
    for (reason, basis) in &row.resolved {
        if !row.matrix_reasons.contains(reason) || !may_resolve(row, *reason, basis) {
            return Err(bad("a row resolves a reason the edition may not resolve"));
        }
    }
    if row.derived.is_empty() == row.resolved.contains_key(&Reason::DependencyMissing) {
        return Err(bad("a row's derived inputs and its resolved dependency_missing disagree"));
    }
    Ok(())
}

fn read_row(r: &Value, files: &[FilePin], resolutions: &BTreeMap<String, String>) -> Result<Binding, ArchiveError> {
    exact_keys(
        r,
        &[
            "anchors",
            "checked_as",
            "checkers",
            "data_status",
            "derived",
            "family",
            "group",
            "historically_incomplete",
            "index",
            "inputs",
            "label",
            "matrix_reasons",
            "missing",
            "pointers",
            "resolved",
            "scope_phrase",
            "source",
            "stage",
            "statement",
            "via",
        ],
        "a row",
    )?;
    let file_index = |v: &Value| -> Result<usize, ArchiveError> {
        v.as_i64()
            .filter(|i| *i >= 0 && (*i as usize) < files.len())
            .map(|i| i as usize)
            .ok_or(bad("a file index is out of range"))
    };
    let s = field(r, "statement")?;
    exact_keys(s, &["covers", "form", "id", "input"], "a statement")?;
    if text(s, "covers")? != "one_instance" {
        return Err(bad("a statement covers another set than one instance"));
    }
    let form = match s.get("form") {
        Some(Value::Null) => None,
        Some(Value::String(f)) if f == "finite" => Some(Form::Finite),
        _ => return Err(bad("a statement's form is neither finite nor null")),
    };
    let statement = TypedStatement { id: digest(s, "id")?, form, input: digest(s, "input")? };
    let inputs = array(r, "inputs")?
        .iter()
        .map(|pair| match pair.as_array() {
            Some([file, Value::String(role)]) => Ok(InputBinding { file: file_index(file)?, role: role.clone() }),
            _ => Err(bad("an input is not a (file, role) pair")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut derived = Vec::new();
    for d in array(r, "derived")? {
        exact_keys(d, &["digest", "pointer", "role", "rule"], "a derived input")?;
        let rule = text(d, "rule")?;
        if !resolutions.contains_key(&rule) {
            return Err(bad("a derived input names a rule the edition does not state"));
        }
        derived.push(Derived {
            pointer: text(d, "pointer")?,
            role: text(d, "role")?,
            digest: digest(d, "digest")?,
            rule,
        });
    }
    let mut missing = Vec::new();
    for x in array(r, "missing")? {
        exact_keys(x, &["cites", "name"], "a missing input")?;
        missing.push(Missing { name: text(x, "name")?, cites: texts(field(x, "cites")?)? });
    }
    let mut resolved = BTreeMap::new();
    for (reason, basis) in field(r, "resolved")?.as_object().ok_or(bad("resolved is not an object"))? {
        let reason = Reason::from_word(reason).ok_or(bad("resolved names no reason of LB5.14"))?;
        let basis = basis.as_str().ok_or(bad("a resolution basis is not text"))?;
        if !resolutions.contains_key(basis) {
            return Err(bad("a row resolves a reason by a basis the edition does not state"));
        }
        resolved.insert(reason, basis.to_string());
    }
    let label = match field(r, "label")? {
        Value::Null => None,
        l => {
            exact_keys(l, &["pointer", "text"], "a label")?;
            Some(Label { pointer: text(l, "pointer")?, text: text(l, "text")? })
        }
    };
    let matrix_reasons = texts(field(r, "matrix_reasons")?)?
        .iter()
        .map(|w| Reason::from_word(w).ok_or(bad("a matrix reason is not one of LB5.14's")))
        .collect::<Result<Vec<_>, _>>()?;
    let checked_as = match field(r, "checked_as")? {
        Value::Null => None,
        v => Some(v.as_str().and_then(CheckedAs::from_word).ok_or(bad("checked_as is unknown"))?),
    };
    let via = match field(r, "via")? {
        Value::Null => None,
        v => Some(file_index(v)?),
    };
    let checkers = texts(field(r, "checkers")?)?;
    if checkers.is_empty() != checked_as.is_none()
        || via.is_some() != matches!(checked_as, Some(CheckedAs::Input | CheckedAs::Table))
    {
        return Err(bad("a row's checkers, checked_as and via disagree"));
    }
    let index = count(r, "index")?;
    Ok(Binding {
        index,
        group: text(r, "group")?,
        family: text(r, "family")?,
        stage: stage(r)?,
        source: file_index(field(r, "source")?)?,
        anchors: texts(field(r, "anchors")?)?,
        pointers: texts(field(r, "pointers")?)?,
        data_status: DataStatus::from_word(&text(r, "data_status")?).ok_or(bad("an unknown data status"))?,
        statement,
        scope_phrase: text(r, "scope_phrase")?,
        inputs,
        derived,
        via,
        checkers,
        checked_as,
        matrix_reasons,
        missing,
        historically_incomplete: texts(field(r, "historically_incomplete")?)?,
        resolved,
        label,
    })
}

fn bad(detail: &str) -> ArchiveError {
    ArchiveError::format(detail)
}

fn field<'v>(v: &'v Value, key: &str) -> Result<&'v Value, ArchiveError> {
    v.get(key).ok_or_else(|| ArchiveError::format(&format!("the field {key} is missing")))
}

fn text(v: &Value, key: &str) -> Result<String, ArchiveError> {
    field(v, key)?.as_str().map(str::to_string).ok_or_else(|| ArchiveError::format(&format!("{key} is not text")))
}

fn digest(v: &Value, key: &str) -> Result<String, ArchiveError> {
    let t = text(v, key)?;
    if crate::is_sha256(&t) {
        Ok(t)
    } else {
        Err(ArchiveError::format(&format!("{key} is not 64 lowercase hex digits")))
    }
}

fn count(v: &Value, key: &str) -> Result<usize, ArchiveError> {
    field(v, key)?
        .as_i64()
        .filter(|n| *n >= 0)
        .map(|n| n as usize)
        .ok_or_else(|| ArchiveError::format(&format!("{key} is not a count")))
}

fn stage(v: &Value) -> Result<u8, ArchiveError> {
    count(v, "stage").ok().filter(|s| (14..=19).contains(s)).map(|s| s as u8).ok_or(bad("a stage outside 14 to 19"))
}

fn array<'v>(v: &'v Value, key: &str) -> Result<&'v [Value], ArchiveError> {
    field(v, key)?.as_array().ok_or_else(|| ArchiveError::format(&format!("{key} is not a list")))
}

fn texts(v: &Value) -> Result<Vec<String>, ArchiveError> {
    v.as_array()
        .ok_or(bad("not a list"))?
        .iter()
        .map(|x| x.as_str().map(str::to_string).ok_or(bad("a list item is not text")))
        .collect()
}

fn exact_keys(v: &Value, keys: &[&str], what: &str) -> Result<(), ArchiveError> {
    let fields = v.as_object().ok_or_else(|| ArchiveError::format(&format!("{what} is not an object")))?;
    if fields.len() != keys.len() || keys.iter().any(|k| !fields.contains_key(*k)) {
        return Err(ArchiveError::format(&format!("{what} does not have exactly its fields")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// q11's group (LB5.17).
    const Q11: &str = "19/q11:research/jumbling/restoration/19-height-depth-framework/records/\
                       type-lex-trimmed-independent-results.json#/q11_depth_certificate";

    /// A host that registers the listed checkers and holds every stored input except the listed paths.
    struct Fixed {
        checkers: Vec<&'static str>,
        missing: Vec<String>,
        mismatched: Vec<String>,
    }

    impl Host for Fixed {
        fn checker(&self, id: &str) -> Option<String> {
            self.checkers.contains(&id).then(|| c600_digest::sha256_hex(id.as_bytes()))
        }

        fn input(&self, file: &FilePin) -> InputState {
            if self.missing.contains(&file.path) {
                InputState::Missing
            } else if self.mismatched.contains(&file.path) {
                InputState::Mismatch
            } else {
                InputState::Present
            }
        }
    }

    /// A host that registers every checker it is asked for and holds every stored input.
    struct Everything;

    impl Host for Everything {
        fn checker(&self, id: &str) -> Option<String> {
            Some(c600_digest::sha256_hex(id.as_bytes()))
        }

        fn input(&self, _: &FilePin) -> InputState {
            InputState::Present
        }
    }

    fn host(checkers: Vec<&'static str>) -> Fixed {
        Fixed { checkers, missing: Vec::new(), mismatched: Vec::new() }
    }

    fn row<'b>(b: &'b Bindings, family: &str, path_end: &str) -> &'b Binding {
        b.rows()
            .iter()
            .find(|r| r.family == family && b.file(r.source).path.ends_with(path_end))
            .unwrap_or_else(|| panic!("no row {family} {path_end}"))
    }

    fn repository() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
    }

    #[test]
    fn edition_1_loads_with_its_pinned_digest() {
        let b = Bindings::edition_1();
        assert_eq!(b.edition, "c600-claim-bindings-1");
        assert_eq!(b.sha256, EDITION_1_SHA256, "a changed edition moves its pin");
        assert_eq!(b.rows().len(), 510);
        assert_eq!(b.matrix.groups, 510);
        assert_eq!(b.matrix.format, MATRIX_FORMAT);
        assert!(!EDITION_1.contains(&b'\r'), "LF only");
        for r in b.rows() {
            assert_eq!(c600_digest::sha256_hex(b.statement_content(r).as_bytes()), r.statement.id, "{}", r.group);
            assert_eq!(b.find(&r.group).map(|x| x.index), Some(r.index));
            assert_eq!(r.scope_phrase, c600_evidence::scope_phrase(&Covers::OneInstance, &[]));
        }
    }

    /// The binding data against the frozen claim matrix, read in place: 510 groups in order, with the matrix's records,
    /// anchors, blockers, missing inputs and unfinished gates; and a matrix that departs is found.
    #[test]
    fn bindings_agree_with_the_claim_matrix_read_in_place() {
        let b = Bindings::edition_1();
        let bytes = std::fs::read(repository().join(&b.matrix.path)).expect("the frozen claim matrix");
        assert_eq!(c600_digest::sha256_hex(&bytes), b.matrix.sha256);
        assert_eq!(b.compare_with_matrix(&bytes).expect("a readable matrix"), Vec::<String>::new());
        let canonical = parse(&bytes).expect("strict JSON").canonical();
        let tail_replay =
            "\"outcome\":{\"kind\":\"unavailable\",\"reasons\":[\"claim_unmapped\",\"dependency_missing\",\
                           \"historical_check_incomplete\",\"no_independent_checker\"]}";
        for (from, to, problem) in [
            (tail_replay, tail_replay.replace("\"historical_check_incomplete\",", ""), "other blockers"),
            ("\"anchors\":[\"/one_fixed_L3\"", "\"anchors\":[\"/one_fixed_L4\"".to_string(), "other anchors"),
            (
                "\"name\":\"type-lex-successor-base-cache.npz and",
                "\"name\":\"type-lex-successor-base-cache and".to_string(),
                "other missing inputs",
            ),
            ("\"data_status\":\"record_only\"", "\"data_status\":\"needs_input\"".to_string(), "another data status"),
        ] {
            assert!(canonical.contains(from), "{from}");
            let changed = canonical.replacen(from, &to, 1);
            let problems = b.compare_with_matrix(changed.as_bytes()).expect("a readable matrix");
            assert!(problems.iter().any(|p| p.contains("not the ones the edition pins")), "{problems:?}");
            assert!(problems.iter().any(|p| p.contains(problem)), "{problem}: {problems:?}");
        }
    }

    #[test]
    fn a_claim_with_no_row_is_claim_unmapped() {
        let b = Bindings::edition_1();
        let a = b.check_availability("19/q11:research/elsewhere.json#/x", &host(vec![]));
        assert_eq!(a, Availability::Unavailable { reasons: vec![Reason::ClaimUnmapped], dependencies: vec![] });
    }

    #[test]
    fn a_named_checker_must_be_registered_and_every_input_held() {
        let b = Bindings::edition_1();
        let r = row(&b, "14/t3p", "s7304-w3-certificate.json");
        assert!(matches!(b.availability(r, &host(vec!["s14.t3p"])), Availability::Ready { .. }));
        assert_eq!(b.availability(r, &host(vec![])).reasons(), [Reason::NoIndependentChecker]);
        assert_eq!(b.availability(r, &host(vec![])).dependencies(), [], "a missing checker is no dependency");
        let table = b.file(r.inputs.iter().find(|i| i.role == "start_table").unwrap().file).path.clone();
        let gone = Fixed { checkers: vec!["s14.t3p"], missing: vec![table.clone()], mismatched: vec![] };
        let a = b.availability(r, &gone);
        assert_eq!(a.reasons(), [Reason::DependencyMissing]);
        assert!(
            matches!(&a.dependencies()[0], Dependency::Input { path, state: InputState::Missing, .. } if *path == table)
        );
        let other = Fixed { checkers: vec!["s14.t3p"], missing: vec![], mismatched: vec![table] };
        assert_eq!(b.availability(r, &other).reasons(), [Reason::DependencyMismatch]);
    }

    /// CT-LB5-06's preflight part: every one of the 510 groups answers `ready` or `unavailable`, and an `unavailable`
    /// answer's reasons are the matrix's for that group, less what the binding resolves (the typed binding; stage 18's
    /// rebuilt start tables) and less `no_independent_checker` where a registered checker checks the claim. With every
    /// checker registered and every input held, 286 groups are ready; with none registered, none is.
    #[test]
    fn ct_lb5_06_preflight_every_group_answers_with_the_matrixs_blockers() {
        let b = Bindings::edition_1();
        let mut ready = 0;
        let mut per_reason: BTreeMap<Reason, usize> = BTreeMap::new();
        for r in b.rows() {
            let a = b.availability(r, &Everything);
            let mut expected: Vec<Reason> =
                r.matrix_reasons.iter().filter(|x| !r.resolved.contains_key(x)).copied().collect();
            if !r.claim_checkers().is_empty() {
                expected.retain(|x| *x != Reason::NoIndependentChecker);
            }
            // The matrix lists its reasons alphabetically; an answer lists them in LB5.14's order.
            expected.sort();
            match &a {
                Availability::Ready { checkers, inputs } => {
                    ready += 1;
                    assert!(expected.is_empty(), "{}", r.group);
                    let ids: Vec<&str> = checkers.iter().map(|c| c.id.as_str()).collect();
                    assert_eq!(ids, r.claim_checkers(), "{}", r.group);
                    assert_eq!(inputs.len(), b.stored(r).len() + r.derived.len(), "{}", r.group);
                    assert_eq!(r.data_status == DataStatus::SelfContained, r.derived.is_empty(), "{}", r.group);
                }
                Availability::Unavailable { reasons, .. } => {
                    assert_eq!(reasons, &expected, "{}", r.group);
                    assert!(!reasons.is_empty());
                    for x in reasons {
                        *per_reason.entry(*x).or_default() += 1;
                    }
                }
            }
            assert!(a.reasons().iter().all(|x| r.matrix_reasons.contains(x)), "{}", r.group);
            assert!(!a.reasons().contains(&Reason::ClaimUnmapped), "{}: every group is bound", r.group);
            let none = b.availability(r, &host(vec![]));
            assert!(!none.is_ready(), "{}", r.group);
            if r.data_status == DataStatus::RecordOnly {
                assert_eq!(none.reasons(), [Reason::UnsupportedClaim], "{}", r.group);
            } else {
                assert!(none.reasons().contains(&Reason::NoIndependentChecker), "{}", r.group);
            }
        }
        assert_eq!(ready, 286);
        assert_eq!(
            per_reason,
            BTreeMap::from([
                (Reason::DependencyMissing, 210),
                (Reason::NoIndependentChecker, 216),
                (Reason::UnsupportedClaim, 6),
                (Reason::HistoricalCheckIncomplete, 2),
            ])
        );
    }

    /// CT-LB5-06's preflight part, LB5.17's example: q11 answers `unavailable([dependency_missing,
    /// no_independent_checker], ...)` with its source reference to the absent successor cache, whatever the host
    /// registers, and keeps its captured string word for word, as the record states it.
    #[test]
    fn ct_lb5_06_preflight_q11_keeps_its_missing_cache_and_captured_string() {
        let b = Bindings::edition_1();
        let r = b.find(Q11).expect("q11's row");
        let source = b.file(r.source).path.clone();
        let expected = Availability::Unavailable {
            reasons: vec![Reason::DependencyMissing, Reason::NoIndependentChecker],
            dependencies: vec![Dependency::MissingInput {
                source: source.clone(),
                name: "type-lex-successor-base-cache.npz and complete successor audit, or a new independently \
                       rebuilt holder/cut certificate"
                    .into(),
                cites: vec!["/inputs_sha256/type-lex-successor-base-cache.npz".into()],
            }],
        };
        let s19 = ["s19.pose-table", "s19.gamma-matching", "s19.gamma-witness", "s19.frontier", "s19.ranks"];
        assert_eq!(b.check_availability(Q11, &Everything), expected);
        assert_eq!(b.check_availability(Q11, &host(s19.to_vec())), expected);
        assert_eq!(b.check_availability(Q11, &host(vec![])), expected);
        let label = r.label.as_ref().expect("q11's captured string");
        assert_eq!(label.text, "PASS_SHORTENED_THREE_BLOCK_WITNESS_AND_Q11_DEPTH3");
        assert_eq!(b.statement(r).source_label.as_deref(), Some(label.text.as_str()));
        let record = std::fs::read(repository().join(&source)).expect("q11's record");
        assert_eq!(c600_digest::sha256_hex(&record), b.file(r.source).sha256);
        let doc = parse(&record).expect("strict JSON");
        assert_eq!(doc.get("status").and_then(Value::as_str), Some(label.text.as_str()), "the record's own words");
        assert_eq!(b.rows().iter().filter(|x| x.label.is_some()).count(), 1, "q11 is the one captured string");
    }

    /// CT-LB5-06's preflight part, stage 18 (LB5.12): T6's step 4 and the rule's three stages keep
    /// `historical_check_incomplete` for gate 3 under every host. T6's tail replay, whose steps 1 to 3 its checker
    /// replays, answers that reason alone; the rule's, which `s18.tail_replay` answers `not_applicable`, keeps the
    /// matrix's blockers whole. The composed strict′ groups are ready once their checker is registered, with their
    /// intermediate start tables named as derived, by the digest the record states. Every gate's group binds the stored
    /// stop table as its start.
    #[test]
    fn ct_lb5_06_preflight_stage_18_gate_3_stays_historically_incomplete() {
        let b = Bindings::edition_1();
        let chains = [
            ("d1-tail", vec!["/steps/3"], 3, vec![Reason::HistoricalCheckIncomplete]),
            (
                "rule-stop",
                vec!["/steps/0", "/steps/1", "/steps/2"],
                2,
                vec![Reason::DependencyMissing, Reason::NoIndependentChecker, Reason::HistoricalCheckIncomplete],
            ),
        ];
        for (chain, gates, rebuilt, reasons) in chains {
            let steps = format!("18-tail-completion/results/{chain}/steps.json");
            let replay = row(&b, "18/tail_replay", &steps);
            let source = b.file(replay.source).path.clone();
            for h in [&Everything as &dyn Host, &host(vec!["s18.tail_replay"])] {
                let a = b.availability(replay, h);
                assert_eq!(a.reasons(), reasons, "{chain}");
                let pointers: Vec<&str> = a
                    .dependencies()
                    .iter()
                    .filter_map(|d| match d {
                        Dependency::HistoricalGate { source: s, pointer } if *s == source => Some(pointer.as_str()),
                        Dependency::MissingInput { source: s, .. } if *s == source => None,
                        other => panic!("{chain}: {other:?}"),
                    })
                    .collect();
                assert_eq!(pointers, gates, "{chain}");
            }
            for family in ["18/expansion", "18/composed", "18/tail_replay"] {
                let r = row(&b, family, &steps);
                let start = r.inputs.iter().find(|i| i.role == "start_table").expect("the stored start");
                assert!(b.file(start.file).path.ends_with("d1-acc-s7101/out4/tables/00113.json"), "{family} {chain}");
            }
            let composed = row(&b, "18/composed", &steps);
            let basis = composed.resolved.get(&Reason::DependencyMissing).map(String::as_str);
            assert_eq!(basis, Some("rebuilt_start_tables"));
            let Availability::Ready { checkers, inputs } = b.availability(composed, &host(vec!["s18.composed"])) else {
                panic!("{chain}: the composed group is ready");
            };
            assert_eq!(checkers[0].id, "s18.composed");
            let derived = inputs.iter().filter(|i| matches!(i, ReadyInput::Derived { .. })).count();
            assert_eq!(derived, rebuilt, "{chain}");
            assert!(inputs.iter().any(|i| matches!(i, ReadyInput::Stored { role, .. } if role == "start_table")));
            assert_eq!(b.availability(composed, &host(vec![])).reasons(), [Reason::NoIndependentChecker]);
        }
    }

    /// CT-LB5-06's preflight part: the record-only anchors answer `unsupported_claim` as the matrix says and name no
    /// checker; a group whose sub-checker reads it only for its scope keeps `no_independent_checker` even when that
    /// sub-checker is registered.
    #[test]
    fn ct_lb5_06_preflight_record_only_and_scope_only_groups_answer_as_the_matrix_says() {
        let b = Bindings::edition_1();
        let record_only: Vec<&Binding> = b.rows().iter().filter(|r| r.data_status == DataStatus::RecordOnly).collect();
        assert_eq!(record_only.len(), 6);
        for r in record_only {
            assert!(r.checkers.is_empty() && r.statement.form.is_none(), "{}", r.group);
            assert_eq!(b.availability(r, &Everything).reasons(), [Reason::UnsupportedClaim], "{}", r.group);
        }
        let scoped: Vec<&Binding> = b.rows().iter().filter(|r| r.checked_as == Some(CheckedAs::ScopeOnly)).collect();
        assert_eq!(scoped.len(), 114);
        for r in scoped {
            assert!(!r.checkers.is_empty() && r.claim_checkers().is_empty(), "{}", r.group);
            assert!(r.derived.is_empty() && r.resolved.len() == 1, "{}: only the typed binding resolves", r.group);
            let a = b.availability(r, &Everything);
            let mut whole: Vec<Reason> =
                r.matrix_reasons.iter().filter(|x| **x != Reason::ClaimUnmapped).copied().collect();
            whole.sort();
            assert_eq!(a.reasons(), whole, "{}", r.group);
            assert!(a.reasons().contains(&Reason::NoIndependentChecker), "{}", r.group);
        }
    }

    /// CT-ERR-11's unavailable part: preflight answers are values under every host state (nothing registered, every
    /// input missing, every input with other bytes), never an error, a pass or a negative. An evidence graph holding
    /// every group's statement and key is the same after all of them: no answer adds a leaf or refutes a claim (EVD.46).
    #[test]
    fn ct_err_11_unavailable_is_a_value_never_a_pass_or_a_negative() {
        let b = Bindings::edition_1();
        let rows_before = b.rows().to_vec();
        let mut graph = c600_evidence::Graph::new();
        for r in b.rows() {
            graph.add_statement(b.statement(r)).expect("one statement per group");
            graph.root(&b.key(r));
        }
        let before = graph.clone();
        let every: Vec<String> = b.files.iter().map(|f| f.path.clone()).collect();
        let hosts = [
            Fixed { checkers: vec![], missing: vec![], mismatched: vec![] },
            Fixed { checkers: vec![], missing: every.clone(), mismatched: vec![] },
            Fixed { checkers: vec![], missing: vec![], mismatched: every },
        ];
        let mut unavailable = 0;
        for h in &hosts {
            for r in b.rows() {
                let a = b.check_availability(&r.group, h);
                assert_eq!(a, b.check_availability(&r.group, h), "deterministic");
                if let Availability::Unavailable { reasons, .. } = &a {
                    unavailable += 1;
                    assert!(!reasons.is_empty());
                }
                assert!(!a.is_ready(), "{}: nothing is ready with no checker registered", r.group);
            }
        }
        assert_eq!(unavailable, 3 * 510);
        assert_eq!(graph, before, "preflight adds no evidence leaf");
        assert!(graph.nodes().iter().all(|n| !matches!(n, c600_evidence::Node::Leaf(_))));
        assert_eq!(b.rows(), rows_before.as_slice());
    }

    #[test]
    fn edition_shape_departures_refuse() {
        let text = std::str::from_utf8(EDITION_1).unwrap();
        for (from, to) in [
            ("\"version\":1", "\"version\":2"),
            ("\"claim_unmapped\",\"dependency_missing\"", "\"dependency_missing\",\"claim_unmapped\""),
            ("\"covers\":\"one_instance\",\"form\":\"finite\"", "\"covers\":\"search_set\",\"form\":\"finite\""),
            ("\"resolved\":{\"claim_unmapped\":\"typed_binding\"}", "\"resolved\":{\"claim_unmapped\":\"guess\"}"),
            ("\"checked_as\":\"scope_only\"", "\"checked_as\":\"seen\""),
            ("\"checked_as\":\"scope_only\"", "\"checked_as\":null"),
        ] {
            assert!(text.contains(from), "{from}");
            let changed = text.replacen(from, to, 1);
            assert_eq!(Bindings::parse(changed.as_bytes()).unwrap_err().code, crate::FORMAT, "{to}");
        }
    }

    /// One row of the edition (each row is one line) changed by `from` -> `to`.
    fn in_row(text: &str, group: &str, from: &str, to: &str) -> String {
        let at = text.find(&format!("\"group\":\"{group}\"")).unwrap_or_else(|| panic!("{group}"));
        let start = text[..at].rfind('\n').unwrap() + 1;
        let end = at + text[at..].find('\n').unwrap();
        let line = &text[start..end];
        assert!(line.contains(from), "{group}: {from}");
        format!("{}{}{}", &text[..start], line.replacen(from, to, 1), &text[end..])
    }

    /// An edition that resolves what no edition may resolve, or whose statement ID is not its content's digest, is
    /// refused when it is read, not only when it is compared with the matrix.
    #[test]
    fn edition_resolutions_and_statement_ids_are_checked_when_read() {
        let b = Bindings::edition_1();
        let text = std::str::from_utf8(EDITION_1).unwrap();
        let t6_replay = &row(&b, "18/tail_replay", "d1-tail/steps.json").group;
        let rule_replay = &row(&b, "18/tail_replay", "rule-stop/steps.json").group;
        let first = &b.rows()[0].group;
        let typed = "\"resolved\":{\"claim_unmapped\":\"typed_binding\"";
        for (group, from, to) in [
            (t6_replay, typed, format!("{typed},\"historical_check_incomplete\":\"typed_binding\"")),
            (rule_replay, typed, format!("{typed},\"dependency_missing\":\"rebuilt_start_tables\"")),
            (first, typed, format!("{typed},\"dependency_missing\":\"rebuilt_start_tables\"")),
            (first, "\"pointers\":[\"\"]", "\"pointers\":[\"/x\"]".to_string()),
            (first, "\"form\":\"finite\"", "\"form\":null".to_string()),
            (first, "\"stage\":14", "\"stage\":15".to_string()),
        ] {
            let changed = in_row(text, group, from, &to);
            assert_eq!(Bindings::parse(changed.as_bytes()).unwrap_err().code, crate::FORMAT, "{group}: {to}");
        }
    }
}
