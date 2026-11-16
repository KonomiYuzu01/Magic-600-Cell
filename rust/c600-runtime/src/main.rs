//! The `c600-runtime` child process: the protocol of `c600_runtime::message` on its standard streams.

#![forbid(unsafe_code)]

fn main() {
    c600_runtime::child::main();
}
