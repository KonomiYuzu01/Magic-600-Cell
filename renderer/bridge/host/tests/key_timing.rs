//! The frame thread's time per twist key (C9, 26 Nov 2026): a timing bench, not a pass/fail check of the product.
//!
//! It drives the host's C interface over the fake core on route B, with a session in a fresh folder on this machine's
//! disk (the build's temporary folder), as a frame loop on a modelled 60 Hz display with a frame latency of 1: each
//! iteration routes a twist key every `EVERY`th time, then submits a frame; the frame is shown at the first refresh
//! after its submission, the loop waits for that refresh as the core's frame-latency wait does, and the fake core then
//! completes the work submitted so far (a GPU finishes a preparation in the interval after its submission). It prints
//! the key call's time, the frame call's time, the refreshes that showed no new frame (the stutter), and the time and
//! the frames from each key-down to the first frame that draws both of the press's revisions, submitted and shown. Run
//! it in a release build:
//!
//! ```text
//! cargo test --release --manifest-path renderer/bridge/Cargo.toml -p c600-shell-host --test key_timing -- --ignored --nocapture
//! ```
//!
//! `KEYS` (default 60) sets the number of keys and `PUZZLE` (0 Jumble off, 1 Jumble on) the puzzle. The fake core
//! has no GPU: the real core's adoption and preparation cost more, and the shell's own work is not here. Nothing here
//! is Windows/DirectX, input or performance evidence.

#[path = "../../tests/common/mod.rs"]
mod common;

use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use c600_jumble::twist::Menu;
use c600_kernel::json;
use c600_kernel::Kernel;
use c600_render::FakeCore;
use c600_shell_host::turns::Turns;
use c600_shell_host::*;
use common::support::SIZE;
use common::{functions, Shim};

/// The modelled display's refresh.
const REFRESH: Duration = Duration::from_micros(16_667);
/// A key every this many frames: about four keys a second.
const EVERY: usize = 15;

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn setting(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}

fn quantiles(mut values: Vec<f64>) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }
    values.sort_by(f64::total_cmp);
    let at = |q: f64| values[((values.len() - 1) as f64 * q).round() as usize];
    format!("n={} p50={:.2} p90={:.2} p99={:.2} max={:.2}", values.len(), at(0.5), at(0.9), at(0.99), at(1.0))
}

