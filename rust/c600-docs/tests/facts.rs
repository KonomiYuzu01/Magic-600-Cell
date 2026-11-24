//! The generated fields of each principle record, printed for the person who refreshes a record after its module
//! changed: the source and closure digests, the model identity, the decomposition digest, and the node and leaf labels
//! a step map may name, with the kernel leaves that need a leaf specification. Run on request:
//! `cargo test -p c600-docs --test facts -- --ignored --nocapture`. A refreshed binding leaves the hand-written parts
//! stale until a person rewrites them against the new decomposition (LB6.2).

mod common;

#[test]
#[ignore]
fn print_facts() {
    for s in common::subjects() {
        println!("FACTS {}", s.facts());
    }
}
