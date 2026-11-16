//! JI.53 at the candidate's observation tables (LP2, M5; the coset side of the Fable solver's lead L10 of 16 November
//! 2026). A domain ID names the coset g K+ of a pose g, so two poses g and h have equal IDs exactly when g^-1 h lies in
//! K+, and the side matters: g k keeps g's ID for every k in K+, while k g keeps it only when g^-1 k g lies in K+. The
//! pose is CT-KN2-02's free-mode vector, the Cayley rotation omega = (2, 0, 0) at cap 0. CT-KN2-02 (part: the
//! free-mode pose lies outside K+ and its domain ID is the exact coset key, unequal to the identity's; not run here:
//! its heights and the agreement with `trees.py` and the library, which `tests/test_jumbling_rust_candidate.py` runs).

use std::path::Path;

use c600_jumble::q5::{matmul, transpose, Q5};
use c600_jumble::{observe, twist, Context};

#[test]
fn a_domain_id_names_the_left_coset() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("assets");
    let ctx = Context::load(&assets).expect("the jumbling model loads");
    let tables = observe::tables(&ctx).expect("the observation tables");
    let omega = [Q5::int(2), Q5::int(0), Q5::int(0)];
    let g = twist::cayley_matrix(&ctx.model.poles[0], &omega, &ctx.model.nn);
    assert!(ctx.kplus.index_of_matrix(&g, &ctx.model).is_none(), "R(omega) lies outside K+");
    let id = tables.domain_id(&g);
    assert_ne!(id, tables.lattice_key());
    // A4 at pole 0 (the identity among them), the generators and a stride through the whole group.
    let mut sample: Vec<u16> = ctx.kplus.stab0.iter().chain(&ctx.kplus.gen_idx).copied().collect();
    sample.extend((0..ctx.kplus.perms.len()).step_by(720).map(|k| k as u16));
    let (mut kept, mut moved) = (0, 0);
    for k in sample {
        let m = ctx.kplus.matrix(k, &ctx.model);
        assert_eq!(tables.domain_id(&matmul(&g, m)), id, "g k, k = {k}");
        // g is a rotation, so its transpose is its inverse.
        let conjugate = matmul(&transpose(&g), &matmul(m, &g));
        let in_kplus = ctx.kplus.index_of_matrix(&conjugate, &ctx.model).is_some();
        assert_eq!(tables.domain_id(&matmul(m, &g)) == id, in_kplus, "k g, k = {k}");
        if in_kplus {
            kept += 1;
        } else {
            moved += 1;
        }
    }
    assert!(kept > 0 && moved > 0, "{kept} elements keep the ID from the left, {moved} move it");
}
