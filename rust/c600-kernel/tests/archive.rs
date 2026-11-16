//! Vectors for the crate's own container readers ([`c600_kernel::archive`]). The DEFLATE, gzip and ZIP bytes below
//! were written by Python's `zlib`, `gzip` and `zipfile` modules (the ZIP's second member with a ZIP64 extra field in
//! its local header, as NumPy's `savez` writes it); the NPY arrays are built here.

use c600_kernel::archive::{crc32, gunzip, inflate, read_npy, ArchiveError, Zip};

const STORED: &[u8] = b"\
    \x01\x17\x00\xe8\xff\x73\x74\x6f\x72\x65\x64\x20\x62\x6c\x6f\x63\x6b\x20\x30\x31\x32\x33\x34\x35\
    \x36\x37\x38\x39\
";
const FIXED: &[u8] = b"\
    \x4b\xcb\xac\x48\x4d\x51\x48\x43\x22\x33\x4a\xd3\xd2\x72\x13\xf3\xd0\x69\x00\
";
const DYNAMIC: &[u8] = b"\
    \xa5\xd5\x59\x0e\x82\x30\x10\xc6\xf1\x77\x4f\xd1\x23\xc8\x2c\x6e\xb7\x41\x16\x45\x51\x9f\xbd\xbd\
    \x22\x2d\x21\x64\xe8\x4c\x6b\x5c\x93\xfe\xfb\xe2\x97\x5f\xec\xbb\x67\xe3\xb6\xee\xd5\xba\xd2\xd5\
    \xef\x67\xf9\xe8\x2a\x77\xee\x5f\xd5\xfd\xe4\xca\x73\x55\x37\xed\xe5\xda\xdd\x36\xfd\x50\x15\x62\
    \x35\x45\x8b\x1a\xc4\x3a\x34\xcb\xcf\xf1\x0e\x8a\x77\x7c\x22\x7e\x8c\xf7\x48\xbc\x37\x2f\x58\x2c\
    \x7e\xc1\xac\xda\x89\xd5\x70\x1e\x5e\x63\xb7\x17\xbb\xef\xf1\xf4\x1c\xbb\x83\xd8\x85\xd3\xa3\x78\
    \x7a\x0b\xbf\xb6\x36\x8a\xf4\xcd\x5f\xb5\x2e\xb5\xb6\x5d\x11\x1f\xcf\x47\x96\xb5\x7c\x1a\x1b\x68\
    \xfe\xee\x73\x6d\xad\xe5\x6e\xc5\xfa\x70\x3e\x88\x2d\xe6\x93\xb5\xb1\x86\x87\x4f\xd6\x16\x0b\x9b\
    \x81\x0d\x12\x24\x4a\xca\xa1\x04\xb9\x96\x40\xc7\x04\x36\x4d\x60\xe5\x04\x56\x4f\x10\x07\x05\x71\
    \x51\x98\x2f\x0a\xff\x15\x85\x16\x51\x68\x17\x85\x69\xa2\x30\x55\x14\x6a\xa2\x50\x17\x85\xba\x28\
    \xd4\x44\x91\x4d\x14\xa5\x89\xa2\x1c\x51\x94\xfd\xef\xa4\x8b\x22\x9b\x28\xb2\x8a\x22\xab\x28\x8a\
    \x8b\xa2\xb8\x28\xce\x17\xc5\xff\x8a\x62\x8b\x28\xb6\x8b\xe2\x34\x51\x9c\x2a\x8a\x35\x51\xac\x8b\
    \x62\x5d\x14\x47\x44\x7d\x00\
";
const GZIP_NAMED: &[u8] = b"\
    \x1f\x8b\x08\x08\x00\x00\x00\x00\x00\xff\x61\x2e\x74\x78\x74\x00\xcb\x48\xcd\xc9\xc9\x57\x48\xaf\
    \xca\x2c\xe0\x02\x00\x39\x7c\x63\x56\x0b\x00\x00\x00\
";
const GZIP_PLAIN: &[u8] = b"\
    \x1f\x8b\x08\x00\x00\x00\x00\x00\x00\xff\x2b\x4e\x4d\xce\xcf\x4b\x51\xc8\x4d\xcd\x4d\x4a\x2d\xe2\
    \x02\x00\x36\x18\x4b\x0e\x0e\x00\x00\x00\
";
const ZIP: &[u8] = b"\
    \x50\x4b\x03\x04\x14\x00\x00\x00\x00\x00\x00\x00\x21\x50\xe5\xca\x8a\x4f\x8c\x00\x00\x00\x8c\x00\
    \x00\x00\x05\x00\x00\x00\x61\x2e\x6e\x70\x79\x93\x4e\x55\x4d\x50\x59\x01\x00\x76\x00\x7b\x27\x64\
    \x65\x73\x63\x72\x27\x3a\x20\x27\x3c\x69\x34\x27\x2c\x20\x27\x66\x6f\x72\x74\x72\x61\x6e\x5f\x6f\
    \x72\x64\x65\x72\x27\x3a\x20\x46\x61\x6c\x73\x65\x2c\x20\x27\x73\x68\x61\x70\x65\x27\x3a\x20\x28\
    \x33\x2c\x29\x2c\x20\x7d\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\
    \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\
    \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x0a\x01\x00\x00\x00\xfe\
    \xff\xff\xff\xe0\x93\x04\x00\x50\x4b\x03\x04\x2d\x00\x00\x00\x08\x00\x00\x00\x21\x50\xa1\x93\xd3\
    \x9f\xff\xff\xff\xff\xff\xff\xff\xff\x05\x00\x14\x00\x62\x2e\x6e\x70\x79\x01\x00\x10\x00\x84\x00\
    \x00\x00\x00\x00\x00\x00\x4b\x00\x00\x00\x00\x00\x00\x00\x9b\xec\x17\xea\x1b\x10\xc9\xc8\x50\xc6\
    \x50\xad\x9e\x92\x5a\x9c\x5c\xa4\x6e\xa5\xa0\x6e\x93\x69\xa4\xae\xa3\xa0\x9e\x96\x5f\x54\x52\x94\
    \x98\x17\x9f\x5f\x94\x92\x0a\x12\x77\x4b\xcc\x29\x4e\xd5\x51\x50\x2f\xce\x48\x2c\x48\x55\xb7\x52\
    \xd0\x30\xd2\xd1\xd4\x51\xa8\x55\xa0\x00\x70\xfd\xff\xcf\xce\x00\x00\x50\x4b\x01\x02\x14\x00\x14\
    \x00\x00\x00\x00\x00\x00\x00\x21\x50\xe5\xca\x8a\x4f\x8c\x00\x00\x00\x8c\x00\x00\x00\x05\x00\x00\
    \x00\x00\x00\x00\x00\x00\x00\x00\x00\x80\x01\x00\x00\x00\x00\x61\x2e\x6e\x70\x79\x50\x4b\x01\x02\
    \x2d\x00\x2d\x00\x00\x00\x08\x00\x00\x00\x21\x50\xa1\x93\xd3\x9f\x4b\x00\x00\x00\x84\x00\x00\x00\
    \x05\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x80\x01\xaf\x00\x00\x00\x62\x2e\x6e\x70\x79\x50\
    \x4b\x05\x06\x00\x00\x00\x00\x02\x00\x02\x00\x66\x00\x00\x00\x31\x01\x00\x00\x00\x00\
";

