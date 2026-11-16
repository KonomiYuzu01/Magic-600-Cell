//! The loader's refusals (M4; findings R1, R2 and R5 of the critical-path read): the model ID names exactly the listed
//! files and digests, every listed digest is checked, every file the kernel reads is listed, a listed name stays a
//! plain file name in `assets/`, and a malformed manifest is refused. Each case writes a manifest, and the small files
//! it lists, into a fresh folder under the test's temporary directory, so the shipped `assets/` is only read.
//!
//! Not tested here: the checks of the arrays and trees inside verified archives (a primitive that is not a permutation
//! of its support, malformed offsets, an execution tree whose walk does not end). Such a test has to write a changed
//! archive under a new model ID, and the crate only reads archives; these tests are deferred (rust/README.md, M4).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use c600_digest::sha256_hex;
use c600_kernel::json::{self, Value};
use c600_kernel::{Kernel, LoadError};

const NOT_PLAIN: &str = "manifest.json lists a file that is not a plain file name in assets/";
const NOT_NAMED: &str = "manifest.json's model_id does not match its file list";

fn assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

/// The shipped manifest's file list and model ID.
fn shipped() -> (BTreeMap<String, String>, String) {
    let manifest = json::parse(&fs::read(assets().join("manifest.json")).unwrap()).unwrap();
    let files = manifest.get("files").and_then(Value::as_object).unwrap();
    let files = files.iter().map(|(name, digest)| (name.clone(), digest.as_str().unwrap().to_owned())).collect();
    (files, manifest.get("model_id").and_then(Value::as_str).unwrap().to_owned())
}

/// The model ID of a file list, as `build_assets.py` computes it:
/// `sha256(json.dumps(files, sort_keys=True, separators=(',', ':')))`, for ASCII names.
fn model_id(files: &BTreeMap<String, String>) -> String {
    let entries: Vec<String> = files.iter().map(|(name, digest)| format!("\"{name}\":\"{digest}\"")).collect();
    sha256_hex(format!("{{{}}}", entries.join(",")).as_bytes())
}

/// A manifest in `build_assets.py`'s indented layout.
fn manifest(files: &BTreeMap<String, String>, model_id: &str) -> String {
    let entries: Vec<String> = files.iter().map(|(name, digest)| format!("    \"{name}\": \"{digest}\"")).collect();
    let files = entries.join(",\n");
    format!("{{\n  \"puzzle\": \"600-cell-Full\",\n  \"files\": {{\n{files}\n  }},\n  \"model_id\": \"{model_id}\"\n}}")
}

/// Loads a fresh folder that holds `manifest` and `files`, and gives the refusal's text.
fn refusal(case: &str, manifest: &str, files: &[(&str, &[u8])]) -> String {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("c600-kernel-load").join(case);
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("manifest.json"), manifest).unwrap();
    for (name, bytes) in files {
        fs::write(folder.join(name), bytes).unwrap();
    }
    match Kernel::load(&folder) {
        Ok(_) => panic!("{case}: the folder loads"),
        Err(LoadError(text)) => text,
    }
}

/// The positive control: the shipped model ID is the digest of the shipped file list, computed as the loader does.
#[test]
fn the_shipped_model_id_names_the_shipped_file_list() {
    let (files, id) = shipped();
    assert_eq!(model_id(&files), id);
}

/// R1: a manifest that lists other digests under the shipped model ID is refused before any file is read. The read's
/// counterexample: `cell_normals.f32` with one byte appended and its entry digested again.
#[test]
fn a_model_id_that_does_not_name_the_file_list_is_refused() {
    let (mut files, id) = shipped();
    let mut normals = fs::read(assets().join("cell_normals.f32")).unwrap();
    normals.push(0);
    files.insert("assets/cell_normals.f32".into(), sha256_hex(&normals));
    assert_eq!(refusal("relisted", &manifest(&files, &id), &[]), NOT_NAMED);

    // The shipped list under another well-formed ID, and the list with one file left out, are refused alike.
    let (mut files, _) = shipped();
    assert_eq!(refusal("other-id", &manifest(&files, &"0".repeat(64)), &[]), NOT_NAMED);
    files.remove("assets/mesh.json");
    assert_eq!(refusal("dropped", &manifest(&files, &id), &[]), NOT_NAMED);
}

