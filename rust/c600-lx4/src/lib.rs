//! LX-4's fixtures for C9 (chapter 11, RTE.48 and RTE.51 steps 1 and 2; CT-EV1-06, CT-RTE-12): a module set generated
//! deterministically from the three LP3 tools' sources (PHS.22) and listed by digest, and a seeded edit script of
//! RTE.51's four edit kinds. Every module and every edit goes through the product front end (`c600_syntax::parse`
//! and `lower`, then `c600_check::check`) before anything is written. A disagreement is LX-4's immediate kill
//! criterion: no timer may run on fixtures that fail it.
//!
//! This is measurement tooling. It times nothing and judges no bar; the timed runs, the frame matching and the report
//! belong to the LX-4 harness on the owner's machine (RTE.51 steps 3 to 9).
//!
//! Sizes (RTE.48): S, M and L are single modules of exactly 200, 1,000 and 5,000 LF-terminated lines; W is a workspace
//! of 50 modules of 400 lines each, 20,000 lines in all. W's import graph is acyclic and synthetic, not a real LP3
//! graph: module k imports one to three earlier modules by digest, and W01 to W09 all import W00. Every module exports
//! `exported() -> Nat`, and a W module's export calls its imports' exports, so the exported-signature edit at W00
//! reaches W00's direct dependents as type refusals once every transitive dependent is re-pinned to the new digests.
//!
//! A module is made of whole declarations of the three tools, renamed, in a seeded order, then `#` comment padding to
//! its exact length. The manifest lists the tools' sources by digest: when a tool changes, the module set changes, and
//! RTE.51 step 1 freezes it again with the candidate. Until then the set is a DRAFT.
//!
//! U8 of LP3's C9 sitting ran LX-4 on the owner's machine with the product build of `013900ff`
//! (`c9-product-2efda50d54592f20`, 21 November 2026) and the module set this crate generated from that candidate's
//! tools, frozen by digest with it. The record, analysed again under the owner decision of 22 November 2026 (items 1,
//! 2 and 10; pull request 411), is `work/experiments/c9-sitting/results/u8-final-lx4-20261122.json`:
//!
//! - CT-RTE-12 (performance, whole: `result.fixtures` lists the set by digest, `manifest_sha256` over 53 modules, S,
//!   M, L and W's 50, and `edits_sha256` over 16 seeded edits; `result.runs` holds a valid run per puzzle;
//!   `result.series` holds the smoke run and series 1 to 3 per puzzle, size and metric, each with its median (`p50`),
//!   p95, p99 and largest value; `result.accounting` counts 10,800 S, M and L samples apart from 600 W samples, 600
//!   turns and 40 smoke samples; `result.decision` is `all-bars` under RTE.53's S and M versus L and W rule;
//!   `bar.rule` is step 5's timing; the gate reference is U4's W3 summary, `result.gate_reference`, owner item 2).
//! - CT-EV1-06 (performance, whole: in `result.series` syntax feedback, completion and diagnostics meet their bars in
//!   all three series at S, M and L on both puzzles, on the generated modules; `verdict` met).
//! - CT-LG4-09 (performance, part: LP3's half: the editor's views were open and re-run, the `rerun` metric, with 600
//!   E2 turns in the same runs, `verdict` met, and charter E2 met in them, `u8-final-e2.json`; not run here: LX-2's
//!   half, in `c600-lx2`).

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt::Write as _;

use c600_check::check;
use c600_syntax::{lower, parse, sha256_hex};

pub const FORMAT: &str = "magic600-c9-lx4-fixtures-v1";
pub const EDIT_FORMAT: &str = "magic600-c9-lx4-edits-v1";
pub const MODULE_SEED: u64 = 2026111801;
pub const EDIT_SEED: u64 = 2026111802;
pub const EDITION: &str = "6cs-draft-1";

/// The three LP3 tools (PHS.22): their repository paths and sources.
pub const TOOLS: [(&str, &str); 3] = [
    ("rust/c600-tools/src/full_600_cell.6cs", include_str!("../../c600-tools/src/full_600_cell.6cs")),
    ("rust/c600-jumble-tool/src/jumble_checker.6cs", include_str!("../../c600-jumble-tool/src/jumble_checker.6cs")),
    ("rust/c600-repr/src/representations.6cs", include_str!("../../c600-repr/src/representations.6cs")),
];

