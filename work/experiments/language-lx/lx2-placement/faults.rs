// Fault injectors, not proposed language limits. The native allocator ceiling
// makes actual allocation failure reproducible without exhausting the machine.
use std::{alloc::{GlobalAlloc, Layout, System}, cell::Cell, hint::black_box, ptr};
use crate::json::{J, num, object, string};

thread_local! { static NATIVE_LEFT: Cell<Option<usize>> = const { Cell::new(None) }; }
struct FaultAllocator;
#[global_allocator]
static ALLOCATOR: FaultAllocator = FaultAllocator;

fn debit(size: usize) -> bool {
    NATIVE_LEFT.try_with(|left| match left.get() {
        None => true,
        Some(n) if size <= n => { left.set(Some(n - size)); true },
        Some(_) => false,
    }).unwrap_or(true)
}

unsafe impl GlobalAlloc for FaultAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if debit(layout.size()) { unsafe { System.alloc(layout) } } else { ptr::null_mut() }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if debit(layout.size()) { unsafe { System.alloc_zeroed(layout) } } else { ptr::null_mut() }
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if debit(size) { unsafe { System.realloc(p, layout, size) } } else { ptr::null_mut() }
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) { unsafe { System.dealloc(p, layout) } }
}

fn exhausted(budget: &str, used: usize, limit: usize) -> J {
    object([("status", string("error")), ("code", string(format!("budget.{budget}"))),
        ("budget", string(budget)), ("used", num(used)), ("limit", num(limit))])
}

#[inline(never)]
fn metered_recursion(depth: usize) -> J {
    if depth == 64 { return exhausted("recursion", depth, 64); }
    let answer = metered_recursion(black_box(depth + 1));
    black_box(depth); answer
}

#[inline(never)]
#[allow(unconditional_recursion)]
fn native_recursion(depth: usize) -> usize {
    let frame = [depth as u8; 4096];
    black_box(&frame);
    let next = native_recursion(black_box(depth.wrapping_add(1)));
    next.wrapping_add(black_box(frame[depth % frame.len()]) as usize)
}

pub fn run(fault: &str) -> J {
    match fault {
        "control" => object([("status", string("ok")), ("code", string("completed"))]),
        "allocation_metered" => {
            let mut chunks = Vec::new(); let mut used = 0;
            loop {
                if used + 65_536 > 1_048_576 { return exhausted("memory_bytes", used, 1_048_576); }
                chunks.push(vec![0xA5u8; 65_536]); used += 65_536;
                black_box(&chunks);
            }
        },
        "recursion_metered" => metered_recursion(0),
        "loop_metered" => {
            let mut used = 0;
            loop {
                if used == 100_000 { return exhausted("instructions", used, 100_000); }
                used += 1;
                black_box(used);
            }
        },
        "allocation_native" => {
            let mut chunks = Vec::with_capacity(16);
            NATIVE_LEFT.with(|left| left.set(Some(8 * 1024 * 1024)));
            loop { chunks.push(vec![0xA5u8; 1024 * 1024]); black_box(&chunks); }
        },
        "recursion_native" => { black_box(native_recursion(0)); unreachable!() },
        "loop_native" | "forced_kill" => {
            let mut step = 0u64;
            loop { step = black_box(step.wrapping_add(1)); }
        },
        _ => panic!("unknown frozen fault"),
    }
}
