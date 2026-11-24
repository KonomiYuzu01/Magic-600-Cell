//! Soft disclosure (LP4-11's `c600-command` part; EV5.8, EV5.9; draft): the goal flows' keys and their concepts
//! against the learning path and the glossary, and the rule that disclosure never locks a command.
//!
//! CT-EV5-02 (part: flow goals need no undisclosed concept: every H-08 row a learning step's goal flow cites is a key
//! of `c600_command::KEYS` that opens at the first step citing it, and each of its concepts is a learner-facing
//! glossary row that this step or a step before it on the path teaches, or that the path assumes; planted keys that
//! need a concept taught later, or by a step off the path, are refused; not run: that the proposed changes have been
//! reviewed, because every step and every LP4-11 row still records in `review` that it waits for the mathematician's
//! review (EV5.14), and the keys' concepts are LP4-11's proposal under the same review; the other parts are in
//! `tests/test_glossary.py`).
//! CT-EV5-04 (part: ordinary soft disclosure never locks commands: in every disclosure state, nothing opened, each
//! step opened in the path's order, and "open all", the palette lists every H-08 row, each row's availability and the
//! shell's preparation of each row are those of the first state, "show all keys" shows every key and changes no
//! disclosure state, "open all" is reversible, and no source of the layer reads the disclosure state; not run: BS3's
//! per-principle capability gate of EV5.9, which waits for LQ2's option set and its joint setting, so nothing is built
//! for it and no learning result grants or withholds a capability).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{CommandLayer, Disclosure, Key, MemoryStore, Secret, Shell, Table, KEYS, SOURCES, STEPS};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    KERNEL
        .get_or_init(|| match Kernel::load(&repository().join("assets")) {
            Ok(kernel) => Arc::new(kernel),
            Err(error) => panic!("the model in assets/ does not load: {error}"),
        })
        .clone()
}

fn read(name: &str) -> Value {
    let path = repository().join("docs/progress/1.0/glossary").join(name);
    json::parse(&std::fs::read(&path).unwrap()).unwrap_or_else(|_| panic!("{name} is JSON"))
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value.and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect()
}

/// The H-08 rows a route text cites: its back-quoted names that are rows of the table.
fn cited(routes: &str, table: &Table) -> Vec<String> {
    routes.split('`').skip(1).step_by(2).filter(|name| table.row(name).is_some()).map(str::to_owned).collect()
}

/// The path's data as the check reads it.
struct Path7 {
    /// The steps in order, each with its cited rows, the steps it comes after and the terms it teaches.
    steps: Vec<(String, Vec<String>, Vec<String>, Vec<String>)>,
    assumed: BTreeSet<String>,
    /// Learner-facing glossary terms, in lower case.
    terms: BTreeSet<String>,
}

fn path_data() -> Path7 {
    let table = Table::h08();
    let path = read("learning-path.json");
    let glossary = read("terms.json");
    let steps = path
        .get("steps")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .map(|step| {
            let id = step.get("id").and_then(Value::as_str).unwrap().to_owned();
            let routes = step.get("routes").and_then(Value::as_str).unwrap_or_default();
            let mut taught = strings(step.get("concepts"));
            taught.extend(strings(step.get("closure")));
            (id, cited(routes, &table), strings(step.get("after")), taught)
        })
        .collect();
    let terms = glossary
        .get("terms")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .filter(|row| row.get("learner_facing") != Some(&Value::Bool(false)))
        .filter_map(|row| row.get("term").and_then(Value::as_str).map(str::to_lowercase))
        .collect();
    let assumed = strings(path.get("assumed")).iter().map(|t| t.to_lowercase()).collect();
    Path7 { steps, assumed, terms }
}