#[test]
#[ignore = "a timing bench; run it in a release build with --ignored --nocapture"]
fn frame_thread_time_per_twist_key() {
    let keys = setting("KEYS", 60);
    let puzzle = setting("PUZZLE", 0) as u32;
    let shim = Box::into_raw(Box::new(Shim { fake: FakeCore::on_surface(SIZE), pad: 0 }));
    // SAFETY: the host uses the shim only inside its calls, never at the same time as the bench.
    let fake = || unsafe { &mut (*shim).fake };
    let assets = checkout().join("assets").to_str().expect("UTF-8").to_owned();
    let menu = checkout().join("research/jumbling/fixtures/wj-I_a.json").to_str().expect("UTF-8").to_owned();
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("key-timing-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let folder = folder.to_str().expect("UTF-8").to_owned();
    let mut host = ptr::null_mut();
    // SAFETY: the shim's table and context, readable paths and writable outputs.
    unsafe {
        assert_eq!(m6h_create(&functions(), shim.cast(), assets.as_ptr(), assets.len(), &mut host), M6H_OK);
        fake().complete();
        assert_eq!(m6h_pump(host), M6H_OK);
        assert_eq!(m6h_set_viewport(host, SIZE.0, SIZE.1), M6H_OK);
        let code = m6h_open_session(
            host,
            puzzle,
            folder.as_ptr(),
            folder.len(),
            menu.as_ptr(),
            menu.len(),
            ptr::null(),
            0,
            ptr::null_mut(),
        );
        assert_eq!(code, M6H_OK);
    }
    let chords = ["KeyJ", "KeyK", "KeyL", "Shift+KeyJ", "Shift+KeyK", "Shift+KeyL"];
    let (mut key_times, mut frame_times) = (Vec::new(), Vec::new());
    let (mut drawn_ms, mut drawn_frames, mut shown_ms) = (Vec::new(), Vec::new(), Vec::new());
    // Presses waiting for their frame: the key-down's instant, its frame index and its revision.
    let mut waiting: Vec<(Instant, usize, u64)> = Vec::new();
    let start = Instant::now();
    // The refresh that shows the newest frame, and the refreshes that showed no new frame.
    let (mut shown, mut missed) = (0u128, 0u128);
    let frames = keys * EVERY + 2 * EVERY;
    for index in 0..frames {
        if index % EVERY == EVERY / 2 && index / EVERY < keys {
            let chord = chords[(index / EVERY) % chords.len()];
            let mut out = M6hKey::default();
            let down = Instant::now();
            // SAFETY: a live handle, readable bytes and a writable row.
            let code = unsafe { m6h_key_down(host, chord.as_ptr(), chord.len(), 0, &mut out) };
            key_times.push(ms(down.elapsed()));
            assert_eq!((code, out.press), (M6H_OK, M6H_PRESS_COMMITTED));
            // SAFETY: as above.
            assert_eq!(unsafe { m6h_key_up(host, chord.as_ptr(), chord.len()) }, M6H_OK);
            waiting.push((down, index, out.engine_revision));
        }
        let mut frame = M6hFrame::default();
        let called = Instant::now();
        // SAFETY: a live handle and a writable row.
        assert_eq!(unsafe { m6h_frame(host, &mut frame) }, M6H_OK);
        let returned = Instant::now();
        frame_times.push(ms(returned - called));
        // Shown at the first refresh after its submission, one frame per refresh.
        let refresh = (returned - start).as_nanos().div_ceil(REFRESH.as_nanos()).max(shown + 1);
        if index > 0 {
            missed += refresh - shown - 1;
        }
        shown = refresh;
        let display = start + REFRESH * shown as u32;
        waiting.retain(|&(down, at, revision)| {
            let drawn = frame.engine_revision >= revision && frame.publication_revision >= revision;
            if drawn {
                drawn_ms.push(ms(returned - down));
                drawn_frames.push((index - at) as f64);
                shown_ms.push(ms(display - down));
            }
            !drawn
        });
        // The core's frame-latency wait: the next frame starts once this one is on screen.
        if let Some(rest) = display.checked_duration_since(Instant::now()) {
            std::thread::sleep(rest);
        }
        fake().complete();
    }
    eprintln!("puzzle={puzzle} keys={keys} frames={frames}");
    eprintln!("m6h_key_down ms: {}", quantiles(key_times));
    eprintln!("m6h_frame ms: {}", quantiles(frame_times));
    eprintln!("refreshes that showed no new frame: {missed} (over {shown} refreshes)");
    eprintln!("key-down to the frame that draws it, submitted, ms: {}", quantiles(drawn_ms));
    eprintln!("key-down to the frame that draws it, frames: {}", quantiles(drawn_frames));
    eprintln!("key-down to the refresh that shows that frame, ms: {}", quantiles(shown_ms));
    assert!(waiting.is_empty(), "every press was drawn");
    // SAFETY: the handle is not used again; the shim outlives it.
    unsafe {
        m6h_destroy(host);
        drop(Box::from_raw(shim));
    }
    let _ = std::fs::remove_dir_all(&folder);
}

/// The `wj-ia-300` session fixture (`renderer/shell/build_d02_fixtures.py`, git-ignored under `work/build/`): its
/// record and a fresh copy of its session folder named `name`; `None` when the fixture is not built on this machine.
fn wj_ia_300(name: &str) -> Option<(PathBuf, PathBuf)> {
    let root = checkout().join("work/build/d02-fixtures/wj-ia-300");
    let record = root.join("fixture.json");
    if !record.is_file() {
        eprintln!("skipped: the wj-ia-300 fixture is not built (renderer/shell/build_d02_fixtures.py)");
        return None;
    }
    let copy = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&copy);
    copy_dir(&root.join("session"), &copy);
    Some((record, copy))
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a folder");
    for entry in std::fs::read_dir(from).expect("a folder") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("a copy");
        }
    }
}

