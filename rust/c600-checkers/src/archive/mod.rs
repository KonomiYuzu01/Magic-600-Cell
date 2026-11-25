//! KN6.1's research-archive kind as a family of sub-checkers (the LP5 plan's reading A).
//!
//! Each claim family of LB5.15 and LB5.16 gets its own sub-checker, with its own ID, its own source digest and its own
//! mutation gate. A sub-checker is a [`SubChecker`] in a file under its stage's folder (`s14` to `s19`), registered by
//! one line in that folder's `SUBCHECKERS`; this file lists the stages once and needs no change when one is added.
//!
//! A sub-checker is reached as `check(engines, certificate, CheckerKind::Archive(id))`. Its run reads the certificate
//! through [`crate::claims`] and replays through [`crate::replay`], as the other kinds do, and it may reuse a kind's
//! checker for a claim group whose records are of that kind (the macro kind for `macro-certificate/1`). Its sources
//! list names its own files and every shared file it runs on (`Cargo.toml`, `lib.rs`, `report.rs`, `claims.rs`,
//! `replay.rs`, `kinds/mod.rs`, and a reused kind's file), so its identity changes exactly when its code does. An ID
//! that no stage registers is refused with `KN6:evidence.no_checker`; a claim group with no sub-checker stays
//! LB5.14's `unavailable(no_independent_checker, ...)` in the preflight data, never a report.

pub mod s14;
pub mod s15;
pub mod s16;
pub mod s17;
pub mod s18;
pub mod s19;

use crate::kinds::Checker;

/// One research-archive sub-checker: its checker (identity, engines, run), the stage and the claim family it checks.
pub struct SubChecker {
    pub checker: Checker,
    /// The restoration stage, 14 to 19.
    pub stage: u8,
    /// The claim family, as LB5.15 or LB5.16 names it.
    pub family: &'static str,
}

impl SubChecker {
    /// The sub-checker's ID, which is its sources' ID.
    pub fn id(&self) -> &'static str {
        self.checker.sources.id
    }
}

/// Every registered sub-checker, stage by stage.
pub fn registry() -> impl Iterator<Item = &'static SubChecker> {
    [s14::SUBCHECKERS, s15::SUBCHECKERS, s16::SUBCHECKERS, s17::SUBCHECKERS, s18::SUBCHECKERS, s19::SUBCHECKERS]
        .into_iter()
        .flatten()
        .copied()
}

/// The registered sub-checker with this ID.
pub fn find(id: &str) -> Option<&'static SubChecker> {
    registry().find(|sub| sub.id() == id)
}

/// The shared files every sub-checker's identity must cover: the check entry, the report, the claim reader, the replay
/// and the checker type, with the crate manifest.
pub const SHARED_SOURCES: [&str; 6] = [
    "c600-checkers/Cargo.toml",
    "c600-checkers/src/claims.rs",
    "c600-checkers/src/kinds/mod.rs",
    "c600-checkers/src/lib.rs",
    "c600-checkers/src/replay.rs",
    "c600-checkers/src/report.rs",
];

/// The registry's own invariants: every ID is unique and starts with its stage (`s14.` to `s19.`), every stage is 14
/// to 19, every sub-checker names a claim family, and its sources cover [`SHARED_SOURCES`] and at least one file of
/// its own. A registration that breaks one is reported by name.
pub fn registry_problems<'a>(subs: impl IntoIterator<Item = &'a SubChecker>) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for sub in subs {
        let id = sub.id();
        if !seen.insert(id) {
            problems.push(format!("{id} is registered twice"));
        }
        if !(14..=19).contains(&sub.stage) {
            problems.push(format!("{id} names stage {}", sub.stage));
        }
        if !id.starts_with(&format!("s{}.", sub.stage)) {
            problems.push(format!("{id} does not start with its stage s{}", sub.stage));
        }
        let named: Vec<&str> = sub.checker.sources.files.iter().map(|(name, _)| *name).collect();
        for shared in SHARED_SOURCES {
            if !named.contains(&shared) {
                problems.push(format!("{id}'s identity does not cover {shared}"));
            }
        }
        if named.len() <= SHARED_SOURCES.len() {
            problems.push(format!("{id} names no source of its own"));
        }
        if sub.family.is_empty() {
            problems.push(format!("{id} names no claim family"));
        }
    }
    problems
}
