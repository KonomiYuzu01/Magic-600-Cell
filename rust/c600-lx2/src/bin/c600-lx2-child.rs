//! LX-2's fault-injection child (option (a), top integrator, 18 November 2026): `c600-runtime`'s child entry point,
//! unchanged (`c600_runtime::child::main`), under a global allocator that refuses any allocation that would take the
//! process's live heap bytes past a ceiling. Built from product source in the product's release profile; it is a
//! measurement device and never shipped: no package depends on `c600-lx2`, and `tests/device.rs` checks that.
//!
//! The ceiling is [`c600_lx2::CEILING_VARIABLE`] in decimal bytes, read once before the child starts; without it the
//! allocator only counts. A malformed value ends the process before the child announces itself. A refused allocation
//! returns null, so the standard library reports the allocation failure and aborts the process, as a real
//! out-of-memory would end the product's child. The unsafe code lives here only; `c600-runtime` and the `c600-lx2`
//! library keep `forbid(unsafe_code)`.

#![deny(unsafe_op_in_unsafe_fn)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

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

fn main() {
    if let Some(value) = std::env::var_os(c600_lx2::CEILING_VARIABLE) {
        match value.to_str().and_then(|text| text.parse::<usize>().ok()) {
            Some(limit) => LIMIT.store(limit, Relaxed),
            None => {
                eprintln!("c600-lx2-child: {} is not a byte count", c600_lx2::CEILING_VARIABLE);
                std::process::exit(2);
            }
        }
    }
    c600_runtime::child::main();
}
