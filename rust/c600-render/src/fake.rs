//! A fake core for headless tests: ABI 5's product calls in Rust (and ABI 6 to 8's), following the header's rules for tickets, frames,
//! reports, picks and device loss. Nothing runs on a GPU: uploads, frames and ID passes stay pending until
//! [`FakeCore::complete`], and the ID pass reads the instance of the slot a test placed at a pixel
//! ([`FakeCore::place`]). It does not model resize, trace windows, the 256-ticket and 1024-pick bounds or engine
//! ring staleness.
//!
//! [`FakeCore::new`] is a `sa2_attach` context (the Godot-texture route): it cannot confirm a presentation or recover.
//! [`FakeCore::on_surface`] is a `sa2_create` context with an attached surface (route B, accepted for now by the owner
//! on 18 Nov 2026, pending three owner-machine checks): its frames name [`TARGET_SURFACE`], [`FakeCore::show`]
//! confirms a frame's presentation under the exact rule, and `recover` rebuilds a removed device.

use std::collections::{BTreeMap, VecDeque};

use c600_digest::sha256_hex;

use crate::abi::*;
use crate::api::{CoreAbi, EngineImage, PoseArrays, PreparationInput};

/// `DXGI_ERROR_DEVICE_REMOVED`, the reason a removed fake device reports.
pub const DEVICE_REMOVED_REASON: i32 = 0x887A_0005_u32 as i32;
/// The vertex evaluations the fake reports for each pane of a frame (a one-view frame is one pane): 120 cells of
/// 600 vertices, a fixed synthetic value.
pub const PANE_EVALS: u64 = 120 * 600;
/// The fake display clock's step per confirmed frame, in QPC ticks: one 60 Hz refresh at a 10 MHz counter.
pub const DISPLAY_TICKS: u64 = 166_667;

#[derive(Clone, Debug)]
struct Ticket {
    row: Sa2PollRow,
    view: u64,
    view_generation: u64,
    base_revision: u64,
    visibility: Vec<u8>,
    instances: BTreeMap<usize, u32>,
}

#[derive(Clone, Debug)]
struct Set {
    view: u64,
    view_generation: u64,
    base_revision: u64,
    visibility: Vec<u8>,
    instances: BTreeMap<usize, u32>,
    active: bool,
    retired: bool,
}

#[derive(Clone, Debug)]
struct Frame {
    id: u64,
    input: Sa2FrameInput,
    /// ABI 8: a composite frame's panes; empty for a one-view frame.
    panes: Vec<Sa2Pane>,
    completed: bool,
    error: Status,
    reported: bool,
}

#[derive(Clone, Debug)]
struct Pick {
    row: Sa2PickRow,
    done: bool,
    set: u64,
}

