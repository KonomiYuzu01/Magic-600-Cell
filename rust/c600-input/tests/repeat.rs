//! CT-EV4-04 (synthetic; EV4.6, R-18): held-key events repeat only the allowlist; a workspace append, a preview job, a
//! twist or a commit is admitted once; held-key state is released on focus loss; busy input is not queued.
//!
//! Every test sends real repeat events and counts the invocations that reach the layer by asking the layer's outcome
//! lookup for each command ID, so no leg can pass because no repeat arrived. The events are synthetic; none of this is
//! Windows input evidence, and nothing here measures time.
//!
//! CT-EV4-04 (whole: the allowlist; the once-only legs for a preview job, cancel, commit, undo, redo and a twist
//! through `twist.apply`, and a workspace append sent once by the router, which the layer answers with
//! `KN5:input.row_not_built`; the focus-loss and busy legs; synthetic events, no Windows input evidence).
//!
//! CT-BND-20 (part: a held commit chord reaches the layer once, through the router, headless; the half where a program
//! cannot obtain a token is in `c600-tools/tests/bridge.rs` and `c600-command/tests/kn5.rs`; not run: the actual
//! Windows half, on the owner's machine (C9)).

mod common;

use c600_command::{code, word_draft, Status, Table};
use c600_input::{Destination, DropReason, Focus, Key, KeyEvent, Routed, REPEAT_ALLOWLIST};
use c600_kernel::json::{self, Value};

use common::{
    code_of, derived, fixture, list, retained, rotation, row, sent, text, word, Fixture, CAPS, TOOL, TOOL_KEY,
};

const REPEATS: usize = 20;

/// Holds `chord`: one key-down, [`REPEATS`] key-downs flagged as auto-repeat, three more key-downs the platform did not
/// flag while the key is still down, then the key-up. Every repeat must be dropped, and exactly one invocation must
/// reach the layer. Returns the key-down's routing.
fn hold_once(f: &mut Fixture, chord: &str) -> Routed {
    let before = f.reached();
    let first = f.down(chord);
    for _ in 0..REPEATS {
        let repeat = f.key(KeyEvent::repeat(chord));
        assert!(matches!(repeat, Routed::Dropped(DropReason::NotRepeatable(Some(_)))), "{chord}: {repeat:?}");
    }
    for _ in 0..3 {
        let unflagged = f.down(chord);
        assert!(matches!(unflagged, Routed::Dropped(DropReason::NotRepeatable(Some(_)))), "{chord}: {unflagged:?}");
    }
    f.up(chord);
    assert_eq!(f.reached(), before + 1, "{chord} reached the layer more than once");
    first
}

fn status(routed: &Routed) -> &Status {
    &routed.receipt().unwrap_or_else(|| panic!("nothing reached the layer: {routed:?}")).status
}

/// CT-EV4-04, the allowlist (EV4.6; DT-M1 K4, fix-first item 2): the router reads each row's H-08 `repeat` flag, and
/// the stored table sets it on exactly the camera, list-movement and preview-scrub rows.
#[test]
fn the_stored_table_repeats_exactly_the_allowlist() {
    let table = Table::h08();
    let mut repeating: Vec<&str> = table.rows().filter(|row| row.repeat).map(|row| row.id.as_str()).collect();
    repeating.sort_unstable();
    let mut allowlist = REPEAT_ALLOWLIST.to_vec();
    allowlist.sort_unstable();
    assert_eq!(repeating, allowlist);
}

