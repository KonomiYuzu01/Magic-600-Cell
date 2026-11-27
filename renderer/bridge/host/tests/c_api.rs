//! The shell's native host library through its C interface, over the bridge's C shim and the fake core on route B
//! (accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks): creation on the shipped model,
//! the publication installed, frames confirmed and read in batches with their display times, a lost device and
//! recovery to epoch 2, the refusals, and the preload's (charter E3's startup, a draft). Every result is a synthetic
//! fixture outcome, not a GPU run.

#[path = "../../tests/common/mod.rs"]
mod common;

use std::path::Path;
use std::ptr;

use c600_render::abi::{Sa2SceneLook, StickerColours, STICKER_CLASSES};
use c600_render::fake::DISPLAY_TICKS;
use c600_render::FakeCore;
use c600_shell_host::*;
use common::support::SIZE;
use common::{abi_version_previous, functions, Shim};

const CAMERA: [f32; 16] = [0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];

/// The shipped model's folder. The scenarios' `assets()` is relative to the crate that compiles them, so it is not
/// used here.
fn assets_path() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets");
    path.to_str().expect("a UTF-8 path").to_owned()
}

fn create(table: &CoreFunctions, shim: *mut Shim, path: &str) -> (i32, *mut ShellHost) {
    let mut out = ptr::null_mut();
    // SAFETY: the table is the shim's, the context a live shim, and the path's bytes are readable.
    let code = unsafe { m6h_create(table, shim.cast(), path.as_ptr(), path.len(), &mut out) };
    (code, out)
}

fn last_error(handle: *mut ShellHost) -> String {
    let mut size = 0u32;
    // SAFETY: a null buffer only asks for the size.
    assert_eq!(unsafe { m6h_last_error(handle, ptr::null_mut(), 0, &mut size) }, M6H_E_INVALID_ARGUMENT);
    let mut buffer = vec![0u8; size as usize];
    // SAFETY: the buffer holds `size` bytes.
    assert_eq!(unsafe { m6h_last_error(handle, buffer.as_mut_ptr(), size, &mut size) }, M6H_OK);
    assert_eq!(buffer.pop(), Some(0));
    String::from_utf8(buffer).expect("UTF-8")
}

fn frame(handle: *mut ShellHost) -> M6hFrame {
    let mut submitted = M6hFrame::default();
    // SAFETY: a live handle and a writable row.
    assert_eq!(unsafe { m6h_frame(handle, &mut submitted) }, M6H_OK, "{}", last_error(handle));
    submitted
}

fn read(handle: *mut ShellHost, capacity: u32) -> Vec<M6hPresented> {
    let mut rows = vec![M6hPresented::default(); capacity as usize];
    let mut count = u32::MAX;
    // SAFETY: `rows` holds `capacity` rows.
    assert_eq!(unsafe { m6h_read_presented(handle, rows.as_mut_ptr(), capacity, &mut count) }, M6H_OK);
    rows.truncate(count as usize);
    rows
}