/// `sa2_engine_adopt`'s pose rules, as the S-A2 core checks them: one index below the pose count per piece, finite
/// rows with the identity first, and lattice entries of -1 or below the group's order with entry 0 equal to 0.
fn check_pose(pose: &PoseArrays<'_>) -> Result<(), &'static str> {
    let count = pose.pose_lattice.len();
    if count == 0 || count > MAX_POSES || pose.poses.len() != count * 16 || pose.pose_index.len() != PIECES {
        return Err("a pose table is not PIECES indices and 1..SA2_MAX_POSES poses and lattice entries");
    }
    if pose.pose_index.iter().any(|&i| i < 0 || i as usize >= count) {
        return Err("a pose index is not below pose_count");
    }
    if pose.poses.iter().any(|v| !v.is_finite()) {
        return Err("a pose entry is not finite");
    }
    if (0..16).any(|i| pose.poses[i] != if i % 5 == 0 { 1.0 } else { 0.0 }) {
        return Err("pose 0 is not the identity");
    }
    if pose.pose_lattice.iter().any(|&e| e < -1 || e >= POSE_GROUP_ORDER) || pose.pose_lattice[0] != 0 {
        return Err("a pose_lattice entry is not -1 or below SA2_POSE_GROUP_ORDER, or entry 0 is not 0");
    }
    Ok(())
}

/// The fake core.
#[derive(Debug)]
pub struct FakeCore {
    pub device_epoch: u64,
    /// The frame target size every frame's viewport must equal.
    pub target_size: (u32, u32),
    /// A `sa2_create` context with an attached surface (route B).
    surface: bool,
    /// Confirmed presentations not yet read, in frame order.
    confirmed: VecDeque<Sa2PresentedRow>,
    /// The newest frame a presentation confirmation passed: no frame up to it is confirmed later.
    shown_up_to: u64,
    /// The fake display clock: the `display_qpc` of the last confirmed frame.
    pub display_qpc: u64,
    removed: bool,
    adopted: Option<u64>,
    next_ticket: u64,
    tickets: BTreeMap<u64, Ticket>,
    sets: BTreeMap<u64, Set>,
    next_frame: u64,
    frames: VecDeque<Frame>,
    surfaces: BTreeMap<u64, (u64, u64)>,
    views: BTreeMap<u64, u64>,
    next_pick: u64,
    picks: BTreeMap<u64, Pick>,
    placed: BTreeMap<(u32, u32), usize>,
    counters: Sa2CounterValues,
    error: String,
    /// Every preparation, as the core copied it: the view, its instances by slot and its visibility.
    pub prepared: Vec<(u64, BTreeMap<usize, u32>, Vec<u8>)>,
    /// Every accepted frame input, in submission order.
    pub submitted: Vec<Sa2FrameInput>,
    /// ABI 8: the panes of every accepted frame, in submission order; empty for a one-view frame.
    pub submitted_panes: Vec<Vec<Sa2Pane>>,
    /// A view whose preparations [`FakeCore::complete`] leaves pending, as an upload behind other work: a test of a
    /// composite frame delays one view's activation with it.
    pub hold_view: Option<u64>,
    /// Every adopted engine revision with its first label.
    pub adoptions: Vec<(u64, u32)>,
    /// ABI 6: every view's shape that `sa2_view_shape_set` set and did not restore to the defaults.
    pub shapes: BTreeMap<u64, Sa2ViewShape>,
    /// ABI 9: every view's focus that `sa2_view_focus_set` set and did not restore to the defaults.
    pub focuses: BTreeMap<u64, Sa2ViewFocus>,
    /// ABI 7: the adopted revision's piece style by channel (fill, outline), dropped by an adoption.
    pub piece_styles: [Option<Vec<u8>>; 2],
    /// ABI 7: the style slots 1 to 4 (index 0 is slot 1).
    pub view_styles: [Option<Sa2ViewStyle>; 4],
    /// Every posed adoption: its revision, its pose count and the SHA-256 (hexadecimal) of the little-endian bytes
    /// of its pose index, pose rows and lattice entries, as the bundle's array descriptors name them.
    pub posed: Vec<(u64, u32, [String; 3])>,
}

impl FakeCore {
    pub fn new(target_size: (u32, u32)) -> FakeCore {
        FakeCore {
            device_epoch: 1,
            target_size,
            surface: false,
            confirmed: VecDeque::new(),
            shown_up_to: 0,
            display_qpc: 0,
            removed: false,
            adopted: None,
            next_ticket: 1,
            tickets: BTreeMap::new(),
            sets: BTreeMap::new(),
            next_frame: 1,
            frames: VecDeque::new(),
            surfaces: BTreeMap::new(),
            views: BTreeMap::new(),
            next_pick: 1,
            picks: BTreeMap::new(),
            placed: BTreeMap::new(),
            shapes: BTreeMap::new(),
            focuses: BTreeMap::new(),
            piece_styles: [None, None],
            view_styles: [None; 4],
            counters: Sa2CounterValues::new(),
            error: String::new(),
            prepared: Vec::new(),
            submitted: Vec::new(),
            submitted_panes: Vec::new(),
            hold_view: None,
            adoptions: Vec::new(),
            posed: Vec::new(),
        }
    }

    /// A `sa2_create` context with a surface of `target_size` attached (route B).
    pub fn on_surface(target_size: (u32, u32)) -> FakeCore {
        FakeCore { surface: true, ..FakeCore::new(target_size) }
    }

    /// The display shows a completed, drawn frame and a statistics row names its own present count: the frame is
    /// confirmed at the fake display clock advanced by [`DISPLAY_TICKS`], and every older frame not yet confirmed
    /// never will be (the exact rule). False, and nothing changes, without a surface or for a frame that is not
    /// retained, did not draw or is not newer than the last shown.
    pub fn show(&mut self, frame_id: u64) -> bool {
        if !self.surface || self.removed || frame_id <= self.shown_up_to {
            return false;
        }
        let Some(frame) = self.frames.iter().find(|f| f.id == frame_id && f.completed && f.error == Status::OK) else {
            return false;
        };
        let i = frame.input;
        let preview = i.flags & FRAME_PREVIEW != 0;
        self.display_qpc += DISPLAY_TICKS;
        self.confirmed.push_back(Sa2PresentedRow {
            previews: u32::from(preview),
            frame_id,
            device_epoch: i.device_epoch,
            surface: i.surface,
            sequence: i.sequence,
            preview: if preview { i.preview } else { 0 },
            display_qpc: self.display_qpc,
            ..Sa2PresentedRow::new()
        });
        self.shown_up_to = frame_id;
        true
    }

    /// A faulty core, for tests: every confirmed presentation not yet read names `sequence` in place of its frame's.
    pub fn misbind_presented(&mut self, sequence: u64) {
        self.confirmed.iter_mut().for_each(|row| row.sequence = sequence);
    }

    /// The ID pass finds `slot` at pixel `(x, y)` of every frame.
    /// A faulty core, for tests: every pick not yet reported answers with `publication_generation` in place of its
    /// frame's.
    pub fn misbind_picks(&mut self, publication_generation: u64) {
        self.picks.values_mut().for_each(|pick| pick.row.publication_generation = publication_generation);
    }

    pub fn place(&mut self, x: u32, y: u32, slot: usize) {
        self.placed.insert((x, y), slot);
    }

    /// The GPU reaches every fence: uploads become ready (a cancelled one stays cancelled, RM.17; one of
    /// [`FakeCore::hold_view`] stays pending), frames complete and ID passes answer.
    pub fn complete(&mut self) {
        for ticket in self.tickets.values_mut() {
            if ticket.row.state == TICKET_PENDING && Some(ticket.view) != self.hold_view {
                ticket.row.state = TICKET_READY;
                let set = ticket.row.resource_set;
                self.sets.insert(
                    set,
                    Set {
                        view: ticket.view,
                        view_generation: ticket.view_generation,
                        base_revision: ticket.base_revision,
                        visibility: ticket.visibility.clone(),
                        instances: ticket.instances.clone(),
                        active: false,
                        retired: false,
                    },
                );
            }
        }
        self.frames.iter_mut().for_each(|f| f.completed = true);
        let newest = self.frames.back().map(|f| f.input);
        for pick in self.picks.values_mut().filter(|p| !p.done) {
            pick.done = true;
            if pick.row.state == PICK_STALE {
                continue;
            }
            let stale = newest.and_then(|n| {
                if n.surface == pick.row.surface && n.surface_generation > pick.row.surface_generation {
                    Some(PICK_STALE_SURFACE)
                } else if n.view == pick.row.view && n.view_generation > pick.row.view_generation {
                    Some(PICK_STALE_VIEW)
                } else {
                    None
                }
            });
            if let Some(field) = stale {
                pick.row.state = PICK_STALE;
                pick.row.stale = field;
                continue;
            }
            pick.row.state = PICK_READY;
            let set = self.sets.get(&pick.set);
            let slot = self.placed.get(&(pick.row.x, pick.row.y));
            pick.row.instance = match (set, slot) {
                (Some(set), Some(&slot)) if set.visibility[slot] == 1 => set.instances.get(&slot).copied().unwrap_or(0),
                _ => 0,
            };
        }
    }

    /// The device is removed: pending and ready tickets fail and unreported frames that the GPU did not complete
    /// report `SA2_E_DEVICE_REMOVED`.
    pub fn remove_device(&mut self) {
        self.removed = true;
        for ticket in self.tickets.values_mut() {
            if matches!(ticket.row.state, TICKET_PENDING | TICKET_READY) {
                ticket.row = Sa2PollRow { state: TICKET_FAILED, reason: Status::DEVICE_REMOVED.0, ..Sa2PollRow::new() };
            }
        }
        for frame in self.frames.iter_mut().filter(|f| !f.completed) {
            frame.completed = true;
            frame.error = Status::DEVICE_REMOVED;
        }
    }

    /// `sa2_frame_submit`'s checks of a frame input, shared by the composite frame.
    fn admit(&mut self, input: &Sa2FrameInput) -> Result<(), Status> {
        if input.struct_size as usize != std::mem::size_of::<Sa2FrameInput>() || input.reserved != 0 {
            return self.fail(Status::INVALID_ARGUMENT, "struct_size or reserved");
        }
        if input.flags & !FRAME_PREVIEW != 0 || input.camera_matrix.iter().any(|v| !v.is_finite()) {
            return self.fail(Status::INVALID_ARGUMENT, "unknown flags or a non-finite camera");
        }
        if input.target >= RING_SLOTS && input.target != TARGET_SURFACE {
            return self.fail(Status::INVALID_ARGUMENT, "target must be a ring slot 0..2 or SA2_TARGET_SURFACE");
        }
        if self.surface != (input.target == TARGET_SURFACE) {
            return self.fail(Status::WRONG_STATE, "the target is not this context's route");
        }
        self.alive()?;
        if input.device_epoch != self.device_epoch || Some(input.engine_revision) != self.adopted {
            return self.fail(Status::WRONG_STATE, "the frame names another epoch or engine revision");
        }
        if (input.viewport_width, input.viewport_height) != self.target_size {
            return self.fail(Status::WRONG_STATE, "the viewport is not the target's size");
        }
        if let Some(&(generation, sequence)) = self.surfaces.get(&input.surface) {
            if input.surface_generation < generation || input.sequence <= sequence {
                return self.fail(Status::WRONG_STATE, "the surface generation or sequence goes back");
            }
        }
        if self.views.get(&input.view).is_some_and(|&g| input.view_generation < g) {
            return self.fail(Status::WRONG_STATE, "the view generation goes back");
        }
        if input.resource_set != 0 {
            let fits = self
                .sets
                .get(&input.resource_set)
                .is_some_and(|s| s.active && s.view == input.view && s.view_generation == input.view_generation);
            if !fits {
                return self.fail(Status::WRONG_STATE, "the set is not active for the frame's view generation");
            }
        }
        Ok(())
    }

    /// Records an admitted frame: its surface and view bindings, its panes, and the next frame ID.
    fn record(&mut self, input: &Sa2FrameInput, panes: Vec<Sa2Pane>) -> u64 {
        self.surfaces.insert(input.surface, (input.surface_generation, input.sequence));
        self.views.insert(input.view, input.view_generation);
        let id = self.next_frame;
        self.next_frame += 1;
        self.frames.push_back(Frame {
            id,
            input: *input,
            panes: panes.clone(),
            completed: false,
            error: Status::OK,
            reported: false,
        });
        self.submitted.push(*input);
        self.submitted_panes.push(panes);
        self.counters.frames_submitted += 1;
        id
    }

    fn fail<T>(&mut self, status: Status, message: &str) -> Result<T, Status> {
        self.error = format!("fake core: {message}");
        Err(status)
    }

    fn alive(&mut self) -> Result<(), Status> {
        if self.removed {
            return self.fail(Status::DEVICE_REMOVED, "the device is removed");
        }
        Ok(())
    }

    fn removed_json(&self) -> String {
        if self.removed {
            format!("\"0x{:08x}\"", DEVICE_REMOVED_REASON as u32)
        } else {
            "null".to_owned()
        }
    }
}

impl CoreAbi for FakeCore {
    fn describe(&self) -> Result<String, Status> {
        let supported = |row: &str| format!("\"{row}\":{{\"status\":\"supported\",\"layout_version\":1}}");
        let unsupported =
            |row: &str, why: &str| format!("\"{row}\":{{\"status\":\"unsupported\",\"reason\":\"{why}\"}}");
        let (f22, f23) = if self.surface {
            (supported("F22"), supported("F23"))
        } else {
            (
                unsupported("F22", "recover: ABI 4 cannot recover a removed device (RPI.46)"),
                unsupported(
                    "F23",
                    "read_presented: the core owns no swap chain and cannot confirm a presentation (RM.26)",
                ),
            )
        };
        let rows = [
            supported("F01"),
            supported("F02"),
            supported("F18"),
            f22,
            f23,
            supported("J01"),
            supported("J02"),
            unsupported("J13", "no grip, piece or certificate-point IDs"),
        ];
        Ok(format!(
            "{{\"interface_version\":{},\"backend_build\":\"{}\",\"device_epoch\":{},\"device_removed\":{},\
             \"features\":{{{}}},\"evidence\":[]}}",
            ABI_VERSION,
            "0".repeat(64),
            self.device_epoch,
            self.removed_json(),
            rows.join(",")
        ))
    }

    fn engine_adopt(&mut self, image: &EngineImage<'_>) -> Result<u64, Status> {
        if image.labels.len() != SLOTS || image.labels.iter().any(|&l| l as usize >= SLOTS) {
            return self.fail(Status::INVALID_ARGUMENT, "the labels are not SA2_SLOTS labels below SA2_SLOTS");
        }
        if let Some(pose) = &image.pose {
            if let Err(message) = check_pose(pose) {
                return self.fail(Status::INVALID_ARGUMENT, message);
            }
        }
        if self.adopted.is_some_and(|a| image.revision <= a) {
            return self.fail(Status::WRONG_STATE, "the revision does not increase");
        }
        self.adopted = Some(image.revision);
        self.adoptions.push((image.revision, image.labels[0]));
        self.piece_styles = [None, None]; // ABI 7: a piece style is bound to its revision
        self.counters.bytes_uploaded += (SLOTS * 4) as u64;
        if let Some(pose) = &image.pose {
            let count = pose.pose_lattice.len();
            let digest = |bytes: Vec<u8>| sha256_hex(&bytes);
            self.posed.push((
                image.revision,
                count as u32,
                [
                    digest(pose.pose_index.iter().flat_map(|v| v.to_le_bytes()).collect()),
                    digest(pose.poses.iter().flat_map(|v| v.to_le_bytes()).collect()),
                    digest(pose.pose_lattice.iter().flat_map(|v| v.to_le_bytes()).collect()),
                ],
            ));
            self.counters.bytes_uploaded += (PIECES * 4 + count * (64 + 4)) as u64;
        }
        Ok(image.revision)
    }

    fn prepare(&mut self, input: &PreparationInput<'_>) -> Result<u64, Status> {
        if input.visibility.len() != SLOTS || input.interaction.len() != SLOTS || input.instances.len() != SLOTS {
            return self.fail(Status::INVALID_ARGUMENT, "a preparation array is not SA2_SLOTS long");
        }
        if input.visibility.iter().chain(input.interaction).any(|&v| v > 1) {
            return self.fail(Status::INVALID_ARGUMENT, "a set holds a value other than 0 or 1");
        }
        self.alive()?;
        if input.device_epoch != self.device_epoch {
            return self.fail(Status::WRONG_STATE, "the preparation names another device epoch");
        }
        let ticket = self.next_ticket;
        self.next_ticket += 1;
        let instances: BTreeMap<usize, u32> =
            input.instances.iter().enumerate().filter(|(_, &i)| i != 0).map(|(s, &i)| (s, i)).collect();
        self.prepared.push((input.view, instances.clone(), input.visibility.to_vec()));
        self.tickets.insert(
            ticket,
            Ticket {
                row: Sa2PollRow { state: TICKET_PENDING, resource_set: 1000 + ticket, ..Sa2PollRow::new() },
                view: input.view,
                view_generation: input.view_generation,
                base_revision: input.base_revision,
                visibility: input.visibility.to_vec(),
                instances,
            },
        );
        self.counters.bytes_uploaded += (SLOTS * 6) as u64;
        Ok(ticket)
    }

    fn poll(&mut self, ticket: u64) -> Result<Sa2PollRow, Status> {
        match self.tickets.get(&ticket) {
            None => self.fail(Status::WRONG_STATE, "an unknown or forgotten ticket"),
            Some(t) if t.row.state == TICKET_READY => Ok(t.row),
            Some(t) => Ok(Sa2PollRow { resource_set: 0, ..t.row }),
        }
    }

    fn activate(&mut self, resource_set: u64, _stamp_digest: &[u8; 32]) -> Result<(), Status> {
        self.alive()?;
        match self.sets.get_mut(&resource_set) {
            Some(set) if !set.active && !set.retired => {
                set.active = true;
                Ok(())
            }
            _ => self.fail(Status::WRONG_STATE, "the set is not ready, or already active or retired"),
        }
    }

    fn cancel(&mut self, ticket: u64, reason: i32, superseded_by: u64) -> Result<(), Status> {
        if !(CANCEL_BY_USER..=CANCEL_BUDGET).contains(&reason) {
            return self.fail(Status::INVALID_ARGUMENT, "an unknown cancel reason");
        }
        let Some(t) = self.tickets.get_mut(&ticket) else {
            return self.fail(Status::WRONG_STATE, "an unknown ticket");
        };
        if matches!(t.row.state, TICKET_CANCELLED | TICKET_FAILED) {
            return Ok(());
        }
        let set = t.row.resource_set;
        if self.sets.get(&set).is_some_and(|s| s.active) {
            return self.fail(Status::WRONG_STATE, "the ticket's set is active; retire it");
        }
        let by = if reason == CANCEL_SUPERSEDED { superseded_by } else { 0 };
        t.row = Sa2PollRow { state: TICKET_CANCELLED, reason, superseded_by: by, ..Sa2PollRow::new() };
        self.sets.remove(&set);
        Ok(())
    }

    fn retire(&mut self, resource_set: u64) -> Result<(), Status> {
        match self.sets.get_mut(&resource_set) {
            Some(set) if !set.retired => {
                set.retired = true;
                set.active = false;
                self.tickets.retain(|_, t| t.row.resource_set != resource_set);
                Ok(())
            }
            _ => self.fail(Status::WRONG_STATE, "an unknown or retired set"),
        }
    }

    fn frame_submit(&mut self, input: &Sa2FrameInput) -> Result<u64, Status> {
        self.admit(input)?;
        Ok(self.record(input, Vec::new()))
    }

    /// ABI 8: the header's pane checks, then the frame's; every pane's set must be active, stamped for its view and
    /// view generation, and prepared at the frame's engine revision.
    fn frame_submit_panes(&mut self, input: &Sa2FrameInput, panes: &[Sa2Pane]) -> Result<u64, Status> {
        if panes.is_empty() || panes.len() > PANES_MAX as usize {
            return self.fail(Status::INVALID_ARGUMENT, "a composite frame has 1..SA2_PANES_MAX panes");
        }
        let first = panes[0];
        if (input.view, input.view_generation, input.resource_set, input.camera_matrix)
            != (first.view, first.view_generation, first.resource_set, first.camera_matrix)
        {
            return self.fail(Status::INVALID_ARGUMENT, "the frame input is not the first pane's");
        }
        for (i, pane) in panes.iter().enumerate() {
            if pane.flags != 0 || pane.style > STYLE_SLOTS || pane.camera_matrix.iter().any(|v| !v.is_finite()) {
                return self.fail(Status::INVALID_ARGUMENT, "a pane's flags, style or camera");
            }
            if !pane.inside(self.target_size) || panes[..i].iter().any(|other| other.overlaps(pane)) {
                return self.fail(Status::INVALID_ARGUMENT, "a pane lies outside the target or overlaps another");
            }
        }
        self.admit(input)?;
        for pane in panes {
            if self.views.get(&pane.view).is_some_and(|&g| pane.view_generation < g) {
                return self.fail(Status::WRONG_STATE, "a pane's view generation goes back");
            }
            if pane.resource_set == 0 {
                continue;
            }
            let fits = self
                .sets
                .get(&pane.resource_set)
                .is_some_and(|s| s.active && (s.view, s.view_generation) == (pane.view, pane.view_generation));
            if !fits {
                return self.fail(Status::WRONG_STATE, "a pane's set is not active for its view generation");
            }
            if self.sets[&pane.resource_set].base_revision != input.engine_revision {
                return self.fail(Status::WRONG_STATE, "a pane's set was prepared at another engine revision");
            }
        }
        for pane in panes {
            let generation = self.views.entry(pane.view).or_insert(pane.view_generation);
            *generation = (*generation).max(pane.view_generation);
        }
        Ok(self.record(input, panes.to_vec()))
    }

    fn frame_panes(&mut self, frame_id: u64, rows: &mut [Sa2PaneRow]) -> Result<usize, Status> {
        let Some(frame) = self.frames.iter().find(|f| f.id == frame_id && !f.panes.is_empty()) else {
            return self.fail(Status::WRONG_STATE, "the frame is not a retained composite frame");
        };
        if !frame.completed || frame.error != Status::OK {
            return self.fail(Status::WRONG_STATE, "the composite frame has not produced");
        }
        for (i, (row, pane)) in rows.iter_mut().zip(&frame.panes).enumerate() {
            *row = Sa2PaneRow {
                pane: i as u32,
                view: pane.view,
                view_generation: pane.view_generation,
                vertex_evals: PANE_EVALS,
                ..Sa2PaneRow::new()
            };
        }
        Ok(frame.panes.len())
    }

    fn frame_reports(&mut self, rows: &mut [Sa2FrameReport]) -> Result<usize, Status> {
        let mut n = 0;
        for frame in self.frames.iter_mut().filter(|f| !f.reported) {
            if n == rows.len() || !frame.completed {
                break;
            }
            frame.reported = true;
            let i = frame.input;
            let evals = if frame.error == Status::OK { PANE_EVALS * frame.panes.len().max(1) as u64 } else { 0 };
            rows[n] = Sa2FrameReport {
                error: frame.error.0,
                frame_id: frame.id,
                device_epoch: i.device_epoch,
                surface: i.surface,
                surface_generation: i.surface_generation,
                sequence: i.sequence,
                engine_revision: i.engine_revision,
                view: i.view,
                view_generation: i.view_generation,
                publication_generation: i.publication_generation,
                gpu_time_ns: if frame.error == Status::OK { 1_000_000 } else { 0 },
                view_vertex_evals: evals,
                ..Sa2FrameReport::new()
            };
            self.counters.frames_reported += 1;
            self.counters.view_vertex_evals_last = evals;
            self.counters.view_vertex_evals_total += evals;
            n += 1;
        }
        while self.frames.iter().filter(|f| f.reported).count() > 64 {
            self.frames.pop_front();
        }
        Ok(n)
    }

    fn presented(&mut self, rows: &mut [Sa2PresentedRow]) -> Result<usize, Status> {
        if !self.surface {
            return self.fail(Status::UNAVAILABLE, "the core owns no swap chain and cannot confirm a presentation");
        }
        self.alive()?;
        let n = rows.len().min(self.confirmed.len());
        for (row, confirmed) in rows.iter_mut().zip(self.confirmed.drain(..n)) {
            *row = confirmed;
        }
        Ok(n)
    }

    fn pick_request(&mut self, frame_id: u64, x: u32, y: u32) -> Result<u64, Status> {
        self.alive()?;
        let Some(frame) = self.frames.iter().find(|f| f.id == frame_id && f.error == Status::OK) else {
            return self.fail(Status::WRONG_STATE, "the frame is not retained or did not draw");
        };
        let i = frame.input;
        if x >= i.viewport_width || y >= i.viewport_height {
            return self.fail(Status::INVALID_ARGUMENT, "the pixel is outside the viewport");
        }
        let mut row = Sa2PickRow {
            frame_id,
            device_epoch: i.device_epoch,
            surface: i.surface,
            surface_generation: i.surface_generation,
            view: i.view,
            view_generation: i.view_generation,
            publication_generation: i.publication_generation,
            camera: i.camera,
            visibility: i.visibility,
            interaction: i.interaction,
            viewport_width: i.viewport_width,
            viewport_height: i.viewport_height,
            x,
            y,
            ..Sa2PickRow::new()
        };
        let done = i.resource_set == 0 || self.sets.get(&i.resource_set).is_none_or(|s| s.retired);
        if i.resource_set != 0 && done {
            row.state = PICK_STALE;
            row.stale = PICK_STALE_PUBLICATION;
        } else if done {
            row.state = PICK_READY;
        } else {
            self.counters.id_passes += 1;
        }
        let ticket = self.next_pick;
        self.next_pick += 1;
        self.picks.insert(ticket, Pick { row, done, set: i.resource_set });
        Ok(ticket)
    }

    fn pick_poll(&mut self, ticket: u64) -> Result<Sa2PickRow, Status> {
        self.alive()?;
        match self.picks.get(&ticket) {
            None => self.fail(Status::WRONG_STATE, "an unknown or reported pick"),
            Some(pick) if !pick.done => Ok(Sa2PickRow { state: PICK_PENDING, instance: 0, ..pick.row }),
            Some(_) => Ok(self.picks.remove(&ticket).expect("present").row),
        }
    }

    fn recover(&mut self) -> (Status, String) {
        let result = |status: &str, reason: &str, epoch: u64, removed: &str| {
            format!(
                "{{\"status\":\"{status}\",\"reason\":\"{reason}\",\"device_epoch\":{epoch},\"device_removed\":{removed}}}"
            )
        };
        if !self.surface {
            self.error = "fake core: ABI 4 cannot recover a device".to_owned();
            let removed = self.removed_json();
            return (
                Status::UNAVAILABLE,
                result("unavailable", "ABI 4 cannot recover a device (RPI.46)", self.device_epoch, &removed),
            );
        }
        if !self.removed {
            self.error = "fake core: the device is not removed; nothing to recover".to_owned();
            let reason = "the device is not removed; nothing to recover";
            return (Status::WRONG_STATE, result("not_removed", reason, self.device_epoch, "null"));
        }
        // The old epoch's tickets, sets, picks and frames are gone: unread reports are dropped and old frames are
        // never confirmed. The adopted engine image is kept (RM.16), and numbers keep counting.
        let removed = self.removed_json();
        self.counters.reports_dropped += self.frames.iter().filter(|f| !f.reported).count() as u64;
        self.frames.clear();
        self.tickets.clear();
        self.sets.clear();
        self.picks.clear();
        self.confirmed.clear();
        self.shown_up_to = self.next_frame - 1;
        self.removed = false;
        self.device_epoch += 1;
        let reason = "recovered on a new device of the same adapter (RPI.46)";
        (Status::OK, result("recovered", reason, self.device_epoch, &removed))
    }

    fn counters(&self) -> Result<Sa2CounterValues, Status> {
        Ok(self.counters)
    }

    fn device_status(&self) -> (Status, i32) {
        if self.removed {
            (Status::DEVICE_REMOVED, DEVICE_REMOVED_REASON)
        } else {
            (Status::OK, 0)
        }
    }

    fn last_error(&self) -> String {
        self.error.clone()
    }

    /// The header's checks; device-free, so it works during device loss.
    fn view_focus_set(&mut self, view: u64, focus: Option<&Sa2ViewFocus>) -> Result<(), Status> {
        let focus = match focus {
            None => Sa2ViewFocus::defaults(),
            Some(focus) => {
                if focus.struct_size as usize != std::mem::size_of::<Sa2ViewFocus>() || focus.reserved != 0 {
                    return self.fail(Status::INVALID_ARGUMENT, "invalid view_focus struct_size or reserved");
                }
                if !focus.in_range() {
                    return self.fail(Status::INVALID_ARGUMENT, "a view focus value is not finite or out of its range");
                }
                *focus
            }
        };
        if focus == Sa2ViewFocus::defaults() {
            self.focuses.remove(&view);
        } else {
            self.focuses.insert(view, focus);
        }
        Ok(())
    }

    /// The header's checks; device-free, so it works during device loss.
    fn view_shape_set(&mut self, view: u64, shape: Option<&Sa2ViewShape>) -> Result<(), Status> {
        let shape = match shape {
            None => Sa2ViewShape::defaults(),
            Some(shape) => {
                if shape.struct_size as usize != std::mem::size_of::<Sa2ViewShape>() || shape.reserved != 0 {
                    return self.fail(Status::INVALID_ARGUMENT, "invalid view_shape struct_size or reserved");
                }
                if !shape.in_range() {
                    return self.fail(Status::INVALID_ARGUMENT, "a view shape value is not finite or out of its range");
                }
                *shape
            }
        };
        if shape == Sa2ViewShape::defaults() {
            self.shapes.remove(&view);
        } else {
            self.shapes.insert(view, shape);
        }
        Ok(())
    }

    /// The header's checks: the adopted revision only, the channel's bits only.
    fn style_set(&mut self, revision: u64, piece_style: Option<&[u8]>, channel: u32) -> Result<(), Status> {
        if channel != STYLE_CHANNEL_FILL && channel != STYLE_CHANNEL_OUTLINE {
            return self.fail(Status::INVALID_ARGUMENT, "channel must be fill or outline");
        }
        if piece_style.is_some_and(|bytes| !piece_style_valid(bytes, channel)) {
            return self.fail(Status::INVALID_ARGUMENT, "a piece style byte sets a bit its channel does not have");
        }
        if self.adopted != Some(revision) {
            return self.fail(Status::INVALID_ARGUMENT, "a piece style names a revision other than the adopted one");
        }
        self.piece_styles[channel as usize] = piece_style.map(<[u8]>::to_vec);
        Ok(())
    }

    fn view_style_set(&mut self, slot: u32, style: Option<&Sa2ViewStyle>) -> Result<(), Status> {
        if !(1..=STYLE_SLOTS).contains(&slot) {
            return self.fail(Status::INVALID_ARGUMENT, "a style slot is 1..SA2_STYLE_SLOTS");
        }
        if let Some(style) = style {
            if style.struct_size as usize != std::mem::size_of::<Sa2ViewStyle>()
                || style.reserved != 0
                || !style.in_range()
            {
                return self.fail(Status::INVALID_ARGUMENT, "an invalid view style");
            }
        }
        self.view_styles[slot as usize - 1] = style.copied();
        Ok(())
    }
}
