//! KN5.26's admission matrix through the command layer (LP3's C7; leads L1 and L2 of the KN5 boundary attack). A
//! draft that names a workspace record, `{"record": name}`, is admitted or refused at `operation.preview` by the host's
//! provenance adapter. Here the adapter is the runtime's own, `c600_run::boundary::admit_preview` over the blocks the
//! host keeps, under the fixture pins, installed as a host installs it. Every session the layer serves is a solving
//! session (BND.14; BND.15 reads the practice copy as one), so the adapter applies everywhere.
//!
//! A refusal is a refusal at admission: no job, no preview, the slot and the generation as they were, nothing written.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use c600_command::{
    code, draft_args, Caller, CommandLayer, Completion, CompletionKind, Error, HeadlessFrames, Invocation, Kind,
    MemoryStore, Overlay, PreviewState, Provenance, Receipt, Secret, Shell, Status, Table, RECORD_ROW,
};
use c600_digest::sha256;
use c600_kernel::json::{self, Value};
use c600_kernel::{codes, Kernel, Verdict};
use c600_run::boundary::{admit_preview, Pins, Store};
use c600_run::provenance::{self, Author, Json, Pin, Producer, Profile};

const SECRET: [u8; 32] = [7; 32];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    KERNEL
        .get_or_init(|| match Kernel::load(&repository().join("assets")) {
            Ok(kernel) => Arc::new(kernel),
            Err(error) => panic!("the model in assets/ does not load: {error}"),
        })
        .clone()
}

// ----- blocks -----

/// FP.18's fixture capability contract and a checker pin, both held by the policy in these tests.
fn pins() -> Pins {
    Pins { contracts: vec![contract()], checkers: vec![Pin::of("m1-draft", b"c600-check fixture\n")] }
}

fn contract() -> Pin {
    Pin::of("fixture-1", b"p3 fixture capability contract 1\n")
}

fn canonical(block: &Json) -> Vec<u8> {
    provenance::canonical(block)
}

fn with(block: Json, key: &str, value: Json) -> Json {
    let Json::Object(mut fields) = block else { panic!("a block is an object") };
    fields.insert(key.into(), value);
    Json::Object(fields)
}

fn parts(digests: &[[u8; 32]]) -> Json {
    Json::List(digests.iter().map(|d| provenance::digest_ref("content", d)).collect())
}

/// The block of a program's output that took in `taken` (BND.22, BND.24).
fn program(author: Author, profile: Profile, taken: &[[u8; 32]]) -> Vec<u8> {
    let producer = Producer {
        contract: contract(),
        checker: pins().checkers.first().cloned(),
        author,
        module: sha256(b"a tool's module"),
        entry: "save".into(),
        profile,
    };
    canonical(&provenance::program(&producer, false, taken))
}

/// A hand-entered word (BND.23) that lists `taken` in `parts`, as a draft insertion or a composition does (BND.24).
fn manual(taken: &[[u8; 32]]) -> Vec<u8> {
    canonical(&with(provenance::manual(&contract()), "parts", parts(taken)))
}

/// An imported 0.4 macro (BND.23, FP.18).
fn imported_macro() -> Vec<u8> {
    canonical(&provenance::import(&contract(), b"[{\"kind\":\"word\",\"moves\":[1,-1]}]"))
}

/// A built-in tool's output that records a pick whose `c600-pick` record does not resolve here (FP.38).
fn picked() -> Vec<u8> {
    let digest = provenance::digest_ref("content", &[7; 32]);
    let record =
        Json::object([("format", Json::text("c600-pick")), ("version", Json::Int(1)), ("content", digest.clone())]);
    let pick = Json::object([("record", record), ("chosen", digest)]);
    let producer = Producer {
        contract: contract(),
        checker: pins().checkers.first().cloned(),
        author: Author::BuiltIn,
        module: sha256(b"a built-in tool's module"),
        entry: "pick".into(),
        profile: Profile::Solving,
    };
    canonical(&with(provenance::program(&producer, false, &[]), "pick", pick))
}

/// A digest that names no block the host keeps.
const MISSING: [u8; 32] = [9; 32];

// ----- the host's adapter -----