/// The check of the flow goals against the disclosure data (CT-EV5-02's part, K3, charter bar M6): every cited row is
/// a key, at the first step citing it, and no key needs a concept its step has not taught by then.
fn flow_goal_errors(path: &Path7, keys: &[(String, String, Vec<String>)]) -> Vec<String> {
    let mut errors = Vec::new();
    let after: BTreeMap<&str, &[String]> = path.steps.iter().map(|(id, _, a, _)| (id.as_str(), &a[..])).collect();
    let teaches: BTreeMap<&str, &[String]> = path.steps.iter().map(|(id, _, _, t)| (id.as_str(), &t[..])).collect();
    let taught_by = |step: &str| -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut todo = vec![step.to_owned()];
        while let Some(s) = todo.pop() {
            if seen.insert(s.clone()) {
                todo.extend(after.get(s.as_str()).copied().unwrap_or_default().iter().cloned());
            }
        }
        let mut terms = path.assumed.clone();
        for s in seen {
            terms.extend(teaches.get(s.as_str()).copied().unwrap_or_default().iter().map(|t| t.to_lowercase()));
        }
        terms
    };
    let mut first: BTreeMap<&str, &str> = BTreeMap::new();
    for (id, rows, _, _) in &path.steps {
        for row in rows {
            first.entry(row.as_str()).or_insert(id.as_str());
        }
    }
    let mut seen = BTreeSet::new();
    for (row, step, concepts) in keys {
        if !seen.insert(row.as_str()) {
            errors.push(format!("{row}: listed twice"));
        }
        match first.get(row.as_str()) {
            None => errors.push(format!("{row}: no goal flow cites it")),
            Some(opens) if opens != step => errors.push(format!("{row}: opens at {step}, but {opens} cites it first")),
            Some(_) => {}
        }
        let taught = taught_by(step);
        for concept in concepts {
            let lower = concept.to_lowercase();
            if !path.terms.contains(&lower) {
                errors.push(format!("{row}: '{concept}' is no learner-facing glossary term"));
            } else if !taught.contains(&lower) {
                errors.push(format!("{row}: needs '{concept}', which nothing up to {step} teaches"));
            }
        }
    }
    for row in first.keys() {
        if !seen.contains(row) {
            errors.push(format!("{row}: a goal flow cites it, but it is no key"));
        }
    }
    errors
}

fn owned(keys: &[Key]) -> Vec<(String, String, Vec<String>)> {
    keys.iter()
        .map(|key| (key.row.to_owned(), key.step.to_owned(), key.concepts.iter().map(|c| (*c).to_owned()).collect()))
        .collect()
}

#[test]
fn goal_flows_need_no_undisclosed_concept() {
    let path = path_data();
    assert_eq!(path.steps.iter().map(|(id, ..)| id.as_str()).collect::<Vec<_>>(), STEPS, "the path's steps");
    let keys = owned(&KEYS);
    assert_eq!(flow_goal_errors(&path, &keys), Vec::<String>::new());

    // Each refusal can fail: a key that needs a concept a later step teaches, a key of a step off the path of the
    // step that teaches its concept (step 4 comes after step 1 only), a concept that is no glossary term, a key at
    // the wrong step, a cited row left out, and a key no flow cites.
    let planted = |change: &dyn Fn(&mut Vec<(String, String, Vec<String>)>)| {
        let mut keys = keys.clone();
        change(&mut keys);
        flow_goal_errors(&path, &keys)
    };
    let errors = planted(&|k| k[0].2.push("Macro".into()));
    assert!(errors.iter().any(|e| e.contains("command.find: needs 'Macro'")), "{errors:?}");
    let errors = planted(&|k| k.iter_mut().find(|k| k.0 == "worksheet.save").unwrap().2.push("Binding".into()));
    assert!(errors.iter().any(|e| e.contains("worksheet.save: needs 'Binding'")), "{errors:?}");
    let errors = planted(&|k| k[0].2.push("Not a term".into()));
    assert!(errors.iter().any(|e| e.contains("no learner-facing glossary term")), "{errors:?}");
    let errors = planted(&|k| k.iter_mut().find(|k| k.0 == "operation.check").unwrap().1 = "S4".into());
    assert!(errors.iter().any(|e| e.contains("operation.check: opens at S4, but S1 cites it first")), "{errors:?}");
    let errors = planted(&|k| k.retain(|k| k.0 != "job.stop"));
    assert!(errors.iter().any(|e| e.contains("job.stop: a goal flow cites it")), "{errors:?}");
    let errors = planted(&|k| k.push(("twist.apply".into(), "S4".into(), vec!["Recipe".into()])));
    assert!(errors.iter().any(|e| e.contains("twist.apply: no goal flow cites it")), "{errors:?}");
}

