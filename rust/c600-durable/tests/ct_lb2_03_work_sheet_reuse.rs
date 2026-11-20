//! CT-LB2-03's work-sheet sentence, named work-sheet reuse invalidates the old preview, through KN5's command layer
//! over M6's journal (`JournalStore`) with C5's provenance reader installed. The solver saves a work sheet with the
//! layer's `worksheet.save` row, reuses it with `worksheet.load` and compares it with the current work with
//! `worksheet.compare`, LB7.2's fresh exact analysis of the sheet with LB2's gains and losses (LB7.2 and LB2's work
//! sheets; LP2's draft rows in `c600-command`'s `src/worksheet.rs` and `src/layer.rs`). `c600-command`'s
//! `tests/worksheet.rs` runs the rows on the layer's memory store; here they run on the journal, across power losses
//! on a memory disk and in new sessions on a file-system disk, and each refusal of the three rows comes back with the
//! layer's code and changes nothing on the journal. Each test names the conformance part it covers with a PHS.62
//! marker.
//!
//! CT-LB2-03 runs whole across crates (crate documentation): its first sentence, recipe chunking and the refusal of an
//! oversized recipe, runs in `c600-kernel` and `c600-tools`, and its second sentence runs here and on the layer's
//! memory store.

mod common;

use c600_command::{
    code, digest, CommandLayer, Completion, CompletionKind, Direction, Error, MacroVersion, Occupancy, Placement,
    PreviewState, Provenance, Receipt, RecordValue, Resolution, Role, Secret, Shell, Status, Worksheet, WorksheetRef,
    RECORD_ROW, WORKSHEET_COMPARE_ROW, WORKSHEET_LOAD_ROW, WORKSHEET_SAVE_ROW,
};
use c600_digest::sha256_hex;
use c600_kernel::json::{self, Value};
use c600_kernel::PIECES;
use c600_session::disk::{Disk, FsDisk, MemDisk};
use c600_session::JournalStore;
use c600_tools::bridge::Bridge;
use c600_tools::workspace;
use common::layer::{completed, record_draft, refused, text, Fixture, SECRET};
use common::{kernel, manual, TempDir, JOURNAL};

/// The complete operation of the sheet the legs save: Prepare, Macro and Cleanup in that order.
const OPERATION: [i32; 2] = [1, 2];

/// A provenance block that C5's reader does not read: it is no canonical FP.17 block.
const FOREIGN: &[u8] = b"a block of another reader";

fn string(text: &str) -> Value {
    Value::String(text.into())
}

/// A draft that names record `name` (a `ResolvedDraftRef`).
fn record(name: &str) -> Value {
    json::object([("record", string(name))])
}

/// `worksheet.save`'s arguments with the draft and the name as given.
fn save_with(draft: Value, name: Value) -> Value {
    json::object([("draft", draft), ("name", name)])
}

/// `worksheet.save`'s arguments: the draft record `source`, saved under `name`.
fn save_args(source: &str, name: &str) -> Value {
    save_with(record(source), string(name))
}

/// `worksheet.load`'s arguments: the sheet in record `name` at record version `version` (H-08's
/// `WorksheetRevisionRef`, draft form).
fn load_args(name: &str, version: u64) -> Value {
    json::object([("worksheet", WorksheetRef { record: name.into(), version }.to_json())])
}

/// `workspace.set` from the shell: `payload` under `name`, with `block` as its provenance.
fn put(f: &mut Fixture, name: &str, payload: Value, block: &[u8]) -> Receipt {
    let args = json::object([("payload", payload), ("provenance", string(&text(block))), ("record", string(name))]);
    f.shell.invoke(&mut f.layer, RECORD_ROW, args).expect("the shell prepares the write")
}

/// `worksheet.save` of the draft record `source` under `name`, from the shell.
fn save(f: &mut Fixture, source: &str, name: &str) -> Receipt {
    f.shell.invoke(&mut f.layer, WORKSHEET_SAVE_ROW, save_args(source, name)).expect("the shell prepares the save")
}

/// `worksheet.load` of the sheet in record `name` at `version`, from the shell.
fn load(f: &mut Fixture, name: &str, version: u64) -> Receipt {
    f.shell.invoke(&mut f.layer, WORKSHEET_LOAD_ROW, load_args(name, version)).expect("the shell prepares the reuse")
}