/// RTE.48's single-module sizes in lines.
pub const SIZES: [(&str, usize); 3] = [("S", 200), ("M", 1_000), ("L", 5_000)];
pub const W_MODULES: usize = 50;
pub const W_LINES: usize = 400;
/// W01 to W09 all import W00.
const W_BASE: usize = 10;

/// The modules the edit script edits, and RTE.51 step 2's four kinds, in the script's order.
pub const EDIT_TARGETS: [&str; 4] = ["S", "M", "L", "W00"];
pub const EDIT_KINDS: [&str; 4] = ["inside-body", "exported-signature", "completion-trigger", "syntax-break"];

/// The exported signature and body every edited module has once (S, M, L and W00 import nothing).
pub const SIGNATURE: &str = "fn exported() -> Nat";
pub const BODY: &str = "    return 1;";
const PADDING: &str = "# lx4 padding";
const COMPLETION: &str = "fn completion_point() -> Nat effects [] {\n    return exact_";

/// SplitMix64, written out so that the fixtures never depend on a library's choice of generator.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// One whole top-level declaration of a tool, from its first line to its closing `}` in column 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fragment {
    pub tool: &'static str,
    pub name: String,
    pub lines: Vec<String>,
}

/// Every declaration of the three tools, in source order.
pub fn fragments() -> Result<Vec<Fragment>, String> {
    let mut out = Vec::new();
    for (path, source) in TOOLS {
        let mut current: Option<Fragment> = None;
        for line in source.lines() {
            if let Some(fragment) = current.as_mut() {
                fragment.lines.push(line.to_string());
                if line == "}" {
                    out.extend(current.take());
                }
            } else if let Some(name) = declared(line) {
                current = Some(Fragment { tool: path, name, lines: vec![line.to_string()] });
            }
        }
        if current.is_some() {
            return Err(format!("{path}: a declaration has no closing brace in column 1"));
        }
    }
    Ok(out)
}

/// The name a column-1 declaration line declares.
fn declared(line: &str) -> Option<String> {
    let rest = line.strip_prefix("export ").unwrap_or(line);
    let rest = ["fn ", "view ", "command "].iter().find_map(|kind| rest.strip_prefix(kind))?;
    let end = rest.find(['(', '<'])?;
    Some(rest[..end].to_string())
}

/// One generated module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    /// S, M, L, or W00 to W49.
    pub name: String,
    /// The module name its header declares.
    pub module: String,
    pub source: String,
    /// W only: the W modules it imports, ascending; the i-th is imported as `d{i}`.
    pub imports: Vec<usize>,
    /// Each declaration it holds: the tool and the declaration's name there.
    pub fragments: Vec<(&'static str, String)>,
}

impl Module {
    /// Its path in a fixture folder.
    pub fn path(&self) -> String {
        if self.name.starts_with('W') {
            format!("W/{}.6cs", self.name)
        } else {
            format!("{}.6cs", self.name)
        }
    }

    pub fn lines(&self) -> usize {
        self.source.matches('\n').count()
    }

    pub fn sha256(&self) -> String {
        sha256_hex(self.source.as_bytes())
    }
}

/// The module set: S, M and L, then W00 to W49 in import order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fixtures {
    pub singles: Vec<Module>,
    pub workspace: Vec<Module>,
}

impl Fixtures {
    pub fn get(&self, name: &str) -> Option<&Module> {
        self.singles.iter().chain(&self.workspace).find(|m| m.name == name)
    }

    pub fn all(&self) -> impl Iterator<Item = &Module> {
        self.singles.iter().chain(&self.workspace)
    }

    /// W00's direct dependents: the modules that import it.
    pub fn direct_dependents(&self) -> Vec<String> {
        self.workspace.iter().filter(|m| m.imports.contains(&0)).map(|m| m.name.clone()).collect()
    }
}

