//! Pinned documents and source ranges (EV1.1, EV1.10; ERR.10, ERR.12).
//!
//! A [`SourceRange`] is H7's: absolute UTF-8 byte offsets on scalar boundaries, half-open, bound to the exact source
//! digest, with `0 <= start <= end <= byte_length`; an empty range is an insertion, also at `byte_length`. The bytes
//! are LG1.3's validated UTF-8/LF source, never normalized. A host declares `utf8`, `utf16` or `scalar` when it opens
//! a [`Document`], and both endpoints convert by decoding those same bytes and counting the declared units from the
//! start. A position inside a multibyte scalar or a surrogate pair is refused, never rounded. Bytes that are not pinned
//! source (invalid UTF-8, a carriage return, a byte-order mark) have no source range at all, not even `[0, 0)`: their
//! diagnostic carries ERR.10's raw context instead.

use c600_syntax::{pinned, sha256_hex, Diagnostic, RawContext};

use crate::envelope::{code, Envelope, Form, NextAction, Reason, Service};

/// The units a host counts positions in (EV1.10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Units {
    Utf8,
    Utf16,
    Scalar,
}

impl Units {
    pub const ALL: [Units; 3] = [Units::Utf8, Units::Utf16, Units::Scalar];

    /// The protocol's name: `utf8`, `utf16` or `scalar`.
    pub fn name(self) -> &'static str {
        match self {
            Units::Utf8 => "utf8",
            Units::Utf16 => "utf16",
            Units::Scalar => "scalar",
        }
    }

    /// The units a protocol name declares, if any.
    pub fn named(name: &str) -> Option<Units> {
        Units::ALL.into_iter().find(|u| u.name() == name)
    }

    fn width(self, c: char) -> usize {
        match self {
            Units::Utf8 => c.len_utf8(),
            Units::Utf16 => c.len_utf16(),
            Units::Scalar => 1,
        }
    }
}

/// A source range: UTF-8 byte offsets `[start, end)` on scalar boundaries of the source whose SHA-256 is `digest`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceRange {
    pub digest: String,
    pub start: usize,
    pub end: usize,
}

impl SourceRange {
    pub fn new(digest: impl Into<String>, start: usize, end: usize) -> SourceRange {
        SourceRange { digest: digest.into(), start, end }
    }

    /// Whether the range is empty: an insertion point.
    pub fn is_insertion(&self) -> bool {
        self.start == self.end
    }
}

/// A range in a host's declared units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostRange {
    pub start: usize,
    pub end: usize,
    pub units: Units,
}

/// Which input a response answers (EV1.1): the source digest, the edition and the module context's digest. A host
/// attaches a response only to the document it [names](Subject::names).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    /// The SHA-256 of the document's bytes.
    pub source: String,
    /// Whether the bytes are pinned source. Unpinned bytes are named by ERR.10's raw digest, which is no validated
    /// source digest.
    pub pinned: bool,
    pub edition: String,
    /// The module context's digest, for the services that read one.
    pub context: Option<String>,
}

impl Subject {
    /// Whether this response answers exactly this document's bytes.
    pub fn names(&self, document: &Document) -> bool {
        self.source == document.digest()
    }
}

#[derive(Clone, Debug)]
enum Content {
    Pinned(String),
    Refused { bytes: Vec<u8>, diagnostic: Diagnostic },
}

/// Immutable source bytes, named by their digest, with the units the host declared when it opened them.
#[derive(Clone, Debug)]
pub struct Document {
    content: Content,
    digest: String,
    units: Units,
}

/// Why a position does not convert.
enum Fault {
    OutOfBounds(usize),
    InsideScalar,
}

