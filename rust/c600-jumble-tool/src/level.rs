//! The level a result reads (KN1.7, KN1.8, KN1.11): poses inside Jumble, labels in 600-cell-Full. A result names its
//! level; results of different levels are never compared, and nothing here lifts a value from labels to poses
//! (KN1.10).

use c600_kernel::Puzzle;

/// The level of a result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Level {
    /// The pose table, whose digest is `Digest<poses>` (`c600_jumble::state::State::digest`).
    Poses,
    /// The label array, whose digest is `Digest<labels>` (`c600_kernel::Snapshot::digest`).
    Labels,
}

impl Level {
    /// The level a puzzle's protection verdicts and relations read (KN1.11): poses inside Jumble, labels in
    /// 600-cell-Full.
    pub fn of(puzzle: Puzzle) -> Level {
        match puzzle {
            Puzzle::Jumbling => Level::Poses,
            Puzzle::Full => Level::Labels,
        }
    }

    /// The level's name as the reports name it: `poses` or `labels`.
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Poses => "poses",
            Level::Labels => "labels",
        }
    }
}
