//! The run-time boundary under capability separation (LP2, M3; chapter 07, LQ21).
//!
//! [`Boundary`] is the runtime's adapter between a running program and the engine side (a [`Host`]). For one run it
//!
//! - builds the provenance block of everything the run stores or proposes from the run's [`Context`] (BND.22): the
//!   author class, the producing module and entry, the profile, whether the run read the state, and, as `parts`, the
//!   blocks of the arguments and stored values the run took in, with their join (BND.24). The IR has no operation that
//!   sets a field;
//! - stores workspace records and granted files with their blocks in a [`Store`], so a word carried through either
//!   keeps its block (BND.32; BND.47's storage row);
//! - in the solving profile, admits each proposal as the command layer's provenance adapter does at preview admission
//!   ([`admit_preview`]: BND.61, BND.84, BND.85), gives the program one fixed acknowledgement whatever the engine
//!   answered (BND.42, BND.43), and keeps the full answer for the shell (charter E6, ERR.13);
//! - in tool authoring and research, records each proposal as a capture and asks the engine for nothing, so no preview
//!   and no token exist (BND.60).
//!
//! What a run's capability set allows is the checker's (`c600-check::admit`); the adapter does not check grants again.
//! The flow rule stays off (BND.41): `read_state` feeds records only, and `LG3:capability.flow_refused` is never raised
//! (CT-BND-24).
//!
//! M3 drafts, which freeze with the IR and the command table:
//!
//! - The acknowledgement is the receipt `{base, id}`. `id` is the run's proposal number, `<run>/<n>`, assigned before
//!   the engine answers, so it is the same for an accepted and a refused proposal; the shell maps it to the preview. A
//!   capture gets the same receipt: no preview exists, so there is no effect, protection, end state or N to give.
//! - Records are per run, as capability separation records them: everything a run stores or proposes cites every
//!   block the run has taken in so far, in first-read order. `read_state` is `yes` when the admitted entry holds a
//!   state read, when the run read a snapshot, or when a part does not record `read_state: no`. A part that records
//!   `not_recorded` or does not read therefore joins as `yes`: FP.17 never infers `no`.
//! - The IR draft has one construct for snapshot and workspace reads. The adapter serves a name it stores as a
//!   workspace record and passes any other read to the engine as a snapshot read.
//! - Admission checks each block's format first and refuses with `PS1:capability.provenance_incompatible` a block
//!   that does not read, is in another form, names a pin the policy in force does not hold, or records a pick: the
//!   store holds no `c600-pick` record yet, so a pick does not resolve (FP.38). A built-in repair-list pick passes
//!   BND.84 (CT-BND-29) once the session store resolves pick records (M6). Then it refuses with
//!   `LG3:capability.not_granted` author `unknown`, research output and a solver-authored program's output.
//! - The engine's preview call does not take the block yet (KN5.9); the shell record keeps it beside the answer.
//! - A menu-word proposal ([`Request::ProposeMenu`], LB1.8) is admitted, answered and captured as a literal-word one;
//!   its shell record and capture hold the menu word in `menu` and leave `word` empty.
//! - The store is in memory and returns the stored value as written; M6's session store persists records and decodes
//!   them by the reader's type.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use c600_digest::sha256;

use crate::interp::{Host, Refusal, Request, RowTariff, Sink};
use crate::menu::{MenuIdentity, INCOMPATIBLE_ALPHABET};
use crate::meter::code::NOT_GRANTED;
use crate::provenance::{self, field, Pin, Producer, Profile, INCOMPATIBLE};
use crate::value::{MenuWord, Value};

/// Where a stored value lives.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Place {
    Workspace(String),
    File(String),
}

/// A stored value, its canonical bytes and the canonical bytes of its block (FP.11's `provenance` field).
#[derive(Clone, Debug)]
pub struct Stored {
    pub value: Value,
    pub bytes: Vec<u8>,
    pub block: Vec<u8>,
}

/// Workspace records, granted files and every block the runtime has kept, by content digest.
#[derive(Clone, Debug, Default)]
pub struct Store {
    pub records: BTreeMap<Place, Stored>,
    pub blocks: BTreeMap<[u8; 32], Vec<u8>>,
}

impl Store {
    /// Keeps block bytes, readable or not, under their content digest (PS1.33) and returns the digest.
    pub fn keep(&mut self, block: &[u8]) -> [u8; 32] {
        let digest = sha256(block);
        self.blocks.entry(digest).or_insert_with(|| block.to_vec());
        digest
    }
}

