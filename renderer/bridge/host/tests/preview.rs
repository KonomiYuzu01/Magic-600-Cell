//! The D02 preview against real journal commits, with device-free capture of the core uploads.
#[path = "../../tests/common/mod.rs"]
mod common;

use std::cell::RefCell;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use c600_display::jumble;
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;
use c600_render::FakeCore;
use c600_shell_host::preview::*;
use c600_shell_host::turns::{Head, Puzzle, Turns};
use c600_shell_host::*;
use common::{functions, Shim};

#[derive(Debug, PartialEq)]
struct Uploaded {
    revision: u64,
    index: Vec<i32>,
    rows: Vec<f32>,
    lattice: Vec<i32>,
}

thread_local! {
    static POSES: RefCell<Option<Uploaded>> = const { RefCell::new(None) };
    static PHASE: RefCell<f32> = const { RefCell::new(0.0) };
}

unsafe extern "C" fn poses(
    _: *mut c_void,
    revision: u64,
    index: *const i32,
    pieces: u32,
    rows: *const f32,
    lattice: *const i32,
    count: u32,
    _: *const u8,
) -> i32 {
    POSES.with_borrow_mut(|set| {
        *set = if index.is_null() {
            None
        } else {
            // SAFETY: the host calls with full pose arrays, and the inputs live through this copy.
            Some(unsafe {
                Uploaded {
                    revision,
                    index: std::slice::from_raw_parts(index, pieces as usize).to_vec(),
                    rows: std::slice::from_raw_parts(rows, count as usize * 16).to_vec(),
                    lattice: std::slice::from_raw_parts(lattice, count as usize).to_vec(),
                }
            })
        };
    });
    0
}

unsafe extern "C" fn transform(
    _: *mut c_void,
    _: u64,
    _: *const f32,
    _: *const f32,
    _: f32,
    _: *const u8,
    _: u32,
    phase: f32,
) -> i32 {
    PHASE.with_borrow_mut(|out| *out = phase);
    0
}

struct Run {
    host: *mut ShellHost,
    shim: Box<Shim>,
}

impl Run {
    fn new(assets: &Path, menu: &Path, folder: &Path, puzzle: u32) -> (Self, u64) {
        let mut shim = Box::new(Shim { fake: FakeCore::on_surface(common::support::SIZE), pad: 0 });
        let mut host = null_mut();
        let assets = assets.to_str().unwrap();
        let menu = menu.to_str().unwrap();
        let folder = folder.to_str().unwrap();
        let mut revision = 0;
        // SAFETY: the shim remains alive until after the host is destroyed. All bytes and outputs are live.
        unsafe {
            let code =
                m6h_create(&functions(), (&mut *shim as *mut Shim).cast(), assets.as_ptr(), assets.len(), &mut host);
            assert_eq!(code, M6H_OK, "{}", read_text(host, m6h_last_error));
            let run = Self { host, shim };
            run.ok(m6h_preview_bind(host, Some(poses), Some(transform)));
            run.ok(m6h_open_session(
                host,
                puzzle,
                folder.as_ptr(),
                folder.len(),
                menu.as_ptr(),
                menu.len(),
                null(),
                0,
                &mut revision,
            ));
            (run, revision)
        }
    }

    fn ok(&self, code: i32) {
        assert_eq!(code, M6H_OK, "{}", read_text(self.host, m6h_last_error));
    }

    fn answer(&self) -> Value {
        json::parse(read_text(self.host, m6h_answer).as_bytes()).unwrap()
    }

    fn head(&self) -> Value {
        // SAFETY: the handle is live and exclusively used.
        self.ok(unsafe { m6h_state_digest(self.host) });
        self.answer()
    }
}

impl Drop for Run {
    fn drop(&mut self) {
        // SAFETY: the host and its core shim are both live through destroy.
        unsafe { m6h_destroy(self.host) };
        let _ = &self.shim;
    }
}

fn read_text(
    host: *mut ShellHost,
    read: unsafe extern "C" fn(*mut ShellHost, *mut u8, u32, *mut u32) -> i32,
) -> String {
    let mut size = 0;
    // SAFETY: ask for the length, then supply exactly that many writable bytes.
    unsafe {
        read(host, null_mut(), 0, &mut size);
        let mut buffer = vec![0; size as usize];
        assert_eq!(read(host, buffer.as_mut_ptr(), size, &mut size), M6H_OK);
        assert_eq!(buffer.pop(), Some(0));
        String::from_utf8(buffer).unwrap()
    }
}

