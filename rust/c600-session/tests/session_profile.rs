//! The session's profile in the command layer's journal (BND.17; draft): the session record keeps it once, at
//! creation, a reopen reads it back, and a session record without it, as journals written before it have, reads as
//! `solving`. Every test uses fresh memory disks and no other session.

mod common;

use c600_command::{CommandLayer, Profile, Secret, Table};
use c600_kernel::json::{self, Value};
use c600_kernel::ProtectedSet;
use c600_session::disk::MemDisk;
use c600_session::journal::{self, Tail};
use c600_session::JournalStore;

const JOURNAL: &str = "journal";

/// A new session in `profile` at the solved state on an empty memory disk.
fn create(disk: &MemDisk, profile: Profile) {
    let k = common::kernel();
    let store = JournalStore::create(Box::new(disk.clone())).expect("an empty disk takes a session");
    let secret = Secret::fixed([7; 32]);
    let solved = k.solved();
    let layer =
        CommandLayer::new_with_profile(k, Table::h08(), secret, store, "s1", solved, ProtectedSet::none(), profile);
    assert!(layer.recovery().is_none(), "{:?}", layer.recovery());
}

/// The profile of the session the disk holds, as a restart reopens it, or the code the open is refused with.
fn reopened(disk: &MemDisk) -> Result<Profile, &'static str> {
    disk.end_holder();
    let store = JournalStore::open(Box::new(disk.clone())).map_err(|e| e.code)?;
    let layer =
        CommandLayer::open(common::kernel(), Table::h08(), Secret::fixed([9; 32]), store).map_err(|e| e.code)?;
    assert!(layer.recovery().is_none(), "{:?}", layer.recovery());
    Ok(layer.profile())
}

/// The journal's records, in order.
fn records(disk: &MemDisk) -> Vec<Value> {
    let bytes = disk.durable(JOURNAL).expect("the disk holds a journal");
    let scan = journal::scan(&bytes);
    assert_eq!(scan.tail, Tail::Clean);
    scan.payloads.iter().map(|payload| json::parse(payload).expect("a record")).collect()
}

/// Replaces the journal with these records, each framed as the store frames it.
fn write(disk: &MemDisk, records: &[Value]) {
    let mut bytes = Vec::new();
    for record in records {
        bytes.extend(journal::frame(record.canonical().as_bytes()));
    }
    disk.overwrite(JOURNAL, &bytes);
}

/// The journal with its session record's fields changed by `edit`.
fn edited(disk: &MemDisk, edit: impl FnOnce(&mut std::collections::BTreeMap<String, Value>)) {
    let mut all = records(disk);
    let Value::Object(fields) = &mut all[0] else { panic!("the session record is an object") };
    edit(fields);
    write(disk, &all);
}

/// BND.17 (draft): the session record keeps the profile by BND.15's name, and a reopen reads it back.
#[test]
fn the_journal_keeps_the_profile_the_session_was_created_in() {
    for profile in Profile::ALL {
        let disk = MemDisk::new();
        create(&disk, profile);
        let first = &records(&disk)[0];
        assert_eq!(first.get("profile").and_then(Value::as_str), Some(profile.name()));
        assert_eq!(reopened(&disk), Ok(profile));
        assert_eq!(reopened(&disk), Ok(profile), "a second reopen reads the same profile");
    }
}

/// A session record without `profile`, as journals written before it have, reads as `solving`. A profile with no
/// BND.15 name, or a record with `profile` that lacks another field (seven fields in all), does not read.
#[test]
fn a_session_record_without_a_profile_reads_as_solving() {
    let disk = MemDisk::new();
    create(&disk, Profile::Research);
    edited(&disk, |fields| {
        fields.remove("profile");
    });
    assert_eq!(reopened(&disk), Ok(Profile::Solving));

    let disk = MemDisk::new();
    create(&disk, Profile::ToolAuthoring);
    edited(&disk, |fields| {
        fields.insert("profile".into(), Value::String("practice".into()));
    });
    assert_eq!(reopened(&disk), Err("KN4:input.unknown_session"));

    let disk = MemDisk::new();
    create(&disk, Profile::ToolAuthoring);
    edited(&disk, |fields| {
        fields.remove("orbits");
    });
    assert_eq!(reopened(&disk), Err("KN4:input.unknown_session"));
}
