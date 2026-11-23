//! LP4-12: the 1.0 log format and its compiler. A log written from a session passes every gate with the session's
//! final hash; a changed model identity fails `model_match`; a move without its legal witness, a flipped byte and a
//! wrong final hash each fail with their own code and leave the log read-only with no bound state; embedded scripts
//! are compiled by the editor services; open documents keep their saved digest and unsaved contents (R-3).

mod common;

use c600_editor::{diagnose, Document, ModuleContext, Units, Verdict};
use c600_kernel::json::{self, Value};
use c600_log::{
    code, compile, read, record, write, Author, CursorAction, DocumentIdentity, Extension, Gate, GateResult, Log,
    Macro, MoveAction, OpenDocument, Recorder, Script, Start,
};

use common::{kernel, sha256};

const TOOL: &str = include_str!("../../c600-tools/src/full_600_cell.6cs");

fn word(letters: &[i64]) -> Value {
    Value::Array(vec![json::object([
        ("kind", Value::String("word".to_owned())),
        ("moves", Value::Array(letters.iter().map(|&m| Value::Int(m)).collect())),
    ])])
}

fn star(orbit: i64, node: i64, sign: i64) -> Value {
    Value::Array(vec![json::object([
        ("kind", Value::String("star".to_owned())),
        ("node", Value::Int(node)),
        ("orbit", Value::Int(orbit)),
        ("sign", Value::Int(sign)),
    ])])
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

/// A session with commits, an undo and a new branch, partial undos, a branch switch, a workspace action, layout steps
/// and an undo, open documents, a macro, a Library entry, a script and preferences. Returns the log and the state at
/// the cursor.
fn session() -> (Log, String) {
    let k = kernel();
    let mut r = Recorder::new(k);
    let first = r.commit(&word(&[2, 4, 6]), "hand", "first", Some("2026-11-23T10:00:00Z")).unwrap();
    let starred = r.commit(&star(33, 0, 1), "macro", "star", Some("2026-11-23T10:00:05Z")).unwrap();
    assert!(r.undo(None));
    assert_eq!(r.head(), Some(first.as_str()));
    let branch = r.commit(&word(&[8, -8, 1200]), "hand", "branch", None).unwrap();
    let kept_one = r.partial_undo(1, None).expect("inside the branch commit");
    assert!(r.partial_undo(3, None).is_none(), "a partial undo keeps fewer letters than the commit");
    r.partial_undo(0, None).expect("back to the commit's pre state");
    assert_ne!(kept_one, branch);
    assert!(r.select(Some(&starred), None), "the abandoned branch is still there");
    r.workspace("macro.save", json::object([("name", text("corner"))]), None);
    r.commit(&word(&[5, -7]), "script", "from a preview", None).unwrap();
    assert!(r.undo(None));
    assert!(r.redo(None));
    r.layout_step("camera", "hand", json::object([("cell", Value::Int(7))]), None);
    r.layout_step("filter", "script", json::object([("expr", text("all"))]), None);
    assert!(r.layout_undo(None));
    let state = r.state().digest().to_owned();

    let log = r.log_mut();
    log.view = json::object([("binding", text("live"))]);
    log.layout = json::object([("panels", Value::Array(vec![text("history"), text("problems")]))]);
    log.workspace = json::object([("tabs", Value::Int(3))]);
    log.preferences = Some(r#"{"orbit":2,"pin_safety":false}"#.to_owned());
    let saved = b"fn old() -> Nat { return 1; }\n".to_vec();
    log.documents = vec![
        OpenDocument {
            identity: DocumentIdentity::Path("work/corner.6cs".to_owned()),
            kind: "script".to_owned(),
            saved: Some(sha256(&saved)),
            dirty: true,
            contents: b"fn new() -> Nat { return 2; }\n".to_vec(),
        },
        OpenDocument {
            identity: DocumentIdentity::Untitled("untitled-1".to_owned()),
            kind: "script".to_owned(),
            saved: None,
            dirty: true,
            contents: "// never ran: \u{2028} \u{1f}\n".as_bytes().to_vec(),
        },
        OpenDocument {
            identity: DocumentIdentity::Path("logs/old.c600log".to_owned()),
            kind: "log".to_owned(),
            saved: Some(sha256(&[0x1f, 0x8b, 0xff, 0x00])),
            dirty: false,
            contents: vec![0x1f, 0x8b, 0xff, 0x00],
        },
    ];
    let steps = word(&[1, 2, 3]);
    let letters: Vec<u8> = [1i32, 2, 3].iter().flat_map(|m| m.to_le_bytes()).collect();
    log.macros = vec![Macro {
        id: "m1".to_owned(),
        name: "three".to_owned(),
        author: Author::Own,
        steps,
        primitive_count: 3,
        expansion: sha256(&letters),
    }];
    log.library = vec![c600_log::LibraryEntry {
        id: "l1".to_owned(),
        name: "corner card".to_owned(),
        author: Author::BuiltIn,
        kind: "nexus_card".to_owned(),
        value: json::object([("orbit", Value::Int(4))]),
    }];
    log.scripts = vec![Script::new("full_600_cell", Author::BuiltIn, TOOL)];
    (r.finish(), state)
}

#[test]
fn a_session_log_passes_every_gate_with_the_sessions_final_hash() {
    let (log, state) = session();
    let bytes = write(&log).unwrap();
    assert!(bytes.ends_with(b"\n") && !bytes.ends_with(b"\n\n"));
    let compiled = compile(kernel(), &bytes);
    assert!(compiled.passed(), "{:#?}", compiled.problems());
    assert_eq!(compiled.gates().iter().map(|(g, _)| *g).collect::<Vec<_>>(), Gate::COMPILE);
    assert!(compiled.problems().is_empty());
    assert!(!compiled.read_only());
    assert_eq!(compiled.state().unwrap().digest(), state);
    assert_eq!(log.saved.state, state);
    // Every recorded part survives the record whole.
    assert_eq!(compiled.log(), Some(&log));
    assert_eq!(read(&bytes).unwrap(), log);
    // The result is bound to these bytes.
    assert!(compiled.binds(&bytes));
    assert_eq!(compiled.bytes_sha256(), sha256(&bytes));
    let mut edited = bytes.clone();
    edited.push(b'\n');
    assert!(!compiled.binds(&edited));
    // The histories hold the abandoned branch, the partial undos and both cursors' actions.
    let kinds: Vec<&str> = log.moves.iter().map(|e| e.action.kind()).collect();
    assert_eq!(
        kinds,
        [
            "commit",
            "commit",
            "cursor",
            "commit",
            "partial_undo",
            "partial_undo",
            "cursor",
            "workspace",
            "commit",
            "cursor",
            "cursor"
        ]
    );
    let cursors: Vec<CursorAction> = log
        .moves
        .iter()
        .filter_map(|e| match &e.action {
            MoveAction::Cursor(c) => Some(c.action),
            _ => None,
        })
        .collect();
    assert_eq!(cursors, [CursorAction::Undo, CursorAction::Select, CursorAction::Undo, CursorAction::Redo]);
    assert_eq!(log.layout_history.len(), 3);
    // The embedded script was compiled by the editor services, exactly as diagnose answers for its text.
    let check = &compiled.scripts()[0];
    let document = Document::open(TOOL.as_bytes(), Units::Utf8);
    let context = ModuleContext::new().with_dependency(TOOL.as_bytes(), None);
    let fresh = diagnose(&document, document.digest(), &context).unwrap();
    assert_eq!(check.verdict.as_ref(), Some(&fresh.verdict));
    assert!(matches!(check.verdict, Some(Verdict::Checked(_))), "{:#?}", check.diagnostics);
    assert_eq!(check.author, Author::BuiltIn);
}

fn first_problem(compiled: &c600_log::Compiled) -> (&'static str, Gate) {
    let problem = compiled.problems().first().expect("a problem");
    (problem.code, problem.gate)
}

/// A log that did not pass the binding gates opens read-only with its problems and no state.
fn assert_read_only(compiled: &c600_log::Compiled) {
    assert!(compiled.read_only());
    assert!(compiled.state().is_none());
    assert!(!compiled.passed());
}

#[test]
fn a_changed_model_identity_fails_model_match() {
    let (mut log, _) = session();
    log.identity.model_id = "0".repeat(64);
    let compiled = compile(kernel(), &write(&log).unwrap());
    assert_eq!(first_problem(&compiled), (code::IDENTITY_MISMATCH, Gate::ModelMatch));
    assert_eq!(code::IDENTITY_MISMATCH, "KN1:input.identity_mismatch");
    assert_eq!(compiled.gate(Gate::Format), GateResult::Passed);
    assert_eq!(compiled.gate(Gate::ModelMatch), GateResult::Failed);
    for gate in [Gate::Moves, Gate::FinalHash, Gate::Macros] {
        assert_eq!(compiled.gate(gate), GateResult::NotRun, "{gate:?}");
    }
    assert_read_only(&compiled);
}

#[test]
fn a_move_without_its_legal_witness_fails_the_moves_gate() {
    let (log, _) = session();
    for broken in [Value::Null, word(&[0]), word(&[1201]), Value::Array(vec![])] {
        let mut log = log.clone();
        let MoveAction::Commit(commit) = &mut log.moves[0].action else { panic!() };
        commit.recipe = broken.clone();
        let compiled = compile(kernel(), &write(&log).unwrap());
        assert_eq!(first_problem(&compiled), (code::RECIPE_SHAPE, Gate::Moves), "{broken:?}");
        assert_eq!(code::RECIPE_SHAPE, "KN2:input.recipe_shape");
        assert_eq!(compiled.gate(Gate::FinalHash), GateResult::NotRun);
        assert_read_only(&compiled);
    }
    // A legal recipe that is not the recorded move: the replay disagrees with the recorded post state.
    let mut log = log.clone();
    let MoveAction::Commit(commit) = &mut log.moves[0].action else { panic!() };
    commit.recipe = word(&[2, 4, 7]);
    let compiled = compile(kernel(), &write(&log).unwrap());
    assert_eq!(first_problem(&compiled), (code::CHECKPOINT_REPLAY, Gate::Moves));
    assert_read_only(&compiled);
}

#[test]
fn a_flipped_byte_fails_the_format_gate_wherever_it_is() {
    let (log, _) = session();
    let bytes = write(&log).unwrap();
    // A changed letter inside a text value still reads as canonical JSON: the seal refuses it.
    let at = bytes.windows(14).position(|w| w == b"from a preview").unwrap();
    let mut flipped = bytes.clone();
    flipped[at] ^= 0x01;
    let compiled = compile(kernel(), &flipped);
    assert_eq!(first_problem(&compiled), (code::DIGEST_MISMATCH, Gate::Format));
    assert_eq!(code::DIGEST_MISMATCH, "PS1:input.digest_mismatch");
    assert_read_only(&compiled);
    assert!(compiled.log().is_none());
    // Every bit flip anywhere is refused by the format gate.
    for at in (0..bytes.len()).step_by(37) {
        let mut flipped = bytes.clone();
        flipped[at] ^= 1 << (at % 8);
        let compiled = compile(kernel(), &flipped);
        assert_eq!(compiled.gate(Gate::Format), GateResult::Failed, "byte {at}");
        assert_eq!(first_problem(&compiled).1, Gate::Format);
        assert_read_only(&compiled);
    }
}

#[test]
fn a_wrong_final_hash_fails_the_final_hash_gate() {
    let (mut log, _) = session();
    log.saved.state = kernel().solved().digest().to_owned();
    let compiled = compile(kernel(), &write(&log).unwrap());
    assert_eq!(compiled.gate(Gate::Moves), GateResult::Passed);
    assert_eq!(first_problem(&compiled), (code::CHECKPOINT_REPLAY, Gate::FinalHash));
    assert_eq!(code::CHECKPOINT_REPLAY, "PS3:input.checkpoint_replay");
    assert_read_only(&compiled);
}

#[test]
fn embedded_scripts_compile_with_editor_diagnostics_and_do_not_unbind_the_state() {
    let (mut log, state) = session();
    log.scripts.push(Script::new("broken", Author::Imported, "fn broken( {\n"));
    let compiled = compile(kernel(), &write(&log).unwrap());
    assert_eq!(compiled.gate(Gate::Scripts), GateResult::Failed);
    assert!(!compiled.passed());
    // The four binding gates passed, so the saved state is bound; the script's diagnostics are Problems rows.
    assert_eq!(compiled.state().unwrap().digest(), state);
    let check = &compiled.scripts()[1];
    assert_eq!((check.name.as_str(), check.author), ("broken", Author::Imported));
    assert!(!check.diagnostics.is_empty());
    assert!(!matches!(check.verdict, Some(Verdict::Checked(_))));
    let scripts: Vec<&str> = compiled.problems().iter().filter(|p| p.gate == Gate::Scripts).map(|p| p.code).collect();
    assert_eq!(scripts, check.diagnostics.iter().map(|e| e.code).collect::<Vec<_>>());

    // A script whose text differs from its source digest is not diagnosed.
    let (mut log, _) = session();
    log.scripts[0].text.push('\n');
    let compiled = compile(kernel(), &write(&log).unwrap());
    assert_eq!(first_problem(&compiled), (code::DIGEST_MISMATCH, Gate::Scripts));
    assert!(compiled.scripts()[0].verdict.is_none());
}

#[test]
fn a_macro_whose_witness_differs_fails_the_macro_gate() {
    let (mut log, _) = session();
    log.macros[0].primitive_count = 4;
    let compiled = compile(kernel(), &write(&log).unwrap());
    assert_eq!(first_problem(&compiled), (code::DIGEST_MISMATCH, Gate::Macros));
    assert!(!compiled.read_only());
    let (mut log, _) = session();
    log.macros[0].steps = Value::Null;
    let compiled = compile(kernel(), &write(&log).unwrap());
    assert_eq!(first_problem(&compiled), (code::RECIPE_SHAPE, Gate::Macros));
}

#[test]
fn open_documents_keep_their_saved_digest_and_unsaved_contents() {
    let (log, _) = session();
    let compiled = compile(kernel(), &write(&log).unwrap());
    let documents = &compiled.log().unwrap().documents;
    assert_eq!(documents, &log.documents);
    let dirty = &documents[0];
    assert!(dirty.dirty);
    assert_eq!(dirty.saved.as_deref(), Some(sha256(b"fn old() -> Nat { return 1; }\n").as_str()));
    assert_eq!(dirty.contents, b"fn new() -> Nat { return 2; }\n");
    assert_eq!(documents[1].identity, DocumentIdentity::Untitled("untitled-1".to_owned()));
    assert_eq!(documents[1].saved, None);
    // Saved again and reopened, the same document still holds both.
    let again = compile(kernel(), &write(compiled.log().unwrap()).unwrap());
    assert_eq!(again.log().unwrap().documents, log.documents);

    // A clean document whose contents are not its saved bytes is refused.
    let (mut log, _) = session();
    log.documents[2].contents.push(0);
    let compiled = compile(kernel(), &write(&log).unwrap());
    assert_eq!(first_problem(&compiled), (code::DIGEST_MISMATCH, Gate::Format));
    // An untitled document has no saved digest; a document with a path has one.
    let (mut log, _) = session();
    log.documents[1].saved = Some(sha256(b""));
    assert_eq!(first_problem(&compile(kernel(), &write(&log).unwrap())), (code::PERSISTENCE_FORMAT, Gate::Format));
    let (mut log, _) = session();
    log.documents[0].saved = None;
    assert_eq!(first_problem(&compile(kernel(), &write(&log).unwrap())), (code::PERSISTENCE_FORMAT, Gate::Format));
}

/// The record's text, edited and resealed, as `write` would seal it.
fn resealed(log: &Log, edit: impl FnOnce(&mut std::collections::BTreeMap<String, Value>)) -> Vec<u8> {
    let Value::Object(mut fields) = record(log).unwrap() else { panic!() };
    if let Some(Value::Object(payload)) = fields.get_mut("payload") {
        payload.remove("seal");
    }
    edit(&mut fields);
    let unsealed = Value::Object(fields.clone());
    let seal = sha256(&c600_log::wire::json_bytes(&unsealed).unwrap());
    if let Some(Value::Object(payload)) = fields.get_mut("payload") {
        payload.insert("seal".to_owned(), c600_log::wire::digest_ref("content", &seal));
    }
    c600_log::wire::json_bytes(&Value::Object(fields)).unwrap()
}

#[test]
fn the_format_gate_dispatches_on_format_and_version_and_refuses_unknown_fields() {
    let (log, _) = session();
    let gate = |bytes: &[u8]| first_problem(&compile(kernel(), bytes));
    let other = resealed(&log, |f| {
        f.insert("format".to_owned(), text("c600-log"));
    });
    assert_eq!(gate(&other), (code::PERSISTENCE_VERSION, Gate::Format));
    let later = resealed(&log, |f| {
        f.insert("version".to_owned(), Value::Int(2));
    });
    assert_eq!(gate(&later), (code::PERSISTENCE_VERSION, Gate::Format));
    let headless = resealed(&log, |f| {
        f.remove("version");
    });
    assert_eq!(gate(&headless), (code::PERSISTENCE_FORMAT, Gate::Format));
    let unknown = resealed(&log, |f| {
        f.insert("note".to_owned(), text("x"));
    });
    assert_eq!(gate(&unknown), (code::PERSISTENCE_FORMAT, Gate::Format));
    let provenance = resealed(&log, |f| {
        f.insert("provenance".to_owned(), json::object([("kind", text("x"))]));
    });
    assert_eq!(gate(&provenance), (code::PERSISTENCE_FORMAT, Gate::Format));
    let required = resealed(&log, |f| {
        let extension = json::object([("name", text("x")), ("required", Value::Bool(true)), ("value", Value::Null)]);
        f.insert("extensions".to_owned(), Value::Array(vec![extension]));
    });
    assert_eq!(gate(&required), (code::PERSISTENCE_VERSION, Gate::Format));

    // An optional extension this reader does not know is kept and written back.
    let mut extended = log.clone();
    extended.extensions = vec![Extension { name: "x.later".to_owned(), required: false, value: Value::Int(1) }];
    let compiled = compile(kernel(), &write(&extended).unwrap());
    assert!(compiled.passed(), "{:#?}", compiled.problems());
    assert_eq!(compiled.log().unwrap().extensions, extended.extensions);

    // Another spelling of the same value is refused: spaces, a missing final LF, a byte-order mark.
    let bytes = write(&log).unwrap();
    let spaced = String::from_utf8(bytes.clone()).unwrap().replacen(",", ", ", 1).into_bytes();
    assert_eq!(gate(&spaced), (code::NONCANONICAL, Gate::Format));
    assert_eq!(gate(&bytes[..bytes.len() - 1]), (code::NONCANONICAL, Gate::Format));
    let mut marked = b"\xef\xbb\xbf".to_vec();
    marked.extend_from_slice(&bytes);
    assert_eq!(gate(&marked), (code::PERSISTENCE_FORMAT, Gate::Format));
}

#[test]
fn the_histories_structure_is_checked_before_any_replay() {
    let (log, _) = session();
    let gate = |log: &Log| first_problem(&compile(kernel(), &write(log).unwrap()));
    // A commit whose parent is not where the cursor stood.
    let mut bad = log.clone();
    let MoveAction::Commit(commit) = &mut bad.moves[1].action else { panic!() };
    commit.parent = None;
    assert_eq!(gate(&bad), (code::PERSISTENCE_FORMAT, Gate::Format));
    // A cursor to a node that does not exist.
    let mut bad = log.clone();
    let MoveAction::Cursor(cursor) = &mut bad.moves[2].action else { panic!() };
    cursor.to = Some("99".to_owned());
    assert_eq!(gate(&bad), (code::PERSISTENCE_FORMAT, Gate::Format));
    // A cursor action that does not move the cursor.
    let mut bad = log.clone();
    let here = bad.moves[1].id.clone();
    let MoveAction::Cursor(cursor) = &mut bad.moves[2].action else { panic!() };
    cursor.to = Some(here);
    assert_eq!(gate(&bad), (code::PERSISTENCE_FORMAT, Gate::Format));
    // A redo that goes back: the undo from the last commit to the workspace action, recorded as a redo.
    let mut bad = log.clone();
    let MoveAction::Cursor(cursor) = &mut bad.moves[9].action else { panic!() };
    assert_eq!(cursor.action, CursorAction::Undo);
    cursor.action = CursorAction::Redo;
    assert_eq!(gate(&bad), (code::PERSISTENCE_FORMAT, Gate::Format));
    // A partial undo that keeps the whole commit.
    let mut bad = log.clone();
    let MoveAction::PartialUndo(undo) = &mut bad.moves[4].action else { panic!() };
    undo.keep = 3;
    assert_eq!(gate(&bad), (code::PERSISTENCE_FORMAT, Gate::Format));
    // A partial undo whose post state is not its kept letters' replay.
    let mut bad = log.clone();
    let MoveAction::PartialUndo(undo) = &mut bad.moves[4].action else { panic!() };
    undo.keep = 2;
    assert_eq!(gate(&bad), (code::CHECKPOINT_REPLAY, Gate::Moves));
    // A saved cursor that is not where the history ends.
    let mut bad = log.clone();
    bad.saved.moves = None;
    assert_eq!(gate(&bad), (code::PERSISTENCE_FORMAT, Gate::Format));
    // A repeated id.
    let mut bad = log.clone();
    bad.moves[1].id = bad.moves[0].id.clone();
    assert_eq!(gate(&bad), (code::PERSISTENCE_FORMAT, Gate::Format));
}

#[test]
fn a_large_log_is_stored_as_gzip_and_a_labelled_start_replays() {
    let (mut log, _) = session();
    log.documents[0].contents = "x".repeat(300_000).into_bytes();
    let bytes = write(&log).unwrap();
    assert!(bytes.starts_with(&[0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, 0, 0xff]));
    let compiled = compile(kernel(), &bytes);
    assert!(compiled.passed(), "{:#?}", compiled.problems());
    assert_eq!(compiled.log(), Some(&log));
    // A small record stored as gzip is not canonical.
    let (small, _) = session();
    let raw = write(&small).unwrap();
    let packed = c600_log::wire::gzip_stored(&raw);
    assert_eq!(first_problem(&compile(kernel(), &packed)), (code::NONCANONICAL, Gate::Format));

    // A log may start from any state, carried as its labels.
    let k = kernel();
    let (scrambled, _) = session();
    let start = compile(k, &write(&scrambled).unwrap()).state().unwrap().clone();
    assert_ne!(start.digest(), k.solved().digest());
    let mut r = Recorder::new(k);
    r.commit(&word(&[1]), "hand", "", None).unwrap();
    let mut from_scramble = r.finish();
    let MoveAction::Commit(commit) = &mut from_scramble.moves[0].action else { panic!() };
    let derived = match k.derive(&start, &k.word(&[1]).unwrap(), &c600_kernel::ProtectedSet::none()).unwrap() {
        c600_kernel::Derivation::Derived(d) => d.state,
        _ => panic!(),
    };
    commit.pre = start.digest().to_owned();
    commit.post = derived.digest().to_owned();
    from_scramble.start = Start::Labels(start.labels().to_vec());
    from_scramble.saved.state = derived.digest().to_owned();
    let bytes = write(&from_scramble).unwrap();
    assert!(bytes.starts_with(&[0x1f, 0x8b]), "the labels carrier passes 250,000 bytes");
    let compiled = compile(k, &bytes);
    assert!(compiled.passed(), "{:#?}", compiled.problems());
    assert_eq!(compiled.state().unwrap().digest(), derived.digest());
}

#[test]
fn compile_reads_bytes_and_changes_nothing() {
    let (log, _) = session();
    let bytes = write(&log).unwrap();
    let solved = kernel().solved().digest().to_owned();
    let first = compile(kernel(), &bytes);
    let second = compile(kernel(), &bytes);
    assert_eq!(first.gates(), second.gates());
    assert_eq!(first.state().map(|s| s.digest()), second.state().map(|s| s.digest()));
    assert_eq!(kernel().solved().digest(), solved);
    assert_eq!(c600_log::EXTENSION, "c600log");
    assert_eq!(c600_log::FORMAT, "c600-log-draft");
}