/// The contract and checker pins the policy in force can read (FP.17).
#[derive(Clone, Debug, Default)]
pub struct Pins {
    pub contracts: Vec<Pin>,
    pub checkers: Vec<Pin>,
}

/// Preview admission in a solving session (BND.61, BND.84, BND.85). It reads the stored blocks of the word and of
/// every part, recursively and in listed order, and performs no flow analysis. A refusal names the first block that
/// fails; the command layer gives the same answer at `operation.preview`.
pub fn admit_preview(block: &[u8], store: &Store, pins: &Pins) -> Result<(), Refusal> {
    let mut pending = admit_one(block, pins, "the word's block")?;
    pending.reverse();
    let mut seen = BTreeSet::new();
    while let Some(digest) = pending.pop() {
        if !seen.insert(digest) {
            continue;
        }
        let what = format!("part {}", provenance::hex(&digest));
        let Some(bytes) = store.blocks.get(&digest) else {
            let detail = format!("{what} has no stored content and reads unknown (BND.21, BND.85)");
            return Err(Refusal { code: NOT_GRANTED, detail });
        };
        let mut parts = admit_one(bytes, pins, &what)?;
        parts.reverse();
        pending.extend(parts);
    }
    Ok(())
}

fn admit_one(bytes: &[u8], pins: &Pins, what: &str) -> Result<Vec<[u8; 32]>, Refusal> {
    let refuse = |code: &'static str, why: &str| Err(Refusal { code, detail: format!("{what} {why}") });
    let Ok(b) = provenance::read(bytes) else {
        return refuse(INCOMPATIBLE, "does not read as an FP.17 block and reads unknown (BND.21)");
    };
    if field::form(&b) != "capability_separation" {
        return refuse(INCOMPATIBLE, "is in another form; a switch of forms is a recorded migration (CT-BND-22)");
    }
    if !field::contract(&b).is_some_and(|p| pins.contracts.contains(&p)) {
        return refuse(INCOMPATIBLE, "names a contract pin the policy in force does not hold (FP.17)");
    }
    if field::checker(&b).is_some_and(|p| !pins.checkers.contains(&p)) {
        return refuse(INCOMPATIBLE, "names a checker pin the policy in force does not hold (FP.17)");
    }
    if field::has_pick(&b) {
        return refuse(INCOMPATIBLE, "records a pick whose `c600-pick` record does not resolve here (FP.38)");
    }
    let author = field::author(&b);
    if author == "unknown" {
        return refuse(NOT_GRANTED, "has author `unknown`, which no row admits (BND.21)");
    }
    if field::profile(&b) == "research" {
        return refuse(NOT_GRANTED, "is research output (BND.61)");
    }
    if field::producer(&b) == "program" && author != "built_in" {
        return refuse(NOT_GRANTED, &format!("is the output of a solver-authored module, author `{author}` (BND.84)"));
    }
    Ok(field::parts(&b))
}

/// What the host knows about one run (BND.17, BND.22).
#[derive(Clone, Debug)]
pub struct Context {
    /// Names the run in proposal IDs.
    pub run: String,
    /// The revision the run reads: the acknowledgement's `base`.
    pub base: String,
    pub producer: Producer,
    /// Whether the admitted entry holds a state read: `c600-check`'s `Separation::reads_state`.
    pub reads_state: bool,
    /// The stored blocks of the entry's arguments, as the shell holds them.
    pub inputs: Vec<Vec<u8>>,
    pub pins: Pins,
}

/// What the shell shows for a proposal in a solving session: the engine's full answer (charter E6, ERR.13).
#[derive(Clone, Debug)]
pub struct Shown {
    pub id: String,
    pub word: Vec<i32>,
    /// A menu-word proposal's word (LB1.8; LP2 draft): its letters and the identity block of its alphabet. `word` is
    /// then empty.
    pub menu: Option<MenuWord>,
    pub target: String,
    pub block: [u8; 32],
    pub answer: Answer,
}

#[derive(Clone, Debug)]
pub enum Answer {
    /// The engine's receipt, every field included.
    Previewed(Value),
    /// Refused at admission, before the engine saw the proposal, or by the engine.
    Refused(Refusal),
}

