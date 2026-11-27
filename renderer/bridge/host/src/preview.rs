//! ADR-D01.10: a read-only Jumble replay and its revision-bound product drawing.
use std::ffi::{c_char, c_void};
use std::ptr::null;

use c600_display::jumble::{self, SweptPlane};
use c600_display::plan::PosedInput;
use c600_jumble::rotation;
use c600_jumble::state::Status;
use c600_render::abi::{PANE_PREVIEW, PANE_PREVIEW_BEFORE, PIECES, RING_SLOTS};

use crate::turns::{binding, Head, Turns};
use crate::{
    json, with_host, Failure, Pane, ShellHost, Value, LOCAL, M6H_E_CORE, M6H_E_INVALID_ARGUMENT, M6H_E_SESSION,
};

type PosesSet = unsafe extern "C" fn(*mut c_void, u64, *const i32, u32, *const f32, *const i32, u32, *const u8) -> i32;
type TransformSet = unsafe extern "C" fn(*mut c_void, u64, *const f32, *const f32, f32, *const u8, u32, f32) -> i32;
type LastError = unsafe extern "C" fn(*mut c_void, *mut c_char, u32) -> i32;

type Letter = (u32, u64);

struct Step {
    letter: Letter,
    digest: String,
    moved: usize,
    moving: Vec<u8>,
    sweep: Option<SweptPlane>,
}

struct Preview {
    id: u64,
    revision: u64,
    before: String,
    protected: Vec<usize>,
    steps: Vec<Step>,
    prefixes: Vec<PosedInput>,
    moved: Vec<u8>,
    orbits: Vec<Value>,
    net: usize,
    shown_step: usize,
    phase: f32,
}

fn invalid(reason: &str) -> Failure {
    (M6H_E_INVALID_ARGUMENT, reason.to_owned())
}

fn session(reason: String) -> Failure {
    (M6H_E_SESSION, reason)
}

impl Preview {
    fn build(turns: &mut Turns, id: u64, revision: u64, letters: &[Letter]) -> Result<Self, String> {
        let Head::Jumbled(exact, mut state) = turns.head()? else {
            return Err("A repair preview needs Jumble on.".to_owned());
        };
        let (before, protected) = turns.preview_basis()?;
        let orbit_ids = &exact.model.orbit_id;
        let guard: Vec<_> = orbit_ids
            .iter()
            .enumerate()
            .filter(|(_, &orbit)| usize::try_from(orbit).is_ok_and(|orbit| protected.contains(&orbit)))
            .map(|(piece, _)| piece as u32)
            .collect();
        let original = state.clone();
        let posed = |state: &c600_jumble::state::State| {
            jumble::posed(&exact, state).map_err(|error| format!("Preview poses: {error:?}"))
        };
        let mut prefixes = vec![posed(&state)?];
        let mut steps = Vec::with_capacity(letters.len());
        let mut moved = vec![0u8; PIECES];
        for &(cap, item) in letters {
            let args = turns.twist_args(cap, item)?;
            let resolved = rotation::resolve(&exact, state.menu(), &binding(&args, "letter")?)
                .map_err(|error| format!("Twist {} does not resolve: {error:?}", steps.len() + 1))?;
            // The product commits one menu row per letter. Match each row's protection check at its prefix.
            let outcome = state
                .apply(&exact, &resolved.twist, None, &guard)
                .map_err(|error| format!("Twist {} failed: {error:?}", steps.len() + 1))?;
            if outcome.status != Status::Admissible {
                let reason = outcome.refusal.map_or_else(|| outcome.status.as_str().to_owned(), |(_, detail)| detail);
                return Err(format!("Twist {} refused. {reason}", steps.len() + 1));
            }
            let plane = match jumble::swept_plane(&resolved.twist) {
                Ok(None) => return Err("A repair witness cannot include an identity twist.".to_owned()),
                Ok(plane) => plane,
                Err(_) => None, // The exact state is available even if J03 supplies no display plane.
            };
            let mut moving = vec![0u8; PIECES];
            for &piece in &outcome.inside {
                moved[piece as usize] = 1;
                moving[piece as usize] = 1;
            }
            steps.push(Step {
                letter: (cap, item), digest: state.digest(&exact), moved: outcome.inside.len(), moving, sweep: plane,
            });
            prefixes.push(posed(&state)?);
        }
        let mut counts = std::collections::BTreeMap::<_, (i64, i64, i64)>::new();
        let mut net = 0;
        for (piece, &orbit) in orbit_ids.iter().enumerate() {
            let start = original.pose_ids()[piece] as usize;
            let end = state.pose_ids()[piece] as usize;
            let changed = original.poses()[start] != state.poses()[end];
            let (total, touched, changed_on_orbit) = counts.entry(orbit).or_default();
            *total += 1;
            *touched += i64::from(moved[piece]);
            *changed_on_orbit += i64::from(changed);
            net += usize::from(changed);
        }
        let orbits = counts
            .into_iter()
            .map(|(id, (total, moved, net))| {
                json::object([
                    ("orbit", Value::Int(i64::from(id))),
                    ("total", Value::Int(total)),
                    ("moved", Value::Int(moved)),
                    ("net", Value::Int(net)),
                ])
            })
            .collect();
        Ok(Self {
            id,
            revision,
            before,
            protected,
            steps,
            prefixes,
            moved,
            orbits,
            net,
            shown_step: letters.len(),
            phase: 0.0,
        })
    }

