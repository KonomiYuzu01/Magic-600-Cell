//! The loader's refusals (M4; findings R1, R2 and R5 of the critical-path read): the model ID names exactly the listed
//! files and digests, every listed digest is checked, every file the kernel reads is listed, a listed name stays a
//! plain file name in `assets/`, and a malformed manifest is refused. Each case writes a manifest, and the small files
//! it lists, into a fresh folder under the test's temporary directory, so the shipped `assets/` is only read.
//!
//! The checks of the arrays and trees inside verified archives (lead L10 of the M4 differential tests, the deferred
//! half of R2) are tested on copies of the five files the kernel reads with one change each: a test-only writer packs
//! the changed array into a stored ZIP archive or the changed trees into a gzip member of stored DEFLATE blocks, and a
//! manifest lists the five files under the model ID of that list. The same copy without a change is the model of
//! another identity that lead L8 offers the shipped kernel's values to (CT-KN1-01).
//!
//! `verify_assets` is the check itself; `Kernel::load` and the jumbling model both load through it.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use c600_digest::sha256_hex;
use c600_kernel::archive::{self, crc32, Zip};
use c600_kernel::json::{self, Value};
use c600_kernel::{codes, Derivation, Error, Kernel, LoadError, ProtectedSet, Refusal, Verdict};

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

/// The files the kernel reads besides the manifest.
const NEEDED: [&str; 5] = ["model.npz", "primitives.npz", "execution_trees.json.gz", "census.json", "seed_atlas.json"];

/// A fresh folder with the five files the kernel reads, from `assets/` unless `changed` gives other bytes, and a
/// manifest that lists only those five under the model ID of that list. A fixture of these tests, never shipped.
fn model_folder(case: &str, changed: &[(&str, Vec<u8>)]) -> (PathBuf, String) {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("c600-kernel-load").join(case);
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    let (shipped, _) = shipped();
    let mut files = BTreeMap::new();
    for name in NEEDED {
        let listed = format!("assets/{name}");
        let digest = match changed.iter().find(|(changed, _)| *changed == name) {
            Some((_, bytes)) => {
                fs::write(folder.join(name), bytes).unwrap();
                sha256_hex(bytes)
            }
            None => {
                fs::copy(assets().join(name), folder.join(name)).unwrap();
                shipped[&listed].clone()
            }
        };
        files.insert(listed, digest);
    }
    let id = model_id(&files);
    fs::write(folder.join("manifest.json"), manifest(&files, &id)).unwrap();
    (folder, id)
}

/// The loader's refusal of a copy of the model with the `changed` files. The copy is removed afterwards.
fn refused_copy(case: &str, changed: &[(&str, Vec<u8>)]) -> String {
    let (folder, _) = model_folder(case, changed);
    let loaded = Kernel::load(&folder);
    let _ = fs::remove_dir_all(&folder);
    match loaded {
        Ok(_) => panic!("{case}: the changed copy loads"),
        Err(LoadError(text)) => text,
    }
}

/// A ZIP archive of stored (uncompressed) members without ZIP64 fields, which the crate's reader accepts.
fn stored_zip(members: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut directory = Vec::new();
    for (name, data) in members {
        let (offset, crc, size) = (out.len() as u32, crc32(data), data.len() as u32);
        // Version needed, flags, method 0 (stored), time and date, CRC-32, compressed and plain sizes, name length.
        let mut fields = vec![20, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        fields.extend(crc.to_le_bytes());
        fields.extend(size.to_le_bytes());
        fields.extend(size.to_le_bytes());
        fields.extend((name.len() as u16).to_le_bytes());
        out.extend(0x0403_4b50u32.to_le_bytes());
        out.extend(&fields);
        out.extend([0, 0]); // no extra field
        out.extend(name.as_bytes());
        out.extend(data);
        directory.extend(0x0201_4b50u32.to_le_bytes());
        directory.extend([20, 0]); // version made by
        directory.extend(&fields);
        directory.extend([0; 12]); // extra and comment lengths, disk, internal and external attributes
        directory.extend(offset.to_le_bytes());
        directory.extend(name.as_bytes());
    }
    let (start, size, count) = (out.len() as u32, directory.len() as u32, members.len() as u16);
    out.extend(directory);
    out.extend(0x0605_4b50u32.to_le_bytes());
    out.extend([0; 4]); // disk numbers
    out.extend(count.to_le_bytes());
    out.extend(count.to_le_bytes());
    out.extend(size.to_le_bytes());
    out.extend(start.to_le_bytes());
    out.extend([0, 0]); // no comment
    out
}

/// A gzip member of stored DEFLATE blocks.
fn stored_gzip(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff];
    let blocks: Vec<&[u8]> = data.chunks(65_535).collect();
    for (i, block) in blocks.iter().enumerate() {
        out.push(u8::from(i + 1 == blocks.len())); // BFINAL on the last block, BTYPE 00
        let length = block.len() as u16;
        out.extend(length.to_le_bytes());
        out.extend((!length).to_le_bytes());
        out.extend(*block);
    }
    out.extend(crc32(data).to_le_bytes());
    out.extend((data.len() as u32).to_le_bytes());
    out
}

