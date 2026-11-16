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
/// N=0, D=1 gives the identity at squared distance 8/5; M refuses with `KN2:input.menu_binding_mismatch`; bounds
/// outside the search limit refuse with `KN2:resource_limit.rotation_search_bound` with no search; the decoding half
/// is the unit test `rotation::tests`; not run: a search near the limit, which is research-only and unbudgeted).
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

/// CT-KN3-05 (whole: at cap 0 against the pinned I_a menu, a direct off-menu C request refuses with
/// `KN2:input.twist_off_menu`; the quarter-turn C=(1,0,0) drag scores all 60 items exactly, picks 15 with the complete
/// tie list 15, 29, 40, 43 and records the winner, the ties, the distance and both digests; the reversed menu keeps
/// its set digest, changes its order digest to 1a87a420…, refuses old bindings and old M requests with
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