#[test]
fn the_host_shows_frames_and_recovers_through_its_c_interface() {
    assert_eq!(m6h_abi_version(), M6H_ABI_VERSION);
    let shim = Box::into_raw(Box::new(Shim { fake: FakeCore::on_surface(SIZE), pad: 0 }));
    // SAFETY: the host uses the shim only inside its calls, never at the same time as the test.
    let fake = || unsafe { &mut (*shim).fake };
    let (code, host) = create(&functions(), shim, &assets_path());
    assert_eq!(code, M6H_OK, "{}", last_error(ptr::null_mut()));
    assert!(!host.is_null());
    assert_eq!(fake().adoptions.len(), 1, "the solved state is adopted");
    assert_eq!(fake().prepared.len(), 1, "the publication's preparation started");

    // The preparation completes; the next poll installs it. Frames go to the surface at the camera's version.
    fake().complete();
    // SAFETY: a live handle and 16 readable floats.
    unsafe {
        assert_eq!(m6h_pump(host), M6H_OK);
        assert_eq!(m6h_set_viewport(host, SIZE.0, SIZE.1), M6H_OK);
        assert_eq!(m6h_set_camera(host, CAMERA.as_ptr()), M6H_OK);
    }
    let first = frame(host);
    assert_eq!((first.sequence, first.device_epoch), (1, 1));
    let input = *fake().submitted.last().expect("a frame");
    assert_eq!(input.surface, first.surface);
    assert_eq!(input.target, c600_render::abi::TARGET_SURFACE);
    assert_eq!((input.camera, input.camera_matrix), (1, CAMERA));
    assert_eq!((input.viewport_width, input.viewport_height), SIZE);
    fake().complete();
    assert!(fake().show(1));
    let shown = read(host, 4);
    let expected = M6hPresented {
        sequence: 1,
        device_epoch: 1,
        surface: first.surface,
        engine_revision: ENGINE_REVISION,
        camera: 1,
        display_qpc: DISPLAY_TICKS,
    };
    assert_eq!(shown, [expected], "the row carries the confirming row's display time");

    // Two more frames, both confirmed, read one row at a time, each with its own display time.
    assert_eq!((frame(host).sequence, frame(host).sequence), (2, 3));
    fake().complete();
    assert!(fake().show(2) && fake().show(3));
    let (second, third) = (read(host, 1)[0], read(host, 1)[0]);
    assert_eq!((second.sequence, third.sequence), (2, 3));
    assert_eq!((second.display_qpc, third.display_qpc), (2 * DISPLAY_TICKS, 3 * DISPLAY_TICKS));
    assert!(read(host, 1).is_empty());

    // The device is removed: frames are unavailable until a recovery, in which the host prepares the installed
    // publication again for the new device (RM.23), under its own command.
    fake().remove_device();
    let mut submitted = M6hFrame::default();
    // SAFETY: a live handle and a writable row.
    assert_eq!(unsafe { m6h_frame(host, &mut submitted) }, M6H_E_UNAVAILABLE);
    assert_eq!(submitted, M6hFrame::default(), "a refused frame writes nothing");
    assert!(last_error(host).contains("renderer"), "{}", last_error(host));
    // SAFETY: a live handle.
    assert_eq!(unsafe { m6h_recover(host) }, M6H_OK, "{}", last_error(host));
    assert_eq!(fake().prepared.len(), 2, "the publication is prepared again for the new device");
    fake().complete();
    // SAFETY: a live handle.
    assert_eq!(unsafe { m6h_pump(host) }, M6H_OK);
    let after = frame(host);
    assert_eq!(after.device_epoch, 2);
    let id = fake().submitted.len() as u64;
    fake().complete();
    assert!(fake().show(id));
    let shown = read(host, 4);
    assert_eq!(shown.len(), 1);
    assert_eq!((shown[0].sequence, shown[0].device_epoch, shown[0].surface), (after.sequence, 2, after.surface));
    assert_eq!(shown[0].display_qpc, 4 * DISPLAY_TICKS);

    // SAFETY: the handle is not used again; the shim outlives it.
    unsafe {
        m6h_destroy(host);
        m6h_destroy(ptr::null_mut());
        drop(Box::from_raw(shim));
    }
}

