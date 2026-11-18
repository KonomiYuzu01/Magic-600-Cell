//! A language evidence builder and its build cache (DET.44 to DET.46; LP2 draft until the IR freeze): CT-DET-21 and
//! CT-DET-22 over `c600_build`, with each test's source tree and grant under Cargo's test scratch directory.

#[path = "../../c600-run/tests/conformance/mod.rs"]
mod conformance;

use std::fs;
use std::path::{Path, PathBuf};

use c600_build::{self as builder, code, Built, Grant, Inputs, Source, Spec};
use c600_digest::sha256_hex;
use c600_ir::Ty;
use c600_run::encode::encode;
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{DraftTariffs, Tariffs};
use c600_run::value::Value;
use c600_syntax::{lower, parse, Lowered};
use conformance::{lowered, m, m_requiring, WIDE};

const DECLARATIONS: &str = "export fn table(n: Nat) -> Nat effects [file, random] {\n    \
    let t = file_read<Text>(\"rows\");\n    let a = file_write(\"report.txt\", t);\n    \
    let b = file_write(\"draws/first\", random_draw());\n    return exact_add(n, 1);\n}\n\n\
    export fn now() -> Display<Nat> effects [clock] {\n    return read_clock();\n}\n\n\
    export fn other() -> Nat effects [file] {\n    return file_read<Nat>(\"elsewhere\");\n}\n\n\
    export fn escape() -> Unit effects [file] {\n    return file_write(\"../up\", 1);\n}\n";

fn source() -> String {
    m_requiring("file, random, clock", DECLARATIONS)
}

/// A fresh directory for one test: a source tree holding the module, a directory inside it, and three outside.
fn dirs(test: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("c600-builder").join(test);
    let _ = fs::remove_dir_all(&root);
    for dir in ["src/inner", "out", "elsewhere", "scratch"] {
        fs::create_dir_all(root.join(dir)).expect("a test directory");
    }
    fs::write(root.join("src/m.6cs"), source()).expect("the module's source");
    root
}

/// Every file under `dir`, by its path relative to `dir` with `/` separators, in order.
fn listing(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).expect("a directory") {
            let entry = entry.expect("an entry");
            let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
            if entry.path().is_dir() {
                walk(&entry.path(), &format!("{name}/"), out);
            } else {
                out.push(name);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, "", &mut out);
    out.sort();
    out
}