/// SF (27 Nov 2026): the menu a session keeps ([`Turns::pinned_menu`]) is built once and answers as a fresh build from
/// the record does: its name, items and two digests, and every listed cap's twists.
#[test]
fn the_kept_menu_answers_as_a_fresh_build_at_the_wj_ia_300_head() {
    let Some((record, copy)) = wj_ia_300("kept-menu") else { return };
    let assets = checkout().join("assets");
    let menu_file = checkout().join("research/jumbling/fixtures/wj-I_a.json");
    let fixture = fixture::read(&record).expect("the fixture");
    let kernel = Arc::new(Kernel::load(&assets).expect("the model"));
    let mut turns = Turns::reopen(kernel, &assets, &copy, &menu_file, &fixture).expect("the copy reopens");
    let kept = turns.pinned_menu().expect("the kept menu");
    assert!(Arc::ptr_eq(&kept, &turns.pinned_menu().expect("the kept menu")), "the menu is built once");
    let exact = c600_jumble::Context::load(&assets).expect("the exact model");
    let file = json::parse(&std::fs::read(&menu_file).expect("the menu file")).expect("JSON");
    let fresh = Menu::from_record(&exact, file.get("menu").expect("a menu record")).expect("a fresh menu");
    assert_eq!((&kept.name, &kept.identity, &kept.order), (&fresh.name, &fresh.identity, &fresh.order));
    assert_eq!(kept.items, fresh.items);
    for cap in [0, 57, 119] {
        let twists = |menu: &Menu| format!("{:?}", menu.for_cap(&exact, cap).expect("the cap's twists"));
        assert_eq!(twists(&kept), twists(&fresh), "cap {cap}");
    }
    // The kept rows: a row read by a call, and the rows a commit reads ahead at the new head, are fresh reads' rows.
    let asked = [(91u32, 1u64), (91, 2), (91, 3)];
    for (cap, item) in asked {
        let row = turns.twist_transform(cap, item).expect("a row");
        assert_eq!(row, turns.fresh_twist_transform(cap, item).expect("a fresh row"), "[{cap}, {item}]");
    }
    turns.set_grip(91, 0).expect("the grip");
    assert_eq!(turns.key_down("KeyJ", false).expect("a press"), c600_shell_host::turns::Press::Committed);
    turns.key_up("KeyJ").expect("a key-up");
    let waited = Instant::now();
    while !asked.iter().all(|&(cap, item)| turns.twist_row_kept(cap, item)) {
        assert!(waited.elapsed() < Duration::from_secs(20), "the rows were not read ahead");
        std::thread::sleep(Duration::from_millis(5));
    }
    for (cap, item) in asked {
        let row = turns.twist_transform(cap, item).expect("a kept row");
        assert_eq!(row, turns.fresh_twist_transform(cap, item).expect("a fresh row"), "ahead [{cap}, {item}]");
    }
    let _ = std::fs::remove_dir_all(&copy);
}