/// The members of the shipped `primitives.npz`, read once with the crate's ZIP reader.
fn primitive_members() -> &'static [(String, Vec<u8>)] {
    static MEMBERS: OnceLock<Vec<(String, Vec<u8>)>> = OnceLock::new();
    MEMBERS.get_or_init(|| {
        let bytes = fs::read(assets().join("primitives.npz")).unwrap();
        let zip = Zip::parse(&bytes).unwrap();
        zip.names().into_iter().map(|name| (name.to_owned(), zip.read(name, 1 << 30).unwrap())).collect()
    })
}

/// The loader's refusal of the shipped model with one `<i4` array of `primitives.npz` changed in place.
fn changed_primitives(case: &str, member: &str, change: impl FnOnce(&mut Values<'_>)) -> String {
    let mut members = primitive_members().to_vec();
    let (_, bytes) = members.iter_mut().find(|(name, _)| name == member).unwrap();
    change(&mut Values::of(bytes));
    refused_copy(case, &[("primitives.npz", stored_zip(&members))])
}

/// The values of an NPY array of little-endian int32, in place.
struct Values<'a> {
    data: &'a mut [u8],
}

impl<'a> Values<'a> {
    fn of(npy: &'a mut [u8]) -> Values<'a> {
        let start = match npy[6] {
            1 => 10 + u16::from_le_bytes([npy[8], npy[9]]) as usize,
            _ => 12 + u32::from_le_bytes([npy[8], npy[9], npy[10], npy[11]]) as usize,
        };
        Values { data: &mut npy[start..] }
    }

    fn len(&self) -> usize {
        self.data.len() / 4
    }

    fn get(&self, i: usize) -> i32 {
        i32::from_le_bytes(self.data[4 * i..4 * i + 4].try_into().unwrap())
    }

    fn set(&mut self, i: usize, value: i32) {
        self.data[4 * i..4 * i + 4].copy_from_slice(&value.to_le_bytes());
    }
}

/// CT-KN1-01 (part: snapshots and recipes of another model; identity blocks and the registry in `tests/kernel.rs`)
/// and lead L8 (M4). A copy of the five files the kernel reads, listed alone, is a model with the shipped mechanics
/// under another model ID. Its kernel refuses the shipped kernel's snapshots and recipes with
/// `KN1:input.identity_mismatch` in every call that takes one, and its own recipe gives the shipped net map.
#[test]
fn ct_kn1_01_values_of_another_model_are_refused() {
    let (folder, id) = model_folder("another-model", &[]);
    let shipped = Kernel::load(&assets()).unwrap();
    let other = Kernel::load(&folder).unwrap();
    let _ = fs::remove_dir_all(&folder);
    assert_eq!(other.identity().model_id, id);
    assert_ne!(id, shipped.identity().model_id);
    assert_eq!(other.check_identity(&shipped.identity()).unwrap_err().code, codes::IDENTITY_MISMATCH);

    let (state, recipe) = (shipped.solved(), shipped.word(&[1]).unwrap());
    let (own_state, own_recipe, none) = (other.solved(), other.word(&[1]).unwrap(), ProtectedSet::none());
    let refusal = Refusal { code: codes::IDENTITY_MISMATCH, message: "The value belongs to another model_id".into() };
    let mismatch = Error::Refusal(refusal.clone());
    assert_eq!(other.derive(&state, &own_recipe, &none).unwrap_err(), mismatch);
    assert_eq!(other.derive(&own_state, &recipe, &none).unwrap_err(), mismatch);
    assert_eq!(other.effect(&recipe).unwrap_err(), mismatch);
    assert_eq!(other.effect_by_replay(&recipe).unwrap_err(), mismatch);
    assert_eq!(other.progress(&state).unwrap_err(), refusal);
    assert_eq!(other.positions(&state, &[0]).unwrap_err(), refusal);
    let size = std::num::NonZeroUsize::new(5).unwrap();
    assert_eq!(other.chunks(&recipe, size).err(), Some(refusal.clone()));
    assert_eq!(other.replay(&state, &none).err(), Some(refusal.clone()));
    assert_eq!(other.replay_under_ceiling(&state, &none).err(), Some(refusal.clone()));
    // Until the finding of lead L8 was fixed, letters() expanded another model's recipe without a check.
    assert_eq!(other.letters(&recipe).unwrap_err(), refusal);
    assert_eq!(other.letters(&own_recipe).unwrap(), shipped.letters(&recipe).unwrap());

    // The same mechanics under another identity give the same net map and the same labels digest.
    assert_eq!(other.effect(&own_recipe).unwrap(), shipped.effect(&recipe).unwrap());
    assert_eq!(own_state.digest(), state.digest());

    // KN1.4 for protected sets (M4's open item): the checked calls refuse a set of the shipped model's orbits, and
    // the unchecked `protection` keeps its old signature. The empty set names no orbit, so every kernel accepts it.
    let (set, own_set) = (shipped.protected_set(&[3]).unwrap(), other.protected_set(&[3]).unwrap());
    let effect = other.effect(&own_recipe).unwrap();
    assert_eq!(other.checked_protection(&effect, &set).unwrap_err(), refusal);
    assert_eq!(other.strict_protection(&own_recipe, &set).unwrap_err(), refusal);
    assert_eq!(other.strict_protection(&recipe, &own_set).unwrap_err(), refusal);
    assert_eq!(other.derive(&own_state, &own_recipe, &set).unwrap_err(), mismatch);
    assert_eq!(other.derive_strict(&own_state, &own_recipe, &set).unwrap_err(), mismatch);
    assert_eq!(other.replay(&own_state, &set).err(), Some(refusal.clone()));
    assert_eq!(other.replay_under_ceiling(&own_state, &set).err(), Some(refusal.clone()));
    assert_eq!(other.checked_protection(&effect, &own_set).unwrap(), other.protection(&effect, &own_set));
    assert_eq!(other.checked_protection(&effect, &shipped.protected_set(&[]).unwrap()).unwrap().protected, []);
    assert!(other.derive(&own_state, &own_recipe, &own_set).is_ok());

    // KN1.4 for protected pieces: both piece calls refuse another model's base, recipe or set, accept the empty set of
    // any kernel, and answer for their own values as the shipped kernel answers for its.
    let (pieces, own_pieces) =
        (shipped.protected_pieces(&[0, 433]).unwrap(), other.protected_pieces(&[0, 433]).unwrap());
    assert_eq!(other.piece_protection(&state, &effect, &own_pieces).unwrap_err(), refusal);
    assert_eq!(other.piece_protection(&own_state, &effect, &pieces).unwrap_err(), refusal);
    assert_eq!(other.strict_piece_protection(&state, &own_recipe, &own_pieces).unwrap_err(), refusal);
    assert_eq!(other.strict_piece_protection(&own_state, &recipe, &own_pieces).unwrap_err(), refusal);
    assert_eq!(other.strict_piece_protection(&own_state, &own_recipe, &pieces).unwrap_err(), refusal);
    let empty = shipped.protected_pieces(&[]).unwrap();
    assert_eq!(other.piece_protection(&own_state, &effect, &empty).unwrap().rows, []);
    assert_eq!(other.strict_piece_protection(&own_state, &own_recipe, &empty).unwrap().rows, []);
    let shipped_effect = shipped.effect(&recipe).unwrap();
    assert_eq!(
        other.piece_protection(&own_state, &effect, &own_pieces).unwrap(),
        shipped.piece_protection(&state, &shipped_effect, &pieces).unwrap()
    );
    assert_eq!(
        other.strict_piece_protection(&own_state, &own_recipe, &own_pieces).unwrap(),
        shipped.strict_piece_protection(&state, &recipe, &pieces).unwrap()
    );
}

/// The orbit of every slot of the shipped model, read from `model.npz`.
fn slot_orbits() -> Vec<i32> {
    let bytes = fs::read(assets().join("model.npz")).unwrap();
    let zip = Zip::parse(&bytes).unwrap();
    let read = |name: &str| archive::read_npy(&zip.read(name, 1 << 30).unwrap()).unwrap().values;
    let (slot_piece, orbit) = (read("slot_piece.npy"), read("orbit_id.npy"));
    slot_piece.iter().map(|&piece| orbit[piece as usize]).collect()
}

/// CT-LG2-12 (part: the kernel half on 600-cell-Full, that literal prefix protection does not follow the key; not run:
/// R-11's operations, the jumbling fixtures and the peak height) and CT-PS1-12 (part: the Strict clause; the rest in
/// `tests/kernel.rs`). On the shipped model every generator moves every orbit, so this test changes a copy:
/// generator 1200 lists its orbit-0 pairs as fixed (each slot to itself), which keeps it a permutation of its listed
/// slots and lets it leave orbit 0 alone. With orbit 0 protected, `[1200]` and `[1200, 1, -1]` have the same net map
/// and the same freely reduced key, and Net accepts both; Strict accepts the first and refuses the second at prefix 2.
/// The first refused prefix of a 150,001-letter word is 150,001 in every chunking into word steps.
#[test]
fn strict_protection_judges_each_literal_prefix() {
    let orbits = slot_orbits();
    let mut members = primitive_members().to_vec();
    let index = |name: &str| members.iter().position(|(member, _)| member == name).unwrap();
    let (o, s, d) = (index("offsets.npy"), index("src.npy"), index("dst.npy"));
    let mut offsets = members[o].1.clone();
    let offsets = Values::of(&mut offsets);
    let mut source = members[s].1.clone();
    let source = Values::of(&mut source);
    let mut destination = Values::of(&mut members[d].1);
    let mut fixed = 0;
    for i in offsets.get(1199) as usize..offsets.get(1200) as usize {
        let slot = source.get(i);
        if orbits[slot as usize] == 0 && destination.get(i) != slot {
            destination.set(i, slot);
            fixed += 1;
        }
    }
    assert!(fixed > 0, "generator 1200 moves orbit 0 in the shipped model");
    let (folder, _) = model_folder("strict-prefix", &[("primitives.npz", stored_zip(&members))]);
    let kernel = Kernel::load(&folder).unwrap();
    let _ = fs::remove_dir_all(&folder);

    let (base, protected) = (kernel.solved(), kernel.protected_set(&[0]).unwrap());
    let (short, long) = (kernel.word(&[1200]).unwrap(), kernel.word(&[1200, 1, -1]).unwrap());
    assert_eq!(kernel.effect(&short).unwrap().net_sha256, kernel.effect(&long).unwrap().net_sha256);
    let state = |derivation: Derivation| match derivation {
        Derivation::Derived(derived) => derived,
        Derivation::Refused(refused) => panic!("refused at {:?}", refused.first_refused_prefix),
    };
    let net = state(kernel.derive(&base, &long, &protected).unwrap());
    assert_eq!(net.state.digest(), state(kernel.derive(&base, &short, &protected).unwrap()).state.digest());
    let strict = state(kernel.derive_strict(&base, &short, &protected).unwrap());
    assert_eq!((strict.state.digest(), strict.protection.verdict), (net.state.digest(), Verdict::Clear));
    let Derivation::Refused(refused) = kernel.derive_strict(&base, &long, &protected).unwrap() else {
        panic!("Strict accepted [1200, 1, -1]");
    };
    assert_eq!(refused.first_refused_prefix, Some(2));
    assert_eq!(refused.effect, net.effect);
    // The certificate is a move of letter 1 in orbit 0, and the conflict is letter 1's orbit-0 row.
    let one = kernel.effect(&kernel.word(&[1]).unwrap()).unwrap();
    let [certificate] = refused.certificates[..] else { panic!("{:?}", refused.certificates) };
    assert_eq!((certificate.orbit, orbits[certificate.source as usize]), (0, 0));
    let at = one.source().binary_search(&certificate.source).unwrap();
    assert_eq!(one.destination()[at], certificate.destination);
    assert_eq!(certificate.source, *one.source().iter().find(|&&slot| orbits[slot as usize] == 0).unwrap());
    let rows: Vec<_> = one.support.iter().filter(|row| row.orbit == 0).cloned().collect();
    assert_eq!(refused.protection.conflicts, rows);

    // Chunking a long word into word steps leaves the first refused prefix where it is.
    let mut letters = vec![1200i64; 150_000];
    letters.push(1);
    let chunked = |sizes: &[usize]| {
        let mut at = 0;
        let steps = sizes.iter().map(|&size| {
            let moves = letters[at..at + size].iter().map(|&m| Value::Int(m)).collect();
            at += size;
            json::object([("kind", Value::String("word".into())), ("moves", Value::Array(moves))])
        });
        kernel.recipe(&Value::Array(steps.collect())).unwrap()
    };
    for sizes in [&[100_000, 50_001][..], &[50_000, 100_000, 1], &[1, 99_999, 50_000, 1]] {
        let verdict = kernel.strict_protection(&chunked(sizes), &protected).unwrap();
        assert_eq!((verdict.verdict, verdict.first_refused_prefix), (Verdict::Moves, Some(150_001)), "{sizes:?}");
    }
    let clear = kernel.strict_protection(&chunked(&[100_000, 50_000]), &protected).unwrap();
    assert_eq!((clear.verdict, clear.first_refused_prefix), (Verdict::Clear, None));
}

/// Lead L10 (M4): the shipped model with one destination of the first primitive repeated is refused.
#[test]
fn a_primitive_that_is_not_a_permutation_of_its_support_is_refused() {
    let text = changed_primitives("not-a-permutation", "dst.npy", |dst| {
        let first = dst.get(0);
        assert_ne!(dst.get(1), first);
        dst.set(1, first);
    });
    assert_eq!(text, "A primitive is not a permutation of its support");
}

/// Lead L10 (M4): the shipped model with the first two sources of the first primitive swapped is refused.
#[test]
fn primitive_sources_out_of_order_are_refused() {
    let text = changed_primitives("sources-out-of-order", "src.npy", |src| {
        let (a, b) = (src.get(0), src.get(1));
        assert!(a < b);
        src.set(0, b);
        src.set(1, a);
    });
    assert_eq!(text, "Primitive sources are not strictly increasing");
}

/// Lead L10 (M4): the shipped model whose last primitive offset stops one short of the arrays' end is refused.
#[test]
fn primitive_offsets_that_disagree_with_the_arrays_are_refused() {
    let text = changed_primitives("short-offsets", "offsets.npy", |offsets| {
        let last = offsets.len() - 1;
        assert_eq!(last, 1_200);
        offsets.set(last, offsets.get(last) - 1);
    });
    assert_eq!(text, "Primitive offsets disagree with the arrays");
}

/// Lead L10 (M4): the shipped model whose first execution tree names a node as its own parent is refused, so a walk
/// to the root always ends.
#[test]
fn an_execution_tree_with_a_parent_at_its_child_is_refused() {
    let gz = fs::read(assets().join("execution_trees.json.gz")).unwrap();
    let mut trees = json::parse(&archive::gunzip(&gz, 1 << 30).unwrap()).unwrap();
    let Value::Array(rows) = &mut trees else { panic!("the trees are not a list") };
    let Value::Object(tree) = &mut rows[0] else { panic!("a tree is not an object") };
    let Some(Value::Array(parent)) = tree.get_mut("parent") else { panic!("a tree has no parent list") };
    assert_eq!(parent[1], Value::Int(0));
    parent[1] = Value::Int(1);
    let text = refused_copy("tree-parent", &[("execution_trees.json.gz", stored_gzip(trees.canonical().as_bytes()))]);
    assert_eq!(text, "An execution tree is malformed");
}

/// `verify_assets`, through which the jumbling half loads (M5, R3 of its critical-path read): on the shipped folder it
/// gives the shipped model ID, the manifest's digest, every listed digest and the bytes of exactly the files asked
/// for; a file asked for that is not listed is refused.
#[test]
fn verify_assets_gives_the_checked_files() {
    let (files, id) = shipped();
    let verified = c600_kernel::verify_assets(&assets(), &["puzzle_definition.txt"]).unwrap();
    assert_eq!(verified.model_id, id);
    assert_eq!(verified.manifest_sha256, sha256_hex(&fs::read(assets().join("manifest.json")).unwrap()));
    assert_eq!(verified.digests, files);
    assert_eq!(verified.files.keys().map(String::as_str).collect::<Vec<_>>(), vec!["puzzle_definition.txt"]);
    assert_eq!(verified.files["puzzle_definition.txt"], fs::read(assets().join("puzzle_definition.txt")).unwrap());
    match c600_kernel::verify_assets(&assets(), &["absent.bin"]) {
        Ok(_) => panic!("a file that is not listed was given"),
        Err(LoadError(text)) => assert_eq!(text, "manifest.json does not list absent.bin"),
    }
}