/// `worksheet.compare`'s arguments: the sheet in record `name` at record version `version` and the draft record
/// `draft`, the current work.
fn compare_args(name: &str, version: u64, draft: &str) -> Value {
    json::object([("worksheet", WorksheetRef { record: name.into(), version }.to_json()), ("draft", record(draft))])
}

/// `worksheet.compare` of the sheet in record `name` at its version with the draft record `draft`, from the shell. It
/// must be admitted as a job on the exclusive lane, which runs and completes as a query whose result digests the
/// comparison the layer keeps; returns that comparison.
fn compare(f: &mut Fixture, name: &str, draft: &str) -> Value {
    let args = compare_args(name, version(f, name), draft);
    let receipt = f.shell.invoke(&mut f.layer, WORKSHEET_COMPARE_ROW, args).expect("the shell prepares the comparison");
    let Status::Running(job) = receipt.status else { panic!("the comparison was not admitted: {:?}", receipt.status) };
    assert_eq!(f.layer.run_jobs(), 1);
    let settled = f.layer.outcome(&receipt.command, &receipt.request).expect("the comparison's outcome");
    let value = f.layer.comparison(job).cloned().expect("the comparison the layer keeps");
    match &settled.status {
        Status::Completed(Completion { kind: CompletionKind::Query, result: Some(result), .. }) => {
            assert_eq!(result.digest, digest(&value));
        }
        status => panic!("the comparison did not complete as a query: {status:?}"),
    }
    value
}

/// `value`'s field `key`, which must be there.
fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).unwrap_or_else(|| panic!("no `{key}` in {value:?}"))
}

/// A list of positions as a comparison reports them.
fn ids(list: &[i64]) -> Value {
    Value::Array(list.iter().map(|&id| Value::Int(id)).collect())
}

/// A comparison's macro version: the one the sheet keeps, if any, and its status.
fn macro_status(saved: Option<&MacroVersion>, status: &str) -> Value {
    json::object([("saved", saved.map_or(Value::Null, MacroVersion::to_json)), ("status", string(status))])
}

/// Record `name`'s value and version.
fn held(f: &Fixture, name: &str) -> (Option<RecordValue>, u64) {
    (f.layer.record(name).cloned(), f.layer.records().get(name).map_or(0, |record| record.version))
}

fn version(f: &Fixture, name: &str) -> u64 {
    f.layer.records()[name].version
}

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

/// What the sheet's complete operation does from the solved state, committed inline on a scratch session.
struct Inline {
    /// A piece the operation moves, as KN2.4's `positions` row reads it, and the position it is in afterwards.
    moved: i64,
    to: i64,
    /// The piece whose stickers fill the moved piece's home afterwards, if one piece does.
    occupant: Option<i64>,
    /// A piece the operation leaves where it is.
    stayed: i64,
    /// The end state's digest.
    end: String,
    /// The positions whose home piece is not solved at the end, in increasing order: the operation's losses from the
    /// solved state, where it can gain none (LB2).
    losses: Vec<i64>,
}

fn inline() -> Inline {
    let mut scratch = Fixture::create(Box::new(MemDisk::new()));
    scratch.commit(&OPERATION);
    let all: Vec<i64> = (0..PIECES as i64).collect();
    let found = scratch.layer.positions(&all).expect("every piece's position");
    let moved = found.iter().find(|p| p.position != Some(p.piece)).expect("a piece the operation moves");
    let stayed = found.iter().find(|p| p.position == Some(p.piece)).expect("a piece the operation leaves");
    let to = moved.position.expect("a twist keeps a piece's stickers together");
    Inline {
        moved: moved.piece as i64,
        to: to as i64,
        occupant: found.iter().find(|p| p.position == Some(moved.piece)).map(|p| p.piece as i64),
        stayed: stayed.piece as i64,
        end: scratch.layer.revision().digest.clone(),
        losses: found.iter().filter(|p| !p.solved()).map(|p| p.piece as i64).collect(),
    }
}

/// The draft sheet of the legs: Prepare `[1]`, Macro `[2]` and an empty Cleanup; a piece role on a piece the operation
/// moves, a position role on that piece's home and a piece role on a piece it leaves; and a reference.
fn drafted(inline: &Inline) -> Worksheet {
    let roles = vec![
        Role::Piece { name: "moved".into(), piece: inline.moved },
        Role::Position { name: "target".into(), position: inline.moved },
        Role::Piece { name: "stayed".into(), piece: inline.stayed },
    ];
    let (phases, reference) = ([vec![1], vec![2], Vec::new()], string("reference 1"));
    let sheet = Worksheet { phases, roles, reference, resolution: None, macro_version: None };
    assert_eq!(sheet.letters(), OPERATION.map(i64::from));
    sheet
}

