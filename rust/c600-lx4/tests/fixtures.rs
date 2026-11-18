//! LX-4's module set and edit script (RTE.48, RTE.51 steps 1 and 2): sizes, determinism, the import graph, the
//! front end's agreement, and the edit replay's refusals.

use c600_lx4::{
    apply, edits, edits_json, fragments, generate, manifest_json, validate, Expected, Fixtures, BODY, EDIT_KINDS,
    EDIT_TARGETS, SIZES, TOOLS, W_LINES, W_MODULES,
};

fn fixtures() -> Fixtures {
    generate().expect("the tools' declarations make a module set")
}

#[test]
fn the_module_set_is_deterministic_and_has_rte48_sizes() {
    let fx = fixtures();
    assert_eq!(fx, fixtures());
    for (m, (name, lines)) in fx.singles.iter().zip(SIZES) {
        assert_eq!((m.name.as_str(), m.lines()), (name, lines));
        assert!(m.source.ends_with('\n') && !m.source.contains('\r'));
    }
    assert_eq!(fx.workspace.len(), W_MODULES);
    assert!(fx.workspace.iter().all(|m| m.lines() == W_LINES));
    assert_eq!(fx.workspace.iter().map(|m| m.lines()).sum::<usize>(), 20_000);
    // Modules repeat whole declarations; padding is only the short tail.
    for m in fx.all() {
        let padding = m.source.lines().rev().take_while(|line| line.starts_with('#')).count();
        assert!(padding < 8, "{}: {padding} padding lines", m.name);
        assert!(!m.fragments.is_empty());
    }
}

#[test]
fn every_tool_declaration_is_used_and_w_imports_only_earlier_modules() {
    let pool = fragments().unwrap();
    assert_eq!(pool.iter().filter(|f| f.tool == TOOLS[0].0).count(), 1);
    assert_eq!(pool.iter().filter(|f| f.tool == TOOLS[1].0).count(), 2);
    assert_eq!(pool.iter().filter(|f| f.tool == TOOLS[2].0).count(), 6);
    let fx = fixtures();
    let large = fx.get("L").unwrap();
    for f in &pool {
        assert!(large.fragments.contains(&(f.tool, f.name.clone())), "{} is not in L", f.name);
    }
    for (k, m) in fx.workspace.iter().enumerate() {
        assert!(m.imports.iter().all(|&i| i < k), "{}: {:?}", m.name, m.imports);
        assert_eq!(m.imports.is_empty(), k == 0, "{}", m.name);
        assert!(!(1..10).contains(&k) || m.imports.contains(&0), "{} must import W00", m.name);
        for &i in &m.imports {
            assert!(m.source.contains(&format!("\"{}\"", fx.workspace[i].sha256())), "{} pins W{i:02}", m.name);
        }
    }
}

#[test]
fn the_front_end_agrees_with_the_module_set_and_the_script() {
    let fx = fixtures();
    let script = edits(&fx).unwrap();
    let order: Vec<(&str, &str)> = script.iter().map(|e| (e.target, e.kind)).collect();
    let wanted: Vec<(&str, &str)> =
        EDIT_TARGETS.iter().flat_map(|t| EDIT_KINDS.iter().map(move |k| (*t, *k))).collect();
    assert_eq!(order, wanted);
    assert_eq!(script, edits(&fx).unwrap());
    let validation = validate(&fx, &script).unwrap();
    let direct = fx.direct_dependents();
    assert!(direct.len() >= 9, "{direct:?}");
    assert_eq!(validation.direct_dependents, direct);
    assert!(direct.iter().all(|name| validation.repinned.contains(name)));
    assert!(validation.repinned.len() > direct.len(), "the re-pin reaches transitive dependents");
    assert!(validation.refusals.iter().all(|(_, function, _)| function == "exported"));
    // A wrong expectation is a disagreement, LX-4's immediate kill criterion.
    let mut wrong = script.clone();
    let syntax_break = wrong.iter_mut().find(|e| e.kind == "syntax-break").unwrap();
    syntax_break.expected = Expected::Clean;
    assert!(validate(&fx, &wrong).unwrap_err().contains("syntax-break"));
}

#[test]
fn an_edit_refuses_a_stale_baseline_a_wrong_preimage_and_a_wrong_result() {
    let fx = fixtures();
    let script = edits(&fx).unwrap();
    let edit = script.iter().find(|e| e.target == "S" && e.kind == "inside-body").unwrap();
    let source = &fx.get("S").unwrap().source;
    let result = apply(source, edit).unwrap();
    assert!(!result.contains(BODY) && result.contains("    return exact_add(1, "));
    assert!(apply(&result, edit).unwrap_err().contains("stale baseline"));
    let mut moved = edit.clone();
    moved.replacements[0].offset += 1;
    assert!(apply(source, &moved).unwrap_err().contains("preimage"));
    let mut other = edit.clone();
    other.replacements[0].insert.push(' ');
    assert!(apply(source, &other).unwrap_err().contains("result differs"));
    let completion = script.iter().find(|e| e.target == "M" && e.kind == "completion-trigger").unwrap();
    let completed = apply(&fx.get("M").unwrap().source, completion).unwrap();
    assert_eq!(completion.cursor, Some(completed.len()));
    assert!(completed.ends_with("return exact_"));
}

#[test]
fn the_records_name_every_module_and_edit_by_digest() {
    let fx = fixtures();
    let script = edits(&fx).unwrap();
    let validation = validate(&fx, &script).unwrap();
    let manifest = manifest_json(&fx, &validation).unwrap();
    assert!(manifest.starts_with("{\n  \"format\": \"magic600-c9-lx4-fixtures-v1\""));
    for m in fx.all() {
        assert!(manifest.contains(&format!("\"sha256\": \"{}\"", m.sha256())), "{}", m.name);
    }
    for (path, source) in TOOLS {
        assert!(manifest.contains(&format!(
            "\"path\": \"{path}\", \"sha256\": \"{}\"",
            c600_syntax::sha256_hex(source.as_bytes())
        )));
    }
    let records = edits_json(&fx, &script);
    for e in &script {
        assert!(records.contains(&format!("\"result_sha256\": \"{}\"", e.result_sha256)));
    }
    assert_eq!(records.matches("\"expected\": ").count(), 16);
}
