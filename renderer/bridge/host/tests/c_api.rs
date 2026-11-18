//! The shell's native host library through its C interface, over the bridge's C shim and the fake core on route B
//! (accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks): creation on the shipped model,
//! the publication installed, frames confirmed and read in batches with their display times, a lost device and
//! recovery to epoch 2, and the refusals. Every result is a synthetic fixture outcome, not a GPU run.

#[path = "../../tests/common/mod.rs"]
mod common;

use std::path::Path;
use std::ptr;

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

    // The device is removed: frames are unavailable until a recovery, which publishes the view again.
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