/// The host's provenance adapter: a record's block must read as an FP.17 block (PS1.16), and a word's admission is
/// `admit_preview` over the blocks the host keeps (KN5.26, BND.84, BND.85, BND.61). Clones share the kept blocks.
#[derive(Clone)]
struct Adapter {
    pins: Pins,
    kept: Arc<Mutex<Store>>,
}

impl Adapter {
    fn new() -> Adapter {
        Adapter { pins: pins(), kept: Arc::default() }
    }

    /// Keeps `block` under its content digest and returns the digest.
    fn keep(&self, block: &[u8]) -> [u8; 32] {
        self.kept().keep(block)
    }

    fn kept(&self) -> MutexGuard<'_, Store> {
        self.kept.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Provenance for Adapter {
    fn check(&self, block: &[u8]) -> Result<(), Error> {
        provenance::read(block).map(|_| ()).map_err(|code| Error::new(code, "the block does not read (PS1.16)"))
    }

    fn admit(&self, block: &[u8]) -> Result<(), Error> {
        admit_preview(block, &self.kept(), &self.pins).map_err(|refusal| Error::new(refusal.code, refusal.detail))
    }
}

// ----- the session -----

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
    adapter: Adapter,
    sent: u64,
}

/// A session at the solved state that protects `protected`, on H-08's table with the draft rows, with the adapter.
fn fixture(protected: &[i64]) -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(protected).unwrap());
    let table = Table::h08().with_drafts();
    let mut layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    let adapter = Adapter::new();
    layer.set_provenance(Box::new(adapter.clone()));
    Fixture {
        layer,
        shell: Shell::new(Secret::fixed(SECRET), "shell"),
        frames: HeadlessFrames::new(),
        adapter,
        sent: 0,
    }
}

fn letters(moves: &[i64]) -> Value {
    Value::Array(moves.iter().map(|&letter| Value::Int(letter)).collect())
}

fn word(moves: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {moves:?}}}]"#).as_bytes()).unwrap()
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn block_text(block: &[u8]) -> Value {
    text(std::str::from_utf8(block).expect("a canonical block is UTF-8"))
}

fn record_draft(name: &str) -> Value {
    json::object([("draft", json::object([("record", text(name))]))])
}

fn preview_ready(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Completed(Completion { kind: CompletionKind::PreviewReady, .. }))
}

impl Fixture {
    /// `workspace.set` of `moves` under `name` with `block`, which must complete.
    fn write(&mut self, name: &str, moves: &[i64], block: &[u8]) {
        let args =
            json::object([("payload", letters(moves)), ("provenance", block_text(block)), ("record", text(name))]);
        let receipt = self.shell.invoke(&mut self.layer, RECORD_ROW, args).unwrap();
        let done = matches!(receipt.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. }));
        assert!(done, "the write of {name}: {:?}", receipt.status);
    }

    fn ask(&mut self, args: Value) -> Receipt {
        self.shell.invoke(&mut self.layer, "operation.preview", args).unwrap()
    }

    /// Previews the draft in `args` and runs the job, which must build a preview; returns its ID.
    fn preview(&mut self, args: Value) -> u64 {
        let receipt = self.ask(args);
        let Status::Running(job) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        let done = self.layer.outcome(&receipt.command, &receipt.request).unwrap();
        assert!(preview_ready(&done), "{:?}", done.status);
        job
    }

    /// What a refusal at admission may not change: the revision, the records, the durable log, the generation, the
    /// slot's preview and its state.
    fn fixed(&self) -> (String, String, usize, u64, Option<(u64, PreviewState)>) {
        let slot = self.layer.preview().map(|p| (p.id, p.state));
        let records = format!("{:?}", self.layer.records());
        (format!("{:?}", self.layer.revision()), records, self.layer.store().log.len(), self.layer.generation(), slot)
    }

    /// A run inside a command the solver invoked (finding B2 of the KN5 boundary attack, ONE.34): the shell invokes a
    /// tool's query row, and the host takes its job and runs it as the job's run (`ProgramJob::run`), which lasts until
    /// the host reports the job. Only such a run prepares a preview.
    fn solver_run(&mut self) -> String {
        let overlay = Overlay { module: "tools/full".into(), entry: "run".into() };
        self.layer.register_overlay("tool.full.run", Kind::Query, &[], overlay).unwrap();
        let receipt = self.shell.invoke(&mut self.layer, "tool.full.run", json::object([])).unwrap();
        assert!(matches!(receipt.status, Status::Running(_)), "the tool's row: {:?}", receipt.status);
        self.layer.take_program_job().expect("the tool's job").run()
    }

    /// `operation.preview` of record `name` by `caller`, which must be refused at admission with `expected`.
    fn refused_by(&mut self, caller: &Caller, args: Value, expected: &str, what: &str) {
        let before = self.fixed();
        let receipt = match caller {
            Caller::Shell => self.ask(args),
            Caller::Runtime { run } => {
                self.sent += 1;
                let epoch = self.layer.revision().epoch;
                let command = format!("{run}/{}", self.sent);
                let row = "operation.preview".to_owned();
                self.layer.invoke(caller, Invocation { command, row, args, token: None, epoch })
            }
        };
        match &receipt.status {
            Status::Refused(error) => assert_eq!(error.code, expected, "{what}: {}", error.detail),
            status => panic!("{what}: expected a refusal with {expected}, got {status:?}"),
        }
        assert_eq!(self.layer.run_jobs(), 0, "{what}: a refusal takes no job");
        assert_eq!(self.fixed(), before, "{what}");
    }

    fn refused(&mut self, name: &str, expected: &str, what: &str) {
        self.refused_by(&Caller::Shell, record_draft(name), expected, what);
    }
}