/// CT-EV4-04, the allowlist leg and the control against a vacuous pass: real repeat events of the camera, zoom,
/// list-movement and preview-scrub keys each reach the layer, whether the platform flags them or the router infers
/// them from a held key, and each repeat resolves its relative names at its own event. The layer refuses these view
/// rows as not built yet; the routing half is what this shows.
#[test]
fn held_camera_list_and_scrub_keys_repeat_and_each_repeat_resolves_at_its_own_event() {
    const N: i64 = 5;
    let mut f = fixture("key", "us-qwerty");
    assert_eq!(row(&f.down("ArrowRight")), "camera.rotate");
    for _ in 0..N {
        assert_eq!(row(&f.key(KeyEvent::repeat("ArrowRight"))), "camera.rotate");
    }
    for _ in 0..N {
        assert_eq!(row(&f.down("ArrowRight")), "camera.rotate", "an unflagged key-down of a held key repeats");
    }
    f.up("ArrowRight");
    assert_eq!(f.reached(), 1 + 2 * N as usize);

    assert_eq!(row(&f.down("Equal")), "camera.zoom");
    for _ in 0..N {
        assert_eq!(row(&f.key(KeyEvent::repeat("Equal"))), "camera.zoom");
    }
    f.up("Equal");

    // The view moves the selection between repeats, as `list.move` will once it is built.
    f.router.set_focus(Focus::List(list(10, 0)));
    let entry = |routed: &Routed| sent(routed).1.get("entry").cloned().unwrap();
    let mut entries = vec![entry(&f.down("ArrowDown"))];
    for _ in 0..N {
        f.router.list_mut().unwrap().selected += 1;
        entries.push(entry(&f.key(KeyEvent::repeat("ArrowDown"))));
    }
    f.up("ArrowDown");
    assert_eq!(entries, (1..=N + 1).map(|i| text(&format!("e{i}"))).collect::<Vec<_>>());

    // Scrubbing the presented preview, with the scrub position moving between repeats.
    f.router.set_focus(Focus::Puzzle);
    let id = f.preview(&[1, 2, 3]);
    let prefix = |routed: &Routed| sent(routed).1.get("prefix").cloned().unwrap();
    let mut prefixes = vec![prefix(&f.down("Alt+ArrowRight"))];
    for _ in 0..N {
        f.router.context.scrub += 1;
        prefixes.push(prefix(&f.key(KeyEvent::repeat("Alt+ArrowRight"))));
    }
    f.up("Alt+ArrowRight");
    assert_eq!(prefixes, (1..=N + 1).map(Value::Int).collect::<Vec<_>>());

    let repeated: Vec<_> = f.router.log().iter().filter(|d| d.binding.command != "operation.preview").collect();
    assert_eq!(repeated.len(), (2 * N + 1 + 3 * (N + 1)) as usize);
    for dispatch in repeated {
        assert!(REPEAT_ALLOWLIST.contains(&dispatch.binding.command.as_str()));
        assert_eq!(code_of(&dispatch.receipt), code::ROW_NOT_BUILT);
        if dispatch.binding.command == "view.preview.scrub" {
            assert_eq!(dispatch.invocation.args.get("preview"), Some(&Value::Int(id as i64)));
        }
    }
    assert_eq!(f.reached(), f.router.log().len());
}

