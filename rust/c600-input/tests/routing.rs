//! CT-EV4-02 (synthetic; EV4.3, EV4.4): key capture, text, list navigation and global dispatch run in the fixed order,
//! and resolved absolute arguments survive later camera, focus, hover and filter changes. Every key event here is
//! synthetic; none of this is Windows input evidence.

mod common;

use c600_command::{code, digest, draft_args, word_draft, Receipt, Status};
use c600_input::{
    default_specs, resolve_binding, Chord, Destination, Focus, KeyEvent, KeyMap, Layout, Resolution, Routed,
    RoutingClass, RoutingContext, LIST_ROWS,
};
use c600_kernel::json::{self, Value};

use common::{code_of, fixture, list, row, sent, slots, text, word, TOOL, TOOL_KEY};

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

/// CT-EV4-02, the argument leg (EV4.3, EV4.4, EV4.8): relative names (the current grip, a slot, a list row, the
/// editor's draft) are resolved at the press. A later camera, focus, hover or filter change neither reinterprets an
/// invocation already sent nor changes what the same global key sends next; only the rebind command changes the
/// applied slot map, and a list row number names the row shown at its own press.
#[test]
fn resolved_arguments_survive_later_camera_focus_hover_and_filter_changes() {
    let mut f = fixture("key", "us-qwerty");
    f.router.context.grip = Some(1);
    let grip = f.tap("KeyW");
    let twist = f.tap("KeyJ");
    assert_eq!(sent(&grip).1, json::object([("cap", text("c1")), ("frame", text("f1"))]));
    let rotation = json::object([("axis", Value::Int(0)), ("inverse", Value::Bool(false))]);
    assert_eq!(sent(&twist).1, json::object([("cap", text("c1")), ("rotation", rotation.clone())]));
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
    f.router.context.rebind(slots("r"));
    assert_eq!(sent(&f.tap("KeyW")).1, json::object([("cap", text("rc1")), ("frame", text("rf1"))]));
    assert_eq!(sent(&f.tap("KeyJ")).1, json::object([("cap", text("rc1")), ("rotation", rotation)]));
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
    keys.router.context.grip = Some(2);
    keys.shell.draft = Some(word_draft(&word(&[1, 3])));
    let mut pairs: Vec<(Receipt, Receipt)> = Vec::new();
    for chord in ["Ctrl+Enter", "Ctrl+Shift+Enter", "Ctrl+KeyZ", "Ctrl+KeyY", "F9", "KeyJ", TOOL_KEY] {
        if chord == "Ctrl+Shift+Enter" {
            // The preview job runs on both layers; its settled receipts agree as well.
            let (key, cmd) = pairs[0].clone();
            let id = keys.build_and_present(&key);
            assert_eq!(cmds.layer.run_jobs(), 1);
            cmds.shell.present(&mut cmds.layer, id).unwrap();
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
    assert_eq!(statuses, ["running", "committed", "committed", "committed", "refused", "refused", "running"]);
    assert_eq!(code_of(&pairs[4].0), code::ROW_NOT_BUILT);
    assert_eq!(code_of(&pairs[5].0), code::ROW_NOT_BUILT);
    // The journals agree on every field but the command ID and the request digest.
    let journal = |f: &common::Fixture| {
        let log = &f.layer.store().log;
        log.iter()
            .map(|t| (t.kind, t.event.clone(), t.order, t.target, t.row.clone(), t.literal.clone(), t.post.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(journal(&keys).len(), 3);
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
