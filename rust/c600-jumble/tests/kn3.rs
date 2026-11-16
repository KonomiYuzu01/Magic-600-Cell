//! Exact rotation requests as `twist.apply`'s input on both puzzles (state-contract amendment A6; KN3.4, KN3.7 to
//! KN3.9; JI.35): free mode on the jumbling puzzle, menu direct and drag against the pinned I_a menu at cap 0 (the
//! menu's own pole, where `work/experiments/language-lx/lx9-exact-request/REQUEST.md` states its vectors), replay of
//! the recorded rotation, and the retained dispatch on 600-cell-Full.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use c600_jumble::codes;
use c600_jumble::q5::Q5;
use c600_jumble::rotation::{self, chooser_runs, frobenius2, Candidate, Choice, Request};
use c600_jumble::state::{State, Status};
use c600_jumble::twist::Menu;
use c600_jumble::Context;
use c600_kernel::json::{self, Value};
use c600_kernel::request::{self as wire, Binding, MenuPin, Operation};
use c600_kernel::{Derivation, Kernel, ProtectedSet};

const SET: &str = "fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c";
const ORDER: &str = "5a0429d2946fbf58b495d8f300e24a3fa164b1d9518ce4a5c3f4ef0a40625c3c";
const REVERSED_ORDER: &str = "1a87a4209a933f47a568eac9e881697614bf53e1f7411e817f5764a3a5f92a78";
/// SHA-256 of the canonical records of `identical_requests_give_identical_choices_and_records`'s five requests.
const RECORD_DIGESTS: [&str; 5] = [
    "e273d48b633591c3b66b710280067d69676c27c6fd3397184bc6eff0df875926",
    "a3faf822b9d5cd1da805b6d4ac1890bc656fd291757481f1af9942381ed4b64b",
    "656efe437ad1a08a2d57aeced94990dd7aba348b6551d19511a44f56accace69",
    "755c55a3e924b63645de0c283707fdadceeb9b5d2b42f2f21944ecf3639b3c78",
    "0a14defe1b2e387f0abeb54251cf54c4cd821250a07e2771839b0a0d5e0ecc96",
];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn ctx() -> &'static Context {
    static CTX: OnceLock<Context> = OnceLock::new();
    CTX.get_or_init(|| Context::load(&repository().join("assets")).expect("the jumbling model loads"))
}

fn menu_record() -> Value {
    let fixture = std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).expect("the I_a fixture");
    json::parse(&fixture).expect("the I_a fixture is JSON").get("menu").expect("a menu").clone()
}

fn pinned() -> Menu {
    Menu::from_record(ctx(), &menu_record()).expect("the pinned I_a menu")
}

fn reversed() -> Menu {
    let mut record = menu_record();
    if let Value::Object(fields) = &mut record {
        if let Some(Value::Array(base)) = fields.get_mut("base") {
            base.reverse();
        }
    }
    Menu::from_record(ctx(), &record).expect("the reversed I_a menu")
}

fn free(request: Vec<u8>, n: u128, d: u128) -> Binding {
    Binding { request, cap: 0, operation: Operation::Free { n, d }, menu: None }
}

fn bound(request: Vec<u8>, operation: Operation, menu: &Menu) -> Binding {
    let pin = MenuPin { set: menu.identity.clone(), order: menu.order.clone() };
    Binding { request, cap: 0, operation, menu: Some(pin) }
}

fn refusal(menu: Option<&Menu>, binding: &Binding) -> &'static str {
    rotation::resolve(ctx(), menu, binding).expect_err("refused").code
}

fn cayley(denominator: u128, numerators: [i64; 3]) -> Candidate {
    Candidate { half_turn: false, denominator, numerators }
}

fn half_turn(axis: [i64; 3]) -> Candidate {
    Candidate { half_turn: true, denominator: 1, numerators: axis }
}

