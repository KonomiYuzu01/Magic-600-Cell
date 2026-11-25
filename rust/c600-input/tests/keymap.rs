//! CT-EV4-06 (synthetic, part: the key-map checks on the specs, EV4.7; LP4-8). They test duplicates and the
//! prefix-code property separately in each routing class and under every declared layout, and any finding refuses
//! the whole map before it applies. `c600-command/tests/keymap.rs` runs the same checks through `Shell::bind` on the
//! shell's one map; the two files are the whole by union.
//! CT-PS1-13 (part: the findings a stored map must report, duplicates, a complete binding that is also a prefix
//! and a layout conversion that collides; not run: the round trip of `c600-binding` and `c600-key-map` records,
//! which no writer stores yet).
//!
//! The layouts are this crate's tables, not a platform's: none of this is Windows input or IME evidence. No assertion
//! reads a finding's detail text; each compares the code, the layout and the specs involved.

use c600_input::keymap::code::{BINDING_CONFLICT, BINDING_LAYOUT_MISMATCH, BINDING_PREFIX_CONFLICT};
use c600_input::{
    check, default_specs, BindingSpec, Chord, ChordIntent, Context, Destination, KeyMap, Layout, Router, RoutingClass,
    LIST_ROWS,
};

use RoutingClass::{Global, List};

const BOTH: [Layout; 2] = [Layout::US, Layout::FRENCH];
const ACTIVE: [&str; 2] = ["us-qwerty", "fr-azerty"];

/// A finding without its prose: its code, the layout it holds under and the specs involved.
type Found = (&'static str, Option<&'static str>, Vec<usize>);

fn found(specs: &[BindingSpec], declared: &[Layout], active: &str) -> Vec<Found> {
    check(specs, declared, active).into_iter().map(|finding| (finding.code, finding.layout, finding.specs)).collect()
}

fn passes(specs: &[BindingSpec], declared: &[Layout], active: &str) {
    let findings = found(specs, declared, active);
    assert!(findings.is_empty(), "{findings:?}");
    assert!(KeyMap::apply(specs, declared, active).is_ok());
}

/// The index of the spec recorded with `sequence` for `command`.
fn at(specs: &[BindingSpec], sequence: &[&str], command: &str) -> usize {
    let sequence: Vec<ChordIntent> = sequence.iter().map(|text| ChordIntent::parse(text).unwrap()).collect();
    specs
        .iter()
        .position(|spec| spec.sequence == sequence && spec.command == command)
        .unwrap_or_else(|| panic!("no spec {sequence:?} for {command}"))
}

/// The default specs with `extra` appended, and its index.
fn with(extra: BindingSpec) -> (Vec<BindingSpec>, usize) {
    let mut specs = default_specs();
    specs.push(extra);
    let n = specs.len() - 1;
    (specs, n)
}

fn chord(text: &str) -> Chord {
    Chord::parse(text).unwrap()
}

/// CT-EV4-06, the control (EV4.7; DT-M1 Q2 (b)): the draft default map passes every check under both declared
/// layouts, although the same arrows sit in the list class and the global class and every twist key has a live and a
/// draft binding. Its physical bindings resolve alike under both layouts; undo and redo of both histories, recorded by
/// character, resolve per layout and name the layout they resolved under.
#[test]
fn the_default_map_passes_every_check_under_both_declared_layouts() {
    let specs = default_specs();
    for active in ACTIVE {
        passes(&specs, &BOTH, active);
    }
    assert_ne!(at(&specs, &["ArrowDown"], "list.move"), at(&specs, &["ArrowDown"], "camera.rotate"));
    assert_ne!(at(&specs, &["KeyJ"], "twist.apply"), at(&specs, &["KeyJ"], "operation.phase.append_twist"));

    let us = KeyMap::default_for("us-qwerty").unwrap();
    let fr = KeyMap::default_for("fr-azerty").unwrap();
    assert_eq!(us.declared(), ACTIVE);
    assert_eq!(us.bindings().len(), fr.bindings().len());
    for (a, b) in us.bindings().iter().zip(fr.bindings()) {
        assert_eq!(a.command, b.command);
        match a.command.as_str() {
            "session.undo" => {
                assert_eq!((&a.sequence, &b.sequence), (&vec![chord("Ctrl+KeyZ")], &vec![chord("Ctrl+KeyW")]));
                assert_eq!((a.layout, b.layout), (Some("us-qwerty"), Some("fr-azerty")));
            }
            "session.redo" => {
                assert_eq!((&a.sequence, &b.sequence), (&vec![chord("Ctrl+KeyY")], &vec![chord("Ctrl+KeyY")]));
                assert_eq!((a.layout, b.layout), (Some("us-qwerty"), Some("fr-azerty")));
            }
            "layout.undo" => {
                let (us, fr) = (vec![chord("Ctrl+Alt+KeyZ")], vec![chord("Ctrl+Alt+KeyW")]);
                assert_eq!((&a.sequence, &b.sequence), (&us, &fr));
                assert_eq!((a.layout, b.layout), (Some("us-qwerty"), Some("fr-azerty")));
            }
            "layout.redo" => {
                let both = vec![chord("Ctrl+Alt+KeyY")];
                assert_eq!((&a.sequence, &b.sequence), (&both, &both));
                assert_eq!((a.layout, b.layout), (Some("us-qwerty"), Some("fr-azerty")));
            }
            _ => {
                assert_eq!(a.sequence, b.sequence);
                assert_eq!((a.layout, b.layout), (None, None));
            }
        }
    }
}

