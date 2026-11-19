//! Fixture types for RM.37's jumbling vectors (CT-RM-07, RM.34's JI.41 crosswalk). They state source-contract
//! outcomes over supplied producer data that is already validated: no jumbling geometry, kernel or backend is built
//! here (M5). Each keeps what JI.41 says a display keeps: complete payloads, explicit incompleteness and undefined
//! results, scoped negatives and fixed orders. None of them selects, ranks, filters or certifies anything.

use c600_digest::sha256_hex;
use c600_kernel::IdentityBlock;
use c600_kernel::Puzzle;

use crate::error::{codes, Error, Form};
use crate::pick::Partial;
use crate::plan::GripStatus;
use crate::validate::CELLS;

/// A synthetic jumbling identity block for the crosswalk fixtures. It names no model: the jumbling model identity
/// arrives with M5.
pub fn fixture_identity() -> IdentityBlock {
    IdentityBlock {
        puzzle: Puzzle::Jumbling,
        model_id: sha256_hex(b"fixture:jumbling-model"),
        menu_id: Some(sha256_hex(b"fixture:menu")),
        menu_order: Some(sha256_hex(b"fixture:menu-order")),
        contract_rev: Some("fixture".to_owned()),
    }
}

/// The state of a grip survey (J04): complete only when all 600 grips carry a certified status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SurveyState {
    Complete,
    /// Incomplete: the grips still uncertain and the number of grips not surveyed.
    Pending {
        uncertain: Vec<u32>,
        missing: usize,
    },
}

/// The survey's state. An uncertain grip stays uncertain; nothing rounds it to a or b.
pub fn survey_state(survey: &[GripStatus]) -> SurveyState {
    let uncertain: Vec<u32> =
        (0u32..).zip(survey).filter(|(_, status)| **status == GripStatus::Uncertain).map(|(grip, _)| grip).collect();
    let missing = (CELLS as usize).saturating_sub(survey.len());
    if uncertain.is_empty() && missing == 0 {
        SurveyState::Complete
    } else {
        SurveyState::Pending { uncertain, missing }
    }
}

/// One seam mismatch of J08 with its domain and cell identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seam {
    pub domain: u32,
    pub cell: u32,
    pub on_grip: bool,
}

/// A seam table (J07/J08): every entry the producer supplied, and whether its all-scope enumeration completed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeamTable {
    pub seams: Vec<Seam>,
    pub complete: bool,
}

impl SeamTable {
    /// The total mismatch count N, on-grip and off-grip together; undefined for an interrupted enumeration.
    pub fn total(&self) -> Partial<u64> {
        if self.complete {
            Partial::Total(self.seams.len() as u64)
        } else {
            Partial::Undefined("the enumeration stopped before full coverage".to_owned())
        }
    }
}

/// A completed cover table at one cap (J09): all meeting caps, in cap-ID order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverTable {
    pub cap: u32,
    rows: Vec<u32>,
}

impl CoverTable {
    /// The table as supplied; rows out of cap-ID order or repeated are refused, never reordered or dropped.
    pub fn new(cap: u32, rows: Vec<u32>) -> Result<CoverTable, Error> {
        if rows.windows(2).any(|w| w[0] >= w[1]) {
            return Err(Error::invalid("cover rows are not in cap-ID order without repeats"));
        }
        Ok(CoverTable { cap, rows })
    }

    pub fn rows(&self) -> &[u32] {
        &self.rows
    }
}

/// A repair list of J09 at one cap and input: completed with its repairs, or cut off.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RepairList {
    Completed { cap: u32, input: String, repairs: Vec<(u32, u32)> },
    CutOff { cap: u32, input: String },
}

impl RepairList {
    /// The negative a list supports: only a completed empty list, scoped to its cap and input. A cut-off list supplies
    /// no completed negative, and no list supplies a global one.
    pub fn negative(&self) -> Option<String> {
        match self {
            RepairList::Completed { cap, input, repairs } if repairs.is_empty() => {
                Some(format!("no repair at cap {cap} for input {input}"))
            }
            _ => None,
        }
    }
}

/// The check log of a release (J11): checks keep their tried order, a release appends only after it completes, and a
/// new revision clears the old revision's checks. It holds no permission flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseLog {
    revision: u64,
    entries: Vec<String>,
}

impl ReleaseLog {
    pub fn new(revision: u64) -> ReleaseLog {
        ReleaseLog { revision, entries: Vec::new() }
    }

    /// The revision changes: the checks of the old revision no longer hold.
    pub fn set_revision(&mut self, revision: u64) {
        if revision != self.revision {
            self.revision = revision;
            self.entries.clear();
        }
    }

    /// A completed check at `revision`; a check of another revision is not recorded.
    pub fn check(&mut self, revision: u64, name: &str) -> bool {
        self.append(revision, name, true)
    }

    /// A release appends only once completed, at the current revision.
    pub fn release(&mut self, revision: u64, name: &str, completed: bool) -> bool {
        self.append(revision, name, completed)
    }

    fn append(&mut self, revision: u64, name: &str, completed: bool) -> bool {
        if !completed || revision != self.revision {
            return false;
        }
        self.entries.push(name.to_owned());
        true
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
}

/// The shipped rows of the fixed correction catalogue (J12).
pub const CORRECTIONS: usize = 36;

/// One correction row: its fixed ID and its nu effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Correction {
    pub id: u32,
    pub effect: Vec<i64>,
}

