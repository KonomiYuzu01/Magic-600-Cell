//! The Rust declarations against `renderer/core/include/sa2_interop.h`: every ABI 4 row's field names, offsets and
//! size under the C rules of x64 (each against the header's static_assert too) and every constant. The header is
//! read, not copied, so a header change fails here. The function table's signatures are checked in `renderer/bridge`.

use std::collections::BTreeMap;
use std::mem::{offset_of, size_of};
use std::path::Path;

use c600_render::abi::*;

fn header() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../renderer/core/include/sa2_interop.h");
    let text = std::fs::read_to_string(&path).expect("the core's header");
    // Comments out: block comments, then line comments.
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        let end = rest[start..].find("*/").expect("a closed comment");
        rest = &rest[start + end + 2..];
    }
    out.push_str(rest);
    out.lines().map(|line| line.split("//").next().unwrap_or("")).collect::<Vec<_>>().join("\n")
}

/// The size of a C scalar of the header.
fn scalar(c: &str) -> usize {
    match c {
        "uint8_t" => 1,
        "int32_t" | "uint32_t" | "float" => 4,
        "uint64_t" => 8,
        other => panic!("no scalar {other}"),
    }
}

/// A C declaration `type name` or `type name[n]`: its type text, name and element count.
fn declaration(text: &str) -> (String, String, Option<usize>) {
    let text = text.trim();
    let split = text.rfind(|c: char| c.is_whitespace() || c == '*').expect("a type and a name") + 1;
    let (ty, name) = (text[..split].trim(), text[split..].trim());
    match name.split_once('[') {
        Some((name, count)) => {
            let count = count.trim_end_matches(']').parse().expect("an array length");
            (ty.to_owned(), name.to_owned(), Some(count))
        }
        None => (ty.to_owned(), name.to_owned(), None),
    }
}

/// The fields and size of `typedef struct name { ... } name;` under x64's C layout.
fn c_struct(header: &str, name: &str) -> (usize, Vec<(String, usize)>) {
    let open = format!("typedef struct {name} {{");
    let start = header.find(&open).unwrap_or_else(|| panic!("{name} is in the header")) + open.len();
    let end = start + header[start..].find(&format!("}} {name};")).expect("the struct closes");
    let (mut offset, mut align, mut fields) = (0usize, 1usize, Vec::new());
    for field in header[start..end].split(';').map(str::trim).filter(|f| !f.is_empty()) {
        let (ty, field, count) = declaration(field);
        let size = if ty.ends_with('*') { 8 } else { scalar(&ty) };
        offset = offset.next_multiple_of(size);
        align = align.max(size);
        fields.push((field, offset));
        offset += size * count.unwrap_or(1);
    }
    let size = offset.next_multiple_of(align);
    let asserted = format!("static_assert(sizeof({name}) == ");
    let at = header.find(&asserted).unwrap_or_else(|| panic!("{name} has a static_assert")) + asserted.len();
    let declared: usize = header[at..].split(',').next().unwrap().trim().parse().expect("a size");
    assert_eq!(size, declared, "{name}: the computed layout differs from the header's static_assert");
    (size, fields)
}

macro_rules! rust_struct {
    ($ty:ty { $($field:ident),* $(,)? }) => {
        (size_of::<$ty>(), vec![$((stringify!($field).to_owned(), offset_of!($ty, $field))),*])
    };
}

