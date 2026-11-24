//! LP5's exit sweep (`docs/progress/1.0/language-lp5-plan.md`, section 2, item 1): every ready claim group of the
//! stage 14 to 19 research archive checked in place, read-only, under the KN6 sub-checkers its binding names, and each
//! report matched with its record's own meaning, scope, original labels and check mode through R0's import.
//!
//! The sweep is driven by the data, not by a list of records: edition 1 of the typed claim bindings
//! (`c600-archive/data/claim-bindings-1.json`) names each group's record, stored inputs, derived inputs and claim
//! checkers; LB5.14's preflight, with `c600-checkers`' registry and the checkout's own bytes as host, says which groups
//! are ready; each ready group's record enters each of its checkers as its binding's `checked_as` says (the
//! certificate; an input of its `via` record; or a stage 19 table stated by its `IMPORT-MANIFEST.json` entry). Stored
//! inputs come by role from the binding's files, checked against their pins (`lp5_exit/roles.rs`); derived inputs are
//! rebuilt by the checker; the I_a menu is accepted only with the edition's identity block. No producer runs, no search
//! runs and nothing in the archive is written: `c600-checkers` imports no producer code (CT-KN6-02's scan in
//! `tests/test_language_scans.py`).
//!
//! # 291 and 286
//!
//! The plan counts 291 groups whose data the matrix marks self-contained (CX.14). Preflight finds 286 ready. The
//! difference is two sets, both counted by the preflight test below:
//!
//! - 7 self-contained groups have no bound claim checker, so they keep `no_independent_checker`: five `19/frontier`
//!   records (`type-lex-282-results.json`, `type-lex-independent-verified.json`,
//!   `type-lex-recovery-review-results.json`, `type-lex-recovery-transfer-full-replay.json`,
//!   `type-lex-recovery-transfer-results.json`) and two `19/ranks` reviews (`type-lex-independent-weight-review.json`,
//!   `type-weight-independent-review.json`). They are LB5.14's visible unavailable outcomes, not waived checks.
//! - 2 groups the matrix marks `needs_input` are ready: stage 18's composed strict′ groups of T6 and of the rule
//!   stages, whose intermediate start tables are rebuilt by exact replay (owner, LP5-3) and bound as derived inputs.
//!
//! 291 − 7 + 2 = 286.
//!
//! # Fast and slow
//!
//! The 287 checks of the 286 ready groups (the lattice run's record names two claim checkers) are split three ways
//! ([`sweep::part`]). The fast sweep runs in a normal `cargo test`: every stage 18 and 19 check but the long replays,
//! and one representative group of each stage 14 to 17 sub-checker (about a minute in a debug build on many threads).
//! The other 168 stage 14 to 17 checks recompute from the stage 13 tables (about 40 CPU-minutes in a debug build, 31
//! in release), so they are one ignored test; and the three checks that replay millions of letters
//! are ignored tests of their own ([`sweep::SLOW`]), each with its reason and its rough time. Run them with
//! `cargo test --release -p c600-research --test lp5_exit_sweep -- --ignored --nocapture`. `--nocapture` prints the
//! summary: ready groups checked and passed, scope-only reads, unavailable groups by reason, and the self-contained
//! groups that are not ready with their reasons.
//!
//! Everything here is source evidence: nothing is Windows, DirectX, input, long-session or performance evidence.

#[path = "lp5_exit/matching.rs"]
mod matching;
#[path = "lp5_exit/roles.rs"]
mod roles;
#[path = "lp5_exit/support.rs"]
mod support;
#[path = "lp5_exit/sweep.rs"]
mod sweep;

use std::collections::{BTreeMap, BTreeSet};

use c600_archive::bindings::{Availability, CheckedAs, DataStatus, Dependency, Reason};

use support::bindings;
use sweep::{by_checker, part, preflight, problems, ready_jobs, run_all, run_job, scope_only_jobs, Job, Part};
use sweep::{REPRESENTATIVES, SLOW};

