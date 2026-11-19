//! The language runtime's child process (route RT1), built from this package so that the view-job tests and a
//! product build of the shell can start the runtime that matches the host library: `c600_runtime::child::main` on its
//! standard streams, as the `c600-runtime` binary of the language workspace runs it.

#![forbid(unsafe_code)]

fn main() {
    c600_runtime::child::main();
}