/// Generates the module set from the tools' sources and the module seed. The same sources give the same bytes.
pub fn generate() -> Result<Fixtures, String> {
    let pool = fragments()?;
    if pool.is_empty() {
        return Err("the tools declare nothing".into());
    }
    let mut rng = Rng(MODULE_SEED);
    let mut singles = Vec::new();
    for (name, lines) in SIZES {
        singles.push(module(name, &format!("lx4_{}", name.to_lowercase()), lines, &[], &pool, &mut rng)?);
    }
    let mut workspace: Vec<Module> = Vec::new();
    for k in 0..W_MODULES {
        let imports = imports_of(k, &mut rng);
        let pins: Vec<String> = imports.iter().map(|&i| workspace[i].sha256()).collect();
        let mut m = module(&format!("W{k:02}"), &format!("lx4_w{k:02}"), W_LINES, &pins, &pool, &mut rng)?;
        m.imports = imports;
        workspace.push(m);
    }
    Ok(Fixtures { singles, workspace })
}

fn imports_of(k: usize, rng: &mut Rng) -> Vec<usize> {
    if k == 0 {
        return Vec::new();
    }
    let wanted = (1 + rng.below(3)).min(k);
    let mut chosen = BTreeSet::new();
    if k < W_BASE {
        chosen.insert(0);
    }
    while chosen.len() < wanted {
        chosen.insert(rng.below(k));
    }
    chosen.into_iter().collect()
}

fn module(
    name: &str,
    module: &str,
    lines: usize,
    pins: &[String],
    pool: &[Fragment],
    rng: &mut Rng,
) -> Result<Module, String> {
    let mut text = format!("edition \"{EDITION}\";\nmodule {module} requires [];\n");
    for (i, pin) in pins.iter().enumerate() {
        let _ = writeln!(text, "import d{i} \"{pin}\";");
    }
    let calls: Vec<String> = (0..pins.len()).map(|i| format!("d{i}.exported()")).collect();
    let value = match calls.len() {
        0 => "1".to_string(),
        1 => format!("exact_add({}, 1)", calls[0]),
        n => calls[..n - 1].iter().rev().fold(calls[n - 1].clone(), |acc, call| format!("exact_add({call}, {acc})")),
    };
    let _ = write!(text, "\nexport {SIGNATURE} effects [] {{\n    return {value};\n}}\n");
    let mut used = text.matches('\n').count();
    if used > lines {
        return Err(format!("{name}: the header alone is longer than {lines} lines"));
    }
    let smallest = pool.iter().min_by_key(|f| f.lines.len()).expect("a non-empty pool");
    let mut fragments = Vec::new();
    while used + smallest.lines.len() < lines {
        let mut fragment = &pool[rng.below(pool.len())];
        if used + fragment.lines.len() + 1 > lines {
            fragment = smallest;
        }
        let renamed = format!("{}_{:04}", fragment.name, fragments.len());
        text.push('\n');
        for (j, line) in fragment.lines.iter().enumerate() {
            if j == 0 {
                text.push_str(&line.replacen(&format!(" {}", fragment.name), &format!(" {renamed}"), 1));
            } else {
                text.push_str(line);
            }
            text.push('\n');
        }
        used += fragment.lines.len() + 1;
        fragments.push((fragment.tool, fragment.name.clone()));
    }
    while used < lines {
        text.push_str(PADDING);
        text.push('\n');
        used += 1;
    }
    Ok(Module { name: name.into(), module: module.into(), source: text, imports: Vec::new(), fragments })
}

/// One replacement of an edit: at a byte offset of the baseline, `delete` gives way to `insert`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Replacement {
    pub offset: usize,
    pub delete: String,
    pub insert: String,
}

/// What the front end must say after an edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expected {
    /// Parses, lowers and checks with no diagnostic; at W00, with W re-pinned.
    Clean,
    /// At W00 with W re-pinned: the checker refuses exactly in W00's direct dependents.
    RefusedInDirectDependents,
    /// Syntax diagnostics or recovery holes; nothing lowers (LG1.5).
    Syntax,
}

impl Expected {
    pub fn name(self) -> &'static str {
        match self {
            Expected::Clean => "clean",
            Expected::RefusedInDirectDependents => "refused-in-direct-dependents",
            Expected::Syntax => "syntax",
        }
    }
}

/// One edit of the script. Each edit starts from the original bytes of its target (the editor resets between edits).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub target: &'static str,
    pub kind: &'static str,
    pub base_sha256: String,
    pub replacements: Vec<Replacement>,
    pub result_sha256: String,
    /// The completion trigger's byte offset in the result, where the completion list is asked for.
    pub cursor: Option<usize>,
    pub expected: Expected,
}