/// The groups preflight answers `ready` (FR-B08's edition 1 with every sub-checker registered and every input held).
const READY: usize = 286;
/// The groups the matrix marks self-contained (CX.14).
const SELF_CONTAINED: usize = 291;
/// The checks of the ready groups: one per claim checker, and the lattice run's record names two.
const READY_CHECKS: usize = READY + 1;
/// The checks of the fast sweep and of the stage 14 to 17 sweep.
const FAST_CHECKS: usize = 116;
const STAGES_14_TO_17_CHECKS: usize = 168;

fn print_checkers(title: &str, done: &[sweep::Done]) {
    println!("{title}");
    for (id, t) in by_checker(done) {
        let (quick, slow) = if t.checks > t.shared { (t.quickest, t.slowest) } else { (0.0, 0.0) };
        println!(
            "  {id:<20} {:>3} checks, {:>3} passed, {:>3} shared reports; {:>7.1} s, quickest {quick:.1} s, \
             slowest {slow:.1} s",
            t.checks, t.passed, t.shared, t.seconds
        );
    }
}

/// What a part of the sweep found beside its passes, counted exactly so that a change in the archive, the import or
/// the bindings shows: the records' labels kept word for word, their own scope facts and check modes (by the set each
/// record states, `none stated` when it states none), the reports shared between groups with the same checker,
/// certificate and inputs, and the bound files a check did not read (by the binding's role).
#[derive(Debug, PartialEq, Eq)]
struct Summary {
    checks: usize,
    groups: usize,
    labels: usize,
    scopes: BTreeMap<String, usize>,
    modes: BTreeMap<String, usize>,
    shared: usize,
    unread: BTreeMap<String, usize>,
}

fn counts<const N: usize>(pairs: [(&str, usize); N]) -> BTreeMap<String, usize> {
    pairs.into_iter().map(|(k, n)| (k.to_owned(), n)).collect()
}

/// Runs a part of the sweep, prints its summary and returns it with the problems found.
fn sweep_part(title: &str, jobs: &[Job]) -> (Summary, Vec<String>) {
    let done = run_all(jobs);
    let passed = done.iter().filter(|d| d.problems.is_empty()).count();
    let failed: BTreeSet<usize> = done.iter().filter(|d| !d.problems.is_empty()).map(|d| d.job.row).collect();
    let groups: BTreeSet<usize> = done.iter().map(|d| d.job.row).collect();
    let mut by_how: BTreeMap<&str, usize> = BTreeMap::new();
    for d in &done {
        *by_how.entry(d.job.how.word()).or_default() += 1;
    }
    let meanings: Vec<&matching::Meaning> = done.iter().filter_map(|d| d.meaning.as_ref().ok()).collect();
    let labels: usize = meanings.iter().map(|m| m.labels.len()).sum();
    let mut modes: BTreeMap<String, usize> = BTreeMap::new();
    let mut scopes: BTreeMap<String, usize> = BTreeMap::new();
    for m in &meanings {
        let join = |set: &BTreeSet<String>| {
            if set.is_empty() {
                "none stated".to_owned()
            } else {
                set.iter().cloned().collect::<Vec<_>>().join(" + ")
            }
        };
        *modes.entry(join(&m.check_modes)).or_default() += 1;
        *scopes.entry(join(&m.scopes)).or_default() += 1;
    }
    let mut unread: BTreeMap<String, usize> = BTreeMap::new();
    for (role, _) in done.iter().flat_map(|d| &d.unread) {
        *unread.entry(role.clone()).or_default() += 1;
    }
    print_checkers(&format!("{title}: {} checks of {} ready groups", done.len(), groups.len()), &done);
    println!("  checks by how the record enters: {by_how:?}");
    println!(
        "  passed and matched {passed} of {} checks; groups with a check that did not: {}",
        done.len(),
        failed.len()
    );
    println!("  the records' labels kept word for word at the groups' pointers: {labels}");
    println!("  the records' own scope facts there: {scopes:?}");
    println!("  the records' own check modes there: {modes:?}");
    println!("  bound files a check did not read, by the binding's role: {unread:?}");
    let mut files: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for (role, path) in done.iter().flat_map(|d| &d.unread) {
        *files.entry((role.as_str(), path.as_str())).or_default() += 1;
    }
    for ((role, path), n) in files {
        println!("    {n:>4} checks did not read the {role} {path}");
    }
    let mut slowest: Vec<&sweep::Done> = done.iter().filter(|d| !d.shared).collect();
    slowest.sort_by(|a, b| b.seconds.total_cmp(&a.seconds));
    println!("  slowest checks:");
    for d in slowest.iter().take(8) {
        println!("    {:>7.1} s  {}", d.seconds, d.job.name());
    }
    let shared = done.iter().filter(|d| d.shared).count();
    let summary = Summary { checks: done.len(), groups: groups.len(), labels, scopes, modes, shared, unread };
    (summary, problems(&done))
}