/// The refused words of L1 (a) to (d), each as a record's block with the parts the host keeps, with the code the
/// adapter answers. A missing part is never kept.
fn refused_words(adapter: &Adapter) -> Vec<(&'static str, Vec<u8>, &'static str)> {
    let own = program(Author::Own, Profile::Solving, &[]);
    let forked = program(Author::ForkedBuiltIn, Profile::Solving, &[]);
    let built_in = program(Author::BuiltIn, Profile::Solving, &[]);
    let research = program(Author::BuiltIn, Profile::Research, &[]);
    let (own_at_1, research_at_1, picked_at_1) = (adapter.keep(&own), adapter.keep(&research), adapter.keep(&picked()));
    let forked_at_1 = adapter.keep(&forked);
    let built_in_at_1 = adapter.keep(&built_in);
    let depth_2 = adapter.keep(&manual(&[own_at_1]));
    let depth_3 = manual(&[adapter.keep(&manual(&[depth_2]))]);
    let missing_at_2 = manual(&[adapter.keep(&manual(&[MISSING]))]);
    let (granted, incompatible) = (code::NOT_GRANTED, code::PROVENANCE_INCOMPATIBLE);
    vec![
        ("an own tool's word", own, granted),
        ("a forked built-in tool's word", forked, granted),
        ("an imported tool's word", program(Author::Imported, Profile::Solving, &[]), granted),
        ("a word whose producer is unknown", program(Author::Unknown, Profile::Solving, &[]), granted),
        ("an own tool's word as a part", manual(&[own_at_1]), granted),
        ("an own tool's word as a part at depth 3", depth_3, granted),
        ("a forked tool's word after a built-in part", manual(&[built_in_at_1, forked_at_1]), granted),
        ("a part with no stored content", manual(&[MISSING]), granted),
        ("a part with no stored content at depth 2", missing_at_2, granted),
        ("research output", research, granted),
        ("research output as a part", manual(&[research_at_1]), granted),
        ("a pick that does not resolve", picked(), incompatible),
        ("a pick that does not resolve, as a part", manual(&[picked_at_1]), incompatible),
    ]
}

/// The admitted words of L1 (f): a hand-entered word, an imported 0.4 macro, a built-in tool's output, and a manual
/// word that took in a built-in tool's output and an imported macro the host keeps.
fn admitted_words(adapter: &Adapter) -> Vec<(&'static str, Vec<u8>)> {
    let built_in = program(Author::BuiltIn, Profile::Solving, &[]);
    let taken = [adapter.keep(&built_in), adapter.keep(&imported_macro())];
    vec![
        ("a hand-entered word", manual(&[])),
        ("an imported 0.4 macro", imported_macro()),
        ("a built-in tool's word", built_in),
        ("a hand-entered word with built-in and imported parts", manual(&taken)),
    ]
}

