//! CT-EV4-02 (synthetic; EV4.3, EV4.4): key capture, text, list navigation and global dispatch run in the fixed order,
//! and resolved absolute arguments survive later camera, focus, hover and filter changes. Every key event here is
//! synthetic; none of this is Windows input evidence.
//!
//! CT-EV4-02 (whole: the routing order, the text, list, global, argument and route legs, `resolve_binding` and the
//! prefix group, on the draft keymap; synthetic events, no Windows input evidence).

mod common;

use c600_command::{code, digest, draft_args, word_draft, CancelReason, Receipt, Status, GRIP_REBIND_ROW};
use c600_input::{
    default_specs, resolve_binding, Chord, Destination, Focus, KeyEvent, KeyMap, Layout, Resolution, Routed,
    RoutingClass, RoutingContext, LIST_ROWS,
};
use c600_kernel::json::{self, Value};

use common::{code_of, fixture, list, pin, rotation, row, sent, text, word, TOOL, TOOL_KEY};

fn chords(texts: &[&str]) -> Vec<Chord> {
    texts.iter().map(|t| Chord::parse(t).unwrap()).collect()
}

/// CT-EV4-02, the routing order (EV4.4): the same chord goes to the key-capture field, then the text field, then the
/// focused list, then the global map, according to what has the focus.
#[test]
fn routing_runs_capture_then_text_then_list_then_global() {
    let mut f = fixture("key", "us-qwerty");
    // ArrowDown: captured, an editing key in text, list movement in a list, the camera in the global map.
    f.router.set_focus(Focus::Capture);
    assert!(matches!(f.tap("ArrowDown"), Routed::Captured(c) if c == Chord::parse("ArrowDown").unwrap()));
    f.router.set_focus(Focus::Text);
    assert!(matches!(f.tap("ArrowDown"), Routed::Text));
    assert_eq!(f.reached(), 0);
    f.router.set_focus(Focus::List(list(4, 1)));
    let routed = f.tap("ArrowDown");
    assert_eq!(sent(&routed), ("list.move".into(), json::object([("list", text("repairs")), ("entry", text("e2"))])));
    assert_eq!(routed.dispatch().unwrap().binding.context, RoutingClass::List);
    f.router.set_focus(Focus::Puzzle);
    let routed = f.tap("ArrowDown");
    assert_eq!(row(&routed), "camera.rotate");
    assert_eq!(routed.dispatch().unwrap().binding.context, RoutingClass::Global);
    assert_eq!(f.reached(), 2);

    // KeyQ: captured, typed, then the global grip key both from a list (not a navigation key) and from the puzzle.
    f.router.set_focus(Focus::Capture);
    assert!(matches!(f.tap("KeyQ"), Routed::Captured(_)));
    f.router.set_focus(Focus::Text);
    assert!(matches!(f.key(KeyEvent::typed("KeyQ", 'q')), Routed::Text));
    f.up("KeyQ");
    let grip = json::object([("cap", text("c0")), ("frame", text("f0"))]);
    f.router.set_focus(Focus::List(list(4, 1)));
    assert_eq!(sent(&f.tap("KeyQ")), ("grip.select".into(), grip.clone()));
    f.router.set_focus(Focus::Puzzle);
    assert_eq!(sent(&f.tap("KeyQ")), ("grip.select".into(), grip));

    // Enter: captured, a new line in text, inspection in a list, and nothing in the global map, so Enter never
    // commits from there (DT-M1 K4, C4).
    f.router.set_focus(Focus::Capture);
    assert!(matches!(f.tap("Enter"), Routed::Captured(_)));
    f.router.set_focus(Focus::Text);
    assert!(matches!(f.tap("Enter"), Routed::Text));
    f.router.set_focus(Focus::List(list(4, 1)));
    assert_eq!(sent(&f.tap("Enter")), ("object.inspect".into(), json::object([("object", text("p1"))])));
    f.router.set_focus(Focus::Puzzle);
    assert!(matches!(f.tap("Enter"), Routed::Unhandled));
    assert_eq!(f.reached(), 5);
}