/// A piece role placed where it was saved, at the state it was saved at.
fn placed(name: &str, piece: i64) -> Placement {
    Placement { name: name.into(), piece, saved: Some(piece), now: Some(piece) }
}

/// The reuse leg on a disk that `disk` hands out again for each session, with `power_loss` between the sessions.
///
/// Session 1 saves the draft sheet under `sheet` and presents a preview of it. A reuse at the same state writes the
/// sheet again, equal to what was saved, at its next version, so the preview that read it goes stale: it stays
/// presented, the commit row is unavailable and the press prepared before the reuse commits nothing. Session 2 opens
/// the disk anew: the records, the actions and their receipts come back, and no preview. A reuse there comes back
/// equal again and makes the session's own preview stale; a fresh engine preview of the reused sheet commits its
/// complete operation, the end state of the operation committed inline. Session 3 opens the disk anew and reuses the
/// sheet after that commit: its piece roles are placed again, the moved piece is shown where it is now, the position
/// role on its home reads the piece that fills it now, the phases, the roles and the reference stay as saved, and the
/// reuse's undo restores the sheet as saved.
fn reuse_leg(disk: &dyn Fn() -> Box<dyn Disk>, power_loss: &dyn Fn()) {
    let inline = inline();
    let drafted = drafted(&inline);
    let block = manual();

    // Session 1: the solver writes the draft and saves it as a work sheet, with its piece roles placed at the head
    // and the draft record's block (LB7.2, LB7.3).
    let mut f = Fixture::create(disk());
    let solved = f.layer.revision().digest.clone();
    let draft = put(&mut f, "draft", drafted.to_payload(), &block);
    let saved = save(&mut f, "draft", "sheet");
    assert!(completed(&draft) && completed(&saved), "{:?} {:?}", draft.status, saved.status);
    let pieces = vec![placed("moved", inline.moved), placed("stayed", inline.stayed)];
    let target =
        Occupancy { name: "target".into(), position: inline.moved, saved: Some(inline.moved), now: Some(inline.moved) };
    let positions = vec![target.clone()];
    let resolution = Resolution { saved: solved.clone(), now: solved.clone(), pieces, positions };
    let value = RecordValue {
        payload: Worksheet { resolution: Some(resolution), ..drafted.clone() }.to_payload(),
        provenance: text(&block),
    };
    assert_eq!(held(&f, "sheet"), (Some(value.clone()), 1));

    // A reuse of the sheet at its version comes back equal at the next version, and the presented preview that read
    // the sheet goes stale (KN0.8).
    let id = f.preview(record_draft("sheet"));
    let press = f.press(id);
    let reused = load(&mut f, "sheet", 1);
    assert!(completed(&reused), "{:?}", reused.status);
    assert_eq!(held(&f, "sheet"), (Some(value.clone()), 2));
    assert_eq!(f.layer.preview_state(id), Some(PreviewState::Presented));
    let stale = f.layer.availability("operation.execute").map_err(|error| error.code);
    assert_eq!(stale, Err(code::REVISION_CHANGED));
    assert_eq!(refused(&f.shell.send(&mut f.layer, press)), Some(code::REVISION_CHANGED));
    assert_eq!(f.layer.revision().digest, solved, "nothing was committed");
    let kept = f.kept();
    drop(f);
    power_loss();

    // Session 2: the records, the actions and their receipts are durable (KN5.23); no preview is.
    let mut f = Fixture::open(disk());
    assert_eq!(f.kept(), kept);
    for receipt in [&draft, &saved, &reused] {
        assert_eq!(f.layer.outcome(&receipt.command, &receipt.request).as_ref(), Some(receipt), "{}", receipt.command);
    }
    assert!(f.layer.preview().is_none(), "a new session holds no preview");
    assert_eq!(held(&f, "sheet"), (Some(value.clone()), 2));
    let id = f.preview(record_draft("sheet"));
    let press = f.press(id);
    let again = load(&mut f, "sheet", 2);
    assert!(completed(&again), "{:?}", again.status);
    assert_eq!(held(&f, "sheet"), (Some(value.clone()), 3));
    assert_eq!(refused(&f.shell.send(&mut f.layer, press)), Some(code::REVISION_CHANGED));

    // The sheet stores no preview that could stand in for a fresh one (LB7.2): a fresh engine preview of the reused
    // sheet commits its complete operation.
    let fresh = f.preview(record_draft("sheet"));
    let press = f.press(fresh);
    let commit = f.shell.send(&mut f.layer, press);
    assert!(committed(&commit), "{:?}", commit.status);
    assert_eq!(f.layer.revision().digest, inline.end);
    let kept = f.kept();
    drop(f);
    power_loss();

    // Session 3: the commit left the sheet as it was. A reuse places its piece roles again at the new head (LB2: reuse
    // resolves roles again) and shows the moved piece where it is now beside where it was saved, and the piece that
    // fills its home now beside the one saved there; the phases, the roles and the reference stay, so the sheet is
    // never retargeted (solving workflow section 4.1, draft).
    let mut f = Fixture::open(disk());
    assert_eq!(f.kept(), kept);
    assert_eq!(held(&f, "sheet"), (Some(value.clone()), 3));
    assert!(completed(&load(&mut f, "sheet", 3)));
    let reused = Worksheet::from_payload(&f.layer.record("sheet").expect("the sheet").payload).expect("a work sheet");
    assert_eq!(
        (&reused.phases, &reused.roles, &reused.reference),
        (&drafted.phases, &drafted.roles, &drafted.reference)
    );
    let moved = Placement { now: Some(inline.to), ..placed("moved", inline.moved) };
    let pieces = vec![moved, placed("stayed", inline.stayed)];
    let positions = vec![Occupancy { now: inline.occupant, ..target }];
    let expected = Resolution { saved: solved, now: inline.end.clone(), pieces, positions };
    assert_eq!(reused.resolution.as_ref(), Some(&expected));
    assert_eq!(expected.pieces.iter().map(Placement::moved).collect::<Vec<_>>(), [true, false]);
    assert!(expected.positions[0].changed(), "another piece fills the moved piece's home: {:?}", inline.occupant);

    // The reuse is an undoable workspace action (LB7.2): its undo restores the sheet as saved.
    let hint = f.hint(Direction::Undo);
    assert!(committed(&f.step(Direction::Undo, hint)));
    assert_eq!(f.layer.record("sheet"), Some(&value));
}