/// The seeded edit script: the four kinds at S, M, L and W00, 16 edits.
pub fn edits(fixtures: &Fixtures) -> Result<Vec<Edit>, String> {
    let mut rng = Rng(EDIT_SEED);
    let mut out = Vec::new();
    for target in EDIT_TARGETS {
        let data = &fixtures.get(target).ok_or_else(|| format!("no module {target}"))?.source;
        let body = once(data, BODY).ok_or_else(|| format!("{target}: `{BODY}` is not there exactly once"))?;
        let signature =
            once(data, SIGNATURE).ok_or_else(|| format!("{target}: `{SIGNATURE}` is not there exactly once"))?;
        let number = 2 + rng.below(9_998);
        let at = |offset: usize, delete: &str, insert: &str| Replacement {
            offset,
            delete: delete.into(),
            insert: insert.into(),
        };
        let signed = if target == "W00" { Expected::RefusedInDirectDependents } else { Expected::Clean };
        let changes = [
            ("inside-body", vec![at(body, BODY, &format!("    return exact_add(1, {number});"))], Expected::Clean),
            (
                "exported-signature",
                vec![at(signature, SIGNATURE, "fn exported() -> Text"), at(body, BODY, "    return \"edited\";")],
                signed,
            ),
            ("completion-trigger", vec![at(data.len(), "", COMPLETION)], Expected::Syntax),
            ("syntax-break", vec![at(body + BODY.len() - 1, ";", "")], Expected::Syntax),
        ];
        for (kind, replacements, expected) in changes {
            let mut edit = Edit {
                target,
                kind,
                base_sha256: sha256_hex(data.as_bytes()),
                replacements,
                result_sha256: String::new(),
                cursor: None,
                expected,
            };
            let result = apply(data, &edit)?;
            edit.result_sha256 = sha256_hex(result.as_bytes());
            if kind == "completion-trigger" {
                edit.cursor = Some(result.len());
            }
            out.push(edit);
        }
    }
    Ok(out)
}

fn once(data: &str, needle: &str) -> Option<usize> {
    let at = data.find(needle)?;
    (data.matches(needle).count() == 1).then_some(at)
}

/// Applies an edit to its baseline. It refuses a stale baseline, a differing preimage and a differing result.
pub fn apply(data: &str, edit: &Edit) -> Result<String, String> {
    if sha256_hex(data.as_bytes()) != edit.base_sha256 {
        return Err(format!("{} {}: the edit refuses a stale baseline", edit.target, edit.kind));
    }
    let mut out = data.to_string();
    let mut order: Vec<&Replacement> = edit.replacements.iter().collect();
    order.sort_by_key(|r| std::cmp::Reverse(r.offset));
    for r in order {
        if out.get(r.offset..r.offset + r.delete.len()) != Some(r.delete.as_str()) {
            return Err(format!("{} {}: the preimage at byte {} differs", edit.target, edit.kind, r.offset));
        }
        out.replace_range(r.offset..r.offset + r.delete.len(), &r.insert);
    }
    if !edit.result_sha256.is_empty() && sha256_hex(out.as_bytes()) != edit.result_sha256 {
        return Err(format!("{} {}: the edit's result differs", edit.target, edit.kind));
    }
    Ok(out)
}

/// W's sources after W[changed] becomes `source`: every later module that imports a changed module is re-pinned to
/// its new digest, in import order, so the change reaches W00's transitive dependents.
pub fn repin(workspace: &[Module], changed: usize, source: String) -> Vec<String> {
    let old: Vec<String> = workspace.iter().map(Module::sha256).collect();
    let mut sources: Vec<String> = workspace.iter().map(|m| m.source.clone()).collect();
    sources[changed] = source;
    for k in changed + 1..workspace.len() {
        for &i in &workspace[k].imports {
            let new = sha256_hex(sources[i].as_bytes());
            if new != old[i] {
                sources[k] = sources[k].replacen(&format!("\"{}\"", old[i]), &format!("\"{new}\""), 1);
            }
        }
    }
    sources
}