#[test]
fn every_row_has_the_header_layout() {
    let h = header();
    let rows = [
        (
            "sa2_engine_image",
            rust_struct!(Sa2EngineImage {
                struct_size,
                pose_count,
                revision,
                labels,
                pose_index,
                poses,
                pose_lattice,
                digest,
            }),
        ),
        (
            "sa2_preparation",
            rust_struct!(Sa2Preparation {
                struct_size,
                reserved,
                command,
                view,
                view_generation,
                publication_generation,
                device_epoch,
                base_revision,
                visibility,
                interaction,
                instances,
                input_digest,
            }),
        ),
        (
            "sa2_poll_row",
            rust_struct!(Sa2PollRow { struct_size, state, reason, reserved, resource_set, superseded_by }),
        ),
        (
            "sa2_frame_input",
            rust_struct!(Sa2FrameInput {
                struct_size,
                target,
                flags,
                reserved,
                device_epoch,
                surface,
                surface_generation,
                sequence,
                engine_revision,
                view,
                view_generation,
                publication_generation,
                camera,
                viewport_width,
                viewport_height,
                visibility,
                interaction,
                resource_set,
                preview,
                preview_base,
                subjects_digest,
                preview_literal_digest,
                camera_matrix,
            }),
        ),
        (
            "sa2_frame_report",
            rust_struct!(Sa2FrameReport {
                struct_size,
                error,
                fallback,
                reserved,
                frame_id,
                device_epoch,
                surface,
                surface_generation,
                sequence,
                engine_revision,
                view,
                view_generation,
                publication_generation,
                gpu_time_ns,
                view_vertex_evals,
            }),
        ),
        (
            "sa2_presented_row",
            rust_struct!(Sa2PresentedRow { struct_size, previews, frame_id, device_epoch, surface, sequence, preview }),
        ),
        (
            "sa2_pick_row",
            rust_struct!(Sa2PickRow {
                struct_size,
                state,
                stale,
                instance,
                frame_id,
                device_epoch,
                surface,
                surface_generation,
                view,
                view_generation,
                publication_generation,
                camera,
                visibility,
                interaction,
                viewport_width,
                viewport_height,
                x,
                y,
            }),
        ),
        (
            "sa2_counter_values",
            rust_struct!(Sa2CounterValues {
                struct_size,
                reserved,
                view_vertex_evals_last,
                view_vertex_evals_total,
                bytes_uploaded,
                frames_submitted,
                frames_reported,
                id_passes,
                reports_dropped,
            }),
        ),
    ];
    for (name, rust) in rows {
        assert_eq!(c_struct(&h, name), rust, "{name}");
    }
    let sizes = [
        Sa2PollRow::new().struct_size,
        Sa2FrameInput::new().struct_size,
        Sa2FrameReport::new().struct_size,
        Sa2PresentedRow::new().struct_size,
        Sa2PickRow::new().struct_size,
        Sa2CounterValues::new().struct_size,
    ];
    assert_eq!(sizes, [32, 264, 104, 48, 112, 64], "new() sets struct_size");
}

#[test]
fn every_constant_has_the_header_value() {
    let defines: BTreeMap<String, i64> = header()
        .lines()
        .filter_map(|line| line.trim().strip_prefix("#define SA2_"))
        .filter_map(|rest| {
            let mut parts = rest.split_whitespace();
            let (name, value) = (parts.next()?, parts.next()?);
            Some((name.to_owned(), value.trim_end_matches('u').parse().ok()?))
        })
        .collect();
    let mut rust: Vec<(String, i64)> = vec![
        ("ABI_VERSION".into(), i64::from(ABI_VERSION)),
        ("SLOTS".into(), SLOTS as i64),
        ("PIECES".into(), PIECES as i64),
        ("MAX_POSES".into(), MAX_POSES as i64),
        ("POSE_GROUP_ORDER".into(), i64::from(POSE_GROUP_ORDER)),
        ("TICKET_PENDING".into(), i64::from(TICKET_PENDING)),
        ("TICKET_READY".into(), i64::from(TICKET_READY)),
        ("TICKET_CANCELLED".into(), i64::from(TICKET_CANCELLED)),
        ("TICKET_FAILED".into(), i64::from(TICKET_FAILED)),
        ("CANCEL_BY_USER".into(), i64::from(CANCEL_BY_USER)),
        ("CANCEL_SUPERSEDED".into(), i64::from(CANCEL_SUPERSEDED)),
        ("CANCEL_REPLACED_REVISION".into(), i64::from(CANCEL_REPLACED_REVISION)),
        ("CANCEL_BUDGET".into(), i64::from(CANCEL_BUDGET)),
        ("PICK_PENDING".into(), i64::from(PICK_PENDING)),
        ("PICK_READY".into(), i64::from(PICK_READY)),
        ("PICK_STALE".into(), i64::from(PICK_STALE)),
        ("PICK_STALE_SURFACE".into(), i64::from(PICK_STALE_SURFACE)),
        ("PICK_STALE_VIEW".into(), i64::from(PICK_STALE_VIEW)),
        ("PICK_STALE_TARGET".into(), i64::from(PICK_STALE_TARGET)),
        ("PICK_STALE_ENGINE".into(), i64::from(PICK_STALE_ENGINE)),
        ("PICK_STALE_PUBLICATION".into(), i64::from(PICK_STALE_PUBLICATION)),
        ("FRAME_PREVIEW".into(), i64::from(FRAME_PREVIEW)),
    ];
    for code in 0..=10 {
        rust.push((Status(code).name().to_owned(), i64::from(code)));
    }
    for (name, value) in rust {
        assert_eq!(defines.get(&name), Some(&value), "SA2_{name}");
    }
    assert_eq!(Status(11).name(), "unknown");
}
