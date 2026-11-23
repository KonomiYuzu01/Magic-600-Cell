//! From a claim group's binding to one KN6 check: the certificate the group's record enters as ([`CheckedAs`]), and
//! the stored inputs by role.
//!
//! A binding names its stored inputs by the edition's roles (`start_table`, `catalogue`, `record`, `step_member`,
//! `end_table`, `model`, `convention`); a sub-checker reads its inputs by its own role names (`table`, `jams`,
//! `certificate`, `start`, a member's path, ...). The table in [`checker_role`] is that translation, written from the
//! sub-checkers' sources and their own tests. It translates only: every byte supplied is a file the binding names,
//! read in place and checked against the edition's pin, and a role the binding does not supply stays absent, so a
//! check that needs it fails rather than reading a file the binding does not name. The bound model files are read by the
//! engines, which load them from the checkout's `assets/`, and are checked against their pins. Any other bound file a
//! sub-checker does not read (a producer's convention module, which no KN6 checker imports, a log, a record or a
//! catalogue it does not consult) is not supplied and is listed as unread. The menu is the one input no binding names:
//! it is the I_a menu record, accepted only with the edition's identity block ([`crate::support::menu`]).

#![allow(dead_code)]

use std::sync::Arc;

use c600_archive::bindings::{Binding, Bindings, CheckedAs};
use c600_checkers::archive;
use c600_checkers::CheckerKind;
use c600_session::persist::codec::{canonical, Json};

use crate::support::{directory, file_name, pinned, sha};

/// Stage 19's records the sub-checkers read by role (the archive's `records/` folder).
pub const Y: &str = "irregularity-descent-source-end-table.json.gz";
pub const Z: &str = "profile-family-validation-input-table.json.gz";
pub const Z_END: &str = "profile-family-validation-end-table.json.gz";
pub const POST_A: &str = "type-lex-independent-282-13-end-table.json.gz";
pub const COMPACT: &str = "type-lex-trimmed-macro-end-table.json.gz";
pub const CLEANUP: &str = "type-lex-recovery-transfer-joined-table.json.gz";
pub const RESULTS: &str = "profile-family-validation-results.json.gz";
pub const MACRO: &str = "type-lex-trimmed-macro.json";
pub const TRIMMED: &str = "type-lex-trimmed-independent-results.json";
pub const WEIGHTS: &str = "type-height-weight-results.json";
pub const COSETS: &str = "type-lex-282-results.json";

/// Stage 19's own manifest (PS3.14), the certificate a `table` group's record is stated by.
pub const S19_MANIFEST: &str = "research/jumbling/restoration/19-height-depth-framework/IMPORT-MANIFEST.json";

/// The largest decoded stage 19 member the sweep inflates.
const DECODED_LIMIT: usize = 1 << 26;

/// One check, ready to run: the sub-checker, the certificate and its stated digest, the inputs by role.
pub struct Prepared {
    pub id: &'static str,
    pub how: CheckedAs,
    /// The certificate's path: the group's record, its `via` record, or (for a table) the stage manifest.
    pub certificate_path: String,
    pub certificate: Arc<Vec<u8>>,
    /// The SHA-256 the certificate is stated to have: the edition's pin, the stage manifest's decoded digest for a
    /// stored gzip member, or the digest of the certificate built from the stage manifest's entry.
    pub digest: String,
    pub inputs: Vec<(String, Arc<Vec<u8>>)>,
    /// The role under which the group's record is supplied when it is not the certificate.
    pub record_role: Option<String>,
    /// Bound files the sub-checker does not read, which are not supplied: (the binding's role, path).
    pub unread: Vec<(String, String)>,
}

impl Prepared {
    pub fn kind(&self) -> CheckerKind {
        CheckerKind::Archive(self.id)
    }

    /// A key that is the same exactly when the check is: the sub-checker, the certificate's digest and every input's
    /// role and digest.
    pub fn key(&self) -> String {
        let mut key = format!("{} {}", self.id, sha(&self.certificate));
        for (role, bytes) in &self.inputs {
            key.push_str(&format!(" {role}={}", sha(bytes)));
        }
        key
    }
}

/// The registered sub-checker's static ID.
pub fn registered(id: &str) -> &'static str {
    archive::find(id).unwrap_or_else(|| panic!("{id} is not registered")).id()
}

/// Whether the sub-checker reads the I_a menu.
fn needs_menu(id: &str, certificate: &str) -> bool {
    match id {
        "s14.locality" | "s19.pose-table" | "s19.gamma-matching" => false,
        "s19.ranks" => file_name(certificate) == TRIMMED,
        _ => true,
    }
}

