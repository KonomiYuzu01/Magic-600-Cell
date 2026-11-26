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
use std::time::{Duration, Instant};

use c600_render::FakeCore;
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
