//! CT-DET-01 (part: a KN6 check job's `ReproKey`). The LP2 half (`c600-run/tests/conformance_det.rs`) ran the key's
//! fields for interpreter runs; its "not run" part runs here: key equality with the menu order, the cost table and
//! the checker identity among the fields, and a key that names no machine, user, locale or location.
//!
//! The locale, user and machine part runs the same job in a child process of this test binary whose locale, time
//! zone, user and host names and working directory differ, and compares the key and payload digests. It varies what
//! the process can see; a second machine is CI's run on Linux, which compares CT-DET-02's pinned payloads.

use c600_checkers::CheckerKind;
use c600_research::record::admission_now;
use c600_research::{Budget, Check, CheckJob, Pinned};
use c600_session::persist::codec::{canonical, Json};
use c600_session::persist::record::identity_json;
use c600_session::persist::run::{self, codes, policy_identity, Grant, Key, Reproduction, Seed};

use crate::common::*;

fn a_job() -> CheckJob {
    let mut capped = short_check(&[9, -9, 4]);
    capped.cap = Some(Budget::new(5, 500, 1 << 20, 1 << 20));
    stated_job(vec![short_check(&[1, 2, 3]), capped], WIDE)
}

fn key_of(job: CheckJob) -> Key {
    admit(job).key().clone()
}

/// Sets `field` of the key's FP.5 value and reads the key back.
fn with(key: &Key, change: impl FnOnce(&mut std::collections::BTreeMap<String, Json>)) -> Key {
    let mut value = key.json();
    let Json::Object(map) = &mut value else { panic!("a key is an object") };
    change(map);
    Key::read(&value).expect("the changed key is well formed")
}

fn sha_ref(role: &str, sha256: &str) -> Json {
    Json::object([("role", Json::text(role)), ("sha256", Json::text(sha256))])
}

/// CT-DET-01 (part: every field of a check job's key alone changes the key, the checker identity, the cost table and
/// the menu order included).
#[test]
fn ct_det_01_each_key_field_alone_changes_the_key() {
    let key = key_of(a_job());
    let other = hex_of("another digest");
    let mut changed: Vec<(&str, Key)> = Vec::new();
    changed
        .push(("source", with(&key, |m| drop(m.insert("source".into(), Json::List(vec![sha_ref("source", &other)]))))));
    changed.push(("modules", {
        let mut k = key.clone();
        k.modules[0].1 = other.clone();
        k
    }));
    changed.push(("edition", with(&key, |m| drop(m.insert("edition".into(), Json::text("6cs-draft-2"))))));
    changed.push(("ir_version", with(&key, |m| drop(m.insert("ir_version".into(), Json::text("ir-draft-1"))))));
    changed.push(("identity.model_id", {
        let mut k = key.clone();
        k.identity.model_id.push_str("-other");
        k
    }));
    changed.push(("input.digest", with(&key, |m| drop(m.insert("input".into(), sha_ref("labels", &other))))));
    changed.push(("input.level", with(&key, |m| drop(m.insert("input".into(), sha_ref("poses", &key.input))))));
    changed.push(("parameters.args", {
        let mut k = key.clone();
        k.parameters.args.swap(0, 1);
        k
    }));
    changed.push(("parameters.captured", {
        let mut k = key.clone();
        k.parameters.captured[0].sha256 = other.clone();
        k
    }));
    changed.push(("parameters.grants", {
        let mut k = key.clone();
        k.parameters.grants.push(Grant {
            kind: "file".into(),
            rights: vec!["read".into()],
            scope: "corpus".into(),
            grantee: other.clone(),
        });
        k
    }));
    changed.push(("parameters.profile", {
        let mut value = key.json();
        if let Json::Object(m) = &mut value {
            if let Some(Json::Object(p)) = m.get_mut("parameters") {
                p.insert("profile".into(), Json::text("tool_authoring"));
            }
        }
        Key::read(&value).expect("well formed")
    }));
    changed.push(("parameters.policy", {
        let mut k = key.clone();
        k.parameters.policy = policy_identity(&["a declassifier"]);
        k
    }));
    changed.push(("seeds", {
        let mut k = key.clone();
        k.seeds.push(Seed {
            sha256: other.clone(),
            path: vec!["s".into()],
            algorithm: "sha256-counter-draft-1".into(),
        });
        k
    }));
    changed.push(("budgets.limits", {
        let mut k = key.clone();
        k.limits.kernel_cost += 1;
        k
    }));
    changed.push(("budgets.metering", {
        let mut k = key.clone();
        k.metering.source[0] ^= 1;
        k
    }));
    changed.push(("kernel_cost_table", {
        let mut k = key.clone();
        k.kernel_cost_table.source[0] ^= 1;
        k
    }));
    changed.push(("kernel_api", {
        let mut k = key.clone();
        k.kernel_api.version.push_str("-other");
        k
    }));
    changed.push(("checkers.source", {
        let mut k = key.clone();
        k.checkers[0].source[0] ^= 1;
        k
    }));
    changed.push(("checkers.id", {
        let mut k = key.clone();
        k.checkers[0].version.push_str("-other");
        k
    }));
    for (field, k) in &changed {
        assert_ne!(k, &key, "{field}");
        assert_ne!(k.digest(), key.digest(), "{field}");
        assert_eq!(Key::read(&k.json()).as_ref().map(Key::digest).ok(), Some(k.digest()), "{field} reads back");
    }
}

