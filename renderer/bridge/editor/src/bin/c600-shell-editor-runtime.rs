//! The language runtime's child process (route RT1), built from this package so that the editor library's tests can
//! run a program job's bound entry in the runtime's own process: `c600_runtime::child::main` on its standard streams,
//! as the host's `c600-shell-runtime` binary runs it. A product build starts the host's binary for both libraries.

#![forbid(unsafe_code)]

fn main() {
    c600_runtime::child::main();
}
