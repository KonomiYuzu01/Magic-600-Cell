//! LP2, two kernel pieces that LP3's C8 (the jumbling tool) waits for (`docs/progress/1.0/language-lp3-plan.md`): the
//! slot action of K⁺ (`levels::Tables::slot_image` and `piece_slots`; state contract A3), which places a piece at a
//! lattice pose other than the identity, and the engine-named rotation plane of a twist (`twist::rotation_plane`;
//! render/1 section 4, RM.6), which a swept preview needs. Everything is checked exactly. Nothing here chooses a twist:
//! every word and element below is fixed in the test.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_jumble::kplus::ORDER;
use c600_jumble::levels::{self, Partial};
use c600_jumble::model::{NP, NS};
use c600_jumble::q5::{dot, identity4, matvec, rank, Mat4, Vec4, Q5};
use c600_jumble::state::{State, Status};
use c600_jumble::twist::{rotation_plane, Menu, RotationPlane, Twist};
use c600_jumble::{observe, Context};
use c600_kernel::json;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn ctx() -> &'static Context {
    static CTX: OnceLock<Context> = OnceLock::new();
    CTX.get_or_init(|| Context::load(&repository().join("assets")).expect("the jumbling model loads"))
}

fn i_a(ctx: &Context) -> Arc<Menu> {
    let bytes = std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).expect("the I_a fixture");
    let doc = json::parse(&bytes).expect("the I_a fixture is JSON");
    Arc::new(Menu::from_record(ctx, doc.get("menu").expect("a menu")).expect("the pinned I_a menu"))
}

// ------------------------------------------------------------------------------------------------- slot action

/// A3's slot action is a left action of K⁺ on the 259,800 slots: the identity fixes every slot, each element permutes
/// them, and `slot_image(compose(a, b), s)` is `slot_image(a, slot_image(b, s))`. The elements are the 12 of the
/// stabiliser of pole 0, the first six retained generators, an inverse and two products.
#[test]
fn the_slot_action_is_a_left_action() {
    let ctx = ctx();
    let t = levels::tables(ctx).expect("the level tables");
    let kp = &ctx.kplus;
    let mut elements: Vec<u16> = kp.stab0.clone();
    elements.extend(kp.gen_idx.iter().take(6));
    elements.push(kp.inv[kp.gen_idx[5] as usize]);
    elements.push(kp.compose(kp.gen_idx[0], kp.gen_idx[7]));
    elements.push(kp.compose(kp.gen_idx[301], kp.gen_idx[2]));
    let images: Vec<Vec<u32>> = elements
        .iter()
        .map(|&k| {
            let image: Vec<u32> = (0..NS as u32).map(|s| t.slot_image(ctx, s, k).expect("an image slot")).collect();
            let mut seen = vec![false; NS];
            for &d in &image {
                assert!(!std::mem::replace(&mut seen[d as usize], true), "element {k} carries two slots to {d}");
            }
            image
        })
        .collect();
    let identity = elements.iter().position(|&k| k == 0).expect("the stabiliser holds the identity");
    assert!(images[identity].iter().enumerate().all(|(s, &d)| d as usize == s), "the identity moves a slot");
    assert!(images.iter().filter(|image| image.iter().enumerate().any(|(s, &d)| d as usize != s)).count() > 12);
    for (i, &a) in elements.iter().enumerate().take(8) {
        for (j, &b) in elements.iter().enumerate().skip(10) {
            let ab = kp.compose(a, b);
            for s in (0..NS as u32).step_by(7) {
                let expected = images[i][images[j][s as usize] as usize];
                assert_eq!(t.slot_image(ctx, s, ab), Ok(expected), "slot {s} under {a} after {b}");
            }
        }
    }
}

/// The slot action puts every sticker where the label projection π(X) puts it: for each piece at a lattice pose k,
/// entry i of `piece_slots(p, k)` is the slot whose label is `slots(p)[i]`. The state is a fixed retained word from
/// solved under the I_a menu, so many pieces sit at poses other than the identity.
#[test]
fn the_slot_action_agrees_with_the_labels() {
    let ctx = ctx();
    let t = levels::tables(ctx).expect("the level tables");
    let mut state = State::new(Some(i_a(ctx)));
    for mid in [1, 3, -5, 2, -7, 11] {
        let twist = levels::primitive(ctx, mid).expect("a primitive id");
        assert_eq!(state.apply(ctx, &twist, None, &[]).expect("no engine error").status, Status::Admissible);
    }
    let Partial::Total(labels) = levels::labels(ctx, &state).expect("no engine error") else {
        panic!("a retained word from solved stays on the lattice");
    };
    let mut turned = 0;
    for p in 0..NP {
        let k = state.pose_of(p).kidx().expect("a lattice pose");
        turned += usize::from(k != 0);
        let placed = t.piece_slots(ctx, p, k).expect("the piece's slots");
        assert_eq!(placed.len(), t.slots(p).len());
        for (&home, &slot) in t.slots(p).iter().zip(&placed) {
            assert_eq!(labels.labels[slot as usize], home as i32, "piece {p}, home slot {home}");
            assert_eq!(t.slot_image(ctx, home, k), Ok(slot));
        }
    }
    assert!(turned > 1000, "only {turned} pieces turned");
}

