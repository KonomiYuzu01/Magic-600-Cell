//! The twist keys on the pinned twist menu (LP3, C5; A6, KN3.7): the exact request each key types, the twist that
//! reaches the command layer through `twist.apply`, the same rotation as `jumble.twist.menu`'s letter with Jumble on,
//! and CT-BND-19's live-twist part, with the router joined to the view model and the tool's view as `c600-tools`
//! builds them. Every event is synthetic and every result is a source fixture outcome: no device, renderer or timing
//! is involved.
//! CT-BND-19 (source, part: no view-model field holds a live twist before its key releases it, with the router joined
//! to the view model and the tool's view; not run here: the tool's views holding a word's values only for the named
//! word, in `c600-tools/tests/views.rs`, which with this file is the whole by union).

mod common;

use std::collections::{BTreeMap, BTreeSet};

use c600_command::{code, CommandLayer, Status, TransactionKind, JUMBLE_ON_ROW, JUMBLE_TWIST_ROW, TWIST_ROW};
use c600_input::{Destination, DropReason, Focus, KeyEvent, Routed, TWIST_KEYS};
use c600_kernel::json::{self, Value};
use c600_kernel::Snapshot;
use c600_tools::tool::{manifest, BoundaryClass};
use c600_tools::views::{self, declared, model_revision};
use c600_view as view;
use common::{code_of, derived, exact, fixture, pin, retained, rotation, row, sent, text, word, Fixture, CAPS};

const BIG: u64 = 1 << 40;

/// `twist.apply`'s arguments for item `item` of the pinned menu at `cap`.
fn twist_args(cap: u32, item: u64) -> Value {
    json::object([("cap", Value::Int(i64::from(cap))), ("rotation", rotation(item))])
}

fn committed(routed: &Routed) -> bool {
    matches!(routed.receipt().map(|receipt| &receipt.status), Some(Status::Committed(_)))
}

/// The twist keys (A6; KN3.7; the drafts in `src/lib.rs`): with the I_a menu pinned, each key of [`TWIST_KEYS`] and
/// its Shift chord send `twist.apply` with the grip's cap and the exact request of the key's item, in KN3.7's binding
/// form against the pinned menu, and commit that item's retained word at the cap, at each of four caps. A key and its
/// Shift chord return the state they started from. With no menu in the router's context a twist key is refused
/// before any envelope, and in the draft destination the same exact request goes to the selected phase's append row.
#[test]
fn every_twist_key_sends_the_exact_request_of_its_item_and_commits_its_retained_word() {
    let mut f = fixture("keys", "us-qwerty");
    f.router.context.grip = Some(0);
    let Routed::Refused { row: refused, error } = f.tap("KeyJ") else { panic!("refused before any envelope") };
    assert_eq!((refused.as_str(), error.code), (TWIST_ROW, code::ARGUMENT_FORM));
    assert_eq!(f.reached(), 0);

    f.pin_menu();
    for (n, cap) in CAPS.into_iter().enumerate() {
        f.router.context.grip = Some(n);
        for (key, forward, inverse) in TWIST_KEYS {
            let start = f.layer.snapshot().clone();
            for (chord, item) in [(key.to_owned(), forward), (format!("Shift+{key}"), inverse)] {
                let what = format!("{chord} at cap {cap}");
                let from = f.layer.snapshot().clone();
                let routed = f.tap(&chord);
                assert_eq!(sent(&routed), (TWIST_ROW.to_owned(), twist_args(cap, item)), "{what}");
                assert!(committed(&routed), "{what}: {:?}", routed.receipt());
                let letters = retained(cap, item);
                assert_eq!(f.layer.snapshot().digest(), derived(&from, &letters).digest(), "{what}");
                assert_eq!(f.layer.store().log.last().expect("journaled").recipe, word(&letters), "{what}");
            }
            assert_eq!(f.layer.snapshot().digest(), start.digest(), "{key} and Shift+{key} at cap {cap}");
        }
    }
    assert_eq!(f.layer.store().log.len(), CAPS.len() * TWIST_KEYS.len() * 2);
    assert!(f.layer.preview().is_none(), "a live twist has no preview");

    // The draft destination: the same exact request, appended to the selected phase.
    f.router.context.destination = Destination::Draft;
    f.router.context.phase = Some(text("phase-1"));
    f.router.context.grip = Some(2);
    let before = f.layer.revision().clone();
    let routed = f.tap("KeyL");
    let Value::Object(mut args) = twist_args(CAPS[2], 3) else { panic!("an object") };
    args.insert("phase".to_owned(), text("phase-1"));
    assert_eq!(sent(&routed), ("operation.phase.append_twist".to_owned(), Value::Object(args)));
    assert_eq!(code_of(routed.receipt().expect("dispatched")), code::ROW_NOT_BUILT);
    assert_eq!(f.layer.revision(), &before);
}