/// CT-DET-01 (part: the menu order and the menu identity of a jumbling input are key fields).
#[test]
fn ct_det_01_menu_order_is_a_key_field() {
    let block = loaded().jumbling_identity(&menu_record()).expect("the menu reads");
    let pinned = |block: &c600_session::persist::codec::Json| {
        let mut job = stated_job(vec![short_check(&[1])], WIDE);
        job.input = Pinned::stated("poses", &hex_of("a table"), block.clone());
        key_of(job)
    };
    let base = identity_json(&block);
    let key = pinned(&base);
    assert!(key.identity.menu_order.is_some());
    for field in ["menu_order", "menu_id", "contract_rev"] {
        let mut value = base.clone();
        if let Json::Object(m) = &mut value {
            m.insert(field.into(), Json::text(&hex_of(field)));
        }
        let other = pinned(&value);
        assert_ne!(other.digest(), key.digest(), "{field}");
    }
}

/// CT-DET-01 (part: the key admitted under any change of a recorded implementation field refuses reproduction with
/// `LG4:input.repro_key_mismatch`, and of the profile, policy or grants with `LG3:capability.grant_mismatch`, before
/// anything runs).
#[test]
fn ct_det_01_a_changed_implementation_refuses_reproduction_before_running() {
    let run = run_job(a_job());
    let now = admission_now(run.key(), &engines()).expect("the host's admission");
    assert_eq!(now, run::Admission::of(run.key()), "this build admits its own record");
    let refuse = |change: &dyn Fn(&mut run::Admission)| {
        let mut changed = now.clone();
        change(&mut changed);
        run::reproduce(run.bytes(), &run.available(), &changed, |_, _| panic!("admission precedes execution"))
    };
    let mismatch = Reproduction::Refused(codes::KEY_MISMATCH.into());
    assert_eq!(refuse(&|a| a.checkers[0].source[0] ^= 1), mismatch, "a checker's source");
    assert_eq!(refuse(&|a| a.checkers[0].version.push('x')), mismatch, "a checker's ID");
    assert_eq!(refuse(&|a| a.checkers.clear()), mismatch, "a checker that is not available");
    assert_eq!(refuse(&|a| a.kernel_cost_table.source[0] ^= 1), mismatch, "the cost table");
    assert_eq!(refuse(&|a| a.kernel_api.source[0] ^= 1), mismatch, "the kernel");
    assert_eq!(refuse(&|a| a.metering.source[0] ^= 1), mismatch, "the metering rule");
    assert_eq!(refuse(&|a| a.edition.push('x')), mismatch, "the edition");
    assert_eq!(refuse(&|a| a.ir_version.push('x')), mismatch, "the IR version");
    assert_eq!(refuse(&|a| a.identity.model_id.push('x')), mismatch, "the model");
    let grant = Reproduction::Refused(codes::GRANT_MISMATCH.into());
    assert_eq!(refuse(&|a| a.policy = policy_identity(&["x"])), grant, "the policy");
    assert_eq!(
        refuse(&|a| a.grants.push(Grant {
            kind: "clock".into(),
            rights: vec!["read".into()],
            scope: "wall".into(),
            grantee: hex_of("m")
        })),
        grant,
        "a grant"
    );
}

/// CT-DET-01 (part: the job adapter's own source at reproduction): a stored record whose key names another adapter
/// source and module digest is refused with `LG4:input.repro_key_mismatch` even when those bytes are available, so this
/// build never re-runs under a key that names another implementation.
#[test]
fn ct_det_01_another_adapter_source_refuses_reproduction() {
    let run = run_job(a_job());
    let listing = b"another adapter listing".to_vec();
    let other = sha(&listing);
    let mut forged = run.record().clone();
    let adapter = forged.key.source[0].clone();
    forged.key.source = vec![other.clone()];
    for module in forged.key.modules.iter_mut() {
        if module.1 == adapter {
            module.1 = other.clone();
        }
    }
    // The provenance names the producing module too; a record is written only when the two agree.
    fn rename(value: &mut Json, from: &str, to: &str) {
        match value {
            Json::Text(text) if text == from => *text = to.to_owned(),
            Json::List(items) => items.iter_mut().for_each(|item| rename(item, from, to)),
            Json::Object(fields) => fields.values_mut().for_each(|item| rename(item, from, to)),
            _ => {}
        }
    }
    rename(&mut forged.provenance, &adapter, &other);
    let bytes = forged.write().expect("a well-formed record");
    let mut available = run.available();
    available.insert(other, listing);
    let answer = c600_research::reproduce(&engines(), &bytes, &available, &c600_research::Cancel::new());
    assert_eq!(answer.answer, Reproduction::Refused(codes::KEY_MISMATCH.into()));
    assert!(answer.run.is_none(), "nothing ran under the forged key");
}