/// A slot outside 0..NS, a piece outside 0..NP or an element outside K⁺ is an engine error, never a panic.
#[test]
fn the_slot_action_rejects_what_is_out_of_range() {
    let ctx = ctx();
    let t = levels::tables(ctx).expect("the level tables");
    assert!(t.slot_image(ctx, NS as u32, 1).is_err());
    assert!(t.slot_image(ctx, 0, ORDER as u16).is_err());
    assert!(t.piece_slots(ctx, NP, 0).is_err());
    assert!(t.piece_slots(ctx, 0, ORDER as u16).is_err());
    assert_eq!(t.piece_slots(ctx, 0, 0).expect("the identity pose"), t.slots(0));
}

// ----------------------------------------------------------------------------------------------- rotation plane

fn nonzero(v: &Vec4) -> bool {
    v.iter().any(|x| !x.is_zero())
}

fn combine(a: &Q5, u: &Vec4, b: &Q5, v: &Vec4) -> Vec4 {
    std::array::from_fn(|i| a.mul(&u[i]).add(&b.mul(&v[i])))
}

/// rank(g − I).
fn moved_rank(g: &Mat4) -> usize {
    let rows: Vec<Vec<Q5>> = (0..4)
        .map(|i| (0..4).map(|j| if i == j { g[i][j].sub(&Q5::one()) } else { g[i][j].clone() }).collect())
        .collect();
    rank(&rows)
}

/// R(θ) of render/1 section 4 from the unnormalized plane: with c = cos θ,
/// R = I + (c − 1)(uuᵀ/u·u + vvᵀ/v·v) + sin θ·(vuᵀ − uvᵀ)/(|u||v|). For θ < π, |v| = sin θ·|u|, so the last
/// factor is 1/u·u and R is exact; for a half-turn the last term vanishes.
fn rebuild(plane: &RotationPlane) -> Mat4 {
    let (u, v, c) = (&plane.u, &plane.v, &plane.cos);
    let (uu, vv) = (dot(u, u), dot(v, v));
    let c1 = c.sub(&Q5::one());
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let delta = if i == j { Q5::one() } else { Q5::zero() };
            let projector = u[i].mul(&u[j]).div(&uu).expect("u ≠ 0").add(&v[i].mul(&v[j]).div(&vv).expect("v ≠ 0"));
            let entry = delta.add(&c1.mul(&projector));
            if plane.is_half_turn() {
                entry
            } else {
                entry.add(&v[i].mul(&u[j]).sub(&u[i].mul(&v[j])).div(&uu).expect("u ≠ 0"))
            }
        })
    })
}

/// The exact facts of a plane named for the rotation g.
fn check(g: &Mat4, plane: &RotationPlane, what: &str) {
    let (u, v, c) = (&plane.u, &plane.v, &plane.cos);
    assert!(nonzero(u) && nonzero(v), "{what}: a zero vector");
    assert!(dot(u, v).is_zero(), "{what}: u·v ≠ 0");
    let first = (0..4).find(|&j| (0..4).any(|i| g[i][j] != if i == j { Q5::one() } else { Q5::zero() }));
    let column: Vec4 = std::array::from_fn(|i| {
        let j = first.expect("g is not the identity");
        if i == j {
            g[i][j].sub(&Q5::one())
        } else {
            g[i][j].clone()
        }
    });
    assert_eq!(u, &column, "{what}: u is not the first nonzero column of g − I");
    let minus = Q5::int(-1);
    if plane.is_half_turn() {
        assert_eq!(matvec(g, u), combine(&minus, u, &Q5::zero(), v), "{what}: g u ≠ −u");
        assert_eq!(matvec(g, v), combine(&Q5::zero(), u, &minus, v), "{what}: g v ≠ −v");
    } else {
        assert_eq!(matvec(g, u), combine(c, u, &Q5::one(), v), "{what}: g u ≠ cos θ·u + v");
        let sin2 = Q5::one().sub(&c.mul(c));
        assert_eq!(sin2.sign(), 1, "{what}: θ is not in (0, π)");
        assert_eq!(dot(v, v), sin2.mul(&dot(u, u)), "{what}: |v| ≠ sin θ·|u|");
    }
    assert_eq!(&rebuild(plane), g, "{what}: R(θ) ≠ g");
}

