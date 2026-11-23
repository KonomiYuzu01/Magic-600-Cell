//! Letter-by-letter play of stage 18's step words and the strict′ contract (shared by gates 2 and 3).
//!
//! Every letter is applied by the jumbling engine through [`crate::replay::Jumbling`]: the I_a item transported to its
//! cap by the retained frame, classified exactly, applied only when admissible. Nothing is composed, cached or carried
//! between checks.
//!
//! The strict′ contract of a step from start S to end T with unit U (tutorial chapter 5, section 5.7.1): every piece of
//! U is on the lattice at T; no piece on the lattice at S is off at T (no collateral); every other piece off at S is,
//! at T, either off at its exact pose at S or on the lattice. With the record's off counts (which drop by exactly the
//! unit size), no other piece lands.
//!
//! A start table that is not stored (every step after the first) is rebuilt by exact replay of the earlier steps'
//! stored words from the stored start table, as a new derivation under the checking sub-checker's own identity (the
//! owner's LP5-3, 23 November 2026; CX.17). The trace marks it `derived`; it is never presented as a table of the
//! archive.

use c600_jumble::model::NP;
use c600_jumble::state::{State, Status};

use super::words::letter_of;
use crate::claims::{reject, Run, Step};
use crate::replay::Jumbling;

/// Whether a letter must be a SAFE-ALL letter (the composed gate's V′ spans) or only an admissible twist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Letter {
    Twist,
    Safe,
}

/// Plays one letter. A twist must be admissible. A SAFE-ALL letter (stage 18 README section 3.3; tutorial chapter 5,
/// section 5.7.2) must also act on its cap as a nonidentity element of K⁺ and move only pieces on the lattice, so it
/// keeps every off piece and its pose.
pub fn play(j: &mut Jumbling, code: u16, kind: Letter, claim: &str) -> Step<()> {
    let (cap, item) = letter_of(code);
    let twist = j.twist(cap, item, claim)?;
    if kind == Letter::Safe && !matches!(twist.kidx, Some(k) if k != 0) {
        return Err(reject(claim, format!("[{cap}, {item}] is not a nonidentity retained letter, so not SAFE-ALL")));
    }
    let outcome = j.apply(&twist, claim)?;
    if outcome.status != Status::Admissible {
        return Err(reject(claim, format!("[{cap}, {item}] is {}", outcome.status.as_str())));
    }
    if kind == Letter::Safe {
        if let Some(&piece) = outcome.inside.iter().find(|&&p| !j.state.is_lattice_piece(p as usize)) {
            return Err(reject(claim, format!("[{cap}, {item}] holds the off piece {piece}, so it is not SAFE-ALL")));
        }
    }
    Ok(())
}

/// The strict′ fields of a step (stage 12's `stepchk.py` report, as tutorial chapter 5 section 5.7.1 defines them).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrictPrime {
    pub unit: u64,
    pub unit_on_lattice: u64,
    pub collateral: u64,
    pub others: u64,
    pub others_exact: u64,
    pub others_landed: u64,
    pub off_before: u64,
    pub off_after: u64,
}

impl StrictPrime {
    pub fn holds(&self) -> bool {
        self.unit_on_lattice == self.unit
            && self.collateral == 0
            && self.others_exact + self.others_landed == self.others
    }
}

impl std::fmt::Display for StrictPrime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unit_on_lattice {}/{}, collateral {}, others_exact {} + others_landed {} of {}, off {} -> {}",
            self.unit_on_lattice,
            self.unit,
            self.collateral,
            self.others_exact,
            self.others_landed,
            self.others,
            self.off_before,
            self.off_after
        )
    }
}

/// The strict′ fields between a start and an end state, recomputed over every piece.
pub fn strict_prime(start: &State, end: &State, unit: &[u32]) -> StrictPrime {
    let mut in_unit = vec![false; NP];
    for &p in unit {
        in_unit[p as usize] = true;
    }
    let mut sp = StrictPrime {
        unit: unit.len() as u64,
        unit_on_lattice: 0,
        collateral: 0,
        others: 0,
        others_exact: 0,
        others_landed: 0,
        off_before: 0,
        off_after: 0,
    };
    for piece in 0..NP {
        let (was, is) = (start.is_lattice_piece(piece), end.is_lattice_piece(piece));
        sp.off_before += u64::from(!was);
        sp.off_after += u64::from(!is);
        if in_unit[piece] {
            sp.unit_on_lattice += u64::from(is);
        } else if was {
            sp.collateral += u64::from(!is);
        } else {
            sp.others += 1;
            if is {
                sp.others_landed += 1;
            } else if start.pose_of(piece) == end.pose_of(piece) {
                sp.others_exact += 1;
            }
        }
    }
    sp
}

/// Checks the strict′ contract and the record's off count after a step, under the step's claim path.
pub fn check_strict(run: &mut Run, path: &str, sp: &StrictPrime, off_after: u64) -> Step<()> {
    run.claim(&format!("{path}.off_after"), &off_after, &sp.off_after)?;
    if !sp.holds() {
        let stop = reject(format!("{path}.strict_prime"), format!("the step is not strict′: {sp}"));
        return Err(run.failed(stop, sp));
    }
    run.held(format!("{path}.strict_prime"), sp);
    Ok(())
}

/// Checks that every unit piece is off at the step's start and the start's off count.
pub fn check_start(run: &mut Run, path: &str, state: &State, unit: &[u32], off_before: u64) -> Step<()> {
    let off = state.off_lattice_count() as u64;
    run.claim(&format!("{path}.off_before"), &off_before, &off)?;
    if let Some(&piece) = unit.iter().find(|&&p| state.is_lattice_piece(p as usize)) {
        return Err(reject(
            format!("{path}.unit"),
            format!("the unit piece {piece} is on the lattice at the step's start"),
        ));
    }
    run.held(format!("{path}.unit"), format!("{} pieces, all off at the start", unit.len()));
    Ok(())
}

/// The trace text of a step's start table: stored, or derived by exact replay under this checker (LP5-3).
pub fn start_note(k: usize, digest: &str) -> String {
    if k == 0 {
        format!("stored: {digest}")
    } else {
        format!(
            "derived: {digest}, rebuilt by exact replay of the stored words of steps[0] to steps[{}] from the stored \
             start table under this checker's identity (LP5-3); a new derivation, not a table of the archive",
            k - 1
        )
    }
}