/// CT-KN3-04 (part: free mode on the jumbling puzzle with explicit N and D: grid members return themselves at
/// distance zero; C=(69264588888054800706886561/2^79,0,0) with N=1, D=16 gives H(1,0,0); `LX9B1|C|1:1|0:0|0:0` with
/// N=0, D=1 gives the identity at squared distance 8/5; at N=0 the grid is the identity alone, so N=0 with the largest
/// admitted D=2^40 returns it after one comparison; M refuses with `KN2:input.menu_binding_mismatch`; bounds outside
/// the search limit refuse with `KN2:resource_limit.rotation_search_bound` with no search; the decoding half is the
/// unit test `rotation::tests`; not run: a search for N ≥ 1 near the limit, which is research-only and unbudgeted).
#[test]
fn free_mode_chooses_the_nearest_grid_point_exactly() {
    let ctx = ctx();
    let cases = [
        // On the grid: the rotation as named, at distance zero.
        (wire::cayley_request(["1:1", "0:0", "0:0"]), 1, 2, cayley(2, [1, 0, 0]), Q5::zero()),
        (wire::cayley_request(["-3:0", "1:0", "0:0"]), 3, 1, cayley(1, [-3, 1, 0]), Q5::zero()),
        (wire::half_turn_request(["1:0", "0:0", "0:0"]), 1, 1, half_turn([1, 0, 0]), Q5::zero()),
        (wire::half_turn_request(["0:0", "1:0", "-1:0"]), 1, 3, half_turn([0, 1, -1]), Q5::zero()),
        // The chapter's vectors.
        (b"LX9B1|C|1:1|0:0|0:0\n".to_vec(), 0, 1, cayley(1, [0, 0, 0]), Q5::of(8, 0, 5)),
        (
            wire::cayley_request(["69264588888054800706886561:79", "0:0", "0:0"]),
            1,
            16,
            half_turn([1, 0, 0]),
            // 8/(1 + ω·ω) with ω = m/2^79: the half-turn's squared distance.
            Q5::new(
                num_bigint::BigInt::from(8u8) << 158usize,
                num_bigint::BigInt::from(0u8),
                (num_bigint::BigInt::from(1u8) << 158usize)
                    + num_bigint::BigInt::parse_bytes(b"69264588888054800706886561", 10).expect("decimal").pow(2),
            ),
        ),
        // ω = 3/8 between grid points: 1/3 at 8(73·10 − 27²)/(73·10) = 4/365.
        (wire::cayley_request(["3:3", "0:0", "0:0"]), 1, 16, cayley(3, [1, 0, 0]), Q5::of(4, 0, 365)),
    ];
    for (request, n, d, expected, distance) in cases {
        let text = String::from_utf8_lossy(&request).into_owned();
        let resolution = rotation::resolve(ctx, None, &free(request, n, d)).expect(&text);
        let Choice::Free { chosen, distance: got, comparisons } = &resolution.choice else {
            panic!("{text}: a free choice");
        };
        assert_eq!((chosen, got), (&expected, &distance), "{text}");
        assert!(*comparisons >= (2 * n as u64 + 1).pow(3) * d as u64, "{text}: the whole grid is compared");
        assert_eq!(*resolution.twist.matrix, expected.matrix(ctx, 0), "{text}");
        assert_eq!(resolution.request_digest, wire::request_digest(&resolution.binding.request));
    }

    // At N = 0 the grid is the identity alone, at every denominator, and no half-turn axis: the search stops at
    // denominator 1, so even the largest admitted D returns at once.
    for d in [2, 1 << 40] {
        for (request, distance) in [
            (b"LX9B1|C|1:1|0:0|0:0\n".to_vec(), Q5::of(8, 0, 5)),
            (wire::half_turn_request(["1:0", "0:0", "0:0"]), Q5::of(8, 0, 1)),
        ] {
            let text = String::from_utf8_lossy(&request).into_owned();
            let resolution = rotation::resolve(ctx, None, &free(request, 0, d)).expect(&text);
            let expected = Choice::Free { chosen: cayley(1, [0, 0, 0]), distance, comparisons: 1 };
            assert_eq!(resolution.choice, expected, "{text}, D={d}");
        }
    }

    // The resolved rotation is `twist.apply`'s input on the jumbling puzzle.
    let resolution = rotation::resolve(ctx, None, &free(wire::cayley_request(["3:3", "0:0", "0:0"]), 1, 16)).unwrap();
    let mut state = State::new(None);
    let solved = state.digest(ctx);
    let outcome = state.apply(ctx, &resolution.twist, None, &[]).expect("no engine error");
    assert_eq!(outcome.status, Status::Admissible);
    assert_ne!(state.digest(ctx), solved);

    // Refusals choose nothing.
    let runs = chooser_runs();
    let menu = pinned();
    let m = wire::menu_request(SET, ORDER, 0);
    assert_eq!(refusal(None, &free(m, 1, 1)), codes::MENU_BINDING_MISMATCH);
    let quarter = wire::cayley_request(["1:0", "0:0", "0:0"]);
    assert_eq!(refusal(Some(&menu), &free(quarter.clone(), 1, 1)), codes::MENU_BINDING_MISMATCH);
    for (n, d) in [(5000, 10), (6000, 1), (0, (1 << 40) + 1), (1, 0), (u128::MAX, 1)] {
        assert_eq!(refusal(None, &free(quarter.clone(), n, d)), codes::ROTATION_SEARCH_BOUND, "N={n}, D={d}");
    }
    let mut far = free(quarter.clone(), 1, 1);
    far.cap = 600;
    assert_eq!(refusal(None, &far), codes::ROTATION_REQUEST_MALFORMED);
    let mut shape = free(quarter.clone(), 1, 1);
    shape.menu = Some(MenuPin { set: SET.into(), order: ORDER.into() });
    assert_eq!(refusal(None, &shape), codes::ROTATION_REQUEST_MALFORMED);
    assert_eq!(refusal(None, &free(b"LX9B1|C|1.0|0:0|0:0\n".to_vec(), 1, 1)), codes::ROTATION_REQUEST_MALFORMED);
    assert_eq!(chooser_runs(), runs, "a refused request runs no chooser");
}

