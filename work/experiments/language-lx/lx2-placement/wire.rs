// Experiment framing: little-endian u32 length, then bytes. No shared memory.
use std::io::{Read, Write};

pub fn read(input: &mut impl Read) -> Result<Vec<u8>, String> {
    let mut size = [0; 4];
    input.read_exact(&mut size).map_err(|_| "pipe closed")?;
    let size = u32::from_le_bytes(size) as usize;
    if size > 2 * 1024 * 1024 { return Err("frame too large".into()); }
    let mut bytes = vec![0; size];
    input.read_exact(&mut bytes).map_err(|_| "truncated frame")?;
    Ok(bytes)
}

pub fn write(output: &mut impl Write, bytes: &[u8]) -> Result<(), String> {
    output.write_all(&(bytes.len() as u32).to_le_bytes()).map_err(|_| "pipe write")?;
    output.write_all(bytes).map_err(|_| "pipe write")?;
    output.flush().map_err(|_| "pipe flush".into())
}

pub fn reply(output: &mut impl Write, result: Result<Vec<u8>, String>) -> Result<(), String> {
    let (tag, bytes) = match result { Ok(b) => (0, b), Err(e) => (1, e.into_bytes()) };
    let mut frame = vec![tag];
    frame.extend(bytes);
    write(output, &frame)
}

pub fn response(input: &mut impl Read) -> Result<Vec<u8>, String> {
    let frame = read(input)?;
    match frame.split_first() {
        Some((0, bytes)) => Ok(bytes.to_vec()),
        Some((1, bytes)) => Err(String::from_utf8_lossy(bytes).into_owned()),
        _ => Err("invalid response tag".into()),
    }
}