/// The fixed correction catalogue: every row, in ID order, whatever the state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalogue {
    rows: Vec<Correction>,
    /// The modulus of each nu coordinate, in coordinate order.
    moduli: Vec<i64>,
    /// The exponent of the finite group the nu coordinates live in: the least common multiple of [`Catalogue::moduli`].
    /// It is derived from them, and [`Catalogue::predict`] does not read it.
    pub modulus: i64,
}

impl Catalogue {
    /// A synthetic catalogue of 36 rows over a nu vector of `width` coordinates, each modulo `modulus`.
    pub fn fixture(width: usize, modulus: i64) -> Catalogue {
        let rows = (0..CORRECTIONS as u32)
            .map(|id| Correction { id, effect: (0..width).map(|c| (i64::from(id) + c as i64) % modulus).collect() })
            .collect();
        Catalogue { rows, moduli: vec![modulus; width], modulus }
    }

    /// A catalogue as supplied, over coordinates with their own moduli (JI.13: F2 for the bits, F5 for the C5
    /// values). It must hold the 36 rows with IDs 0 to 35 in order, each effect one residue in 0..m per coordinate of
    /// modulus m, and every modulus at least 2. Anything else is refused with `EV3:input.display_plan_invalid`, never
    /// reordered, reduced or padded.
    pub fn new(rows: Vec<Correction>, moduli: Vec<i64>) -> Result<Catalogue, Error> {
        if rows.len() != CORRECTIONS {
            return Err(Error::invalid(format!("the catalogue has {} rows, not {CORRECTIONS}", rows.len())));
        }
        if let Some((index, row)) = rows.iter().enumerate().find(|(index, row)| row.id as usize != *index) {
            return Err(Error::invalid(format!(
                "row {index} has ID {}: the rows are not IDs 0 to 35 in order",
                row.id
            )));
        }
        if let Some(&m) = moduli.iter().find(|&&m| m < 2) {
            return Err(Error::invalid(format!("a coordinate modulus is {m}, below 2")));
        }
        for row in &rows {
            if row.effect.len() != moduli.len() {
                let (id, width) = (row.id, row.effect.len());
                return Err(Error::invalid(format!("row {id} has {width} coordinates, not {}", moduli.len())));
            }
            if let Some((c, &e)) = row.effect.iter().enumerate().find(|&(c, &e)| !(0..moduli[c]).contains(&e)) {
                let m = moduli[c];
                return Err(Error::invalid(format!("row {}: coordinate {c} is {e}, not a residue in 0..{m}", row.id)));
            }
        }
        let modulus = moduli
            .iter()
            .try_fold(1i64, |l, &m| (l / gcd(l, m)).checked_mul(m))
            .ok_or_else(|| Error::invalid("the least common multiple of the moduli does not fit 64 bits"))?;
        Ok(Catalogue { rows, moduli, modulus })
    }

    /// All rows in fixed ID order. It takes no state, so it cannot filter, light or reorder by state.
    pub fn rows(&self) -> &[Correction] {
        &self.rows
    }

    /// The modulus of each coordinate, in coordinate order.
    pub fn moduli(&self) -> &[i64] {
        &self.moduli
    }

    /// The predicted nu of the caller's multiplicities: the finite-group sum of JI.13, each coordinate modulo its own
    /// modulus. It selects nothing.
    pub fn predict(&self, multiplicities: &[i64]) -> Vec<i64> {
        let mut nu = vec![0; self.moduli.len()];
        for (row, &m) in self.rows.iter().zip(multiplicities) {
            for ((n, &e), &modulus) in nu.iter_mut().zip(&row.effect).zip(&self.moduli) {
                let sum = i128::from(*n) + i128::from(m.rem_euclid(modulus)) * i128::from(e);
                *n = sum.rem_euclid(i128::from(modulus)) as i64;
            }
        }
        nu
    }
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// A nu observation of J12.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NuObservation {
    OffLattice,
    /// A lattice result and the indices of its fixed I_a coordinates.
    Lattice {
        nu: Vec<i64>,
        fixed: Vec<usize>,
    },
}

/// The nu result: `undefined(off_lattice)` off the lattice, never a zero or an older value; a nonzero fixed
/// coordinate is `KN2:consistency.fixed_nu_nonzero` in form `freeze`, which no display shows as a valid state.
pub fn nu(observation: &NuObservation) -> Result<Partial<Vec<i64>>, Error> {
    match observation {
        NuObservation::OffLattice => Ok(Partial::Undefined("off_lattice".to_owned())),
        NuObservation::Lattice { nu, fixed } => {
            if let Some(&index) = fixed.iter().find(|&&i| nu.get(i).is_some_and(|&v| v != 0)) {
                return Err(Error::new(codes::FIXED_NU_NONZERO, "a fixed I_a coordinate of nu is nonzero")
                    .with("coordinate", index)
                    .in_form(Form::Freeze));
            }
            Ok(Partial::Total(nu.clone()))
        }
    }
}

/// Certified exposed-face geometry (J10): available only with its certified data and size profile.
pub fn exposed_geometry(certified: Option<&[u8]>, profile: Option<&str>) -> Result<String, Error> {
    match (certified, profile) {
        (Some(bytes), Some(_)) => Ok(sha256_hex(bytes)),
        (bytes, profile) => {
            let missing = match (bytes, profile) {
                (None, None) => "geometry and size profile",
                (None, Some(_)) => "geometry",
                _ => "size profile",
            };
            Err(Error::new(codes::EXPOSED_GEOMETRY_UNAVAILABLE, "certified exposed-face geometry is unavailable")
                .with("feature", "J10")
                .with("missing", missing))
        }
    }
}