fn fresh() -> PathBuf {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("d02-preview-{}-{unique}", std::process::id()))
}

#[test]
fn three_letters_preview_the_same_prefixes_as_the_menu_row() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let assets = repo.join("assets");
    let menu = repo.join("research/jumbling/fixtures/wj-I_a.json");
    let folder = fresh();
    let (run, revision) = Run::new(&assets, &menu, &folder.join("preview"), M6H_PUZZLE_JUMBLE_ON);
    let head = run.head();
    let letters = [[0, 1], [0, 2], [0, 3]];
    // SAFETY: every handle is live, the arrays and outputs match their FFI lengths, and the calls are serial.
    unsafe {
        run.ok(m6h_preview_build(run.host, letters.as_ptr(), letters.len() as u32));
        let preview = run.answer();
        let id = preview.get("id").and_then(Value::as_i64).unwrap() as u64;
        assert_eq!(preview.get("before"), head.get("digest"));
        assert_eq!(run.head(), head, "building the preview changed the head's digest or pieces");
        run.ok(m6h_preview_report(run.host));
        assert_eq!(run.answer().get("head_revision"), Some(&Value::Int(revision as i64)));
        let steps = preview.get("steps").and_then(Value::as_array).unwrap();

        let kernel = Arc::new(Kernel::load(&assets).expect("the shipped model"));
        let mut control = Turns::open(kernel, &assets, &folder.join("control"), &menu, Puzzle::JumbleOn).unwrap();
        for (n, ([cap, item], step)) in letters.into_iter().zip(steps).enumerate() {
            control.twist_letter(cap, u64::from(item)).unwrap();
            let Head::Jumbled(exact, state) = control.head().unwrap() else { panic!("Jumble on") };
            assert_eq!(step.get("digest").and_then(Value::as_str), Some(state.digest(&exact).as_str()));
            let posed = jumble::posed(&exact, &state).unwrap();
            run.ok(m6h_preview_scrub(run.host, id, (n + 1) as u32, 0.0));
            POSES.with_borrow(|set| {
                assert_eq!(
                    set.as_ref(),
                    Some(&Uploaded {
                        revision,
                        index: posed.pose_index.as_i32(),
                        rows: posed.poses.as_f32(),
                        lattice: posed.pose_lattice.as_i32(),
                    })
                )
            });
        }
        assert_eq!(run.head(), head, "scrubbing committed a twist");
        assert_ne!(preview.get("after"), head.get("digest"));
        run.ok(m6h_preview_scrub(run.host, id, 0, 0.5));
        PHASE.with_borrow(|phase| assert_eq!(*phase, 0.5));
        run.ok(m6h_preview_clear(run.host));
        POSES.with_borrow(|set| assert!(set.is_none()));
        assert_ne!(m6h_preview_scrub(run.host, id, 0, 0.0), M6H_OK);
        assert_eq!(run.head(), head);

        run.ok(m6h_preview_build(run.host, letters.as_ptr(), letters.len() as u32));
        let second = run.answer().get("id").and_then(Value::as_i64).unwrap() as u64;
        assert!(second > id);
        assert_ne!(m6h_preview_args(run.host, id, 0), M6H_OK, "opening a new preview left the old one eligible");
        let row = b"jumble.twist.menu";
        for (step, expected) in control.committed().iter().enumerate() {
            run.ok(m6h_preview_args(run.host, second, step as u32));
            assert_eq!(&run.answer(), expected, "the preview's arguments differ from Turns::twist_letter");
            let args = expected.canonical();
            let mut job = 0;
            run.ok(m6h_invoke(run.host, row.as_ptr(), row.len(), args.as_ptr(), args.len(), &mut job));
            let receipt = run.answer();
            assert_eq!(
                receipt.get("status").and_then(|s| s.get("name")).and_then(Value::as_str),
                Some("committed"),
                "{receipt:?}"
            );
        }
        assert_eq!(run.head().get("digest"), preview.get("after"));
        run.ok(m6h_preview_report(run.host));
        assert_eq!(run.answer().get("head_revision"), Some(&Value::Int(revision as i64 + 3)));
        run.ok(m6h_preview_clear(run.host));

        let (off, _) = Run::new(&assets, &menu, &folder.join("off"), M6H_PUZZLE_JUMBLE_OFF);
        let off_head = off.head();
        assert_eq!(m6h_preview_build(off.host, letters.as_ptr(), letters.len() as u32), M6H_E_SESSION);
        assert!(read_text(off.host, m6h_last_error).contains("needs Jumble on"));
        assert_eq!(off.head(), off_head);
    }
}