/// CT-KN5-13 (part: through the layer with the runtime's adapter, the word of a record is refused at
/// `operation.preview` admission when its block or any part's block, recursively, is a solver-authored tool's output,
/// research output, a part with no stored content or a pick that does not resolve, and a hand-entered word, an
/// imported 0.4 macro and built-in output pass; not run: a pick from the built-in repair list, which resolves only
/// when the session store holds `c600-pick` records, and the routes that produce such records, draft insertion,
/// composition and file round trips), CT-BND-26 (part: a tool block for each of `own`, `forked_built_in`, `imported`
/// and `unknown`, directly and nested in `parts`; not run: the pick routes), CT-BND-28 (part: a missing part, also
/// nested, never reads as manual or built-in), CT-BND-29 (part: the three producers pass the producer check through
/// the layer; not run: the repair-list pick) and CT-BND-30 (part: research output directly and in recursive parts),
/// with KN5.11, KN5.26, BND.21, BND.61, BND.84 and BND.85. A pick that does not resolve answers
/// `PS1:capability.provenance_incompatible`, as `admit_preview` does (FP.38); the other refusals
/// `LG3:capability.not_granted`. A record write reads the block alone, so each word is stored, and each refusal leaves
/// the presented preview, the generation and the records as they were.
#[test]
fn a_records_word_is_admitted_or_refused_by_every_block_in_its_parts() {
    let mut f = fixture(&[]);
    let kept = f.preview(draft_args(&word(&[5])));
    f.frames.present(&mut f.shell, &mut f.layer, kept).unwrap();
    let refused = refused_words(&f.adapter);
    for (i, (what, block, _)) in refused.iter().enumerate() {
        f.write(&format!("r{i}"), &[1, 2], block);
        assert!(f.layer.record(&format!("r{i}")).is_some(), "{what}");
    }
    for (i, (what, _, expected)) in refused.iter().enumerate() {
        f.refused(&format!("r{i}"), expected, what);
    }
    assert_eq!(f.layer.preview_state(kept), Some(PreviewState::Presented));

    for (i, (what, block)) in admitted_words(&f.adapter).into_iter().enumerate() {
        f.write(&format!("a{i}"), &[1, 2], &block);
        let id = f.preview(record_draft(&format!("a{i}")));
        let preview = f.layer.preview().expect(what);
        assert_eq!(
            (preview.id, preview.resolved.as_ref().map(|r| r.letters.clone())),
            (id, Some(vec![1, 2])),
            "{what}"
        );
    }
}