/// Jumble on (EV4.4; DT-M1 C6; the drafts in `src/lib.rs`): the router reads no Jumble mode, so a twist key still
/// sends `twist.apply`, which the layer refuses with Jumble on and which changes nothing. The rotation the key sent
/// is exactly the `letter` that C6's menu-twist row takes: sent through `jumble.twist.menu` with the same cap and the
/// current revision, it commits one pose move, and the retained state stays.
#[test]
fn with_jumble_on_a_twist_keys_rotation_is_the_menu_twist_rows_letter() {
    let mut f = fixture("jumble", "us-qwerty");
    f.pin_menu();
    f.router.context.grip = Some(1);
    let on = json::object([("revision", f.layer.revision().to_json())]);
    let receipt = f.shell.invoke(&mut f.layer, JUMBLE_ON_ROW, on).expect("prepared");
    assert!(matches!(receipt.status, Status::Committed(_)), "{:?}", receipt.status);
    let (before, labels) = (f.layer.revision().clone(), f.layer.snapshot().digest().to_owned());

    let routed = f.tap("KeyJ");
    let (row, args) = sent(&routed);
    assert_eq!((row.as_str(), &args), (TWIST_ROW, &twist_args(CAPS[1], 1)));
    assert_eq!(code_of(routed.receipt().expect("dispatched")), code::RETAINED_TOOL_NEEDS_JUMBLE_OFF);
    assert_eq!(f.layer.revision(), &before);

    let field = |name: &str| args.get(name).expect("the key's argument").clone();
    let menu = json::object([("cap", field("cap")), ("letter", field("rotation")), ("revision", before.to_json())]);
    let receipt = f.shell.invoke(&mut f.layer, JUMBLE_TWIST_ROW, menu).expect("prepared");
    assert!(matches!(receipt.status, Status::Committed(_)), "{:?}", receipt.status);
    let after = f.layer.revision().clone();
    assert_eq!(
        (after.state, after.workspace, after.protection),
        (before.state + 1, before.workspace, before.protection)
    );
    assert_ne!(after.digest, before.digest);
    assert_eq!(f.layer.jumbled().expect("Jumble is on").digest(&exact()), after.digest);
    let last = f.layer.store().log.last().expect("journaled");
    assert_eq!((last.kind, last.row.as_str()), (TransactionKind::JumbleMove, JUMBLE_TWIST_ROW));
    assert_eq!(f.layer.snapshot().digest(), labels, "the retained state stays");
}

// ----- CT-BND-19, the live twist: the view model and the tool's view as `c600-tools`' view tests build them

/// A complete fixture profile of the view model: every field generous, `overdraw` 1.
fn profile() -> view::LimitProfile {
    let entries: Vec<(&str, u64, &str)> = view::Field::ALL
        .iter()
        .map(|field| (field.name(), if *field == view::Field::Overdraw { 1 } else { BIG }, field.unit()))
        .collect();
    view::LimitProfile::from_entries("fixture-1", &entries).expect("a complete fixture profile")
}