/// CT-KN3-05 (whole: at cap 0 against the pinned I_a menu, direct off-menu requests refuse with
/// `KN2:input.twist_off_menu`, the C request C=(1,0,0) and the H requests H(1,1,0), H(0,1,-1) and H(1,2,3); the
/// menu's half-turns H(1,0,0), H(0,1,0) and H(0,0,1) are the direct members 11, 1 and 10, matched against the fixture's
/// exact matrices, and each one's drag picks it alone at distance zero; the quarter-turn C=(1,0,0) drag
/// scores all 60 items exactly, picks 15 with the complete tie list 15, 29, 40, 43 and records the winner, the ties,
/// the distance and both digests; the H(1,1,0) drag picks 23 with the complete tie list 23, 24, 41, 54 at
/// (9 − 3√5)/2, and the H(1,2,3) drag picks 25 alone at (45 − 15√5)/14; the reversed menu keeps its set digest,
/// changes its order digest to 1a87a420…, refuses old bindings and old M requests with
/// `KN2:input.menu_binding_mismatch`, and its drag picks the smallest new index 16, the old 43) and CT-LG2-16 (part:
/// a menu drag resolves through the kernel call and records its choice; not run: the language half).
#[test]
fn a_drag_scores_the_pinned_order_and_reversing_the_menu_rebinds_it() {
    let ctx = ctx();
    let menu = pinned();
    assert_eq!((menu.identity.as_str(), menu.order.as_str(), menu.items.len()), (SET, ORDER, 60));
    let twists = menu.for_cap(ctx, 0).expect("the menu at cap 0");
    let quarter = wire::cayley_request(["1:0", "0:0", "0:0"]);
    let request = Request::decode(&quarter).expect("canonical");
    let matrix = request.matrix(ctx, 0).expect("a C request");

    assert_eq!(refusal(Some(&menu), &bound(quarter.clone(), Operation::MenuDirect, &menu)), codes::TWIST_OFF_MENU);
    let drag = rotation::resolve(ctx, Some(&menu), &bound(quarter.clone(), Operation::MenuDrag, &menu)).unwrap();
    let Choice::MenuDrag { index, ties, distance } = &drag.choice else { panic!("a drag choice") };
    assert_eq!((*index, ties.as_slice()), (15, &[15, 29, 40, 43][..]));
    for (i, twist) in twists.iter().enumerate() {
        let d = frobenius2(&matrix, &twist.matrix);
        assert!(if ties.contains(&i) { d == *distance } else { d > *distance }, "item {i}");
    }
    assert_eq!(drag.twist.matrix, twists[15].matrix);
    let record = drag.to_json();
    let choice = record.get("choice").expect("a choice");
    assert_eq!(choice.get("index"), Some(&Value::Int(15)));
    let recorded_ties: Vec<i64> =
        choice.get("ties").and_then(Value::as_array).unwrap().iter().flat_map(Value::as_i64).collect();
    assert_eq!(recorded_ties, [15, 29, 40, 43]);
    assert_eq!(choice.get("distance2"), Some(&distance.to_json()));
    let pin = record.get("binding").and_then(|b| b.get("menu")).expect("both digests");
    assert_eq!(
        (pin.get("set"), pin.get("order")),
        (Some(&Value::String(SET.into())), Some(&Value::String(ORDER.into())))
    );
    assert_eq!(record.get("request_digest"), Some(&Value::String(c600_digest::sha256_hex(&quarter))));

    // H requests take the same two paths. The menu holds the half-turns about the frame's three axes (A4's retained
    // items, as the fixture's exact matrices, not `half_turn_matrix`), so each one is a direct member and its own
    // drag's only item at distance zero; no other rational axis is on the menu.
    let members = [(["1:0", "0:0", "0:0"], 11, 47), (["0:0", "1:0", "0:0"], 1, 1), (["0:0", "0:0", "1:0"], 10, 35)];
    for (axis, at, kidx) in members {
        let half = wire::half_turn_request(axis);
        let direct = rotation::resolve(ctx, Some(&menu), &bound(half.clone(), Operation::MenuDirect, &menu)).unwrap();
        assert_eq!((&direct.choice, twists[at].kidx), (&Choice::MenuDirect { index: at }, Some(kidx)), "H{axis:?}");
        assert_eq!(direct.twist.matrix, twists[at].matrix, "H{axis:?}");
        let own = rotation::resolve(ctx, Some(&menu), &bound(half, Operation::MenuDrag, &menu)).unwrap();
        assert_eq!(own.choice, Choice::MenuDrag { index: at, ties: vec![at], distance: Q5::zero() }, "H{axis:?}");
    }
    for axis in [["1:0", "1:0", "0:0"], ["0:0", "1:0", "-1:0"], ["1:0", "2:0", "3:0"]] {
        let half = wire::half_turn_request(axis);
        assert_eq!(
            refusal(Some(&menu), &bound(half, Operation::MenuDirect, &menu)),
            codes::TWIST_OFF_MENU,
            "H{axis:?}"
        );
    }
    // Off-menu drags. For two half-turns at one cap, Σ(R − M)² = 8 − 8(u·v)²: u = (1,1,0)/√2 against the icosahedral
    // axis v = (φ, 1, 1/φ)/2 gives (9 − 3√5)/2, where four items tie exactly, and u = (1,2,3)/√14 against
    // v = (1, 1/φ, φ)/2 gives (45 − 15√5)/14.
    let drags = [
        (["1:0", "1:0", "0:0"], &[23, 24, 41, 54][..], Q5::of(9, -3, 2)),
        (["1:0", "2:0", "3:0"], &[25][..], Q5::of(45, -15, 14)),
    ];
    for (axis, expected_ties, expected_distance) in drags {
        let half = wire::half_turn_request(axis);
        let requested = Request::decode(&half).expect("canonical").matrix(ctx, 0).expect("an H request");
        let pulled = rotation::resolve(ctx, Some(&menu), &bound(half, Operation::MenuDrag, &menu)).unwrap();
        let Choice::MenuDrag { index, ties, distance } = &pulled.choice else { panic!("a drag choice") };
        assert_eq!(
            (*index, ties.as_slice(), distance),
            (expected_ties[0], expected_ties, &expected_distance),
            "H{axis:?}"
        );
        for (i, twist) in twists.iter().enumerate() {
            let d = frobenius2(&requested, &twist.matrix);
            assert!(if ties.contains(&i) { d == *distance } else { d > *distance }, "H{axis:?}, item {i}");
        }
        assert_eq!(pulled.twist.matrix, twists[*index].matrix, "H{axis:?}");
    }

    // Direct members: the identity by C, every item by M; M with a drag, an index past the menu or another menu
    // refuses.
    let identity = wire::cayley_request(["0:0", "0:0", "0:0"]);
    let direct = rotation::resolve(ctx, Some(&menu), &bound(identity, Operation::MenuDirect, &menu)).unwrap();
    let Choice::MenuDirect { index: at } = direct.choice else { panic!("a direct choice") };
    assert_eq!((twists[at].kidx, direct.twist.kidx), (Some(0), Some(0)));
    for (i, twist) in twists.iter().enumerate() {
        let m = wire::menu_request(SET, ORDER, i as u64);
        let item = rotation::resolve(ctx, Some(&menu), &bound(m, Operation::MenuDirect, &menu)).unwrap();
        assert_eq!((&item.choice, &item.twist.matrix), (&Choice::MenuDirect { index: i }, &twist.matrix));
    }
    let m15 = wire::menu_request(SET, ORDER, 15);
    assert_eq!(
        refusal(Some(&menu), &bound(m15.clone(), Operation::MenuDrag, &menu)),
        codes::ROTATION_REQUEST_MALFORMED
    );
    let past = wire::menu_request(SET, ORDER, 60);
    assert_eq!(refusal(Some(&menu), &bound(past, Operation::MenuDirect, &menu)), codes::ROTATION_REQUEST_MALFORMED);
    let other = wire::menu_request(SET, REVERSED_ORDER, 15);
    assert_eq!(refusal(Some(&menu), &bound(other, Operation::MenuDirect, &menu)), codes::MENU_BINDING_MISMATCH);
    assert_eq!(refusal(None, &bound(quarter.clone(), Operation::MenuDrag, &menu)), codes::MENU_BINDING_MISMATCH);

    // Reversing the menu: same set, new order; old bindings and old M requests refuse; the smallest new index wins.
    let rev = reversed();
    assert_eq!((rev.identity.as_str(), rev.order.as_str()), (SET, REVERSED_ORDER));
    for operation in [Operation::MenuDirect, Operation::MenuDrag] {
        assert_eq!(refusal(Some(&rev), &bound(quarter.clone(), operation, &menu)), codes::MENU_BINDING_MISMATCH);
    }
    assert_eq!(refusal(Some(&rev), &bound(m15.clone(), Operation::MenuDirect, &menu)), codes::MENU_BINDING_MISMATCH);
    assert_eq!(refusal(Some(&rev), &bound(m15, Operation::MenuDirect, &rev)), codes::MENU_BINDING_MISMATCH);
    let again = rotation::resolve(ctx, Some(&rev), &bound(quarter, Operation::MenuDrag, &rev)).unwrap();
    let Choice::MenuDrag { index, ties, distance: d } = &again.choice else { panic!("a drag choice") };
    assert_eq!((*index, ties.as_slice(), d), (16, &[16, 19, 30, 44][..], distance));
    assert_eq!(again.twist.matrix, twists[43].matrix);
    assert_ne!(again.twist.matrix, drag.twist.matrix, "the reversed order changes the rotation");
}