/// CT-LB2-03 (part: named work-sheet reuse invalidates the old preview, through `worksheet.save` and `worksheet.load`
/// on the layer over M6's journal on a memory disk, with a power loss between the sessions: a saved sheet comes back
/// equal at a reuse in the same session and after the power loss, each reuse makes the presented preview of the sheet
/// stale so that its press commits nothing, a fresh engine preview commits the sheet's complete operation, and a reuse
/// after that commit places the piece roles again without retargeting the sheet; not run here: LB7.2's fresh exact
/// analysis of a reused sheet through `worksheet.compare`, which
/// [`ct_lb2_03_a_comparison_writes_nothing_and_comes_back_equal_after_a_power_loss`] runs; the first sentence, recipe
/// chunking and the refusal of an oversized recipe, which `c600-kernel` and `c600-tools` run), LB7.2, KN0.8, KN5.23.
/// The steps are at [`reuse_leg`].
#[test]
fn ct_lb2_03_a_reused_work_sheet_comes_back_equal_after_a_power_loss() {
    let disk = MemDisk::new();
    reuse_leg(&|| Box::new(disk.clone()) as Box<dyn Disk>, &|| disk.crash());
}

/// CT-LB2-03 (part: named work-sheet reuse invalidates the old preview, through `worksheet.save` and `worksheet.load`
/// on the layer over M6's journal in a fresh directory of the file system, which each new session opens again: a
/// saved sheet comes back equal at a reuse in the same session and in a new session, each reuse makes the presented
/// preview of the sheet stale so that its press commits nothing, a fresh engine preview commits the sheet's complete
/// operation, and a reuse after that commit places the piece roles again without retargeting the sheet; not run here:
/// LB7.2's fresh exact analysis of a reused sheet through `worksheet.compare`, which
/// [`ct_lb2_03_a_comparison_writes_nothing_and_comes_back_equal_after_a_power_loss`] runs on a memory disk; the first
/// sentence, recipe chunking and the refusal of an oversized recipe, which `c600-kernel` and `c600-tools` run), LB7.2,
/// KN0.8, KN5.23. The steps are at [`reuse_leg`].
#[test]
fn ct_lb2_03_a_reused_work_sheet_comes_back_equal_in_a_new_session_on_disk() {
    let dir = TempDir::new("ct-lb2-03");
    let path = dir.path().to_path_buf();
    reuse_leg(&|| Box::new(FsDisk::new(&path).expect("a store directory")) as Box<dyn Disk>, &|| {});
}

