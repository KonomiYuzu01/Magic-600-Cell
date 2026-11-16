//! EVD.14: the four versioned tables a word reads besides the facts: the order on grades, the invalid pairs, the
//! threshold and the import table.

use std::collections::BTreeSet;

use crate::model::{Basis, Check, Grade};

/// EVD.19's order on basis values: sample lies below theorem, certificate, exhaustive computation and exact
/// computation, which are pairwise incomparable. A value outside the enumeration is comparable only with itself.
pub fn basis_le(a: &Basis, b: &Basis) -> bool {
    a == b || (*a == Basis::Sample && b.is_known() && *b != Basis::Sample)
}

/// EVD.19's order on check values: none lies below producer, and producer below independent checker, model review,
/// human review and formal proof, which are pairwise incomparable.
pub fn check_le(a: &Check, b: &Check) -> bool {
    if a == b {
        return true;
    }
    match a {
        Check::None => b.is_known(),
        Check::Producer => {
            matches!(b, Check::IndependentChecker | Check::ModelReview | Check::HumanReview | Check::FormalProof)
        }
        _ => false,
    }
}

/// EVD.19: the product order.
pub fn product_le(a: &Grade, b: &Grade) -> bool {
    basis_le(&a.basis, &b.basis) && check_le(&a.check, &b.check)
}

/// EVD.29: U0 = {(b, k) : b != sample, k != none}.
pub fn u0() -> BTreeSet<Grade> {
    Grade::all().into_iter().filter(|g| g.basis != Basis::Sample && g.check != Check::None).collect()
}

/// Why an edition was not admitted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditionError {
    /// EVD.29: the threshold must lie inside U0.
    ThresholdOutsideU0(Grade),
    /// EVD.29: the threshold must be an up-set of P.
    ThresholdNotUpSet { member: Grade, above: Grade },
    /// A threshold or order entry outside P.
    OutsideP(Grade),
}

/// A pinned edition of the four tables (EVD.14).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edition {
    name: String,
    grades: BTreeSet<Grade>,
    /// `None` is EVD.19's product order. `Some` lists extra pairs (a, b) with a <= b on top of equality.
    order: Option<BTreeSet<(Grade, Grade)>>,
    invalid_pairs: BTreeSet<Grade>,
    threshold: BTreeSet<Grade>,
    import_table: String,
}

impl Edition {
    /// The draft edition this library uses by default: P is all 30 pairs, the order is EVD.19's product, the
    /// invalid-pairs table is empty (EVD.21), the threshold is the trial U0 (EVD.29, EVD.63), and the import table is
    /// the draft label table (EVD.49). No threshold is chosen by this: the owner decides it (section 13).
    pub fn draft_1() -> Edition {
        Edition {
            name: "evidence-draft-1".to_string(),
            grades: Grade::all().into_iter().collect(),
            order: None,
            invalid_pairs: BTreeSet::new(),
            threshold: u0(),
            import_table: "evidence-import/corpus-labels.json (draft)".to_string(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn import_table(&self) -> &str {
        &self.import_table
    }

    pub fn threshold(&self) -> &BTreeSet<Grade> {
        &self.threshold
    }

    pub fn grades(&self) -> &BTreeSet<Grade> {
        &self.grades
    }

    pub fn invalid_pairs(&self) -> &BTreeSet<Grade> {
        &self.invalid_pairs
    }

    /// The same tables under another name.
    pub fn with_name(&self, name: &str) -> Edition {
        let mut e = self.clone();
        e.name = name.to_string();
        e
    }

    /// The same tables with another import-table pin.
    pub fn with_import_table(&self, pin: &str) -> Edition {
        let mut e = self.clone();
        e.import_table = pin.to_string();
        e
    }

    /// The same tables with another invalid-pairs table (EVD.21).
    pub fn with_invalid_pairs(&self, pairs: &[Grade]) -> Edition {
        let mut e = self.clone();
        e.invalid_pairs = pairs.iter().cloned().collect();
        e
    }

    /// The same tables with another threshold, which must be an up-set of P inside U0 (EVD.29).
    pub fn with_threshold(&self, threshold: &BTreeSet<Grade>) -> Result<Edition, EditionError> {
        let u0 = u0();
        for g in threshold {
            if !self.grades.contains(g) {
                return Err(EditionError::OutsideP(g.clone()));
            }
            if !u0.contains(g) {
                return Err(EditionError::ThresholdOutsideU0(g.clone()));
            }
        }
        let mut e = self.clone();
        e.threshold = threshold.clone();
        for g in threshold {
            for h in &self.grades {
                if e.le(g, h) && !threshold.contains(h) {
                    return Err(EditionError::ThresholdNotUpSet { member: g.clone(), above: h.clone() });
                }
            }
        }
        Ok(e)
    }

    /// The same tables with another order table: the listed pairs (a, b) mean a <= b. The caller supplies a
    /// reflexive-transitive relation; the library reads the table as given.
    pub fn with_order(&self, pairs: &[(Grade, Grade)]) -> Result<Edition, EditionError> {
        for (a, b) in pairs {
            for g in [a, b] {
                if !self.grades.contains(g) {
                    return Err(EditionError::OutsideP(g.clone()));
                }
            }
        }
        let mut e = self.clone();
        e.order = Some(pairs.iter().cloned().collect());
        Ok(e)
    }

    /// a <= b in this edition's order.
    pub fn le(&self, a: &Grade, b: &Grade) -> bool {
        if a == b {
            return true;
        }
        match &self.order {
            None => product_le(a, b),
            Some(pairs) => pairs.contains(&(a.clone(), b.clone())),
        }
    }

    /// The grade lies in the threshold U.
    pub fn passes(&self, g: &Grade) -> bool {
        self.threshold.contains(g)
    }

    /// I1: the grade lies outside P, or the invalid-pairs table lists it.
    pub fn grade_is_invalid(&self, g: &Grade) -> bool {
        !g.is_known() || !self.grades.contains(g) || self.invalid_pairs.contains(g)
    }
}