/// CT-KN3-06 (part: replaying a recorded free or drag rotation, read back from its canonical text, runs neither the
/// producer nor the chooser and gives the same twist and the same state; producer bytes from binary64 values resolve
/// exactly; a record whose matrix is not a rotation refuses; not run: the shell's gesture producer).
#[test]
fn replay_uses_the_recorded_rotation_without_the_chooser() {
    let ctx = ctx();
    let menu = pinned();
    // The bit patterns of 3/8, -0 and the value nearest 1/10, as the shell's producer would pass them.
    let token = |bits: u64| wire::token_of_binary64(bits).expect("finite");
    let (a, b, c) = (token(0x3fd8_0000_0000_0000), token(0x8000_0000_0000_0000), token(0x3fb9_9999_9999_999a));
    let produced = wire::cayley_request([&a, &b, &c]);
    assert_eq!(produced, b"LX9B1|C|3:3|0:0|3602879701896397:55\n");
    let resolutions = [
        rotation::resolve(ctx, None, &free(produced, 1, 16)).unwrap(),
        rotation::resolve(
            ctx,
            Some(&menu),
            &bound(wire::cayley_request(["1:0", "0:0", "0:0"]), Operation::MenuDrag, &menu),
        )
        .unwrap(),
    ];
    for resolution in resolutions {
        let text = resolution.to_json().canonical();
        let record = json::parse(text.as_bytes()).expect("the record is JSON");
        let runs = chooser_runs();
        let twist = rotation::replay(ctx, &record).expect("the record replays");
        assert_eq!(chooser_runs(), runs, "replay runs no chooser");
        assert_eq!((twist.grip, &twist.matrix, twist.kidx), (0, &resolution.twist.matrix, resolution.twist.kidx));

        let (mut live, mut replayed) = (State::new(None), State::new(None));
        live.apply(ctx, &resolution.twist, None, &[]).expect("no engine error");
        replayed.apply(ctx, &twist, None, &[]).expect("no engine error");
        assert_eq!(live.digest(ctx), replayed.digest(ctx));

        // Two swapped rows make a reflection: the exact check refuses the record.
        let mut tampered = record.clone();
        if let Value::Object(fields) = &mut tampered {
            if let Some(Value::Array(rows)) = fields.get_mut("matrix") {
                rows.swap(0, 1);
            }
        }
        assert!(rotation::replay(ctx, &tampered).is_err());
    }
}