/// CT-LB2-03 (part: named work-sheet reuse with LB7.2's fresh exact analysis of the sheet, through `worksheet.compare`
/// on the layer over M6's journal on a memory disk, with a power loss between the sessions: a comparison completes as
/// a query whose result digests it, writes nothing to the journal and keeps a presented preview fresh; its analysis of
/// the sheet's complete operation from the head gains nothing, loses the positions the operation unfinishes, as LB2
/// counts them, and ends where the operation committed inline ends; a sheet saved from a record of letters reports that
/// record as its current macro version, and a sheet saved from a sheet without one reports none; after the power loss
/// both comparisons come back equal, and a write of the record of letters makes the macro version changed while the
/// sheet's own word is still the one analysed; not run: the first sentence, recipe chunking and the refusal of an
/// oversized recipe, which `c600-kernel` and `c600-tools` run), LB7.2, LB2, KN5.23.
#[test]
fn ct_lb2_03_a_comparison_writes_nothing_and_comes_back_equal_after_a_power_loss() {
    let disk = MemDisk::new();
    let inline = inline();
    let drafted = drafted(&inline);
    let block = manual();

    // Session 1: the draft sheet saved under `sheet`, and the operation's letters in record `w` saved under `macro`,
    // which keeps `w`'s name, version and content digest as its macro version (LB7.2).
    let mut f = Fixture::create(Box::new(disk.clone()));
    let solved = f.layer.revision().digest.clone();
    assert!(completed(&put(&mut f, "draft", drafted.to_payload(), &block)));
    assert!(completed(&save(&mut f, "draft", "sheet")));
    assert!(completed(&f.write("w", &OPERATION, &block)));
    assert!(completed(&save(&mut f, "w", "macro")));
    let source = f.layer.record("w").expect("the letters").digest();
    let saved = MacroVersion { record: "w".into(), version: version(&f, "w"), digest: source };
    let id = f.preview(record_draft("sheet"));

    // A comparison is a query: it writes nothing to the journal and keeps the presented preview fresh.
    let before = fixed(&f.layer, &disk);
    let sheet = compare(&mut f, "sheet", "draft");
    let letters = compare(&mut f, "macro", "w");
    assert_eq!(fixed(&f.layer, &disk), before);
    assert_eq!(f.layer.preview_state(id), Some(PreviewState::Presented));
    assert_eq!(f.layer.availability("operation.execute"), Ok(()));

    // Both sheets hold the same complete operation, analysed afresh from the solved head: it gains nothing, loses the
    // positions it unfinishes and ends where the operation committed inline ends (LB2). No piece role has moved.
    for value in [&sheet, &letters] {
        let analysis = field(value, "analysis");
        assert_eq!((field(analysis, "base"), field(analysis, "end")), (&string(&solved), &string(&inline.end)));
        assert_eq!((field(analysis, "gains"), field(analysis, "losses")), (&ids(&[]), &ids(&inline.losses)));
        assert_eq!(field(value, "moved"), &Value::Array(Vec::new()));
        assert_eq!(field(value, "occupant_changed"), &Value::Array(Vec::new()));
    }
    assert!(!inline.losses.is_empty());
    assert_eq!(field(&sheet, "macro_version"), &macro_status(None, "none"));
    assert_eq!(field(&letters, "macro_version"), &macro_status(Some(&saved), "current"));
    drop(f);
    disk.crash();

    // Session 2: the sheets, the records they name and the state are durable (KN5.23), so both comparisons come back
    // equal, and they still write nothing.
    let mut f = Fixture::open(Box::new(disk.clone()));
    let before = fixed(&f.layer, &disk);
    assert_eq!(compare(&mut f, "sheet", "draft"), sheet);
    assert_eq!(compare(&mut f, "macro", "w"), letters);
    assert_eq!(fixed(&f.layer, &disk), before);

    // A write of the record of letters makes the macro version changed; the sheet's own word is still the one analysed.
    assert!(completed(&f.write("w", &[2, 1], &block)));
    let changed = compare(&mut f, "macro", "w");
    assert_eq!(field(&changed, "macro_version"), &macro_status(Some(&saved), "changed"));
    assert_eq!(field(&changed, "analysis"), field(&letters, "analysis"));
}

/// A test reader that reads every block. It stands in for a host's reader that accepted a block C5's reader does not
/// read.
struct Lenient;