#[test]
fn the_c_interface_refuses_what_it_cannot_use() {
    let shim = Box::into_raw(Box::new(Shim { fake: FakeCore::on_surface(SIZE), pad: 0 }));
    let path = assets_path();
    let mut out = ptr::null_mut();
    // SAFETY: every pointer is null or valid.
    unsafe {
        assert_eq!(
            m6h_create(&functions(), shim.cast(), path.as_ptr(), path.len(), ptr::null_mut()),
            M6H_E_INVALID_ARGUMENT
        );
        assert_eq!(m6h_create(ptr::null(), shim.cast(), path.as_ptr(), path.len(), &mut out), M6H_E_INVALID_ARGUMENT);
        assert_eq!(
            m6h_create(&functions(), ptr::null_mut(), path.as_ptr(), path.len(), &mut out),
            M6H_E_INVALID_ARGUMENT
        );
        assert_eq!(m6h_create(&functions(), shim.cast(), [0xFFu8].as_ptr(), 1, &mut out), M6H_E_INVALID_ARGUMENT);
    }
    assert!(out.is_null());
    assert!(last_error(ptr::null_mut()).contains("UTF-8"));

    // A core of the previous ABI is not bound, and a folder without the model does not load.
    let mut previous = functions();
    previous.abi_version = abi_version_previous;
    assert_eq!(create(&previous, shim, &path).0, M6H_E_CORE);
    assert!(last_error(ptr::null_mut()).contains("UNSUPPORTED"), "{}", last_error(ptr::null_mut()));
    let missing = format!("{path}/no-such-folder");
    let (code, host) = create(&functions(), shim, &missing);
    assert_eq!((code, host), (M6H_E_MODEL, ptr::null_mut()));
    assert!(last_error(ptr::null_mut()).contains("shipped model"));

    // The preload: a null output or path, a path that is not UTF-8 and a session flag other than 0 or 1 start nothing.
    let mut preload = ptr::null_mut();
    // SAFETY: every pointer is null or valid.
    unsafe {
        assert_eq!(m6h_preload(path.as_ptr(), path.len(), 0, ptr::null_mut()), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_preload(ptr::null(), 0, 0, &mut preload), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_preload([0xFFu8].as_ptr(), 1, 0, &mut preload), M6H_E_INVALID_ARGUMENT);
        assert!(last_error(ptr::null_mut()).contains("UTF-8"));
        assert_eq!(m6h_preload(path.as_ptr(), path.len(), 2, &mut preload), M6H_E_INVALID_ARGUMENT);
        assert!(last_error(ptr::null_mut()).contains("session flag"), "{}", last_error(ptr::null_mut()));
        assert!(preload.is_null());
    }
    // A create from a preload takes it whatever it answers: a null preload, table or context is refused, a core of
    // the previous ABI is not bound, and a folder without the model answers as m6h_create does.
    let started = |folder: &str, session: u32| {
        let mut preload = ptr::null_mut();
        // SAFETY: the path's bytes are readable and the output writable.
        assert_eq!(unsafe { m6h_preload(folder.as_ptr(), folder.len(), session, &mut preload) }, M6H_OK);
        assert!(!preload.is_null());
        preload
    };
    let mut out = ptr::null_mut();
    // SAFETY: every pointer is null or valid, and each preload goes to one call.
    unsafe {
        assert_eq!(m6h_create_preloaded(&functions(), shim.cast(), ptr::null_mut(), &mut out), M6H_E_INVALID_ARGUMENT);
        assert!(last_error(ptr::null_mut()).contains("null preload"));
        let preload = started(&path, 0);
        assert_eq!(m6h_create_preloaded(&functions(), shim.cast(), preload, ptr::null_mut()), M6H_E_INVALID_ARGUMENT);
        let preload = started(&path, 0);
        assert_eq!(m6h_create_preloaded(ptr::null(), shim.cast(), preload, &mut out), M6H_E_INVALID_ARGUMENT);
        let preload = started(&path, 1);
        assert_eq!(m6h_create_preloaded(&previous, shim.cast(), preload, &mut out), M6H_E_CORE);
        assert!(last_error(ptr::null_mut()).contains("UNSUPPORTED"), "{}", last_error(ptr::null_mut()));
        let preload = started(&missing, 1);
        assert_eq!(m6h_create_preloaded(&functions(), shim.cast(), preload, &mut out), M6H_E_MODEL);
        assert!(last_error(ptr::null_mut()).contains("shipped model"));
        assert!(out.is_null());
        // A preload that no call takes is freed after its loads end.
        m6h_preload_free(started(&path, 1));
        m6h_preload_free(ptr::null_mut());
    }

    // A null handle or output.
    let mut count = 0;
    // SAFETY: null handles are refused before any use.
    unsafe {
        assert_eq!(m6h_pump(ptr::null_mut()), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_frame(ptr::null_mut(), ptr::null_mut()), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_set_camera(ptr::null_mut(), CAMERA.as_ptr()), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_read_presented(ptr::null_mut(), ptr::null_mut(), 0, &mut count), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_read_presented(ptr::null_mut(), ptr::null_mut(), 1, &mut count), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_last_error(ptr::null_mut(), ptr::null_mut(), 0, ptr::null_mut()), M6H_E_INVALID_ARGUMENT);
        drop(Box::from_raw(shim));
    }
}

#[test]
fn the_host_sets_a_view_focus() {
    let shim = Box::into_raw(Box::new(Shim { fake: FakeCore::on_surface(SIZE), pad: 0 }));
    // SAFETY: the host uses the shim only inside its calls, never at the same time as the test.
    let fake = || unsafe { &mut (*shim).fake };
    let (code, host) = create(&functions(), shim, &assets_path());
    assert_eq!(code, M6H_OK, "{}", last_error(ptr::null_mut()));
    let local = LOCAL.0;
    let cap = 121.0f32 / 125.0;
    // SAFETY: a live handle, or null where the refusal is the point.
    unsafe {
        // Local is not open yet: its focus is refused and nothing reaches the core.
        assert_eq!(m6h_set_view_focus(host, local, 1, 150, cap, 1.7), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_open_local(host), M6H_OK, "{}", last_error(host));
        assert_eq!(m6h_set_view_focus(host, local, 1, 150, cap, 1.7), M6H_OK, "{}", last_error(host));
        let focus = fake().focuses[&local];
        assert_eq!((focus.projection, focus.focus_cell, focus.focus_cos, focus.spread), (1, 150, cap, 1.7));
        // Out-of-range values change nothing: a projection, a cell, a threshold or a spread outside its range.
        for (projection, cell, cos, spread) in
            [(3, 150, cap, 1.7), (1, 600, cap, 1.7), (1, 150, f32::NAN, 1.7), (1, 150, 1.5, 1.7), (1, 150, cap, 0.5), (1, 150, cap, 3.5)]
        {
            assert_eq!(m6h_set_view_focus(host, local, projection, cell, cos, spread), M6H_E_INVALID_ARGUMENT);
        }
        assert_eq!(fake().focuses[&local].focus_cell, 150);
        // No cell, the whole view: still a focus (orthographic), so the core keeps it.
        assert_eq!(m6h_set_view_focus(host, local, 1, u32::MAX, -1.0, 1.7), M6H_OK);
        assert_eq!(fake().focuses[&local].focus_cell, u32::MAX);
        // The default (perspective, none, -1, 1) clears it; Global's focus is the default throughout.
        assert_eq!(m6h_set_view_focus(host, local, 0, u32::MAX, -1.0, 1.0), M6H_OK);
        assert!(fake().focuses.is_empty());
        assert_eq!(m6h_set_view_focus(ptr::null_mut(), local, 0, u32::MAX, -1.0, 1.0), M6H_E_INVALID_ARGUMENT);
        m6h_destroy(host);
        drop(Box::from_raw(shim));
    }
}

