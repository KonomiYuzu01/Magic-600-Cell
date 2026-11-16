//! A protection verdict that carries its level (LB3.4, KN1.11): pose level inside Jumble, label level in
//! 600-cell-Full. A verdict answers only for a puzzle of its own level; asked for the other, it is a [`Crossing`]. So a
//! label-level pass after crossing never stands in for the pose-level check inside Jumble, and a pose-level verdict
//! never answers for 600-cell-Full.

use std::fmt;

use c600_jumble::state::PoseProtection;
use c600_kernel::{PiecePosition, Puzzle};

use crate::level::Level;

/// The chapter 13 code of a crossing: the verdict does not hold the level asked for.
pub const LEVEL_NOT_HELD: &str = "KN1:input.level_not_held";

/// What a protection check found at its level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Finding {
    /// No protected piece moves.
    Clear,
    /// These protected pieces move, ascending.
    Moves(Vec<u32>),
}

/// A protection verdict and the level it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    level: Level,
    protected: Vec<u32>,
    finding: Finding,
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

/// The `positions` rows before and after an operation do not name the same pieces in the same order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowsDiffer;

impl fmt::Display for RowsDiffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the positions rows before and after do not name the same pieces")
    }
}

impl std::error::Error for RowsDiffer {}

impl Verdict {
    /// The pose-level verdict of `c600-jumble`'s Net protection report of a twist (selection 4), judged on the state
    /// whose `Digest<poses>` is `read`.
    pub fn poses(report: &PoseProtection, read: &str) -> Verdict {
        let finding = if report.moves() { Finding::Moves(report.pieces.clone()) } else { Finding::Clear };
        Verdict { level: Level::Poses, protected: report.protected.clone(), finding, read: read.to_owned() }
    }

    /// The label-level Net verdict for the pieces of KN2.4's `positions` rows of one operation's base (`before`) and
    /// result (`after`), judged on the base whose `Digest<labels>` is `read`: a piece moves when one of its stickers
    /// is in another slot after the operation.
    pub fn labels(before: &[PiecePosition], after: &[PiecePosition], read: &str) -> Result<Verdict, RowsDiffer> {
        let same = |(b, a): (&PiecePosition, &PiecePosition)| b.piece == a.piece && b.home == a.home;
        if before.len() != after.len() || !before.iter().zip(after).all(same) {
            return Err(RowsDiffer);
        }
        let mut protected: Vec<u32> = before.iter().map(|row| row.piece as u32).collect();
        protected.sort_unstable();
        protected.dedup();
        let mut moved: Vec<u32> =
            before.iter().zip(after).filter(|(b, a)| b.slots != a.slots).map(|(b, _)| b.piece as u32).collect();
        moved.sort_unstable();
        moved.dedup();
        let finding = if moved.is_empty() { Finding::Clear } else { Finding::Moves(moved) };
        Ok(Verdict { level: Level::Labels, protected, finding, read: read.to_owned() })
    }

    /// The level the verdict holds.
    pub fn level(&self) -> Level {
        self.level
    }

    /// The protected pieces, ascending.
    pub fn protected(&self) -> &[u32] {
        &self.protected
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
