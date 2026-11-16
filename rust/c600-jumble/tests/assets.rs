//! The jumbling model is read through the kernel's asset checks (KN0.1): a manifest whose model ID is not the SHA-256
//! of its file list, or that does not list both archives, is refused before any array is read, so the identity
//! block's model ID always names the files that were read.

use std::path::{Path, PathBuf};

use c600_jumble::model::Model;

/// A scratch `assets/` folder under the test's temporary directory: the given files and a manifest of 600-cell-Full
/// listing them, with the model ID of their file list unless `model_id` replaces it.
fn scratch_assets(name: &str, files: &[(&str, &[u8])], model_id: Option<&str>) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("c600-jumble-assets").join(name);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    let mut listing = Vec::new();
    for (file, bytes) in files {
        std::fs::write(folder.join(file), bytes).unwrap();
        listing.push(format!("\"assets/{file}\":\"{}\"", c600_digest::sha256_hex(bytes)));
    }
    listing.sort();
    let list = format!("{{{}}}", listing.join(","));
    let id = model_id.map_or_else(|| c600_digest::sha256_hex(list.as_bytes()), str::to_owned);
    let manifest = format!("{{\"puzzle\":\"600-cell-Full\",\"model_id\":\"{id}\",\"files\":{list}}}");
    std::fs::write(folder.join("manifest.json"), manifest).unwrap();
    folder
}

fn refusal(folder: &Path) -> String {
    match Model::load(folder) {
        Ok(_) => panic!("the assets were accepted"),
        Err(error) => error.0,
    }
}

#[test]
fn the_model_id_names_the_files_read() {
    let archive: &[u8] = b"not an archive";
    let both = [("model.npz", archive), ("primitives.npz", archive)];
    let other_id = "0".repeat(64);
    let renamed = scratch_assets("model-id", &both, Some(&other_id));
    assert_eq!(refusal(&renamed), "manifest.json's model_id does not match its file list");
    let unlisted = scratch_assets("unlisted", &both[..1], None);
    assert_eq!(refusal(&unlisted), "manifest.json does not list assets/primitives.npz");
    // A manifest that passes the checks reaches the archive, which this one is not.
    let listed = scratch_assets("listed", &both, None);
    assert!(refusal(&listed).starts_with("model.npz: "));
    for folder in [renamed, unlisted, listed] {
        let _ = std::fs::remove_dir_all(folder);
    }
}