    fn digest(&self, step: usize) -> &str {
        if step == 0 {
            &self.before
        } else {
            &self.steps[step - 1].digest
        }
    }

    fn answer(&self) -> Value {
        let steps = self
            .steps
            .iter()
            .map(|step| {
                json::object([
                    ("cap", Value::Int(i64::from(step.letter.0))),
                    ("item", Value::Int(step.letter.1 as i64)),
                    ("digest", Value::String(step.digest.clone())),
                    ("moved", Value::Int(step.moved as i64)),
                    ("sweep", Value::Bool(step.sweep.is_some())),
                ])
            })
            .collect();
        json::object([
            ("id", Value::Int(self.id as i64)),
            ("revision", Value::Int(self.revision as i64)),
            ("before", Value::String(self.before.clone())),
            ("after", Value::String(self.digest(self.steps.len()).to_owned())),
            ("steps", Value::Array(steps)),
            ("moved", Value::Int(self.moved.iter().map(|&byte| i64::from(byte)).sum())),
            ("net", Value::Int(self.net as i64)),
            ("orbits", Value::Array(self.orbits.clone())),
            (
                "protection",
                Value::String(
                    if self.protected.is_empty() { "No protected orbits" } else { "Protected orbits unchanged" }
                        .to_owned(),
                ),
            ),
        ])
    }
}

#[derive(Clone, Copy)]
struct Core {
    poses: PosesSet,
    transform: TransformSet,
}

pub(crate) struct PreviewDrawing {
    context: *mut c_void,
    last_error: LastError,
    core: Option<Core>,
    pending: Option<Preview>,
    next: u64,
    frames: std::collections::VecDeque<Drawn>,
}

struct Drawn {
    sequence: u64,
    revision: u64,
    preview: Option<(u64, usize)>,
    flags: Vec<u32>,
}

impl PreviewDrawing {
    pub(crate) fn new(context: *mut c_void, last_error: LastError) -> Self {
        Self { context, last_error, core: None, pending: None, next: 1, frames: Default::default() }
    }

    pub(crate) fn current(&self, adopted: u64, assigned: u64) -> bool {
        adopted == assigned && self.pending.as_ref().is_some_and(|p| p.revision == adopted)
    }

