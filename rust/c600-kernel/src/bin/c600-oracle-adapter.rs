//! The oracle adapter of LP2's M4 (CT-KN2-01, the 600-cell-Full half).
//!
//! ```text
//! c600-oracle-adapter build
//! c600-oracle-adapter trace --assets <dir> <case> <trace> [<case> <trace> ...]
//! c600-oracle-adapter agree --assets <dir>
//! ```
//!
//! `trace` reads each `C600-ORACLE-CASE-draft` case (JSON or gzip JSON) and writes a `C600-ORACLE-TRACE-draft` trace
//! for `tools/oracle/compare.py`, replaying every step through the kernel API: [`Kernel::recipe`] validates the whole
//! recipe, [`Kernel::derive`] computes the effect and the Net protection verdict, and [`Kernel::progress`] gives the
//! progress rows. A recipe the kernel refuses is an `invalid-input` step with null effects and an unchanged state; a
//! protection refusal is a `protected` step with its effects and an unchanged state. The trace's `model` block is
//! the loaded identity and its build string is [`c600_kernel::build`]. A case whose model identity or initial state
//! differs from the loaded model, a malformed case and a failed model check exit with 2 and write no trace for it.
//!
//! `agree` is the KN2.9 agreement test: for every orbit, its first, middle and last frame node and both signs (the
//! 210 stars of O03), the star cache's net map and every field derived from it must equal those of per-primitive
//! replay, and the word `[1, -1]` must have an empty net and the expansion digest of `struct.pack("<ii", 1, -1)`.
//! It exits with 1 on a disagreement. Both commands print a one-line JSON summary with timings to stdout.

use std::collections::HashMap;
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use c600_kernel::json::{self, Value};
use c600_kernel::{archive, Derivation, Effect, Error, IdentityBlock, Kernel, ProtectedSet, Snapshot, SupportRow};

const CASE_FORMAT: &str = "C600-ORACLE-CASE-draft";
const TRACE_FORMAT: &str = "C600-ORACLE-TRACE-draft";
const CANDIDATE: &str = "c600-kernel-full-candidate";
const USAGE: &str =
    "usage: c600-oracle-adapter build | trace --assets <dir> <case> <trace> [<case> <trace> ...] | agree --assets <dir>";
/// 0.4's word limit; the initial scramble enters the kernel in words of at most this many letters.
const WORD_CHUNK: usize = 100_000;
/// The largest decompressed case the adapter reads.
const CASE_LIMIT: usize = 1 << 30;
/// The expansion digest of the word `[1, -1]`, the SHA-256 of `struct.pack("<ii", 1, -1)`.
const ONE_MINUS_ONE: &str = "b15348c8f462384c01e83b6d499c6faf3f96808f5aa07c6bab4b65b36b4445d4";

struct Failure {
    code: u8,
    message: String,
}

fn malformed(message: impl Into<String>) -> Failure {
    Failure { code: 2, message: message.into() }
}

fn model_failure(error: Error) -> Failure {
    malformed(format!("model check failed: {error}"))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("c600-oracle-adapter: {}", failure.message);
            ExitCode::from(failure.code)
        }
    }
}

fn run(args: &[String]) -> Result<(), Failure> {
    let assets = |rest: &[String]| match rest {
        [flag, dir, tail @ ..] if flag == "--assets" => Ok((dir.clone(), tail.to_vec())),
        _ => Err(malformed(USAGE)),
    };
    match args.split_first() {
        Some((command, [])) if command == "build" => {
            println!("{}", c600_kernel::build());
            Ok(())
        }
        Some((command, rest)) if command == "trace" => {
            let (dir, files) = assets(rest)?;
            if files.is_empty() || files.len() % 2 != 0 {
                return Err(malformed(USAGE));
            }
            trace(Path::new(&dir), &files)
        }
        Some((command, rest)) if command == "agree" => match assets(rest)? {
            (dir, files) if files.is_empty() => agree(Path::new(&dir)),
            _ => Err(malformed(USAGE)),
        },
        _ => Err(malformed(USAGE)),
    }
}

