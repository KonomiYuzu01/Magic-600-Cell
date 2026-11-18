//! The Rust declarations against `renderer/core/include/sa2_interop.h`: every ABI 4 row's field names, offsets and
//! size under the C rules of x64 (each against the header's static_assert too), every constant, and the signature
//! of every function in the table. The header is read, not copied, so a header change fails here.

mod common;

use std::any::type_name;
use std::collections::BTreeMap;
use std::ffi::{c_char, c_void};
use std::mem::{offset_of, size_of};
use std::path::Path;

use c600_render::abi::*;
use c600_render::ffi::SYMBOLS;

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

/// A type name without module paths.
fn short(name: &str) -> String {
    let (mut out, mut ident) = (String::new(), String::new());
    let mut chars = name.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_alphanumeric() || c == '_' {
            ident.push(c);
        } else if c == ':' && chars.peek() == Some(&':') {
            chars.next();
            ident.clear();
        } else {
            out.push_str(&ident);
            ident.clear();
            out.push(c);
        }
    }
    out + &ident
}

fn ty<T>(_: &T) -> String {
    short(type_name::<T>())
}

/// The Rust type of a C parameter or return type of the header.
fn rust_type(c: &str, array: bool) -> String {
    let (konst, c) = match c.strip_prefix("const ") {
        Some(rest) => (true, rest.trim()),
        None => (false, c.trim()),
    };
    let (pointer, base) = match c.strip_suffix('*') {
        Some(base) => (true, base.trim()),
        None => (array, c),
    };
    let base = match base {
        "sa2_context" => short(type_name::<c_void>()),
        "char" => short(type_name::<c_char>()),
        "uint8_t" => "u8".to_owned(),
        "int32_t" => "i32".to_owned(),
        "uint32_t" => "u32".to_owned(),
        "uint64_t" => "u64".to_owned(),
        "float" => "f32".to_owned(),
        row => row.split('_').map(|part| part[..1].to_uppercase() + &part[1..]).collect(),
    };
    match (pointer, konst) {
        (false, _) => base,
        (true, true) => format!("*const {base}"),
        (true, false) => format!("*mut {base}"),
    }
}

/// The header prototype of `symbol` as a Rust function pointer type.
fn prototype(header: &str, symbol: &str) -> String {
    let line = header
        .lines()
        .find(|line| line.starts_with("SA2_API") && line.contains(&format!(" {symbol}(")))
        .unwrap_or_else(|| panic!("{symbol} is exported by the header"));
    let returns = line["SA2_API".len()..].split_whitespace().next().unwrap();
    let params = &line[line.find('(').unwrap() + 1..line.rfind(')').unwrap()];
    let params: Vec<String> = if params.trim() == "void" {
        Vec::new()
    } else {
        params
            .split(',')
            .map(|param| {
                let (ty, _, count) = declaration(param);
                rust_type(&ty, count.is_some())
            })
            .collect()
    };
    format!("unsafe extern \"C\" fn({}) -> {}", params.join(", "), rust_type(returns, false))
}

#[test]
fn every_function_of_the_table_has_the_header_signature() {
    let h = header();
    let f = common::functions();
    let fields = [
        ty(&f.abi_version),
        ty(&f.last_error),
        ty(&f.describe),
        ty(&f.engine_adopt),
        ty(&f.prepare),
        ty(&f.poll),
        ty(&f.activate),
        ty(&f.cancel),
        ty(&f.retire),
        ty(&f.frame_submit),
        ty(&f.frame_reports),
        ty(&f.presented),
        ty(&f.pick_request),
        ty(&f.pick_poll),
        ty(&f.recover),
        ty(&f.counters),
        ty(&f.device_status),
    ];
    assert_eq!(fields.len(), SYMBOLS.len());
    // Controls: the comparison sees whole signatures, array parameters as pointers and `void` as no parameters.
    assert_eq!(prototype(&h, "sa2_abi_version"), "unsafe extern \"C\" fn() -> u32");
    assert_eq!(prototype(&h, "sa2_activate"), "unsafe extern \"C\" fn(*mut c_void, u64, *const u8) -> i32");
    assert_eq!(fields[5], "unsafe extern \"C\" fn(*mut c_void, u64, *mut Sa2PollRow) -> i32");
    assert_ne!(fields[3], prototype(&h, "sa2_prepare"));
    for (symbol, field) in SYMBOLS.iter().zip(fields) {
        assert_eq!(field, prototype(&h, symbol), "{symbol}");
    }
}