    pub(crate) fn submitted(&mut self, sequence: u64, revision: u64, panes: &[Pane]) {
        if self.core.is_none() {
            return;
        }
        let preview = self
            .pending
            .as_ref()
            .filter(|p| p.revision == revision && panes.iter().any(|pane| pane.flags & PANE_PREVIEW != 0))
            .map(|p| (p.id, p.shown_step));
        let flags = panes.iter().map(|pane| pane.flags).collect();
        self.frames.push_back(Drawn { sequence, revision, preview, flags });
        if self.frames.len() > RING_SLOTS as usize + 1 {
            self.frames.pop_front();
        }
    }

    fn functions(&self) -> Result<Core, Failure> {
        self.core.ok_or_else(|| (M6H_E_CORE, "The renderer has no preview pose path.".to_owned()))
    }

    fn check(&self, code: i32) -> Result<(), Failure> {
        if code == 0 {
            return Ok(());
        }
        let mut bytes = [0; 1024];
        // SAFETY: the host owns this live context and the buffer is writable.
        unsafe { (self.last_error)(self.context, bytes.as_mut_ptr().cast(), bytes.len() as u32) };
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        Err((M6H_E_CORE, format!("Preview draw: {}", String::from_utf8_lossy(&bytes[..end]))))
    }

    fn upload(&self, preview: &Preview, step: usize) -> Result<(), Failure> {
        let pose = &preview.prefixes[step];
        let index = pose.pose_index.as_i32();
        let rows = pose.poses.as_f32();
        let lattice = pose.pose_lattice.as_i32();
        // SAFETY: the three arrays use the adoption path's lengths, and the mask is PIECES bytes.
        self.check(unsafe {
            (self.functions()?.poses)(
                self.context,
                preview.revision,
                index.as_ptr(),
                index.len() as u32,
                rows.as_ptr(),
                lattice.as_ptr(),
                lattice.len() as u32,
                preview.moved.as_ptr(),
            )
        })
    }

    fn transform(&self, preview: &Preview, step: usize, phase: f32) -> Result<(), Failure> {
        let sweep = preview.steps.get(step).and_then(|letter| letter.sweep.as_ref().map(|plane| (plane, &letter.moving)));
        let (u, v, angle, mask, count) = match sweep.filter(|_| phase != 0.0) {
            Some((plane, mask)) => {
                (plane.plane[0].as_ptr(), plane.plane[1].as_ptr(), plane.angle, mask.as_ptr(), mask.len() as u32)
            }
            None => (null(), null(), 0.0, null(), 0),
        };
        // SAFETY: pointers are null or the certified plane and the PIECES-byte mask stay readable through the call.
        self.check(unsafe {
            (self.functions()?.transform)(self.context, preview.revision, u, v, angle, mask, count, phase)
        })
    }

    fn clear(&mut self, adopted: u64) -> Result<(), Failure> {
        if self.pending.as_ref().is_some_and(|p| p.revision == adopted) {
            // SAFETY: the null arrays clear the set on this live context.
            self.check(unsafe {
                (self.functions()?.poses)(self.context, adopted, null(), 0, null(), null(), 0, null())
            })?;
        }
        self.pending = None;
        Ok(())
    }
}

/// Binds the two additive core ABI 10 calls without changing the original core function table.
///
/// # Safety
/// The handle is live and exclusively owned. Both functions, when present, are from its loaded core.
#[no_mangle]
pub unsafe extern "C" fn m6h_preview_bind(
    handle: *mut ShellHost,
    poses: Option<PosesSet>,
    transform: Option<TransformSet>,
) -> i32 {
    unsafe {
        with_host(handle, |shell| {
            let (Some(poses), Some(transform)) = (poses, transform) else {
                return Err(invalid("a preview needs both core functions"));
            };
            if shell.preview.core.is_some() {
                return Err(invalid("the preview core is already bound"));
            }
            shell.preview.core = Some(Core { poses, transform });
            Ok(())
        })
    }
}