/// CT-EV4-06, duplicates (EV4.3, EV4.7): two bindings of one chord sequence in one routing class are
/// `EV4:input.binding_conflict`, in the global class and in the list class alike, whichever declared layout is active;
/// two character bindings collide under each declared layout, and the finding names it. A binding with no destination
/// collides with both destinations' bindings, a draft-only one with the draft binding only. The same chord in another
/// routing class is no duplicate.
#[test]
fn duplicates_fail_per_routing_class() {
    let (specs, n) = with(BindingSpec::new(&["F9"], Global, "display.frame.toggle", vec![]));
    let f9 = at(&specs, &["F9"], "keyboard.toggle");
    for active in ACTIVE {
        assert_eq!(found(&specs, &BOTH, active), vec![(BINDING_CONFLICT, None, vec![f9, n])]);
        assert!(KeyMap::apply(&specs, &BOTH, active).is_err());
    }

    let (specs, n) = with(BindingSpec::new(&["ArrowUp"], List, "object.inspect", vec![]));
    let up = at(&specs, &["ArrowUp"], "list.move");
    assert_eq!(found(&specs, &BOTH, "us-qwerty"), vec![(BINDING_CONFLICT, None, vec![up, n])]);

    let (specs, _) = with(BindingSpec::new(&["Escape"], List, "object.inspect", vec![]));
    passes(&specs, &BOTH, "us-qwerty");

    let (specs, n) = with(BindingSpec::new(&["Ctrl+'z'"], Global, "session.redo", vec![]));
    let undo = at(&specs, &["Ctrl+'z'"], "session.undo");
    assert_eq!(
        found(&specs, &BOTH, "us-qwerty"),
        vec![
            (BINDING_CONFLICT, Some("us-qwerty"), vec![undo, n]),
            (BINDING_CONFLICT, Some("fr-azerty"), vec![undo, n])
        ]
    );

    let (specs, n) = with(BindingSpec::new(&["KeyJ"], Global, "grip.select", vec![]));
    let live = at(&specs, &["KeyJ"], "twist.apply");
    let draft = at(&specs, &["KeyJ"], "operation.phase.append_twist");
    assert_eq!(
        found(&specs, &BOTH, "us-qwerty"),
        vec![(BINDING_CONFLICT, None, vec![live, n]), (BINDING_CONFLICT, None, vec![draft, n])]
    );
    let (specs, n) = with(BindingSpec::new(&["KeyJ"], Global, "grip.select", vec![]).only(Destination::Draft));
    assert_eq!(found(&specs, &BOTH, "us-qwerty"), vec![(BINDING_CONFLICT, None, vec![draft, n])]);
}

/// CT-EV4-06, the prefix-code property (EV4.7): no complete binding may also be a prefix group. The check runs apart
/// from the duplicate check (a second binding of a whole group sequence is a duplicate, not a prefix conflict), per
/// routing class, and without regard to the destination. A binding with no chord is a prefix of every sequence.
#[test]
fn a_complete_binding_that_is_also_a_prefix_fails_per_routing_class() {
    let (specs, n) = with(BindingSpec::new(&["Ctrl+KeyG"], Global, "keyboard.toggle", vec![]));
    let frame = at(&specs, &["Ctrl+KeyG", "KeyF"], "display.frame.toggle");
    let inverse = at(&specs, &["Ctrl+KeyG", "KeyI"], "twist.inverse.toggle");
    for active in ACTIVE {
        assert_eq!(
            found(&specs, &BOTH, active),
            vec![(BINDING_PREFIX_CONFLICT, None, vec![frame, n]), (BINDING_PREFIX_CONFLICT, None, vec![inverse, n])]
        );
        assert!(KeyMap::apply(&specs, &BOTH, active).is_err());
    }

    let (specs, n) = with(BindingSpec::new(&["Ctrl+KeyG", "KeyF"], Global, "keyboard.toggle", vec![]));
    assert_eq!(found(&specs, &BOTH, "us-qwerty"), vec![(BINDING_CONFLICT, None, vec![frame, n])]);

    let (specs, _) = with(BindingSpec::new(&["Ctrl+KeyG"], List, "list.move", vec![]));
    passes(&specs, &BOTH, "us-qwerty");

    let (specs, n) = with(BindingSpec::new(&["KeyJ", "KeyK"], Global, "grip.select", vec![]).only(Destination::Draft));
    let live = at(&specs, &["KeyJ"], "twist.apply");
    let draft = at(&specs, &["KeyJ"], "operation.phase.append_twist");
    assert_eq!(
        found(&specs, &BOTH, "us-qwerty"),
        vec![(BINDING_PREFIX_CONFLICT, None, vec![live, n]), (BINDING_PREFIX_CONFLICT, None, vec![draft, n])]
    );

    let (specs, n) = with(BindingSpec::new(&[], Global, "keyboard.toggle", vec![]));
    assert_eq!(found(&specs, &BOTH, "us-qwerty"), vec![(BINDING_PREFIX_CONFLICT, None, vec![n])]);
}