/// CT-EV4-02, the text leg (EV4.4): a character typed into a text field, IME composition, the clipboard keys and text
/// undo stay with the editor in both destinations and under both declared layouts, so none reaches a puzzle or
/// workspace row; a key that is not text still reaches the global map from the same field.
#[test]
fn a_character_typed_into_text_input_never_reaches_a_puzzle_or_workspace_row() {
    for layout in ["us-qwerty", "fr-azerty"] {
        let mut f = fixture("key", layout);
        f.router.context.grip = Some(0);
        f.router.context.phase = Some(text("phase-1"));
        f.preview(&[1]);
        let before = f.reached();
        f.router.set_focus(Focus::Text);
        for destination in [Destination::Live, Destination::Draft] {
            f.router.context.destination = destination;
            // Grip and twist keys, with the character the layout types, without any (a dead key), with Shift, and
            // held with real repeat events.
            let typed = [
                KeyEvent::typed("KeyQ", if layout == "us-qwerty" { 'q' } else { 'a' }),
                KeyEvent::typed("KeyJ", 'j'),
                KeyEvent::typed("Shift+KeyJ", 'J'),
                KeyEvent::down("KeyW"),
                KeyEvent::typed("Digit1", if layout == "us-qwerty" { '1' } else { '&' }),
                KeyEvent::typed("Ctrl+Alt+Digit0", '@'),
                KeyEvent::down("Space"),
                KeyEvent::down("Backspace"),
            ];
            for event in typed {
                let KeyEvent::Down { chord, .. } = event.clone() else { unreachable!() };
                assert!(matches!(f.key(event), Routed::Text), "{chord} reached past the text field");
                for _ in 0..3 {
                    assert!(matches!(f.key(KeyEvent::repeat(&chord.to_string())), Routed::Text));
                }
                f.key(KeyEvent::Up(chord.key));
            }
            assert!(matches!(f.key(KeyEvent::Compose("漢字".into())), Routed::Text));
            // The clipboard keys, select-all and text undo and redo, by the character the active layout gives them:
            // on AZERTY, text undo is the key that types z.
            let office = match layout {
                "us-qwerty" => {
                    ["Ctrl+KeyZ", "Ctrl+Shift+KeyZ", "Ctrl+KeyY", "Ctrl+KeyC", "Ctrl+KeyX", "Ctrl+KeyV", "Ctrl+KeyA"]
                }
                _ => ["Ctrl+KeyW", "Ctrl+Shift+KeyW", "Ctrl+KeyY", "Ctrl+KeyC", "Ctrl+KeyX", "Ctrl+KeyV", "Ctrl+KeyQ"],
            };
            for chord in office {
                assert!(matches!(f.tap(chord), Routed::Text), "{chord} left the text field under {layout}");
            }
            for chord in ["Shift+Insert", "Ctrl+Insert", "Shift+Delete", "Ctrl+ArrowLeft", "Shift+Home"] {
                assert!(matches!(f.tap(chord), Routed::Text), "{chord} left the text field");
            }
            // DT-M1 C4: the commit works from any focus outside a text field, so the field keeps the commit chord.
            assert!(matches!(f.tap("Ctrl+Shift+Enter"), Routed::Text));
        }
        assert_eq!(f.reached(), before, "nothing typed reached the layer under {layout}");
        assert!(f.router.log().iter().all(|d| d.binding.command == "operation.preview"));
        assert_eq!(f.layer.store().log.len(), 0);
        // A key that is not text goes on to the global map from the same field.
        assert_eq!(row(&f.tap("F9")), "keyboard.toggle");
        assert_eq!(f.reached(), before + 1);
    }
}