/// Builds and draws a fresh preview at its end. The answer contains the adopted revision, the witness, its prefix
/// digests and complete effect. Refusals are available through m6h_last_error.
///
/// # Safety
/// The handle is live and exclusively owned. `letters` is null or holds `count` readable (cap, item) pairs.
#[no_mangle]
pub unsafe extern "C" fn m6h_preview_build(handle: *mut ShellHost, letters: *const [u32; 2], count: u32) -> i32 {
    unsafe {
        with_host(handle, |shell| {
            if letters.is_null() || count == 0 {
                return Err(invalid("a preview needs at least one letter"));
            }
            shell.preview.clear(shell.revision)?;
            if shell.turns()?.puzzle() != crate::Puzzle::JumbleOn {
                return Err(session("A repair preview needs Jumble on.".to_owned()));
            }
            if shell.assigned != shell.revision {
                return Err(session("Wait for the committed state to draw, then stage again.".to_owned()));
            }
            shell.preview.functions()?;
            let letters: Vec<_> = std::slice::from_raw_parts(letters, count as usize)
                .iter()
                .map(|&[cap, item]| (cap, u64::from(item)))
                .collect();
            let id = shell.preview.next;
            shell.preview.next += 1;
            let revision = shell.revision;
            let preview = Preview::build(shell.turns()?, id, revision, &letters).map_err(session)?;
            shell.preview.upload(&preview, letters.len())?;
            shell.answer = preview.answer().canonical();
            shell.preview.pending = Some(preview);
            Ok(())
        })
    }
}

/// Draws the prefix at `step`, optionally partway through the next step (phase within 0..1).
///
/// # Safety
/// The handle is live and exclusively owned.
#[no_mangle]
pub unsafe extern "C" fn m6h_preview_scrub(handle: *mut ShellHost, id: u64, step: u32, phase: f32) -> i32 {
    unsafe {
        with_host(handle, |shell| {
            let drawing = &mut shell.preview;
            if !drawing.current(shell.revision, shell.assigned) {
                return Err(invalid("the preview is absent or stale"));
            }
            let preview =
                drawing.pending.as_ref().filter(|p| p.id == id).ok_or_else(|| invalid("the preview ID has changed"))?;
            let step = step as usize;
            if !(0.0..=1.0).contains(&phase)
                || step > preview.steps.len()
                || (step == preview.steps.len() && phase != 0.0)
            {
                return Err(invalid("scrub needs a valid prefix and a phase of its next twist"));
            }
            let changed = step != preview.shown_step;
            if changed {
                drawing.upload(preview, step)?;
            }
            if phase != 0.0 || (!changed && preview.phase != 0.0) {
                drawing.transform(preview, step, phase)?;
            }
            let preview = drawing.pending.as_mut().expect("the preview being scrubbed");
            preview.shown_step = step;
            preview.phase = phase;
            Ok(())
        })
    }
}

/// Discards the preview. Its adopted revision, journal and commit state are untouched.
///
/// # Safety
/// The handle is live and exclusively owned.
#[no_mangle]
pub unsafe extern "C" fn m6h_preview_clear(handle: *mut ShellHost) -> i32 {
    unsafe { with_host(handle, |shell| shell.preview.clear(shell.revision)) }
}

/// Answers the existing menu row's arguments for the next letter, after rechecking its exact prefix and protection.
///
/// # Safety
/// The handle is live and exclusively owned.
#[no_mangle]
pub unsafe extern "C" fn m6h_preview_args(handle: *mut ShellHost, id: u64, step: u32) -> i32 {
    unsafe {
        with_host(handle, |shell| {
            let index = step as usize;
            let preview = shell
                .preview
                .pending
                .as_ref()
                .filter(|p| p.id == id)
                .ok_or_else(|| invalid("the preview ID has changed"))?;
            let (cap, item) = preview.steps.get(index).ok_or_else(|| invalid("the preview has no such twist"))?.letter;
            if shell.assigned != preview.revision + u64::from(step) {
                return Err(invalid("The committed revision changed. Stage again."));
            }
            let expected = (preview.digest(index).to_owned(), preview.protected.clone());
            let turns = shell.turns()?;
            let actual = turns.preview_basis().map_err(session)?;
            if actual != expected {
                return Err(invalid("The committed state or protection changed. Stage again."));
            }
            shell.answer = turns.twist_args(cap, item).map_err(session)?.canonical();
            Ok(())
        })
    }
}