/// Stage 19's roles by file name, for the record a sub-checker checks.
fn stage19_role(id: &str, certificate: &str, name: &str) -> Option<&'static str> {
    let cert = file_name(certificate);
    let roles: &[(&str, &str)] = match (id, cert) {
        ("s19.gamma-matching", _) => &[(Y, "reference"), (Z, "input")],
        ("s19.gamma-witness", _) => &[(Y, "reference"), (Z, "input"), (Z_END, "end")],
        ("s19.frontier", MACRO) => &[(POST_A, "start"), (COMPACT, "end")],
        ("s19.frontier", TRIMMED) => {
            &[(RESULTS, "blocks"), (Y, "origin"), (POST_A, "start"), (COMPACT, "end"), (CLEANUP, "long")]
        }
        ("s19.ranks", WEIGHTS) => &[(COSETS, "source"), (Y, "start")],
        ("s19.ranks", TRIMMED) => &[(RESULTS, "blocks"), (Y, "origin"), (POST_A, "start")],
        _ => &[],
    };
    roles.iter().find(|(file, _)| *file == name).map(|(_, role)| *role)
}

/// The sub-checker's role for a bound file, or `None` when it does not read the file. `certificate` is the path of the
/// check's certificate; stage 18's members are named relative to its folder.
pub fn checker_role(id: &str, binding_role: &str, path: &str, certificate: &str) -> Option<String> {
    let name = file_name(path);
    let relative = || path.strip_prefix(directory(certificate)).map(str::to_owned);
    let fixed = |role: &str| Some(role.to_owned());
    match (id, binding_role) {
        ("s14.locality", "model") if path == "assets/manifest.json" => fixed("model"),
        (
            "s14.t3p" | "s14.literal" | "s14.levels" | "s14.blocks" | "s15.macro" | "s15.pstar" | "s16.macro"
            | "s16.release" | "s16.mobility" | "s16.reductions" | "s17.macro",
            "start_table",
        ) => fixed("table"),
        ("s15.macro" | "s16.macro" | "s17.macro" | "s16.release" | "s16.mobility" | "s16.reductions", "catalogue")
            if name == "jams.json" =>
        {
            fixed("jams")
        }
        ("s14.t3p" | "s15.macro" | "s16.macro" | "s17.macro", "record") => fixed("certificate"),
        ("s14.levels", "record") => fixed("word"),
        ("s15.pstar", "record") => fixed("chain"),
        ("s18.expansion" | "s18.composed" | "s18.tail_replay", "start_table") => fixed("start"),
        ("s18.expansion" | "s18.composed" | "s18.tail_replay", "end_table") => fixed("end"),
        ("s18.expansion" | "s18.composed" | "s18.tail_replay", "step_member") => relative(),
        ("s18.lattice_run" | "s18.lattice_replay", "record") => relative(),
        ("s19.gamma-matching" | "s19.gamma-witness" | "s19.frontier" | "s19.ranks", "record") => {
            stage19_role(id, certificate, name).map(str::to_owned)
        }
        _ => None,
    }
}

/// The binding role the group's record has when it enters a check as an input rather than as the certificate.
fn record_as_input(id: &str) -> &'static str {
    match id {
        "s18.expansion" | "s18.composed" | "s18.tail_replay" => "step_member",
        _ => "record",
    }
}

/// The files `s18.lattice_replay` reads, by role: the menu, the run's start table, its words and its step tables (the
/// run record names them; the sub-checker refuses any other input).
fn lattice_replay_roles(run: &[u8]) -> Vec<String> {
    let doc = c600_checkers::claims::document(run).ok().expect("the run record is JSON");
    let mut roles = vec!["menu".to_owned()];
    roles.extend(doc.get("start").and_then(|s| s.get("file")).and_then(|f| f.as_str()).map(str::to_owned));
    for step in doc.get("steps").and_then(|s| s.as_array()).expect("the run's steps") {
        roles.extend(step.get("word").and_then(|w| w.as_str()).map(str::to_owned));
        roles.extend(step.get("table").and_then(|t| t.as_str()).map(str::to_owned));
    }
    roles
}

/// Stage 19's manifest entry for a member, by its path relative to the manifest's folder: (sha256, stored bytes,
/// uncompressed sha256, uncompressed bytes).
pub fn manifest_entry(manifest: &[u8], member: &str) -> (String, i64, String, i64) {
    let doc = c600_checkers::claims::document(manifest).ok().expect("the stage manifest is JSON");
    let files = doc.get("files").and_then(|f| f.as_array()).expect("the manifest's files");
    let entry = files
        .iter()
        .find(|f| f.get("path").and_then(|p| p.as_str()) == Some(member))
        .unwrap_or_else(|| panic!("{member} has no stage manifest entry"));
    let text = |key: &str| entry.get(key).and_then(|v| v.as_str()).expect(key).to_owned();
    let int = |key: &str| entry.get(key).and_then(|v| v.as_i64()).expect(key);
    (text("sha256"), int("stored_bytes"), text("uncompressed_sha256"), int("uncompressed_bytes"))
}

fn stage_manifest(bindings: &Bindings) -> Arc<Vec<u8>> {
    let pin = bindings.files.iter().find(|f| f.path == S19_MANIFEST).expect("the edition pins stage 19's manifest");
    pinned(pin)
}