const STORED_TEXT: &[u8] = b"stored block 0123456789";
const FIXED_TEXT: &[u8] = b"fixed fixed fixed huffman huffman huffman";
const LIMIT: usize = 1 << 20;

/// The text of `DYNAMIC`: sixty numbered lines with repeated letters.
fn dynamic_text() -> Vec<u8> {
    let letters = "abcdefghij";
    let mut text = String::new();
    for i in 0..60 {
        text.push_str(&format!("line {i} of a dynamic block: {}\n", letters[i % 10..].repeat(i % 4 + 1)));
    }
    text.into_bytes()
}

fn npy(version: u8, descr: &str, fortran: bool, shape: &str, data: &[u8]) -> Vec<u8> {
    let order = if fortran { "True" } else { "False" };
    let mut header = format!("{{'descr': '{descr}', 'fortran_order': {order}, 'shape': {shape}, }}");
    let prefix = if version == 1 { 10 } else { 12 };
    while (prefix + header.len() + 1) % 64 != 0 {
        header.push(' ');
    }
    header.push('\n');
    let mut out = b"\x93NUMPY".to_vec();
    out.extend_from_slice(&[version, 0]);
    if version == 1 {
        out.extend_from_slice(&(header.len() as u16).to_le_bytes());
    } else {
        out.extend_from_slice(&(header.len() as u32).to_le_bytes());
    }
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(data);
    out
}