fn seconds(start: Instant) -> Value {
    Value::Number(format!("{:.3}", start.elapsed().as_secs_f64()))
}

fn load(assets: &Path) -> Result<Kernel, Failure> {
    Kernel::load(assets).map_err(|error| malformed(format!("model check failed: {error}")))
}

fn read_document(path: &Path) -> Result<Value, Failure> {
    let bytes = std::fs::read(path).map_err(|_| malformed("cannot read a case file"))?;
    let bytes = if bytes.starts_with(&[0x1f, 0x8b]) {
        archive::gunzip(&bytes, CASE_LIMIT).map_err(|error| malformed(format!("unreadable gzip case: {error}")))?
    } else {
        bytes
    };
    json::parse(&bytes).map_err(|error| malformed(format!("malformed case JSON: {error}")))
}

fn trace(assets: &Path, files: &[String]) -> Result<(), Failure> {
    let start = Instant::now();
    let kernel = load(assets)?;
    let load_seconds = seconds(start);
    let build = c600_kernel::build();
    let mut cases = Vec::new();
    for pair in files.chunks(2) {
        let started = Instant::now();
        let case = read_document(Path::new(&pair[0]))?;
        let (case_id, steps, trace) = replay(&kernel, &case, &build)?;
        let mut text = trace.canonical();
        text.push('\n');
        std::fs::write(&pair[1], text).map_err(|_| malformed("cannot write a trace file"))?;
        cases.push(json::object([
            ("case_id", Value::String(case_id)),
            ("steps", Value::Int(steps as i64)),
            ("seconds", seconds(started)),
        ]));
    }
    let summary = json::object([
        ("command", Value::String("trace".into())),
        ("candidate", Value::String(CANDIDATE.into())),
        ("build", Value::String(build)),
        ("load_seconds", load_seconds),
        ("cases", Value::Array(cases)),
        ("total_seconds", seconds(start)),
    ]);
    println!("{}", summary.canonical());
    Ok(())
}

fn text(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).filter(|text| !text.is_empty()).map(str::to_owned)
}

fn integers(value: Option<&Value>) -> Option<Vec<i64>> {
    value?.as_array()?.iter().map(Value::as_i64).collect()
}

fn progress(kernel: &Kernel, memo: &mut HashMap<String, String>, state: &Snapshot) -> Result<String, Failure> {
    if let Some(sha256) = memo.get(state.digest()) {
        return Ok(sha256.clone());
    }
    let sha256 = kernel.progress(state).map_err(|refusal| malformed(refusal.to_string()))?.sha256;
    memo.insert(state.digest().to_owned(), sha256.clone());
    Ok(sha256)
}

fn rows(rows: &[SupportRow]) -> Value {
    Value::Array(
        rows.iter()
            .map(|row| {
                json::object([
                    ("orbit", Value::Int(row.orbit as i64)),
                    ("stickers", Value::Int(row.stickers as i64)),
                    ("pieces", Value::Int(row.pieces as i64)),
                ])
            })
            .collect(),
    )
}

/// One trace step. `effect` is `None` for an invalid input, whose effect fields are null.
fn step(
    index: i64,
    error: Option<&str>,
    state: &Snapshot,
    effect: Option<(&Effect, &[SupportRow])>,
    progress: String,
) -> Value {
    let effect_fields = match effect {
        Some((effect, conflicts)) => [
            Value::String(effect.net_sha256.clone()),
            Value::Int(effect.moved_stickers() as i64),
            Value::Int(effect.moved_pieces as i64),
            Value::String(effect.expansion_sha256.clone()),
            Value::Int(effect.primitive_count as i64),
            rows(&effect.support),
            rows(conflicts),
        ],
        None => std::array::from_fn(|_| Value::Null),
    };
    let [net, stickers, pieces, expansion, count, support, conflicts] = effect_fields;
    json::object([
        ("index", Value::Int(index)),
        ("accepted", Value::Bool(error.is_none())),
        ("error", error.map_or(Value::Null, |error| Value::String(error.to_owned()))),
        ("state_hash", Value::String(state.digest().to_owned())),
        ("net_sha256", net),
        ("net_moved_stickers", stickers),
        ("net_moved_pieces", pieces),
        ("expansion_sha256", expansion),
        ("primitive_count", count),
        ("support", support),
        ("conflicts", conflicts),
        ("progress_sha256", Value::String(progress)),
    ])
}

