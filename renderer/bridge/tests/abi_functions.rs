//! The C function table against `renderer/core/include/sa2_interop.h`: the signature of every function the table
//! holds, in [`SYMBOLS`] order. The header is read, not copied, so a header change fails here.

mod common;

use std::any::type_name;
use std::ffi::{c_char, c_void};
use std::path::Path;

use c600_render_ffi::SYMBOLS;

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
        ty(&f.view_shape_set),
        ty(&f.style_set),
        ty(&f.view_style_set),
        ty(&f.frame_submit_panes),
        ty(&f.frame_panes),
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
