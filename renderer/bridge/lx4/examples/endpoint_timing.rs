//! Headless timing of LX-4's editor endpoints through the module's C interface: each edit of a fixture folder
//! (`c600-lx4-fixtures --out`) replayed as lx4.py posts it, with a simulated frame loop in place of the shell's.
//!
//!     cargo run --release --manifest-path renderer/bridge/Cargo.toml -p c600-shell-lx4 --example endpoint_timing \
//!         -- FIXTURES [--samples N] [--frame-ms F] [--settle-ms W] [--targets S,M,L]
//!
//! Per sample: Reset and the frame that queues `ready`; then per replacement, in descending offset order, a Select,
//! frames for `--settle-ms` (default 250), the insert's keys in one burst and one frame. On the owner's machine
//! (25 November 2026, U8) lx4.py posted a sample's first key about 330 ms (p50; p10 250 ms) after its Reset, since it
//! waits for the shell's `ready` and `lx4-select` rows, so the work an earlier input started has ended by then; a
//! shorter settle times that work too. The clock starts just before the sample's timed post (the
//! first key for echo, the last key for every other endpoint, as lx4.py times them) and stops at the update that
//! first queues the endpoint. The frame loop calls `lx4_update` every `--frame-ms` (default 1, so the time is the
//! language work's own; 16.667 adds a 60 Hz shell's polling). It prints p50, p95 and max per target, kind and
//! endpoint, in milliseconds. Nothing here is a present, so no number is a display latency, Windows/DirectX or input
//! evidence: it times the work the shell waits for between the post and the frame that shows the endpoint.

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::path::Path;
use std::ptr;
use std::time::{Duration, Instant};

use c600_shell_lx4::fixtures::{self, Edit};
use c600_shell_lx4::*;

/// One of lx4.py's messages.
#[derive(Clone, Copy)]
enum Input {
    Select(u64, u64),
    Char(u16),
    Back,
}

struct Handle(*mut Lx4Module);

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: a live handle, used by nothing after this.
        unsafe { lx4_destroy(self.0) };
    }
}

fn check(code: i32, what: &str) {
    assert_eq!(code, LX4_OK, "{what} failed with {code}");
}

fn post(handle: &Handle, input: Input) {
    let mut taken = 0u32;
    // SAFETY: a live handle; `taken` is writable.
    unsafe {
        match input {
            Input::Select(start, end) => check(lx4_select(handle.0, start, end), "select"),
            Input::Char(unit) => check(lx4_char(handle.0, unit, 0, &mut taken), "char"),
            Input::Back => {
                check(lx4_backspace(handle.0), "backspace");
                check(lx4_char(handle.0, 0x08, 0x0E, &mut taken), "char 0x08");
            }
        }
    }
}

/// One frame: the update, then the endpoints it queued.
fn frame(handle: &Handle, id: u64) -> Vec<Lx4Shown> {
    let mut update = Lx4Update::default();
    // SAFETY: a live handle; `update` is writable.
    check(unsafe { lx4_update(handle.0, id, &mut update) }, "update");
    let mut rows = Vec::new();
    loop {
        let mut shown = Lx4Shown { sample: 0, frame_id: 0, endpoint: 0, reserved: 0, buffer_sha256: [0; 64] };
        let mut present = 0u32;
        // SAFETY: a live handle; both outputs are writable.
        check(unsafe { lx4_next_shown(handle.0, &mut shown, &mut present) }, "next_shown");
        if present == 0 {
            return rows;
        }
        rows.push(shown);
    }
}

/// The replacements' messages in descending offset order, one group per replacement: its Select and its keys.
fn groups(edit: &Edit) -> Vec<(Input, Vec<Input>)> {
    let mut order = edit.replacements.clone();
    order.sort_by_key(|r| Reverse(r.offset));
    order
        .iter()
        .map(|r| {
            let select = Input::Select(r.offset as u64, (r.offset + r.delete.len()) as u64);
            let keys = if r.insert.is_empty() {
                vec![Input::Back]
            } else {
                r.insert.replace('\n', "\r").encode_utf16().map(Input::Char).collect()
            };
            (select, keys)
        })
        .collect()
}

