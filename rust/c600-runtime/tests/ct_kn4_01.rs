//! CT-KN4-01's source scan for the runtime process: the code that runs a program (the child's loop, the program
//! loader, the codec, the wire and the messages) reaches no file system, path or network of its own.

use std::path::{Path, PathBuf};

/// The first segments of the `std::` paths in Rust source with its line comments removed. A brace group gives the
/// first segment of each of its items.
fn std_modules(source: &str) -> Vec<String> {
    let ident = |s: &str| s.trim_start().chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect::<String>();
    let code: Vec<&str> = source.lines().map(|line| line.split("//").next().unwrap_or("")).collect();
    let code = code.join("\n");
    let mut found = Vec::new();
    let mut rest = code.as_str();
    while let Some(at) = rest.find("std::") {
        let inside_a_name = rest[..at].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_');
        rest = &rest[at + "std::".len()..];
        if inside_a_name {
            continue;
        }
        let Some(group) = rest.trim_start().strip_prefix('{') else {
            found.push(ident(rest));
            continue;
        };
        let (mut depth, mut item) = (0usize, String::new());
        for c in group.chars() {
            match c {
                '{' => depth += 1,
                '}' if depth == 0 => break,
                '}' => depth -= 1,
                ',' if depth == 0 => found.push(ident(&std::mem::take(&mut item))),
                _ => {}
            }
            if depth > 0 || !matches!(c, ',' | '{' | '}') {
                item.push(c);
            }
        }
        if !ident(&item).is_empty() {
            found.push(ident(&item));
        }
    }
    found
}

/// CT-KN4-01 (part: the runtime child's source reaches no file system, path or network; not run: an operating-system
/// level check that the child opens no session file, which needs the LP3 owner-machine run): the child's loop, the
/// program loader, the codec, the wire, the messages and the entry point use no `std::fs`, `std::path` or `std::net`.
/// The child's own use of the environment (the fault variable), its exits and its standard streams is not claimed
/// here. The parent (which locates the child's executable) and the test-only hooked binary are outside the scan.
#[test]
fn the_runtime_child_makes_no_file_system_or_network_call() {
    let sample = std_modules("use std::{fs, io::{self, Write}};\nlet x = std::path::Path::new(\"a\"); // std::net\n");
    assert_eq!(sample, ["fs", "io", "path"], "the scan reads groups and skips comments");

    const DENIED: [&str; 3] = ["fs", "path", "net"];
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let scanned: Vec<PathBuf> = ["child", "program", "codec", "wire", "message", "main", "lib"]
        .iter()
        .map(|name| src.join(format!("{name}.rs")))
        .collect();
    let mut seen = Vec::new();
    for file in &scanned {
        let source = std::fs::read_to_string(file).expect("a runtime source file");
        for module in std_modules(&source) {
            assert!(!DENIED.contains(&module.as_str()), "{} uses std::{module}", file.display());
            seen.push(module);
        }
    }
    assert!(seen.iter().any(|m| m == "io"), "the scan found the child's std paths");
    // The scan is not vacuous: the parent does use `std::path`, and the scan sees it.
    let parent = std::fs::read_to_string(src.join("parent.rs")).expect("the parent source");
    assert!(std_modules(&parent).iter().any(|m| m == "path"), "the scan would see a path use");
}