/// What the front end says of a set of sources lowered together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Clean,
    Syntax,
    Lowering(String),
    /// The checker's diagnostics: module, function and code.
    Refused(Vec<(String, String, &'static str)>),
    Internal(String),
}

pub fn front_end(sources: &[&str]) -> Outcome {
    let files: Vec<_> = sources.iter().map(|s| parse(s.as_bytes())).collect();
    if files.iter().any(|f| !f.diagnostics.is_empty() || !f.holes.is_empty()) {
        return Outcome::Syntax;
    }
    let lowered = match lower(&files) {
        Ok(lowered) => lowered,
        Err(error) => return Outcome::Lowering(error.to_string()),
    };
    match check(&lowered.program) {
        Ok(report) if report.diagnostics.is_empty() => Outcome::Clean,
        Ok(report) => Outcome::Refused(
            report.diagnostics.iter().map(|d| (d.module.clone(), d.function.clone(), d.code)).collect(),
        ),
        Err(error) => Outcome::Internal(format!("{error:?}")),
    }
}

/// What the front end said of the fixtures and the script, for the manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Validation {
    /// W00's direct dependents, where the exported-signature edit at W00 is refused.
    pub direct_dependents: Vec<String>,
    /// The W modules that edit re-pins: W00's transitive dependents.
    pub repinned: Vec<String>,
    /// The checker's refusals after that edit.
    pub refusals: Vec<(String, String, &'static str)>,
}

/// Checks the module set and the script through the front end. Any disagreement is refused with its reason.
pub fn validate(fixtures: &Fixtures, script: &[Edit]) -> Result<Validation, String> {
    for (m, (name, lines)) in fixtures.singles.iter().zip(SIZES) {
        if m.name != name || m.lines() != lines || !m.source.ends_with('\n') {
            return Err(format!("{}: not {lines} LF-terminated lines", m.name));
        }
        if front_end(&[&m.source]) != Outcome::Clean {
            return Err(format!("{}: {:?}", m.name, front_end(&[&m.source])));
        }
    }
    if fixtures.workspace.len() != W_MODULES || fixtures.workspace.iter().any(|m| m.lines() != W_LINES) {
        return Err(format!("W: not {W_MODULES} modules of {W_LINES} lines"));
    }
    let workspace: Vec<&str> = fixtures.workspace.iter().map(|m| m.source.as_str()).collect();
    match front_end(&workspace) {
        Outcome::Clean => {}
        other => return Err(format!("W: {other:?}")),
    }
    let direct = fixtures.direct_dependents();
    let mut validation = Validation { direct_dependents: direct.clone(), repinned: Vec::new(), refusals: Vec::new() };
    if script.len() != EDIT_TARGETS.len() * EDIT_KINDS.len() {
        return Err(format!("the script has {} edits", script.len()));
    }
    for edit in script {
        let target = fixtures.get(edit.target).ok_or_else(|| format!("no module {}", edit.target))?;
        let result = apply(&target.source, edit)?;
        let outcome = if edit.target == "W00" && edit.expected != Expected::Syntax {
            let sources = repin(&fixtures.workspace, 0, result);
            let outcome = front_end(&sources.iter().map(String::as_str).collect::<Vec<_>>());
            if edit.kind == "exported-signature" {
                validation.repinned = fixtures
                    .workspace
                    .iter()
                    .zip(&sources)
                    .skip(1)
                    .filter(|(m, s)| m.source != **s)
                    .map(|(m, _)| m.name.clone())
                    .collect();
            }
            outcome
        } else {
            front_end(&[&result])
        };
        let agrees = match (edit.expected, &outcome) {
            (Expected::Clean, Outcome::Clean) | (Expected::Syntax, Outcome::Syntax) => true,
            (Expected::RefusedInDirectDependents, Outcome::Refused(refusals)) => {
                let modules: BTreeSet<&str> = refusals.iter().map(|(m, _, _)| m.as_str()).collect();
                let wanted: BTreeSet<String> =
                    direct.iter().map(|name| format!("lx4_{}", name.to_lowercase())).collect();
                validation.refusals = refusals.clone();
                modules == wanted.iter().map(String::as_str).collect()
            }
            _ => false,
        };
        if !agrees {
            return Err(format!(
                "{} {}: expected {}, the front end said {outcome:?}",
                edit.target,
                edit.kind,
                edit.expected.name()
            ));
        }
    }
    Ok(validation)
}

/// A JSON string literal.
pub fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_list<T>(items: &[T], each: impl Fn(&T) -> String) -> String {
    format!("[{}]", items.iter().map(each).collect::<Vec<_>>().join(", "))
}

/// MANIFEST.json: the tools by digest, every module by digest with its size and imports, and the front end's verdict.
pub fn manifest_json(fixtures: &Fixtures, validation: &Validation) -> Result<String, String> {
    let pool = fragments()?;
    let tools = json_list(&TOOLS, |(path, source)| {
        let names: Vec<String> = pool.iter().filter(|f| f.tool == *path).map(|f| f.name.clone()).collect();
        format!(
            "{{\"path\": {}, \"sha256\": {}, \"declarations\": {}}}",
            json_str(path),
            json_str(&sha256_hex(source.as_bytes())),
            json_list(&names, |n| json_str(n))
        )
    });
    let modules: Vec<&Module> = fixtures.all().collect();
    let modules = json_list(&modules, |m| {
        let imports: Vec<String> = m.imports.iter().map(|&i| fixtures.workspace[i].name.clone()).collect();
        format!(
            "{{\"name\": {}, \"module\": {}, \"path\": {}, \"bytes\": {}, \"lines\": {}, \"sha256\": {}, \"imports\": {}, \"declarations\": {}}}",
            json_str(&m.name),
            json_str(&m.module),
            json_str(&m.path()),
            m.source.len(),
            m.lines(),
            json_str(&m.sha256()),
            json_list(&imports, |n| json_str(n)),
            m.fragments.len()
        )
    });
    let refusals = json_list(&validation.refusals, |(module, function, code)| {
        format!(
            "{{\"module\": {}, \"function\": {}, \"code\": {}}}",
            json_str(module),
            json_str(function),
            json_str(code)
        )
    });
    Ok(format!(
        "{{\n  \"format\": {},\n  \"status\": \"DRAFT: generated from the three LP3 tools as they stand; RTE.51 step 1 freezes the set with the candidate\",\n  \"edition\": {},\n  \"module_seed\": {MODULE_SEED},\n  \"sizes\": {{\"S\": 200, \"M\": 1000, \"L\": 5000, \"W\": {{\"modules\": {W_MODULES}, \"lines_each\": {W_LINES}}}}},\n  \"tools\": {tools},\n  \"modules\": {modules},\n  \"front_end\": {{\"modules\": \"clean\", \"script\": \"agrees\", \"w00_direct_dependents\": {}, \"w00_signature_repinned\": {}, \"w00_signature_refusals\": {refusals}}}\n}}\n",
        json_str(FORMAT),
        json_str(EDITION),
        json_list(&validation.direct_dependents, |n| json_str(n)),
        json_list(&validation.repinned, |n| json_str(n)),
    ))
}

/// EDITS.json: the script, each edit with its baseline and result digests and what the front end must say.
pub fn edits_json(fixtures: &Fixtures, script: &[Edit]) -> String {
    let edits = json_list(script, |e| {
        let path = fixtures.get(e.target).map(Module::path).unwrap_or_default();
        let replacements = json_list(&e.replacements, |r| {
            format!(
                "{{\"offset\": {}, \"delete\": {}, \"insert\": {}}}",
                r.offset,
                json_str(&r.delete),
                json_str(&r.insert)
            )
        });
        let cursor = e.cursor.map_or("null".to_string(), |c| c.to_string());
        format!(
            "{{\"target\": {}, \"path\": {}, \"kind\": {}, \"base_sha256\": {}, \"replacements\": {replacements}, \"result_sha256\": {}, \"cursor\": {cursor}, \"expected\": {}}}",
            json_str(e.target),
            json_str(&path),
            json_str(e.kind),
            json_str(&e.base_sha256),
            json_str(&e.result_sha256),
            json_str(e.expected.name())
        )
    });
    format!(
        "{{\n  \"format\": {},\n  \"seed\": {EDIT_SEED},\n  \"reset_between_edits\": true,\n  \"offsets\": \"UTF-8 bytes of the baseline\",\n  \"kinds\": {},\n  \"edits\": {edits}\n}}\n",
        json_str(EDIT_FORMAT),
        json_list(&EDIT_KINDS, |k| json_str(k))
    )
}