fn replay(kernel: &Kernel, case: &Value, build: &str) -> Result<(String, usize, Value), Failure> {
    if case.get("format").and_then(Value::as_str) != Some(CASE_FORMAT) {
        return Err(malformed("wrong case format"));
    }
    let case_id = text(case.get("case_id")).ok_or_else(|| malformed("missing case_id"))?;
    let model = case.get("model");
    let model_id = text(model.and_then(|m| m.get("model_id"))).ok_or_else(|| malformed("missing model.model_id"))?;
    let manifest =
        text(model.and_then(|m| m.get("manifest_sha256"))).ok_or_else(|| malformed("missing model.manifest_sha256"))?;
    // The identity check goes through the kernel (KN1.4); the manifest digest is the second half of the identity.
    kernel.check_identity(&IdentityBlock::full(&model_id)).map_err(|refusal| {
        malformed(format!("case model identity differs from the loaded model ({})", refusal.code))
    })?;
    if manifest != kernel.manifest_sha256() {
        return Err(malformed("case manifest_sha256 differs from the loaded model"));
    }
    let independent = match case.get("mode").and_then(Value::as_str) {
        Some("independent") => true,
        Some("chain") => false,
        _ => return Err(malformed("malformed case mode")),
    };
    let initial = case.get("initial");
    let moves = integers(initial.and_then(|i| i.get("moves"))).ok_or_else(|| malformed("malformed initial moves"))?;
    let expected =
        text(initial.and_then(|i| i.get("state_hash"))).ok_or_else(|| malformed("missing initial state_hash"))?;
    let protected = integers(case.get("protected")).ok_or_else(|| malformed("malformed protected orbits"))?;
    let protected = kernel.protected_set(&protected).map_err(|_| malformed("malformed protected orbits"))?;
    let steps = case.get("steps").and_then(Value::as_array).filter(|steps| !steps.is_empty());
    let steps = steps.ok_or_else(|| malformed("missing steps"))?;
    let mut ordered = Vec::with_capacity(steps.len());
    for item in steps {
        let index = item.get("index").and_then(Value::as_i64).filter(|&index| index >= 0);
        let index = index.ok_or_else(|| malformed("malformed step index"))?;
        let recipe = item.get("recipe").ok_or_else(|| malformed("missing step recipe"))?;
        ordered.push((index, recipe));
    }
    ordered.sort_by_key(|&(index, _)| index);
    if ordered.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(malformed("duplicate step index"));
    }

    let none = ProtectedSet::none();
    let mut start = kernel.solved();
    for chunk in moves.chunks(WORD_CHUNK) {
        let recipe = kernel.word(chunk).map_err(|_| malformed("malformed initial moves"))?;
        start = match kernel.derive(&start, &recipe, &none).map_err(model_failure)? {
            Derivation::Derived(derived) => derived.state,
            Derivation::Refused(_) => return Err(malformed("the initial word was refused")),
        };
    }
    if start.digest() != expected {
        return Err(malformed("initial state_hash differs from the loaded model's replay"));
    }

    let mut memo = HashMap::new();
    let mut current = start.clone();
    let mut out = Vec::with_capacity(ordered.len());
    for (index, value) in ordered {
        let base = if independent { start.clone() } else { current.clone() };
        let row = match kernel.recipe(value) {
            Err(_) => step(index, Some("invalid-input"), &base, None, progress(kernel, &mut memo, &base)?),
            Ok(recipe) => match kernel.derive(&base, &recipe, &protected).map_err(model_failure)? {
                Derivation::Derived(derived) => {
                    let sha256 = progress(kernel, &mut memo, &derived.state)?;
                    let row = step(
                        index,
                        None,
                        &derived.state,
                        Some((&derived.effect, &derived.protection.conflicts)),
                        sha256,
                    );
                    current = derived.state;
                    row
                }
                Derivation::Refused(refused) => {
                    let sha256 = progress(kernel, &mut memo, &base)?;
                    step(
                        index,
                        Some("protected"),
                        &base,
                        Some((&refused.effect, &refused.protection.conflicts)),
                        sha256,
                    )
                }
            },
        };
        out.push(row);
    }
    let count = out.len();
    let trace = json::object([
        ("format", Value::String(TRACE_FORMAT.into())),
        ("case_id", Value::String(case_id.clone())),
        (
            "model",
            json::object([
                ("model_id", Value::String(kernel.identity().model_id)),
                ("manifest_sha256", Value::String(kernel.manifest_sha256().to_owned())),
            ]),
        ),
        (
            "candidate",
            json::object([("name", Value::String(CANDIDATE.into())), ("build", Value::String(build.to_owned()))]),
        ),
        ("steps", Value::Array(out)),
    ]);
    Ok((case_id, count, trace))
}

