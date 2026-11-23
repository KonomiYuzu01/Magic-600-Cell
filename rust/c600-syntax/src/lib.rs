//! The parser, formatter and lowering of 600 Cell Script (LP2, milestone M7).
//!
//! The surface is LX-8's candidate A, which the owner chose for LQ13 on 16 November 2026: declaration keywords,
//! braces, local `let` and an explicit `return`. Its keyword and punctuation details are drafts that freeze with the
//! syntax and the IR (PHS.49); `rust/README.md` lists them. The crate depends on the IR draft only.
//!
//! - [`parse`] checks the source bytes (LG1.3), reads the edition header (LG1.1, LG1.2) and parses with recovery
//!   (LG1.5).
//! - [`format`] prints a parsed program in one canonical layout (LG1.4).
//! - [`lower`] turns parsed files into an IR [`c600_ir::Program`] with a source map (LG4.12). Exact literals stay
//!   exact decimal text and approximate literals keep their precision (LG1.6, LG1.7).
//! - [`keywords`] and [`builtins`] give, read-only, the reserved names lowering checks, for a host's completion list,
//!   and [`base_types`], [`type_constructors`], [`precisions`], [`effect_names`], [`capability_names`] and [`orders`]
//!   the other names lowering reads.
//!
//! For the authoring services (LP4, EV1.2; M7 deferred them, PHS.21) a parsed file also keeps its comments and
//! whitespace ([`ast::Trivia`]) and, for each recovery hole, what the parser read of the broken declaration
//! ([`ast::RecoveredDecl`]); [`lexemes`] gives the tokens the parser reads, [`pinned`] the byte check alone and
//! [`type_text`] a type's canonical text. None of these changes what parses, lowers or formats.

#![forbid(unsafe_code)]

pub mod ast;
mod format;
mod lex;
mod lower;
mod parse;

pub use ast::ParsedFile;
pub use c600_digest::sha256_hex;
pub use format::{format, type_text};
pub use lex::{lexemes, Lexeme, LexemeKind};
pub use lower::{
    base_types, builtins, capability_names, effect_names, keywords, lower, orders, precisions, type_constructors,
    LowerError, Lowered, SourceMap,
};
pub use parse::{parse, pinned};

/// The only edition this parser reads. Like the IR draft's name, it says that its rules may still change before the
/// syntax freeze (LG1.1, LG1.2).
pub const EDITION: &str = "6cs-draft-1";

/// The stable syntax codes (LG1, ERR). A diagnostic's message may change; its code does not (LG1.5).
pub mod code {
    pub const EDITION_MISSING: &str = "LG1:syntax.edition_missing";
    pub const EDITION_UNSUPPORTED: &str = "LG1:syntax.edition_unsupported";
    pub const INVALID_ENCODING: &str = "LG1:syntax.invalid_encoding";
    pub const INVALID_LITERAL: &str = "LG1:syntax.invalid_literal";
    pub const UNEXPECTED_TOKEN: &str = "LG1:syntax.unexpected_token";
    pub const RECOVERY_GAP: &str = "LG1:syntax.recovery_gap";
}

/// A source range: byte offsets into the pinned source bytes, and the 1-based line and column of its start. A column
/// counts Unicode scalar values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

/// ERR.10's raw context of an encoding failure: the SHA-256 of the raw input, which is not a validated source digest,
/// and the offending byte interval `[start, end)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawContext {
    pub digest: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    /// Absent only for an encoding failure, which carries [`RawContext`] instead (LG1.3, ERR.10).
    pub span: Option<Span>,
    pub raw: Option<RawContext>,
}