/// CT-EV4-04, the once-only legs (EV4.6; R-18; KN5.14, KN5.17): a held preview, cancel, commit, undo, redo, grip,
/// twist, workspace-append, toggle, prefix-group, Escape or tool-command key is admitted once, however many real repeat
/// events arrive, flagged by the platform or not.
///
/// Built legs: the preview leg queues one job; the cancel, commit, undo and redo legs settle once in the layer (one
/// journal transaction each for commit, undo and redo); each twist key commits once through `twist.apply` against the
/// pinned menu (one journal transaction each; the twist leg below shows the state it reaches); the tool-command leg
/// queues one program job. Routing-only legs: the grip, workspace-append (a twist key in the draft destination), toggle
/// and Escape legs reach the layer once, and the layer refuses those rows as `KN5:input.row_not_built`.
#[test]
fn held_state_changing_keys_are_admitted_once_however_many_repeats_arrive() {
    let mut f = fixture("key", "us-qwerty");
    f.pin_menu();
    f.router.context.grip = Some(0);
    f.router.context.phase = Some(text("phase-1"));

    f.shell.draft = Some(word_draft(&word(&[1])));
    let preview = hold_once(&mut f, "Ctrl+Enter");
    assert!(matches!(status(&preview), Status::Running(_)));
    f.build_and_present(preview.receipt().unwrap());
    assert_eq!(f.layer.run_jobs(), 0, "the repeats queued no job");

    let cancel = hold_once(&mut f, "Ctrl+Shift+Backspace");
    assert!(matches!(status(&cancel), Status::Completed(_)));
    assert!(f.shell.presented().is_none());

    let preview = hold_once(&mut f, "Ctrl+Enter");
    f.build_and_present(preview.receipt().unwrap());
    let commit = hold_once(&mut f, "Ctrl+Shift+Enter");
    assert!(matches!(status(&commit), Status::Committed(_)));
    assert_eq!(f.layer.store().log.len(), 1);

    assert!(matches!(status(&hold_once(&mut f, "Ctrl+KeyZ")), Status::Committed(_)));
    assert_eq!(f.layer.store().log.len(), 2);
    assert!(matches!(status(&hold_once(&mut f, "Ctrl+KeyY")), Status::Committed(_)));
    assert_eq!(f.layer.store().log.len(), 3);

    let grip = hold_once(&mut f, "KeyQ");
    assert_eq!((row(&grip), code_of(grip.receipt().unwrap())), ("grip.select".into(), code::ROW_NOT_BUILT));
    let twist = hold_once(&mut f, "KeyJ");
    assert_eq!(row(&twist), "twist.apply");
    assert!(matches!(status(&twist), Status::Committed(_)), "{:?}", status(&twist));
    assert_eq!(f.layer.store().log.len(), 4);
    let inverse = hold_once(&mut f, "Shift+KeyK");
    assert_eq!(row(&inverse), "twist.apply");
    assert!(matches!(status(&inverse), Status::Committed(_)), "{:?}", status(&inverse));
    assert_eq!(f.layer.store().log.len(), 5);
    f.router.context.destination = Destination::Draft;
    let append = hold_once(&mut f, "KeyJ");
    assert_eq!(row(&append), "operation.phase.append_twist");
    assert_eq!(code_of(append.receipt().unwrap()), code::ROW_NOT_BUILT);
    f.router.context.destination = Destination::Live;

    assert_eq!(row(&hold_once(&mut f, "F9")), "keyboard.toggle");
    assert!(matches!(f.tap("Ctrl+KeyG"), Routed::Prefix));
    assert_eq!(row(&hold_once(&mut f, "KeyF")), "display.frame.toggle");
    assert_eq!(row(&hold_once(&mut f, "Escape")), "transient.close");

    let tool = hold_once(&mut f, TOOL_KEY);
    assert_eq!(row(&tool), TOOL);
    assert!(matches!(status(&tool), Status::Running(_)));
    assert!(f.layer.take_program_job().is_some());
    assert!(f.layer.take_program_job().is_none(), "the repeats queued no program job");

    assert_eq!(f.layer.store().log.len(), 5);
    assert_eq!(f.layer.run_jobs(), 0);
    // Fourteen held keys, one invocation each; the prefix key `Ctrl+KeyG` reached nothing.
    assert_eq!(f.reached(), 14);
}

