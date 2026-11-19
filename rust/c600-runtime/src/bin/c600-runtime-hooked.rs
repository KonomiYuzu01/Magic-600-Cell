//! A test child with LX-2's fault hook (feature `lx2-faults`): `c600_runtime::child::main_with` with a hook that
//! appends each count of answers it is given, one per line, to the file `C600_RUNTIME_HOOK_LOG` names. It injects no
//! fault: c600-lx2's injection device does that in its own crate.

#![forbid(unsafe_code)]

use std::io::Write;

/// The variable that names the file the hook appends to.
const LOG: &str = "C600_RUNTIME_HOOK_LOG";

fn record(answers: u64) {
    let Some(path) = std::env::var_os(LOG) else { return };
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path).expect("the hook's log opens");
    writeln!(file, "{answers}").expect("the hook's log takes a line");
}

fn main() {
    c600_runtime::child::main_with(record);
}