#[test]
fn the_host_sets_a_view_layout() {
    let shim = Box::into_raw(Box::new(Shim { fake: FakeCore::on_surface(SIZE), pad: 0 }));
    // SAFETY: the host uses the shim only inside its calls, never at the same time as the test.
    let fake = || unsafe { &mut (*shim).fake };
    let (code, host) = create(&functions(), shim, &assets_path());
    assert_eq!(code, M6H_OK, "{}", last_error(ptr::null_mut()));
    let mut rings = c600_render::abi::Sa2ViewLayout::new();
    rings.kind = c600_render::abi::LAYOUT_HOPF_RINGS;
    rings.j = [0.0, 1.0, 0.0, 0.0];
    rings.k = [0.0, 0.0, 1.0, 0.0];
    // SAFETY: a live handle and a readable row, or null.
    unsafe {
        // Local is not open: its layout is refused.
        assert_eq!(m6h_set_view_layout(host, LOCAL.0, &rings), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_set_view_layout(host, VIEW.0, &rings), M6H_OK, "{}", last_error(host));
        assert_eq!(fake().layouts[&VIEW.0], c600_render::abi::LAYOUT_HOPF_RINGS);
        // A row of another size, an unknown kind, a ring or half-step out of range or a value not finite changes nothing.
        let mut bad = rings;
        bad.struct_size = 1500;
        assert_eq!(m6h_set_view_layout(host, VIEW.0, &bad), M6H_E_INVALID_ARGUMENT);
        for change in 0..4 {
            let mut bad = rings;
            match change {
                0 => bad.kind = 3,
                1 => bad.ring_of[0] = 20,
                2 => bad.half_step[0] = 60,
                _ => bad.origins[0] = f32::NAN,
            }
            assert_eq!(m6h_set_view_layout(host, VIEW.0, &bad), M6H_E_INVALID_ARGUMENT);
        }
        assert_eq!(fake().layouts.len(), 1);
        // Null restores the chain.
        assert_eq!(m6h_set_view_layout(host, VIEW.0, ptr::null()), M6H_OK);
        assert!(fake().layouts.is_empty());
        assert_eq!(m6h_set_view_layout(ptr::null_mut(), VIEW.0, ptr::null()), M6H_E_INVALID_ARGUMENT);
        m6h_destroy(host);
        drop(Box::from_raw(shim));
    }
}

#[test]
fn the_host_sets_its_view_shape() {
    let shim = Box::into_raw(Box::new(Shim { fake: FakeCore::on_surface(SIZE), pad: 0 }));
    // SAFETY: the host uses the shim only inside its calls, never at the same time as the test.
    let fake = || unsafe { &mut (*shim).fake };
    let (code, host) = create(&functions(), shim, &assets_path());
    assert_eq!(code, M6H_OK, "{}", last_error(ptr::null_mut()));
    let opening = [0.7f32, 0.55, 1.18, 1.15];
    let view = 1u64;
    // SAFETY: a live handle and 4 readable floats, or null.
    unsafe {
        assert_eq!(m6h_set_view_shape(host, view, opening.as_ptr()), M6H_OK, "{}", last_error(host));
        let shape = fake().shapes[&view];
        assert_eq!([shape.cell_shrink, shape.sticker_shrink, shape.d4, shape.zoom], opening);
        // A view that is not open, and values out of range or not finite, change nothing.
        assert_eq!(m6h_set_view_shape(host, 2, opening.as_ptr()), M6H_E_INVALID_ARGUMENT);
        for bad in
            [[f32::NAN, 0.55, 1.18, 1.15], [0.7, 0.0, 1.18, 1.15], [0.7, 0.55, 1.0, 1.15], [0.7, 0.55, 1.18, 65.0]]
        {
            assert_eq!(m6h_set_view_shape(host, view, bad.as_ptr()), M6H_E_INVALID_ARGUMENT);
        }
        assert_eq!(fake().shapes.len(), 1);
        assert_eq!(fake().shapes[&view].sticker_shrink, 0.55);
        // Null restores the defaults, the core's ABI 5 constants.
        assert_eq!(m6h_set_view_shape(host, view, ptr::null()), M6H_OK);
        assert!(fake().shapes.is_empty());
        assert_eq!(m6h_set_view_shape(ptr::null_mut(), view, ptr::null()), M6H_E_INVALID_ARGUMENT);
        m6h_destroy(host);
        drop(Box::from_raw(shim));
    }
}