impl Provenance for Lenient {
    fn check(&self, _: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn admit(&self, _: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

/// Payloads that are no work sheet: the stored form with one of `Worksheet::from_payload`'s rules broken.
fn malformed() -> Vec<(&'static str, Value)> {
    let sheet = |roles: Vec<Role>, letters: Vec<i64>| Worksheet {
        phases: [Vec::new(), letters, Vec::new()],
        roles,
        reference: Value::Null,
        resolution: None,
        macro_version: None,
    };
    let piece = |piece: i64| Role::Piece { name: "a".into(), piece };
    let twice = vec![piece(9), Role::Position { name: "a".into(), position: 7 }];
    let unplaced = Resolution { saved: String::new(), now: String::new(), pieces: Vec::new(), positions: Vec::new() };
    let short = br#"{"worksheet": {"phases": {"prepare": [], "macro": [1]}, "roles": [], "reference": null}}"#;
    vec![
        ("twice", sheet(twice, vec![1]).to_payload()),
        ("far", sheet(vec![piece(PIECES as i64)], vec![1]).to_payload()),
        ("wide", sheet(Vec::new(), vec![1 << 31]).to_payload()),
        ("unplaced", Worksheet { resolution: Some(unplaced), ..sheet(vec![piece(9)], vec![1]) }.to_payload()),
        ("short", json::parse(short).expect("JSON")),
    ]
}

/// The digest of the journal's bytes, written or synced, read through a clone of the disk.
fn journal(disk: &MemDisk) -> String {
    sha256_hex(&disk.clone().read(JOURNAL).expect("a readable disk").expect("a journal"))
}

/// What a refusal may not change: the revision, the records, the pending preview and the journal's bytes.
fn fixed(layer: &CommandLayer<JournalStore>, disk: &MemDisk) -> String {
    let pending = layer.preview().map(|preview| preview.id);
    format!("{:?} {:?} {pending:?} {}", layer.revision(), layer.records(), journal(disk))
}

/// Sends `row` with `args` from the shell, which must be refused with `expected` and change nothing.
fn refuse(f: &mut Fixture, disk: &MemDisk, row: &str, args: Value, expected: &str, what: &str) {
    let before = fixed(&f.layer, disk);
    let receipt = f.shell.invoke(&mut f.layer, row, args).expect("the shell prepares the row");
    assert_eq!(refused(&receipt), Some(expected), "{what}: {:?}", receipt.status);
    assert_eq!(fixed(&f.layer, disk), before, "{what}");
}

/// CT-LB2-03 (part: the refusals that LP2's draft rows give for a work sheet, `worksheet.save`, `worksheet.load` and
/// `worksheet.compare`, on the layer over M6's journal after a power loss, each with the layer's code, with no job
/// queued and with no change to the revision, the records, the pending preview or the journal's bytes: without the
/// host's reader no row is built; the runtime may invoke none; a malformed argument; an unknown or removed record, a
/// record that holds no operation to save, no saved sheet to reuse or compare, or no work to compare with, and a
/// malformed sheet; a sheet named at another version; and a block that C5's reader refuses, which a test reader
/// accepted before the power loss and the journal replays without a check; not run: Jumble on, which needs a twist
/// menu pinned from `c600-jumble`, not a dependency of this crate, and which `c600-command`'s `tests/jumble_on.rs`
/// runs for the save and the reuse; a comparison while the exclusive lane is held (`KN7:busy.lane_full`), which
/// `c600-command`'s `tests/worksheet.rs` runs), LB7.2, LB7.3, PS1.16.
#[test]
fn ct_lb2_03_the_work_sheet_rows_refuse_on_the_journal_and_change_nothing() {
    let disk = MemDisk::new();
    let block = manual();
    let mut f = Fixture::create(Box::new(disk.clone()));
    // A record written and undone holds no value; the others hold letters, text, a sheet not saved yet and malformed
    // sheets, which `workspace.set` stores as any other payload. `sheet` is saved from the letters.
    assert!(completed(&f.write("gone", &[1], &block)));
    let hint = f.hint(Direction::Undo);
    assert!(committed(&f.step(Direction::Undo, hint)));
    assert!(completed(&f.write("w", &OPERATION, &block)));
    assert!(completed(&put(&mut f, "text", string("not letters"), &block)));
    let unsaved = Worksheet {
        phases: [Vec::new(), vec![1], Vec::new()],
        roles: Vec::new(),
        reference: Value::Null,
        resolution: None,
        macro_version: None,
    };
    assert!(completed(&put(&mut f, "draft", unsaved.to_payload(), &block)));
    for (name, payload) in malformed() {
        assert!(completed(&put(&mut f, name, payload, &block)), "{name}");
    }
    assert!(completed(&save(&mut f, "w", "sheet")));
    // Under the test reader, a record with a block that C5's reader does not read, and a sheet saved from it, which
    // keeps the block (LB7.3).
    f.layer.set_provenance(Box::new(Lenient));
    assert!(completed(&f.write("foreign", &OPERATION, FOREIGN)));
    assert!(completed(&save(&mut f, "foreign", "kept")));
    drop(f);
    disk.crash();

    // Opened without the host's reader, the journal's records come back and neither row is built, in its availability
    // and when the shell sends it, and nothing changes.
    let store = JournalStore::open(Box::new(disk.clone())).expect("the journal opens");
    let mut bare = match CommandLayer::open(kernel(), workspace::table(), Secret::fixed(SECRET), store) {
        Ok(layer) => layer,
        Err(error) => panic!("the session does not reopen: {error:?}"),
    };
    let mut shell = Shell::new(Secret::fixed(SECRET), "shell");
    let unchanged = fixed(&bare, &disk);
    let rows = [
        (WORKSHEET_SAVE_ROW, save_args("w", "s")),
        (WORKSHEET_LOAD_ROW, load_args("sheet", 1)),
        (WORKSHEET_COMPARE_ROW, compare_args("sheet", 1, "w")),
    ];
    for (row, args) in rows {
        assert_eq!(bare.availability(row).map_err(|error| error.code), Err(code::ROW_NOT_BUILT), "{row}");
        let receipt = shell.invoke(&mut bare, row, args).expect("the shell prepares the row");
        assert_eq!(refused(&receipt), Some(code::ROW_NOT_BUILT), "{row}");
    }
    assert_eq!(fixed(&bare, &disk), unchanged, "a session without the reader");
    assert_eq!(bare.run_jobs(), 0, "a session without the reader");
    drop(bare);

    // With C5's reader installed, the three rows are available. A preview of the sheet is presented; no refusal below
    // makes it stale.
    let mut f = Fixture::open(Box::new(disk.clone()));
    for row in [WORKSHEET_SAVE_ROW, WORKSHEET_LOAD_ROW, WORKSHEET_COMPARE_ROW] {
        assert_eq!(f.layer.availability(row), Ok(()), "{row}");
    }
    assert_eq!(f.layer.record("kept").map(|value| value.provenance.clone()), Some(text(FOREIGN)));
    f.preview(record_draft("sheet"));

    // The runtime may invoke none of the rows: a work sheet changes only in a command the solver invokes (LB7.2), and
    // LP2's draft gives the comparison the same callers.
    let before = fixed(&f.layer, &disk);
    {
        let mut bridge = Bridge::new(&mut f.layer, "run-1");
        let saved = bridge.invoke(WORKSHEET_SAVE_ROW, save_args("w", "s"), None);
        let loaded = bridge.invoke(WORKSHEET_LOAD_ROW, load_args("sheet", 1), None);
        let compared = bridge.invoke(WORKSHEET_COMPARE_ROW, compare_args("sheet", 1, "w"), None);
        let codes = [refused(&saved), refused(&loaded), refused(&compared)];
        assert_eq!(codes, [Some(code::NOT_GRANTED); 3]);
    }
    assert_eq!(fixed(&f.layer, &disk), before, "the runtime's invocations");

    // A malformed argument, among them a recipe draft, a third argument and a missing reference.
    let recipe = json::parse(br#"[{"kind": "word", "moves": [1]}]"#).expect("JSON");
    let bad_saves = [
        ("a recipe draft", save_with(json::object([("recipe", recipe.clone())]), string("s"))),
        ("an empty name", save_with(record("w"), string(""))),
        ("a third argument", json::object([("draft", record("w")), ("name", string("s")), ("x", Value::Null)])),
    ];
    for (what, args) in bad_saves {
        refuse(&mut f, &disk, WORKSHEET_SAVE_ROW, args, code::ARGUMENT_FORM, what);
    }
    let negative = json::object([("record", string("sheet")), ("version", Value::Int(-1))]);
    let bad_loads = [
        ("a reference that is not an object", json::object([("worksheet", string("sheet"))])),
        ("a negative version", json::object([("worksheet", negative)])),
        ("a missing reference", json::object([])),
    ];
    for (what, args) in bad_loads {
        refuse(&mut f, &disk, WORKSHEET_LOAD_ROW, args, code::ARGUMENT_FORM, what);
    }
    let at = || WorksheetRef { record: "sheet".into(), version: 1 }.to_json();
    let with = |draft: Value| json::object([("draft", draft), ("worksheet", at())]);
    let bad_compares = [
        ("a recipe draft to compare with", with(json::object([("recipe", recipe)]))),
        ("a draft with a second key", with(json::object([("record", string("w")), ("x", Value::Null)]))),
        ("a missing draft", json::object([("worksheet", at())])),
        ("a missing sheet reference", json::object([("draft", record("w"))])),
        (
            "a third comparison argument",
            json::object([("draft", record("w")), ("worksheet", at()), ("x", Value::Null)]),
        ),
    ];
    for (what, args) in bad_compares {
        refuse(&mut f, &disk, WORKSHEET_COMPARE_ROW, args, code::ARGUMENT_FORM, what);
    }

    // An unknown or removed record, a record that holds no operation to save or no saved sheet to reuse, and a
    // malformed sheet are no work sheet (`KN2:input.named_object_invalid`).
    let invalid = code::NAMED_OBJECT_INVALID;
    let shapes: Vec<&str> = malformed().iter().map(|(name, _)| *name).collect();
    for source in ["missing", "gone", "text"].into_iter().chain(shapes.iter().copied()) {
        refuse(&mut f, &disk, WORKSHEET_SAVE_ROW, save_args(source, "s"), invalid, source);
    }
    refuse(&mut f, &disk, WORKSHEET_LOAD_ROW, load_args("missing", 0), invalid, "missing");
    for name in ["gone", "w", "draft"].into_iter().chain(shapes.iter().copied()) {
        let at = version(&f, name);
        refuse(&mut f, &disk, WORKSHEET_LOAD_ROW, load_args(name, at), invalid, name);
    }
    // The same records are no saved sheet to compare, and a missing or removed record, or one that holds neither
    // letters nor a sheet, is no work to compare with.
    refuse(&mut f, &disk, WORKSHEET_COMPARE_ROW, compare_args("missing", 0, "w"), invalid, "a missing sheet");
    for name in ["gone", "w", "draft"].into_iter().chain(shapes.iter().copied()) {
        let at = version(&f, name);
        refuse(&mut f, &disk, WORKSHEET_COMPARE_ROW, compare_args(name, at, "w"), invalid, name);
    }
    for draft in ["missing", "gone", "text"] {
        refuse(&mut f, &disk, WORKSHEET_COMPARE_ROW, compare_args("sheet", 1, draft), invalid, draft);
    }

    // A sheet named at another version than its record's (`KN5:stale.revision_changed`).
    for stale in [0, 2] {
        refuse(&mut f, &disk, WORKSHEET_LOAD_ROW, load_args("sheet", stale), code::REVISION_CHANGED, "another version");
        let args = compare_args("sheet", stale, "w");
        refuse(&mut f, &disk, WORKSHEET_COMPARE_ROW, args, code::REVISION_CHANGED, "a comparison at another version");
    }

    // C5's reader reads the block again at a save, at a reuse and at a comparison (PS1.16, KN5.26): a block it does not
    // read, replayed from the journal, is refused with its error.
    let incompatible = code::PROVENANCE_INCOMPATIBLE;
    refuse(&mut f, &disk, WORKSHEET_SAVE_ROW, save_args("foreign", "s"), incompatible, "a save of the block");
    let at = version(&f, "kept");
    refuse(&mut f, &disk, WORKSHEET_LOAD_ROW, load_args("kept", at), incompatible, "a reuse of the block");
    let args = compare_args("kept", at, "w");
    refuse(&mut f, &disk, WORKSHEET_COMPARE_ROW, args, incompatible, "a comparison of the block");
    assert_eq!(f.layer.run_jobs(), 0, "no refusal queues a job");

    // The preview is still fresh and no record `s` was written. The sheet is still reused at its version, and that
    // reuse writes the journal and makes the preview stale, as each refusal above would have if it had written.
    assert_eq!(f.layer.availability("operation.execute"), Ok(()));
    assert!(!f.layer.records().contains_key("s"));
    let before = journal(&disk);
    assert!(completed(&load(&mut f, "sheet", 1)));
    assert_ne!(journal(&disk), before);
    let stale = f.layer.availability("operation.execute").map_err(|error| error.code);
    assert_eq!(stale, Err(code::REVISION_CHANGED));
}