/// CT-LG2-16 (part: on 600-cell-Full every item of the pinned menu at cap 0, requested by M, either dispatches to the
/// retained word of its A4 element, which M4's kernel reads and derives, or refuses with
/// `KN5:input.menu_word_needs_jumble_on`; a free resolution refuses there; not run: the language half and the command
/// layer's dispatch, which consumes this word).
#[test]
fn on_600_cell_full_retained_items_dispatch_and_jumble_items_refuse() {
    let ctx = ctx();
    let kernel = Kernel::load(&repository().join("assets")).expect("the 600-cell-Full model loads");
    let menu = pinned();
    let twists = menu.for_cap(ctx, 0).expect("the menu at cap 0");
    let a4 = ctx.kplus.a4(0);
    let mut retained = 0;
    for (i, twist) in twists.iter().enumerate() {
        let m = wire::menu_request(SET, ORDER, i as u64);
        let resolution = rotation::resolve(ctx, Some(&menu), &bound(m, Operation::MenuDirect, &menu)).unwrap();
        let in_a4 = twist.kidx.is_some_and(|k| a4.contains(&k));
        match rotation::retained_word(ctx, &resolution) {
            Ok(word) => {
                assert!(in_a4, "item {i} is retained");
                retained += 1;
                assert_eq!(ctx.kplus.word_matrix(&word, &ctx.model).as_ref(), Some(&*twist.matrix), "item {i}");
                if word.is_empty() {
                    continue;
                }
                let recipe = kernel.word(&word).expect("the kernel reads the retained word");
                match kernel.derive(&kernel.solved(), &recipe, &ProtectedSet::none()).expect("no certificate fails") {
                    Derivation::Derived(_) => {}
                    Derivation::Refused(refused) => panic!("item {i}: refused: {}", refused.refusal),
                }
            }
            Err(refusal) => {
                assert!(!in_a4, "item {i} is a jumble item");
                assert_eq!(refusal.code, codes::MENU_WORD_NEEDS_JUMBLE_ON, "item {i}");
            }
        }
    }
    assert_eq!(retained, 12, "A4 at cap 0");

    let resolution = rotation::resolve(ctx, None, &free(wire::cayley_request(["0:0", "0:0", "0:0"]), 0, 1)).unwrap();
    assert_eq!(rotation::retained_word(ctx, &resolution).unwrap_err().code, codes::MENU_BINDING_MISMATCH);
}

