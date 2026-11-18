//! [`DllCore`] over the C shim: what it refuses before calling the core (a null context, another ABI, arrays of
//! the wrong length), and what it marshals (JSON buffers that grow, struct sizes, the last error, device status).

mod common;

use c600_render::abi::{Sa2FrameInput, Status, PIECES, SLOTS};
use c600_render::{CoreAbi, CoreFunctions, DllCore, EngineImage, FakeCore, PoseArrays, PreparationInput};

use common::{Driver, Shimmed};

#[test]
fn a_null_context_or_another_abi_is_refused() {
    let mut shim = common::Shim { fake: FakeCore::new((8, 8)), pad: 0 };
    let context: *mut common::Shim = &mut shim;
    // SAFETY: the shim's table; a null context is refused before any call that takes it.
    assert_eq!(
        unsafe { DllCore::new(common::functions(), std::ptr::null_mut()) }.err(),
        Some(Status::INVALID_ARGUMENT)
    );
    let abi3 = CoreFunctions { abi_version: common::abi_version_3, ..common::functions() };
    // SAFETY: the shim's table and a live shim; only the ABI version is read.
    assert_eq!(unsafe { DllCore::new(abi3, context.cast()) }.err(), Some(Status::UNSUPPORTED));
}

#[test]
fn a_report_longer_than_the_first_buffer_is_read_again() {
    let core = Shimmed::new(FakeCore::new((8, 8)), 6000);
    let text = core.describe().expect("a report");
    assert!(text.len() > 4096 && text.starts_with("      ") && text.ends_with('}'), "{}", text.len());
    let (status, recovered) = Shimmed::new(FakeCore::new((8, 8)), 0).recover();
    assert_eq!(status, Status::UNAVAILABLE);
    assert!(recovered.contains("RPI.46"), "the JSON comes with the status");
    let oversized = Shimmed::new(FakeCore::new((8, 8)), c600_render::ffi::JSON_LIMIT);
    assert_eq!(oversized.describe(), Err(Status::INVALID_ARGUMENT), "a report over the limit is not read again");
}

#[test]
fn arrays_of_the_wrong_length_never_reach_the_core() {
    let mut core = Shimmed::new(FakeCore::new((8, 8)), 0);
    let labels: Vec<u32> = (0..SLOTS as u32).collect();
    let image = |labels| EngineImage { revision: 1, labels, pose: None, digest: [0; 32] };
    assert_eq!(core.engine_adopt(&image(&labels[1..])), Err(Status::INVALID_ARGUMENT));

    let index = vec![0i32; PIECES];
    let poses = [0f32; 32];
    let lattice = [0i32; 2];
    let posed = |pose_index, poses, pose_lattice| EngineImage {
        revision: 1,
        labels: &labels,
        pose: Some(PoseArrays { pose_index, poses, pose_lattice }),
        digest: [0; 32],
    };
    assert_eq!(core.engine_adopt(&posed(&index[1..], &poses, &lattice)), Err(Status::INVALID_ARGUMENT));
    assert_eq!(core.engine_adopt(&posed(&index, &poses[..31], &lattice)), Err(Status::INVALID_ARGUMENT));
    assert_eq!(core.engine_adopt(&posed(&index, &[], &[])), Err(Status::INVALID_ARGUMENT), "no empty pose table");
    assert!(core.fake().adoptions.is_empty());
    assert_eq!(core.engine_adopt(&posed(&index, &poses, &lattice)), Ok(1), "two poses pass");

    let full = vec![0u8; SLOTS];
    let instances = vec![0u32; SLOTS];
    let preparation = |visibility, instances| PreparationInput {
        command: 1,
        view: 1,
        view_generation: 1,
        publication_generation: 1,
        device_epoch: 1,
        base_revision: 1,
        visibility,
        interaction: &full,
        instances,
        input_digest: [0; 32],
    };
    assert_eq!(core.prepare(&preparation(&full[1..], &instances)), Err(Status::INVALID_ARGUMENT));
    assert_eq!(core.prepare(&preparation(&full, &instances[1..])), Err(Status::INVALID_ARGUMENT));
    assert!(core.fake().prepared.is_empty());
    assert!(core.prepare(&preparation(&full, &instances)).is_ok());
}

#[test]
fn rows_are_sized_and_errors_read_back() {
    let mut core = Shimmed::new(FakeCore::new((8, 8)), 0);
    let unsized_row = Sa2FrameInput { struct_size: 0, viewport_width: 8, viewport_height: 8, ..Sa2FrameInput::new() };
    // The shim checks struct_size; DllCore sets it, so the core refuses the frame for its state, not its size.
    assert_eq!(core.frame_submit(&unsized_row), Err(Status::WRONG_STATE));
    assert!(core.last_error().contains("epoch or engine revision"), "{}", core.last_error());
    assert_eq!(core.poll(77).map(|_| ()), Err(Status::WRONG_STATE));
    assert_eq!(core.last_error(), "fake core: an unknown or forgotten ticket");
    assert_eq!(core.counters().expect("counters").frames_submitted, 0);
    assert_eq!(core.device_status(), (Status::OK, 0));
    core.fake().remove_device();
    assert_eq!(core.device_status().0, Status::DEVICE_REMOVED);
    assert_eq!(core.pick_request(1, 0, 0), Err(Status::DEVICE_REMOVED));
}