/// CT-EV4-04, the focus-loss leg (EV4.6; DT-M1 K4's lost key-up): focus loss releases every held key, so repeat events
/// that arrive after it run nothing until a fresh key-down; a key whose key-up was lost presses afresh; a text
/// field's held key and a pending prefix group are released too.
#[test]
fn held_key_state_is_released_on_focus_loss() {
    let mut f = fixture("key", "us-qwerty");
    assert_eq!(row(&f.down("ArrowRight")), "camera.rotate");
    for _ in 0..2 {
        assert_eq!(row(&f.key(KeyEvent::repeat("ArrowRight"))), "camera.rotate");
    }
    assert_eq!(f.router.held(), [Key::parse("ArrowRight").unwrap()]);
    assert!(matches!(f.key(KeyEvent::FocusLost), Routed::Released));
    assert!(f.router.held().is_empty());
    for _ in 0..REPEATS {
        assert!(matches!(f.key(KeyEvent::repeat("ArrowRight")), Routed::Dropped(DropReason::NotHeld)));
    }
    assert_eq!(f.reached(), 3);
    assert_eq!(row(&f.down("ArrowRight")), "camera.rotate", "a fresh key-down acts again");
    f.up("ArrowRight");
    assert_eq!(f.reached(), 4);

    // A preview key whose key-up is lost with the focus: while held, an unflagged key-down is a repeat and is dropped;
    // after the focus loss the next key-down is a fresh press.
    f.shell.draft = Some(word_draft(&word(&[1])));
    assert!(matches!(status(&f.down("Ctrl+Enter")), Status::Running(_)));
    assert!(matches!(f.down("Ctrl+Enter"), Routed::Dropped(DropReason::NotRepeatable(_))));
    assert_eq!(f.layer.run_jobs(), 1);
    f.key(KeyEvent::FocusLost);
    assert!(matches!(f.key(KeyEvent::repeat("Ctrl+Enter")), Routed::Dropped(DropReason::NotHeld)));
    assert!(matches!(status(&f.down("Ctrl+Enter")), Status::Running(_)));
    assert_eq!(f.layer.run_jobs(), 1);
    assert_eq!(f.reached(), 6);

    // A text field's held key.
    f.router.set_focus(Focus::Text);
    assert!(matches!(f.key(KeyEvent::typed("KeyJ", 'j')), Routed::Text));
    assert!(matches!(f.key(KeyEvent::repeat("KeyJ")), Routed::Text));
    f.key(KeyEvent::FocusLost);
    assert!(matches!(f.key(KeyEvent::repeat("KeyJ")), Routed::Dropped(DropReason::NotHeld)));

    // A pending prefix group.
    f.router.set_focus(Focus::Puzzle);
    assert!(matches!(f.tap("Ctrl+KeyG"), Routed::Prefix));
    f.key(KeyEvent::FocusLost);
    assert!(f.router.pending().is_empty());
    assert!(matches!(f.tap("KeyF"), Routed::Unhandled));
    assert_eq!(f.reached(), 6);
}

/// CT-EV4-04, the busy leg (EV4.6; DT-M1 Q6 (a); KN5.17): a key-down that meets a busy lane gets the layer's refusal
/// as its receipt and is never queued; when the lane frees, nothing runs on its behalf, and only a fresh press acts.
///
/// The commit lane is held with the layer's public test hook `CommandLayer::hold_commit_lane`, which stands in for an
/// unsettled commit of another invocation: a headless commit settles inside its own `invoke`, so no other commit can
/// be in flight without it. No `c600-command` change is needed for this leg.
#[test]
fn busy_input_is_refused_and_never_queued() {
    let mut f = fixture("key", "us-qwerty");
    f.commit(&[1]);
    f.preview(&[2]);
    f.layer.hold_commit_lane(true);
    let before = f.reached();
    let commit = f.down("Ctrl+Shift+Enter");
    assert_eq!(code_of(commit.receipt().unwrap()), code::COMMIT_IN_FLIGHT);
    for _ in 0..REPEATS {
        assert!(matches!(f.key(KeyEvent::repeat("Ctrl+Shift+Enter")), Routed::Dropped(_)));
    }
    f.up("Ctrl+Shift+Enter");
    let undo = f.tap("Ctrl+KeyZ");
    assert_eq!((row(&undo), code_of(undo.receipt().unwrap())), ("session.undo".into(), code::COMMIT_IN_FLIGHT));
    assert_eq!(f.reached(), before + 2);

    f.layer.hold_commit_lane(false);
    assert_eq!(f.layer.run_jobs(), 0);
    assert_eq!(f.layer.store().log.len(), 1, "the refused commit and undo did not run when the lane freed");
    assert_eq!(f.reached(), before + 2);
    let refused = commit.receipt().unwrap();
    assert_eq!(code_of(&f.layer.outcome(&refused.command, &refused.request).unwrap()), code::COMMIT_IN_FLIGHT);
    assert!(matches!(status(&f.tap("Ctrl+Shift+Enter")), Status::Committed(_)), "a fresh press commits");
    assert_eq!(f.layer.store().log.len(), 2);

    // The exclusive lane: a second preview press while one preview job is queued is refused, and one job runs.
    f.shell.draft = Some(word_draft(&word(&[3])));
    assert!(matches!(status(&f.tap("Ctrl+Enter")), Status::Running(_)));
    assert_eq!(code_of(f.tap("Ctrl+Enter").receipt().unwrap()), code::LANE_FULL);
    assert_eq!(f.layer.run_jobs(), 1);
    assert_eq!(f.layer.run_jobs(), 0);

    // The program lane: a second tool press while the first job holds the lane is refused; the host gets one job.
    assert!(matches!(status(&f.tap(TOOL_KEY)), Status::Running(_)));
    assert_eq!(code_of(f.tap(TOOL_KEY).receipt().unwrap()), code::LANE_FULL);
    assert!(f.layer.take_program_job().is_some());
    assert!(f.layer.take_program_job().is_none());
}