/// CT-KN5-13 (part: restored workspace provenance, L1 (e)) and CT-BND-27 (part: a reopen restores each record's block,
/// and the restored word is refused at admission as before; not run: a part's block, which the journal does not keep
/// (the durable home for part blocks is LP2's next step), and a restore from a file), with KN5.26 and BND.85. A reopen
/// restores the records without an adapter, and every record draft is refused with `not_granted` until the host
/// installs one. With an adapter that keeps the same blocks, each refusal and each admission is as before the reopen.
/// With an adapter that keeps none, as a host has after a restart today, every refusal stands, a word with parts now
/// refused on its first part, and the word with built-in parts is refused too: a missing part reads `unknown`
/// (BND.21), which fails closed.
#[test]
fn a_reopen_keeps_every_refusal_and_fails_closed_on_parts_it_lost() {
    let mut f = fixture(&[]);
    let refused = refused_words(&f.adapter);
    for (i, (_, block, _)) in refused.iter().enumerate() {
        f.write(&format!("r{i}"), &[1, 2], block);
    }
    let admitted = admitted_words(&f.adapter);
    let with_parts = admitted.len() - 1;
    for (i, (_, block)) in admitted.iter().enumerate() {
        f.write(&format!("a{i}"), &[1, 2], block);
    }

    let reopen = |f: &Fixture| {
        let store = f.layer.store().clone();
        let layer = CommandLayer::open(kernel(), Table::h08().with_drafts(), Secret::fixed(SECRET), store).unwrap();
        Fixture {
            layer,
            shell: Shell::new(Secret::fixed(SECRET), "shell"),
            frames: HeadlessFrames::new(),
            adapter: Adapter::new(),
            sent: 0,
        }
    };

    let mut bare = reopen(&f);
    bare.refused("a0", code::NOT_GRANTED, "a hand-entered word with no adapter");

    let mut same = reopen(&f);
    let kept = f.adapter.kept().clone();
    *same.adapter.kept() = kept;
    same.layer.set_provenance(Box::new(same.adapter.clone()));
    for (i, (what, _, expected)) in refused.iter().enumerate() {
        same.refused(&format!("r{i}"), expected, what);
    }
    for (i, (what, _)) in admitted.iter().enumerate() {
        let id = same.preview(record_draft(&format!("a{i}")));
        assert_eq!(same.layer.preview().map(|p| p.id), Some(id), "{what}");
    }

    let mut lost = reopen(&f);
    lost.layer.set_provenance(Box::new(lost.adapter.clone()));
    for (i, (what, block, expected)) in refused.iter().enumerate() {
        // A word with parts now fails on its first part, which reads `unknown`.
        let has_parts = !provenance::field::parts(&provenance::read(block).unwrap()).is_empty();
        lost.refused(&format!("r{i}"), if has_parts { code::NOT_GRANTED } else { *expected }, what);
    }
    for (i, (what, _)) in admitted.iter().enumerate().take(with_parts) {
        lost.preview(record_draft(&format!("a{i}")));
        assert!(lost.layer.preview().is_some(), "{what}");
    }
    lost.refused(&format!("a{with_parts}"), code::NOT_GRANTED, "a word whose built-in parts the host lost");
}

/// CT-KN5-13 (part: an admissible permitted word with a protection conflict still gets KN5.9's preview, L1 (g)), with
/// KN5.9 and KN5.26: a hand-entered word in a record that moves a protected orbit is admitted, and its preview is
/// built with the conflict, which refuses the commit; an own tool's word with the same letters is refused at admission,
/// before any conflict is computed.
#[test]
fn a_permitted_word_that_moves_a_protected_orbit_gets_its_conflict_preview() {
    let mut f = fixture(&[1]);
    let moving = (1..=1200)
        .find(|&letter| {
            f.preview(draft_args(&word(&[letter])));
            f.layer.preview().unwrap().protection.verdict == Verdict::Moves
        })
        .expect("a letter that moves orbit 1");

    f.write("mine", &[moving], &manual(&[]));
    let id = f.preview(record_draft("mine"));
    let preview = f.layer.preview().unwrap();
    assert_eq!((preview.id, preview.protection.verdict), (id, Verdict::Moves));
    assert!(preview.end.is_none() && !preview.protection.conflicts.is_empty());
    f.frames.present(&mut f.shell, &mut f.layer, id).unwrap();
    assert_eq!(f.layer.availability("operation.execute").unwrap_err().code, code::ORBIT_WOULD_MOVE);

    f.write("tool", &[moving], &program(Author::Own, Profile::Solving, &[]));
    f.refused("tool", code::NOT_GRANTED, "an own tool's word that moves a protected orbit");
}

/// L2 (i), with KN5.11 and KN5.26: a solver tool's word whose letters the kernel also refuses is refused at admission
/// with the adapter's `not_granted`, never failed in a job: the adapter reads the block before the kernel reads the
/// letters. A hand-entered word with the same letters is refused at admission with the kernel's code. Neither takes a
/// job; the exclusive lane stays free for the next preview.
#[test]
fn a_tool_word_the_kernel_also_refuses_is_refused_at_admission_by_the_adapter() {
    let mut f = fixture(&[]);
    f.write("tool", &[1, 0], &program(Author::Own, Profile::Solving, &[]));
    f.write("mine", &[1, 0], &manual(&[]));
    f.refused("tool", code::NOT_GRANTED, "a tool word with letter 0");
    f.refused("mine", codes::RECIPE_SHAPE, "a hand-entered word with letter 0");
    f.preview(draft_args(&word(&[1])));
}