/// SF (27 Nov 2026): `m6h_twist_transform` at the `wj-ia-300` head, which the shell calls twice per twist on its frame
/// thread (the press's row before the key, the cut slice's after the commit), takes under 10 ms per call once its row
/// is kept: rows asked before are read ahead after each commit. Twelve presses at the gate's pace (250 ms), the rows
/// [91, 1..3] read 250 ms after each. It took 66-72 ms when every call built the menu again; a row never asked before
/// still reads afresh (printed, not bounded: `State::apply`'s admissibility check). Not Windows/DirectX or performance
/// evidence.
#[test]
fn twist_transform_at_the_wj_ia_300_head_takes_under_10_ms() {
    let Some((record, copy)) = wj_ia_300("twist-transform") else { return };
    let shim = Box::into_raw(Box::new(Shim { fake: FakeCore::on_surface(SIZE), pad: 0 }));
    // SAFETY: the host uses the shim only inside its calls, never at the same time as the test.
    let fake = || unsafe { &mut (*shim).fake };
    let assets = checkout().join("assets").to_str().expect("UTF-8").to_owned();
    let menu = checkout().join("research/jumbling/fixtures/wj-I_a.json").to_str().expect("UTF-8").to_owned();
    let (folder, record) = (copy.to_str().expect("UTF-8").to_owned(), record.to_str().expect("UTF-8").to_owned());
    let mut host = ptr::null_mut();
    let mut revision = 0u64;
    // SAFETY: the shim's table and context, readable paths and writable outputs.
    unsafe {
        assert_eq!(m6h_create(&functions(), shim.cast(), assets.as_ptr(), assets.len(), &mut host), M6H_OK);
        fake().complete();
        assert_eq!(m6h_pump(host), M6H_OK);
        let code = m6h_open_session(
            host,
            M6H_PUZZLE_JUMBLE_ON,
            folder.as_ptr(),
            folder.len(),
            menu.as_ptr(),
            menu.len(),
            record.as_ptr(),
            record.len(),
            &mut revision,
        );
        assert_eq!(code, M6H_OK);
    }
    let (mut plane, mut angle, mut status, mut at) = ([0f32; 8], 0f32, 0u32, 0u64);
    let mut moving = vec![0u8; 177_120];
    let mut read = |cap: u32, item: u64| {
        let called = Instant::now();
        // SAFETY: a live handle and writable outputs of the documented sizes.
        let code = unsafe {
            m6h_twist_transform(host, cap, item, plane.as_mut_ptr(), &mut angle, moving.as_mut_ptr(), &mut status, &mut at)
        };
        assert_eq!(code, M6H_OK);
        ms(called.elapsed())
    };
    let rows = [(91u32, 1u64), (91, 2), (91, 3)];
    let cold: Vec<f64> = rows.iter().map(|&(cap, item)| read(cap, item)).collect();
    // SAFETY: a live handle.
    assert_eq!(unsafe { m6h_grip_set(host, 91, 0) }, M6H_OK);
    let mut times = Vec::new();
    for press in 0..12 {
        let chord = ["KeyJ", "KeyK", "KeyL"][press % 3];
        let mut out = M6hKey::default();
        // SAFETY: a live handle, readable chord bytes and a writable result.
        unsafe {
            assert_eq!(m6h_key_down(host, chord.as_ptr(), chord.len(), 0, &mut out), M6H_OK);
            assert_eq!(m6h_key_up(host, chord.as_ptr(), chord.len()), M6H_OK);
        }
        assert_eq!(out.press, M6H_PRESS_COMMITTED, "press {press}");
        let pressed = Instant::now();
        while pressed.elapsed() < Duration::from_millis(250) {
            fake().complete();
            // SAFETY: a live handle.
            assert_eq!(unsafe { m6h_pump(host) }, M6H_OK);
            std::thread::sleep(Duration::from_millis(16));
        }
        times.extend(rows.iter().map(|&(cap, item)| read(cap, item)));
    }
    eprintln!("m6h_twist_transform ms at wj-ia-300, first reads: {}", quantiles(cold));
    eprintln!("m6h_twist_transform ms at wj-ia-300, kept rows after a commit: {}", quantiles(times.clone()));
    // SAFETY: the handle is live and no other thread uses it.
    unsafe {
        m6h_destroy(host);
        drop(Box::from_raw(shim));
    }
    let _ = std::fs::remove_dir_all(&copy);
    let slowest = times.iter().copied().fold(0.0, f64::max);
    assert!(slowest < 10.0, "the slowest m6h_twist_transform took {slowest:.2} ms");
}

/// SF (27 Nov 2026): `m6h_key_down`'s steps at the `wj-ia-300` head, split on the host's own calls: the routed press
/// (the router, the command layer's commit and journal write, the host's two head digests and the read-ahead's start)
/// and the head's copy for the adoption worker. The C interface's rest (the hand-off and the preview sync) is the
/// difference from the scratch bench's `m6h_key_down`. A timing bench, not a check.
#[test]
#[ignore = "a timing bench; run it in a release build with --ignored --nocapture"]
fn key_down_steps_at_the_wj_ia_300_head() {
    let Some((record, copy)) = wj_ia_300("key-down-steps") else { return };
    let assets = checkout().join("assets");
    let menu_file = checkout().join("research/jumbling/fixtures/wj-I_a.json");
    let fixture = fixture::read(&record).expect("the fixture");
    let kernel = Arc::new(Kernel::load(&assets).expect("the model"));
    let mut turns = Turns::reopen(kernel, &assets, &copy, &menu_file, &fixture).expect("the copy reopens");
    turns.set_grip(91, 0).expect("the grip");
    let (mut routed, mut copied) = (Vec::new(), Vec::new());
    for press in 0..20 {
        let chord = ["KeyJ", "KeyK", "KeyL"][press % 3];
        let start = Instant::now();
        let pressed = turns.key_down(chord, false).expect("a press");
        let routed_at = Instant::now();
        let _head = turns.head().expect("the head");
        let copied_at = Instant::now();
        turns.key_up(chord).expect("a key-up");
        assert_eq!(pressed, c600_shell_host::turns::Press::Committed, "press {press}");
        routed.push(ms(routed_at - start));
        copied.push(ms(copied_at - routed_at));
        std::thread::sleep(Duration::from_millis(250));
    }
    eprintln!("Turns::key_down ms: {}", quantiles(routed));
    eprintln!("Turns::head ms: {}", quantiles(copied));
    let _ = std::fs::remove_dir_all(&copy);
}