/// A proposal in tool authoring or research, recorded for a test to read (BND.60).
#[derive(Clone, Debug)]
pub struct Capture {
    pub id: String,
    pub word: Vec<i32>,
    /// A menu-word proposal's word (LB1.8; LP2 draft); `word` is then empty.
    pub menu: Option<MenuWord>,
    pub target: String,
    pub block: [u8; 32],
}

/// The runtime's adapter for one run.
pub struct Boundary<'a> {
    engine: &'a mut dyn Host,
    store: &'a mut Store,
    context: Context,
    parts: Vec<[u8; 32]>,
    reads_state: bool,
    proposals: u64,
    pub shell: Vec<Shown>,
    pub captures: Vec<Capture>,
}

impl<'a> Boundary<'a> {
    pub fn new(engine: &'a mut dyn Host, store: &'a mut Store, context: Context) -> Self {
        let inputs = context.inputs.clone();
        let mut boundary = Boundary {
            engine,
            store,
            reads_state: context.reads_state,
            context,
            parts: Vec::new(),
            proposals: 0,
            shell: Vec::new(),
            captures: Vec::new(),
        };
        for input in &inputs {
            boundary.join(input);
        }
        boundary
    }

    /// The canonical block of what the run stores or proposes now (BND.22, BND.24).
    pub fn block(&self) -> Vec<u8> {
        provenance::canonical(&provenance::program(&self.context.producer, self.reads_state, &self.parts))
    }

    fn join(&mut self, block: &[u8]) {
        let digest = self.store.keep(block);
        if !self.parts.contains(&digest) {
            self.parts.push(digest);
        }
        if provenance::read(block).map_or(true, |b| field::may_read_state(&b)) {
            self.reads_state = true;
        }
    }

    fn propose(&mut self, request: &Request, word: &[i32], menu: Option<MenuWord>, target: &str) -> Value {
        self.proposals += 1;
        let id = format!("{}/{}", self.context.run, self.proposals);
        let block = self.block();
        let digest = self.store.keep(&block);
        let (word, target) = (word.to_vec(), target.to_string());
        if self.context.producer.profile == Profile::Solving {
            let answer = match admit_preview(&block, self.store, &self.context.pins) {
                Err(refusal) => Answer::Refused(refusal),
                Ok(()) => match self.engine.perform(request) {
                    Ok(receipt) => Answer::Previewed(receipt),
                    Err(code) => Answer::Refused(Refusal { code, detail: "the engine refused the preview".into() }),
                },
            };
            self.shell.push(Shown { id: id.clone(), word, menu, target, block: digest, answer });
        } else {
            self.captures.push(Capture { id: id.clone(), word, menu, target, block: digest });
        }
        Value::receipt(vec![("base", Value::text(&self.context.base)), ("id", Value::text(&id))])
    }
}

impl Host for Boundary<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.engine.tariff(request)
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        let place = match request {
            Request::Snapshot { query, .. } => Place::Workspace(query.to_string()),
            Request::FileRead { path, .. } => Place::File(path.to_string()),
            Request::Propose { word, target, .. } => return Ok(self.propose(request, word, None, target)),
            Request::ProposeMenu { letters, menu, target, .. } => {
                // The interpreter builds the request from a word of the pinned alphabet; a request built otherwise
                // with an index of 600 or more is refused before it is shown or captured.
                let Some(word) = MenuWord::new(Arc::new(MenuIdentity::clone(menu)), letters) else {
                    return Err(INCOMPATIBLE_ALPHABET);
                };
                return Ok(self.propose(request, &[], Some(word), target));
            }
            // An engine row for an argument is state-derived data whatever the program stores (LG3.13).
            Request::SnapshotWith { .. } => {
                self.reads_state = true;
                return self.engine.perform(request);
            }
            _ => return self.engine.perform(request),
        };
        if let Some(stored) = self.store.records.get(&place) {
            let (value, block) = (stored.value.clone(), stored.block.clone());
            self.join(&block);
            return Ok(value);
        }
        if matches!(request, Request::Snapshot { .. }) {
            self.reads_state = true;
        }
        self.engine.perform(request)
    }

    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        let place = match sink {
            Sink::Workspace { record, .. } => Place::Workspace(record.to_string()),
            Sink::File { path } => Place::File(path.to_string()),
            _ => return self.engine.sink(sink, value, bytes),
        };
        let block = self.block();
        self.store.keep(&block);
        self.store.records.insert(place, Stored { value: value.clone(), bytes: bytes.to_vec(), block });
        Ok(())
    }
}