impl Document {
    /// Opens bytes with the host's declared units. Bytes that are not pinned source open too, so that parse and
    /// diagnose can report ERR.10's encoding diagnostic, but every position on them is refused.
    pub fn open(bytes: impl Into<Vec<u8>>, units: Units) -> Document {
        let bytes = bytes.into();
        let digest = sha256_hex(&bytes);
        let content = match pinned(&bytes) {
            Ok(_) => Content::Pinned(String::from_utf8(bytes).unwrap_or_default()),
            Err(diagnostic) => Content::Refused { bytes, diagnostic },
        };
        Document { content, digest, units }
    }

    /// Opens bytes with units named as the protocol names them; another name is refused as undeclared units.
    pub fn open_declared(bytes: impl Into<Vec<u8>>, units: &str) -> Result<Document, Envelope> {
        match Units::named(units) {
            Some(units) => Ok(Document::open(bytes, units)),
            None => Err(Envelope::new(
                code::SOURCE_RANGE_INVALID,
                Form::Refusal,
                Service::Open,
                Reason::UndeclaredUnits { declared: None, requested: units.to_owned() },
                NextAction::UseDeclaredUnits,
            )
            .saying("a document declares utf8, utf16 or scalar units")),
        }
    }

    /// The SHA-256 of the bytes in lowercase hex. For pinned bytes it is the source digest.
    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn units(&self) -> Units {
        self.units
    }

    pub fn bytes(&self) -> &[u8] {
        match &self.content {
            Content::Pinned(text) => text.as_bytes(),
            Content::Refused { bytes, .. } => bytes,
        }
    }

    /// The source text, if the bytes are pinned source.
    pub fn text(&self) -> Option<&str> {
        match &self.content {
            Content::Pinned(text) => Some(text),
            Content::Refused { .. } => None,
        }
    }

    /// ERR.10's encoding diagnostic, if the bytes are not pinned source.
    pub fn encoding(&self) -> Option<&Diagnostic> {
        match &self.content {
            Content::Pinned(_) => None,
            Content::Refused { diagnostic, .. } => Some(diagnostic),
        }
    }

    /// The subject of a response about this document.
    pub(crate) fn subject(&self, edition: &str, context: Option<String>) -> Subject {
        Subject { source: self.digest.clone(), pinned: self.text().is_some(), edition: edition.to_owned(), context }
    }

    /// The source text, if the request names these bytes and they are pinned; otherwise the refusal.
    pub(crate) fn expect(&self, digest: &str, service: Service) -> Result<&str, Envelope> {
        if digest != self.digest {
            return Err(stale(service, digest, &self.digest));
        }
        self.text().ok_or_else(|| self.not_pinned(service))
    }

    fn not_pinned(&self, service: Service) -> Envelope {
        let raw = self.encoding().and_then(|d| d.raw.clone()).unwrap_or_else(|| RawContext {
            digest: self.digest.clone(),
            start: 0,
            end: self.bytes().len(),
        });
        Envelope::new(
            code::SOURCE_RANGE_INVALID,
            Form::Refusal,
            service,
            Reason::NotPinned { raw },
            NextAction::Reencode,
        )
        .saying("bytes that are not UTF-8/LF source have no source range")
    }

    /// The byte offset of a position in the declared units, for a request naming `digest`.
    pub fn offset(&self, digest: &str, host: usize) -> Result<usize, Envelope> {
        self.byte_offset(digest, host, Service::Range)
    }

    pub(crate) fn byte_offset(&self, digest: &str, host: usize, service: Service) -> Result<usize, Envelope> {
        let text = self.expect(digest, service)?;
        to_byte(text, host, self.units).map_err(|f| self.fault(service, host, self.units, f))
    }

    /// The source range of a range in the declared units.
    pub fn range(&self, digest: &str, start: usize, end: usize) -> Result<SourceRange, Envelope> {
        let text = self.expect(digest, Service::Range)?;
        if start > end {
            return Err(reversed(start, end));
        }
        let convert =
            |host| to_byte(text, host, self.units).map_err(|f| self.fault(Service::Range, host, self.units, f));
        Ok(SourceRange::new(self.digest.clone(), convert(start)?, convert(end)?))
    }

