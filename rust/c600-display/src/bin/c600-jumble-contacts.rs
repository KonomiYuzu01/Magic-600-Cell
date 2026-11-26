//! `c600-jumble-contacts <assets directory> <record.json>...`: the domain contact graph of each jumbled state record
//! (ADR-D02.21), for the Look Lab sample builder (`tools/looklab/fixtures/make_d02_samples.py`, step B15 of the ADR-D02
//! build). For each record it loads the state, checks the state's digest against the record's, and prints one JSON
//! line: `{"digest", "domains", "edges": [[a, b, on_grip, off_grip], ...], "planes", "seam_faces", "tests"}`, with
//! `a` and `b` indices into `domains` (the lattice domain first). Exit status 0 on success, 1 on a refused input and
//! 2 on a usage error.

use std::path::Path;
use std::process::ExitCode;

use c600_display::jumble::contacts;
use c600_jumble::state::State;
use c600_jumble::Context;
use c600_kernel::json::{self, Value};

fn one(ctx: &Context, path: &str) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let record = json::parse(&bytes).map_err(|e| e.to_string())?;
    let state = State::from_record(ctx, None, &record)?;
    let digest = state.digest(ctx);
    if let Some(expected) = record.get("digest").and_then(Value::as_str) {
        if expected != digest {
            return Err(format!("the record names digest {expected}; the state it holds has {digest}"));
        }
    }
    let graph = contacts::contact_graph(ctx, &state)?;
    let edges = graph
        .edges
        .iter()
        .map(|c| {
            Value::Array(vec![
                Value::Int(i64::from(c.a)),
                Value::Int(i64::from(c.b)),
                Value::Bool(c.on_grip),
                Value::Bool(c.off_grip),
            ])
        })
        .collect();
    let count = |n: u64| Value::Int(i64::try_from(n).unwrap_or(i64::MAX));
    Ok(json::object([
        ("digest", Value::String(digest)),
        ("domains", Value::Array(graph.keys.into_iter().map(Value::String).collect())),
        ("edges", Value::Array(edges)),
        ("planes", count(graph.planes)),
        ("seam_faces", count(graph.seam_faces)),
        ("tests", count(graph.tests)),
    ])
    .canonical())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: c600-jumble-contacts <assets directory> <record.json>...");
        return ExitCode::from(2);
    }
    let ctx = match Context::load(Path::new(&args[1])) {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("assets: {}", e.0);
            return ExitCode::from(1);
        }
    };
    for path in &args[2..] {
        match one(&ctx, path) {
            Ok(line) => println!("{line}"),
            Err(e) => {
                eprintln!("{path}: {e}");
                return ExitCode::from(1);
            }
        }
    }
    ExitCode::SUCCESS
}