fn disagree(message: String) -> Failure {
    Failure { code: 1, message }
}

fn agree(assets: &Path) -> Result<(), Failure> {
    let start = Instant::now();
    let kernel = load(assets)?;
    let load_seconds = seconds(start);
    let started = Instant::now();
    let recipe = kernel.word(&[1, -1]).map_err(|refusal| malformed(refusal.to_string()))?;
    let cached = kernel.effect(&recipe).map_err(model_failure)?;
    let replayed = kernel.effect_by_replay(&recipe).map_err(model_failure)?;
    if cached != replayed || cached.moved_stickers() != 0 || cached.expansion_sha256 != ONE_MINUS_ONE {
        return Err(disagree("the word [1, -1] does not have an empty net and the expected expansion digest".into()));
    }
    let (mut stars, mut primitives) = (0usize, 0usize);
    for orbit in 0..c600_kernel::ORBITS {
        let nodes = kernel.star_nodes(orbit).ok_or_else(|| malformed("missing execution tree"))?;
        for node in [0, nodes / 2, nodes - 1] {
            for sign in [1, -1] {
                let text = format!(r#"[{{"kind":"star","orbit":{orbit},"node":{node},"sign":{sign}}}]"#);
                let value = json::parse(text.as_bytes()).map_err(|error| malformed(error.to_string()))?;
                let recipe = kernel.recipe(&value).map_err(|refusal| malformed(refusal.to_string()))?;
                let cached = kernel.effect(&recipe).map_err(model_failure)?;
                let replayed = kernel.effect_by_replay(&recipe).map_err(model_failure)?;
                if cached != replayed {
                    return Err(disagree(format!(
                        "the star cache disagrees with per-primitive replay at orbit {orbit}, node {node}, sign {sign}"
                    )));
                }
                stars += 1;
                primitives += replayed.primitive_count;
            }
        }
    }
    let summary = json::object([
        ("command", Value::String("agree".into())),
        ("check", Value::String("KN2.9: star-cache net maps against per-primitive replay".into())),
        (
            "compared",
            Value::String(
                "source, destination, net_sha256, support, moved pieces, expansion_sha256, primitive_count".into(),
            ),
        ),
        ("stars", Value::Int(stars as i64)),
        ("nodes", Value::String("first, middle and last frame node of every orbit, signs 1 and -1".into())),
        ("primitives", Value::Int(primitives as i64)),
        ("word_1_minus_1", Value::String(format!("empty net, expansion_sha256 {ONE_MINUS_ONE}"))),
        ("build", Value::String(c600_kernel::build())),
        ("load_seconds", load_seconds),
        ("seconds", seconds(started)),
    ]);
    println!("{}", summary.canonical());
    Ok(())
}
