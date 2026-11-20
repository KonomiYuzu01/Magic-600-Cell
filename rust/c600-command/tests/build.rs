//! The command layer's build string hashes every source file of the crate, as the kernel's does
//! (`c600-kernel/tests/kernel.rs`, `build_hashes_every_source_file`), so a new module cannot leave the build identity.

use std::path::Path;

#[test]
fn build_hashes_every_source_file() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec!["Cargo.toml".to_owned()];
    let mut pending = vec![root.join("src")];
    while let Some(folder) = pending.pop() {
        for entry in std::fs::read_dir(folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"));
            }
        }
    }
    files.sort();
    let listed: Vec<&str> =
        c600_command::SOURCES.iter().filter_map(|(name, _)| name.strip_prefix("c600-command/")).collect();
    assert_eq!(listed, files);
    for (name, bytes) in c600_command::SOURCES.iter().filter(|(name, _)| name.starts_with("c600-command/")) {
        assert_eq!(std::fs::read(root.join("..").join(name)).unwrap(), *bytes, "{name}");
    }
    // The one other entry is the stored command table.
    let others: Vec<&str> =
        c600_command::SOURCES.iter().map(|(name, _)| *name).filter(|name| !name.starts_with("c600-command/")).collect();
    assert_eq!(others, ["docs/progress/1.0/command-table.json"]);
    let build = c600_command::build();
    let digest = build.strip_prefix(&format!("c600-command {} sources-sha256:", env!("CARGO_PKG_VERSION"))).unwrap();
    assert!(digest[..64].bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
    assert_eq!(&digest[64..66], "; ");
}