/// CT-EV4-02, the list leg (EV4.4; DT-M1 fix-first item 2): a focused list takes the arrows, Enter, Tab and the number
/// keys, and nothing else; what it takes reaches only navigation, inspection and O9's row copy.
#[test]
fn a_focused_list_takes_arrows_enter_tab_and_number_keys_and_nothing_else() {
    let mut f = fixture("key", "us-qwerty");
    let repairs = list(5, 2);
    f.router.set_focus(Focus::List(repairs.clone()));
    let entry = |id: &str| json::object([("list", text("repairs")), ("entry", text(id))]);
    for (chord, id) in [("ArrowUp", "e1"), ("ArrowLeft", "e1"), ("ArrowDown", "e3"), ("ArrowRight", "e3")] {
        assert_eq!(sent(&f.tap(chord)), ("list.move".into(), entry(id)), "{chord}");
    }
    assert_eq!(sent(&f.tap("Enter")), ("object.inspect".into(), json::object([("object", text("p2"))])));
    for (chord, piece) in [("Digit1", "p0"), ("Digit2", "p1"), ("Digit5", "p4")] {
        let args = json::object([("result", repairs.result.clone()), ("object", text(piece))]);
        assert_eq!(sent(&f.tap(chord)), ("list.row.copy".into(), args), "{chord}");
    }
    // Row 10 does not exist in a five-row list: the list still takes the key, and the press stops at the shell.
    match f.tap("Digit0") {
        Routed::Refused { row, error } => {
            assert_eq!((row.as_str(), error.code), ("list.row.copy", code::ARGUMENT_FORM))
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(f.tap("Tab"), Routed::Focus));
    assert!(matches!(f.tap("Shift+Tab"), Routed::Focus));
    let taken = f.reached();
    assert_eq!(taken, 8);
    assert!(f
        .router
        .log()
        .iter()
        .all(|d| d.binding.context == RoutingClass::List && LIST_ROWS.contains(&d.binding.command.as_str())));

    // Every other key passes the list to the global map: grip, toggle, scrub, the commit chord, Escape, and modified
    // arrows and digits, which are not the list's navigation keys.
    assert_eq!(row(&f.tap("KeyQ")), "grip.select");
    assert_eq!(row(&f.tap("F9")), "keyboard.toggle");
    assert_eq!(row(&f.tap("Escape")), "transient.close");
    match f.tap("Alt+ArrowRight") {
        Routed::Refused { row, error } => {
            assert_eq!((row.as_str(), error.code), ("view.preview.scrub", code::NO_FRESH_PREVIEW))
        }
        other => panic!("{other:?}"),
    }
    match f.tap("Ctrl+Shift+Enter") {
        Routed::Refused { row, error } => {
            assert_eq!((row.as_str(), error.code), ("operation.execute", code::NO_FRESH_PREVIEW))
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(f.tap("Shift+ArrowDown"), Routed::Unhandled));
    assert!(matches!(f.tap("Ctrl+Digit1"), Routed::Unhandled));
    assert_eq!(f.reached(), taken + 3);
    assert!(f.router.log()[taken..].iter().all(|d| d.binding.context == RoutingClass::Global));
}

/// CT-EV4-02, the global leg (EV4.4): a global key reaches its row whether the puzzle, a text field or a list has the
/// focus. A key-capture field comes first in the order and records the chord instead; that is its purpose.
#[test]
fn a_global_key_reaches_its_row_whatever_has_focus() {
    let mut f = fixture("key", "us-qwerty");
    f.shell.draft = Some(word_draft(&word(&[1])));
    for focus in [Focus::Puzzle, Focus::Text, Focus::List(list(3, 0))] {
        f.router.set_focus(focus.clone());
        assert_eq!(row(&f.tap("F9")), "keyboard.toggle", "{focus:?}");
        assert_eq!(row(&f.tap("Escape")), "transient.close", "{focus:?}");
        let preview = f.tap("Ctrl+Enter");
        assert_eq!(sent(&preview), ("operation.preview".into(), draft_args(&word(&[1]))), "{focus:?}");
        assert!(matches!(preview.receipt().unwrap().status, Status::Running(_)));
        assert_eq!(f.layer.run_jobs(), 1);
        assert_eq!(row(&f.tap(TOOL_KEY)), TOOL, "{focus:?}");
        let job = f.layer.take_program_job().expect("the tool row's job").job;
        f.layer.finish_program_job(job, Ok(Value::Null));
    }
    assert_eq!(f.reached(), 12);
    f.router.set_focus(Focus::Capture);
    assert!(matches!(f.tap("F9"), Routed::Captured(_)));
    assert_eq!(f.reached(), 12);
}

/// CT-EV4-02, the argument leg (EV4.3, EV4.4, EV4.8): relative names (the current grip, a slot, a twist key's menu
/// item, a list row, the editor's draft) are resolved at the press. A later camera, focus, hover or filter change
/// neither reinterprets an invocation already sent nor changes what the same global key sends next; only the rebind
/// command changes the applied slot map, and a list row number names the row shown at its own press.
#[test]
fn resolved_arguments_survive_later_camera_focus_hover_and_filter_changes() {
    let mut f = fixture("key", "us-qwerty");
    f.router.context.grip = Some(1);
    f.router.context.menu = Some(pin());
    let grip = f.tap("KeyW");
    let twist = f.tap("KeyJ");
    assert_eq!(sent(&grip).1, json::object([("cap", text("c1")), ("frame", text("f1"))]));
    assert_eq!(sent(&twist).1, json::object([("cap", text("c1")), ("rotation", rotation(1))]));
    let first = f.router.log().to_vec();

    f.router.context.camera = json::object([("yaw", Value::Int(40))]);
    f.router.context.hover = Some(text("piece-17"));
    f.router.context.filter = Some(text("orbit-3"));
    f.router.set_focus(Focus::List(list(3, 0)));
    f.router.set_focus(Focus::Text);
    f.router.set_focus(Focus::Puzzle);
    for (dispatch, before) in f.router.log().iter().zip(&first) {
        assert_eq!(dispatch.invocation.args, before.invocation.args);
        let stored = f.layer.outcome(&dispatch.receipt.command, &dispatch.receipt.request).unwrap();
        assert_eq!(stored, dispatch.receipt, "the layer holds the envelope as sent");
    }
    assert_eq!(sent(&f.tap("KeyW")), sent(&grip), "the camera, focus, hover and filter derive no new slot map");
    assert_eq!(sent(&f.tap("KeyJ")), sent(&twist));

    // The rebind command alone changes the applied map; what was sent keeps the old caps.
    let map = (0..4).map(|n| json::object([("cap", Value::Int(100 + n)), ("frame", text(&format!("rf{n}")))]));
    let expected = Value::String(f.layer.slot_map_digest());
    let args =
        json::object([("anchor", Value::Int(5)), ("map", Value::Array(map.collect())), ("expected_map", expected)]);
    let rebound = f.shell.invoke(&mut f.layer, GRIP_REBIND_ROW, args).unwrap();
    assert!(matches!(rebound.status, Status::Completed(_)), "{:?}", rebound.status);
    assert_eq!(sent(&f.tap("KeyW")).1, json::object([("cap", Value::Int(101)), ("frame", text("rf1"))]));
    assert_eq!(sent(&f.tap("KeyJ")).1, json::object([("cap", Value::Int(101)), ("rotation", rotation(1))]));
    assert_eq!(f.router.log()[0].invocation.args, first[0].invocation.args);
    assert_eq!(f.router.log()[1].invocation.args, first[1].invocation.args);

    // A list row number names the row shown at its press; a later filter that reorders the list does not change it.
    f.router.set_focus(Focus::List(list(3, 0)));
    let copied = f.tap("Digit2");
    assert_eq!(sent(&copied).1.get("object"), Some(&text("p1")));
    f.router.list_mut().unwrap().entries.reverse();
    f.router.context.filter = Some(text("orbit-5"));
    assert_eq!(f.router.log().last().unwrap().invocation.args, sent(&copied).1);
    assert_eq!(sent(&f.tap("Digit2")).1.get("object"), Some(&text("p1")), "the middle row of three stays p1");
    f.router.list_mut().unwrap().entries.rotate_left(1);
    assert_eq!(sent(&f.tap("Digit2")).1.get("object"), Some(&text("p0")));

    // The draft a preview press resolved is the one the preview is built from, whatever the editor holds later.
    f.router.set_focus(Focus::Puzzle);
    f.shell.draft = Some(word_draft(&word(&[1])));
    let preview = f.tap("Ctrl+Enter");
    f.shell.draft = Some(word_draft(&word(&[2, 2])));
    f.router.context.camera = Value::Null;
    f.build_and_present(preview.receipt().unwrap());
    assert_eq!(f.layer.preview().unwrap().deps.draft, digest(&word_draft(&word(&[1]))));
}

/// CT-EV4-02, the route leg (EV4.3, EV4.5): the key route and the command route (`Shell::invoke` with the same row and
/// absolute arguments) give receipts that agree on everything except the command ID and the request digest. Both
/// routes send on the shell's channel, so the layer files both receipts in the shell's command-ID space (ruling 4).
#[test]
fn the_key_route_and_the_command_route_give_the_same_receipts() {
    let mut keys = fixture("key", "us-qwerty");
    let mut cmds = fixture("cmd", "us-qwerty");
    keys.pin_menu();
    cmds.pin_menu();
    keys.router.context.grip = Some(2);
    keys.shell.draft = Some(word_draft(&word(&[1, 3])));
    let mut pairs: Vec<(Receipt, Receipt)> = Vec::new();
    for chord in ["Ctrl+Enter", "Ctrl+Shift+Enter", "Ctrl+KeyZ", "Ctrl+KeyY", "F9", "KeyJ", TOOL_KEY] {
        if chord == "Ctrl+Shift+Enter" {
            // The preview job runs on both layers; its settled receipts agree as well.
            let (key, cmd) = pairs[0].clone();
            let id = keys.build_and_present(&key);
            assert_eq!(cmds.layer.run_jobs(), 1);
            cmds.frames.present(&mut cmds.shell, &mut cmds.layer, id).unwrap();
            let key = keys.layer.outcome(&key.command, &key.request).unwrap();
            let cmd = cmds.layer.outcome(&cmd.command, &cmd.request).unwrap();
            assert!(matches!(key.status, Status::Completed(_)));
            assert_eq!((key.job, key.status), (cmd.job, cmd.status));
        }
        let routed = keys.tap(chord);
        let dispatch = routed.dispatch().unwrap_or_else(|| panic!("{chord}: {routed:?}")).clone();
        let invocation = &dispatch.invocation;
        let filed = keys.layer.outcome(&dispatch.receipt.command, &dispatch.receipt.request);
        assert_eq!(filed.as_ref(), Some(&dispatch.receipt), "{chord}: the shell's lookup answers the key route");
        let receipt = cmds.shell.invoke(&mut cmds.layer, &invocation.row, invocation.args.clone()).unwrap();
        assert_eq!(dispatch.receipt.job, receipt.job, "{chord}");
        assert_eq!(dispatch.receipt.status, receipt.status, "{chord}");
        assert_ne!(dispatch.receipt.command, receipt.command);
        assert_ne!(dispatch.receipt.request, receipt.request);
        pairs.push((dispatch.receipt, receipt));
    }
    let statuses: Vec<&str> = pairs.iter().map(|(key, _)| key.status.name()).collect();
    assert_eq!(statuses, ["running", "committed", "committed", "committed", "refused", "committed", "running"]);
    assert_eq!(code_of(&pairs[4].0), code::ROW_NOT_BUILT);
    // The journals agree on every field but the command ID and the request digest.
    let journal = |f: &common::Fixture| {
        let log = &f.layer.store().log;
        log.iter()
            .map(|t| (t.kind, t.event.clone(), t.order, t.target, t.row.clone(), t.literal.clone(), t.post.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(journal(&keys).len(), 4);
    assert_eq!(journal(&keys), journal(&cmds));
}

/// CT-EV4-02, EV4.3's resolution: `resolve_binding` is pure and answers only in the binding's routing class, for its
/// destination, and under the layout a character binding was resolved for.
#[test]
fn resolve_binding_is_pure_and_answers_in_its_class_destination_and_layout() {
    let map = KeyMap::default_for("us-qwerty").unwrap();
    let bindings = map.bindings().to_vec();
    let at = |class, destination| RoutingContext { class, destination };
    let live = Destination::Live;
    let command = |r: Resolution| match r {
        Resolution::Bound(b) => b.command.clone(),
        other => format!("{other:?}"),
    };
    let us = "us-qwerty";
    for _ in 0..2 {
        assert_eq!(
            command(resolve_binding(&chords(&["ArrowDown"]), at(RoutingClass::List, live), &bindings, us)),
            "list.move"
        );
        assert_eq!(
            command(resolve_binding(&chords(&["ArrowDown"]), at(RoutingClass::Global, live), &bindings, us)),
            "camera.rotate"
        );
        assert_eq!(
            command(resolve_binding(&chords(&["KeyQ"]), at(RoutingClass::Text, live), &bindings, us)),
            "Unhandled"
        );
        assert_eq!(
            command(resolve_binding(&chords(&["KeyJ"]), at(RoutingClass::Global, live), &bindings, us)),
            "twist.apply"
        );
        assert_eq!(
            command(resolve_binding(&chords(&["KeyJ"]), at(RoutingClass::Global, Destination::Draft), &bindings, us)),
            "operation.phase.append_twist"
        );
        assert_eq!(
            command(resolve_binding(&chords(&["Ctrl+KeyG"]), at(RoutingClass::Global, live), &bindings, us)),
            "Prefix"
        );
        assert_eq!(
            command(resolve_binding(&chords(&["Ctrl+KeyG", "KeyF"]), at(RoutingClass::Global, live), &bindings, us)),
            "display.frame.toggle"
        );
        assert_eq!(
            command(resolve_binding(&chords(&["Ctrl+KeyZ"]), at(RoutingClass::Global, live), &bindings, us)),
            "session.undo"
        );
        // Undo was resolved by character for US QWERTY; under AZERTY the map must be applied again.
        assert_eq!(
            command(resolve_binding(&chords(&["Ctrl+KeyZ"]), at(RoutingClass::Global, live), &bindings, "fr-azerty")),
            "Unhandled"
        );
        assert_eq!(command(resolve_binding(&[], at(RoutingClass::Global, live), &bindings, us)), "Unhandled");
    }
    assert_eq!(bindings, map.bindings());
    let french = KeyMap::default_for("fr-azerty").unwrap();
    let undo = |m: &KeyMap| m.bindings().iter().find(|b| b.command == "session.undo").unwrap().sequence.clone();
    assert_eq!(undo(&map), chords(&["Ctrl+KeyZ"]));
    assert_eq!(undo(&french), chords(&["Ctrl+KeyW"]));
    assert_eq!(default_specs().len(), map.bindings().len());
    assert_eq!(map.layout(), &Layout::US);
}

/// CT-EV4-02 with K1: a prefix group waits with no time-out, Escape closes it before anything else, and a key that
/// does not continue it closes it and runs nothing.
#[test]
fn a_prefix_group_waits_without_a_time_out_and_escape_closes_it() {
    let mut f = fixture("key", "us-qwerty");
    assert!(matches!(f.tap("Ctrl+KeyG"), Routed::Prefix));
    f.router.context.hover = Some(text("piece-3"));
    f.router.context.camera = json::object([("yaw", Value::Int(5))]);
    assert_eq!(f.router.pending(), chords(&["Ctrl+KeyG"]).as_slice());
    assert_eq!(row(&f.tap("KeyF")), "display.frame.toggle");
    assert!(f.router.pending().is_empty());

    assert!(matches!(f.tap("Ctrl+KeyG"), Routed::Prefix));
    assert!(matches!(f.tap("Escape"), Routed::PrefixClosed));
    assert_eq!(f.reached(), 1);
    assert_eq!(row(&f.tap("Escape")), "transient.close");

    assert!(matches!(f.tap("Ctrl+KeyG"), Routed::Prefix));
    assert!(matches!(f.tap("KeyX"), Routed::PrefixClosed));
    assert!(matches!(f.tap("KeyF"), Routed::Unhandled));
    assert_eq!(f.reached(), 2);
}

/// CT-EV4-03 (whole; EV4.5, KN5.9): a blocked interpreter leaves kernel-row key dispatch independent and undo
/// available through KN5. The tool key's program job is taken and held, as a slow program holds it; meanwhile the twist
/// key commits through KN5 and starts no program job, and the undo key restores the start. The held job's revision was
/// replaced, so its late result publishes nothing. The command route's half of the rule is CT-KN5-09 in
/// `c600-command`'s layer tests.
#[test]
fn a_held_program_job_blocks_neither_a_twist_key_nor_the_undo_key() {
    let mut f = fixture("key", "us-qwerty");
    f.pin_menu();
    f.router.context.grip = Some(2);
    let start = f.layer.snapshot().digest().to_owned();
    let tool = f.tap(TOOL_KEY).receipt().expect("the tool key reached the layer").clone();
    assert!(matches!(tool.status, Status::Running(_)), "{:?}", tool.status);
    let held = f.layer.take_program_job().expect("the tool row's job");
    assert_eq!(held.row, TOOL);

    let twist = f.tap("KeyJ");
    assert_eq!(row(&twist), "twist.apply");
    assert!(matches!(twist.receipt().unwrap().status, Status::Committed(_)), "{twist:?}");
    assert_ne!(f.layer.snapshot().digest(), start);
    assert!(f.layer.take_program_job().is_none(), "the twist key started no program");

    let undo = f.tap("Ctrl+KeyZ");
    assert_eq!(row(&undo), "session.undo");
    assert!(matches!(undo.receipt().unwrap().status, Status::Committed(_)), "{undo:?}");
    assert_eq!(f.layer.snapshot().digest(), start);
    assert!(f.layer.take_program_job().is_none(), "the undo key started no program");

    assert_eq!(f.layer.finish_program_job(held.job, Ok(Value::Null)), None, "the late result publishes nothing");
    assert_eq!(
        f.layer.outcome(&tool.command, &tool.request).unwrap().status,
        Status::Cancelled(CancelReason::ReplacedRevision)
    );
    assert_eq!(f.layer.store().log.len(), 2, "the twist and the undo");
}

/// CT-LB7-02 (part: a key bound to a tool row adds no token or capability to the row's program job: the job the key
/// route takes equals the job the command route takes (`Shell::invoke` of the same row and arguments on a second
/// layer) in row, kind, arguments, caller, revision, overlay, inputs and budget, and neither route sends a token,
/// since the row takes none; not run: grants, profiles, storage and forks, which `c600-run/tests/conformance_lg3.rs`
/// runs) (EV4.5, LI.20).
#[test]
fn a_key_bound_to_a_tool_row_adds_no_token_or_capability_to_its_job() {
    let mut keys = fixture("key", "us-qwerty");
    let mut cmds = fixture("cmd", "us-qwerty");
    assert!(!keys.layer.table().row(TOOL).unwrap().takes_token());
    let routed = keys.tap(TOOL_KEY);
    let dispatch = routed.dispatch().expect("the tool key reached the layer").clone();
    assert_eq!((dispatch.invocation.row.as_str(), dispatch.binding.context), (TOOL, RoutingClass::Global));
    assert!(dispatch.invocation.token.is_none(), "the key route sends no token");
    let by_key = keys.layer.take_program_job().expect("the key's job");

    let invoked = cmds.shell.invoke(&mut cmds.layer, TOOL, dispatch.invocation.args.clone()).unwrap();
    assert!(matches!(invoked.status, Status::Running(_)), "{:?}", invoked.status);
    let by_command = cmds.layer.take_program_job().expect("the command's job");

    assert_eq!((&by_key.row, by_key.kind, &by_key.args), (&by_command.row, by_command.kind, &by_command.args));
    assert_eq!((&by_key.caller, &by_key.overlay), (&by_command.caller, &by_command.overlay));
    assert_eq!((&by_key.inputs, by_key.budget), (&by_command.inputs, by_command.budget));
    assert_eq!(by_key.revision, by_command.revision);
    assert_eq!(by_key.caller, c600_command::Caller::Shell, "the press runs as the shell, not as a run");
    assert!(by_key.inputs.is_empty(), "no provenance block of the key reaches the job");
}

/// CT-LI-06 (part: a completed preview is no presentation permission, and building it changes no session head, on the
/// key route; the view model's atomic installation runs in `c600-view`'s view rows, and a derived plan that changes
/// no session head and goes stale when the head moves in `c600-tools/tests/views.rs`) (LI.24, ONE.36): the
/// preview job completes, but the commit key finds no fresh preview until a frame presents it, and neither the job nor
/// the refusal moves the head or writes the journal. Once presented, the same key commits.
#[test]
fn a_completed_preview_is_no_permission_to_commit_until_it_is_presented() {
    let mut f = fixture("key", "us-qwerty");
    let start = f.layer.snapshot().digest().to_owned();
    f.shell.draft = Some(word_draft(&word(&[1])));
    let preview = f.tap("Ctrl+Enter").receipt().expect("the preview reached the layer").clone();
    let Status::Running(job) = preview.status else { panic!("{:?}", preview.status) };
    assert_eq!(f.layer.run_jobs(), 1);
    let built = f.layer.outcome(&preview.command, &preview.request).unwrap();
    assert!(matches!(built.status, Status::Completed(_)), "{:?}", built.status);

    match f.tap("Ctrl+Shift+Enter") {
        Routed::Refused { row, error } => {
            assert_eq!((row.as_str(), error.code), ("operation.execute", code::NO_FRESH_PREVIEW))
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(f.layer.snapshot().digest(), start);
    assert!(f.layer.store().log.is_empty());

    f.frames.present(&mut f.shell, &mut f.layer, job).unwrap();
    let commit = f.tap("Ctrl+Shift+Enter");
    assert!(matches!(commit.receipt().unwrap().status, Status::Committed(_)), "{commit:?}");
    assert_ne!(f.layer.snapshot().digest(), start);
    assert_eq!(f.layer.store().log.len(), 1);
}