/// Every element of K⁺ that turns one plane gets a plane that rebuilds it exactly through R(θ); the identity and the
/// rotations that turn two planes get none. The elements are the 12 of the stabiliser of pole 0, which hold three
/// half-turns, the elements of the retained generators, each of which fixes its grip's pole and so turns one plane,
/// and every seventh element of K⁺.
#[test]
fn k_plus_elements_that_turn_one_plane_get_a_plane_that_rebuilds_them() {
    let ctx = ctx();
    let kp = &ctx.kplus;
    let mut elements: Vec<u16> = kp.stab0.iter().chain(&kp.gen_idx).copied().collect();
    elements.extend((0..ORDER as u16).step_by(7));
    elements.sort_unstable();
    elements.dedup();
    let (mut simple, mut half, mut double) = (0, 0, 0);
    for &k in &elements {
        let g = kp.matrix(k, &ctx.model);
        match (rotation_plane(g), moved_rank(g)) {
            (None, 0) => assert_eq!(k, 0, "only the identity fixes everything"),
            (None, 4) => double += 1,
            (Some(plane), 2) => {
                check(g, &plane, &format!("K+ element {k}"));
                simple += 1;
                half += usize::from(plane.is_half_turn());
            }
            (plane, r) => panic!("K+ element {k}: rank(g − I) = {r} with plane {plane:?}"),
        }
    }
    let stab0_planes = kp.stab0.iter().filter(|&&k| rotation_plane(kp.matrix(k, &ctx.model)).is_some()).count();
    assert_eq!(stab0_planes, 11, "every rotation fixing pole 0 but the identity has a plane");
    let stab0_half =
        kp.stab0.iter().filter_map(|&k| rotation_plane(kp.matrix(k, &ctx.model))).filter(|p| p.is_half_turn());
    assert_eq!(stab0_half.count(), 3, "A4 holds three half-turns");
    let unplaned: Vec<u16> =
        kp.gen_idx.iter().copied().filter(|&k| rotation_plane(kp.matrix(k, &ctx.model)).is_none()).collect();
    assert!(unplaned.is_empty(), "retained generators without a plane: {unplaned:?}");
    assert!(half >= 3 && double > 0, "simple {simple}, half {half}, double {double}");
}

/// A twist's plane is the plane of its matrix: a retained generator at grip 0 and an exact Cayley rotation off the
/// lattice (ω = (2, 0, 0), CT-KN1-06's Q) each get one that rebuilds them, and the identity twist gets none.
#[test]
fn a_twist_gets_the_plane_of_its_rotation() {
    let ctx = ctx();
    let h0 = levels::primitive(ctx, 1).expect("retained generator 1");
    let plane = h0.plane().expect("H_0 turns one plane");
    check(&h0.matrix, &plane, "H_0");
    assert_eq!(rotation_plane(&h0.matrix), Some(plane));

    let omega = [Q5::int(2), Q5::zero(), Q5::zero()];
    let cayley = Twist::from_record(ctx, &observe::cayley_record(ctx, 0, &omega)).expect("the Cayley record reads");
    assert_eq!(cayley.kidx, None, "the Cayley rotation lies off the lattice");
    let plane = cayley.plane().expect("a Cayley rotation turns one plane");
    assert!(!plane.is_half_turn());
    check(&cayley.matrix, &plane, "the Cayley rotation");
    assert!(dot(&plane.u, &ctx.model.poles[0]).is_zero() && dot(&plane.v, &ctx.model.poles[0]).is_zero());

    let identity = Twist::new(ctx, 0, identity4()).expect("the identity is a twist");
    assert_eq!(identity.plane(), None);
}

/// A matrix that is not exactly a rotation has no plane, even when it fixes a plane pointwise.
#[test]
fn a_matrix_that_is_not_a_rotation_has_no_plane() {
    let mut reflection = identity4();
    reflection[3][3] = Q5::int(-1);
    assert_eq!(rotation_plane(&reflection), None);
    let mut scaled = identity4();
    scaled[2][2] = Q5::int(2);
    scaled[3][3] = Q5::of(1, 0, 2);
    assert_eq!(rotation_plane(&scaled), None);
}