#[test]
fn the_host_sets_the_piece_style_and_the_style_slots() {
    use c600_render::abi::{Sa2ViewStyle, PIECES, STYLE_CHANNEL_FILL, STYLE_CHANNEL_OUTLINE, STYLE_TINT_FILL};
    let shim = Box::into_raw(Box::new(Shim { fake: FakeCore::on_surface(SIZE), pad: 0 }));
    // SAFETY: the host uses the shim only inside its calls, never at the same time as the test.
    let fake = || unsafe { &mut (*shim).fake };
    let (code, host) = create(&functions(), shim, &assets_path());
    assert_eq!(code, M6H_OK, "{}", last_error(ptr::null_mut()));
    let mut fill = vec![0u8; PIECES];
    fill[3] = 0x83;
    let mut style = Sa2ViewStyle::new();
    style.tint_mode = STYLE_TINT_FILL;
    style.palette[12..16].copy_from_slice(&[1.0, 0.0, 0.0, 1.0]);
    // SAFETY: a live handle and readable rows, or null.
    unsafe {
        assert_eq!(
            m6h_set_piece_style(host, STYLE_CHANNEL_FILL, fill.as_ptr(), PIECES as u32, 0),
            M6H_OK,
            "{}",
            last_error(host)
        );
        assert_eq!(fake().piece_styles[0].as_deref(), Some(&fill[..]));
        // A short array, an outline byte with the dim bit, a third channel and a per-pose array without a session
        // change nothing.
        assert_eq!(m6h_set_piece_style(host, STYLE_CHANNEL_FILL, fill.as_ptr(), 10, 0), M6H_E_INVALID_ARGUMENT);
        assert_eq!(
            m6h_set_piece_style(host, STYLE_CHANNEL_OUTLINE, fill.as_ptr(), PIECES as u32, 0),
            M6H_E_INVALID_ARGUMENT
        );
        assert_eq!(m6h_set_piece_style(host, 2, fill.as_ptr(), PIECES as u32, 0), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_set_piece_style(host, STYLE_CHANNEL_FILL, fill.as_ptr(), 4, 1), M6H_E_SESSION);
        assert!(fake().piece_styles[1].is_none());
        assert_eq!(m6h_set_piece_style(host, STYLE_CHANNEL_FILL, ptr::null(), 0, 0), M6H_OK);
        assert!(fake().piece_styles[0].is_none());
        // Style slots 1 to 4; null clears; a value out of range or slot 5 changes nothing.
        assert_eq!(m6h_set_view_style(host, 1, &style), M6H_OK, "{}", last_error(host));
        assert_eq!(fake().view_styles[0], Some(style));
        let mut bad = style;
        bad.dim_amount = 2.0;
        assert_eq!(m6h_set_view_style(host, 2, &bad), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_set_view_style(host, 5, &style), M6H_E_INVALID_ARGUMENT);
        assert!(fake().view_styles[1..].iter().all(Option::is_none));
        assert_eq!(m6h_set_view_style(host, 1, ptr::null()), M6H_OK);
        assert!(fake().view_styles[0].is_none());
        // ADR-D02 B18 review findings B18-04 and B18-07: a transform names the adopted revision, and a bad byte,
        // plane, angle or phase answers M6H_E_INVALID_ARGUMENT in the host before the core is asked (the fake core
        // has no `sa2_transform_set`, so a transform that reaches it answers M6H_E_HOST); a clear needs no revision.
        let plane = [1.0f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let moving = vec![0u8; PIECES];
        let adopted = ENGINE_REVISION;
        let set = |revision: u64, plane: &[f32; 8], angle: f32, moving: &[u8], phase: f32| {
            m6h_set_transform(host, revision, plane.as_ptr(), angle, moving.as_ptr(), moving.len() as u32, phase)
        };
        assert_eq!(set(adopted + 1, &plane, 1.0, &moving, 0.0), M6H_E_INVALID_ARGUMENT, "another revision");
        let mut two = moving.clone();
        two[5] = 2;
        assert_eq!(set(adopted, &plane, 1.0, &two, 0.0), M6H_E_INVALID_ARGUMENT, "a moving byte of 2");
        assert_eq!(set(adopted, &plane, 1.0, &moving, 1.5), M6H_E_INVALID_ARGUMENT, "a phase above 1");
        assert_eq!(set(adopted, &plane, f32::NAN, &moving, 0.0), M6H_E_INVALID_ARGUMENT, "a NaN angle");
        assert_eq!(set(adopted, &plane, 4.0, &moving, 0.0), M6H_E_INVALID_ARGUMENT, "an angle above pi");
        let flat = [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];
        assert_eq!(set(adopted, &flat, 1.0, &moving, 0.0), M6H_E_INVALID_ARGUMENT, "a degenerate plane");
        assert_eq!(set(adopted, &plane, 1.0, &moving[..10], 0.0), M6H_E_INVALID_ARGUMENT, "a short moving set");
        let no_plane = m6h_set_transform(host, adopted, ptr::null(), 1.0, moving.as_ptr(), PIECES as u32, 0.0);
        assert_eq!(no_plane, M6H_E_INVALID_ARGUMENT, "a moving set without a plane");
        assert_eq!(set(adopted, &plane, 1.0, &moving, 0.0), M6H_E_HOST, "a valid transform reaches the core");
        assert_eq!(m6h_set_transform(host, 0, ptr::null(), 0.0, ptr::null(), 0, 0.0), M6H_E_HOST, "a clear reaches it");
        m6h_destroy(host);
        drop(Box::from_raw(shim));
    }
}

/// ADR-D02 B18b: a host on the fake core with its first publication installed, the viewport and the camera set, and,
/// with `local`, Local opened and installed too.
fn two_view_host(local: bool) -> (*mut Shim, *mut ShellHost) {
    let shim = Box::into_raw(Box::new(Shim { fake: FakeCore::on_surface(SIZE), pad: 0 }));
    let (code, host) = create(&functions(), shim, &assets_path());
    assert_eq!(code, M6H_OK, "{}", last_error(ptr::null_mut()));
    // SAFETY: a live handle and 16 readable floats; the shim is used only between the host's calls.
    unsafe {
        (*shim).fake.complete();
        assert_eq!(m6h_pump(host), M6H_OK);
        assert_eq!(m6h_set_viewport(host, SIZE.0, SIZE.1), M6H_OK);
        assert_eq!(m6h_set_camera(host, CAMERA.as_ptr()), M6H_OK);
        if local {
            assert_eq!(m6h_open_local(host), M6H_OK, "{}", last_error(host));
            (*shim).fake.complete();
            assert_eq!(m6h_pump(host), M6H_OK);
        }
    }
    (shim, host)
}

/// The left and right halves of the surface.
const LEFT: (i32, i32, u32, u32) = (0, 0, SIZE.0 / 2, SIZE.1);
const RIGHT: (i32, i32, u32, u32) = ((SIZE.0 / 2) as i32, 0, SIZE.0 / 2, SIZE.1);

#[test]
fn scene_colours_validate_before_calling_the_core() {
    let (shim, host) = two_view_host(false);
    let mut colours: StickerColours = [[0.2, 0.4, 0.6]; STICKER_CLASSES];
    let count = STICKER_CLASSES as u32;
    let mut look = Sa2SceneLook::default();
    // SAFETY: the handle, table and look are live; the handle is destroyed before the shim.
    unsafe {
        assert_eq!(m6h_set_sticker_colours(host, colours.as_ptr().cast(), count - 1), M6H_E_INVALID_ARGUMENT);
        colours[STICKER_CLASSES - 1][2] = f32::NAN;
        assert_eq!(m6h_set_sticker_colours(host, colours.as_ptr().cast(), count), M6H_E_INVALID_ARGUMENT);
        colours[STICKER_CLASSES - 1][2] = 0.6;
        assert_eq!(m6h_set_sticker_colours(host, colours.as_ptr().cast(), count), M6H_E_HOST);
        assert!(last_error(host).contains("the fake core does not draw sticker colours"));
        assert_eq!(m6h_set_sticker_colours(host, ptr::null(), count), M6H_E_HOST);
        assert_eq!(m6h_set_sticker_colours(host, colours.as_ptr().cast(), 0), M6H_E_HOST);

        assert_eq!(m6h_set_scene_look(host, &look), M6H_E_HOST);
        assert!(last_error(host).contains("the fake core does not draw scene looks"));
        look.fog = 1.5;
        assert_eq!(m6h_set_scene_look(host, &look), M6H_E_INVALID_ARGUMENT);
        look.struct_size -= 4;
        assert_eq!(m6h_set_scene_look(host, &look), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_set_scene_look(host, ptr::null()), M6H_E_HOST);

        m6h_destroy(host);
        drop(Box::from_raw(shim));
    }
}

fn set_pane(host: *mut ShellHost, pane: u32, view: u64, rect: (i32, i32, u32, u32), style: u32) -> i32 {
    // SAFETY: a live handle.
    unsafe { m6h_set_pane(host, pane, view, rect.0, rect.1, rect.2, rect.3, style) }
}

#[test]
fn one_view_frames_are_unchanged() {
    // The same calls on a host without Local and on one with Local open but no pane set give the core the same
    // frames: no composite frame, and the same inputs.
    let mut logs = Vec::new();
    for local in [false, true] {
        let (shim, host) = two_view_host(local);
        let frames = [frame(host), frame(host)];
        assert!(frames.iter().all(|f| (f.panes, f.held) == (0, 0)), "{frames:?}");
        // SAFETY: the shim is used only between the host's calls; the handle is not used again.
        unsafe {
            let fake = &(*shim).fake;
            assert!(fake.submitted_panes.iter().all(Vec::is_empty), "no composite frame without panes");
            logs.push(fake.submitted.clone());
            m6h_destroy(host);
            drop(Box::from_raw(shim));
        }
    }
    assert_eq!(logs[0], logs[1], "Local open without panes changes no frame");
}

#[test]
fn a_fifth_pane_is_refused() {
    let (shim, host) = two_view_host(false);
    let local = LOCAL.0;
    // Local is not open yet: a pane, a camera, a shape or a visibility for it is refused.
    assert_eq!(set_pane(host, 0, local, LEFT, 0), M6H_E_INVALID_ARGUMENT);
    // SAFETY: a live handle and 16 readable floats.
    unsafe {
        assert_eq!(m6h_set_view_camera(host, local, CAMERA.as_ptr()), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_set_view_visible(host, local, 0), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_open_local(host), M6H_OK, "{}", last_error(host));
        assert_eq!(m6h_open_local(host), M6H_E_INVALID_ARGUMENT, "Local is never opened twice");
        (*shim).fake.complete();
        assert_eq!(m6h_pump(host), M6H_OK);
        assert_eq!(m6h_set_view_visible(host, VIEW.0, 0), M6H_E_INVALID_ARGUMENT, "Global is never hidden");
        assert_eq!(m6h_set_view_visible(host, local, 2), M6H_E_INVALID_ARGUMENT);
    }
    // Four panes fill the table; a fifth index, an overlap, an empty or outside rectangle and style 5 are refused.
    let quarter = |i: i32| (i * (SIZE.0 / 4) as i32, 0, SIZE.0 / 4, SIZE.1);
    for pane in 0..M6H_PANES_MAX {
        let view = if pane % 2 == 0 { VIEW.0 } else { local };
        assert_eq!(set_pane(host, pane, view, quarter(pane as i32), pane), M6H_OK, "{}", last_error(host));
    }
    assert_eq!(set_pane(host, M6H_PANES_MAX, VIEW.0, (0, 0, 1, 1), 0), M6H_E_INVALID_ARGUMENT, "a fifth pane");
    assert_eq!(set_pane(host, 1, local, (150, 0, 100, 10), 0), M6H_E_INVALID_ARGUMENT, "an overlap");
    assert_eq!(set_pane(host, 1, local, (200, 0, 0, 10), 0), M6H_E_INVALID_ARGUMENT, "an empty rectangle");
    assert_eq!(set_pane(host, 3, local, (700, 0, 101, 10), 0), M6H_E_INVALID_ARGUMENT, "outside the viewport");
    assert_eq!(set_pane(host, 1, local, quarter(1), 5), M6H_E_INVALID_ARGUMENT, "style 5");
    assert_eq!(set_pane(host, 1, 3, quarter(1), 0), M6H_E_INVALID_ARGUMENT, "view 3");
    // SAFETY: a live handle.
    unsafe {
        assert_eq!(m6h_clear_pane(host, M6H_PANES_MAX), M6H_E_INVALID_ARGUMENT);
    }
    // The four panes draw as one composite frame of both views.
    let drawn = frame(host);
    assert_eq!((drawn.panes, drawn.held), (4, 0), "{drawn:?}");
    // SAFETY: the shim is used only between the host's calls; the handle is not used again.
    unsafe {
        let panes = (*shim).fake.submitted_panes.last().cloned().expect("a frame");
        assert_eq!(panes.iter().map(|p| (p.view, p.style)).collect::<Vec<_>>(), [(1, 0), (2, 1), (1, 2), (2, 3)]);
        assert!(panes.iter().all(|p| p.resource_set != 0), "each pane draws its view's installed set");
        for pane in 0..M6H_PANES_MAX {
            assert_eq!(m6h_clear_pane(host, pane), M6H_OK);
        }
        assert_eq!(frame(host).panes, 0, "with every pane clear, a one-view frame");
        assert!((*shim).fake.submitted_panes.last().expect("a frame").is_empty());
        m6h_destroy(host);
        drop(Box::from_raw(shim));
    }
}

#[test]
fn a_hidden_local_adds_no_vertex_evaluations() {
    use c600_render::fake::PANE_EVALS;
    use c600_render::CoreAbi;
    let (shim, host) = two_view_host(true);
    // SAFETY: the shim is used only between the host's calls.
    let fake = || unsafe { &mut (*shim).fake };
    let turned = [1.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    // SAFETY: a live handle and 16 readable floats.
    unsafe {
        assert_eq!(m6h_set_view_camera(host, LOCAL.0, turned.as_ptr()), M6H_OK, "{}", last_error(host));
    }
    assert_eq!(set_pane(host, 0, VIEW.0, LEFT, 0), M6H_OK, "{}", last_error(host));
    assert_eq!(set_pane(host, 1, LOCAL.0, RIGHT, 0), M6H_OK, "{}", last_error(host));
    // Each frame reads the reports of the frames before it.
    let evals = |fake: &mut FakeCore| fake.counters().expect("counters").view_vertex_evals_last;
    assert_eq!(frame(host).panes, 2);
    let panes = fake().submitted_panes.last().cloned().expect("a composite frame");
    assert_eq!((panes[0].camera_matrix, panes[1].camera_matrix), (CAMERA, turned), "each pane has its view's camera");
    fake().complete();
    assert_eq!(frame(host).panes, 2);
    assert_eq!(evals(fake()), 2 * PANE_EVALS, "two panes, two vertex passes");
    // SAFETY: a live handle.
    unsafe {
        assert_eq!(m6h_set_view_visible(host, LOCAL.0, 0), M6H_OK);
    }
    fake().complete();
    assert_eq!(frame(host).panes, 1, "a hidden Local gets no pane");
    fake().complete();
    assert_eq!(frame(host).panes, 1);
    assert_eq!(evals(fake()), PANE_EVALS, "a hidden Local adds no vertex evaluations");
    let views: Vec<u64> = fake().submitted_panes.last().expect("a frame").iter().map(|p| p.view).collect();
    assert_eq!(views, [VIEW.0]);
    // Shown again, its pane is drawn again.
    // SAFETY: a live handle; the handle is not used again and the shim outlives it.
    unsafe {
        assert_eq!(m6h_set_view_visible(host, LOCAL.0, 1), M6H_OK);
        assert_eq!(frame(host).panes, 2);
        m6h_destroy(host);
        drop(Box::from_raw(shim));
    }
}

/// ADR-D02 B21: a pane's flags reach the core's pane (the raw view's antipode chart); an unset pane, a fifth index or
/// another flag is refused, and a later m6h_set_pane resets them to 0.
#[test]
fn pane_flags_reach_the_core() {
    let (shim, host) = two_view_host(false);
    // SAFETY: a live handle.
    unsafe {
        assert_eq!(m6h_set_pane_flags(host, 0, 1), M6H_E_INVALID_ARGUMENT, "an unset pane");
    }
    assert_eq!(set_pane(host, 0, VIEW.0, LEFT, 0), M6H_OK, "{}", last_error(host));
    assert_eq!(set_pane(host, 1, VIEW.0, RIGHT, 0), M6H_OK, "{}", last_error(host));
    // SAFETY: a live handle.
    unsafe {
        assert_eq!(m6h_set_pane_flags(host, 1, 2), M6H_E_INVALID_ARGUMENT, "an unknown flag");
        assert_eq!(m6h_set_pane_flags(host, M6H_PANES_MAX, 1), M6H_E_INVALID_ARGUMENT, "a fifth pane");
        assert_eq!(m6h_set_pane_flags(host, 1, 1), M6H_OK, "{}", last_error(host));
    }
    assert_eq!(frame(host).panes, 2);
    // SAFETY: the shim is used only between the host's calls.
    unsafe {
        let panes = (*shim).fake.submitted_panes.last().cloned().expect("a frame");
        assert_eq!(panes.iter().map(|p| p.flags).collect::<Vec<_>>(), vec![0, 1]);
    }
    assert_eq!(set_pane(host, 1, VIEW.0, RIGHT, 0), M6H_OK, "{}", last_error(host));
    assert_eq!(frame(host).panes, 2);
    // SAFETY: the shim is used only between the host's calls; the handle is not used again.
    unsafe {
        let panes = (*shim).fake.submitted_panes.last().cloned().expect("a frame");
        assert!(panes.iter().all(|p| p.flags == 0), "a set pane resets its flags");
    }
}
