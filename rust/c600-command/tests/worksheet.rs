//! LB7.2's work sheets against the command layer (draft; CT-LB2-03's work-sheet part): `worksheet.save` saves the
//! operation that a draft record holds as a named sheet, `worksheet.load` reuses a saved sheet, placing its piece roles
//! again at the head, and `worksheet.compare` compares a saved sheet with the current work and analyses it afresh. Each
//! test names the conformance rows it covers headless with PHS.62 markers. Where the frozen text is silent, the test
//! follows the draft rulings recorded in `src/lib.rs` and says so.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, digest, draft_args, Caller, CommandLayer, Completion, CompletionKind, Direction, Error, HeadlessFrames,
    Invocation, MacroVersion, MemoryStore, Occupancy, Placement, PreviewState, Provenance, Receipt, RecordValue,
    Resolution, Role, Secret, Shell, Status, Table, Worksheet, WorksheetRef, RECORD_ROW, WORKSHEET_COMPARE_ROW,
    WORKSHEET_LOAD_ROW, WORKSHEET_SAVE_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::{Kernel, PIECES};

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

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

fn letters(moves: &[i64]) -> Value {
    Value::Array(moves.iter().map(|&letter| Value::Int(letter)).collect())
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

/// A draft that names record `name` (a `ResolvedDraftRef`).
fn record(name: &str) -> Value {
    json::object([("record", text(name))])
}

/// `operation.preview`'s arguments for the draft that names record `name`.
fn record_draft(name: &str) -> Value {
    json::object([("draft", record(name))])
}

/// `worksheet.save`'s arguments.
fn save_args(draft: Value, name: Value) -> Value {
    json::object([("draft", draft), ("name", name)])
}

/// `worksheet.load`'s arguments: the sheet in record `name` at `version` (H-08's `WorksheetRevisionRef`, draft form).
fn load_args(name: &str, version: u64) -> Value {
    json::object([("worksheet", WorksheetRef { record: name.into(), version }.to_json())])
}

/// `worksheet.load`'s arguments with the reference's two fields as given.
fn named(record: Value, version: Value) -> Value {
    json::object([("worksheet", json::object([("record", record), ("version", version)]))])
}

fn piece(name: &str, piece: i64) -> Role {
    Role::Piece { name: name.into(), piece }
}

fn position(name: &str, position: i64) -> Role {
    Role::Position { name: name.into(), position }
}

/// A sheet that has not been saved, with the phases Prepare, Macro and Cleanup, `roles` and a reference.
fn sheet(phases: [&[i64]; 3], roles: Vec<Role>) -> Worksheet {
    let phases = phases.map(<[i64]>::to_vec);
    Worksheet { phases, roles, reference: text("reference 1"), resolution: None, macro_version: None }
}

/// `worksheet.compare`'s arguments: the sheet in record `name` at `version` and the draft record `draft`.
fn compare_args(name: &str, version: u64, draft: &str) -> Value {
    json::object([("worksheet", WorksheetRef { record: name.into(), version }.to_json()), ("draft", record(draft))])
}

/// The positions whose home piece is not solved at `f`'s head, in increasing order.
fn unsolved(f: &Fixture) -> Vec<i64> {
    let all: Vec<i64> = (0..PIECES as i64).collect();
    f.layer.positions(&all).unwrap().iter().filter(|p| !p.solved()).map(|p| p.piece as i64).collect()
}

/// `value`'s field `key`, which must be there.
fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).unwrap_or_else(|| panic!("no `{key}` in {value:?}"))
}

/// A list of positions as a comparison reports them.
fn ids(list: &[i64]) -> Value {
    Value::Array(list.iter().map(|&id| Value::Int(id)).collect())
}

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

fn completed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. }))
}

/// The host's provenance reader as these tests stand it in: at a write it refuses the block `refused`, and the block
/// `revoked` once the test revokes it, as a reader that no longer accepts a stored block would (PS1.16); it admits
/// every record's word except one whose block is `unadmitted` (KN5.26).
struct Reader {
    revoked: Arc<AtomicBool>,
}

impl Provenance for Reader {
    fn check(&self, block: &[u8]) -> Result<(), Error> {
        if block == b"refused" || (block == b"revoked" && self.revoked.load(Ordering::SeqCst)) {
            return Err(Error::new(code::PROVENANCE_INCOMPATIBLE, "the test reader refuses the block"));
        }
        Ok(())
    }

    fn admit(&self, block: &[u8]) -> Result<(), Error> {
        if block == b"unadmitted" {
            return Err(Error::new(code::NOT_GRANTED, "the test reader does not admit the block"));
        }
        Ok(())
    }
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
    revoked: Arc<AtomicBool>,
}

/// `layer` with a shell, the headless frames and, when `reader`, the test reader.
fn with_reader(mut layer: CommandLayer, reader: bool) -> Fixture {
    let revoked = Arc::new(AtomicBool::new(false));
    if reader {
        layer.set_provenance(Box::new(Reader { revoked: revoked.clone() }));
    }
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new(), revoked }
}

/// A session at the solved state whose table holds the draft rows, with the test reader when `reader`.
fn session(reader: bool) -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let table = Table::h08().with_drafts();
    let layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    with_reader(layer, reader)
}

fn fixture() -> Fixture {
    session(true)
}

/// A reopen of `layer`'s store, which replays its record writes without a provenance reader.
fn reopen(layer: &CommandLayer) -> Fixture {
    let store = layer.store().clone();
    let layer = CommandLayer::open(kernel(), Table::h08().with_drafts(), Secret::fixed(SECRET), store).unwrap();
    with_reader(layer, false)
}

