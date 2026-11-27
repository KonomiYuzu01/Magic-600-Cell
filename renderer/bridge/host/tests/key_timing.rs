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
            m6h_twist_transform(
                host,
                cap,
                item,
                plane.as_mut_ptr(),
                &mut angle,
                moving.as_mut_ptr(),
                &mut status,
                &mut at,
            )
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

/// Brief SF2: a host over the fake core with a fresh copy of the `wj-ia-300` session open, Jumble on.
struct Opened {
    host: *mut ShellHost,
    shim: *mut Shim,
    copy: PathBuf,
}

impl Opened {
    fn new(name: &str) -> Option<Opened> {
        let (record, copy) = wj_ia_300(name)?;
        let shim = Box::into_raw(Box::new(Shim { fake: FakeCore::on_surface(SIZE), pad: 0 }));
        let assets = checkout().join("assets").to_str().expect("UTF-8").to_owned();
        let menu = checkout().join("research/jumbling/fixtures/wj-I_a.json").to_str().expect("UTF-8").to_owned();
        let (folder, record) = (copy.to_str().expect("UTF-8").to_owned(), record.to_str().expect("UTF-8").to_owned());
        let mut host = ptr::null_mut();
        let mut revision = 0u64;
        // SAFETY: the shim's table and context, readable paths and writable outputs; the shim is used only inside
        // the host's calls.
        unsafe {
            assert_eq!(m6h_create(&functions(), shim.cast(), assets.as_ptr(), assets.len(), &mut host), M6H_OK);
            (*shim).fake.complete();
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
        Some(Opened { host, shim, copy })
    }

    fn row(&self, cap: u32, item: u64) -> Option<u64> {
        let (mut plane, mut angle, mut status, mut at) = ([0f32; 8], 0f32, 0u32, 0u64);
        let mut moving = vec![0u8; 177_120];
        // SAFETY: a live handle and writable outputs of the documented sizes.
        let code = unsafe {
            m6h_twist_transform(
                self.host,
                cap,
                item,
                plane.as_mut_ptr(),
                &mut angle,
                moving.as_mut_ptr(),
                &mut status,
                &mut at,
            )
        };
        assert_eq!(code, M6H_OK);
        (status == 1).then_some(at)
    }

    fn digest(&self) -> String {
        // SAFETY: a live handle.
        assert_eq!(unsafe { m6h_state_digest(self.host) }, M6H_OK);
        let mut size = 0u32;
        // SAFETY: a null buffer only asks for the size; then the buffer holds `size` bytes.
        unsafe {
            assert_eq!(m6h_answer(self.host, ptr::null_mut(), 0, &mut size), M6H_E_INVALID_ARGUMENT);
            let mut buffer = vec![0u8; size as usize];
            assert_eq!(m6h_answer(self.host, buffer.as_mut_ptr(), size, &mut size), M6H_OK);
            String::from_utf8_lossy(&buffer).into_owned()
        }
    }

    /// Every file of the session's folder but the lock, by relative path.
    fn files(&self) -> std::collections::BTreeMap<String, Vec<u8>> {
        let mut files = std::collections::BTreeMap::new();
        read_folder(&self.copy, &self.copy, &mut files);
        files
    }

    /// Closes the session (its correctness control reopens the journal under a fresh random epoch) and removes it.
    fn close(self) {
        let mut pass = 0u32;
        // SAFETY: a live handle and a writable flag; then the handle is destroyed and no thread uses it.
        unsafe {
            assert_eq!(m6h_close_session(self.host, 0, &mut pass), M6H_OK);
            m6h_destroy(self.host);
            drop(Box::from_raw(self.shim));
        }
        let _ = std::fs::remove_dir_all(&self.copy);
    }
}

fn read_folder(root: &Path, at: &Path, files: &mut std::collections::BTreeMap<String, Vec<u8>>) {
    for entry in std::fs::read_dir(at).expect("a folder") {
        let entry = entry.expect("an entry");
        let path = entry.path();
        if entry.file_type().expect("a type").is_dir() {
            read_folder(root, &path, files);
        } else if entry.file_name() != "lock" {
            let name = path.strip_prefix(root).expect("inside").to_string_lossy().replace('\\', "/");
            files.insert(name, std::fs::read(&path).expect("a file"));
        }
    }
}

/// A cap at the `wj-ia-300` head where `KeyJ` (item 1, its own inverse) twists, and one where its row is not
/// admissible, so the command layer refuses it.
fn caps(opened: &Opened) -> Option<(u32, u32)> {
    let (mut twists, mut refuses) = (None, None);
    for cap in 0..120u32 {
        match opened.row(cap, 1) {
            Some(_) => twists = twists.or(Some(cap)),
            None => refuses = refuses.or(Some(cap)),
        }
        if let (Some(twists), Some(refuses)) = (twists, refuses) {
            return Some((twists, refuses));
        }
    }
    None
}

/// Sets the grip cap (`m6h_grip_set`; behind posted keys it routes them first, in order).
fn grip(host: *mut ShellHost, cap: u32) {
    // SAFETY: a live handle.
    assert_eq!(unsafe { m6h_grip_set(host, cap, 0) }, M6H_OK);
}

/// Polls until `count` answers arrived (or 60 s passed): (ticket, code, press, engine revision, row status, row
/// revision) in arrival order.
fn poll_answers(host: *mut ShellHost, count: usize) -> Vec<(u64, i32, u32, u64, u32, u64)> {
    let mut answers = Vec::new();
    let start = Instant::now();
    while answers.len() < count && start.elapsed() < Duration::from_secs(60) {
        let mut answer = M6hKeyAnswer::default();
        // SAFETY: a live handle and a writable answer.
        assert_eq!(unsafe { m6h_key_poll(host, &mut answer) }, M6H_OK);
        if answer.ticket == 0 {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }
        answers.push((
            answer.ticket,
            answer.code,
            answer.press,
            answer.engine_revision,
            answer.row_status,
            answer.row_revision,
        ));
    }
    answers
}

/// Brief SF2: a burst of keys posted in a row, with a refused key in the middle, ends at the revision, the head's state
/// digests and the session's bytes (its journal and objects, read before the close, whose correctness control reopens
/// the journal under a fresh random epoch) that the same keys sent synchronously give; each answer
/// is the synchronous call's, in press order, with the armed row read at the head the key twisted. A burst posted and
/// then closed at once is drained by the close: the same bytes, and every key answered after it. Each post returns
/// on the caller's thread in under 2 ms.
#[test]
fn posted_keys_end_where_synchronous_keys_end_at_the_wj_ia_300_head() {
    let Some(sync) = Opened::new("keys-sync") else { return };
    let Some((twists, refuses)) = caps(&sync) else { panic!("no cap at the wj-ia-300 head refuses KeyJ") };
    // KeyJ twice at a cap where it twists (back to the same poses), once where it is refused, once more where it
    // twists; the grip changes between them route the keys posted before them first.
    let burst = [(twists, "KeyJ", 1u64), (twists, "KeyJ", 1), (refuses, "KeyJ", 1), (twists, "KeyJ", 1)];
    let mut expected = Vec::new();
    let mut at_cap = u32::MAX;
    for &(cap, chord, item) in &burst {
        if cap != at_cap {
            grip(sync.host, cap);
            at_cap = cap;
        }
        let at = sync.row(cap, item);
        let mut out = M6hKey::default();
        // SAFETY: a live handle, readable chord bytes and a writable result.
        unsafe {
            assert_eq!(m6h_key_down(sync.host, chord.as_ptr(), chord.len(), 0, &mut out), M6H_OK);
            assert_eq!(m6h_key_up(sync.host, chord.as_ptr(), chord.len()), M6H_OK);
        }
        expected.push((M6H_OK, out.press, out.engine_revision, u32::from(at.is_some()), at.unwrap_or(0)));
    }
    let presses: Vec<u32> = expected.iter().map(|e| e.1).collect();
    assert_eq!(presses, [M6H_PRESS_COMMITTED, M6H_PRESS_COMMITTED, M6H_PRESS_REFUSED, M6H_PRESS_COMMITTED]);
    let sync_digest = sync.digest();
    let sync_files = sync.files();
    sync.close();
    assert!(sync_files.keys().any(|name| name.starts_with("journal")), "the session has a journal");

    let mut post_times = Vec::new();
    let post = |host: *mut ShellHost, times: &mut Vec<f64>| -> Vec<u64> {
        let mut tickets = Vec::new();
        let mut at_cap = u32::MAX;
        for &(cap, chord, item) in &burst {
            if cap != at_cap {
                grip(host, cap);
                at_cap = cap;
            }
            let mut ticket = 0u64;
            let called = Instant::now();
            // SAFETY: a live handle, readable chord bytes and a writable ticket.
            let code = unsafe { m6h_key_post(host, chord.as_ptr(), chord.len(), 0, 1, cap, item, &mut ticket) };
            times.push(ms(called.elapsed()));
            assert_eq!(code, M6H_OK);
            // SAFETY: as above; behind posted keys the key-up is queued.
            assert_eq!(unsafe { m6h_key_up(host, chord.as_ptr(), chord.len()) }, M6H_OK);
            tickets.push(ticket);
        }
        tickets
    };

    let posted = Opened::new("keys-posted").expect("the fixture is built");
    let tickets = post(posted.host, &mut post_times);
    let answers = poll_answers(posted.host, burst.len());
    assert_eq!(answers.iter().map(|a| a.0).collect::<Vec<_>>(), tickets, "every key answered, in press order");
    let got: Vec<_> = answers.iter().map(|a| (a.1, a.2, a.3, a.4, a.5)).collect();
    assert_eq!(got, expected, "each answer is the synchronous call's");
    assert_eq!(posted.digest(), sync_digest, "the head's state digests");
    same_files(&posted.files(), &sync_files, "the posted keys", false);
    posted.close();

    let closed = Opened::new("keys-closed").expect("the fixture is built");
    let tickets = post(closed.host, &mut post_times);
    let mut pass = 0u32;
    // SAFETY: a live handle and a writable flag.
    assert_eq!(unsafe { m6h_close_session(closed.host, 0, &mut pass) }, M6H_OK, "the close drains the queue");
    let answers = poll_answers(closed.host, burst.len());
    assert_eq!(answers.iter().map(|a| a.0).collect::<Vec<_>>(), tickets, "no key lost or half-answered at the close");
    assert_eq!(answers.iter().map(|a| a.2).collect::<Vec<_>>(), presses);
    let mut files = std::collections::BTreeMap::new();
    read_folder(&closed.copy, &closed.copy, &mut files);
    // SAFETY: the handle is live and no other thread uses it.
    unsafe {
        m6h_destroy(closed.host);
        drop(Box::from_raw(closed.shim));
    }
    let _ = std::fs::remove_dir_all(&closed.copy);
    // The control's reopen after the drain appends records under its own random epoch: the keys' bytes come first.
    same_files(&files, &sync_files, "the drained keys", true);

    eprintln!("m6h_key_post ms on the caller's thread at wj-ia-300: {}", quantiles(post_times.clone()));
    let slowest = post_times.iter().copied().fold(0.0, f64::max);
    assert!(slowest < 2.0, "the slowest m6h_key_post took {slowest:.2} ms");
}

/// The JSON records of a journal, in order, with the session's random command epoch masked (each open writes a fresh
/// one, which every command ID of that session carries, and so every request digest), and the framing bytes between
/// them dropped.
fn records(journal: &[u8], epochs: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(start) = journal[at..].windows(2).position(|w| w == b"{\"").map(|p| p + at) {
        let (mut depth, mut text, mut escaped, mut end) = (0i32, false, false, None);
        for (k, &b) in journal[start..].iter().enumerate() {
            if text {
                match (escaped, b) {
                    (true, _) => escaped = false,
                    (false, b'\\') => escaped = true,
                    (false, b'"') => text = false,
                    _ => {}
                }
                continue;
            }
            match b {
                b'"' => text = true,
                b'{' | b'[' => depth += 1,
                b'}' | b']' => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                end = Some(start + k + 1);
                break;
            }
        }
        let Some(end) = end else { break };
        let mut record = String::from_utf8_lossy(&journal[start..end]).into_owned();
        for epoch in epochs {
            record = record.replace(epoch.as_str(), "EPOCH");
        }
        // A request's digest hashes its command ID, which carries the epoch.
        let key = "\"request\":\"";
        let mut from = 0;
        while let Some(found) = record[from..].find(key) {
            let start = from + found + key.len();
            let end = record[start..].find('"').map_or(record.len(), |e| start + e);
            record.replace_range(start..end, "REQUEST");
            from = start;
        }
        out.push(record);
        at = end;
    }
    out
}

/// The command epochs a journal names (`{"epoch":N,"record":"command_epoch"}`).
fn epochs(journal: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(journal);
    text.match_indices("{\"epoch\":")
        .filter_map(|(at, key)| {
            let digits: String = text[at + key.len()..].chars().take_while(char::is_ascii_digit).collect();
            (!digits.is_empty()).then_some(digits)
        })
        .collect()
}

fn newest(journal: &[u8]) -> Vec<String> {
    epochs(journal).pop().into_iter().collect()
}

/// Asserts that `got` holds `want`'s session: its objects byte for byte and its journal's records in order, equal
/// but for each session's random command epoch (with `prefix`, `got`'s journal starts with `want`'s records).
fn same_files(
    got: &std::collections::BTreeMap<String, Vec<u8>>,
    want: &std::collections::BTreeMap<String, Vec<u8>>,
    what: &str,
    prefix: bool,
) {
    assert_eq!(got.keys().collect::<Vec<_>>(), want.keys().collect::<Vec<_>>(), "{what}: the session's files");
    for (name, bytes) in want {
        if name == "journal" {
            // The fixture's own epochs are shared; this session's open wrote the newest one.
            // With `prefix`, the epoch at the same place (the control's reopen wrote a later one).
            let at = epochs(bytes).len();
            let their_epoch: Vec<String> = epochs(&got[name]).into_iter().skip(at - 1).take(1).collect();
            let (theirs, ours) = (records(&got[name], &their_epoch), records(bytes, &newest(bytes)));
            assert!(
                ours.len() > 10 && theirs.len() >= ours.len(),
                "{what}: {} records vs {}",
                theirs.len(),
                ours.len()
            );
            let theirs = if prefix { &theirs[..ours.len()] } else { &theirs[..] };
            assert_eq!(theirs.len(), ours.len(), "{what}: the journal's record count");
            for (k, (a, b)) in theirs.iter().zip(&ours).enumerate() {
                assert_eq!(a, b, "{what}: journal record {k} of {}", ours.len());
            }
        } else {
            assert!(got[name] == *bytes, "{what}: {name} differs");
        }
    }
}