/// The endpoints an edit kind shows, as the module's tests expect them (W's dependents left out).
fn expected(edit: &Edit) -> Vec<&'static str> {
    let mut out = vec!["echo", "syntax"];
    match edit.kind.as_str() {
        "inside-body" | "exported-signature" => out.push("diagnostics"),
        "completion-trigger" => out.push("completion"),
        _ => {}
    }
    out
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let rank = ((p * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    sorted[rank - 1]
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let folder = args.first().expect("usage: endpoint_timing FIXTURES [--samples N] [--frame-ms F] [--targets S,M,L]");
    let option = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let samples: usize = option("--samples").map_or(30, |s| s.parse().expect("--samples N"));
    let frame_ms: f64 = option("--frame-ms").map_or(1.0, |s| s.parse().expect("--frame-ms F"));
    let settle =
        Duration::from_secs_f64(option("--settle-ms").map_or(250.0, |s| s.parse().expect("--settle-ms W")) / 1000.0);
    let targets: Vec<String> = option("--targets")
        .map_or_else(|| vec!["S".into(), "M".into(), "L".into()], |s| s.split(',').map(str::to_owned).collect());
    let period = Duration::from_secs_f64(frame_ms / 1000.0);
    let loaded = fixtures::load(Path::new(folder)).expect("the fixtures load");
    let mut out = ptr::null_mut();
    // SAFETY: the path's bytes are readable and `out` is writable.
    check(unsafe { lx4_create(folder.as_ptr(), folder.len(), &mut out) }, "create");
    let handle = Handle(out);
    let mut id = 1_000u64;
    let mut sample = 1u64;
    let mut times: BTreeMap<(String, String, &str), Vec<f64>> = BTreeMap::new();
    let tick = |handle: &Handle, id: &mut u64| {
        let next = Instant::now() + period;
        let rows = frame(handle, *id);
        *id += 1;
        while Instant::now() < next {
            std::hint::spin_loop();
        }
        rows
    };
    for round in 0..samples {
        for (index, edit) in loaded.edits.iter().enumerate() {
            if !targets.contains(&edit.target) {
                continue;
            }
            // SAFETY: a live handle.
            check(unsafe { lx4_reset(handle.0, index as u32, sample) }, "reset");
            let ready = tick(&handle, &mut id);
            assert!(ready.iter().any(|r| ENDPOINTS[r.endpoint as usize] == "ready"), "no ready row");
            let mut first: Option<Instant> = None;
            let mut last = Instant::now();
            let mut seen: Vec<Lx4Shown> = Vec::new();
            for (select, keys) in groups(edit) {
                post(&handle, select);
                let settled = Instant::now() + settle;
                seen.extend(tick(&handle, &mut id));
                while Instant::now() < settled {
                    seen.extend(tick(&handle, &mut id));
                }
                for (k, &key) in keys.iter().enumerate() {
                    if first.is_none() {
                        first = Some(Instant::now());
                    }
                    if k + 1 == keys.len() {
                        last = Instant::now();
                    }
                    post(&handle, key);
                }
                seen.extend(tick(&handle, &mut id));
            }
            let first = first.expect("a typed key");
            let mut at: BTreeMap<&str, f64> = BTreeMap::new();
            let wanted = expected(edit);
            let deadline = Instant::now() + Duration::from_secs(30);
            loop {
                let now = Instant::now();
                for row in seen.drain(..) {
                    let name = ENDPOINTS[row.endpoint as usize];
                    let start = if name == "echo" { first } else { last };
                    at.entry(name).or_insert_with(|| now.duration_since(start).as_secs_f64() * 1000.0);
                }
                if wanted.iter().all(|w| at.contains_key(w)) {
                    break;
                }
                assert!(Instant::now() < deadline, "{} {}: endpoints {:?}", edit.target, edit.kind, at.keys());
                seen.extend(tick(&handle, &mut id));
            }
            if round > 0 {
                for (name, ms) in at {
                    times.entry((edit.target.clone(), edit.kind.clone(), name)).or_default().push(ms);
                }
            }
            sample += 1;
        }
    }
    println!(
        "frame period {frame_ms} ms, settle {} ms, {} timed samples per cell (one warm-up round dropped)",
        settle.as_secs_f64() * 1000.0,
        samples - 1
    );
    println!("{:<4} {:<20} {:<12} {:>9} {:>9} {:>9}", "size", "edit", "endpoint", "p50", "p95", "max");
    for ((target, kind, name), mut list) in times {
        list.sort_by(f64::total_cmp);
        println!(
            "{target:<4} {kind:<20} {name:<12} {:>9.2} {:>9.2} {:>9.2}",
            percentile(&list, 0.5),
            percentile(&list, 0.95),
            list[list.len() - 1]
        );
    }
}
