//! Readers for the retained asset containers, written here because no external crate may be added (RTE.70).
//!
//! - [`crc32`]: the CRC-32 of ZIP and gzip (ISO 3309, reflected polynomial `0xEDB88320`).
//! - [`inflate`]: DEFLATE (RFC 1951) with stored, fixed and dynamic blocks.
//! - [`gunzip`]: gzip members (RFC 1952). Members follow each other, and zero bytes between them are skipped, as
//!   Python's `gzip.decompress` does.
//! - [`Zip`]: a ZIP archive as NumPy's `savez` writes it, with ZIP64 extra fields in the local headers.
//! - [`read_npy`]: an NPY array (format versions 1 and 2) of little-endian `int32` or `int16`.
//! - [`read_npy_raw`]: the same container with `<f8` allowed too, its payload left undecoded (the jumbling crate reads
//!   binary64 bit patterns as exact dyadic rationals).
//!
//! Every reader checks what its container declares: signatures, CRC-32 values, sizes and lengths. Nothing is
//! repaired, and every output is bounded by a caller's limit. Error messages never quote the input.

use std::fmt;

/// Why a container was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveError(pub &'static str);

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for ArchiveError {}

const CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
};

/// The CRC-32 of `data`.
pub fn crc32(data: &[u8]) -> u32 {
    let mut c = !0u32;
    for &byte in data {
        c = CRC_TABLE[((c ^ byte as u32) & 0xff) as usize] ^ (c >> 8);
    }
    !c
}

