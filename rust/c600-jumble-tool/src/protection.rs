//! A protection verdict that carries its level (LB3.4, KN1.11): pose level inside Jumble, label level in
//! 600-cell-Full. A verdict answers only for a puzzle of its own level; asked for the other, it is a [`Crossing`]. So a
//! label-level pass after crossing never stands in for the pose-level check inside Jumble, and a pose-level verdict
//! never answers for 600-cell-Full.
//!
//! A verdict holds an engine's report and judges nothing itself. At pose level the report is M8's protection of a
//! whole word (`c600_jumble::levels::derive`); at label level it is the kernel's protection of named pieces, fixed
//! cell centres included (`c600_kernel::Kernel::piece_protection` under Net, `strict_piece_protection` under Strict).
//! Both read the kernel's modes (KN2.7): Net judges the end of the whole word, and Strict every prefix, naming the
//! first one refused.

use std::fmt;

use c600_jumble::levels::WordProtection;
use c600_kernel::{PieceProtection, ProtectionMode, Puzzle, Verdict as Judged};

use crate::level::Level;

/// The chapter 13 code of a crossing: the verdict does not hold the level asked for.
pub const LEVEL_NOT_HELD: &str = "KN1:input.level_not_held";

/// What a protection check found at its level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Finding {
    /// No protected piece moves.
    Clear,
    /// These protected pieces move, ascending: at the end of the word under Net, at the first refused prefix under
    /// Strict.
    Moves(Vec<u32>),
}

impl Finding {
    fn of(verdict: Judged, moved: Vec<u32>) -> Finding {
        match verdict {
            Judged::Clear => Finding::Clear,
            Judged::Moves => Finding::Moves(moved),
        }
    }
}

/// A protection verdict and the level it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    level: Level,
    mode: ProtectionMode,
    protected: Vec<u32>,
    finding: Finding,
    first_refused_prefix: Option<u64>,
    read: String,
}

/// A verdict asked for a puzzle whose protection reads another level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Crossing {
    /// The level the verdict holds.
    pub held: Level,
    /// The level the puzzle's protection reads.
    pub asked: Level,
}

impl Crossing {
    pub fn code(&self) -> &'static str {
        LEVEL_NOT_HELD
    }
}

impl fmt::Display for Crossing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (held, asked) = (self.held.as_str(), self.asked.as_str());
        write!(f, "{}: the verdict holds level {held}, and the puzzle's protection reads level {asked}", self.code())
    }
}

impl std::error::Error for Crossing {}

impl Verdict {
    /// The pose-level verdict of M8's protection report of a word, a derived value's (`levels::Derived::protection`)
    /// or a protection refusal's (`levels::Refused::protection`), judged on the base whose `Digest<poses>` is `read`.
    pub fn poses(report: &WordProtection, read: &str) -> Verdict {
        Verdict {
            level: Level::Poses,
            mode: report.mode,
            protected: report.protected.clone(),
            finding: Finding::of(report.verdict, report.pieces.clone()),
            first_refused_prefix: report.first_refused_prefix,
            read: read.to_owned(),
        }
    }

    /// The label-level verdict of the kernel's protection report of named pieces (`Kernel::piece_protection` under
    /// Net, `Kernel::strict_piece_protection` under Strict), judged on the base whose `Digest<labels>` is `read`. The
    /// kernel names a piece by its home position, the piece number `c600_jumble` uses.
    pub fn labels(report: &PieceProtection, read: &str) -> Verdict {
        let level = match report.level {
            c600_kernel::Level::Labels => Level::Labels,
        };
        // A piece is below 177,120, so it fits in u32.
        let pieces = |pieces: &[usize]| -> Vec<u32> { pieces.iter().map(|&piece| piece as u32).collect() };
        Verdict {
            level,
            mode: report.mode,
            protected: pieces(&report.protected),
            finding: Finding::of(report.verdict, pieces(&report.moved())),
            first_refused_prefix: report.first_refused_prefix,
            read: read.to_owned(),
        }
    }

    /// The level the verdict holds.
    pub fn level(&self) -> Level {
        self.level
    }

    /// The mode the report judged in.
    pub fn mode(&self) -> ProtectionMode {
        self.mode
    }

    /// The protected pieces, ascending.
    pub fn protected(&self) -> &[u32] {
        &self.protected
    }

    /// Strict only: the length of the first refused prefix (1 for the first letter); `None` for a clear verdict and
    /// under Net.
    pub fn first_refused_prefix(&self) -> Option<u64> {
        self.first_refused_prefix
    }

    /// The digest of the state the verdict judged, at its level (KN0.6).
    pub fn read(&self) -> &str {
        &self.read
    }

    /// The finding, for a protection check of `puzzle`. Refused with a [`Crossing`] when the puzzle's protection reads
    /// another level than the verdict holds (KN1.11): the verdict never crosses over.
    pub fn answer(&self, puzzle: Puzzle) -> Result<&Finding, Crossing> {
        let asked = Level::of(puzzle);
        if asked == self.level {
            Ok(&self.finding)
        } else {
            Err(Crossing { held: self.level, asked })
        }
    }
}