/// A stored stage 19 member decoded, with the decoded digest its stage manifest states (checked here).
fn decoded(bindings: &Bindings, path: &str, stored: &[u8]) -> (Vec<u8>, String) {
    let member = path.strip_prefix(directory(S19_MANIFEST)).expect("a stage 19 member");
    let (stored_sha, _, decoded_sha, decoded_bytes) = manifest_entry(&stage_manifest(bindings), member);
    assert_eq!(sha(stored), stored_sha, "{path}: the stored bytes are the stage manifest's");
    let bytes = c600_checkers::archive::s19::common::decode(stored, DECODED_LIMIT)
        .unwrap_or_else(|e| panic!("{path}: {e}"))
        .into_owned();
    assert_eq!(sha(&bytes), decoded_sha, "{path}: the decoded bytes are the stage manifest's");
    assert_eq!(bytes.len() as i64, decoded_bytes, "{path}");
    (bytes, decoded_sha)
}

/// The `kn6-s19-pose-table/1` certificate of a stored table: its stage manifest entry and the table's own digest, as
/// stage 19's tests state it.
fn pose_certificate(bindings: &Bindings, path: &str, stored: &[u8]) -> Vec<u8> {
    let member = path.strip_prefix(directory(S19_MANIFEST)).expect("a stage 19 member");
    let (stored_sha, stored_bytes, decoded_sha, decoded_bytes) = manifest_entry(&stage_manifest(bindings), member);
    let (table, _) = decoded(bindings, path, stored);
    let doc = c600_checkers::claims::document(&table).ok().expect("the table is JSON");
    let digest = doc.get("digest").and_then(|d| d.as_str()).expect("the table's digest").to_owned();
    canonical(&Json::object([
        ("format", Json::text("kn6-s19-pose-table/1")),
        ("path", Json::text(member)),
        ("sha256", Json::text(&stored_sha)),
        ("stored_bytes", Json::Int(stored_bytes)),
        ("uncompressed_sha256", Json::text(&decoded_sha)),
        ("uncompressed_bytes", Json::Int(decoded_bytes)),
        ("digest", Json::text(&digest)),
    ]))
}

/// Prepares the check of `row` by sub-checker `id`, its record entering as `how`.
pub fn prepare(bindings: &Bindings, row: &Binding, id: &str, how: CheckedAs) -> Prepared {
    let id = registered(id);
    let record = bindings.file(row.source);
    let (certificate_path, certificate, digest, record_role) = match how {
        CheckedAs::Certificate | CheckedAs::ScopeOnly => {
            let stored = pinned(record);
            if record.path.ends_with(".gz") {
                let (bytes, digest) = decoded(bindings, &record.path, &stored);
                (record.path.clone(), Arc::new(bytes), digest, None)
            } else {
                (record.path.clone(), stored, record.sha256.clone(), None)
            }
        }
        CheckedAs::Input => {
            let via = bindings.file(row.via.expect("an input group names its certificate"));
            let role = checker_role(id, record_as_input(id), &record.path, &via.path)
                .unwrap_or_else(|| panic!("{}: {id} reads no role for the record", row.group));
            (via.path.clone(), pinned(via), via.sha256.clone(), Some(role))
        }
        CheckedAs::Table => {
            let via = bindings.file(row.via.expect("a table group names its stage manifest"));
            assert_eq!(via.path, S19_MANIFEST, "{}", row.group);
            let bytes = pose_certificate(bindings, &record.path, &pinned(record));
            let digest = sha(&bytes);
            (via.path.clone(), Arc::new(bytes), digest, Some("table".to_owned()))
        }
    };
    let mut inputs: Vec<(String, Arc<Vec<u8>>)> = Vec::new();
    let mut unread = Vec::new();
    if needs_menu(id, &certificate_path) {
        inputs.push(("menu".to_owned(), Arc::new(crate::support::menu().to_vec())));
    }
    for (file, binding_role) in bindings.stored(row) {
        let pin = bindings.file(file);
        if pin.path == certificate_path {
            continue;
        }
        let role = if file == row.source {
            match how {
                CheckedAs::Certificate | CheckedAs::ScopeOnly => continue,
                _ => record_role.clone(),
            }
        } else {
            checker_role(id, binding_role, &pin.path, &certificate_path)
        };
        match role {
            Some(role) => {
                assert!(inputs.iter().all(|(r, _)| *r != role), "{}: two bound files for the role {role}", row.group);
                inputs.push((role, pinned(pin)));
            }
            // The model is read by the engines, loaded from the checkout's `assets/`: the bound model files are those
            // bytes, checked against their pins.
            None if binding_role == "model" && pin.path.starts_with("assets/") => {
                pinned(pin);
            }
            None => unread.push((binding_role.to_owned(), pin.path.clone())),
        }
    }
    if id == "s18.lattice_replay" {
        let named = lattice_replay_roles(&certificate);
        let (kept, dropped): (Vec<_>, Vec<_>) = inputs.into_iter().partition(|(role, _)| named.contains(role));
        inputs = kept;
        unread.extend(
            dropped
                .into_iter()
                .map(|(role, _)| ("record".to_owned(), format!("{}{role}", directory(&certificate_path)))),
        );
    }
    inputs.sort_by(|a, b| a.0.cmp(&b.0));
    Prepared { id, how, certificate_path, certificate, digest, inputs, record_role, unread }
}
