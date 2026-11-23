//! KN6.1's four non-archive kinds, one checker each. Each checker names its own sources, so its identity changes
//! exactly when its code or the shared core it runs on changes (KN6.3).

pub mod admissibility;
pub mod checkpoints;
pub mod expansion;
pub mod macros;

use crate::claims::{Run, Step};
use crate::report::Sources;
use crate::Engines;

/// The exact engines a checker reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Needs {
    pub kernel: bool,
    pub jumble: bool,
}

impl Needs {
    pub const JUMBLE: Needs = Needs { kernel: false, jumble: true };
    pub const KERNEL: Needs = Needs { kernel: true, jumble: false };
    /// Both engines: a kind whose certificates name either puzzle.
    pub const BOTH: Needs = Needs { kernel: true, jumble: true };
}

/// One checker: its sources (its identity), the engines it needs, and its check.
pub struct Checker {
    pub sources: &'static Sources,
    pub needs: Needs,
    pub run: fn(&Engines, &mut Run) -> Step<()>,
}