/// A piece that `moves` moves from the solved state, one it leaves where it is, and the position the moved piece is in
/// after the commit, as KN2.4's `positions` row reads them.
fn pieces_moved_by(moves: &[i64]) -> (i64, i64, i64) {
    let mut scratch = fixture();
    assert!(committed(&scratch.commit(&word(moves))));
    let all: Vec<i64> = (0..PIECES as i64).collect();
    let found = scratch.layer.positions(&all).unwrap();
    let moved = found.iter().find(|p| p.position != Some(p.piece)).expect("a piece the word moves");
    let stayed = found.iter().find(|p| p.position == Some(p.piece)).expect("a piece the word leaves");
    let to = moved.position.expect("a twist keeps a piece's stickers together");
    (moved.piece as i64, stayed.piece as i64, to as i64)
}

/// The piece whose stickers fill `position` at the head, found by KN2.4's `positions` row over every piece.
fn occupant(f: &Fixture, position: i64) -> Option<i64> {
    let all: Vec<i64> = (0..PIECES as i64).collect();
    let found = f.layer.positions(&all).unwrap();
    found.iter().find(|p| p.position.map(|at| at as i64) == Some(position)).map(|p| p.piece as i64)
}

impl Fixture {
    /// `workspace.set` through the command route.
    fn write(&mut self, name: &str, payload: Value, provenance: &str) -> Receipt {
        let args = json::object([("payload", payload), ("provenance", text(provenance)), ("record", text(name))]);
        self.shell.invoke(&mut self.layer, RECORD_ROW, args).unwrap()
    }

    /// `worksheet.save` of the draft record `source` under `name`, through the command route.
    fn save(&mut self, source: &str, name: &str) -> Receipt {
        self.shell.invoke(&mut self.layer, WORKSHEET_SAVE_ROW, save_args(record(source), text(name))).unwrap()
    }

    /// `worksheet.load` of the sheet in record `name` at its current version, through the command route.
    fn load(&mut self, name: &str) -> Receipt {
        let version = self.layer.records()[name].version;
        self.shell.invoke(&mut self.layer, WORKSHEET_LOAD_ROW, load_args(name, version)).unwrap()
    }

    /// The work sheet that record `name` holds.
    fn sheet(&self, name: &str) -> Worksheet {
        Worksheet::from_payload(&self.layer.record(name).expect("a record").payload).expect("a work sheet")
    }

    /// Previews the draft in `args` and runs the job; returns the preview's ID.
    fn preview_with(&mut self, args: Value) -> u64 {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", args).unwrap();
        let Status::Running(job) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        job
    }

    /// The next headless frame, which shows the pending preview through the built-in chain, reported for `id`.
    fn present(&mut self, id: u64) {
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
    }

    /// The commit chord for preview `id`, prepared and not sent.
    fn press(&mut self, id: u64) -> Invocation {
        let args = json::object([("preview", Value::Int(id as i64))]);
        self.shell.prepare(&self.layer, "operation.execute", args).unwrap()
    }

    /// Previews the draft in `args`, presents it and sends its commit chord.
    fn commit_with(&mut self, args: Value) -> Receipt {
        let id = self.preview_with(args);
        self.present(id);
        let press = self.press(id);
        self.shell.send(&mut self.layer, press)
    }

    fn commit(&mut self, recipe: &Value) -> Receipt {
        self.commit_with(draft_args(recipe))
    }

    /// The undo or redo of the action on top of its stack; no action here releases an orbit, so none confirms.
    fn step(&mut self, direction: Direction) -> Receipt {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        self.shell.invoke(&mut self.layer, row, json::object([("hint", hint.to_json())])).unwrap()
    }

    /// What no refusal may change: the state digest, the revision, the records and the durable log.
    fn fixed(&self) -> (String, String, String, usize) {
        let r = self.layer.revision();
        (r.digest.clone(), format!("{r:?}"), format!("{:?}", self.layer.records()), self.layer.store().log.len())
    }

    /// Delivers `invocation` on `caller`'s channel, which must be refused with `expected` and change nothing.
    fn refused_on(&mut self, caller: &Caller, invocation: Invocation, expected: &str, what: &str) {
        let before = self.fixed();
        let receipt = self.layer.invoke(caller, invocation);
        assert_eq!(self.fixed(), before, "{what}");
        match receipt.status {
            Status::Refused(error) if error.code == expected => {}
            status => panic!("{what}: expected {expected}, got {status:?}"),
        }
    }

    /// `row` with `args` from the shell, which must be refused with `expected` and change nothing.
    fn refused(&mut self, row: &str, args: Value, expected: &str, what: &str) {
        let invocation = self.shell.prepare(&self.layer, row, args).unwrap();
        self.refused_on(&Caller::Shell, invocation, expected, what);
    }