    /// A source range in the declared units.
    pub fn host(&self, range: &SourceRange) -> Result<HostRange, Envelope> {
        self.host_in(range, self.units)
    }

    /// A source range in the named units, which must be the declared ones (EV1.10).
    pub fn host_in(&self, range: &SourceRange, units: Units) -> Result<HostRange, Envelope> {
        let text = self.expect(&range.digest, Service::Range)?;
        if units != self.units {
            return Err(Envelope::new(
                code::SOURCE_RANGE_INVALID,
                Form::Refusal,
                Service::Range,
                Reason::UndeclaredUnits { declared: Some(self.units), requested: units.name().to_owned() },
                NextAction::UseDeclaredUnits,
            )
            .saying("positions count the units declared when the document was opened"));
        }
        self.validate_in(text, range, Service::Range)?;
        let convert = |byte| to_host(text, byte, units).map_err(|f| self.fault(Service::Range, byte, Units::Utf8, f));
        Ok(HostRange { start: convert(range.start)?, end: convert(range.end)?, units })
    }

    /// Checks a source range against these bytes: the digest, the order and both endpoints.
    pub fn validate(&self, range: &SourceRange) -> Result<(), Envelope> {
        let text = self.expect(&range.digest, Service::Range)?;
        self.validate_in(text, range, Service::Range)
    }

    pub(crate) fn validate_in(&self, text: &str, range: &SourceRange, service: Service) -> Result<(), Envelope> {
        if range.start > range.end {
            let mut e = reversed(range.start, range.end);
            e.service = service;
            return Err(e);
        }
        for byte in [range.start, range.end] {
            to_byte(text, byte, Units::Utf8).map_err(|f| self.fault(service, byte, Units::Utf8, f))?;
        }
        Ok(())
    }

    fn fault(&self, service: Service, offset: usize, units: Units, fault: Fault) -> Envelope {
        let reason = match fault {
            Fault::OutOfBounds(length) => Reason::OutOfBounds { offset, units, length },
            Fault::InsideScalar => Reason::InsideScalar { offset, units },
        };
        Envelope::new(code::SOURCE_RANGE_INVALID, Form::Refusal, service, reason, NextAction::ChoosePosition)
            .saying("a position lies on a scalar boundary inside the document")
    }
}

/// The refusal of a request that names other bytes than the document holds.
pub(crate) fn stale(service: Service, requested: &str, held: &str) -> Envelope {
    Envelope::new(
        code::STALE_EDITOR_SOURCE,
        Form::Refusal,
        service,
        Reason::StaleSource { requested: requested.to_owned(), held: held.to_owned() },
        NextAction::Refresh,
    )
    .saying("the request names source bytes that are no longer the document's")
}

fn reversed(start: usize, end: usize) -> Envelope {
    Envelope::new(
        code::SOURCE_RANGE_INVALID,
        Form::Refusal,
        Service::Range,
        Reason::Reversed { start, end },
        NextAction::ChoosePosition,
    )
    .saying("a range starts at or before its end")
}

/// The byte offset of `host` units from the start, if it is a scalar boundary inside the text.
fn to_byte(text: &str, host: usize, units: Units) -> Result<usize, Fault> {
    let mut counted = 0;
    for (byte, c) in text.char_indices() {
        if counted == host {
            return Ok(byte);
        }
        counted += units.width(c);
        if counted > host {
            return Err(Fault::InsideScalar);
        }
    }
    if counted == host {
        Ok(text.len())
    } else {
        Err(Fault::OutOfBounds(counted))
    }
}

/// The count of `units` before the byte offset `byte`, if it is a scalar boundary inside the text.
fn to_host(text: &str, byte: usize, units: Units) -> Result<usize, Fault> {
    if byte > text.len() {
        return Err(Fault::OutOfBounds(text.len()));
    }
    if !text.is_char_boundary(byte) {
        return Err(Fault::InsideScalar);
    }
    Ok(text[..byte].chars().map(|c| units.width(c)).sum())
}
