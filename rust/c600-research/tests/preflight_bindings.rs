//! LP5 piece R1: the typed claim bindings of `c600-archive` against the KN6 registry of `c600-checkers`.
//!
//! Test names carry the CT ID and part (PHS.62): CT-LB5-06's preflight part. Every research-archive sub-checker a
//! binding names is registered, every registered sub-checker is named by at least one binding, and LB5.14's preflight
//! with the registry as its host answers `ready` exactly for the groups whose claim checkers are registered and whose
//! blockers the binding resolves, naming each checker by its registered source digest. Preflight runs no check and
//! reads no file; the stored inputs are answered as held here, since this test is about the checkers.

use std::collections::{BTreeMap, BTreeSet};

use c600_archive::bindings::{Availability, Bindings, CheckedAs, DataStatus, FilePin, Host, InputState, Reason};
use c600_checkers::archive;

/// The host's KN6 registry: a research-archive sub-checker by its ID, with its source digest; every input held.
struct Registry;

impl Host for Registry {
    fn checker(&self, id: &str) -> Option<String> {
        archive::find(id).map(|sub| sub.checker.sources.identity().source)
    }

    fn input(&self, _: &FilePin) -> InputState {
        InputState::Present
    }
}

#[test]
fn ct_lb5_06_preflight_bindings_and_registry_name_each_other() {
    let bindings = Bindings::edition_1();
    assert!(archive::registry_problems(archive::registry()).is_empty());
    let registered: BTreeSet<&str> = archive::registry().map(|sub| sub.id()).collect();
    let named: BTreeSet<&str> =
        bindings.rows().iter().flat_map(|row| row.checkers.iter().map(String::as_str)).collect();
    let unregistered: Vec<&&str> = named.difference(&registered).collect();
    assert!(unregistered.is_empty(), "named by a binding but not registered: {unregistered:?}");
    let unnamed: Vec<&&str> = registered.difference(&named).collect();
    assert!(unnamed.is_empty(), "registered but named by no binding: {unnamed:?}");
    for row in bindings.rows() {
        for id in row.claim_checkers() {
            let sub = archive::find(id).expect("registered");
            assert_eq!(sub.stage, row.stage, "{}: {id} checks a claim of its own stage", row.group);
        }
        if row.checked_as == Some(CheckedAs::ScopeOnly) {
            for id in &row.checkers {
                assert!(archive::find(id).expect("registered").stage <= row.stage, "{}: {id}", row.group);
            }
        }
    }
}

#[test]
fn ct_lb5_06_preflight_with_the_registry_as_host() {
    let bindings = Bindings::edition_1();
    let mut ready_by_checker: BTreeMap<String, usize> = BTreeMap::new();
    let mut unavailable = 0;
    for row in bindings.rows() {
        match bindings.availability(row, &Registry) {
            Availability::Ready { checkers, .. } => {
                let ids: Vec<&str> = checkers.iter().map(|c| c.id.as_str()).collect();
                assert_eq!(ids, row.claim_checkers(), "{}", row.group);
                for checker in &checkers {
                    let sub = archive::find(&checker.id).expect("registered");
                    assert_eq!(checker.digest, sub.checker.sources.identity().source, "{}", row.group);
                    *ready_by_checker.entry(checker.id.clone()).or_default() += 1;
                }
            }
            Availability::Unavailable { reasons, .. } => {
                unavailable += 1;
                assert!(!reasons.is_empty(), "{}", row.group);
                if row.claim_checkers().is_empty() && row.data_status != DataStatus::RecordOnly {
                    assert!(reasons.contains(&Reason::NoIndependentChecker), "{}", row.group);
                } else {
                    assert!(!reasons.contains(&Reason::NoIndependentChecker), "{}", row.group);
                }
            }
        }
    }
    assert_eq!(unavailable, 510 - 286);
    let expected: BTreeMap<String, usize> = [
        ("s14.blocks", 4),
        ("s14.levels", 14),
        ("s14.literal", 9),
        ("s14.locality", 1),
        ("s14.t3p", 16),
        ("s15.macro", 55),
        ("s15.pstar", 2),
        ("s16.macro", 48),
        ("s16.mobility", 6),
        ("s16.release", 4),
        ("s17.macro", 20),
        ("s18.composed", 2),
        ("s18.expansion", 35),
        ("s18.lattice_replay", 1),
        ("s18.lattice_run", 58),
        ("s19.frontier", 2),
        ("s19.gamma-matching", 1),
        ("s19.gamma-witness", 1),
        ("s19.pose-table", 6),
        ("s19.ranks", 2),
    ]
    .into_iter()
    .map(|(id, n)| (id.to_string(), n))
    .collect();
    assert_eq!(ready_by_checker, expected);
}