/// The disclosure states the CT-EV5-04 walk visits: nothing opened, each step opened in the path's order, open all.
fn states() -> Vec<Disclosure> {
    let mut states = vec![Disclosure::new()];
    let mut state = Disclosure::new();
    for step in STEPS {
        assert!(state.open_step(step), "{step}");
        states.push(state.clone());
    }
    let mut all = Disclosure::new();
    all.set_open_all(true);
    states.push(all);
    states
}

#[test]
fn soft_disclosure_never_locks_a_command() {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    let rows: Vec<String> = layer.table().rows().map(|row| row.id.clone()).collect();
    let mut shell = Shell::new(Secret::fixed(SECRET), "shell");
    let outcome = |shell: &mut Shell, layer: &CommandLayer| -> Vec<String> {
        rows.iter()
            .map(|row| {
                let available = layer.availability(row).map(|_| "available").unwrap_or_else(|e| e.code);
                let args = json::object([]);
                let prepared = shell.prepare(layer, row, args).map(|_| "prepared").unwrap_or_else(|e| e.code);
                format!("{row}: {available}, {prepared}")
            })
            .collect()
    };
    let first = outcome(&mut shell, &layer);
    for state in states() {
        shell.disclosure = state.clone();
        let palette: Vec<String> = shell.read_palette(&layer).entries().iter().map(|e| e.row.clone()).collect();
        assert_eq!(palette, rows, "the palette lists every command: {:?}", state.opened());
        assert_eq!(outcome(&mut shell, &layer), first, "no state locks a command: {:?}", state.opened());
        // "Show all keys" shows every key and changes no disclosure state.
        assert!(rows.iter().all(|row| state.shows(row, true)));
        assert_eq!(shell.disclosure, state);
    }

    // A key opens with its step and is new until used; open all is a saved choice that reverses.
    let mut d = Disclosure::new();
    let find = KEYS[0].row;
    assert!(!d.is_open(find) && d.is_open("twist.apply"), "a goal key starts closed; a key no flow cites is open");
    assert!(d.open_step("S1") && !d.open_step("S1") && !d.open_step("S9"));
    assert!(d.is_open(find) && d.is_new(find));
    d.use_key(find);
    assert!(d.is_open(find) && !d.is_new(find));
    let before = d.clone();
    d.set_open_all(true);
    assert!(KEYS.iter().all(|key| d.is_open(key.row)));
    d.set_open_all(false);
    assert_eq!(d, before, "open all reverses");

    // No source of the layer reads the disclosure state: only the module, the crate root's exports and the shell's
    // field name it, and the shell never reads its own field.
    let rust = SOURCES.iter().filter(|(name, _)| name.starts_with("c600-command/src/") && name.ends_with(".rs"));
    for (name, bytes) in rust {
        let text = String::from_utf8_lossy(bytes);
        let names_it = text.contains("Disclosure") || text.contains("disclosure::") || text.contains(".disclosure");
        let allowed = ["c600-command/src/disclosure.rs", "c600-command/src/lib.rs", "c600-command/src/shell.rs"];
        assert!(!names_it || allowed.contains(name), "{name} reads the disclosure state");
        if *name == "c600-command/src/shell.rs" {
            assert!(!text.contains("self.disclosure"), "the shell reads its disclosure state");
        }
    }
}