/// CT-DET-01 (part: what is not a key field changes nothing: the directory the inputs were read from, the thread the
/// job was admitted on, and the machine and user; the key's bytes name none of them).
#[test]
fn ct_det_01_paths_threads_machine_and_user_are_not_key_fields() {
    let scratch = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("r8-ct-det-01");
    let certificate = expansion(&[3, 1, 4, 1, 5], None);
    let mut keys = Vec::new();
    for place in ["a", "b/deeper/still"] {
        let dir = scratch.join(place);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let file = dir.join("certificate.json");
        std::fs::write(&file, &certificate).expect("written");
        let bytes = std::fs::read(&file).expect("read back");
        keys.push(key_of(stated_job(vec![Check::new(CheckerKind::Expansion, bytes, Vec::new())], WIDE)));
    }
    assert_eq!(keys[0], keys[1], "the directory is not a key field");
    let there = std::thread::spawn(move || {
        key_of(stated_job(vec![Check::new(CheckerKind::Expansion, certificate, Vec::new())], WIDE)).digest()
    })
    .join()
    .expect("the thread ends");
    assert_eq!(there, keys[0].digest(), "the thread is not a key field");

    let run = run_job(stated_job(vec![short_check(&[2, 7, 1, 8])], WIDE));
    let text = String::from_utf8_lossy(run.bytes()).into_owned();
    let mut names: Vec<String> = vec![repository().display().to_string(), scratch.display().to_string()];
    for var in ["USERNAME", "USER", "COMPUTERNAME", "HOSTNAME", "USERDOMAIN", "HOME", "USERPROFILE"] {
        if let Ok(value) = std::env::var(var) {
            names.push(value);
        }
    }
    // A short value, or one that is hex, could occur inside a digest by chance; the others must not occur at all.
    let checked = names.iter().filter(|n| n.len() >= 4 && !n.chars().all(|c| c.is_ascii_hexdigit()));
    for (i, name) in checked.enumerate() {
        assert!(!text.contains(name.as_str()), "the record names host value {i} (value not printed)");
    }
    let key_text = String::from_utf8(canonical(&run.key().json())).expect("UTF-8");
    assert!(!key_text.contains(":\\") && !key_text.contains("/home/") && !key_text.contains("\\\\"));
}

/// The environment variable that makes [`ct_det_01_locale_user_and_machine_are_not_key_fields`] print its digests
/// instead of starting its probe.
const PROBE: &str = "C600_R8_LOCALE_PROBE";

/// CT-DET-01 (part: locale, user and machine): the same job in a child process whose locale, time zone, user and
/// host names and working directory differ seals the same key and writes the same exact payload.
#[test]
fn ct_det_01_locale_user_and_machine_are_not_key_fields() {
    let mut capped = long_check(10, 10);
    capped.cap = Some(Budget { kernel_cost: 7, ..WIDE });
    let run = run_job(stated_job(vec![short_check(&[2, 7, 1, 8]), capped], WIDE));
    let line = format!("r8-probe key={} payload={}", run.key().digest(), payload_digest(&run));
    if std::env::var_os(PROBE).is_some() {
        println!("{line}");
        return;
    }
    let elsewhere = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("r8-ct-det-01-probe");
    std::fs::create_dir_all(&elsewhere).expect("a scratch directory");
    let output = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args(["ct_det_01::ct_det_01_locale_user_and_machine_are_not_key_fields", "--exact", "--nocapture"])
        .args(["--test-threads=1"])
        .current_dir(&elsewhere)
        .env(PROBE, "1")
        .env("LANG", "tr_TR.UTF-8")
        .env("LC_ALL", "tr_TR.UTF-8")
        .env("LC_NUMERIC", "de_DE.UTF-8")
        .env("TZ", "Pacific/Chatham")
        .env("USERNAME", "r8-probe-user")
        .env("USER", "r8-probe-user")
        .env("COMPUTERNAME", "R8-PROBE-HOST")
        .env("HOSTNAME", "r8-probe-host")
        .output()
        .expect("the probe starts");
    assert!(output.status.success(), "the probe failed: {}", String::from_utf8_lossy(&output.stderr));
    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(printed.contains(&line), "another locale, user or machine changed the key or the payload: {printed}");
}
