// Experiment-only host. File I/O and JSON transport are outside function timers.
mod json;
mod parser;
mod checker;
include!("numeric.rs");

use std::collections::BTreeMap;
use std::fs;
use std::hint::black_box;
use std::time::Instant;
use json::{obj, J};

fn front(data: &[u8], surface: char, interfaces: &BTreeMap<String, J>, cat: &J) -> J {
    let parsed = parser::parse(data, surface);
    let (ir, error) = match checker::lower(&parsed, interfaces, cat) {
        Ok(ir) => (ir, J::Null), Err(error) => (J::Null, error.into()),
    };
    obj(&[("parsed", parsed), ("ir", ir), ("error", error)])
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert!(args.len() == 4 && ["check", "measure"].contains(&args[1].as_str()), "check|measure request response");
    let request = json::parse(&fs::read_to_string(&args[2]).unwrap()).unwrap();
    let cat = json::parse(include_str!("catalogue.json")).unwrap();
    let mut interfaces = BTreeMap::new();
    for (digest, path) in request["interfaces"].obj() {
        let data = fs::read(path.s()).unwrap(); assert_eq!(sha256(&data), *digest, "interface bytes changed");
        interfaces.insert(digest.clone(), json::parse(std::str::from_utf8(&data).unwrap()).unwrap());
    }
    let mut outputs = vec![];
    for job in request["jobs"].a() {
        if args[1] == "check" {
            let data = fs::read(job["source"].s()).unwrap();
            let surface = job["surface"].s().chars().next().unwrap();
            outputs.push(front(&data, surface, &interfaces, &cat));
        } else {
            let sources = job["sources"].a().iter().map(|p| fs::read(p.s()).unwrap()).collect::<Vec<_>>();
            let mode = job["mode"].s();
            let parsed = if mode == "check" { sources.iter().map(|s| parser::parse(s, 'a')).collect() } else { vec![] };
            let run = || {
                let mut errors = 0usize;
                if mode == "check" {
                    for p in &parsed { if black_box(checker::lower(black_box(p), &interfaces, &cat)).is_err() { errors += 1; } }
                } else {
                    for source in &sources {
                        let p = parser::parse(black_box(source), 'a');
                        if mode == "parse" { errors += usize::from(!p["diagnostics"].a().is_empty()); black_box(p); }
                        else if black_box(checker::lower(&p, &interfaces, &cat)).is_err() { errors += 1; }
                    }
                }
                errors
            };
            let expected = job["expected_errors"].n();
            // One smoke sample, followed by complete series; never truncate a slow series.
            let start = Instant::now(); let errors = run(); let smoke = start.elapsed().as_nanos() as u64;
            let mut invalid = vec![];
            if errors != expected { invalid.push(obj(&[("series", 0usize.into()), ("sample", 0usize.into()), ("errors", errors.into())])); }
            let mut series = vec![];
            for s in 0..job["series"].n() {
                let mut samples = vec![];
                for n in 0..job["samples"].n() {
                    let start = Instant::now(); let errors = run(); let elapsed = start.elapsed().as_nanos() as u64;
                    if errors != expected { invalid.push(obj(&[("series", (s + 1).into()), ("sample", n.into()), ("errors", errors.into())])); }
                    samples.push(J::Num(elapsed));
                }
                series.push(J::Arr(samples));
            }
            outputs.push(obj(&[("id", job["id"].clone()), ("smoke_ns", J::Num(smoke)), ("series_ns", series.into()), ("invalid", invalid.into())]));
        }
    }
    fs::write(&args[3], json::dump(&J::Arr(outputs))).unwrap();
}