/// CT-LB5-06 (source, part: LB5.14's preflight of all 510 claim groups with the KN6 registry and the checkout's own
/// bytes as host: 286 ready, each naming its registered checkers by source digest, and 224 unavailable with the
/// matrix's blockers; every stored input held with its pinned digest; the self-contained groups that are not ready
/// listed with their reasons; not run: the claim part, which `c600-checkers/tests/kn6_s19.rs` runs, and the manifest
/// part, which R0's `tests/test_lp5_archive.py` runs).
#[test]
fn ct_lb5_06_preflight_every_group_with_the_registry_and_the_checkout_as_host() {
    let b = bindings();
    let answers = preflight();
    assert_eq!(answers.len(), 510);
    let mut ready = 0;
    for (row, answer) in b.rows().iter().zip(answers) {
        match answer {
            Availability::Ready { checkers, inputs } => {
                ready += 1;
                let ids: Vec<&str> = checkers.iter().map(|c| c.id.as_str()).collect();
                assert_eq!(ids, row.claim_checkers(), "{}", row.group);
                for c in checkers {
                    let sub = c600_checkers::archive::find(&c.id).expect("registered");
                    assert_eq!(c.digest, sub.checker.sources.identity().source, "{}", row.group);
                }
                assert_eq!(inputs.len(), b.stored(row).len() + row.derived.len(), "{}", row.group);
            }
            Availability::Unavailable { reasons, dependencies } => {
                assert!(!reasons.is_empty(), "{}", row.group);
                for reason in reasons {
                    assert!(
                        row.matrix_reasons.contains(reason) && !row.resolved.contains_key(reason),
                        "{}: {reason:?} is not one of the matrix's unresolved reasons {:?}",
                        row.group,
                        row.matrix_reasons
                    );
                }
                assert!(
                    !reasons.contains(&Reason::DependencyMismatch) && !reasons.contains(&Reason::ClaimUnmapped),
                    "{}: {reasons:?}",
                    row.group
                );
                for d in dependencies {
                    assert!(!matches!(d, Dependency::Input { .. }), "{}: the checkout lacks {d:?}", row.group);
                }
            }
        }
    }
    assert_eq!(ready, READY);
    let unavailable: Vec<Availability> = answers.iter().filter(|a| !a.is_ready()).cloned().collect();
    let per_reason = sweep::reasons(&unavailable);
    assert_eq!(
        per_reason,
        BTreeMap::from([
            ("dependency_missing", 210),
            ("historical_check_incomplete", 2),
            ("no_independent_checker", 216),
            ("unsupported_claim", 6),
        ])
    );

    // The self-contained groups that are not ready, and the ready groups that are not self-contained.
    let rows = b.rows();
    let contained: Vec<usize> =
        rows.iter().filter(|r| r.data_status == DataStatus::SelfContained).map(|r| r.index).collect();
    assert_eq!(contained.len(), SELF_CONTAINED);
    let not_ready: Vec<usize> = contained.iter().copied().filter(|&i| !answers[i].is_ready()).collect();
    let extra: Vec<usize> = (0..rows.len())
        .filter(|&i| answers[i].is_ready() && rows[i].data_status != DataStatus::SelfContained)
        .collect();
    assert_eq!((not_ready.len(), extra.len()), (7, 2));
    for &i in &not_ready {
        assert_eq!(answers[i].reasons(), [Reason::NoIndependentChecker], "{}", rows[i].group);
        assert!(rows[i].checkers.is_empty(), "{}: no sub-checker is bound", rows[i].group);
        assert!(matches!(rows[i].family.as_str(), "19/frontier" | "19/ranks"), "{}", rows[i].group);
    }
    for &i in &extra {
        assert_eq!(rows[i].family, "18/composed", "{}", rows[i].group);
        assert_eq!(rows[i].data_status, DataStatus::NeedsInput);
        assert!(!rows[i].derived.is_empty() && rows[i].resolved.contains_key(&Reason::DependencyMissing));
    }
    assert_eq!(SELF_CONTAINED - not_ready.len() + extra.len(), READY);

    println!("preflight of {} groups: {ready} ready, {} unavailable", answers.len(), unavailable.len());
    println!("unavailable groups by reason (a group counts once per reason):");
    for (reason, n) in &per_reason {
        println!("  {reason:<28} {n:>3}");
    }
    println!("unavailable groups by reason set:");
    for (set, n) in sweep::reason_sets(answers) {
        println!("  {n:>3}  [{set}]");
    }
    println!("self-contained ({SELF_CONTAINED}) but not ready ({}):", not_ready.len());
    for &i in &not_ready {
        let words: Vec<&str> = answers[i].reasons().iter().map(Reason::word).collect();
        println!("  {}  [{}]", rows[i].group, words.join(", "));
    }
    println!("ready but not self-contained ({}): derived start tables rebuilt by exact replay (LP5-3)", extra.len());
    for &i in &extra {
        println!("  {}  ({} derived)", rows[i].group, rows[i].derived.len());
    }
}