/// CT-EV4-06, the DT-M1 US and French layout counterexample (EV4.7; DT-M1 Q2 and K5, the counterexample of the design
/// table's command-keyboard topic): `Shift+KeyQ` bound by position and Shift+A bound by character are two chords on
/// US QWERTY and one on French AZERTY, where KeyQ types `a`. The pair passes with US alone declared and fails under
/// AZERTY with `EV4:input.binding_layout_mismatch` naming that layout, even while US is active, so the conflict is
/// reported before the map applies; the router keeps its current map.
#[test]
fn the_us_and_french_layout_counterexample_fails_under_its_conflicting_layout() {
    let capital = ChordIntent::parse("'A'").unwrap();
    for layout in BOTH {
        assert_eq!(capital.resolve(&layout), ChordIntent::parse("Shift+'a'").unwrap().resolve(&layout));
    }
    assert_eq!(capital.resolve(&Layout::US), Some(chord("Shift+KeyA")));
    assert_eq!(capital.resolve(&Layout::FRENCH), Some(chord("Shift+KeyQ")));

    let pair = vec![
        BindingSpec::new(&["Shift+KeyQ"], Global, "twist.apply", vec![]),
        BindingSpec::new(&["'A'"], Global, "keyboard.toggle", vec![]),
    ];
    passes(&pair, &[Layout::US], "us-qwerty");
    for active in ACTIVE {
        assert_eq!(found(&pair, &BOTH, active), vec![(BINDING_LAYOUT_MISMATCH, Some("fr-azerty"), vec![0, 1])]);
    }

    let mut specs = default_specs();
    specs.extend(pair);
    let (a, b) = (specs.len() - 2, specs.len() - 1);
    assert_eq!(found(&specs, &BOTH, "us-qwerty"), vec![(BINDING_LAYOUT_MISMATCH, Some("fr-azerty"), vec![a, b])]);

    let mut router = Router::new(KeyMap::default_for("us-qwerty").unwrap(), Context::new(Vec::new()));
    let before = router.keymap().clone();
    let refused = router.apply(&specs, &BOTH, "us-qwerty").unwrap_err();
    assert_eq!(refused.iter().map(|finding| finding.code).collect::<Vec<_>>(), [BINDING_LAYOUT_MISMATCH]);
    assert_eq!(router.keymap(), &before, "the refused map did not apply");
    router.apply(&specs, &[Layout::US], "us-qwerty").unwrap();
    assert_eq!(router.keymap().declared(), ["us-qwerty"]);
    assert_eq!(router.keymap().bindings().len(), before.bindings().len() + 2);
}

/// CT-EV4-06, layout resolution (EV4.7): a character that a declared layout does not type fails under that layout,
/// and an active layout outside the declared ones fails, both before the map applies.
#[test]
fn a_character_a_declared_layout_cannot_type_and_an_undeclared_active_layout_fail() {
    let (specs, n) = with(BindingSpec::new(&["Ctrl+'é'"], Global, "keyboard.toggle", vec![]));
    passes(&specs, &[Layout::FRENCH], "fr-azerty");
    for active in ACTIVE {
        assert_eq!(found(&specs, &BOTH, active), vec![(BINDING_LAYOUT_MISMATCH, Some("us-qwerty"), vec![n])]);
    }

    assert_eq!(found(&default_specs(), &[Layout::US], "fr-azerty"), vec![(BINDING_LAYOUT_MISMATCH, None, vec![])]);
    assert!(KeyMap::apply(&default_specs(), &[Layout::US], "fr-azerty").is_err());
    assert!(KeyMap::default_for("de-qwertz").is_err());
}

/// The class rule (EV4.4; DT-M1 C2 and fix-first item 2; a draft reading of `EV4:input.binding_conflict`, recorded in
/// the crate's drafts): a text-class or capture-class binding, and a list-class binding to a row other than
/// navigation, inspection and O9's row copy, conflict with the keys their routing class takes itself.
#[test]
fn a_binding_its_routing_class_cannot_take_fails() {
    for class in [RoutingClass::Text, RoutingClass::Capture] {
        let (specs, n) = with(BindingSpec::new(&["F2"], class, "keyboard.toggle", vec![]));
        assert_eq!(found(&specs, &BOTH, "us-qwerty"), vec![(BINDING_CONFLICT, None, vec![n])]);
    }
    let (specs, n) = with(BindingSpec::new(&["KeyJ"], List, "twist.apply", vec![]));
    assert_eq!(found(&specs, &BOTH, "us-qwerty"), vec![(BINDING_CONFLICT, None, vec![n])]);
    for row in LIST_ROWS {
        let (specs, _) = with(BindingSpec::new(&["PageDown"], List, row, vec![]));
        passes(&specs, &BOTH, "us-qwerty");
    }
}