#[test]
fn crc32_check_values() {
    assert_eq!(crc32(b""), 0);
    assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    assert_eq!(crc32(&dynamic_text()), 0x0940_2800);
}

#[test]
fn inflate_reads_stored_fixed_and_dynamic_blocks() {
    let text = dynamic_text();
    assert_eq!(text.len(), 2540);
    for (stream, block_type, expected) in [(STORED, 0, STORED_TEXT), (FIXED, 1, FIXED_TEXT), (DYNAMIC, 2, &text[..])] {
        assert_eq!(stream[0] >> 1 & 3, block_type);
        assert_eq!(inflate(stream, LIMIT).unwrap(), (expected.to_vec(), stream.len()));
    }
}

#[test]
fn inflate_stops_at_the_end_of_its_stream() {
    let mut data = DYNAMIC.to_vec();
    data.extend_from_slice(b"trailer");
    assert_eq!(inflate(&data, LIMIT).unwrap(), (dynamic_text(), DYNAMIC.len()));
}

#[test]
fn inflate_refuses_damaged_streams_and_keeps_its_limit() {
    let refused = |data: &[u8], limit: usize| inflate(data, limit).unwrap_err();
    let over = ArchiveError("DEFLATE output exceeds its limit");
    assert_eq!(refused(STORED, STORED_TEXT.len() - 1), over);
    assert_eq!(refused(FIXED, 10), over);
    assert_eq!(refused(DYNAMIC, 100), over);
    assert_eq!(refused(&DYNAMIC[..DYNAMIC.len() - 4], LIMIT), ArchiveError("Truncated DEFLATE stream"));
    assert_eq!(refused(&[0x07], LIMIT), ArchiveError("Invalid block type in DEFLATE stream"));
    assert_eq!(
        refused(&[0x01, 0x05, 0x00, 0x00, 0x00], LIMIT),
        ArchiveError("Stored block length check failed in DEFLATE stream")
    );
    assert_eq!(refused(&[0x01, 0x05, 0x00, 0xfa, 0xff, b'a'], LIMIT), ArchiveError("Truncated DEFLATE stream"));
    // A dynamic block header that declares 287 literal and length codes.
    assert_eq!(
        refused(&[0xf5, 0x00, 0x00], LIMIT),
        ArchiveError("Too many length or distance codes in DEFLATE stream")
    );
}

#[test]
fn gunzip_reads_named_members_and_zero_padding() {
    assert_eq!(gunzip(GZIP_NAMED, LIMIT).unwrap(), b"hello gzip\n");
    assert_eq!(gunzip(GZIP_PLAIN, LIMIT).unwrap(), b"second member\n");
    let mut joined = GZIP_NAMED.to_vec();
    joined.extend_from_slice(&[0, 0, 0]);
    joined.extend_from_slice(GZIP_PLAIN);
    assert_eq!(gunzip(&joined, LIMIT).unwrap(), b"hello gzip\nsecond member\n");
}

#[test]
fn gunzip_checks_header_crc_trailer_flags_and_limit() {
    // A header CRC (FHCRC): the low 16 bits of the CRC-32 of the header before it (RFC 1952).
    let mut with_crc = GZIP_PLAIN[..10].to_vec();
    with_crc[3] |= 0x02;
    let header_crc = crc32(&with_crc) as u16;
    with_crc.extend_from_slice(&header_crc.to_le_bytes());
    with_crc.extend_from_slice(&GZIP_PLAIN[10..]);
    assert_eq!(gunzip(&with_crc, LIMIT).unwrap(), b"second member\n");
    let mut wrong = with_crc.clone();
    wrong[10] ^= 1;
    assert_eq!(gunzip(&wrong, LIMIT).unwrap_err(), ArchiveError("gzip header CRC check failed"));

    let n = GZIP_PLAIN.len();
    let damaged = |at: usize| {
        let mut data = GZIP_PLAIN.to_vec();
        data[at] ^= 1;
        gunzip(&data, LIMIT).unwrap_err()
    };
    assert_eq!(damaged(n - 8), ArchiveError("gzip CRC check failed"));
    assert_eq!(damaged(n - 4), ArchiveError("gzip length check failed"));
    assert_eq!(gunzip(&GZIP_PLAIN[..n - 1], LIMIT).unwrap_err(), ArchiveError("Truncated gzip trailer"));
    assert_eq!(gunzip(b"PK\x03\x04 not gzip", LIMIT).unwrap_err(), ArchiveError("Not a gzip member"));
    let mut reserved = GZIP_PLAIN.to_vec();
    reserved[3] = 0x20;
    assert_eq!(gunzip(&reserved, LIMIT).unwrap_err(), ArchiveError("Reserved gzip flags are set"));
    assert_eq!(gunzip(GZIP_PLAIN, 4).unwrap_err(), ArchiveError("DEFLATE output exceeds its limit"));
}