/// R2: every listed digest is checked, and a listed file must exist.
#[test]
fn every_listed_digest_is_checked() {
    let original = fs::read(assets().join("puzzle_definition.txt")).unwrap();
    let files = BTreeMap::from([("assets/puzzle_definition.txt".to_owned(), sha256_hex(&original))]);
    let text = manifest(&files, &model_id(&files));
    let mut changed = original.clone();
    changed.push(b'\n');
    let mismatch = refusal("changed", &text, &[("puzzle_definition.txt", &changed)]);
    assert_eq!(mismatch, "Asset hash mismatch: assets/puzzle_definition.txt");
    assert_eq!(refusal("missing", &text, &[]), "Cannot read assets/puzzle_definition.txt");
}

/// R2: a file the kernel reads must be listed, so that its digest is checked.
#[test]
fn every_file_the_kernel_reads_is_listed() {
    let original = fs::read(assets().join("puzzle_definition.txt")).unwrap();
    let files = BTreeMap::from([("assets/puzzle_definition.txt".to_owned(), sha256_hex(&original))]);
    let text = refusal("unlisted", &manifest(&files, &model_id(&files)), &[("puzzle_definition.txt", &original)]);
    assert_eq!(text, "manifest.json does not list model.npz");
}

/// R2 and R5: a listed name stays a plain file name in `assets/`. A separator, a parent, a drive or stream colon, a
/// non-ASCII letter, an empty name or a name outside `assets/` is refused before any file is read.
#[test]
fn listed_names_are_plain_file_names_in_assets() {
    let digest = "0".repeat(64);
    let names = [
        "assets/../manifest.json",
        "assets/sub/model.npz",
        "assets/a\\\\b",
        "assets/C:model.npz",
        "assets/model.npz:stream",
        "assets/mod\\u00e9l.npz",
        "assets/",
        "assets/.",
        "assets/..",
        "model.npz",
        "/assets/model.npz",
    ];
    for name in names {
        let text = format!(
            "{{\"puzzle\": \"600-cell-Full\", \"files\": {{\"{name}\": \"{digest}\"}}, \"model_id\": \"{digest}\"}}"
        );
        assert_eq!(refusal("names", &text, &[]), NOT_PLAIN, "{name}");
    }
}

/// A malformed manifest is refused with the loader's texts.
#[test]
fn malformed_manifests_are_refused() {
    let digest = "0".repeat(64);
    let upper = "A".repeat(64);
    let cases = [
        ("not JSON", "{\"puzzle\": ".to_owned(), "manifest.json: "),
        (
            "another puzzle",
            format!("{{\"puzzle\": \"600-cell-Partial\", \"files\": {{}}, \"model_id\": \"{digest}\"}}"),
            "manifest.json does not name 600-cell-Full",
        ),
        (
            "short model_id",
            "{\"puzzle\": \"600-cell-Full\", \"files\": {}, \"model_id\": \"abc\"}".to_owned(),
            "manifest.json has no well-formed model_id",
        ),
        (
            "upper-case model_id",
            format!("{{\"puzzle\": \"600-cell-Full\", \"files\": {{}}, \"model_id\": \"{upper}\"}}"),
            "manifest.json has no well-formed model_id",
        ),
        (
            "no file list",
            format!("{{\"puzzle\": \"600-cell-Full\", \"model_id\": \"{digest}\"}}"),
            "manifest.json lists no files",
        ),
        (
            "malformed digest",
            format!("{{\"puzzle\": \"600-cell-Full\", \"files\": {{\"assets/model.npz\": \"xyz\"}}, \"model_id\": \"{digest}\"}}"),
            "Malformed digest in manifest",
        ),
        (
            "empty list",
            format!("{{\"puzzle\": \"600-cell-Full\", \"files\": {{}}, \"model_id\": \"{}\"}}", sha256_hex(b"{}")),
            "manifest.json does not list model.npz",
        ),
    ];
    for (case, manifest, expected) in cases {
        let text = refusal("malformed", &manifest, &[]);
        assert!(text.starts_with(expected), "{case}: {text}");
    }
}
