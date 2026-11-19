//! Probe, never shipped (C9 U7's limit windows): how long the shell's metered LX-2 view jobs (the bodies of
//! Lx2Module.cs) take to reach their meter's limit, in process with NoHost, at rising limits up to the host's
//! VIEW_LIMITS (steps 2^32, memory 2^32), each capped at 70 s by a by-user cancel. It shows whether a job reaches its
//! limit before the host's 60 s view deadline. Measured on 19 November 2026 (release, CPU shared with a build): none
//! does; the loop runs about 6 M steps/s (2^28 steps in 44 s), the recursion reached 2^30 bytes in 58 s, and the
//! allocation's push grows quadratically (2^18 bytes in 37 s, 2^20 not within 70 s). Usage:
//! cargo run --release --locked --offline --manifest-path rust/Cargo.toml -p c600-lx2 --example view_limits_probe

use std::time::{Duration, Instant};

use c600_run::interp::{Cancel, NoHost};
use c600_run::meter::{Amounts, Interruption};
use c600_runtime::{run_in_process, Ended, Job, JobOptions};

const HEADER: &str = "edition \"6cs-draft-1\";\nmodule lx2 requires [];\n\n";
const WIDE: Amounts = Amounts { steps: 1 << 32, kernel_cost: 1 << 40, memory: 1 << 32, output: 1 << 26 };
const CAP: Duration = Duration::from_secs(70);

struct Deadline(Instant);

impl Cancel for Deadline {
    fn pending(&self) -> Option<Interruption> {
        (Instant::now() >= self.0).then_some(Interruption::ByUser)
    }
}

fn source(fault: &str) -> String {
    let (body, down) = match fault {
        "alloc_metered" => (
            "    let grown = repeat(1000000000000, list<Nat>(), fn(done: List<Nat>) -> List<Nat> {\n        return push(done, length(done));\n    });\n    return length(grown);\n",
            "",
        ),
        "recursion_metered" => (
            "    return down(0);\n",
            "\nfn down(n: Nat) -> Nat effects [] {\n    return down(exact_add(n, 1));\n}\n",
        ),
        _ => (
            "    return repeat(1000000000000, 0, fn(acc: Nat) -> Nat {\n        return exact_add(acc, 1);\n    });\n",
            "",
        ),
    };
    format!("{HEADER}export fn work() -> Nat effects [] {{\n{body}}}\n{down}")
}

fn main() {
    for fault in ["loop_metered", "recursion_metered", "alloc_metered"] {
        let exponents: &[u32] =
            if fault == "loop_metered" { &[20, 22, 24, 26, 28, 30, 32] } else { &[16, 18, 20, 22, 24, 26, 28, 30] };
        for &e in exponents {
            let limits = if fault == "loop_metered" {
                Amounts { steps: 1 << e, ..WIDE }
            } else {
                Amounts { memory: 1 << e, ..WIDE }
            };
            let job = Job {
                sources: vec![source(fault).into_bytes()],
                module: "lx2".into(),
                entry: "work".into(),
                args: Vec::new(),
                options: JobOptions { record: false, ..JobOptions::new(limits) },
            };
            let started = Instant::now();
            let ended = run_in_process(&job, &mut NoHost, Some(&Deadline(started + CAP)));
            let seconds = started.elapsed().as_secs_f64();
            let outcome = match ended {
                Ended::Ran(run) => format!(
                    "{:?} {:?} steps={} memory={}",
                    run.summary.outcome, run.summary.code, run.summary.used.steps, run.summary.used.memory
                ),
                Ended::Refused(refused) => format!("refused {refused:?}"),
            };
            println!("{fault} 2^{e}: {seconds:.3} s, {outcome}");
            if seconds >= CAP.as_secs_f64() {
                break;
            }
        }
    }
}