#[test]
fn zip_reads_stored_and_deflated_members() {
    let zip = Zip::parse(ZIP).unwrap();
    assert_eq!(zip.names(), ["a.npy", "b.npy"]);
    let a = read_npy(&zip.read("a.npy", LIMIT).unwrap()).unwrap();
    assert_eq!((a.descr.as_str(), a.shape.as_slice(), a.values.as_slice()), ("<i4", &[3][..], &[1, -2, 300_000][..]));
    let b = read_npy(&zip.read("b.npy", LIMIT).unwrap()).unwrap();
    assert_eq!((b.descr.as_str(), b.shape.as_slice(), b.values.as_slice()), ("<i2", &[2][..], &[-1, 7][..]));
    assert_eq!(zip.read("c.npy", LIMIT).unwrap_err(), ArchiveError("Missing ZIP member"));
    assert_eq!(zip.read("a.npy", 100).unwrap_err(), ArchiveError("ZIP member exceeds its limit"));
}

#[test]
fn zip_refuses_damaged_archives() {
    // The first member's data starts after its 30-byte local header and its 5-byte name; its 128-byte NPY header
    // comes before the values.
    let mut damaged = ZIP.to_vec();
    damaged[30 + 5 + 128] ^= 1;
    assert_eq!(
        Zip::parse(&damaged).unwrap().read("a.npy", LIMIT).unwrap_err(),
        ArchiveError("ZIP member CRC check failed")
    );
    let mut renamed = ZIP.to_vec();
    renamed[30] = b'x';
    assert_eq!(
        Zip::parse(&renamed).unwrap().read("a.npy", LIMIT).unwrap_err(),
        ArchiveError("ZIP local header disagrees with the central directory")
    );
    assert_eq!(Zip::parse(&ZIP[..ZIP.len() - 1]).err(), Some(ArchiveError("Missing ZIP end record")));
}

#[test]
fn npy_reads_versions_one_and_two() {
    let data: Vec<u8> = (-3..3i32).flat_map(i32::to_le_bytes).collect();
    for version in [1, 2] {
        let array = read_npy(&npy(version, "<i4", false, "(2, 3)", &data)).unwrap();
        assert_eq!(array.shape, [2, 3]);
        assert_eq!(array.values, [-3, -2, -1, 0, 1, 2]);
    }
    let scalar = read_npy(&npy(1, "<i2", false, "()", &[0x34, 0x12])).unwrap();
    assert_eq!((scalar.shape.len(), scalar.values.as_slice()), (0, &[0x1234][..]));
}

#[test]
fn npy_refuses_what_it_does_not_read() {
    let four = [0u8; 4];
    let refused = |bytes: Vec<u8>| read_npy(&bytes).unwrap_err().0;
    assert_eq!(refused(npy(1, "<i4", true, "(1,)", &four)), "Fortran-order NPY arrays are not read");
    assert_eq!(refused(npy(1, "<f4", false, "(1,)", &four)), "Unsupported NPY dtype");
    assert_eq!(refused(npy(1, ">i4", false, "(1,)", &four)), "Unsupported NPY dtype");
    assert_eq!(refused(npy(1, "<i4", false, "(2,)", &four)), "NPY data length disagrees with its shape");
    assert_eq!(refused(npy(1, "<i4", false, "(1)", &four)), "Malformed NPY header");
    assert_eq!(refused(npy(3, "<i4", false, "(1,)", &four)), "Unsupported NPY format version");
    assert_eq!(refused(b"\x93NUMPZ\x01\x00\x00\x00".to_vec()), "Not an NPY array");
}
