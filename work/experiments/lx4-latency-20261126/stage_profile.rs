//! The editor services' stages on one buffer, as LX-4's worker asks them (26 November 2026). Not part of any build:
//! copy it to rust/c600-editor/examples/ and run, in rust/,
//!
//!     cargo run --release --locked --offline -p c600-editor --example stage_profile -- BUFFER CARET [DEPENDENCY ...]
//!
//! BUFFER is one edit's result (an L fixture with an edit of EDITS.json applied), CARET the byte offset after its
//! last key, and each DEPENDENCY a module of the context (the fixtures' W/*.6cs). It prints p50 and min of 15 calls
//! of each piece on one document, then of parse, complete and diagnose in order on a fresh document per job, as
//! the worker opens one per buffer revision.
use std::time::Instant;

use c600_editor::{complete, diagnose, parse, Document, ModuleContext, Units};

fn time<T>(what: &str, n: usize, mut f: impl FnMut() -> T) -> T {
    let mut out = f();
    let mut list = Vec::new();
    for _ in 0..n {
        let t = Instant::now();
        out = f();
        list.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    list.sort_by(f64::total_cmp);
    println!("{what:<40} p50 {:>8.2} ms  min {:>8.2}", list[n / 2], list[0]);
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let text = std::fs::read_to_string(&args[0]).unwrap();
    let caret: usize = args[1].parse().unwrap();
    let deps = &args[2..];
    let mut context = ModuleContext::new();
    for d in deps {
        context.add_dependency(std::fs::read(d).unwrap(), None);
    }
    let n = 15;
    println!("bytes {}", text.len());
    time("c600_syntax::parse", n, || c600_syntax::parse(text.as_bytes()));
    time("c600_syntax::lexemes", n, || c600_syntax::lexemes(&text));
    let file = c600_syntax::parse(text.as_bytes());
    time("ParsedFile clone", n, || file.clone());
    if let Ok(lowered) = c600_syntax::lower(std::slice::from_ref(&file)) {
        time("c600_syntax::lower", n, || c600_syntax::lower(std::slice::from_ref(&file)).unwrap());
        time("c600_check::check", n, || c600_check::check(&lowered.program).unwrap());
    }
    time("Document::open", n, || Document::open(text.as_bytes(), Units::Utf8));
    let document = Document::open(text.as_bytes(), Units::Utf8);
    time("context.digest", n, || context.digest());
    time("c600_editor::parse", n, || parse(&document, &context.edition));
    let _ = time("c600_editor::complete", n, || complete(&document, document.digest(), caret, &context).map(|c| c.candidates.len()));
    let _ = time("c600_editor::diagnose", n, || diagnose(&document, document.digest(), &context).map(|d| d.diagnostics.len()));
    // As the bridge's worker: a fresh document per job, then parse, complete and diagnose in order.
    let mut stages = [Vec::new(), Vec::new(), Vec::new()];
    for _ in 0..n {
        let d = Document::open(text.as_bytes(), Units::Utf8);
        let t0 = Instant::now();
        let p = parse(&d, &context.edition);
        let t1 = Instant::now();
        let c = complete(&d, d.digest(), caret, &context).map(|c| c.candidates.len());
        let t2 = Instant::now();
        let g = diagnose(&d, d.digest(), &context).map(|g| g.diagnostics.len());
        let t3 = Instant::now();
        std::hint::black_box((p, c, g));
        for (k, (a, b)) in [(t0, t1), (t1, t2), (t2, t3)].into_iter().enumerate() {
            stages[k].push((b - a).as_secs_f64() * 1000.0);
        }
    }
    for (k, name) in ["job: parse", "job: complete", "job: diagnose"].into_iter().enumerate() {
        stages[k].sort_by(f64::total_cmp);
        println!("{name:<40} p50 {:>8.2} ms  min {:>8.2}", stages[k][n / 2], stages[k][0]);
    }
}
