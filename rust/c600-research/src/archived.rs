//! Archived runs, as recorded (DET.48; CT-DET-24).
//!
//! The research archive's runs were made by producers this repository does not run: timed searches under child
//! deadlines and CPU caps (stages 13 to 15), fixed-seed constructions under letter budgets (stage 18), and data with no
//! producer at all (stage 19). `c600-archive` imports their records as data and keeps each label as the import table
//! maps it. Here they stay recorded facts: an archived run has no `ReproKey`, so it supplies no new run and no portable
//! replay; its reproduction is `not run` for want of the key's bytes (DET.7), and a timed stop keeps its recorded
//! outcome and claim and establishes no absence (DET.30). A witness the archive stores counts only once a KN6 check
//! job checks it (KN7.4), and then with that job's own record and scope.

use c600_archive::import::{ImportReport, OccurrenceOutcome};
use c600_archive::{ImportTable, Outcome};
use c600_session::persist::run::{codes, Reproduction};

/// One archived run fact: a field of a record whose import-table facts state a job outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchivedRun {
    /// The member's name in the granted archive.
    pub member: String,
    /// The field, as the import table names it.
    pub field: String,
    /// The job outcome the facts state, word for word (`cut off`, `completed`, ...).
    pub job_outcome: String,
    pub claim: Option<String>,
    pub check: Option<String>,
    pub scope: Option<String>,
}

impl ArchivedRun {
    /// DET.7 and DET.48: an archived run has no key, so it reproduces as `not run` with
    /// `LG4:input.repro_input_missing`, whatever its recorded outcome.
    pub fn reproduction(&self) -> Reproduction {
        Reproduction::NotRun(codes::INPUT_MISSING.into())
    }

    /// Whether the recorded stop is a timed or budgeted one, which keeps its limit and never reads complete.
    pub fn is_stop(&self) -> bool {
        self.job_outcome != "completed"
    }
}

/// The archived run facts of an import, member by member, in the report's order.
pub fn archived_runs(report: &ImportReport, table: &ImportTable) -> Vec<ArchivedRun> {
    let rules = table.rules();
    report
        .occurrences
        .iter()
        .filter_map(|group| {
            let OccurrenceOutcome::Facts { rule } = group.outcome else {
                return None;
            };
            let Outcome::Facts(facts) = &rules.get(rule)?.outcome else {
                return None;
            };
            Some(ArchivedRun {
                member: report.members.get(group.member)?.name.clone(),
                field: group.field.clone(),
                job_outcome: facts.job_outcome.clone()?,
                claim: facts.claim.clone(),
                check: facts.check.clone(),
                scope: facts.scope.clone(),
            })
        })
        .collect()
}