/// L2 (ii), with KN0.8, KN5.11, KN5.16 and KN5.26: the letters of a record draft are read and admitted when the
/// preview is admitted, and the job builds those letters. A write of the record between admission and the build, here
/// a solver tool's word, is never previewed in their place: the preview holds the admitted letters, the same word as
/// those letters inline, and is stale at once: once a frame presents it, the commit row is unavailable with
/// `revision_changed`. Without the write, the same preview is committable. A new draft of the record now reads the
/// tool's block and is refused.
#[test]
fn a_record_written_after_admission_never_replaces_the_admitted_letters() {
    let mut inline = fixture(&[]);
    inline.preview(draft_args(&word(&[1, 2])));
    let literal = inline.layer.preview().unwrap().literal.clone();
    for written in [false, true] {
        let mut f = fixture(&[]);
        f.write("w", &[1, 2], &manual(&[]));
        let receipt = f.ask(record_draft("w"));
        assert!(matches!(receipt.status, Status::Running(_)), "{:?}", receipt.status);
        if written {
            f.write("w", &[2, 1], &program(Author::Own, Profile::Solving, &[]));
        }
        assert_eq!(f.layer.run_jobs(), 1);
        assert!(preview_ready(&f.layer.outcome(&receipt.command, &receipt.request).unwrap()));

        let preview = f.layer.preview().expect("the admitted preview").clone();
        assert_eq!(preview.resolved.as_ref().map(|r| r.letters.clone()), Some(vec![1, 2]), "{written}");
        assert_eq!(preview.literal, literal, "{written}");
        f.frames.present(&mut f.shell, &mut f.layer, preview.id).unwrap();
        if written {
            assert_eq!(f.layer.availability("operation.execute").unwrap_err().code, code::REVISION_CHANGED);
            f.refused("w", code::NOT_GRANTED, "the record's new word");
        } else {
            assert_eq!(f.layer.availability("operation.execute"), Ok(()));
        }
    }
}

/// L2 (iii), with KN5.26, BND.21 and BND.23 (the frozen text does not say who stamps an inline word's block; this is
/// the reading of ruling 6: the host stamps it, never the caller). A draft is `{"recipe": [...]}` or `{"record":
/// name}`, so neither a run nor the shell can give a draft a block of its own: a draft with any other key is refused
/// with `KN5:input.argument_form`. A run's record draft reaches the same adapter as the shell's. A run's own word gets
/// its block from the runtime's boundary, which stamps the run's producer and admits it before the proposal leaves
/// the run (`c600-run`'s `tests/boundary.rs`). The run is inside a command the solver invoked (finding B2).
#[test]
fn a_draft_carries_no_block_of_the_callers() {
    let mut f = fixture(&[]);
    f.write("tool", &[1, 2], &program(Author::Own, Profile::Solving, &[]));
    f.write("mine", &[1, 2], &manual(&[]));
    let declared = block_text(&manual(&[]));
    let name = f.solver_run();
    let run = Caller::Runtime { run: name.clone() };
    for caller in [Caller::Shell, run.clone()] {
        for draft in [
            json::object([("provenance", declared.clone()), ("recipe", word(&[1, 2]))]),
            json::object([("provenance", declared.clone()), ("record", text("tool"))]),
        ] {
            let args = json::object([("draft", draft)]);
            f.refused_by(&caller, args, code::ARGUMENT_FORM, &format!("{caller:?}: a draft with its own block"));
        }
        f.refused_by(&caller, record_draft("tool"), code::NOT_GRANTED, &format!("{caller:?}: a tool's record"));
    }
    let epoch = f.layer.revision().epoch;
    let invocation = Invocation {
        command: format!("{name}/9"),
        row: "operation.preview".into(),
        args: record_draft("mine"),
        token: None,
        epoch,
    };
    let receipt = f.layer.invoke(&run, invocation);
    assert!(matches!(receipt.status, Status::Running(_)), "{:?}", receipt.status);
    assert_eq!(f.layer.run_jobs(), 1);
}