fn u16_at(data: &[u8], at: usize) -> Result<u16, ArchiveError> {
    let bytes = data.get(at..at + 2).ok_or(ArchiveError("Truncated container"))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn u32_at(data: &[u8], at: usize) -> Result<u32, ArchiveError> {
    let bytes = data.get(at..at + 4).ok_or(ArchiveError("Truncated container"))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn u64_at(data: &[u8], at: usize) -> Result<u64, ArchiveError> {
    let bytes = data.get(at..at + 8).ok_or(ArchiveError("Truncated container"))?;
    let mut word = [0u8; 8];
    word.copy_from_slice(bytes);
    Ok(u64::from_le_bytes(word))
}

// ---------------------------------------------------------------------------------------------------------------
// DEFLATE

const FAST_BITS: u32 = 10;
const LENGTH_BASE: [u16; 29] =
    [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LENGTH_EXTRA: [u8; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145,
    8193, 12289, 16385, 24577,
];
const DISTANCE_EXTRA: [u8; 30] =
    [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];
const CODE_LENGTH_ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    buffer: u64,
    count: u32,
}

impl Bits<'_> {
    fn refill(&mut self) {
        while self.count <= 56 && self.pos < self.data.len() {
            self.buffer |= (self.data[self.pos] as u64) << self.count;
            self.pos += 1;
            self.count += 8;
        }
    }

    fn bits(&mut self, n: u32) -> Result<u32, ArchiveError> {
        if n == 0 {
            return Ok(0);
        }
        if self.count < n {
            self.refill();
            if self.count < n {
                return Err(ArchiveError("Truncated DEFLATE stream"));
            }
        }
        let value = (self.buffer & ((1u64 << n) - 1)) as u32;
        self.buffer >>= n;
        self.count -= n;
        Ok(value)
    }

    /// Drops the bits up to the next byte boundary and returns the position of the next unread byte.
    fn align(&mut self) -> usize {
        let partial = self.count % 8;
        self.buffer >>= partial;
        self.count -= partial;
        self.pos - (self.count / 8) as usize
    }

    fn decode(&mut self, code: &Huffman) -> Result<u16, ArchiveError> {
        if self.count < 15 {
            self.refill();
        }
        let entry = code.fast[(self.buffer & ((1u64 << FAST_BITS) - 1)) as usize];
        let length = entry >> 16;
        if entry != 0 && length <= self.count {
            self.buffer >>= length;
            self.count -= length;
            return Ok(entry as u16);
        }
        // Canonical decoding one bit at a time, for codes longer than the table and at the end of the input.
        let (mut value, mut first, mut index) = (0i32, 0i32, 0i32);
        for len in 1..16 {
            value |= self.bits(1)? as i32;
            let count = code.counts[len] as i32;
            if value - first < count {
                return Ok(code.symbols[(index + value - first) as usize]);
            }
            index += count;
            first = (first + count) << 1;
            value <<= 1;
        }
        Err(ArchiveError("Invalid Huffman code in DEFLATE stream"))
    }
}

struct Huffman {
    counts: [u16; 16],
    symbols: Vec<u16>,
    fast: Vec<u32>,
}

impl Huffman {
    /// Builds the canonical code for `lengths` (each at most 15). Returns the code and whether it is complete.
    fn new(lengths: &[u8]) -> Result<(Huffman, bool), ArchiveError> {
        let mut counts = [0u16; 16];
        for &length in lengths {
            counts[length as usize] += 1;
        }
        counts[0] = 0;
        let mut left = 1i32;
        for &count in &counts[1..] {
            left = (left << 1) - count as i32;
            if left < 0 {
                return Err(ArchiveError("Over-subscribed Huffman code in DEFLATE stream"));
            }
        }
        let mut offsets = [0usize; 16];
        for len in 1..15 {
            offsets[len + 1] = offsets[len] + counts[len] as usize;
        }
        let mut symbols = vec![0u16; counts.iter().map(|&c| c as usize).sum()];
        for (symbol, &length) in lengths.iter().enumerate() {
            if length != 0 {
                symbols[offsets[length as usize]] = symbol as u16;
                offsets[length as usize] += 1;
            }
        }
        let mut fast = vec![0u32; 1 << FAST_BITS];
        let (mut code, mut index) = (0u32, 0usize);
        for len in 1..=FAST_BITS {
            for _ in 0..counts[len as usize] {
                let reversed = code.reverse_bits() >> (32 - len);
                let mut slot = reversed as usize;
                while slot < fast.len() {
                    fast[slot] = (len << 16) | symbols[index] as u32;
                    slot += 1 << len;
                }
                code += 1;
                index += 1;
            }
            code <<= 1;
        }
        Ok((Huffman { counts, symbols, fast }, left == 0))
    }

    /// An incomplete code is accepted only when it has no code at all or a single code of length 1 (RFC 1951
    /// section 3.2.7); decoding a symbol of an empty code fails.
    fn checked(lengths: &[u8]) -> Result<Huffman, ArchiveError> {
        let (code, complete) = Huffman::new(lengths)?;
        let used = lengths.iter().filter(|&&length| length != 0).count();
        if !complete && used != 0 && !(used == 1 && code.counts[1] == 1) {
            return Err(ArchiveError("Incomplete Huffman code in DEFLATE stream"));
        }
        Ok(code)
    }
}

/// Inflates one raw DEFLATE stream at the start of `data`. Returns the output and the number of input bytes the
/// stream used, up to its final byte boundary. The output may hold at most `limit` bytes.
pub fn inflate(data: &[u8], limit: usize) -> Result<(Vec<u8>, usize), ArchiveError> {
    let mut bits = Bits { data, pos: 0, buffer: 0, count: 0 };
    let mut out = Vec::new();
    loop {
        let last = bits.bits(1)? == 1;
        match bits.bits(2)? {
            0 => {
                let start = bits.align();
                let length = u16_at(data, start)?;
                let complement = u16_at(data, start + 2)?;
                if length != !complement {
                    return Err(ArchiveError("Stored block length check failed in DEFLATE stream"));
                }
                let begin = start + 4;
                let block = data.get(begin..begin + length as usize).ok_or(ArchiveError("Truncated DEFLATE stream"))?;
                if out.len() + block.len() > limit {
                    return Err(ArchiveError("DEFLATE output exceeds its limit"));
                }
                out.extend_from_slice(block);
                bits = Bits { data, pos: begin + block.len(), buffer: 0, count: 0 };
            }
            1 => {
                let mut lengths = [0u8; 288];
                lengths[..144].fill(8);
                lengths[144..256].fill(9);
                lengths[256..280].fill(7);
                lengths[280..].fill(8);
                let (literal, _) = Huffman::new(&lengths)?;
                let (distance, _) = Huffman::new(&[5u8; 32])?;
                block(&mut bits, &literal, &distance, &mut out, limit)?;
            }
            2 => {
                let literals = bits.bits(5)? as usize + 257;
                let distances = bits.bits(5)? as usize + 1;
                let code_lengths = bits.bits(4)? as usize + 4;
                if literals > 286 || distances > 30 {
                    return Err(ArchiveError("Too many length or distance codes in DEFLATE stream"));
                }
                let mut lengths = [0u8; 19];
                for &position in &CODE_LENGTH_ORDER[..code_lengths] {
                    lengths[position] = bits.bits(3)? as u8;
                }
                let (length_code, complete) = Huffman::new(&lengths)?;
                if !complete {
                    return Err(ArchiveError("Incomplete code length code in DEFLATE stream"));
                }
                let total = literals + distances;
                let mut lengths = vec![0u8; total];
                let mut i = 0;
                while i < total {
                    let symbol = bits.decode(&length_code)?;
                    if symbol < 16 {
                        lengths[i] = symbol as u8;
                        i += 1;
                        continue;
                    }
                    let (value, repeat) = match symbol {
                        16 => {
                            if i == 0 {
                                return Err(ArchiveError("Repeat without a previous length in DEFLATE stream"));
                            }
                            (lengths[i - 1], 3 + bits.bits(2)? as usize)
                        }
                        17 => (0, 3 + bits.bits(3)? as usize),
                        _ => (0, 11 + bits.bits(7)? as usize),
                    };
                    if i + repeat > total {
                        return Err(ArchiveError("Too many code lengths in DEFLATE stream"));
                    }
                    lengths[i..i + repeat].fill(value);
                    i += repeat;
                }
                if lengths[256] == 0 {
                    return Err(ArchiveError("Missing end-of-block code in DEFLATE stream"));
                }
                let literal = Huffman::checked(&lengths[..literals])?;
                let distance = Huffman::checked(&lengths[literals..])?;
                block(&mut bits, &literal, &distance, &mut out, limit)?;
            }
            _ => return Err(ArchiveError("Invalid block type in DEFLATE stream")),
        }
        if last {
            let used = bits.align();
            return Ok((out, used));
        }
    }
}

fn block(
    bits: &mut Bits<'_>,
    literal: &Huffman,
    distance: &Huffman,
    out: &mut Vec<u8>,
    limit: usize,
) -> Result<(), ArchiveError> {
    loop {
        let symbol = bits.decode(literal)? as usize;
        if symbol < 256 {
            if out.len() >= limit {
                return Err(ArchiveError("DEFLATE output exceeds its limit"));
            }
            out.push(symbol as u8);
            continue;
        }
        if symbol == 256 {
            return Ok(());
        }
        let index = symbol - 257;
        if index >= LENGTH_BASE.len() {
            return Err(ArchiveError("Invalid length code in DEFLATE stream"));
        }
        let length = LENGTH_BASE[index] as usize + bits.bits(LENGTH_EXTRA[index] as u32)? as usize;
        let code = bits.decode(distance)? as usize;
        if code >= DISTANCE_BASE.len() {
            return Err(ArchiveError("Invalid distance code in DEFLATE stream"));
        }
        let back = DISTANCE_BASE[code] as usize + bits.bits(DISTANCE_EXTRA[code] as u32)? as usize;
        if back > out.len() {
            return Err(ArchiveError("Distance before the start of the DEFLATE output"));
        }
        if out.len() + length > limit {
            return Err(ArchiveError("DEFLATE output exceeds its limit"));
        }
        let start = out.len() - back;
        if back >= length {
            out.extend_from_within(start..start + length);
        } else {
            for k in 0..length {
                let byte = out[start + k];
                out.push(byte);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// gzip

/// Decompresses gzip members. Each member's CRC-32 and length (ISIZE) are checked; a header CRC, when present, too.
/// The output may hold at most `limit` bytes.
pub fn gunzip(data: &[u8], limit: usize) -> Result<Vec<u8>, ArchiveError> {
    let mut out = Vec::new();
    let mut rest = data;
    loop {
        if rest.len() < 10 {
            return Err(ArchiveError("Truncated gzip header"));
        }
        if rest[0] != 0x1f || rest[1] != 0x8b {
            return Err(ArchiveError("Not a gzip member"));
        }
        if rest[2] != 8 {
            return Err(ArchiveError("Unknown gzip compression method"));
        }
        let flags = rest[3];
        if flags & 0xe0 != 0 {
            return Err(ArchiveError("Reserved gzip flags are set"));
        }
        let mut at = 10;
        if flags & 0x04 != 0 {
            at += 2 + u16_at(rest, at)? as usize;
        }
        for flag in [0x08, 0x10] {
            if flags & flag != 0 {
                let end = rest.get(at..).and_then(|tail| tail.iter().position(|&b| b == 0));
                at += end.ok_or(ArchiveError("Truncated gzip header"))? + 1;
            }
        }
        if flags & 0x02 != 0 {
            let stored = u16_at(rest, at)?;
            if crc32(&rest[..at]) as u16 != stored {
                return Err(ArchiveError("gzip header CRC check failed"));
            }
            at += 2;
        }
        let body = rest.get(at..).ok_or(ArchiveError("Truncated gzip header"))?;
        let (member, used) = inflate(body, limit - out.len())?;
        let trailer = at + used;
        let crc = u32_at(rest, trailer).map_err(|_| ArchiveError("Truncated gzip trailer"))?;
        let size = u32_at(rest, trailer + 4).map_err(|_| ArchiveError("Truncated gzip trailer"))?;
        if crc != crc32(&member) {
            return Err(ArchiveError("gzip CRC check failed"));
        }
        if size != member.len() as u32 {
            return Err(ArchiveError("gzip length check failed"));
        }
        out.extend_from_slice(&member);
        rest = &rest[trailer + 8..];
        let zeros = rest.iter().take_while(|&&b| b == 0).count();
        rest = &rest[zeros..];
        if rest.is_empty() {
            return Ok(out);
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// ZIP

struct Entry {
    name: String,
    flags: u16,
    method: u16,
    crc: u32,
    compressed: u64,
    size: u64,
    offset: u64,
}

/// A ZIP archive's central directory over its bytes.
pub struct Zip<'a> {
    data: &'a [u8],
    directory: usize,
    entries: Vec<Entry>,
}

impl<'a> Zip<'a> {
    /// Reads the end record and the central directory. ZIP64 end records, multi-disk archives, encryption and
    /// non-ASCII names are refused.
    pub fn parse(data: &'a [u8]) -> Result<Zip<'a>, ArchiveError> {
        let last = data.len().checked_sub(22).ok_or(ArchiveError("Truncated ZIP archive"))?;
        let mut end = None;
        for at in (last.saturating_sub(65_535)..=last).rev() {
            if data[at..at + 4] == [0x50, 0x4b, 0x05, 0x06] && at + 22 + u16_at(data, at + 20)? as usize == data.len() {
                end = Some(at);
                break;
            }
        }
        let end = end.ok_or(ArchiveError("Missing ZIP end record"))?;
        let (disk, directory_disk) = (u16_at(data, end + 4)?, u16_at(data, end + 6)?);
        let (on_disk, count) = (u16_at(data, end + 8)?, u16_at(data, end + 10)?);
        let (size, directory) = (u32_at(data, end + 12)?, u32_at(data, end + 16)?);
        if disk != 0 || directory_disk != 0 || on_disk != count {
            return Err(ArchiveError("Multi-disk ZIP archives are not read"));
        }
        if count == 0xffff || size == 0xffff_ffff || directory == 0xffff_ffff {
            return Err(ArchiveError("ZIP64 end records are not read"));
        }
        let (directory, size) = (directory as usize, size as usize);
        if directory + size != end {
            return Err(ArchiveError("ZIP central directory does not end at the end record"));
        }
        let mut entries: Vec<Entry> = Vec::new();
        let mut at = directory;
        for _ in 0..count {
            if u32_at(data, at)? != 0x0201_4b50 {
                return Err(ArchiveError("Bad ZIP central directory signature"));
            }
            let flags = u16_at(data, at + 8)?;
            let method = u16_at(data, at + 10)?;
            let crc = u32_at(data, at + 16)?;
            let mut compressed = u32_at(data, at + 20)? as u64;
            let mut size = u32_at(data, at + 24)? as u64;
            let name_length = u16_at(data, at + 28)? as usize;
            let extra_length = u16_at(data, at + 30)? as usize;
            let comment_length = u16_at(data, at + 32)? as usize;
            let mut offset = u32_at(data, at + 42)? as u64;
            let name = data.get(at + 46..at + 46 + name_length).ok_or(ArchiveError("Truncated ZIP archive"))?;
            if !name.is_ascii() {
                return Err(ArchiveError("Non-ASCII ZIP member name"));
            }
            let extra_start = at + 46 + name_length;
            let extra =
                data.get(extra_start..extra_start + extra_length).ok_or(ArchiveError("Truncated ZIP archive"))?;
            if size == 0xffff_ffff || compressed == 0xffff_ffff || offset == 0xffff_ffff {
                let mut fields = zip64_extra(extra)?.into_iter();
                for value in [&mut size, &mut compressed, &mut offset] {
                    if *value == 0xffff_ffff {
                        *value = fields.next().ok_or(ArchiveError("Short ZIP64 extra field"))?;
                    }
                }
            }
            let name = String::from_utf8(name.to_vec()).map_err(|_| ArchiveError("Non-ASCII ZIP member name"))?;
            if entries.iter().any(|entry| entry.name == name) {
                return Err(ArchiveError("Duplicate ZIP member name"));
            }
            entries.push(Entry { name, flags, method, crc, compressed, size, offset });
            at = extra_start + extra_length + comment_length;
        }
        if at != end {
            return Err(ArchiveError("ZIP central directory size disagrees with its entries"));
        }
        Ok(Zip { data, directory, entries })
    }

    /// The member names in directory order.
    pub fn names(&self) -> Vec<&str> {
        self.entries.iter().map(|entry| entry.name.as_str()).collect()
    }

    /// Reads one member, checks its local header against the directory, its size and its CRC-32.
    pub fn read(&self, name: &str, limit: usize) -> Result<Vec<u8>, ArchiveError> {
        let entry = self.entries.iter().find(|entry| entry.name == name).ok_or(ArchiveError("Missing ZIP member"))?;
        // Bits 1 and 2 are DEFLATE options and bit 11 marks UTF-8 names; any other flag is refused.
        if entry.flags & !0x0806 != 0 {
            return Err(ArchiveError("Unsupported ZIP member flags"));
        }
        let data = self.data;
        let at = usize::try_from(entry.offset).map_err(|_| ArchiveError("ZIP member offset out of range"))?;
        if u32_at(data, at)? != 0x0403_4b50 {
            return Err(ArchiveError("Bad ZIP local header signature"));
        }
        let flags = u16_at(data, at + 6)?;
        let method = u16_at(data, at + 8)?;
        let crc = u32_at(data, at + 14)?;
        let compressed = u32_at(data, at + 18)?;
        let size = u32_at(data, at + 22)?;
        let name_length = u16_at(data, at + 26)? as usize;
        let extra_length = u16_at(data, at + 28)? as usize;
        let local_name = data.get(at + 30..at + 30 + name_length).ok_or(ArchiveError("Truncated ZIP archive"))?;
        let extra_start = at + 30 + name_length;
        let extra = data.get(extra_start..extra_start + extra_length).ok_or(ArchiveError("Truncated ZIP archive"))?;
        if local_name != entry.name.as_bytes() || flags != entry.flags || method != entry.method || crc != entry.crc {
            return Err(ArchiveError("ZIP local header disagrees with the central directory"));
        }
        let (local_size, local_compressed) = if size == 0xffff_ffff && compressed == 0xffff_ffff {
            let fields = zip64_extra(extra)?;
            if fields.len() < 2 {
                return Err(ArchiveError("Short ZIP64 extra field"));
            }
            (fields[0], fields[1])
        } else {
            (size as u64, compressed as u64)
        };
        if local_size != entry.size || local_compressed != entry.compressed {
            return Err(ArchiveError("ZIP local sizes disagree with the central directory"));
        }
        let begin = extra_start + extra_length;
        let length = usize::try_from(entry.compressed).map_err(|_| ArchiveError("ZIP member too large"))?;
        let end = begin.checked_add(length).ok_or(ArchiveError("ZIP member too large"))?;
        if end > self.directory {
            return Err(ArchiveError("ZIP member data overlaps the central directory"));
        }
        let body = &data[begin..end];
        let expected = usize::try_from(entry.size).map_err(|_| ArchiveError("ZIP member too large"))?;
        if expected > limit {
            return Err(ArchiveError("ZIP member exceeds its limit"));
        }
        let out = match entry.method {
            0 => body.to_vec(),
            8 => {
                let (out, used) = inflate(body, expected)?;
                if used != body.len() {
                    return Err(ArchiveError("ZIP member data continues after its DEFLATE stream"));
                }
                out
            }
            _ => return Err(ArchiveError("Unsupported ZIP compression method")),
        };
        if out.len() != expected {
            return Err(ArchiveError("ZIP member length check failed"));
        }
        if crc32(&out) != entry.crc {
            return Err(ArchiveError("ZIP member CRC check failed"));
        }
        Ok(out)
    }
}

/// The 64-bit values of the ZIP64 extended information field (header ID 1) in an extra block.
fn zip64_extra(extra: &[u8]) -> Result<Vec<u64>, ArchiveError> {
    let mut at = 0;
    while at + 4 <= extra.len() {
        let id = u16_at(extra, at)?;
        let size = u16_at(extra, at + 2)? as usize;
        let body = extra.get(at + 4..at + 4 + size).ok_or(ArchiveError("Truncated ZIP extra field"))?;
        if id == 1 {
            if size % 8 != 0 {
                return Err(ArchiveError("Malformed ZIP64 extra field"));
            }
            return (0..size / 8).map(|i| u64_at(body, i * 8)).collect();
        }
        at += 4 + size;
    }
    Err(ArchiveError("Missing ZIP64 extra field"))
}

// ---------------------------------------------------------------------------------------------------------------
// NPY

/// A one- or more-dimensional NPY array of `<i4` or `<i2` values, widened to `i32`, in C order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Npy {
    /// `"<i4"` or `"<i2"`.
    pub descr: String,
    pub shape: Vec<usize>,
    pub values: Vec<i32>,
}

/// A C-order NPY array whose little-endian payload the caller decodes itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpyRaw {
    /// `"<i4"`, `"<i2"` or `"<f8"`.
    pub descr: String,
    pub shape: Vec<usize>,
    /// The payload bytes, `width` bytes per value in C order.
    pub data: Vec<u8>,
}

/// Reads an NPY file of format version 1.0 or 2.0 holding `<i4`, `<i2` or `<f8` values in C order, without decoding
/// them. `<f8` payloads are binary64 bit patterns: the jumbling crate reads them as exact dyadic rationals, so no
/// float type is involved (CT-KN2-05).
pub fn read_npy_raw(bytes: &[u8]) -> Result<NpyRaw, ArchiveError> {
    let (descr, shape, data) = npy_parts(bytes, &[("<i4", 4), ("<i2", 2), ("<f8", 8)])?;
    Ok(NpyRaw { descr, shape, data: data.to_vec() })
}

/// Reads an NPY file of format version 1.0 or 2.0 holding little-endian `int32` or `int16` values in C order.
pub fn read_npy(bytes: &[u8]) -> Result<Npy, ArchiveError> {
    let (descr, shape, data) = npy_parts(bytes, &[("<i4", 4), ("<i2", 2)])?;
    let values = if descr == "<i4" {
        data.chunks_exact(4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect()
    } else {
        data.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]]) as i32).collect()
    };
    Ok(Npy { descr, shape, values })
}

/// The dtype, shape and payload of an NPY file whose dtype is one of `accepted` (name and byte width).
fn npy_parts<'a>(bytes: &'a [u8], accepted: &[(&str, usize)]) -> Result<(String, Vec<usize>, &'a [u8]), ArchiveError> {
    if bytes.len() < 10 || &bytes[..6] != b"\x93NUMPY" {
        return Err(ArchiveError("Not an NPY array"));
    }
    let (length, start) = match (bytes[6], bytes[7]) {
        (1, 0) => (u16_at(bytes, 8)? as usize, 10),
        (2, 0) => (u32_at(bytes, 8)? as usize, 12),
        _ => return Err(ArchiveError("Unsupported NPY format version")),
    };
    let header = bytes.get(start..start + length).ok_or(ArchiveError("Truncated NPY header"))?;
    if !header.is_ascii() || header.last() != Some(&b'\n') {
        return Err(ArchiveError("Malformed NPY header"));
    }
    let header = std::str::from_utf8(header).map_err(|_| ArchiveError("Malformed NPY header"))?;
    let (descr, fortran, shape) = npy_header(header)?;
    if fortran {
        return Err(ArchiveError("Fortran-order NPY arrays are not read"));
    }
    let width = match accepted.iter().find(|(name, _)| *name == descr) {
        Some(&(_, width)) => width,
        None => return Err(ArchiveError("Unsupported NPY dtype")),
    };
    let count = shape
        .iter()
        .try_fold(1usize, |total, &extent| total.checked_mul(extent))
        .ok_or(ArchiveError("NPY shape overflows"))?;
    let data = &bytes[start + length..];
    if Some(data.len()) != count.checked_mul(width) {
        return Err(ArchiveError("NPY data length disagrees with its shape"));
    }
    Ok((descr, shape, data))
}

/// Parses the header dictionary NumPy writes: `{'descr': ..., 'fortran_order': ..., 'shape': (...), }`.
fn npy_header(text: &str) -> Result<(String, bool, Vec<usize>), ArchiveError> {
    const MALFORMED: ArchiveError = ArchiveError("Malformed NPY header");
    let bytes = text.as_bytes();
    let mut at = 0;
    let skip = |at: &mut usize| {
        while *at < bytes.len() && bytes[*at] == b' ' {
            *at += 1;
        }
    };
    let quoted = |at: &mut usize| -> Result<String, ArchiveError> {
        if bytes.get(*at) != Some(&b'\'') {
            return Err(MALFORMED);
        }
        let end = text[*at + 1..].find('\'').ok_or(MALFORMED)? + *at + 1;
        let value = text[*at + 1..end].to_owned();
        if value.contains('\\') {
            return Err(MALFORMED);
        }
        *at = end + 1;
        Ok(value)
    };
    let (mut descr, mut fortran, mut shape) = (None, None, None);
    skip(&mut at);
    if bytes.get(at) != Some(&b'{') {
        return Err(MALFORMED);
    }
    at += 1;
    loop {
        skip(&mut at);
        if bytes.get(at) == Some(&b'}') {
            at += 1;
            break;
        }
        let key = quoted(&mut at)?;
        skip(&mut at);
        if bytes.get(at) != Some(&b':') {
            return Err(MALFORMED);
        }
        at += 1;
        skip(&mut at);
        let duplicate = match key.as_str() {
            "descr" => descr.replace(quoted(&mut at)?).is_some(),
            "fortran_order" => {
                let value = if text[at..].starts_with("True") {
                    true
                } else if text[at..].starts_with("False") {
                    false
                } else {
                    return Err(MALFORMED);
                };
                at += if value { 4 } else { 5 };
                fortran.replace(value).is_some()
            }
            "shape" => {
                if bytes.get(at) != Some(&b'(') {
                    return Err(MALFORMED);
                }
                let end = text[at..].find(')').ok_or(MALFORMED)? + at;
                let mut extents = Vec::new();
                let inner = &text[at + 1..end];
                let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
                for (i, part) in parts.iter().enumerate() {
                    if part.is_empty() {
                        // Only "()", "(n,)" and a trailing comma leave an empty part, and only at the end.
                        if i + 1 != parts.len() || (i == 0 && !inner.trim().is_empty()) {
                            return Err(MALFORMED);
                        }
                        continue;
                    }
                    if !part.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(MALFORMED);
                    }
                    extents.push(part.parse::<usize>().map_err(|_| MALFORMED)?);
                }
                if extents.len() == 1 && !inner.trim_end().ends_with(',') {
                    return Err(MALFORMED);
                }
                at = end + 1;
                shape.replace(extents).is_some()
            }
            _ => return Err(MALFORMED),
        };
        if duplicate {
            return Err(MALFORMED);
        }
        skip(&mut at);
        match bytes.get(at) {
            Some(b',') => at += 1,
            Some(b'}') => {}
            _ => return Err(MALFORMED),
        }
    }
    if !text[at..].bytes().all(|b| b == b' ' || b == b'\n') {
        return Err(MALFORMED);
    }
    match (descr, fortran, shape) {
        (Some(descr), Some(fortran), Some(shape)) => Ok((descr, fortran, shape)),
        _ => Err(MALFORMED),
    }
}