/// The host's captured input `rows`, which reads `changed` from its `at`-th read on. It counts its reads.
struct Rows {
    text: &'static str,
    reads: u32,
    change: Option<(u32, &'static str)>,
}

impl Rows {
    fn new(text: &'static str) -> Rows {
        Rows { text, reads: 0, change: None }
    }
}

impl Inputs for Rows {
    fn read(&mut self, name: &str) -> Option<(Ty, Value)> {
        self.reads += 1;
        let text = match self.change {
            Some((at, changed)) if self.reads >= at => changed,
            _ => self.text,
        };
        (name == "rows").then(|| (Ty::Text, Value::text(text)))
    }
}

fn spec(lowered: &Lowered, function: &str, args: Vec<Value>) -> Spec {
    Spec {
        modules: lowered.sources.iter().map(|s| (s.module.clone(), s.digest.clone())).collect(),
        module: "m".into(),
        function: function.into(),
        args,
        seed: Some(([7; 32], vec!["build".into()])),
        inputs: vec!["rows".into()],
        limits: WIDE,
        tariffs: Tariffs::TEST,
        draft: DraftTariffs::TEST,
    }
}

fn canonical(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    encode(value, &mut out).expect("a canonical value");
    out
}

/// CT-DET-21 (whole: a builder cannot consult a clock, add a computation thread or import ambient modules, and its
/// output and scratch grant points outside the source tree. The clock read stops the build with
/// `LG3:capability.not_granted`, as does an input that was not captured, and nothing is stored or published; the
/// interpreter and the builder start no thread, so the build runs on the caller's thread alone; the program's modules
/// must be exactly the sealed closure, and an import resolves only among the given sources; a grant that is the source
/// tree, lies inside it or holds it is refused with `LG3:capability.grant_mismatch`, as is a write outside the grant)
#[test]
fn ct_det_21_a_builder_has_no_clock_thread_or_ambient_module_and_writes_outside_the_source_tree() {
    let root = dirs("ct-det-21");
    let (src, out, scratch) = (root.join("src"), root.join("out"), root.join("scratch"));
    for (output, scratch) in [
        (src.clone(), scratch.clone()),
        (src.join("inner"), scratch.clone()),
        (root.clone(), scratch.clone()),
        (out.clone(), src.clone()),
        (out.clone(), src.join("inner")),
        (out.clone(), root.clone()),
        (root.join("missing"), scratch.clone()),
    ] {
        let refused = Grant::new(&src, &output, &scratch).expect_err("a grant in the source tree");
        assert_eq!(refused.code, code::GRANT_MISMATCH, "{output:?} {scratch:?}");
    }
    let grant = Grant::new(&src, &out, &scratch).expect("a grant outside the source tree");
    assert_eq!(grant.output(), fs::canonicalize(&out).expect("the output directory"));

    let lowered = lowered(&[&source()]);
    let refused = |function: &str, spec_of: &dyn Fn(Spec) -> Spec| {
        let spec = spec_of(spec(&lowered, function, Vec::new()));
        builder::build(&lowered.program, &spec, &mut Rows::new("a,b"), &grant).expect_err("a refused build").code
    };
    let same = |s: Spec| s;
    assert_eq!(refused("now", &same), NOT_GRANTED);
    assert_eq!(refused("other", &same), NOT_GRANTED);
    assert_eq!(refused("escape", &same), code::GRANT_MISMATCH);
    // A module outside the sealed closure, or a sealed module the program lacks, refuses the build before it runs.
    let wider = |mut s: Spec| {
        s.modules.push(("ambient".into(), sha256_hex(b"ambient")));
        s
    };
    let narrower = |mut s: Spec| {
        s.modules.clear();
        s
    };
    assert_eq!(refused("now", &wider), code::GRANT_MISMATCH);
    assert_eq!(refused("now", &narrower), code::GRANT_MISMATCH);
    assert!(listing(&out).is_empty() && listing(&scratch).is_empty(), "nothing stored or published");
    // An import names a module by its source's digest among the given sources: there is no module search path.
    let ambient = m("export fn f() -> Nat effects [] {\n    return 1;\n}\n");
    let importing = format!(
        "edition \"6cs-draft-1\";\nmodule n requires [];\nimport a \"{}\";\n\n\
         export fn g() -> Nat effects [] {{\n    return a.f();\n}}\n",
        sha256_hex(ambient.as_bytes())
    );
    let lowers = |sources: &[&str]| lower(&sources.iter().map(|s| parse(s.as_bytes())).collect::<Vec<_>>()).is_ok();
    assert!(lowers(&[&ambient, &importing]) && !lowers(&[&importing]));

    // The interpreter and the builder start no thread: the run takes the caller's thread.
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    for sources in [crate_dir.join("src"), crate_dir.join("..").join("c600-run").join("src")] {
        let files = listing(&sources);
        assert!(!files.is_empty(), "the sources under {} were found", sources.display());
        for file in files {
            let text = fs::read_to_string(sources.join(&file)).expect("a source file");
            for spawn in ["thread::spawn", "thread::Builder", "thread::scope", "std::thread"] {
                assert!(!text.contains(spawn), "{file} names {spawn}");
            }
        }
    }

    // A build in the grant: its files stay under the output directory, and nothing enters the source tree.
    let built =
        builder::build(&lowered.program, &spec(&lowered, "table", vec![Value::int(4)]), &mut Rows::new("a,b"), &grant)
            .expect("a build");
    assert_eq!(built.value, canonical(&Value::int(5)));
    assert_eq!(listing(&out), ["draws/first", "report.txt"]);
    assert_eq!(fs::read(out.join("report.txt")).expect("the report"), canonical(&Value::text("a,b")));
    assert_eq!(listing(&src), ["m.6cs"]);
}

/// The blob of `sha256` in the cache under `scratch`.
fn blob(scratch: &Path, sha256: &str) -> PathBuf {
    scratch.join("build-cache").join("blobs").join(sha256)
}

/// CT-DET-22 (whole: tampered cache contents, a tampered or unreadable entry and a captured input changed before
/// publication; reuse fails closed with `LG4:input.cache_entry_mismatch` as a host-envelope diagnostic, the rejected
/// entry is a miss, and the verified rebuild has the original run's charges; an input changed during construction
/// leaves nothing stored or published; a verified entry is reused, without a run, with the recorded charges, and the
/// output location enters no key)
#[test]
fn ct_det_22_a_build_cache_fails_closed_and_a_rebuild_keeps_its_charges() {
    let root = dirs("ct-det-22");
    let (src, out, scratch) = (root.join("src"), root.join("out"), root.join("scratch"));
    let grant = Grant::new(&src, &out, &scratch).expect("a grant");
    let lowered = lowered(&[&source()]);
    let table = spec(&lowered, "table", vec![Value::int(4)]);
    let run = |rows: &mut Rows, grant: &Grant| builder::build(&lowered.program, &table, rows, grant);

    // A construction captures its input twice: before the key is sealed and after the run.
    let mut rows = Rows::new("a,b");
    let first = run(&mut rows, &grant).expect("a build");
    assert_eq!((first.source, rows.reads, first.diagnostics.len()), (Source::Built, 2, 0));
    assert!(first.used.steps > 0 && first.used.output > 0);
    let report = canonical(&Value::text("a,b"));
    let report_sha = sha256_hex(&report);
    assert_eq!(first.files.iter().map(|f| f.0.as_str()).collect::<Vec<_>>(), ["draws/first", "report.txt"]);
    assert_eq!(first.files[1].1, report_sha);

    // A verified entry is reused without a run (one capture), with the recorded charges, also into another output.
    let mut rows = Rows::new("a,b");
    let elsewhere = Grant::new(&src, &root.join("elsewhere"), &scratch).expect("another output");
    let reused = run(&mut rows, &elsewhere).expect("a reuse");
    assert_eq!(rows.reads, 1);
    assert_eq!(reused, Built { source: Source::Reused, ..first.clone() });
    assert_eq!(fs::read(root.join("elsewhere/report.txt")).expect("the report"), report);

    // Tampered contents: the stored report's bytes change. Reuse fails closed, the entry is a miss, and the rebuild
    // has the original charges and publishes the verified bytes.
    fs::write(blob(&scratch, &report_sha), b"tampered").expect("a tampered blob");
    fs::write(out.join("report.txt"), b"stale").expect("a stale output");
    let rebuilt = run(&mut Rows::new("a,b"), &grant).expect("a rebuild");
    assert_eq!(rebuilt.diagnostics, [code::CACHE_ENTRY_MISMATCH]);
    assert_eq!(rebuilt, Built { diagnostics: vec![code::CACHE_ENTRY_MISMATCH], ..first.clone() });
    assert_eq!(fs::read(out.join("report.txt")).expect("the report"), report);
    assert_eq!(run(&mut Rows::new("a,b"), &grant).expect("a reuse").source, Source::Reused);

    // A tampered entry: its recorded input digest, its recorded charges or its bytes change. Each is a miss.
    let entry = scratch.join("build-cache").join(format!("{}.json", first.key));
    let input = format!("\"rows\":{{\"sha256\":\"{report_sha}\"");
    let recorded = fs::read_to_string(&entry).expect("the entry");
    assert!(recorded.contains(&input));
    let steps = format!("\"steps\":\"{}\"", first.used.steps);
    for tampered in [
        recorded.replacen(&input, &format!("\"rows\":{{\"sha256\":\"{}\"", sha256_hex(b"other")), 1),
        recorded.replacen(&steps, "\"steps\":\"1\"", 1),
        recorded.replacen('{', "{ ", 1),
        String::from("not an entry"),
    ] {
        fs::write(&entry, tampered).expect("a tampered entry");
        let rebuilt = run(&mut Rows::new("a,b"), &grant).expect("a rebuild");
        assert_eq!(rebuilt, Built { diagnostics: vec![code::CACHE_ENTRY_MISMATCH], ..first.clone() });
    }

    // Another captured input is another key: a plain miss, built with no diagnostic.
    let other = run(&mut Rows::new("c"), &grant).expect("another build");
    assert_eq!((other.source, other.diagnostics.len()), (Source::Built, 0));
    assert_ne!(other.key, first.key);

    // A captured input that changes after construction, before publication: nothing is stored or published.
    let fresh = dirs("ct-det-22-changed");
    let grant = Grant::new(&fresh.join("src"), &fresh.join("out"), &fresh.join("scratch")).expect("a grant");
    let mut rows = Rows { text: "a,b", reads: 0, change: Some((2, "a,b,c")) };
    let refused = run(&mut rows, &grant).expect_err("a changed input");
    assert_eq!((refused.code, rows.reads), (code::CACHE_ENTRY_MISMATCH, 2));
    assert!(listing(&fresh.join("out")).is_empty() && listing(&fresh.join("scratch")).is_empty());
    // Captured again, the input builds with the original charges of its own key.
    assert_eq!(run(&mut Rows::new("a,b"), &grant).expect("a build"), first);
}