/// The ready checks of one part, after checking that the parts split every ready check once.
fn jobs_of(wanted: Part) -> Vec<Job> {
    let jobs = ready_jobs();
    let groups: BTreeSet<usize> = jobs.iter().map(|j| j.row).collect();
    assert_eq!(groups.len(), READY);
    assert_eq!(jobs.len(), READY_CHECKS, "the lattice run's record names two claim checkers, every other one");
    assert!(jobs.iter().all(|j| j.how != CheckedAs::ScopeOnly), "a ready group's checkers check its claim");
    let mut per_group: BTreeMap<usize, usize> = BTreeMap::new();
    for j in &jobs {
        *per_group.entry(j.row).or_default() += 1;
    }
    let twice: Vec<&str> = per_group
        .iter()
        .filter(|(_, n)| **n > 1)
        .map(|(row, _)| bindings().file(bindings().rows()[*row].source).path.as_str())
        .collect();
    assert_eq!(twice.len(), 1);
    assert!(twice[0].ends_with("lattice-d1/run/run.json"), "{twice:?}");
    for (checker, record, _) in SLOW {
        let n = jobs.iter().filter(|j| j.checker == checker && j.record().ends_with(record)).count();
        assert_eq!(n, 1, "{checker} on {record} is one ready check");
    }
    for (checker, record) in REPRESENTATIVES {
        let n = jobs.iter().filter(|j| j.checker == checker && j.record().ends_with(record)).count();
        assert_eq!(n, 1, "{checker} on {record} is one ready check");
    }
    let count = |p: Part| jobs.iter().filter(|j| part(j) == p).count();
    assert_eq!(
        (count(Part::Fast), count(Part::Stages14To17), count(Part::Slow)),
        (FAST_CHECKS, STAGES_14_TO_17_CHECKS, SLOW.len())
    );
    jobs.into_iter().filter(|j| part(j) == wanted).collect()
}

