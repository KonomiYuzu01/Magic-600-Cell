//! The cost profile (EV2.7; LP4-6): KN7's granted, used and remaining amounts of a run, with the named budget and the
//! stop reason, and the word costs by alphabet.
//!
//! Every amount is the meter's: interpreter steps and declared kernel cost are separate counters (DET.19), memory is
//! the peak of live logical memory, and output counts logical units. A kernel row charges its declared tariff whether
//! the host answered it from a cache or computed it (KN7.8, DET.23), so the profile of a cached run equals the profile
//! of an uncached one. Elapsed time may accompany the profile as display data; it is outside the profile's equality and
//! never changes a stop.

use std::time::Duration;

use c600_run::meter::{Amounts, Dim, Summary};

/// The alphabet a word's cost is counted in (partition LG2): retained generators, menu letters or UI actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Alphabet {
    RetainedGenerators,
    MenuLetters,
    UiActions,
}

impl Alphabet {
    pub fn name(self) -> &'static str {
        match self {
            Alphabet::RetainedGenerators => "retained_generators",
            Alphabet::MenuLetters => "menu_letters",
            Alphabet::UiActions => "ui_actions",
        }
    }
}

/// A word cost: its letters, counted in its alphabet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WordCost {
    pub alphabet: Alphabet,
    pub letters: u128,
}

/// A run's cost profile (EV2.7). Equality covers every deterministic field and leaves out `elapsed`.
#[derive(Clone, Debug)]
pub struct CostProfile {
    /// The named budget: the run's name and the scope the meter counted it in.
    pub budget: String,
    pub granted: Amounts,
    pub used: Amounts,
    pub remaining: Amounts,
    /// The stop reason: the code and the failed phase, or `None` for a completed run or one still running.
    pub stop: Option<(&'static str, &'static str)>,
    pub words: Vec<WordCost>,
    /// Elapsed wall-clock time, as display data only.
    pub elapsed: Option<Duration>,
}

impl PartialEq for CostProfile {
    fn eq(&self, other: &CostProfile) -> bool {
        (&self.budget, self.granted, self.used, self.remaining, self.stop, &self.words)
            == (&other.budget, other.granted, other.used, other.remaining, other.stop, &other.words)
    }
}

/// What remains of `granted` after `used`: steps, kernel cost and output as limit less use, memory as limit less the
/// peak.
pub fn remaining(granted: Amounts, used: Amounts) -> Amounts {
    let left = |dim: Dim| granted.get(dim).saturating_sub(used.get(dim));
    Amounts {
        steps: left(Dim::Steps),
        kernel_cost: left(Dim::KernelCost),
        memory: left(Dim::Memory),
        output: left(Dim::Output),
    }
}

/// The profile of an ended run: its terminal record under `granted`, its budget's name and its word costs.
pub fn of_summary(budget: &str, granted: Amounts, ended: &Summary, words: Vec<WordCost>) -> CostProfile {
    let stop = ended.code.map(|code| (code, ended.stop.as_ref().map_or("", |p| p.phase)));
    CostProfile {
        budget: budget.to_owned(),
        granted,
        used: ended.used,
        remaining: remaining(granted, ended.used),
        stop,
        words,
        elapsed: None,
    }
}

/// The profile of a run still running, at the amounts it has used so far.
pub fn so_far(budget: &str, granted: Amounts, used: Amounts) -> CostProfile {
    CostProfile {
        budget: budget.to_owned(),
        granted,
        used,
        remaining: remaining(granted, used),
        stop: None,
        words: Vec::new(),
        elapsed: None,
    }
}