/// The view model's candidate bundle of the layer's current revision. The labels part is the digest of the state's
/// label digest; the metadata and styles parts are fixture digests bound to the revision.
fn candidate(layer: &CommandLayer) -> view::BundleCandidate {
    let revision = model_revision(layer.revision()).expect("a label digest");
    let part = |kind: view::PartKind, digest: [u8; 32]| view::BundlePart { kind, revision, digest };
    let fixture = |name: &str| {
        let text = format!("{name} of {}", layer.revision().to_json().canonical());
        c600_digest::sha256(text.as_bytes())
    };
    let labels = c600_digest::sha256(layer.revision().digest.as_bytes());
    let parts = vec![
        part(view::PartKind::Metadata, fixture("metadata")),
        part(view::PartKind::Labels, labels),
        part(view::PartKind::Styles, fixture("styles")),
    ];
    view::BundleCandidate { revision, mode: view::Mode::JumbleOff, parts }
}

/// The command ID the command layer gives the solver's `view.open` in these tests.
const OPEN: view::CommandId = view::CommandId::Caller(1 << 20);

/// A view model in the solving profile at the layer's revision, with the tool's view open under its declared class.
fn model_with_view(layer: &CommandLayer) -> (view::ViewModel, view::ViewId) {
    let workload = view::Workload::default();
    let mut model = view::ViewModel::new(view::Profile::Solving, profile(), &candidate(layer), workload, workload)
        .expect("Global and Local fit");
    let id = view::ViewId::new("full_600_cell.preview");
    let workload = view::Workload { vertex_evals: 1, cpu_bytes: 1, gpu_bytes: 1, ..view::Workload::default() };
    let declared = declared(&manifest(), BoundaryClass::Checking, workload, view::OutputAllowance::new(BIG, 16))
        .expect("the manifest declares the class");
    model
        .open_view(OPEN, view::Origin::Solver, id.clone(), views::source(&manifest()), declared)
        .expect("the view opens");
    (model, id)
}

/// A state digest in hex as the view model's digest bytes.
fn bytes(hex: &str) -> view::Digest {
    assert_eq!(hex.len(), 64, "a SHA-256 in hex");
    let mut digest = [0u8; 32];
    for (byte, pair) in digest.iter_mut().zip(hex.as_bytes().chunks(2)) {
        *byte = std::str::from_utf8(pair).ok().and_then(|pair| u8::from_str_radix(pair, 16).ok()).expect("hex");
    }
    digest
}

/// Every digest a field of the view model holds: the adopted revision, the bundle and its parts, the next frame's
/// inputs and, for every open view, the revision it shows, its installed output and its status.
fn held(model: &view::ViewModel) -> BTreeSet<view::Digest> {
    let mut held = BTreeSet::new();
    let bundle = model.bundle();
    held.extend([model.adopted().digest, bundle.revision().digest, *bundle.digest()]);
    held.extend(bundle.parts().iter().flat_map(|part| [part.revision.digest, part.digest]));
    let frame = model.frame_input();
    held.extend([frame.revision.digest, frame.bundle]);
    for drawn in &frame.views {
        match &drawn.draw {
            view::Draw::Output { base, publication, .. } => held.extend([base.revision.digest, *publication]),
            view::Draw::Stale { base } => held.extend([base.revision.digest]),
            view::Draw::Nothing => {}
        }
    }
    for id in model.views() {
        held.extend(model.shown_revision(id).map(|revision| revision.digest));
        if let Some(installed) = model.installed(id) {
            held.extend([installed.base.revision.digest, installed.digest]);
        }
        match model.status(id) {
            Some(view::ViewStatus::Current { base, publication }) => held.extend([base.revision.digest, publication]),
            Some(view::ViewStatus::Empty { revision, base }) => {
                held.extend([revision.digest]);
                held.extend(base.map(|base| base.revision.digest));
            }
            Some(view::ViewStatus::Loading { revision, .. }) => held.extend([revision.digest]),
            Some(view::ViewStatus::Stale { base, adopted, .. }) => held.extend([base.revision.digest, adopted.digest]),
            _ => {}
        }
    }
    held
}

