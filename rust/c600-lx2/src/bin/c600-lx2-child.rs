//! LX-2's fault-injection child (option (a), top integrator, 18 November 2026): `c600-runtime`'s child entry point,
//! unchanged, under a global allocator that refuses any allocation that would take the process's live heap bytes past a
//! ceiling, and, built with this package's feature `device`, on LP3's fault hook (`c600_runtime::child::main_with`,
//! c600-runtime's feature `lx2-faults`) for the native recursion and the native loop. Built from product source in the
//! product's release profile; it is a measurement device and never shipped: no package depends on `c600-lx2`, and
//! `tests/device.rs` checks that.
//!
//! The ceiling is [`c600_lx2::CEILING_VARIABLE`] in decimal bytes, read once before the child starts; without it the
//! allocator only counts. A malformed value ends the process before the child announces itself. A refused allocation
//! returns null, so the standard library reports the allocation failure and aborts the process, as a real
//! out-of-memory would end the product's child.
//!
//! The native fault is [`c600_lx2::FAULT_VARIABLE`], `recursion@<answers>` or `loop@<answers>`, read once before the
//! child starts. The hook runs on the interpreter's thread before each call frame and each result with the answers the
//! child has received; at the first call with at least the given answers the device writes its line to standard error
//! and fires: the recursion calls itself in native code without end until the thread's stack overflows, which the
//! standard library reports before the process ends; the loop spins in native code without a kernel call until the
//! parent's watchdog kills the process. A malformed value ends the process before the child announces itself, and so
//! does any value in a build without the feature, so that an armed trial never runs on a device without the hook.
//!
//! The unsafe code lives here only; `c600-runtime` and the `c600-lx2` library keep `forbid(unsafe_code)`.

#![deny(unsafe_op_in_unsafe_fn)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

use c600_lx2::NativeFault;

/// The live heap bytes handed out by this allocator and not yet returned.
static LIVE: AtomicUsize = AtomicUsize::new(0);
/// The ceiling on [`LIVE`]; no ceiling until `main` reads the variable.
static LIMIT: AtomicUsize = AtomicUsize::new(usize::MAX);

/// Reserves `size` bytes under the ceiling; false when they would pass it.
fn reserve(size: usize) -> bool {
    let limit = LIMIT.load(Relaxed);
    LIVE.fetch_update(Relaxed, Relaxed, |live| live.checked_add(size).filter(|&total| total <= limit)).is_ok()
}

fn release(size: usize) {
    LIVE.fetch_sub(size, Relaxed);
}

/// [`System`] behind the ceiling.
struct Ceiling;

// SAFETY: every call is forwarded unchanged to `System`, which upholds `GlobalAlloc`'s contract; the wrapper only
// refuses (returns null, which the contract allows) or counts.
unsafe impl GlobalAlloc for Ceiling {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !reserve(layout.size()) {
            return null_mut();
        }
        // SAFETY: the caller upholds `alloc`'s contract for `layout`.
        let block = unsafe { System.alloc(layout) };
        if block.is_null() {
            release(layout.size());
        }
        block
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if !reserve(layout.size()) {
            return null_mut();
        }
        // SAFETY: the caller upholds `alloc_zeroed`'s contract for `layout`.
        let block = unsafe { System.alloc_zeroed(layout) };
        if block.is_null() {
            release(layout.size());
        }
        block
    }

    unsafe fn dealloc(&self, block: *mut u8, layout: Layout) {
        // SAFETY: `block` came from this allocator, that is from `System`, with `layout`.
        unsafe { System.dealloc(block, layout) };
        release(layout.size());
    }

    unsafe fn realloc(&self, block: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old = layout.size();
        if new_size > old && !reserve(new_size - old) {
            // The old block stays valid, as after a failed `System.realloc`.
            return null_mut();
        }
        // SAFETY: `block` came from `System` with `layout`, and the caller upholds `realloc`'s contract.
        let moved = unsafe { System.realloc(block, layout, new_size) };
        match (moved.is_null(), new_size > old) {
            (true, true) => release(new_size - old),
            (false, false) => release(old - new_size),
            _ => {}
        }
        moved
    }
}