/// CT-DET-16 (synthetic, part: under A6, identical exact request bytes, operation, bounds and menu binding give
/// identical choices and byte-identical records, resolved again, against a menu built again from the same record and
/// against pinned record digests, the exact four-way ties of the C=(1,0,0) and H(1,1,0) drags included; each record
/// replays to its rotation without running a chooser; the kernel half of the producer refuses non-finite binary64
/// patterns and writes both zeros as `0:0`; not run: the shell's gesture producer, LP3, and a run on another platform)
#[test]
fn identical_requests_give_identical_choices_and_records() {
    let ctx = ctx();
    let (menu, again) = (pinned(), pinned());
    let cases = [
        (false, free(wire::cayley_request(["3:3", "0:0", "1:4"]), 1, 16)),
        (true, bound(wire::cayley_request(["1:0", "0:0", "0:0"]), Operation::MenuDrag, &menu)),
        (true, bound(wire::half_turn_request(["1:0", "1:0", "0:0"]), Operation::MenuDrag, &menu)),
        (true, bound(wire::half_turn_request(["0:0", "1:0", "0:0"]), Operation::MenuDirect, &menu)),
        (true, bound(wire::menu_request(SET, ORDER, 15), Operation::MenuDirect, &menu)),
    ];
    let mut digests = Vec::new();
    for (with_menu, binding) in &cases {
        let resolve = |m: &Menu| rotation::resolve(ctx, with_menu.then_some(m), binding).expect("the request resolves");
        let first = resolve(&menu);
        let text = first.to_json().canonical();
        for other in [resolve(&menu), resolve(&again)] {
            assert_eq!(other.choice, first.choice, "{binding:?}");
            assert_eq!(other.to_json().canonical(), text, "{binding:?}");
        }
        let record = json::parse(text.as_bytes()).expect("the record is JSON");
        let runs = chooser_runs();
        let twist = rotation::replay(ctx, &record).expect("the record replays");
        assert_eq!((chooser_runs(), &twist.matrix), (runs, &first.twist.matrix), "{binding:?}");
        digests.push(c600_digest::sha256_hex(text.as_bytes()));
    }
    assert_eq!(digests, RECORD_DIGESTS);

    // The kernel half of the shell's producer: the infinities and a NaN refuse, and both zeros give one token.
    for bits in [0x7ff0_0000_0000_0000_u64, 0xfff0_0000_0000_0000, 0x7ff8_0000_0000_0000] {
        let refused = wire::token_of_binary64(bits).expect_err("a non-finite value has no token");
        assert_eq!(refused.code, codes::ROTATION_REQUEST_MALFORMED);
    }
    assert_eq!(wire::token_of_binary64(0x8000_0000_0000_0000).unwrap(), "0:0");
    assert_eq!(wire::token_of_binary64(0).unwrap(), "0:0");
}