/// CT-KN6-03, CT-LB5-01, CT-LB5-02, CT-LB5-03, CT-LB5-04, CT-LB5-05, CT-LB5-06 (source, part: the exit sweep's fast
/// part, driven from the bindings: every ready group of stages 18 and 19 but the long replays, and one representative
/// group of each stage 14 to 17 sub-checker; each claim checker the binding names runs on the group's record as its
/// binding says it enters, with the stored inputs the binding names checked against their pins, and passes; each
/// report's checker identity, kind and input digest are the binding's checker and record, and the report is matched
/// with the record's meaning, scope phrase, original labels word for word and check mode through R0's import; not run
/// in a normal test run: the stage 14 to 17 sweep and the three long replays, which are the ignored tests below, and
/// every mutation gate, which `c600-checkers`' own tests run).
#[test]
fn ct_kn6_03_every_ready_group_passes_under_its_bound_subcheckers() {
    let jobs = jobs_of(Part::Fast);
    println!(
        "ready groups {READY}, checks {READY_CHECKS}: fast sweep {FAST_CHECKS}, stage 14 to 17 sweep \
         {STAGES_14_TO_17_CHECKS} (ignored), long replays {} (ignored)",
        SLOW.len()
    );
    for (checker, record, why) in SLOW {
        println!("  long replay: {checker} on {record}: {why}");
    }
    let (summary, found) = sweep_part("fast sweep", &jobs);
    assert_eq!(
        summary,
        Summary {
            checks: FAST_CHECKS,
            groups: FAST_CHECKS,
            labels: 208,
            scopes: counts([("declared search set", 1), ("none stated", 113), ("one finite instance", 2)]),
            modes: counts([("fresh exact audit", 1), ("none stated", 115)]),
            shared: 90,
            unread: counts([("catalogue", 3), ("convention", 1276), ("record", 97)]),
        }
    );
    assert!(found.is_empty(), "{} of {} checks do not pass and match:\n{}", found.len(), jobs.len(), found.join("\n"));
}

/// CT-KN6-03, CT-LB5-01, CT-LB5-02, CT-LB5-03, CT-LB5-04 (source, part: the exit sweep's stage 14 to 17 part, driven
/// from the bindings: every ready group of stages 14 to 17 that the fast part does not check, under the sub-checkers
/// its binding names, matched as in the fast part; not run in a normal test run).
#[test]
#[ignore = "168 checks recomputed from the stage 13 tables: about 40 CPU-minutes in a debug build and 31 in release, \
            two minutes of wall time on 32 threads"]
fn ct_kn6_03_stage14_to_17_sweep() {
    let jobs = jobs_of(Part::Stages14To17);
    let (summary, found) = sweep_part("stage 14 to 17 sweep", &jobs);
    assert_eq!(
        summary,
        Summary {
            checks: STAGES_14_TO_17_CHECKS,
            groups: STAGES_14_TO_17_CHECKS,
            labels: 458,
            scopes: counts([("declared search set", 7), ("none stated", 159), ("one finite instance", 2)]),
            modes: counts([("actual replay", 65), ("none stated", 103)]),
            shared: 0,
            unread: counts([("catalogue", 17), ("convention", 1848)]),
        }
    );
    assert!(found.is_empty(), "{} of {} checks do not pass and match:\n{}", found.len(), jobs.len(), found.join("\n"));
}

/// CT-LB5-06 (source, part: a sub-checker named scope-only reads its group's record and answers `not_applicable` with
/// the record's own bounds, never a pass, so the group keeps `no_independent_checker` and its preflight stays
/// unavailable; LB5.15's search, selector, reduction, RR and rule-stop records; not run: the claims themselves, which
/// no KN6 checker of a search's coverage exists for).
#[test]
fn ct_lb5_06_scope_only_reads_answer_not_applicable() {
    let jobs = scope_only_jobs();
    assert_eq!(jobs.len(), 114);
    for j in &jobs {
        assert!(!preflight()[j.row].is_ready(), "{}", j.name());
        assert!(preflight()[j.row].reasons().contains(&Reason::NoIndependentChecker), "{}", j.name());
    }
    let done = run_all(&jobs);
    let found = problems(&done);
    print_checkers(&format!("scope-only reads: {} groups", jobs.len()), &done);
    assert!(found.is_empty(), "{} scope-only reads did not answer not_applicable:\n{}", found.len(), found.join("\n"));
}

