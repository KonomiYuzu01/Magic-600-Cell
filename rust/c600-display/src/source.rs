//! Source linkage under EV1.10 and RM.10. A range is a half-open UTF-8 byte range on scalar boundaries, bound to the
//! digest of the producing source. Admission checks its bounds and boundaries against the pinned bytes; a malformed
//! range is `EV3:input.display_plan_invalid`. Navigation opens the producing version, never a nearby declaration in
//! newer source; applying a range to other source returns `EV3:stale.source_link` before any conversion.

use std::collections::BTreeMap;
use std::sync::Arc;

use c600_digest::sha256_hex;

use crate::error::{codes, Error};
use crate::plan::SourceLink;

/// One range in the three host units of EV1.10: UTF-8 bytes, UTF-16 code units and Unicode scalars.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostRange {
    pub utf8: (u64, u64),
    pub utf16: (u64, u64),
    pub scalar: (u64, u64),
}

/// Where a followed link leads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Followed {
    /// The range in the producing version, with the selected text.
    Range { digest: String, range: HostRange, text: String },
    /// RM.10's `source_missing(reason)`, with the producing digest.
    Missing { digest: String, reason: String },
}

/// The UTF-16 and scalar offsets of a UTF-8 offset, or `None` off a scalar boundary or past the end.
pub fn boundary(source: &str, utf8: u64) -> Option<(u64, u64)> {
    let utf8 = usize::try_from(utf8).ok()?;
    if utf8 > source.len() || !source.is_char_boundary(utf8) {
        return None;
    }
    let prefix = &source[..utf8];
    Some((prefix.encode_utf16().count() as u64, prefix.chars().count() as u64))
}

/// Converts a range of `bytes` to host units, checking EV1.10's rules: valid UTF-8, `start <= end <= len` and both
/// ends on scalar boundaries.
pub fn convert(bytes: &[u8], start: u64, end: u64) -> Result<HostRange, Error> {
    let source = std::str::from_utf8(bytes).map_err(|_| Error::invalid("the pinned source is not UTF-8"))?;
    if start > end {
        return Err(Error::invalid(format!("source range [{start},{end}) is reversed")));
    }
    let (Some(from), Some(to)) = (boundary(source, start), boundary(source, end)) else {
        return Err(Error::invalid(format!("source range [{start},{end}) is out of bounds or splits a scalar")));
    };
    Ok(HostRange { utf8: (start, end), utf16: (from.0, to.0), scalar: (from.1, to.1) })
}

/// Admission's check of one link against the pinned sources (RM.10): a present range names pinned bytes with its
/// digest and lies on scalar boundaries; missing source names its producing digest and a reason.
pub fn admit(link: &SourceLink, pinned: &BTreeMap<String, Arc<Vec<u8>>>) -> Result<(), Error> {
    match link {
        SourceLink::Range { digest, start, end } => {
            let bytes = pinned
                .get(digest)
                .ok_or_else(|| Error::invalid(format!("source range names {digest}, which has no pinned bytes")))?;
            if sha256_hex(bytes) != *digest {
                return Err(Error::invalid("pinned source bytes do not match their digest"));
            }
            convert(bytes, *start, *end).map(|_| ())
        }
        SourceLink::Missing { digest, reason } => {
            if digest.is_empty() || reason.is_empty() {
                return Err(Error::invalid("missing source must name its producing digest and a reason"));
            }
            Ok(())
        }
    }
}

/// Navigation: opens the producing version that the publication pinned, whatever the newest source is (RM.10).
pub fn follow(link: &SourceLink, pinned: &BTreeMap<String, Arc<Vec<u8>>>) -> Result<Followed, Error> {
    match link {
        SourceLink::Range { digest, start, end } => {
            let bytes = pinned.get(digest).ok_or_else(|| Error::invalid("the pinned source is gone"))?;
            let range = convert(bytes, *start, *end)?;
            let text = String::from_utf8_lossy(&bytes[*start as usize..*end as usize]).into_owned();
            Ok(Followed::Range { digest: digest.clone(), range, text })
        }
        SourceLink::Missing { digest, reason } => {
            Ok(Followed::Missing { digest: digest.clone(), reason: reason.clone() })
        }
    }
}

/// A source action against `current` source bytes: `EV3:stale.source_link` when they are not the producing version;
/// the host-unit conversion follows the digest check.
pub fn apply(link: &SourceLink, current: &[u8]) -> Result<HostRange, Error> {
    let digest = sha256_hex(current);
    if digest != link.digest() {
        return Err(Error::new(codes::STALE_SOURCE, "the source changed since this output was produced")
            .with("producing", link.digest())
            .with("current", digest));
    }
    match link {
        SourceLink::Range { start, end, .. } => convert(current, *start, *end),
        SourceLink::Missing { reason, .. } => Err(Error::invalid(format!("no range: source missing ({reason})"))),
    }
}