/// Copies the preview's full moved set as one 0 or 1 byte per piece.
///
/// # Safety
/// The handle is live and exclusively owned. `out` is null or points to `count` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn m6h_preview_moved(handle: *mut ShellHost, id: u64, out: *mut u8, count: u32) -> i32 {
    unsafe {
        with_host(handle, |shell| {
            if out.is_null() || count as usize != PIECES {
                return Err(invalid("a moved set output holds one byte per piece"));
            }
            let preview = shell
                .preview
                .pending
                .as_ref()
                .filter(|p| p.id == id)
                .ok_or_else(|| invalid("the preview ID has changed"))?;
            std::ptr::copy_nonoverlapping(preview.moved.as_ptr(), out, PIECES);
            Ok(())
        })
    }
}

/// Copies the union of exact moving sets through the selected prefix of the current preview.
///
/// # Safety
/// The handle is live and exclusively owned. `out` is null or points to `count` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn m6h_preview_prefix_moved(handle: *mut ShellHost, id: u64, step: u32, out: *mut u8, count: u32) -> i32 {
    unsafe {
        with_host(handle, |shell| {
            if out.is_null() || count as usize != PIECES {
                return Err(invalid("a moved set output holds one byte per piece"));
            }
            if !shell.preview.current(shell.revision, shell.assigned) {
                return Err(invalid("the preview is absent or stale"));
            }
            let steps = shell.preview.pending.as_ref().filter(|p| p.id == id)
                .and_then(|p| p.steps.get(..step as usize)).ok_or_else(|| invalid("the preview ID or repair prefix has changed"))?;
            let mask = std::slice::from_raw_parts_mut(out, PIECES);
            mask.fill(0);
            for letter in steps {
                for (out, &moved) in mask.iter_mut().zip(&letter.moving) {
                    *out |= moved;
                }
            }
            Ok(())
        })
    }
}

/// Answers the assigned/adopted revision and the most recent produced composite frame's actual pane rows.
///
/// # Safety
/// The handle is live and exclusively owned.
#[no_mangle]
pub unsafe extern "C" fn m6h_preview_report(handle: *mut ShellHost) -> i32 {
    unsafe {
        with_host(handle, |shell| {
            let mut drawn_frame = Value::Null;
            for frame in shell.preview.frames.iter().rev() {
                if frame.flags.is_empty() {
                    break;
                }
                let Ok(rows) = shell.host.backend.read_sequence_pane_rows(frame.sequence) else { continue };
                let panes = rows
                    .iter()
                    .zip(&frame.flags)
                    .map(|(row, &flags)| {
                        let name = if flags & PANE_PREVIEW_BEFORE != 0 {
                            "Preview before"
                        } else if flags & PANE_PREVIEW != 0 {
                            "Preview after"
                        } else if row.view == LOCAL.0 {
                            "Local"
                        } else {
                            "Global"
                        };
                        json::object([
                            ("view", Value::String(name.to_owned())),
                            ("vertex_evals", Value::Int(row.vertex_evals as i64)),
                        ])
                    })
                    .collect();
                let (id, step) = frame.preview.unwrap_or((0, 0));
                drawn_frame = json::object([
                    ("sequence", Value::Int(frame.sequence as i64)),
                    ("revision", Value::Int(frame.revision as i64)),
                    ("preview_id", Value::Int(id as i64)),
                    ("prefix", Value::Int(step as i64)),
                    ("panes", Value::Array(panes)),
                ]);
                break;
            }
            shell.answer = json::object([
                ("head_revision", Value::Int(shell.assigned as i64)),
                ("adopted_revision", Value::Int(shell.revision as i64)),
                ("drawn", drawn_frame),
            ])
            .canonical();
            Ok(())
        })
    }
}