/// The values a live twist from `base` could put in the view model, at each of the fixture's four grip slots (`CAPS`)
/// and for every retained item of the pinned menu: the digest of the state it reaches, and that state's labels part as
/// [`candidate`] builds it.
fn twist_values(base: &Snapshot) -> BTreeMap<(u32, u64), [view::Digest; 2]> {
    let mut values = BTreeMap::new();
    for cap in CAPS {
        for item in 1..=11 {
            let reached = derived(base, &retained(cap, item));
            let hex = reached.digest();
            values.insert((cap, item), [bytes(hex), c600_digest::sha256(hex.as_bytes())]);
        }
    }
    values
}

/// What holds while no live twist is released: the host adopts the layer's bundle and the view model keeps the start
/// frame's revision and inputs, schedules no run and holds no value of any twist; the layer holds no preview and no
/// journal entry.
fn nothing_released(
    f: &Fixture,
    model: &mut view::ViewModel,
    start: &view::FrameInput,
    values: &BTreeMap<(u32, u64), [view::Digest; 2]>,
    what: &str,
) {
    assert_eq!(model.adopt(&candidate(&f.layer)).expect("a complete bundle"), start.revision, "{what}");
    assert_eq!(&model.frame_input(), start, "{what}");
    assert!(model.take_jobs().is_empty(), "{what}: no run is scheduled");
    assert!(f.layer.preview().is_none() && f.layer.store().log.is_empty(), "{what}");
    let held = held(model);
    for (twist, digests) in values {
        assert!(digests.iter().all(|digest| !held.contains(digest)), "{what}: the view model holds twist {twist:?}");
    }
}

fn twist_repeats_are_dropped(f: &mut Fixture, what: &str) {
    for _ in 0..3 {
        let routed = f.key(KeyEvent::repeat("KeyJ"));
        let dropped = matches!(&routed, Routed::Dropped(DropReason::NotRepeatable(Some(row))) if row == TWIST_ROW);
        assert!(dropped, "{what}: {routed:?}");
    }
}