    /// `worksheet.compare` of the sheet in record `name` at its current version with the draft record `draft`, through
    /// the command route: admitted as a job, which runs; returns the settled receipt and, when it completed, the
    /// comparison it kept.
    fn compare(&mut self, name: &str, draft: &str) -> (Receipt, Option<Value>) {
        let version = self.layer.records()[name].version;
        let receipt = self.shell.invoke(&mut self.layer, WORKSHEET_COMPARE_ROW, compare_args(name, version, draft));
        let receipt = receipt.unwrap();
        let Status::Running(job) = receipt.status else { panic!("comparison not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        let settled = self.layer.outcome(&receipt.command, &receipt.request).expect("a settled receipt");
        (settled, self.layer.comparison(job).cloned())
    }

    /// [`Fixture::compare`], which must complete as a query whose result reference digests the comparison.
    fn compared(&mut self, name: &str, draft: &str) -> Value {
        let (receipt, value) = self.compare(name, draft);
        let value = value.unwrap_or_else(|| panic!("no comparison: {:?}", receipt.status));
        match &receipt.status {
            Status::Completed(Completion { kind: CompletionKind::Query, result: Some(result), .. }) => {
                assert_eq!(result.digest, digest(&value));
            }
            status => panic!("expected completed(query), got {status:?}"),
        }
        value
    }
}

/// LB7.2 and the draft rulings in `src/lib.rs`: `worksheet.save` saves the operation that a draft record holds as a
/// work sheet under a name, in one record write that keeps the draft record's provenance block. A sheet keeps its
/// phases, roles, reference and macro version, and a record of letters becomes the Macro phase alone, with no role and
/// no reference, and names the record, at its version and content digest, as its macro version.
/// The piece roles are placed at the head, where KN2.4's `positions` row finds them, and position roles stay as given,
/// with the piece that fills each read there.
/// The state and the presented preview stay and the workspace revision advances. The save is undoable and redoable as a
/// workspace inverse (LB7.2; H-08's `undo: workspace`), a save under a name in use replaces that record's value, and
/// a saved sheet saves again as a draft, placed again.
#[test]
fn a_saved_work_sheet_keeps_its_operation_and_places_its_pieces() {
    let mut f = fixture();
    let p = f.preview_with(draft_args(&word(&[5])));
    f.present(p);
    let drafted = sheet([&[1], &[2, 3], &[4]], vec![piece("piece", 9), position("target", 7)]);
    assert!(completed(&f.write("draft", drafted.to_payload(), "block")));
    assert!(completed(&f.write("letters", letters(&[1, 2]), "other block")));
    let (solved, state, workspace) = {
        let r = f.layer.revision();
        (r.digest.clone(), r.state, r.workspace)
    };
    let placed = |pieces, positions| Some(Resolution { saved: solved.clone(), now: solved.clone(), pieces, positions });

    assert!(completed(&f.save("draft", "sheet")));
    let nine = Placement { name: "piece".into(), piece: 9, saved: Some(9), now: Some(9) };
    let seven = Occupancy { name: "target".into(), position: 7, saved: Some(7), now: Some(7) };
    let saved = Worksheet { resolution: placed(vec![nine], vec![seven]), ..drafted.clone() };
    assert_eq!(f.layer.record("sheet"), Some(&RecordValue { payload: saved.to_payload(), provenance: "block".into() }));
    assert_eq!(f.layer.record("draft").map(|value| &value.payload), Some(&drafted.to_payload()));

    assert!(completed(&f.save("letters", "macro")));
    let phases = [Vec::new(), vec![1, 2], Vec::new()];
    let source = f.layer.record("letters").unwrap().digest();
    let macro_version = Some(MacroVersion { record: "letters".into(), version: 1, digest: source });
    let (roles, resolution) = (Vec::new(), placed(Vec::new(), Vec::new()));
    let alone = Worksheet { phases, roles, reference: Value::Null, resolution, macro_version };
    assert_eq!(f.sheet("macro"), alone);
    assert_eq!(f.layer.record("macro").unwrap().provenance, "other block");
    let r = f.layer.revision();
    assert_eq!((r.digest.as_str(), r.state, r.workspace), (solved.as_str(), state, workspace + 2));
    assert_eq!(f.layer.preview_state(p), Some(PreviewState::Presented));

    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!((f.layer.record("macro"), f.layer.records()["macro"].version), (None, 2));
    assert!(committed(&f.step(Direction::Redo)));
    assert_eq!((f.sheet("macro"), f.layer.records()["macro"].version), (alone.clone(), 3));

    assert!(completed(&f.save("letters", "sheet")));
    assert_eq!((f.sheet("sheet"), f.layer.records()["sheet"].version), (alone.clone(), 2));
    assert!(completed(&f.save("macro", "copy")));
    assert_eq!(f.sheet("copy"), alone);
}

/// LB2 and the draft rulings in `src/lib.rs`: a draft `{"record": name}` of a record that holds a work sheet, saved or
/// not, previews the sheet's complete operation, Prepare, Macro and Cleanup in that order: the same word as its letters
/// inline, with the sheet's record in the preview's read set (KN0.8).
#[test]
fn a_work_sheets_record_previews_its_complete_operation() {
    let mut inline = fixture();
    inline.preview_with(draft_args(&word(&[1, 2, 3, 4])));
    let same = inline.layer.preview().expect("the inline preview");
    let expected = (&same.literal, same.end.as_ref().map(|end| end.digest()));

    let mut f = fixture();
    let drafted = sheet([&[1], &[2, 3], &[4]], vec![piece("piece", 9)]);
    assert!(completed(&f.write("draft", drafted.to_payload(), "block")));
    assert!(completed(&f.save("draft", "sheet")));
    for name in ["draft", "sheet"] {
        f.preview_with(record_draft(name));
        let preview = f.layer.preview().expect("the sheet's preview");
        let resolved = preview.resolved.as_ref().expect("a draft that names a record");
        assert_eq!(resolved.letters, [1, 2, 3, 4], "{name}");
        assert_eq!(resolved.reads.iter().map(|read| read.record.as_str()).collect::<Vec<_>>(), [name], "{name}");
        assert_eq!((&preview.literal, preview.end.as_ref().map(|end| end.digest())), expected, "{name}");
    }
}

/// CT-LB2-03 (part: named work-sheet reuse invalidates the old preview, with its piece roles placed again and a fresh
/// engine preview; not run here: M6's session store, which `c600-durable/tests/ct_lb2_03_work_sheet_reuse.rs` runs;
/// `worksheet.compare` runs in the tests below), LB7.2, KN0.8, solving workflow section 4.2 (draft). A reuse at the
/// same state writes the sheet again, unchanged, at its next version, so the preview that read the sheet stays
/// presented but is stale: the commit row is unavailable, the press prepared before the reuse is `revision_changed`,
/// and so is a reuse that names the sheet's earlier version. The sheet stores no preview or analysis that could stand
/// in for a fresh one: a fresh engine preview of the reused sheet commits its complete operation.
#[test]
fn reusing_a_named_work_sheet_invalidates_the_old_preview() {
    let mut f = fixture();
    assert!(completed(&f.write("draft", sheet([&[1], &[2], &[]], vec![piece("piece", 9)]).to_payload(), "block")));
    assert!(completed(&f.save("draft", "sheet")));
    let p = f.preview_with(record_draft("sheet"));
    f.present(p);
    let press = f.press(p);
    let (saved, state) = (f.layer.record("sheet").cloned(), f.layer.revision().state);
    assert_eq!(f.layer.availability("operation.execute"), Ok(()));

    assert!(completed(&f.load("sheet")));
    assert_eq!((f.layer.record("sheet").cloned(), f.layer.records()["sheet"].version), (saved, 2));
    assert_eq!((f.layer.revision().state, f.layer.preview_state(p)), (state, Some(PreviewState::Presented)));
    assert_eq!(f.layer.availability("operation.execute").unwrap_err().code, code::REVISION_CHANGED);
    f.refused_on(&Caller::Shell, press, code::REVISION_CHANGED, "the press prepared before the reuse");
    f.refused(WORKSHEET_LOAD_ROW, load_args("sheet", 1), code::REVISION_CHANGED, "the sheet's earlier version");

    let mut inline = fixture();
    assert!(committed(&inline.commit(&word(&[1, 2]))));
    assert!(committed(&f.commit_with(record_draft("sheet"))));
    assert_eq!(f.layer.revision().digest, inline.layer.revision().digest);
}

/// Solving workflow sections 4.1 and 4.2 (draft): after a commit moves a role's piece, a reuse places the piece again
/// and shows that it moved, with where it was saved and where it is now, and with the digests of the states of the save
/// and the reuse. The position role on the moved piece's home reads its occupant again: another piece fills it now,
/// shown beside what it held at the save (U1-057, U4-037). The phases, the reference and the position roles stay as
/// saved, so each mismatch is shown and the sheet is never retargeted. A later reuse keeps the save's placement and
/// occupant, and a reuse's undo restores the sheet as it was.
#[test]
fn a_reuse_shows_a_moved_piece_and_never_retargets_the_sheet() {
    let (moved, stayed, to) = pieces_moved_by(&[5]);
    let mut f = fixture();
    let roles = vec![piece("moved", moved), position("target", moved), piece("stayed", stayed)];
    let drafted = sheet([&[1], &[2, 3], &[4]], roles);
    assert!(completed(&f.write("draft", drafted.to_payload(), "block")));
    assert!(completed(&f.save("draft", "sheet")));
    let solved = f.layer.revision().digest.clone();
    assert!(committed(&f.commit(&word(&[5]))));
    assert!(completed(&f.load("sheet")));

    let reused = f.sheet("sheet");
    let kept = (&reused.phases, &reused.roles, &reused.reference);
    assert_eq!(kept, (&drafted.phases, &drafted.roles, &drafted.reference));
    let placements = vec![
        Placement { name: "moved".into(), piece: moved, saved: Some(moved), now: Some(to) },
        Placement { name: "stayed".into(), piece: stayed, saved: Some(stayed), now: Some(stayed) },
    ];
    let head = f.layer.revision().digest.clone();
    assert_ne!(head, solved);
    let held = occupant(&f, moved);
    assert!(held.is_some() && held != Some(moved), "another piece fills the moved piece's home: {held:?}");
    let target = Occupancy { name: "target".into(), position: moved, saved: Some(moved), now: held };
    let expected = Resolution { saved: solved.clone(), now: head, pieces: placements, positions: vec![target] };
    assert_eq!(reused.resolution.as_ref(), Some(&expected));
    assert_eq!(expected.pieces.iter().map(Placement::moved).collect::<Vec<_>>(), [true, false]);
    assert!(expected.positions[0].changed());

    let first = f.layer.record("sheet").cloned();
    assert!(committed(&f.commit(&word(&[5]))));
    let now = f.layer.positions(&[moved]).unwrap()[0].position.map(|position| position as i64);
    assert!(completed(&f.load("sheet")));
    let again = f.sheet("sheet").resolution.expect("a saved sheet");
    assert_eq!((again.saved.as_str(), again.now.as_str()), (solved.as_str(), f.layer.revision().digest.as_str()));
    assert_eq!((again.pieces[0].saved, again.pieces[0].now), (Some(moved), now));
    assert_eq!((again.positions[0].saved, again.positions[0].now), (Some(moved), occupant(&f, moved)));
    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!(f.layer.record("sheet").cloned(), first);
}

/// LB7.2 and the draft rulings in `src/lib.rs`, in the order the rows check them: without the host's provenance reader
/// both rows are not built (`row_not_built`, also in their availability); the runtime may invoke neither
/// (`not_granted`), since a work sheet changes only in a command the solver invokes; a bad argument is `argument_form`,
/// a recipe draft among them; a record that is missing or holds no value, or that holds no operation to save or no
/// saved sheet to reuse, is `named_object_invalid`; a sheet named at another version is `revision_changed`; and a block
/// the reader refuses now is refused with the reader's error. None of them writes anything.
#[test]
fn the_work_sheet_rows_refuse_and_change_nothing() {
    let mut bare = session(false);
    for row in [WORKSHEET_SAVE_ROW, WORKSHEET_LOAD_ROW] {
        assert_eq!(bare.layer.availability(row).unwrap_err().code, code::ROW_NOT_BUILT, "{row}");
    }
    bare.refused(WORKSHEET_SAVE_ROW, save_args(record("w"), text("s")), code::ROW_NOT_BUILT, "a save, no reader");
    bare.refused(WORKSHEET_LOAD_ROW, load_args("w", 1), code::ROW_NOT_BUILT, "a reuse, no reader");

    let mut f = fixture();
    assert!(completed(&f.write("gone", letters(&[1]), "block")));
    assert!(committed(&f.step(Direction::Undo)));
    assert!(completed(&f.write("w", letters(&[1, 2]), "block")));
    assert!(completed(&f.write("text", text("not letters"), "block")));
    assert!(completed(&f.write("draft", sheet([&[], &[1], &[]], Vec::new()).to_payload(), "block")));
    assert!(completed(&f.write("kept", letters(&[3]), "revoked")));
    assert!(completed(&f.save("kept", "sheet")));
    let runtime = Caller::Runtime { run: "run-1".into() };
    for (row, args) in
        [(WORKSHEET_SAVE_ROW, save_args(record("w"), text("s"))), (WORKSHEET_LOAD_ROW, load_args("sheet", 1))]
    {
        assert_eq!(f.layer.availability(row), Ok(()), "{row}");
        let epoch = f.layer.revision().epoch;
        let served = f.layer.table().row(row).is_some_and(|found| found.overlay.is_some());
        let theirs = Invocation {
            command: "run-1/1".into(),
            row: row.into(),
            args,
            token: None,
            epoch,
            budget: served.then_some(c600_command::Budget::DRAFT_CEILING),
        };
        f.refused_on(&runtime, theirs, code::NOT_GRANTED, row);
    }

    let two_keys = json::object([("record", text("w")), ("recipe", word(&[1]))]);
    let bad_saves = [
        ("a draft that is not an object", save_args(text("w"), text("s"))),
        ("a recipe draft", save_args(json::object([("recipe", word(&[1]))]), text("s"))),
        ("a draft with a second key", save_args(two_keys, text("s"))),
        ("a record name that is not text", save_args(json::object([("record", Value::Int(1))]), text("s"))),
        ("an empty name", save_args(record("w"), text(""))),
        ("a name that is not text", save_args(record("w"), Value::Int(1))),
        ("a missing name", json::object([("draft", record("w"))])),
        ("a third argument", json::object([("draft", record("w")), ("name", text("s")), ("x", Value::Null)])),
    ];
    for (what, args) in bad_saves {
        f.refused(WORKSHEET_SAVE_ROW, args, code::ARGUMENT_FORM, what);
    }
    for (what, source) in [("a missing record", "missing"), ("a removed record", "gone"), ("a record of text", "text")]
    {
        f.refused(WORKSHEET_SAVE_ROW, save_args(record(source), text("s")), code::NAMED_OBJECT_INVALID, what);
    }

    let third = json::object([("record", text("sheet")), ("version", Value::Int(1)), ("x", Value::Null)]);
    let bad_loads = [
        ("a reference that is not an object", json::object([("worksheet", text("sheet"))])),
        ("a missing version", json::object([("worksheet", record("sheet"))])),
        ("a negative version", named(text("sheet"), Value::Int(-1))),
        ("a version that is not a number", named(text("sheet"), text("1"))),
        ("a record name that is not text", named(Value::Int(1), Value::Int(1))),
        ("a reference with a third key", json::object([("worksheet", third)])),
        ("a missing reference", json::object([])),
    ];
    for (what, args) in bad_loads {
        f.refused(WORKSHEET_LOAD_ROW, args, code::ARGUMENT_FORM, what);
    }
    let version = |f: &Fixture, name: &str| f.layer.records()[name].version;
    let not_sheets = [
        ("a missing record", load_args("missing", 0)),
        ("a removed record", load_args("gone", version(&f, "gone"))),
        ("a record of letters", load_args("w", version(&f, "w"))),
        ("a sheet that was not saved", load_args("draft", version(&f, "draft"))),
    ];
    for (what, args) in not_sheets {
        f.refused(WORKSHEET_LOAD_ROW, args, code::NAMED_OBJECT_INVALID, what);
    }
    for stale in [0, 2] {
        f.refused(WORKSHEET_LOAD_ROW, load_args("sheet", stale), code::REVISION_CHANGED, "another version");
    }

    f.revoked.store(true, Ordering::SeqCst);
    let incompatible = code::PROVENANCE_INCOMPATIBLE;
    f.refused(WORKSHEET_SAVE_ROW, save_args(record("kept"), text("s")), incompatible, "a revoked block, saved");
    f.refused(WORKSHEET_LOAD_ROW, load_args("sheet", 1), incompatible, "a revoked block, reused");
    assert!(!f.layer.records().contains_key("s"));
}

/// CT-KN5-09's reopen for work sheets (draft): a reopen replays a save and a reuse from the journal as record writes,
/// without a provenance reader, so the reopened layer holds the same records and undoes and redoes them as the live
/// layer does; until the host gives it a reader again, it saves and reuses no sheet.
#[test]
fn a_reopened_layer_replays_work_sheet_saves_and_reuses() {
    let mut live = fixture();
    let drafted = sheet([&[1], &[2], &[3]], vec![piece("piece", 9)]);
    assert!(completed(&live.write("draft", drafted.to_payload(), "block")));
    assert!(completed(&live.save("draft", "sheet")));
    assert!(committed(&live.commit(&word(&[5]))));
    assert!(completed(&live.load("sheet")));
    let same = |a: &Fixture, b: &Fixture, what: &str| {
        let (x, y) = (a.layer.revision(), b.layer.revision());
        assert_eq!(a.layer.records(), b.layer.records(), "{what}");
        assert_eq!((&x.head, x.state, &x.digest, x.workspace), (&y.head, y.state, &y.digest, y.workspace), "{what}");
    };

    let mut reopened = reopen(&live.layer);
    same(&reopened, &live, "reopened");
    for row in [WORKSHEET_SAVE_ROW, WORKSHEET_LOAD_ROW] {
        assert_eq!(reopened.layer.availability(row).unwrap_err().code, code::ROW_NOT_BUILT, "{row}");
    }
    let (undo, redo) = (Direction::Undo, Direction::Redo);
    for direction in [undo, undo, undo, redo, redo, redo] {
        assert!(committed(&live.step(direction)), "live {direction:?}");
        assert!(committed(&reopened.step(direction)), "reopened {direction:?}");
        same(&reopened, &live, &format!("{direction:?}"));
    }
    reopened.layer.set_provenance(Box::new(Reader { revoked: reopened.revoked.clone() }));
    assert!(completed(&reopened.load("sheet")));
}

/// LB7.2, LB2 and U2-065, U3-102, U5-055, U6-040 (draft): `worksheet.compare` analyses the saved sheet's complete
/// operation afresh from the head, as one job on the exclusive lane whose completion is a query: the analysis reports
/// what a fresh preview of the same word reports, with LB2's exact gains and losses, the positions whose home piece the
/// operation finishes or unfinishes. It writes nothing and leaves the revision and the presented preview as they were.
/// A sheet saved from a record of letters names that record as its macro version, which the comparison reports as
/// current, as changed once the record is written again, and as missing when the record holds no value; the sheet's
/// own word is analysed, never the record's new one.
#[test]
fn a_comparison_analyses_the_sheet_afresh_and_changes_nothing() {
    let mut scratch = fixture();
    assert!(committed(&scratch.commit(&word(&[5]))));
    let five = unsolved(&scratch);
    assert!(!five.is_empty());
    let mut inline = fixture();
    inline.preview_with(draft_args(&word(&[5])));
    let summary = inline.layer.preview().expect("the inline preview").summary();

    let mut f = fixture();
    let p = f.preview_with(draft_args(&word(&[1])));
    f.present(p);
    assert!(completed(&f.write("letters", letters(&[5]), "block")));
    assert!(completed(&f.save("letters", "sheet")));
    let before = f.fixed();
    let value = f.compared("sheet", "letters");
    assert_eq!(f.fixed(), before);
    assert_eq!(f.layer.preview_state(p), Some(PreviewState::Presented));
    assert_eq!(f.layer.availability("operation.execute"), Ok(()));

    let analysis = field(&value, "analysis");
    let summarized = ["literal", "base", "end", "net_sha256", "net_moved_stickers", "net_moved_pieces"];
    for key in summarized.into_iter().chain(["primitive_count", "support", "conflicts", "verdict"]) {
        assert_eq!(field(analysis, key), field(&summary, key), "{key}");
    }
    assert_eq!((field(analysis, "gains"), field(analysis, "losses")), (&ids(&[]), &ids(&five)));
    let source = f.layer.record("letters").unwrap().digest();
    let saved = MacroVersion { record: "letters".into(), version: 1, digest: source.clone() };
    assert_eq!(f.sheet("sheet").macro_version.as_ref(), Some(&saved));
    let macro_version = |value: &Value, status: &str| {
        let expected = json::object([("saved", saved.to_json()), ("status", text(status))]);
        assert_eq!(field(value, "macro_version"), &expected);
    };
    macro_version(&value, "current");
    let same = |phase: bool| {
        let phases =
            [("cleanup", true), ("macro", phase), ("prepare", true)].map(|(name, same)| (name, Value::Bool(same)));
        json::object([
            ("macro_version", Value::Bool(phase)),
            ("phases", json::object(phases)),
            ("reference", Value::Bool(true)),
            ("roles", Value::Array(Vec::new())),
        ])
    };
    assert_eq!(field(&value, "inputs"), &same(true));
    let draft = json::object([("digest", text(&source)), ("record", text("letters")), ("version", Value::Int(1))]);
    assert_eq!(field(&value, "draft"), &draft);
    assert_eq!(field(&value, "worksheet"), &WorksheetRef { record: "sheet".into(), version: 1 }.to_json());
    assert_eq!(field(&value, "moved"), &Value::Array(Vec::new()));
    assert_eq!(field(&value, "occupant_changed"), &Value::Array(Vec::new()));

    assert!(committed(&f.commit(&word(&[5]))));
    assert!(completed(&f.write("inverse", letters(&[-5]), "block")));
    assert!(completed(&f.save("inverse", "back")));
    let back = f.compared("back", "inverse");
    let analysis = field(&back, "analysis");
    assert_eq!((field(analysis, "gains"), field(analysis, "losses")), (&ids(&five), &ids(&[])));

    assert!(completed(&f.write("letters", letters(&[6]), "block")));
    let changed = f.compared("sheet", "letters");
    macro_version(&changed, "changed");
    assert_eq!(field(&changed, "inputs"), &same(false));
    assert_eq!(field(field(&changed, "analysis"), "literal"), field(&summary, "literal"));

    let head = f.layer.revision().digest.clone();
    let resolution = Some(Resolution { saved: head.clone(), now: head, pieces: Vec::new(), positions: Vec::new() });
    let nowhere = MacroVersion { record: "nowhere".into(), ..saved.clone() };
    let forged = Worksheet { resolution, macro_version: Some(nowhere.clone()), ..sheet([&[], &[5], &[]], Vec::new()) };
    assert!(completed(&f.write("forged", forged.to_payload(), "block")));
    let missing = f.compared("forged", "letters");
    let expected = json::object([("saved", nowhere.to_json()), ("status", text("missing"))]);
    assert_eq!(field(&missing, "macro_version"), &expected);
}

/// Solving workflow sections 4.1 and 4.2 and U2-065 (draft): a comparison reports, role by role in name order, which
/// roles the current work keeps, changes, lacks or adds, whether each phase and the reference are the same, and where
/// the sheet's piece roles are now, placed as a reuse would place them, naming the ones that moved, and the position
/// roles whose occupant changed: shown, never retargeted. A sheet saved from a sheet draft without one names no macro
/// version. Its gains and losses are those of the sheet's complete operation from the head.
#[test]
fn a_comparison_reports_roles_placements_gains_and_losses() {
    let (moved, stayed, to) = pieces_moved_by(&[5]);
    let mut scratch = fixture();
    assert!(committed(&scratch.commit(&word(&[5]))));
    let before = unsolved(&scratch);
    assert!(committed(&scratch.commit(&word(&[1, 2, 3, 4]))));
    let after = unsolved(&scratch);
    let gains: Vec<i64> = before.iter().copied().filter(|p| !after.contains(p)).collect();
    let losses: Vec<i64> = after.iter().copied().filter(|p| !before.contains(p)).collect();
    assert!(!losses.is_empty());

    let mut f = fixture();
    let roles = vec![piece("moved", moved), position("target", moved), piece("stayed", stayed)];
    assert!(completed(&f.write("draft", sheet([&[1], &[2, 3], &[4]], roles).to_payload(), "block")));
    assert!(completed(&f.save("draft", "sheet")));
    assert!(committed(&f.commit(&word(&[5]))));
    let work = vec![piece("moved", moved), position("target", to), piece("extra", stayed)];
    let work = Worksheet { reference: text("reference 2"), ..sheet([&[1], &[3], &[4]], work) };
    assert!(completed(&f.write("work", work.to_payload(), "block")));
    let value = f.compared("sheet", "work");

    let row = |name: &str, saved: Option<Role>, draft: Option<Role>, status: &str| {
        let json = |role: Option<Role>| role.as_ref().map_or(Value::Null, Role::to_json);
        json::object([("draft", json(draft)), ("name", text(name)), ("saved", json(saved)), ("status", text(status))])
    };
    let roles = vec![
        row("extra", None, Some(piece("extra", stayed)), "added"),
        row("moved", Some(piece("moved", moved)), Some(piece("moved", moved)), "same"),
        row("stayed", Some(piece("stayed", stayed)), None, "missing"),
        row("target", Some(position("target", moved)), Some(position("target", to)), "changed"),
    ];
    let phases = [("cleanup", true), ("macro", false), ("prepare", true)].map(|(name, same)| (name, Value::Bool(same)));
    let inputs = json::object([
        ("macro_version", Value::Bool(true)),
        ("phases", json::object(phases)),
        ("reference", Value::Bool(false)),
        ("roles", Value::Array(roles)),
    ]);
    assert_eq!(field(&value, "inputs"), &inputs);
    assert_eq!(field(&value, "moved"), &Value::Array(vec![text("moved")]));
    assert_eq!(field(&value, "occupant_changed"), &Value::Array(vec![text("target")]));
    assert_eq!(field(&value, "macro_version"), &json::object([("saved", Value::Null), ("status", text("none"))]));
    let analysis = field(&value, "analysis");
    assert_eq!((field(analysis, "gains"), field(analysis, "losses")), (&ids(&gains), &ids(&losses)));

    assert!(completed(&f.load("sheet")));
    let placed = f.sheet("sheet").resolution.expect("a saved sheet");
    assert_eq!(field(&value, "resolution"), &placed.to_json());
}

/// LB7.2 and the draft rulings in `src/lib.rs`, in the order the row checks them: without the host's provenance reader
/// the row is not built (`row_not_built`, also in its availability); the runtime may not invoke it (`not_granted`); a
/// bad argument is `argument_form`, a recipe draft among them; a sheet that is missing, holds no value or holds no
/// saved sheet, and a draft record that is missing, holds no value or holds neither letters nor a sheet, are
/// `named_object_invalid`; a sheet named at another version is `revision_changed`; a sheet whose block the reader does
/// not admit is the reader's refusal; and a held exclusive lane is `lane_full`. None of them queues a job or writes.
#[test]
fn the_compare_row_refuses_and_changes_nothing() {
    let mut bare = session(false);
    assert_eq!(bare.layer.availability(WORKSHEET_COMPARE_ROW).unwrap_err().code, code::ROW_NOT_BUILT);
    bare.refused(WORKSHEET_COMPARE_ROW, compare_args("sheet", 1, "w"), code::ROW_NOT_BUILT, "no reader");

    let mut f = fixture();
    assert!(completed(&f.write("gone", letters(&[1]), "block")));
    assert!(committed(&f.step(Direction::Undo)));
    assert!(completed(&f.write("w", letters(&[1, 2]), "block")));
    assert!(completed(&f.write("text", text("not letters"), "block")));
    assert!(completed(&f.write("draft", sheet([&[], &[1], &[]], Vec::new()).to_payload(), "block")));
    assert!(completed(&f.save("w", "sheet")));
    assert!(completed(&f.write("hidden", letters(&[3]), "unadmitted")));
    assert!(completed(&f.save("hidden", "closed")));
    assert_eq!(f.layer.availability(WORKSHEET_COMPARE_ROW), Ok(()));
    let runtime = Caller::Runtime { run: "run-1".into() };
    let (epoch, args) = (f.layer.revision().epoch, compare_args("sheet", 1, "w"));
    let theirs = Invocation {
        command: "run-1/1".into(),
        row: WORKSHEET_COMPARE_ROW.into(),
        args,
        token: None,
        epoch,
        budget: None,
    };
    f.refused_on(&runtime, theirs, code::NOT_GRANTED, "the runtime");

    let at = || WorksheetRef { record: "sheet".into(), version: 1 }.to_json();
    let with = |draft: Value| json::object([("draft", draft), ("worksheet", at())]);
    let bad = [
        ("a draft that is not an object", with(text("w"))),
        ("a recipe draft", with(json::object([("recipe", word(&[1]))]))),
        ("a draft with a second key", with(json::object([("record", text("w")), ("recipe", word(&[1]))]))),
        ("a record name that is not text", with(json::object([("record", Value::Int(1))]))),
        ("a missing draft", json::object([("worksheet", at())])),
        ("a reference that is not an object", json::object([("draft", record("w")), ("worksheet", text("sheet"))])),
        ("a missing version", json::object([("draft", record("w")), ("worksheet", record("sheet"))])),
        ("a missing reference", json::object([("draft", record("w"))])),
        ("a third argument", json::object([("draft", record("w")), ("worksheet", at()), ("x", Value::Null)])),
    ];
    for (what, args) in bad {
        f.refused(WORKSHEET_COMPARE_ROW, args, code::ARGUMENT_FORM, what);
    }
    let version = |f: &Fixture, name: &str| f.layer.records()[name].version;
    let invalid = [
        ("a missing sheet", compare_args("missing", 0, "w")),
        ("a removed sheet", compare_args("gone", version(&f, "gone"), "w")),
        ("a record of letters", compare_args("w", version(&f, "w"), "w")),
        ("a sheet that was not saved", compare_args("draft", version(&f, "draft"), "w")),
        ("a missing draft record", compare_args("sheet", 1, "missing")),
        ("a removed draft record", compare_args("sheet", 1, "gone")),
        ("a draft record of text", compare_args("sheet", 1, "text")),
    ];
    for (what, args) in invalid {
        f.refused(WORKSHEET_COMPARE_ROW, args, code::NAMED_OBJECT_INVALID, what);
    }
    for stale in [0, 2] {
        f.refused(WORKSHEET_COMPARE_ROW, compare_args("sheet", stale, "w"), code::REVISION_CHANGED, "another version");
    }
    f.refused(WORKSHEET_COMPARE_ROW, compare_args("closed", 1, "w"), code::NOT_GRANTED, "a block not admitted");
    assert_eq!(f.layer.run_jobs(), 0);

    let queued = f.shell.invoke(&mut f.layer, "operation.preview", draft_args(&word(&[1]))).unwrap();
    assert!(matches!(queued.status, Status::Running(_)));
    f.refused(WORKSHEET_COMPARE_ROW, compare_args("sheet", 1, "w"), code::LANE_FULL, "a queued preview");
    assert_eq!(f.layer.run_jobs(), 1);
    assert_eq!(f.layer.run_jobs(), 0);
}

/// L16 (a) of the KN5 attack leads (KN5.9, KN7.1, KN7.2; CT-KN7-05's other order, after
/// `the_compare_row_refuses_and_changes_nothing`'s preview first): a comparison waiting on the exclusive lane refuses a
/// preview with `lane_full`, and the presented preview stays pending and presented. L13 (b) (KN5.6, CT-KN5-12): the
/// comparison's terminal has advanced the generation by the time its receipt is read, and a preview is admitted again.
#[test]
fn a_waiting_comparison_holds_the_exclusive_lane_against_a_preview() {
    let mut f = fixture();
    assert!(completed(&f.write("letters", letters(&[5]), "block")));
    assert!(completed(&f.save("letters", "sheet")));
    let p = f.preview_with(draft_args(&word(&[1])));
    f.present(p);
    let receipt = f.shell.invoke(&mut f.layer, WORKSHEET_COMPARE_ROW, compare_args("sheet", 1, "letters")).unwrap();
    let Status::Running(job) = receipt.status else { panic!("comparison not admitted: {:?}", receipt.status) };
    assert_eq!(f.layer.availability("operation.preview").unwrap_err().code, code::LANE_FULL);
    f.refused("operation.preview", draft_args(&word(&[2])), code::LANE_FULL, "a waiting comparison");
    assert_eq!(f.layer.preview().map(|pending| pending.id), Some(p));
    assert_eq!(f.layer.preview_state(p), Some(PreviewState::Presented));
    assert!(f.layer.availability("operation.execute").is_ok());

    let before = f.layer.generation();
    assert_eq!(f.layer.run_jobs(), 1);
    let settled = f.layer.outcome(&receipt.command, &receipt.request).expect("a settled receipt");
    assert!(matches!(settled.status, Status::Completed(Completion { kind: CompletionKind::Query, .. })));
    assert!(f.layer.generation() > before, "the comparison's terminal advanced the generation");
    assert!(f.layer.comparison(job).is_some());
    assert!(f.layer.availability("operation.preview").is_ok());
    let q = f.preview_with(draft_args(&word(&[2])));
    assert_eq!(f.layer.preview_state(p), Some(PreviewState::Replaced));
    assert_eq!(f.layer.preview_state(q), Some(PreviewState::Ready));
}

/// The draft rulings in `src/lib.rs`: the job reads the sheet and the draft record again when it runs, so a sheet
/// written between the admission and the run fails the job with `revision_changed`, and no comparison is kept. A queued
/// comparison holds the exclusive lane but is no preview: it is not building, and `operation.preview.cancel` does not
/// take it. An action that moves the state ends a kept comparison (LB7.2).
#[test]
fn a_comparison_of_a_sheet_written_since_admission_fails() {
    let mut f = fixture();
    assert!(completed(&f.write("letters", letters(&[5]), "block")));
    assert!(completed(&f.save("letters", "sheet")));
    let receipt = f.shell.invoke(&mut f.layer, WORKSHEET_COMPARE_ROW, compare_args("sheet", 1, "letters")).unwrap();
    let Status::Running(job) = receipt.status else { panic!("comparison not admitted: {:?}", receipt.status) };
    assert!(completed(&f.save("letters", "sheet")));
    assert_eq!(f.layer.run_jobs(), 1);
    match f.layer.outcome(&receipt.command, &receipt.request).expect("a settled receipt").status {
        Status::Failed(error) if error.code == code::REVISION_CHANGED => {}
        status => panic!("expected failed(revision_changed), got {status:?}"),
    }
    assert_eq!(f.layer.comparison(job), None);

    let receipt = f.shell.invoke(&mut f.layer, WORKSHEET_COMPARE_ROW, compare_args("sheet", 2, "letters")).unwrap();
    let Status::Running(job) = receipt.status else { panic!("comparison not admitted: {:?}", receipt.status) };
    assert_eq!(f.layer.preview_state(job), None);
    assert_eq!(f.layer.availability("operation.preview.cancel").unwrap_err().code, code::NO_FRESH_PREVIEW);
    assert_eq!(f.layer.run_jobs(), 1);
    assert!(f.layer.comparison(job).is_some());
    assert!(committed(&f.commit(&word(&[5]))));
    assert_eq!(f.layer.comparison(job), None);
}
