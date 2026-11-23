//! Runs `import_archive` on an explicit grant and writes the import report; optionally exports a check bundle.
//!
//! ```text
//! c600-archive-import <grant.json> <report.json> [<bundle-request.json> <bundle.zip>]
//! ```
//!
//! The grant names every file by path: `{"manifest":{"name":N,"path":P},"members":[{"name":N,"path":P},...]}`. The
//! names are source names (metadata); the paths are where the caller put the bytes, which the test driver names by
//! digest so that no two source names share a path. Nothing else is read: no directory is listed, no repository is
//! opened, no process is started and no network is used. A bundle request is
//! `{"envelopes":[{"name":N,"dependencies":[{"name":N,"sha256":H|null,"member":N|null,"reason":R}]}]}`; a dependency
//! is present when `member` names a granted member.

use std::fs;

use c600_archive::{
    export_check_bundle, import_archive, Discriminator, GrantedFile, GrantedFiles, ImportTable, ImportTableRef, Limits,
    LimitsRef,
};
use c600_kernel::json::{parse, Value};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !(args.len() == 2 || args.len() == 4) {
        return Err("usage: c600-archive-import <grant.json> <report.json> [<bundle-request.json> <bundle.zip>]".into());
    }
    run(&args)
}

fn read(path: &str, limit: usize) -> Result<Vec<u8>, String> {
    let len = fs::metadata(path).map_err(|_| "a granted file cannot be read".to_string())?.len();
    if len > limit as u64 {
        return Err(format!("{}: a granted file is over {limit} bytes", c600_archive::SIZE_LIMIT));
    }
    fs::read(path).map_err(|_| "a granted file cannot be read".to_string())
}

fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v.get(key).and_then(Value::as_str).ok_or_else(|| format!("the grant lacks {key}"))
}

fn run(args: &[String]) -> Result<(), String> {
    let limits = Limits::research_archive();
    let grant = parse(&read(&args[0], 16 << 20)?).map_err(|_| "the grant is not strict JSON".to_string())?;
    let manifest = grant.get("manifest").ok_or("the grant names no manifest")?;
    let manifest = GrantedFile {
        name: text(manifest, "name")?.to_string(),
        bytes: read(text(manifest, "path")?, limits.max_manifest_bytes)?,
    };
    let mut members = Vec::new();
    for m in grant.get("members").and_then(Value::as_array).ok_or("the grant names no members")? {
        members.push(GrantedFile {
            name: text(m, "name")?.to_string(),
            bytes: read(text(m, "path")?, limits.max_member_stored_bytes)?,
        });
    }
    let table = ImportTable::edition_1();
    let discriminator = Discriminator::edition_1();
    let report = import_archive(
        GrantedFiles { manifest, members },
        ImportTableRef { table: &table, discriminator: &discriminator },
        LimitsRef(&limits),
    )
    .map_err(|e| e.to_string())?;
    let mut out = report.to_json().canonical().into_bytes();
    out.push(b'\n');
    fs::write(&args[1], out).map_err(|_| "the report cannot be written".to_string())?;

    if args.len() == 4 {
        let request = parse(&read(&args[2], 16 << 20)?).map_err(|_| "the bundle request is not strict JSON")?;
        let mut envelopes = Vec::new();
        for e in request.get("envelopes").and_then(Value::as_array).ok_or("the request names no envelopes")? {
            let name = text(e, "name")?;
            let mut envelope = report.envelope(name).ok_or("an envelope names no present record")?;
            for d in e.get("dependencies").and_then(Value::as_array).unwrap_or(&[]) {
                let bytes = match d.get("member").and_then(Value::as_str) {
                    Some(member) => {
                        let i = report.member(member).ok_or("a dependency names no listed member")?;
                        report.stored_bytes(i).map(<[u8]>::to_vec)
                    }
                    None => None,
                };
                envelope = envelope.with_dependency(
                    text(d, "name")?,
                    d.get("sha256").and_then(Value::as_str),
                    bytes,
                    text(d, "reason")?,
                );
            }
            envelopes.push(envelope);
        }
        let bundle = export_check_bundle(&envelopes, LimitsRef(&limits)).map_err(|e| e.to_string())?;
        fs::write(&args[3], &bundle.zip).map_err(|_| "the bundle cannot be written".to_string())?;
    }
    Ok(())
}