/// CT-BND-19 (BND.4, BND.5), part, source: the live twist, with the router joined to the view model and the tool's view
/// of `c600-tools`. A live twist has no preview (owner answer of 6 November 2026), so the solver releases it with the
/// twist key's key-down, the one deliberate action that commits it. Before that, through a twist key with no grip, the
/// grip key, a twist key typed in a text field, one in the draft destination, one with no pinned menu and one that met
/// a held commit lane, with its repeats and its key-up, no field of the view model holds a value of any twist a key
/// could release at the fixture's four grip slots. The view model's only input is the layer's bundle, which the host
/// adopts after each event: the layer stays at the start revision, and the view model schedules no run and holds no
/// digest of a twisted state. After the key-down commits the twist, the view model adopts the committed state at once:
/// Global and Local show it, and the tool's view runs once at it and shows it. The key's repeats and its key-up release
/// nothing more.
#[test]
fn ct_bnd_19_no_view_model_field_holds_a_live_twist_before_its_key_releases_it() {
    let mut f = fixture("bnd", "us-qwerty");
    f.pin_menu();
    let (mut model, tool) = model_with_view(&f.layer);
    assert_eq!(model.pump(|_| view::RunOutcome::Published(view::Content::default())), 1);
    let start = model.frame_input();
    let solved = f.layer.snapshot().clone();
    let values = twist_values(&solved);
    let distinct: BTreeSet<&view::Digest> = values.values().flatten().collect();
    assert_eq!(distinct.len(), 2 * CAPS.len() * 11, "every twist reaches a state of its own");
    nothing_released(&f, &mut model, &start, &values, "at the start");

    // Before the release.
    let Routed::Refused { row: refused, error } = f.tap("KeyJ") else { panic!("no grip: refused before any envelope") };
    assert_eq!((refused.as_str(), error.code), (TWIST_ROW, code::ARGUMENT_FORM));
    nothing_released(&f, &mut model, &start, &values, "a twist key with no grip");

    let routed = f.tap("KeyQ");
    assert_eq!(
        (row(&routed).as_str(), code_of(routed.receipt().expect("dispatched"))),
        ("grip.select", code::ROW_NOT_BUILT)
    );
    nothing_released(&f, &mut model, &start, &values, "the grip key");
    f.router.context.grip = Some(0);

    f.router.set_focus(Focus::Text);
    assert!(matches!(f.key(KeyEvent::typed("KeyJ", 'j')), Routed::Text));
    f.up("KeyJ");
    f.router.set_focus(Focus::Puzzle);
    nothing_released(&f, &mut model, &start, &values, "a twist key typed in a text field");

    f.router.context.destination = Destination::Draft;
    f.router.context.phase = Some(text("phase-1"));
    let routed = f.tap("KeyJ");
    let shape = (row(&routed), code_of(routed.receipt().expect("dispatched")));
    assert_eq!(shape, ("operation.phase.append_twist".to_owned(), code::ROW_NOT_BUILT));
    f.router.context.destination = Destination::Live;
    f.router.context.phase = None;
    nothing_released(&f, &mut model, &start, &values, "a twist key in the draft destination");

    f.router.context.menu = None;
    let Routed::Refused { row: refused, error } = f.tap("KeyJ") else { panic!("no menu: refused before any envelope") };
    assert_eq!((refused.as_str(), error.code), (TWIST_ROW, code::ARGUMENT_FORM));
    f.router.context.menu = Some(pin());
    nothing_released(&f, &mut model, &start, &values, "a twist key with no pinned menu");

    f.layer.hold_commit_lane(true);
    let busy = f.down("KeyJ");
    assert_eq!(code_of(busy.receipt().expect("dispatched")), code::COMMIT_IN_FLIGHT);
    twist_repeats_are_dropped(&mut f, "a busy twist key");
    f.up("KeyJ");
    f.layer.hold_commit_lane(false);
    assert_eq!(f.layer.run_jobs(), 0);
    nothing_released(&f, &mut model, &start, &values, "a twist key that met a held commit lane");

    // The release: the twist key's key-down commits the twist, and the view model adopts the committed state at once.
    let release = f.down("KeyJ");
    assert_eq!(sent(&release), (TWIST_ROW.to_owned(), twist_args(CAPS[0], 1)));
    assert!(committed(&release), "{:?}", release.receipt());
    assert_eq!(f.layer.snapshot().digest(), derived(&solved, &retained(CAPS[0], 1)).digest());
    assert!(f.layer.preview().is_none());
    let adopted = model.adopt(&candidate(&f.layer)).expect("a complete bundle of one revision");
    assert_eq!((adopted.state, adopted.digest), (start.revision.state + 1, values[&(CAPS[0], 1)][0]));
    for id in [view::ViewId::global(), view::ViewId::local()] {
        assert_eq!(model.shown_revision(&id), Some(adopted), "{id:?} shows the committed state");
    }
    assert!(matches!(model.status(&tool), Some(view::ViewStatus::Stale { adopted: now, .. }) if now == adopted));
    let mut bases = Vec::new();
    let ran = model.pump(|job| {
        bases.push(job.base.revision);
        view::RunOutcome::Published(view::Content::default())
    });
    assert_eq!((ran, bases), (1, vec![adopted]), "the tool's view runs once, at the committed state");
    assert_eq!(model.shown_revision(&tool), Some(adopted));
    let frame = model.frame_input();
    assert_eq!(frame.revision, adopted);
    assert!(frame.views.iter().all(|drawn| !matches!(drawn.draw, view::Draw::Stale { .. })), "{frame:?}");
    let held_after = held(&model);
    for (twist, digests) in &values {
        let released = *twist == (CAPS[0], 1);
        assert!(digests.iter().all(|digest| held_after.contains(digest) == released), "{twist:?}");
    }

    // The key's repeats and its key-up release nothing more.
    twist_repeats_are_dropped(&mut f, "the released twist key");
    assert!(matches!(f.up("KeyJ"), Routed::Released));
    assert_eq!(model.adopt(&candidate(&f.layer)).expect("the same bundle"), adopted);
    assert_eq!(model.pump(|_| view::RunOutcome::Published(view::Content::default())), 0);
    assert_eq!(held(&model), held_after);
    assert_eq!(f.layer.store().log.len(), 1);
}
