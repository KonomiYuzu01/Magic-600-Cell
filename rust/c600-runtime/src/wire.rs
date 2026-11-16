//! Frames and the byte primitives of the transfer encoding (draft `c600-rt1-wire-1`).
//!
//! A frame is a little-endian `u32` payload length and the payload. A payload starts with the protocol byte
//! [`PROTOCOL`] and a message kind byte. A length above [`MAX_FRAME`] is refused before any payload byte is read or
//! allocated. Fields are fixed-width little-endian integers, so every value has exactly one encoding; a byte string or
//! a text is a `u64` length and its bytes, a text must be UTF-8, a flag is 0 or 1, and an absent value is 0 where a
//! present one is 1 and the value. A decoder refuses trailing bytes.

use std::io::{self, ErrorKind, Read, Write};

/// The protocol byte that starts every payload. It changes with any change of a message or field.
pub const PROTOCOL: u8 = 1;

/// The largest payload a frame may carry, in bytes, in either direction.
pub const MAX_FRAME: usize = 16 << 20;

/// Why a frame was not read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameError {
    /// The stream ended on a frame boundary.
    Closed,
    /// The stream ended inside a frame.
    Truncated,
    /// The length prefix exceeds [`MAX_FRAME`].
    Oversized(u64),
    /// The operating system refused the read.
    Io(String),
}

/// Reads one frame. It never allocates more than [`MAX_FRAME`] bytes.
pub fn read_frame(input: &mut impl Read) -> Result<Vec<u8>, FrameError> {
    let mut prefix = [0u8; 4];
    let mut got = 0;
    while got < prefix.len() {
        match input.read(&mut prefix[got..]) {
            Ok(0) => return Err(if got == 0 { FrameError::Closed } else { FrameError::Truncated }),
            Ok(n) => got += n,
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) if e.kind() == ErrorKind::BrokenPipe => {
                return Err(if got == 0 { FrameError::Closed } else { FrameError::Truncated })
            }
            Err(e) => return Err(FrameError::Io(e.kind().to_string())),
        }
    }
    let length = u64::from(u32::from_le_bytes(prefix));
    if length > MAX_FRAME as u64 {
        return Err(FrameError::Oversized(length));
    }
    let mut payload = vec![0u8; length as usize];
    match input.read_exact(&mut payload) {
        Ok(()) => Ok(payload),
        Err(e) if matches!(e.kind(), ErrorKind::UnexpectedEof | ErrorKind::BrokenPipe) => Err(FrameError::Truncated),
        Err(e) => Err(FrameError::Io(e.kind().to_string())),
    }
}

/// Writes one frame and flushes it.
pub fn write_frame(output: &mut impl Write, payload: &[u8]) -> io::Result<()> {
    if payload.len() > MAX_FRAME {
        return Err(io::Error::new(ErrorKind::InvalidInput, "a payload above the frame bound"));
    }
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    frame.extend_from_slice(payload);
    output.write_all(&frame)?;
    output.flush()
}

/// A payload that does not decode, with what was wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Malformed(pub String);

impl Malformed {
    pub fn new(what: impl Into<String>) -> Malformed {
        Malformed(what.into())
    }
}

impl std::fmt::Display for Malformed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// An encoder. With recording on, it also keeps the name of every message field it writes, in order, so a test can
/// hold the encoding against [`crate::message::SCHEMA`].
#[derive(Default)]
pub struct Enc {
    pub bytes: Vec<u8>,
    fields: Option<Vec<&'static str>>,
}

impl Enc {
    pub fn new() -> Enc {
        Enc::default()
    }

    /// An encoder that records field names.
    pub fn recording() -> Enc {
        Enc { bytes: Vec::new(), fields: Some(Vec::new()) }
    }

    /// The recorded field names.
    pub fn fields(&self) -> &[&'static str] {
        self.fields.as_deref().unwrap_or(&[])
    }

    /// Names the message field written next.
    pub fn field(&mut self, name: &'static str) {
        if let Some(fields) = &mut self.fields {
            fields.push(name);
        }
    }

    pub fn u8(&mut self, v: u8) {
        self.bytes.push(v);
    }

    pub fn bool(&mut self, v: bool) {
        self.bytes.push(u8::from(v));
    }

    pub fn u32(&mut self, v: u32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    pub fn i32(&mut self, v: i32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    pub fn u64(&mut self, v: u64) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    pub fn u128(&mut self, v: u128) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    pub fn i128(&mut self, v: i128) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    pub fn count(&mut self, n: usize) {
        self.u64(n as u64);
    }

    pub fn bytes(&mut self, v: &[u8]) {
        self.count(v.len());
        self.bytes.extend_from_slice(v);
    }

    pub fn text(&mut self, v: &str) {
        self.bytes(v.as_bytes());
    }
}

/// A decoder over one payload.
pub struct Dec<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Dec<'a> {
    pub fn new(bytes: &'a [u8]) -> Dec<'a> {
        Dec { bytes, at: 0 }
    }

    /// The bytes not read yet.
    pub fn remaining(&self) -> usize {
        self.bytes.len() - self.at
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], Malformed> {
        if self.remaining() < n {
            return Err(Malformed::new("the payload ends inside a field"));
        }
        let out = &self.bytes[self.at..self.at + n];
        self.at += n;
        Ok(out)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], Malformed> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    pub fn u8(&mut self) -> Result<u8, Malformed> {
        Ok(self.take(1)?[0])
    }

    pub fn bool(&mut self) -> Result<bool, Malformed> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Malformed::new("a flag other than 0 or 1")),
        }
    }

    pub fn u32(&mut self) -> Result<u32, Malformed> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub fn i32(&mut self) -> Result<i32, Malformed> {
        Ok(i32::from_le_bytes(self.array()?))
    }

    pub fn u64(&mut self) -> Result<u64, Malformed> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    pub fn u128(&mut self) -> Result<u128, Malformed> {
        Ok(u128::from_le_bytes(self.array()?))
    }

    pub fn i128(&mut self) -> Result<i128, Malformed> {
        Ok(i128::from_le_bytes(self.array()?))
    }

    /// A `u64` that indexes local memory.
    pub fn usize(&mut self) -> Result<usize, Malformed> {
        usize::try_from(self.u64()?).map_err(|_| Malformed::new("an index above the address space"))
    }

    /// The length of a sequence whose items take at least `item` bytes each. A length the remaining bytes cannot
    /// hold is refused before anything is allocated for it.
    pub fn count(&mut self, item: usize) -> Result<usize, Malformed> {
        let n = self.u64()?;
        if n > (self.remaining() / item.max(1)) as u64 {
            return Err(Malformed::new("a length beyond the payload"));
        }
        Ok(n as usize)
    }

    pub fn bytes(&mut self) -> Result<&'a [u8], Malformed> {
        let n = self.count(1)?;
        self.take(n)
    }

    pub fn text(&mut self) -> Result<&'a str, Malformed> {
        std::str::from_utf8(self.bytes()?).map_err(|_| Malformed::new("a text that is not UTF-8"))
    }

    /// Refuses trailing bytes.
    pub fn end(&self) -> Result<(), Malformed> {
        if self.remaining() == 0 {
            Ok(())
        } else {
            Err(Malformed::new("trailing bytes after the message"))
        }
    }
}