#[global_allocator]
static ALLOCATOR: Ceiling = Ceiling;

/// Keeps Windows Error Reporting out of this process's crashes (C9 U7, 20 Nov 2026: the unattended run's injected stack
/// overflows left hundreds of WER events). The overflow ends the process through the unhandled-exception filter, which
/// with `SEM_NOGPFAULTERRORBOX` does not start Windows Error Reporting; the exit status (0xC00000FD) and the standard
/// library's line on standard error are unchanged. The error mode is this process's own, inherited only by processes
/// it starts; no system or user setting changes. A fail-fast abort, such as the allocator ceiling's, still reaches
/// Windows Error Reporting.
#[cfg(windows)]
fn quiet_error_reporting() {
    const SEM_FAILCRITICALERRORS: u32 = 0x0001;
    const SEM_NOGPFAULTERRORBOX: u32 = 0x0002;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetErrorMode() -> u32;
        fn SetErrorMode(mode: u32) -> u32;
    }
    // SAFETY: both calls take and return a flag word and touch only this process's error mode.
    unsafe {
        SetErrorMode(GetErrorMode() | SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
    }
}

#[cfg(not(windows))]
fn quiet_error_reporting() {}

/// Ends the process before the child announces itself.
fn refuse(reason: &str) -> ! {
    eprintln!("c600-lx2-child: {reason}");
    std::process::exit(2);
}

/// The native faults on c600-runtime's fault hook.
#[cfg(feature = "device")]
mod native {
    use std::hint::black_box;
    use std::sync::OnceLock;

    use c600_lx2::NativeFault;

    /// The armed fault and the answers after which it fires.
    pub static ARMED: OnceLock<(NativeFault, u64)> = OnceLock::new();

    /// Calls itself without end. Each frame keeps a block the optimizer cannot drop and adds the callee's result to
    /// it, so no call is a tail call and every call takes stack.
    #[inline(never)]
    fn recurse(depth: u64) -> u64 {
        let frame = black_box([depth; 32]);
        if black_box(true) {
            recurse(frame[0].wrapping_add(1)).wrapping_add(frame[31])
        } else {
            frame[31]
        }
    }

    /// Spins without a kernel call and never returns.
    fn spin() -> ! {
        let mut turns = 0_u64;
        loop {
            turns = black_box(turns.wrapping_add(1));
        }
    }

    /// The fault hook: fires the armed fault at the first point with at least its answers received.
    pub fn point(answers: u64) {
        let Some(&(fault, after)) = ARMED.get() else { return };
        if answers < after {
            return;
        }
        eprintln!("{} after {answers} answers", fault.line());
        match fault {
            NativeFault::Recursion => {
                black_box(recurse(0));
            }
            NativeFault::Loop => spin(),
        }
    }
}

fn main() {
    quiet_error_reporting();
    if let Some(value) = std::env::var_os(c600_lx2::CEILING_VARIABLE) {
        match value.to_str().and_then(|text| text.parse::<usize>().ok()) {
            Some(limit) => LIMIT.store(limit, Relaxed),
            None => refuse(&format!("{} is not a byte count", c600_lx2::CEILING_VARIABLE)),
        }
    }
    let armed = std::env::var_os(c600_lx2::FAULT_VARIABLE).map(|value| {
        value
            .to_str()
            .and_then(NativeFault::parse)
            .unwrap_or_else(|| refuse(&format!("{} is not <recursion|loop>@<answers>", c600_lx2::FAULT_VARIABLE)))
    });
    #[cfg(feature = "device")]
    {
        if let Some(armed) = armed {
            let _ = native::ARMED.set(armed);
        }
        c600_runtime::child::main_with(native::point);
    }
    #[cfg(not(feature = "device"))]
    {
        if armed.is_some() {
            refuse(&format!(
                "{} is set, but this build has no fault hook: build c600-lx2 with --features device",
                c600_lx2::FAULT_VARIABLE
            ));
        }
        c600_runtime::child::main();
    }
}