/// CT-EV4-04, the twist leg (EV4.6; R-18; DT-M1 K4 and Q6 (a); KN3.7, KN5.17): a held twist key is admitted once, its
/// repeat events are not admitted, and busy input is not queued. The key-down reaches the command layer through
/// `twist.apply` with the grip's cap and KN3.7's exact request of the key's item of the pinned I_a menu, and commits
/// the item's retained word once, with no preview and no job; no repeat, flagged by the platform or not, reaches the
/// layer. A twist key-down that meets a held commit lane gets `KN5:busy.commit_in_flight` as its receipt; when the lane
/// frees, nothing commits on its behalf, and only a fresh press commits, once.
#[test]
fn a_held_twist_key_commits_once_and_a_busy_twist_is_not_queued() {
    let mut f = fixture("key", "us-qwerty");
    f.pin_menu();
    f.router.context.grip = Some(1);
    let start = f.layer.snapshot().clone();
    let once = derived(&start, &retained(CAPS[1], 2));

    let twist = hold_once(&mut f, "KeyK");
    let args = json::object([("cap", Value::Int(i64::from(CAPS[1]))), ("rotation", rotation(2))]);
    assert_eq!(sent(&twist), ("twist.apply".to_owned(), args));
    assert!(matches!(status(&twist), Status::Committed(_)), "{:?}", status(&twist));
    assert_eq!(f.layer.snapshot().digest(), once.digest(), "the item's retained word, once");
    assert_eq!(f.layer.store().log.len(), 1);
    assert_eq!(f.layer.store().log[0].recipe, word(&retained(CAPS[1], 2)));
    assert!(f.layer.preview().is_none());
    assert_eq!(f.layer.run_jobs(), 0, "a live twist queues no job");

    // The commit lane is held, as by an unsettled commit of another invocation (see the busy leg).
    f.layer.hold_commit_lane(true);
    let before = f.reached();
    let busy = f.down("KeyK");
    assert_eq!((row(&busy), code_of(busy.receipt().unwrap())), ("twist.apply".into(), code::COMMIT_IN_FLIGHT));
    for _ in 0..REPEATS {
        let repeat = f.key(KeyEvent::repeat("KeyK"));
        assert!(matches!(repeat, Routed::Dropped(DropReason::NotRepeatable(Some(_)))), "{repeat:?}");
    }
    f.up("KeyK");
    assert_eq!(f.reached(), before + 1);
    f.layer.hold_commit_lane(false);
    assert_eq!(f.layer.run_jobs(), 0);
    assert_eq!(f.reached(), before + 1);
    assert_eq!(f.layer.store().log.len(), 1, "the refused twist did not commit when the lane freed");
    assert_eq!(f.layer.snapshot().digest(), once.digest());
    let refused = busy.receipt().unwrap();
    assert_eq!(code_of(&f.layer.outcome(&refused.command, &refused.request).unwrap()), code::COMMIT_IN_FLIGHT);

    let again = hold_once(&mut f, "KeyK");
    assert!(matches!(status(&again), Status::Committed(_)), "a fresh press commits: {:?}", status(&again));
    assert_eq!(f.layer.snapshot().digest(), derived(&once, &retained(CAPS[1], 2)).digest());
    assert_eq!(f.layer.store().log.len(), 2);
}