/// CT-LB5-06 (source, part: an unavailable group runs no check: no claim checker is planned for it, its answer names
/// LB5.14's reasons and its known blockers, and q11 keeps LB5.17's missing successor cache; not run: any check of
/// these groups, which is the point).
#[test]
fn ct_lb5_06_unavailable_groups_run_no_check() {
    let b = bindings();
    let planned: BTreeSet<usize> = ready_jobs().iter().map(|j| j.row).collect();
    let mut by_family: BTreeMap<&str, usize> = BTreeMap::new();
    let mut unavailable = 0;
    for (row, answer) in b.rows().iter().zip(preflight()) {
        if answer.is_ready() {
            continue;
        }
        unavailable += 1;
        assert!(!planned.contains(&row.index), "{}: a check is planned for an unavailable group", row.group);
        assert!(!answer.reasons().is_empty(), "{}", row.group);
        if answer.reasons().contains(&Reason::HistoricalCheckIncomplete) {
            assert!(answer.dependencies().iter().any(|d| matches!(d, Dependency::HistoricalGate { .. })));
        }
        *by_family.entry(row.family.as_str()).or_default() += 1;
    }
    assert_eq!(unavailable, 510 - READY);
    let q11 = b.rows().iter().find(|r| r.family == "19/q11").expect("q11's group");
    let answer = &preflight()[q11.index];
    assert_eq!(answer.reasons(), [Reason::DependencyMissing, Reason::NoIndependentChecker]);
    assert!(matches!(&answer.dependencies()[0], Dependency::MissingInput { name, .. }
        if name.starts_with("type-lex-successor-base-cache.npz")));
    println!("unavailable groups, no check run ({unavailable}), by family:");
    for (family, n) in by_family {
        println!("  {family:<22} {n:>3}");
    }
}

/// Runs one of the slow checks and asserts it passes and matches.
fn slow_check(record: &str) {
    let (checker, _, why) = *SLOW.iter().find(|(_, r, _)| *r == record).expect("a slow check");
    let job =
        ready_jobs().into_iter().find(|j| j.checker == checker && j.record().ends_with(record)).expect("a ready check");
    assert!(job.how == CheckedAs::Certificate);
    if checker == "s18.composed" {
        assert!(!job.binding().derived.is_empty(), "{}: the composed chain's later starts are derived", job.name());
    }
    let done = run_job(&job);
    println!("{} ({why}): {:.1} s", job.name(), done.seconds);
    assert!(done.problems.is_empty(), "{}: {:#?}", job.name(), done.problems);
}

/// CT-LB5-05, CT-KN6-03 (source, part: gate 2 of T6's four steps checked in place from the stored stop table, steps 2
/// to 4 starting at start tables rebuilt by exact replay as derived inputs, ending at the stored end table; not run in
/// a normal test run).
#[test]
#[ignore = "T6's four steps, 1,510,756 letters played one by one: about three and a half hours in release at about 120 \
            letters a second"]
fn ct_lb5_05_gate2_composed_t6() {
    slow_check(SLOW[0].1);
}

/// CT-LB5-05, CT-KN6-03 (source, part: gate 2 of the rule's three stages checked in place, stages 2 and 3 starting at
/// rebuilt start tables; not run in a normal test run).
#[test]
#[ignore = "the rule's three stages, 688,686 letters played one by one: about 95 minutes in release (about 120 letters \
            a second)"]
fn ct_lb5_05_gate2_composed_rule_stages() {
    slow_check(SLOW[1].1);
}

/// CT-LB5-05, CT-KN6-03 (source, part: the lattice phase replayed letter by letter from T6's stored end table, a new
/// check with no archived `check.jsonl` to match; not run in a normal test run).
#[test]
#[ignore = "the lattice phase, 13,231,266 letters played one by one: 11 hours or more in release, about 30 at gate 2's \
            rate of about 120 letters a second"]
fn ct_lb5_05_lattice_replay() {
    slow_check(SLOW[2].1);
}
