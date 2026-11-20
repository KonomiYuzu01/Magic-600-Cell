//! The interpreter of route RT1 (LQ9): it runs a loaded IR program under the `lp1-exec-draft-1` meter (DET.55 to
//! DET.59), one charge row per operation.
//!
//! - [`load`] checks the IR identity and the program's names, and compiles every function body into a table of nodes:
//!   names become frame slots and closure captures, and each code unit gets its frame layout (DET.57). It refuses
//!   IR that it cannot run with `LG4:input.ir_rejected`. It does not re-run the checker; an operand of the wrong type
//!   that reaches an operation stops the run with `LG4:input.ir_rejected` at that operation.
//! - [`Loaded::run`] evaluates with an explicit work stack, so deep recursion ends at a budget and never overflows the
//!   host stack (CT-DET-20). Before each charge it observes a pending interruption (DET.56).
//!
//! # Operations (DET.55)
//!
//! Operands are evaluated left to right in the IR's child order, then their consumer runs. Each name below is one
//! step. Memory is the logical reservation of DET.57; an excess reservation shrinks to the actual size after success,
//! and consumed operands are released after admission, both recorded on the operation's row (DET.56, DET.58).
//!
//! | IR | Operations |
//! | --- | --- |
//! | literal, function reference | `value` (the payload; 32 bytes for a function reference) |
//! | variable | `visit` (a borrow: no owner, no memory) |
//! | `let` | `visit` (moves an owned value into its slot) or `alias` (retains a borrowed one); the body; at the end `scope_exit`, which releases the slot, after an `alias` when the result borrows part of the slot; a result that borrows the whole slot moves |
//! | `if`, `match`, `and`, `or` | the condition, then `branch`; a match prices DET.62's equality against every arm |
//! | `display_if` | the condition, then `display_if`, then the selected branch only (DET.64) |
//! | call | the arguments, then `call` (the callee's frame, DET.57); the body; `return`, after an `alias` when the result is a borrow (DET.58); arguments are borrowed, and owned argument temporaries are released at the return |
//! | closure | `value` (32 bytes), then one `alias` per capture in declaration order |
//! | record | `value` (the header), then per field `visit` (a move) or `alias` (a borrow) |
//! | field | `visit`: a borrowed record lends its field; an owned one retains the field with a charged owner and is released |
//! | list (LP2 draft) | `value` (the header), then per element `visit` (a move) or `alias` (a borrow), as a record is built |
//! | `length`, `index` (LP2 drafts) | `length` reserves its integer; `index` lends or retains one element as a field read does, and an index outside the list stops with `LG4:input.pointwise_failed` before its step |
//! | `push`, `concat`, `inverse`, `conjugate`, `commutator`, `compose`, `word_key` (LP2 drafts) | one step each, priced before the value is built: `expand_per_letter` per letter built or scanned, [`LIST_PER_ELEMENT`] per element built |
//! | `trace_key` (LB1.10; LP2 draft) | one step, priced before the key is built: `expand_per_letter` per read of the least topological scan ([`crate::trace::Relation::reads`], [`crate::trace::Relation::menu_reads`] for a menu word) and 2 bytes reserved per letter, 4 per menu letter; a run that pins no trace relation of the word's alphabet (`full-disjoint-support/1` for a Full word, a `jumble-locality/1` relation that keys the pinned menu alphabet for a menu word) stops with `LB1:input.key_contract_mismatch`, and a letter outside the relation's alphabet with `LB1:input.incompatible_alphabet`, before its step |
//! | `word_key` of a menu word (LG2.18; LP2 draft) | one step, priced before the key is built: `expand_per_letter` per letter scanned plus the draft work of its exact products at `field_per_unit_pair` ([`crate::menu_key::price`]), with that price's bound of its tokens reserved; a run whose pinned menu carries no item matrices stops with `LB1:input.key_contract_mismatch`, and a word of another alphabet with `LB1:input.incompatible_alphabet`, before its step. A run whose pinned menu carries item matrices keys no Full word |
//! | `length`, `concat`, `inverse`, `conjugate`, `commutator`, `compose` of menu words (LB1.8; LP2 drafts) | as on words: `expand_per_letter` per letter built and 4 bytes reserved per letter; an inverse reads the pinned menu's inverse items, and a word of another alphabet stops with `LB1:input.incompatible_alphabet` before its step |
//! | set, map (LP2 drafts) | one step `set` or `map`: [`LIST_PER_ELEMENT`] per element and n·⌈log₂ n⌉ comparisons of the widest key, each `compare_per_unit` per unit of two such keys; it reserves every item written and settles to the built value |
//! | `contains`, `insert`, `lookup`, `elements`, `keys`, `entries` (LP2 drafts) | one step each: ⌈log₂(n + 1)⌉ comparisons of the probe for a search, [`LIST_PER_ELEMENT`] per element built; `lookup` lends or retains its value as a field read does, and a missing key stops with `LG4:input.pointwise_failed` before its step |
//! | `compress`, `expand`, and `concat` and `length` of compressed words (LP2 drafts) | one step each: `compress` costs `expand_per_letter` per letter; `concat` an equality walk over both words, the integer row's addition of their lengths and [`LIST_PER_ELEMENT`] per part of both plus one, and it reserves the parts of both and settles to the built word; `length` reserves its integer and expands nothing; `expand` costs `expand_per_letter` per letter out and [`LIST_PER_ELEMENT`] per part it may visit, 2·(height + count) + 1, and a range past the word's end stops with `LG4:input.pointwise_failed` before its step |
//! | exact arithmetic | `exact_add`, `exact_mul` (LP2 draft), `exact_less`, `exact_eq`, `not`, `to_approx`, `approx_add`, `approx_less`, `approx_and`, `approx_not`; an integer sum or product is priced by the vectors' integer rows (DET.57), a rational or Q(√5) one by `field_per_unit_pair` per pair of widened operand bytes |
//! | `assert` (LP2 draft) | the predicate, then one step `assert` with no memory; a false predicate stops the run with `LG4:input.pointwise_failed` after that step, before the guarded operation (DET.55) |
//! | snapshot read, declassifier | `kernel` with the host row's tariff and result bound |
//! | snapshot read with an argument (LP2 draft) | `encode` of the argument, then `kernel` with the host row's tariff and result bound and the argument bytes as output; the host gets the canonical bytes, and the buffer is released after the answer; an approximate argument stops with `LG4:input.ir_rejected` before the encoding |
//! | file read, clock | `read` with the host row's tariff and result bound |
//! | view, workspace and file output | `publish_view` or `emit`: one fused row per output unit, its encoding work, buffer and bytes in one admission (DET.56, DET.59); the buffer is released after the publication |
//! | `propose` | `encode` of the request, then `invoke` with the host row's tariff and the request bytes as output; a menu word is proposed as [`Request::ProposeMenu`] (LP2 draft) |
//! | receipt field | `read_receipt`; a field the host withheld stops with `LG3:capability.not_granted` (BND.42) |
//! | exact sink | `encode` of the value, then `use_exact`; the host gets the canonical bytes, and the buffer is released after the hand-over (PHS.52, owner decision of 16 November 2026) |
//! | display accounting | `accounting`, read after its own step |
//! | random draw | `rng_draw` (DET.61) |
//! | child scope (LP2 draft) | `child`, one step in the enclosing scopes; then, in a scope of its own under the effective caps (DET.60), the body's `call`, the body and its `return`, whose result moves to the parent. A stop the child absorbs unwinds it; then, when the stop branch names a status, `status` (4 bytes), and the stop branch's `call`, body and `return` run in the parent |
//! | `repeat` (LP2 draft) | the count, the initial value, then `visit` (moves an owned initial value into the loop's slot) or `alias` (retains a borrowed one); per iteration k from 0, `loop_test:k`, the body, and `visit` or `alias` to refill the slot, the old value released after admission; then the final false test `loop_test:n` and `scope_exit`, which releases the count and moves the slot's value out. A count that is no natural number stops with `LG4:input.ir_rejected` at `loop_test:0` |
//!
//! A run is entered with `rng_init` when a seed is given, one `read` per argument, and the entry `call`. Its result
//! is the top-level output unit: one fused `emit` row (DET.59).
//!
//! A run pins at most one menu alphabet ([`Options::menu`]; LB1.8, LP2 draft), and every menu word in it is a word of
//! that alphabet. A menu-word literal takes the alphabet when it is read, and it stops the run before its step when a
//! letter is outside it or the run pins none; an argument that holds a menu word of another alphabet is refused
//! before the run; a host's answer that holds one stops the run after its row. Each has the code
//! `LB1:input.incompatible_alphabet`. A menu-word match pattern compares letters, since a literal names no alphabet of
//! its own ([`crate::menu`]).
//!
//! A run pins at most one trace relation ([`Options::relation`]; LB1.10, LP2 draft), and every trace key in it is a
//! key under that relation: `trace_key` stops the run before its step when it pins none of the word's alphabet, and a
//! host's answer that holds a key under another header, or letters that are not the key of their trace, stops the run
//! after its row. Each has the code `LB1:input.key_contract_mismatch`. A trace key has no canonical form, so no
//! argument or result holds one ([`crate::trace`]).
//!
//! # Drafts (M2)
//!
//! The operation names, the stop locator text, the frame layout (parameters, captures, `let` slots and the
//! temporaries that hold operands; a body's own result has no slot, since it moves to the caller), the per-capture
//! `alias`, the receipt request record, [`DraftTariffs`], the `Status` accounting value `"running"`, the JSON form of
//! a word, and a value's canonical byte length as retained metadata (so an output unit is one fused row, DET.59) are
//! drafts that change freely until the IR freeze. A preview request keeps DET.59's other form: a priced `encode` pass
//! on the retained bound, then `invoke`. A snapshot read with an argument (LP2 draft) takes the same form, and the
//! request record [`Request::SnapshotWith`] with its argument as canonical bytes is a draft too.
//!
//! # Child scopes (DET.60; LP2 draft)
//!
//! A child scope's body and its stop branch are code units of their own, as a closure's body is. Each borrows the
//! slots it names from the enclosing frame, without an `alias`: the child borrows its parent's inputs. The child's
//! meter scope is named by its locator without `#child`. A stop ends the scope the meter names
//! ([`Meter::leave_child`]): the innermost open child unwinds, its tasks, frames and values above its entry go, and the
//! meter drops the owners it counts. A child absorbs a stop of its own, and its stop branch runs; a stop of an
//! enclosing scope, an interruption, or a stop in the stop branch goes on to the next open child, or ends the run.
//! The run's own generator is shared. `rng_init` inside a child takes the child's stream path (DET.61): its parent's,
//! followed by the decimal index of the child among the children its parent has started, from 0. The `status` row,
//! the stop branch's frame and optional status, the shared generator and the index's base are drafts.
//!
//! # Loops (DET.55; LP2 draft)
//!
//! `repeat(n, init, fn(acc: T) -> T { … })` runs its body n times, each time with `acc` bound to the value so far, in
//! a `let` slot of the enclosing frame: there is no call per iteration. Every test costs one step, the final false one
//! too (DET.55). An operation inside an iteration's body names it in its locator as `label:path#body:k > `, after the
//! call sites and loops that enclose it, so the stop position names the loop path and the iteration (DET.56). The
//! count is an operand held for the whole loop: it takes one temporary of the frame, and its handle is released at
//! the loop's `scope_exit`. The surface form, the operation names `loop_test:k`, the `#body:k` locator, the refill's
//! `visit` or `alias` and the loop's `scope_exit` are drafts.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering as Atomic};
use std::sync::Arc;

use c600_ir::{
    self as ir, Accounting, Callee, EndTestOutcome, ExactRole, Expr, Field, FnKind, Order, Prim, QTy, ReceiptField, Ty,
};
use num_bigint::{BigInt, Sign};

use crate::approx;
use crate::encode;
use crate::menu::{self, MenuAlphabet, MenuIdentity};
use crate::menu_key;
use crate::meter::{
    self, frame_size, integer_size, Amounts, DraftTariffs, Interruption, Meter, OwnerId, SourceRange, Stop, Summary,
    Tariffs, GENERATOR_SIZE,
};
use crate::rng::{self, Cursor};
use crate::trace::{self, Relation};
use crate::value::{
    self, entry_key, entry_value, find, free_reduction, inverse, map_insert, set_insert, Closure, Compressed,
    Generator, List, MenuWord, Record, Text, Value, WordStream,
};

/// The draft price of a built list (LP2): kernel units per element it holds. DET.55 names no row for it.
///
/// The list and word operations' names and prices, the word operations' reuse of `expand_per_letter`, the pointwise
/// stop of an index outside its list and a list's JSON form are LP2 drafts that change freely until the IR freeze
/// (PHS.49). So are the set and map operations, their prices (a comparison priced as an exact equality walk over two
/// keys of the bounding size), the canonical key order ([`value::key_order`]), the pointwise stop of a missing key
/// and their JSON forms. So are the compressed word ([`value::Compressed`]), its parts, its logical layout, its
/// operations and their prices, and its lack of a JSON form and of an exact equality. So are the F2 and F5 elements
/// (KN3.2), their operations, their prices, the pointwise assertion that guards `field_inverse`, their key order and
/// their lack of a JSON form and of a canonical one.
pub const LIST_PER_ELEMENT: u128 = 1;

/// The codes the interpreter raises in addition to the meter's (chapter 13).
pub mod code {
    pub const IR_VERSION_UNSUPPORTED: &str = "LG4:input.ir_version_unsupported";
    pub const IR_REJECTED: &str = "LG4:input.ir_rejected";
    pub const NONCANONICAL_VALUE: &str = "PS1:input.noncanonical_value";
    pub const HOST_EXHAUSTED: &str = "KN7:resource_limit.host_exhausted";
    /// A menu word that is not a word of the run's pinned alphabet (LB1.8; LP2 draft).
    pub use crate::menu::INCOMPATIBLE_ALPHABET;
    /// A trace key under another header than the run's pinned relation, or none pinned (LB1.10; LP2 draft).
    pub use crate::trace::KEY_CONTRACT_MISMATCH;
}

/// A refusal before any operation is metered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub code: &'static str,
    pub detail: String,
}

fn refuse(code: &'static str, detail: impl Into<String>) -> Refusal {
    Refusal { code, detail: detail.into() }
}

/// The declared tariff of a host row: its kernel cost and the bound of its result's logical size (DET.57).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowTariff {
    pub kernel_cost: u128,
    pub result_bound: u128,
}

/// A request to the host.
#[derive(Clone, Copy, Debug)]
pub enum Request<'a> {
    Snapshot {
        query: &'a str,
        ty: &'a QTy,
    },
    /// The engine's row `query` for an argument (LP2 draft), such as a word's protection. `arg` holds the argument's
    /// canonical bytes (DET.59, FP.5), which the run priced before asking; `c600_run::provenance::parse` reads them.
    /// It is always an engine read, never a workspace record.
    SnapshotWith {
        query: &'a str,
        arg: &'a [u8],
        ty: &'a QTy,
    },
    Declassified {
        id: &'a str,
        ty: &'a Ty,
    },
    FileRead {
        path: &'a str,
        ty: &'a Ty,
    },
    Clock,
    /// A literal-word preview (KN5.9) with its canonical request bytes.
    Propose {
        word: &'a [i32],
        target: &'a str,
        request: &'a [u8],
    },
    /// A menu-word preview (LB1.8; LP2 draft) with its canonical request bytes: the letters `[cap, item]` and the
    /// identity block of their alphabet, the run's pinned one, so the engine refuses a word played at a state of
    /// another alphabet by the first field in which the blocks differ (KN1.4). A host that gates previews gates this
    /// one as it gates [`Request::Propose`]; [`Request::is_proposal`] names both.
    ProposeMenu {
        letters: &'a [[u32; 2]],
        menu: &'a MenuIdentity,
        target: &'a str,
        request: &'a [u8],
    },
}

impl Request<'_> {
    /// Whether the request is a preview: [`Request::Propose`] or [`Request::ProposeMenu`].
    pub fn is_proposal(&self) -> bool {
        matches!(self, Request::Propose { .. } | Request::ProposeMenu { .. })
    }
}

/// Where a value goes.
#[derive(Clone, Copy, Debug)]
pub enum Sink<'a> {
    View,
    Workspace { record: &'a str, ty: &'a QTy },
    File { path: &'a str },
    Exact { role: ExactRole },
}

/// The host of a run: the engine, the session and the granted files behind the program's effects. A refusal is a stop
/// code, such as `LG3:capability.not_granted`.
pub trait Host {
    /// The tariff of a row, before it is admitted.
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str>;
    /// Performs an admitted row. The result's size must stay within the declared bound.
    fn perform(&mut self, request: &Request) -> Result<Value, &'static str>;
    /// Delivers an admitted value: published or written bytes, or an exact value to its sink.
    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str>;
}

/// A host that grants nothing, for pure programs.
pub struct NoHost;

impl Host for NoHost {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Err(meter::code::NOT_GRANTED)
    }
    fn perform(&mut self, _: &Request) -> Result<Value, &'static str> {
        Err(meter::code::NOT_GRANTED)
    }
    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Err(meter::code::NOT_GRANTED)
    }
}

/// A solver or host interruption, observed before each operation (DET.56, KN7.6).
pub trait Cancel {
    fn pending(&self) -> Option<Interruption>;
}

/// A cancellation flag another thread may set.
#[derive(Debug, Default)]
pub struct CancelFlag(AtomicU8);

impl CancelFlag {
    pub fn cancel(&self, why: Interruption) {
        self.0.store(
            match why {
                Interruption::ByUser => 1,
                Interruption::RevisionReplaced => 2,
            },
            Atomic::SeqCst,
        );
    }
}

impl Cancel for CancelFlag {
    fn pending(&self) -> Option<Interruption> {
        match self.0.load(Atomic::SeqCst) {
            1 => Some(Interruption::ByUser),
            2 => Some(Interruption::RevisionReplaced),
            _ => None,
        }
    }
}

/// The source ranges of the executed bytes (LG4.12, DET.56): the range of the node at `path` ("root" or dot-joined
/// child indices) of `function`'s body.
pub trait SourceRanges {
    fn range(&self, module: &str, function: &str, path: &str) -> Option<SourceRange>;
}

/// How to run.
#[derive(Clone, Copy)]
pub struct Options<'a> {
    pub limits: Amounts,
    pub tariffs: Tariffs,
    pub draft: DraftTariffs,
    /// The generator's 32-byte seed and stream path (DET.61). Without it, a draw is not granted.
    pub seed: Option<(&'a [u8], &'a [&'a str])>,
    /// Whether to keep one row per operation; a quiet run keeps the same counters and stops.
    pub record: bool,
    pub cancel: Option<&'a dyn Cancel>,
    pub sources: Option<&'a dyn SourceRanges>,
    /// The menu alphabet the run pins for its menu words (LB1.8; LP2 draft). Without one the run holds no menu word.
    /// With its item matrices ([`MenuAlphabet::with_matrices`]) the run keys menu words and no Full word (LG2.18).
    pub menu: Option<&'a MenuAlphabet>,
    /// The independence relation the run pins for its trace keys (LB1.10; LP2 draft). Without one the run holds no
    /// trace key.
    pub relation: Option<&'a Relation>,
}

impl<'a> Options<'a> {
    /// Test tariffs, rows kept, no seed, no interruption, no source ranges, no menu alphabet and no trace relation.
    pub fn new(limits: Amounts) -> Options<'a> {
        Options {
            limits,
            tariffs: Tariffs::TEST,
            draft: DraftTariffs::TEST,
            seed: None,
            record: true,
            cancel: None,
            sources: None,
            menu: None,
            relation: None,
        }
    }
}

/// The end of a run: the meter's terminal record and, when it completed, the returned value and its output bytes.
#[derive(Debug)]
pub struct Run {
    pub summary: Summary,
    pub value: Option<Value>,
    pub output: Option<Vec<u8>>,
}

type NodeId = usize;

/// A frame slot: a parameter, a capture or a `let` binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlotRef {
    Param(usize),
    Capture(usize),
    Let(usize),
}

#[derive(Debug)]
enum Kind {
    Lit(Value),
    Var(SlotRef),
    FnRef(usize),
    Let(usize),
    If,
    DisplayIf,
    Match(Vec<Value>),
    Call(usize),
    CallValue,
    Closure {
        code: usize,
        captures: Vec<SlotRef>,
    },
    Prim(Prim),
    Record(Vec<Arc<Text>>),
    Field(Box<str>),
    List,
    Set,
    Map,
    ReadSnapshot {
        query: String,
        ty: QTy,
    },
    ReadSnapshotWith {
        query: String,
        ty: QTy,
    },
    PublishView,
    WriteWorkspace {
        record: String,
        ty: QTy,
    },
    Propose,
    ReadReceipt(ReceiptField),
    UseExact(ExactRole),
    Accounting(Accounting),
    Declassified {
        id: String,
        ty: Ty,
    },
    FileRead {
        path: String,
        ty: Ty,
    },
    FileWrite {
        path: String,
    },
    RandomDraw,
    ReadClock,
    /// `rng_init`'s path segments after the run's stream path (DET.61; LP2 draft).
    RngInit(Vec<String>),
    /// A child scope (DET.60; LP2 draft): its caps in [`meter::Dim::ALL`]'s order, then its body's and its stop
    /// branch's code units, each with the enclosing frame's slots it borrows.
    Child {
        caps: [Option<u128>; 4],
        body: usize,
        body_captures: Vec<SlotRef>,
        otherwise: usize,
        otherwise_captures: Vec<SlotRef>,
    },
    /// A bounded loop (DET.55; LP2 draft): the `let` slot that holds the value so far.
    Repeat(usize),
}

#[derive(Debug)]
struct Node {
    kind: Kind,
    /// The operands, in evaluation order. A closure's body is not one, nor are a child scope's body and stop branch:
    /// each runs in its own frame.
    kids: Vec<NodeId>,
    /// The parent and this node's index among the IR children, which give the node's path.
    parent: Option<(NodeId, u32)>,
    /// The named function whose body contains the node.
    function: usize,
}

/// A code unit: a named function's body, a closure's, or a child scope's body or stop branch.
#[derive(Debug)]
struct Code {
    params: usize,
    captures: usize,
    lets: usize,
    /// DET.57's frame slots: parameters, captures, `let` bindings and temporaries.
    slots: u128,
    body: NodeId,
}

#[derive(Debug)]
struct FunctionInfo {
    module: usize,
    name: String,
    label: String,
    code: usize,
    kind: FnKind,
    params: usize,
}

/// A program ready to run.
#[derive(Debug)]
pub struct Loaded {
    modules: Vec<String>,
    functions: Vec<FunctionInfo>,
    codes: Vec<Code>,
    nodes: Vec<Node>,
}

impl Loaded {
    /// The index of a function, if the program has it.
    pub fn function(&self, module: &str, name: &str) -> Option<usize> {
        self.functions.iter().position(|f| self.modules[f.module] == module && f.name == name)
    }

    /// A function's kind.
    pub fn kind(&self, function: usize) -> FnKind {
        self.functions[function].kind
    }

    /// The frame size of a function's calls (DET.57).
    pub fn frame_size(&self, function: usize) -> u128 {
        frame_size(self.codes[self.functions[function].code].slots)
    }

    fn path(&self, node: NodeId) -> String {
        let mut parts = Vec::new();
        let mut at = node;
        while let Some((parent, index)) = self.nodes[at].parent {
            parts.push(index.to_string());
            at = parent;
        }
        if parts.is_empty() {
            return "root".into();
        }
        parts.reverse();
        parts.join(".")
    }
}

/// A binding in scope during compilation.
struct Binding {
    name: String,
    slot: SlotRef,
    /// The declaration order across the function; captures follow it (DET.57).
    id: usize,
}

struct Capture {
    name: String,
    id: usize,
    outer: SlotRef,
}

/// The function or closure body being compiled.
struct Context {
    code: usize,
    scope: Vec<Binding>,
    captures: Vec<Capture>,
    lets: usize,
    /// Nodes that name this context's captures, patched once the captures are sorted.
    uses: Vec<NodeId>,
    /// The closure or child scope whose code unit this is, and which: 0 for a closure's body or a child's body, 1 for
    /// a child's stop branch.
    closure: Option<(NodeId, u32)>,
}

enum Step<'e> {
    Visit {
        expr: &'e Expr,
        parent: Option<(NodeId, u32)>,
        attach: bool,
    },
    Bind {
        name: &'e str,
        slot: usize,
    },
    Unbind,
    /// Opens a child scope's code unit, whose body is the next node visited.
    Enter {
        code: usize,
        param: Option<&'e str>,
        node: NodeId,
        unit: u32,
    },
    Leave,
}

struct Compiler {
    loaded: Loaded,
    names: HashMap<(usize, String), usize>,
    contexts: Vec<Context>,
    next_id: usize,
}

fn rejected(detail: impl Into<String>) -> Refusal {
    refuse(code::IR_REJECTED, detail)
}

/// Loads a program: checks its IR identity and names and compiles every function body.
pub fn load(program: &ir::Program) -> Result<Loaded, Refusal> {
    if program.ir_version != ir::IR_VERSION || program.exec_contract != ir::EXEC_CONTRACT {
        return Err(refuse(
            code::IR_VERSION_UNSUPPORTED,
            format!(
                "{} / {} (this interpreter runs {} / {})",
                program.ir_version,
                program.exec_contract,
                ir::IR_VERSION,
                ir::EXEC_CONTRACT
            ),
        ));
    }
    let mut modules: Vec<String> = Vec::new();
    for m in &program.modules {
        if modules.contains(&m.name) {
            return Err(rejected(format!("module {} is declared twice", m.name)));
        }
        modules.push(m.name.clone());
    }
    let mut functions = Vec::new();
    let mut names = HashMap::new();
    // A stage module runs only in the stage evaluator, never in a program (RPI.68, RPI.73): its functions are not
    // loaded, and the checker refuses every call of one.
    let runnable = || program.modules.iter().enumerate().filter(|(_, m)| m.stage().is_none());
    for (mi, m) in runnable() {
        for f in &m.functions {
            if names.insert((mi, f.name.clone()), functions.len()).is_some() {
                return Err(rejected(format!("function {}.{} is declared twice", m.name, f.name)));
            }
            functions.push(FunctionInfo {
                module: mi,
                name: f.name.clone(),
                label: format!("{}.{}", m.name, f.name),
                code: 0,
                kind: f.kind,
                params: f.params.len(),
            });
        }
    }
    let exported: Vec<bool> = runnable().flat_map(|(_, m)| m.functions.iter().map(|f| f.exported)).collect();
    let mut compiler = Compiler {
        loaded: Loaded { modules, functions, codes: Vec::new(), nodes: Vec::new() },
        names,
        contexts: Vec::new(),
        next_id: 0,
    };
    let mut fi = 0;
    for (mi, m) in runnable() {
        for f in &m.functions {
            compiler.function(fi, mi, f, &exported)?;
            fi += 1;
        }
    }
    compiler.frames();
    Ok(compiler.loaded)
}

/// ⌈log₂ n⌉, and 0 for n ≤ 1: the comparisons of a binary search among n − 1 keys, or the merge levels of n keys.
fn log2_ceil(n: u128) -> u128 {
    if n <= 1 {
        0
    } else {
        u128::from(128 - (n - 1).leading_zeros())
    }
}

fn arity(op: Prim) -> Option<usize> {
    match op {
        Prim::ExactAdd
        | Prim::ExactMul
        | Prim::ExactLess
        | Prim::ExactEq
        | Prim::And
        | Prim::Or
        | Prim::ApproxAdd
        | Prim::ApproxLess
        | Prim::ApproxAnd => Some(2),
        Prim::Not | Prim::ApproxNot | Prim::ToApprox(_) | Prim::Assert | Prim::RngDraw => Some(1),
        Prim::ToExact => None,
        Prim::Length | Prim::Inverse | Prim::WordKey | Prim::TraceKey | Prim::Compress => Some(1),
        Prim::Index | Prim::Push | Prim::Concat | Prim::Conjugate | Prim::Commutator | Prim::Compose => Some(2),
        Prim::Elements(_) | Prim::Keys(_) | Prim::Entries(_) => Some(1),
        Prim::Contains | Prim::Insert | Prim::Lookup => Some(2),
        Prim::InsertEntry | Prim::Expand => Some(3),
        Prim::ToField(_) | Prim::Residue | Prim::FieldNeg | Prim::FieldInverse => Some(1),
        Prim::TextDecode | Prim::StreamDigest => Some(1),
        Prim::Q5Parts => Some(1),
        Prim::Q5Of => Some(3),
        Prim::WordStream => Some(2),
        Prim::FieldAdd | Prim::FieldSub | Prim::FieldMul => Some(2),
        Prim::Stage(s) => Some(s.arity()),
    }
}

impl Compiler {
    fn id(&mut self) -> usize {
        self.next_id += 1;
        self.next_id
    }

    fn resolve_fn(
        &self,
        module: usize,
        target: &Option<String>,
        name: &str,
        exported: &[bool],
    ) -> Result<usize, Refusal> {
        let mi = match target {
            None => module,
            Some(m) => self
                .loaded
                .modules
                .iter()
                .position(|n| n == m)
                .ok_or_else(|| rejected(format!("unknown module {m}")))?,
        };
        let f = *self.names.get(&(mi, name.to_string())).ok_or_else(|| rejected(format!("unknown function {name}")))?;
        if mi != module && !exported[f] {
            return Err(rejected(format!("{} is not exported", self.loaded.functions[f].label)));
        }
        Ok(f)
    }

    /// The slot a name refers to. A name bound outside the current closure becomes a capture of every closure between
    /// its binding and its use.
    fn resolve(&mut self, name: &str) -> Option<SlotRef> {
        let depth = self.contexts.len();
        let mut found = None;
        for c in (0..depth).rev() {
            let ctx = &self.contexts[c];
            if let Some(b) = ctx.scope.iter().rev().find(|b| b.name == name) {
                found = Some((c, b.slot, b.id));
                break;
            }
            if let Some(i) = ctx.captures.iter().position(|k| k.name == name) {
                found = Some((c, SlotRef::Capture(i), ctx.captures[i].id));
                break;
            }
        }
        let (c, mut slot, id) = found?;
        for inner in c + 1..depth {
            let ctx = &mut self.contexts[inner];
            ctx.captures.push(Capture { name: name.to_string(), id, outer: slot });
            slot = SlotRef::Capture(ctx.captures.len() - 1);
        }
        Some(slot)
    }

    fn function(&mut self, fi: usize, mi: usize, f: &ir::Function, exported: &[bool]) -> Result<(), Refusal> {
        let code = self.loaded.codes.len();
        let body = self.loaded.nodes.len();
        self.loaded.codes.push(Code { params: f.params.len(), captures: 0, lets: 0, slots: 0, body });
        self.loaded.functions[fi].code = code;
        let mut scope = Vec::new();
        for (i, p) in f.params.iter().enumerate() {
            let id = self.id();
            scope.push(Binding { name: p.name.clone(), slot: SlotRef::Param(i), id });
        }
        self.contexts.push(Context { code, scope, captures: Vec::new(), lets: 0, uses: Vec::new(), closure: None });
        let mut steps = vec![Step::Visit { expr: &f.body, parent: None, attach: false }];
        while let Some(step) = steps.pop() {
            match step {
                Step::Visit { expr, parent, attach } => {
                    self.visit(fi, mi, expr, parent, attach, &mut steps, exported)?
                }
                Step::Bind { name, slot } => {
                    let id = self.id();
                    let ctx = self.contexts.last_mut().expect("a context");
                    ctx.scope.push(Binding { name: name.to_string(), slot: SlotRef::Let(slot), id });
                }
                Step::Unbind => {
                    self.contexts.last_mut().expect("a context").scope.pop();
                }
                Step::Enter { code, param, node, unit } => {
                    self.loaded.codes[code].body = self.loaded.nodes.len();
                    let mut scope = Vec::new();
                    if let Some(name) = param {
                        let id = self.id();
                        scope.push(Binding { name: name.to_string(), slot: SlotRef::Param(0), id });
                    }
                    let (captures, uses, closure) = (Vec::new(), Vec::new(), Some((node, unit)));
                    self.contexts.push(Context { code, scope, captures, lets: 0, uses, closure });
                }
                Step::Leave => self.leave(),
            }
        }
        let ctx = self.contexts.pop().expect("the function's context");
        self.loaded.codes[code].lets = ctx.lets;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn visit<'e>(
        &mut self,
        fi: usize,
        mi: usize,
        expr: &'e Expr,
        parent: Option<(NodeId, u32)>,
        attach: bool,
        steps: &mut Vec<Step<'e>>,
        exported: &[bool],
    ) -> Result<(), Refusal> {
        let id = self.loaded.nodes.len();
        if let (Some((p, _)), true) = (parent, attach) {
            self.loaded.nodes[p].kids.push(id);
        }
        self.loaded.nodes.push(Node { kind: Kind::RandomDraw, kids: Vec::new(), parent, function: fi });
        let kids = |steps: &mut Vec<Step<'e>>, children: Vec<&'e Expr>| {
            for (i, child) in children.into_iter().enumerate().rev() {
                steps.push(Step::Visit { expr: child, parent: Some((id, i as u32)), attach: true });
            }
        };
        let kind = match expr {
            Expr::Lit(l) => Kind::Lit(value::literal(l).ok_or_else(|| rejected(format!("invalid literal {l:?}")))?),
            Expr::Var(name) => {
                let slot = self.resolve(name).ok_or_else(|| rejected(format!("unbound name {name}")))?;
                if let SlotRef::Capture(_) = slot {
                    self.contexts.last_mut().expect("a context").uses.push(id);
                }
                Kind::Var(slot)
            }
            Expr::FnRef { module, name } => Kind::FnRef(self.resolve_fn(mi, module, name, exported)?),
            Expr::Let { name, value, body } => {
                let ctx = self.contexts.last_mut().expect("a context");
                let slot = ctx.lets;
                ctx.lets += 1;
                steps.push(Step::Unbind);
                steps.push(Step::Visit { expr: body, parent: Some((id, 1)), attach: true });
                steps.push(Step::Bind { name, slot });
                steps.push(Step::Visit { expr: value, parent: Some((id, 0)), attach: true });
                Kind::Let(slot)
            }
            Expr::Repeat { count, init, acc, body, .. } => {
                // The value so far is a `let` slot of this frame, bound over the body only (DET.55; LP2 draft).
                let ctx = self.contexts.last_mut().expect("a context");
                let slot = ctx.lets;
                ctx.lets += 1;
                steps.push(Step::Unbind);
                steps.push(Step::Visit { expr: body, parent: Some((id, 2)), attach: true });
                steps.push(Step::Bind { name: acc, slot });
                steps.push(Step::Visit { expr: init, parent: Some((id, 1)), attach: true });
                steps.push(Step::Visit { expr: count, parent: Some((id, 0)), attach: true });
                Kind::Repeat(slot)
            }
            Expr::If { .. } => {
                kids(steps, expr.children());
                Kind::If
            }
            Expr::DisplayIf { .. } => {
                kids(steps, expr.children());
                Kind::DisplayIf
            }
            Expr::Match { arms, .. } => {
                let mut values = Vec::new();
                for (l, _) in arms {
                    values.push(value::literal(l).ok_or_else(|| rejected(format!("invalid match arm {l:?}")))?);
                }
                kids(steps, expr.children());
                Kind::Match(values)
            }
            Expr::Call { callee, args } => match callee {
                Callee::Fn { module, name } => {
                    let f = self.resolve_fn(mi, module, name, exported)?;
                    if self.loaded.functions[f].params != args.len() {
                        return Err(rejected(format!(
                            "{} takes {} arguments",
                            self.loaded.functions[f].label, self.loaded.functions[f].params
                        )));
                    }
                    kids(steps, expr.children());
                    Kind::Call(f)
                }
                Callee::Value(_) => {
                    kids(steps, expr.children());
                    Kind::CallValue
                }
                Callee::Unresolved { name, .. } => return Err(rejected(format!("unresolved callee {name}"))),
            },
            Expr::Closure { params, body, .. } => {
                let code = self.loaded.codes.len();
                self.loaded.codes.push(Code { params: params.len(), captures: 0, lets: 0, slots: 0, body: id + 1 });
                let mut scope = Vec::new();
                for (i, p) in params.iter().enumerate() {
                    let pid = self.id();
                    scope.push(Binding { name: p.name.clone(), slot: SlotRef::Param(i), id: pid });
                }
                self.contexts.push(Context {
                    code,
                    scope,
                    captures: Vec::new(),
                    lets: 0,
                    uses: Vec::new(),
                    closure: Some((id, 0)),
                });
                steps.push(Step::Leave);
                steps.push(Step::Visit { expr: body, parent: Some((id, 0)), attach: false });
                Kind::Closure { code, captures: Vec::new() }
            }
            Expr::Child { caps, body, status, otherwise, .. } => {
                // The body takes no parameter, the stop branch the status if it names one; each body is set when its
                // unit opens.
                let code = self.loaded.codes.len();
                for params in [0, usize::from(status.is_some())] {
                    self.loaded.codes.push(Code { params, captures: 0, lets: 0, slots: 0, body: 0 });
                }
                steps.push(Step::Leave);
                steps.push(Step::Visit { expr: otherwise, parent: Some((id, 1)), attach: false });
                steps.push(Step::Enter { code: code + 1, param: status.as_deref(), node: id, unit: 1 });
                steps.push(Step::Leave);
                steps.push(Step::Visit { expr: body, parent: Some((id, 0)), attach: false });
                steps.push(Step::Enter { code, param: None, node: id, unit: 0 });
                Kind::Child {
                    caps: [caps.steps, caps.kernel_cost, caps.memory, caps.output],
                    body: code,
                    body_captures: Vec::new(),
                    otherwise: code + 1,
                    otherwise_captures: Vec::new(),
                }
            }
            Expr::Prim { op, args } => {
                match (arity(*op), op) {
                    (None, _) => return Err(rejected("a conversion to exact is never run (LG2.19)")),
                    (_, Prim::Stage(s)) => {
                        let note =
                            format!("the stage operation `{}` runs only in a stage's evaluator (RPI.68)", s.name());
                        return Err(rejected(note));
                    }
                    (Some(n), _) if n != args.len() => return Err(rejected(format!("{op:?} takes {n} operands"))),
                    (_, Prim::ToApprox(p)) if !approx::PRECISIONS.contains(p) => {
                        return Err(rejected(format!("unsupported precision {p}")))
                    }
                    _ => {}
                }
                kids(steps, expr.children());
                Kind::Prim(*op)
            }
            Expr::Record(fields) => {
                let mut names: Vec<Arc<Text>> = Vec::new();
                for (name, _) in fields {
                    if names.iter().any(|n| n.as_str() == name) {
                        return Err(rejected(format!("record field {name} is repeated")));
                    }
                    names.push(Text::new(name));
                }
                kids(steps, expr.children());
                Kind::Record(names)
            }
            Expr::Field { field, .. } => {
                kids(steps, expr.children());
                Kind::Field(field.as_str().into())
            }
            Expr::Collection { ty, items } => {
                let kind = match ty {
                    Ty::List(_) => Kind::List,
                    Ty::Set(_) => Kind::Set,
                    Ty::Map(..) if items.len() % 2 == 0 => Kind::Map,
                    _ => return Err(rejected(format!("a collection of type {ty:?} with {} items", items.len()))),
                };
                kids(steps, expr.children());
                kind
            }
            Expr::ReadSnapshot { query, ty } => Kind::ReadSnapshot { query: query.clone(), ty: ty.clone() },
            Expr::ReadSnapshotWith { query, ty, .. } => {
                kids(steps, expr.children());
                Kind::ReadSnapshotWith { query: query.clone(), ty: ty.clone() }
            }
            Expr::PublishView { .. } => {
                kids(steps, expr.children());
                Kind::PublishView
            }
            Expr::WriteWorkspace { record, ty, .. } => {
                kids(steps, expr.children());
                Kind::WriteWorkspace { record: record.clone(), ty: ty.clone() }
            }
            Expr::Propose { .. } => {
                kids(steps, expr.children());
                Kind::Propose
            }
            Expr::ReadReceipt { field, .. } => {
                kids(steps, expr.children());
                Kind::ReadReceipt(*field)
            }
            Expr::UseExact { role, .. } => {
                kids(steps, expr.children());
                Kind::UseExact(*role)
            }
            Expr::Accounting(a) => Kind::Accounting(*a),
            Expr::Declassified { id, ty } => Kind::Declassified { id: id.clone(), ty: ty.clone() },
            Expr::FileRead { path, ty } => Kind::FileRead { path: path.clone(), ty: ty.clone() },
            Expr::FileWrite { path, .. } => {
                kids(steps, expr.children());
                Kind::FileWrite { path: path.clone() }
            }
            Expr::RandomDraw => Kind::RandomDraw,
            Expr::ReadClock => Kind::ReadClock,
            Expr::RngInit { path } => Kind::RngInit(path.clone()),
        };
        self.loaded.nodes[id].kind = kind;
        Ok(())
    }

    /// Closes a closure body, or a child scope's body or stop branch: its captures take declaration order, and the
    /// closure or child node reads each from the enclosing frame.
    fn leave(&mut self) {
        let ctx = self.contexts.pop().expect("a closure or child context");
        let mut order: Vec<usize> = (0..ctx.captures.len()).collect();
        order.sort_by_key(|&i| ctx.captures[i].id);
        let mut perm = vec![0; order.len()];
        for (new, &old) in order.iter().enumerate() {
            perm[old] = new;
        }
        let remap = |s: &mut SlotRef| {
            if let SlotRef::Capture(i) = s {
                *i = perm[*i];
            }
        };
        for &u in &ctx.uses {
            match &mut self.loaded.nodes[u].kind {
                Kind::Var(s) => remap(s),
                Kind::Closure { captures, .. } => captures.iter_mut().for_each(remap),
                Kind::Child { body_captures, otherwise_captures, .. } => {
                    body_captures.iter_mut().chain(otherwise_captures.iter_mut()).for_each(remap)
                }
                _ => unreachable!("only variables, closures and child scopes name captures"),
            }
        }
        let outer: Vec<SlotRef> = order.iter().map(|&i| ctx.captures[i].outer).collect();
        let code = &mut self.loaded.codes[ctx.code];
        code.captures = outer.len();
        code.lets = ctx.lets;
        let (node, unit) = ctx.closure.expect("a closure or child context");
        let named = |s: &[SlotRef]| s.iter().any(|s| matches!(s, SlotRef::Capture(_)));
        // A child scope's node is patched once for both of its units: the stop branch adds it only if the body did not.
        let listed = match &self.loaded.nodes[node].kind {
            Kind::Child { body_captures, .. } if unit == 1 => named(body_captures),
            _ => false,
        };
        if named(&outer) && !listed {
            self.contexts.last_mut().expect("an enclosing context").uses.push(node);
        }
        match &mut self.loaded.nodes[node].kind {
            Kind::Closure { captures, .. } => *captures = outer,
            Kind::Child { body_captures, .. } if unit == 0 => *body_captures = outer,
            Kind::Child { otherwise_captures, .. } => *otherwise_captures = outer,
            _ => unreachable!("a closure or a child scope"),
        }
    }

    /// Sizes every frame (DET.57): parameters, captures, `let` slots and the temporaries its body holds at once. A
    /// temporary holds an operand while the operations after it run, up to its consumer. The value a node produces is
    /// its consumer's operand, not a temporary of the node, so the body's own result has no slot: it moves to the
    /// caller (DET.58). A body that holds no operand, such as a call without arguments or one host read, has no
    /// temporary.
    fn frames(&mut self) {
        let nodes = &self.loaded.nodes;
        let mut temps = vec![0u128; nodes.len()];
        for id in (0..nodes.len()).rev() {
            let kids = &nodes[id].kids;
            let t = |k: NodeId| temps[k];
            temps[id] = match &nodes[id].kind {
                Kind::Closure { .. } | Kind::Child { .. } => 0,
                _ if kids.is_empty() => 0,
                Kind::Let(_) => t(kids[0]).max(t(kids[1])),
                // The count is held while the initial value and every iteration run; the value so far has its slot.
                Kind::Repeat(_) => t(kids[0]).max(1 + t(kids[1])).max(1 + t(kids[2])),
                // The condition is the operand of `branch` or `display_if`; the selected branch runs after it.
                Kind::If | Kind::DisplayIf | Kind::Match(_) | Kind::Prim(Prim::And | Prim::Or) => {
                    kids.iter().map(|&k| t(k)).max().unwrap_or(0).max(1)
                }
                _ => kids.iter().enumerate().map(|(i, &k)| i as u128 + t(k)).max().unwrap_or(0).max(kids.len() as u128),
            };
        }
        for code in &mut self.loaded.codes {
            code.slots = (code.params + code.captures + code.lets) as u128 + temps[code.body];
        }
    }
}

/// Where a value on the work stack comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Origin {
    /// The value has its own owners.
    Owned,
    /// A borrow of a parameter or a capture, live for the whole frame.
    Frame,
    /// A borrow of a `let` slot of the current frame, or of a part of it.
    Let { slot: usize, whole: bool },
}

#[derive(Debug)]
struct Held {
    value: Value,
    owners: Vec<OwnerId>,
    origin: Origin,
}

#[derive(Debug)]
struct Frame {
    code: usize,
    /// The caller's call node; the entry frame has none.
    site: Option<NodeId>,
    params: Vec<Value>,
    captures: Vec<Value>,
    lets: Vec<Option<(Value, Vec<OwnerId>)>>,
    owner: OwnerId,
    /// Owned argument and callee temporaries, released at the return.
    temps: Vec<OwnerId>,
    /// The loops running in this frame, innermost last (DET.55; LP2 draft).
    loops: Vec<Loop>,
}

/// A running loop (DET.55; LP2 draft): its node, its count and the count's owners, which its `scope_exit` releases,
/// the index of its next test, and whether the body of iteration `k` is running, which the locator names.
#[derive(Debug)]
struct Loop {
    node: NodeId,
    count: u128,
    owners: Vec<OwnerId>,
    k: u128,
    body: bool,
}

#[derive(Clone, Copy, Debug)]
enum Task {
    Eval(NodeId),
    Apply(NodeId),
    Branch(NodeId),
    Bind(NodeId),
    ScopeExit(NodeId),
    Return,
    /// The innermost open child's body has returned (DET.60).
    ChildDone,
    /// A loop's count and initial value are evaluated (DET.55; LP2 draft).
    LoopEnter(NodeId),
    /// A loop's next test.
    LoopTest(NodeId),
    /// An iteration's body has its value.
    LoopNext(NodeId),
}

/// An open child scope (DET.60; LP2 draft): its meter scope and node, the depths its unwinding restores, and its
/// stream path.
#[derive(Debug)]
struct Open {
    child: meter::Child,
    node: NodeId,
    tasks: usize,
    frames: usize,
    values: usize,
    /// The child's stream path after the run's: its parent's, then its invocation index (DET.61).
    path: Vec<String>,
    /// The children this one has started.
    children: u64,
}

struct Machine<'a> {
    loaded: &'a Loaded,
    tariffs: Tariffs,
    draft: DraftTariffs,
    cancel: Option<&'a dyn Cancel>,
    sources: Option<&'a dyn SourceRanges>,
    menu: Option<&'a MenuAlphabet>,
    relation: Option<&'a Relation>,
    host: &'a mut dyn Host,
    meter: Meter,
    frames: Vec<Frame>,
    values: Vec<Held>,
    tasks: Vec<Task>,
    generator: Option<Cursor>,
    /// The run's seed and stream path, for `rng_init` (DET.61; LP2 draft).
    seed: Option<(&'a [u8], &'a [&'a str])>,
    /// The open child scopes, innermost last (DET.60; LP2 draft).
    open: Vec<Open>,
    /// The children the root has started.
    children: u64,
}

/// The stop locator of an operation: the call path from the entry (each caller's call site, and each running loop's
/// iteration as `label:path#body:k`), then the operation's node path and name; `entry#…` for the operations around
/// the entry call. The source range is the node's.
fn locate(
    loaded: &Loaded,
    frames: &[Frame],
    sources: Option<&dyn SourceRanges>,
    node: Option<NodeId>,
    op: &str,
) -> (String, Option<SourceRange>) {
    let Some(node) = node else {
        return (format!("entry#{op}"), None);
    };
    let mut at = String::new();
    for frame in frames {
        if let Some(site) = frame.site {
            at.push_str(&format!("{}:{} > ", loaded.functions[loaded.nodes[site].function].label, loaded.path(site)));
        }
        for l in frame.loops.iter().filter(|l| l.body) {
            let label = &loaded.functions[loaded.nodes[l.node].function].label;
            at.push_str(&format!("{label}:{}#body:{} > ", loaded.path(l.node), l.k));
        }
    }
    let function = &loaded.functions[loaded.nodes[node].function];
    let path = loaded.path(node);
    at.push_str(&format!("{}:{}#{}", function.label, path, op));
    let source = sources.and_then(|s| s.range(&loaded.modules[function.module], &function.name, &path));
    (at, source)
}

fn bits(n: u128) -> u128 {
    128 - n.leading_zeros() as u128
}

fn is_approx(v: &Value) -> bool {
    matches!(v, Value::Approx(_) | Value::ApproxBool(_))
}

fn is_compressed(v: &Value) -> bool {
    matches!(v, Value::Compressed(_))
}

fn receipt_field(field: ReceiptField) -> &'static str {
    match field {
        ReceiptField::Id => "id",
        ReceiptField::Base => "base",
        ReceiptField::Effect => "effect",
        ReceiptField::Protection => "protection",
        ReceiptField::EndState => "end_state",
        ReceiptField::N => "n",
        ReceiptField::Error => "error",
    }
}

impl<'a> Machine<'a> {
    fn charge(&mut self, node: Option<NodeId>, op: &str, kernel: u128, memory: u128, output: u128) -> Result<(), Stop> {
        let (loaded, frames, sources) = (self.loaded, &self.frames, self.sources);
        if let Some(why) = self.cancel.and_then(|c| c.pending()) {
            let (at, source) = locate(loaded, frames, sources, node, op);
            return Err(self.meter.interrupted(why, &at, source.as_ref()));
        }
        self.meter.charge_at(|| locate(loaded, frames, sources, node, op), kernel, memory, output)
    }

    fn fail(&self, node: Option<NodeId>, op: &str, code: &'static str) -> Stop {
        let (at, source) = locate(self.loaded, &self.frames, self.sources, node, op);
        self.meter.error(code, &at, source.as_ref())
    }

    fn own(&mut self, size: u128) -> Vec<OwnerId> {
        vec![self.meter.own(size)]
    }

    fn release(&mut self, held: Held) {
        for owner in held.owners {
            self.meter.drop_owner(owner, true);
        }
    }

    fn push(&mut self, value: Value, owners: Vec<OwnerId>) {
        self.values.push(Held { value, owners, origin: Origin::Owned });
    }

    fn frame(&self) -> &Frame {
        self.frames.last().expect("a frame")
    }

    fn read_slot(&self, slot: SlotRef) -> (Value, Origin) {
        let frame = self.frame();
        match slot {
            SlotRef::Param(i) => (frame.params[i].clone(), Origin::Frame),
            SlotRef::Capture(i) => (frame.captures[i].clone(), Origin::Frame),
            SlotRef::Let(i) => {
                (frame.lets[i].as_ref().expect("a bound let").0.clone(), Origin::Let { slot: i, whole: true })
            }
        }
    }

    /// Admits a result reservation, then shrinks it to the value's size and gives the value an owner.
    fn settle(&mut self, reserved: u128, value: Value) {
        let size = value.size();
        self.meter.release(reserved.checked_sub(size).expect("a result within its reservation"));
        let owners = self.own(size);
        self.push(value, owners);
    }

    fn execute(&mut self) -> Result<(), Stop> {
        while let Some(task) = self.tasks.pop() {
            let done = match task {
                Task::Eval(n) => self.eval(n),
                Task::Apply(n) => self.apply(n),
                Task::Branch(n) => self.branch(n),
                Task::Bind(n) => self.bind(n),
                Task::ScopeExit(n) => self.scope_exit(n),
                Task::Return => self.ret(),
                Task::ChildDone => self.child_done(),
                Task::LoopEnter(n) => self.loop_enter(n),
                Task::LoopTest(n) => self.loop_test(n),
                Task::LoopNext(n) => self.loop_next(n),
            };
            if let Err(stop) = done {
                self.unwind(stop)?;
            }
        }
        Ok(())
    }

    /// `child` (DET.60; LP2 draft): one step in the enclosing scopes, then the body runs in a scope of its own under
    /// the effective caps, in a frame of its own that borrows the slots it names from the enclosing frame.
    fn child(&mut self, n: NodeId, caps: [Option<u128>; 4], code: usize, captures: &[SlotRef]) -> Result<(), Stop> {
        self.charge(Some(n), "child", 0, 0, 0)?;
        let (at, _) = locate(self.loaded, &self.frames, self.sources, Some(n), "child");
        let child = self.meter.open_child(at.strip_suffix("#child").expect("a child locator"), caps);
        let (index, mut path) = match self.open.last_mut() {
            Some(parent) => {
                parent.children += 1;
                (parent.children - 1, parent.path.clone())
            }
            None => {
                self.children += 1;
                (self.children - 1, Vec::new())
            }
        };
        path.push(index.to_string());
        let captures = captures.iter().map(|&s| self.read_slot(s).0).collect();
        let (tasks, frames, values) = (self.tasks.len(), self.frames.len(), self.values.len());
        self.open.push(Open { child, node: n, tasks, frames, values, path, children: 0 });
        self.tasks.push(Task::ChildDone);
        self.enter(Some(n), code, Vec::new(), captures, Vec::new())
    }

    /// The innermost child's body returned: the child closes, and its result moves to the parent (DET.60).
    fn child_done(&mut self) -> Result<(), Stop> {
        let open = self.open.pop().expect("an open child");
        let result = self.values.last().expect("the child's result");
        let returned = result.owners.iter().map(|&o| self.meter.size(o)).sum();
        self.meter.leave_child(open.child, Ok(returned))
    }

    /// Unwinds the open children a stop ends, innermost first (DET.60): each one's tasks, frames and values above its
    /// entry go, and the meter drops the owners it counts. The child whose own scope the stop ends absorbs it, and its
    /// stop branch runs in the parent; a stop of the stop branch goes on in the same way. With no child left, the stop
    /// ends the run.
    fn unwind(&mut self, mut stop: Stop) -> Result<(), Stop> {
        while let Some(open) = self.open.pop() {
            self.tasks.truncate(open.tasks);
            self.frames.truncate(open.frames);
            self.values.truncate(open.values);
            let status = stop.outcome.name();
            stop = match self.meter.leave_child(open.child, Err(stop)) {
                Err(stop) => stop,
                Ok(()) => match self.otherwise(open.node, status) {
                    Ok(()) => return Ok(()),
                    Err(stop) => stop,
                },
            };
        }
        Err(stop)
    }

    /// A child's stop branch (DET.60; LP2 draft): when the branch names a status, `status` settles the stop's outcome,
    /// `cut_off` or `error`, as a `Status` owned by the parent; the branch's code unit then runs with it, borrowing the
    /// enclosing frame's slots.
    fn otherwise(&mut self, n: NodeId, status: &'static str) -> Result<(), Stop> {
        let loaded = self.loaded;
        let Kind::Child { otherwise, otherwise_captures, .. } = &loaded.nodes[n].kind else { unreachable!("a child") };
        let mut args = Vec::new();
        if loaded.codes[*otherwise].params == 1 {
            self.charge(Some(n), "status", 0, 4, 0)?;
            self.settle(4, Value::tag(status));
            args.push(self.values.pop().expect("the status"));
        }
        let captures = otherwise_captures.iter().map(|&s| self.read_slot(s).0).collect();
        self.enter(Some(n), *otherwise, args, captures, Vec::new())
    }

    fn eval(&mut self, n: NodeId) -> Result<(), Stop> {
        let loaded = self.loaded;
        let node = &loaded.nodes[n];
        match &node.kind {
            Kind::Lit(v) => {
                // A menu-word literal takes the run's pinned alphabet; a letter outside it, or no alphabet, stops the
                // run before the step (LB1.8).
                let v = match v {
                    Value::MenuWord(w) => match self.menu {
                        Some(menu) if w.letters().iter().all(|&l| menu.holds(l)) => {
                            Value::MenuWord(Arc::new(w.named(menu.shared_identity().clone())))
                        }
                        _ => return Err(self.fail(Some(n), "value", code::INCOMPATIBLE_ALPHABET)),
                    },
                    v => v.clone(),
                };
                let size = v.size();
                self.charge(Some(n), "value", 0, size, 0)?;
                let owners = self.own(size);
                self.push(v, owners);
            }
            Kind::Var(slot) => {
                self.charge(Some(n), "visit", 0, 0, 0)?;
                let (value, origin) = self.read_slot(*slot);
                self.values.push(Held { value, owners: Vec::new(), origin });
            }
            Kind::FnRef(f) => {
                self.charge(Some(n), "value", 0, 32, 0)?;
                let owners = self.own(32);
                self.push(Value::Fn(*f), owners);
            }
            Kind::Let(_) => {
                self.tasks.push(Task::Bind(n));
                self.tasks.push(Task::Eval(node.kids[0]));
            }
            Kind::Repeat(_) => {
                self.tasks.push(Task::LoopEnter(n));
                self.tasks.push(Task::Eval(node.kids[1]));
                self.tasks.push(Task::Eval(node.kids[0]));
            }
            Kind::If | Kind::DisplayIf | Kind::Match(_) | Kind::Prim(Prim::And | Prim::Or) => {
                self.tasks.push(Task::Branch(n));
                self.tasks.push(Task::Eval(node.kids[0]));
            }
            Kind::Closure { code, captures } => {
                self.charge(Some(n), "value", 0, 32, 0)?;
                let mut owners = self.own(32);
                let mut values = Vec::new();
                for &slot in captures {
                    let (v, _) = self.read_slot(slot);
                    let size = v.size();
                    self.charge(Some(n), "alias", 0, size, 0)?;
                    owners.extend(self.own(size));
                    values.push(v);
                }
                self.push(Value::Closure(Arc::new(Closure::new(*code, values))), owners);
            }
            Kind::ReadSnapshot { query, ty } => self.host_read(n, "kernel", Request::Snapshot { query, ty }, 0)?,
            Kind::Declassified { id, ty } => self.host_read(n, "kernel", Request::Declassified { id, ty }, 0)?,
            Kind::FileRead { path, ty } => self.host_read(n, "read", Request::FileRead { path, ty }, 0)?,
            Kind::ReadClock => self.host_read(n, "read", Request::Clock, 0)?,
            Kind::Accounting(a) => self.accounting(n, *a)?,
            Kind::RandomDraw => self.draw(n)?,
            Kind::RngInit(segments) => self.rng_init(n, segments)?,
            Kind::Child { caps, body, body_captures, .. } => self.child(n, *caps, *body, body_captures)?,
            _ => {
                self.tasks.push(Task::Apply(n));
                self.tasks.extend(node.kids.iter().rev().map(|&k| Task::Eval(k)));
            }
        }
        Ok(())
    }

    fn bind(&mut self, n: NodeId) -> Result<(), Stop> {
        let Kind::Let(slot) = self.loaded.nodes[n].kind else { unreachable!("a let") };
        let held = self.values.pop().expect("the let value");
        let filled = self.fill(n, held)?;
        self.frames.last_mut().expect("a frame").lets[slot] = Some(filled);
        self.tasks.push(Task::ScopeExit(n));
        self.tasks.push(Task::Eval(self.loaded.nodes[n].kids[1]));
        Ok(())
    }

    /// What a slot holds once filled with a value: `visit` moves an owned value in, and `alias` retains a borrowed
    /// one with a charged owner (DET.55, DET.58).
    fn fill(&mut self, n: NodeId, held: Held) -> Result<(Value, Vec<OwnerId>), Stop> {
        if held.origin == Origin::Owned {
            self.charge(Some(n), "visit", 0, 0, 0)?;
            Ok((held.value, held.owners))
        } else {
            let size = held.value.size();
            self.charge(Some(n), "alias", 0, size, 0)?;
            // An aliased generator draws from a cursor of its own (DET.61).
            Ok((held.value.alias(), self.own(size)))
        }
    }

    /// A loop's start (DET.55; LP2 draft): the initial value fills the loop's slot as a `let`'s value fills its own,
    /// and the count is held until the loop's `scope_exit`. A count that is no natural number is refused at the first
    /// test, which would read it, after the initial value fills the slot; the cleanup releases both. A count beyond
    /// `u128` saturates, since no budget admits that many tests.
    fn loop_enter(&mut self, n: NodeId) -> Result<(), Stop> {
        let Kind::Repeat(slot) = self.loaded.nodes[n].kind else { unreachable!("a loop") };
        let init = self.values.pop().expect("the initial value");
        let count = self.values.pop().expect("the count");
        let times = match &count.value {
            Value::Int(i) if i.sign() != Sign::Minus => Some(u128::try_from(&**i).unwrap_or(u128::MAX)),
            _ => None,
        };
        let filled = self.fill(n, init)?;
        self.frames.last_mut().expect("a frame").lets[slot] = Some(filled);
        let Some(times) = times else { return Err(self.fail(Some(n), "loop_test:0", code::IR_REJECTED)) };
        let frame = self.frames.last_mut().expect("a frame");
        frame.loops.push(Loop { node: n, count: times, owners: count.owners, k: 0, body: false });
        self.tasks.push(Task::LoopTest(n));
        Ok(())
    }

    /// `loop_test` (DET.55): every test costs one step, the final false one too. A true test runs the body of
    /// iteration k; the final one ends the loop with `scope_exit`, which releases the count after admission and moves
    /// the value so far out of its slot, as a `let` whose body returns its slot.
    fn loop_test(&mut self, n: NodeId) -> Result<(), Stop> {
        let Kind::Repeat(slot) = self.loaded.nodes[n].kind else { unreachable!("a loop") };
        let running = self.frame().loops.last().expect("a running loop");
        debug_assert_eq!(running.node, n, "the innermost running loop");
        let (k, count) = (running.k, running.count);
        self.charge(Some(n), &format!("loop_test:{k}"), 0, 0, 0)?;
        if k < count {
            self.frames.last_mut().expect("a frame").loops.last_mut().expect("a running loop").body = true;
            self.tasks.push(Task::LoopNext(n));
            self.tasks.push(Task::Eval(self.loaded.nodes[n].kids[2]));
            return Ok(());
        }
        self.charge(Some(n), "scope_exit", 0, 0, 0)?;
        let frame = self.frames.last_mut().expect("a frame");
        let done = frame.loops.pop().expect("a running loop");
        let (value, owners) = frame.lets[slot].take().expect("a filled loop slot");
        for owner in done.owners {
            self.meter.drop_owner(owner, true);
        }
        self.push(value, owners);
        Ok(())
    }

    /// The end of an iteration (DET.55; LP2 draft): the body's value refills the loop's slot, with `visit` for an
    /// owned value or the slot's own, and `alias` for any other borrow. The old value is released after admission,
    /// so it cannot fund its replacement (DET.56).
    fn loop_next(&mut self, n: NodeId) -> Result<(), Stop> {
        let Kind::Repeat(slot) = self.loaded.nodes[n].kind else { unreachable!("a loop") };
        let held = self.values.pop().expect("the body's value");
        if held.origin == (Origin::Let { slot, whole: true }) {
            self.charge(Some(n), "visit", 0, 0, 0)?;
        } else {
            let filled = self.fill(n, held)?;
            let old = self.frames.last_mut().expect("a frame").lets[slot].replace(filled);
            for owner in old.expect("a filled loop slot").1 {
                self.meter.drop_owner(owner, true);
            }
        }
        let running = self.frames.last_mut().expect("a frame").loops.last_mut().expect("a running loop");
        running.k += 1;
        running.body = false;
        self.tasks.push(Task::LoopTest(n));
        Ok(())
    }

    fn scope_exit(&mut self, n: NodeId) -> Result<(), Stop> {
        let Kind::Let(slot) = self.loaded.nodes[n].kind else { unreachable!("a let") };
        let mut result = self.values.pop().expect("the let body's value");
        match result.origin {
            Origin::Let { slot: s, whole: true } if s == slot => {
                self.charge(Some(n), "scope_exit", 0, 0, 0)?;
                let (_, owners) = self.frames.last_mut().expect("a frame").lets[slot].take().expect("a bound let");
                result.owners = owners;
                result.origin = Origin::Owned;
            }
            origin => {
                if origin == (Origin::Let { slot, whole: false }) {
                    let size = result.value.size();
                    self.charge(Some(n), "alias", 0, size, 0)?;
                    result.value = result.value.alias();
                    result.owners = self.own(size);
                    result.origin = Origin::Owned;
                }
                self.charge(Some(n), "scope_exit", 0, 0, 0)?;
                let (_, owners) = self.frames.last_mut().expect("a frame").lets[slot].take().expect("a bound let");
                for owner in owners {
                    self.meter.drop_owner(owner, true);
                }
            }
        }
        self.values.push(result);
        Ok(())
    }

    fn branch(&mut self, n: NodeId) -> Result<(), Stop> {
        let loaded = self.loaded;
        let node = &loaded.nodes[n];
        let held = self.values.pop().expect("a condition");
        let next = match (&node.kind, &held.value) {
            (Kind::If, Value::Bool(b)) => {
                self.charge(Some(n), "branch", 0, 0, 0)?;
                node.kids[if *b { 1 } else { 2 }]
            }
            (Kind::DisplayIf, Value::ApproxBool(b)) => {
                self.charge(Some(n), "display_if", 0, 0, 0)?;
                node.kids[if *b { 1 } else { 2 }]
            }
            (Kind::Match(arms), v) => {
                // A match on an end test's verdict names `unestablished` in an arm of its own, or both established
                // outcomes, so its `_` arm never takes unknown together with yes or no (LG2.9; LP2 draft). The checker
                // refuses such a match on a verdict's type; behind a type variable, the run refuses it here.
                if let Value::EndTest(..) = v {
                    let named =
                        |o: EndTestOutcome| arms.iter().any(|a| matches!(a, Value::Tag(t) if t.as_str() == o.tag()));
                    let taken: Vec<EndTestOutcome> = EndTestOutcome::ALL.into_iter().filter(|o| !named(*o)).collect();
                    if taken.len() > 1 && taken.contains(&EndTestOutcome::Unestablished) {
                        return Err(self.fail(Some(n), "branch", code::IR_REJECTED));
                    }
                }
                let kernel = arms
                    .iter()
                    .fold(0u128, |s, a| s.saturating_add(value::equal_price(v, a, &self.tariffs, &self.draft)));
                self.charge(Some(n), "branch", kernel, 0, 0)?;
                let arm = arms.iter().position(|a| pattern_matches(v, a)).unwrap_or(arms.len());
                node.kids[1 + arm]
            }
            (Kind::Prim(op), Value::Bool(b)) => {
                self.charge(Some(n), "branch", 0, 0, 0)?;
                if *b == (*op == Prim::Or) {
                    self.values.push(held);
                    return Ok(());
                }
                node.kids[1]
            }
            _ => {
                let op = if matches!(node.kind, Kind::DisplayIf) { "display_if" } else { "branch" };
                return Err(self.fail(Some(n), op, code::IR_REJECTED));
            }
        };
        self.release(held);
        self.tasks.push(Task::Eval(next));
        Ok(())
    }

    fn apply(&mut self, n: NodeId) -> Result<(), Stop> {
        let loaded = self.loaded;
        let node = &loaded.nodes[n];
        let mut args = self.values.split_off(self.values.len() - node.kids.len());
        match &node.kind {
            Kind::Call(f) => self.call(Some(n), loaded.functions[*f].code, args, None),
            Kind::CallValue => {
                let callee = args.pop().expect("a callee");
                let (code, ok) = match &callee.value {
                    Value::Fn(f) => (loaded.functions[*f].code, true),
                    Value::Closure(c) => (c.code, true),
                    _ => (0, false),
                };
                if !ok || loaded.codes[code].params != args.len() {
                    return Err(self.fail(Some(n), "call", code::IR_REJECTED));
                }
                self.call(Some(n), code, args, Some(callee))
            }
            Kind::Prim(op) => self.prim(n, *op, args),
            Kind::Record(names) => self.record(n, names, args),
            Kind::List => self.list(n, args),
            Kind::Set => self.keyed(n, false, args),
            Kind::Map => self.keyed(n, true, args),
            Kind::Field(name) => self.field(n, args.pop().expect("a record"), name, "visit"),
            Kind::ReadReceipt(field) => self.read_receipt(n, args.pop().expect("a receipt"), *field),
            Kind::PublishView => self.output(n, Sink::View, args.pop().expect("a value")),
            Kind::WriteWorkspace { record, ty } => {
                self.output(n, Sink::Workspace { record, ty }, args.pop().expect("a value"))
            }
            Kind::FileWrite { path } => self.output(n, Sink::File { path }, args.pop().expect("a value")),
            Kind::Propose => self.propose(n, args),
            Kind::ReadSnapshotWith { query, ty } => self.read_with(n, query, ty, args.pop().expect("an argument")),
            Kind::UseExact(role) => {
                let held = args.pop().expect("a value");
                if is_approx(&held.value) {
                    return Err(self.fail(Some(n), "use_exact", code::IR_REJECTED));
                }
                // The host gets the canonical bytes, as at every sink; a value with none, a function or a receipt
                // among them, stops the run before the host is asked (PHS.52, owner decision of 16 November 2026).
                let (bytes, buffer) = self.encode(Some(n), &held.value)?;
                self.charge(Some(n), "use_exact", 0, 0, 0)?;
                if let Err(c) = self.host.sink(&Sink::Exact { role: *role }, &held.value, &bytes) {
                    return Err(self.fail(Some(n), "use_exact", c));
                }
                self.meter.drop_owner(buffer, true);
                self.release(held);
                self.push(Value::Unit, Vec::new());
                Ok(())
            }
            _ => unreachable!("an operation with operands"),
        }
    }

    /// The `call` operation and the callee's frame. Arguments are borrowed; owned temporaries wait for the return.
    fn call(&mut self, site: Option<NodeId>, code: usize, args: Vec<Held>, callee: Option<Held>) -> Result<(), Stop> {
        let (mut captures, mut owners) = (Vec::new(), Vec::new());
        if let Some(h) = callee {
            if let Value::Closure(closure) = &h.value {
                captures = closure.captures().to_vec();
            }
            owners = h.owners;
        }
        self.enter(site, code, args, captures, owners)
    }

    /// The `call` operation and a code unit's frame (DET.57): its arguments, its captures (a closure's, or the slots
    /// a child scope borrows from the enclosing frame) and the callee's owners, which are released at the return.
    fn enter(
        &mut self,
        site: Option<NodeId>,
        code: usize,
        args: Vec<Held>,
        captures: Vec<Value>,
        callee: Vec<OwnerId>,
    ) -> Result<(), Stop> {
        let c = &self.loaded.codes[code];
        let bytes = frame_size(c.slots);
        self.charge(site, "call", 0, bytes, 0)?;
        let owner = self.meter.own(bytes);
        let mut temps = Vec::new();
        let mut params = Vec::new();
        for a in args {
            params.push(a.value);
            temps.extend(a.owners);
        }
        temps.extend(callee);
        let lets = vec![None; c.lets];
        self.frames.push(Frame { code, site, params, captures, lets, owner, temps, loops: Vec::new() });
        self.tasks.push(Task::Return);
        self.tasks.push(Task::Eval(c.body));
        Ok(())
    }

    /// The `return` operation: a borrowed result first gets a charged owner; then the frame and the temporaries are
    /// released and the result moves to the caller (DET.58).
    fn ret(&mut self) -> Result<(), Stop> {
        let body = self.loaded.codes[self.frame().code].body;
        let mut result = self.values.pop().expect("a result");
        if result.origin != Origin::Owned {
            let size = result.value.size();
            self.charge(Some(body), "alias", 0, size, 0)?;
            result.value = result.value.alias();
            result.owners = self.own(size);
            result.origin = Origin::Owned;
        }
        self.charge(Some(body), "return", 0, 0, 0)?;
        let frame = self.frames.pop().expect("a frame");
        self.meter.drop_owner(frame.owner, true);
        for (_, owners) in frame.lets.into_iter().flatten() {
            for owner in owners {
                self.meter.drop_owner(owner, true);
            }
        }
        for owner in frame.temps {
            self.meter.drop_owner(owner, true);
        }
        self.values.push(result);
        Ok(())
    }

    fn prim(&mut self, n: NodeId, op: Prim, args: Vec<Held>) -> Result<(), Stop> {
        let (t, d) = (self.tariffs, self.draft);
        let (a, b) = (&args[0].value, args.get(1).map(|h| &h.value));
        let name = match op {
            Prim::ExactAdd => "exact_add",
            Prim::ExactMul => "exact_mul",
            Prim::Q5Parts => "q5_parts",
            Prim::Q5Of => return self.q5_of(n, args),
            Prim::Assert => "assert",
            Prim::RngDraw => "rng_draw",
            Prim::ExactLess => "exact_less",
            Prim::ExactEq => "exact_eq",
            Prim::Not => "not",
            Prim::ToApprox(_) => "to_approx",
            Prim::ApproxAdd => "approx_add",
            Prim::ApproxLess => "approx_less",
            Prim::ApproxAnd => "approx_and",
            Prim::ApproxNot => "approx_not",
            Prim::And | Prim::Or | Prim::ToExact => unreachable!("branches and refused conversions"),
            Prim::Length
            | Prim::Index
            | Prim::Push
            | Prim::Concat
            | Prim::Inverse
            | Prim::Conjugate
            | Prim::Commutator
            | Prim::Compose
            | Prim::WordKey
            | Prim::TraceKey
            | Prim::Compress
            | Prim::Expand => return self.sequence(n, op, args),
            Prim::Contains
            | Prim::Insert
            | Prim::InsertEntry
            | Prim::Lookup
            | Prim::Elements(_)
            | Prim::Keys(_)
            | Prim::Entries(_) => return self.keyed_op(n, op, args),
            Prim::ToField(_)
            | Prim::Residue
            | Prim::FieldAdd
            | Prim::FieldSub
            | Prim::FieldNeg
            | Prim::FieldMul
            | Prim::FieldInverse => return self.finite_field(n, op, args),
            Prim::TextDecode => return self.text_decode(n, args),
            Prim::WordStream => return self.word_stream(n, args),
            Prim::StreamDigest => return self.stream_digest(n, args),
            Prim::Stage(_) => unreachable!("refused when loaded"),
        };
        let display = |v: &Value| match v {
            Value::Bool(x) | Value::ApproxBool(x) => Some(*x),
            _ => None,
        };
        // The row is priced from the operands' metadata, and mistyped operands are rejected before any charge. The
        // result is computed only after admission.
        let exact = |v: &Value| matches!(v, Value::Int(_) | Value::Rat(_) | Value::Q5(_));
        let price = match (op, a, b) {
            (Prim::ExactAdd, a, Some(b)) => value::add_price(a, b, &t, &d),
            (Prim::ExactMul, a, Some(b)) => value::mul_price(a, b, &t, &d),
            (Prim::Q5Parts, a, None) => value::q5_parts_price(a, &d),
            // One step and no memory: the predicate was evaluated before, with ordinary operations (DET.55).
            (Prim::Assert, Value::Bool(_), None) => Some((0, 0)),
            // One block of the generator's stream and its 256-bit integer, as the run's own draw (DET.61).
            (Prim::RngDraw, Value::Generator(_), None) => Some((t.rng_block, integer_size(256))),
            (Prim::ExactLess, a, Some(b)) => value::compare_price(a, b, &d).map(|k| (k, 1)),
            // A compressed word has no exact equality, and an end test's verdict is matched on its outcomes only, so
            // an equality of verdicts behind a type variable stops here (LP2 drafts; LG2.9).
            (Prim::ExactEq, a, Some(b))
                if !is_approx(a)
                    && !is_approx(b)
                    && !is_compressed(a)
                    && !is_compressed(b)
                    && !a.holds_end_test()
                    && !b.holds_end_test() =>
            {
                Some((value::equal_price(a, b, &t, &d), 1))
            }
            (Prim::Not, Value::Bool(_), None) => Some((0, 1)),
            (Prim::ToApprox(p), a, None) if exact(a) => Some((value::convert_price(a, p, &d), approx::width(p))),
            (Prim::ApproxAdd, Value::Approx(x), Some(Value::Approx(y))) if x.precision == y.precision => {
                Some((0, approx::width(x.precision)))
            }
            (Prim::ApproxLess, Value::Approx(x), Some(Value::Approx(y))) if x.precision == y.precision => Some((0, 1)),
            (Prim::ApproxAnd, a, Some(b)) if display(a).is_some() && display(b).is_some() => Some((0, 1)),
            (Prim::ApproxNot, a, None) if display(a).is_some() => Some((0, 1)),
            _ => None,
        };
        let Some((kernel, memory)) = price else {
            return Err(self.fail(Some(n), name, code::IR_REJECTED));
        };
        self.charge(Some(n), name, kernel, memory, 0)?;
        let result = match (op, a, b) {
            (Prim::ExactAdd, a, Some(b)) => value::add(a, b).expect("priced operands add"),
            (Prim::ExactMul, a, Some(b)) => value::mul(a, b).expect("priced operands multiply"),
            (Prim::Q5Parts, a, None) => value::q5_parts(a).expect("a priced number"),
            // A false predicate ends the run before the guarded operation, and the assertion keeps its admitted step
            // (DET.55, DET.56).
            (Prim::Assert, Value::Bool(false), None) => {
                return Err(self.fail(Some(n), name, meter::code::POINTWISE_FAILED));
            }
            (Prim::Assert, Value::Bool(true), None) => Value::Unit,
            // The draw advances the cursor the operand shares with its owner; only a successful draw does.
            (Prim::RngDraw, Value::Generator(g), None) => match g.draw() {
                Some(block) => Value::int(BigInt::from_bytes_be(Sign::Plus, &block)),
                None => return Err(self.fail(Some(n), name, meter::code::GENERATOR_EXHAUSTED)),
            },
            (Prim::ExactLess, a, Some(b)) => Value::Bool(value::compare(a, b) == Some(std::cmp::Ordering::Less)),
            (Prim::ExactEq, a, Some(b)) => Value::Bool(value::equal(a, b)),
            (Prim::Not, Value::Bool(x), None) => Value::Bool(!x),
            (Prim::ToApprox(p), a, None) => Value::Approx(value::to_approx(a, p).expect("an exact number")),
            (Prim::ApproxAdd, Value::Approx(x), Some(Value::Approx(y))) => Value::Approx(x.add(*y)),
            (Prim::ApproxLess, Value::Approx(x), Some(Value::Approx(y))) => {
                Value::ApproxBool(x.compare(y) == std::cmp::Ordering::Less)
            }
            (Prim::ApproxAnd, a, Some(b)) => Value::ApproxBool(display(a) == Some(true) && display(b) == Some(true)),
            (Prim::ApproxNot, a, None) => Value::ApproxBool(display(a) == Some(false)),
            _ => unreachable!("a priced operation"),
        };
        self.settle(memory, result);
        let result = self.values.pop().expect("the result");
        for held in args {
            self.release(held);
        }
        self.values.push(result);
        Ok(())
    }

    /// `q5_of(a, b, d)` (LP2 draft): the Q(√5) value (a + b√5)/d of three integers, in canonical form. The row is
    /// priced as the product of a + b√5 and 1/d (DET.57, [`value::q5_of_price`]), and a mistyped operand is rejected
    /// before any charge. A zero denominator has no value, so the operation is guarded by a pointwise assertion
    /// (DET.55), as `field_inverse` is: its predicate `denominator_nonzero` reserves its `Bool`, the `assert` step
    /// consumes it, and only then does the construction run. A zero `d` stops with `LG4:input.pointwise_failed` after
    /// the `assert` step and keeps the admitted charges (chapter 13).
    fn q5_of(&mut self, n: NodeId, args: Vec<Held>) -> Result<(), Stop> {
        let (t, dt) = (self.tariffs, self.draft);
        let (a, b, d) = (&args[0].value, &args[1].value, &args[2].value);
        let Some((kernel, memory)) = value::q5_of_price(a, b, d, &t, &dt) else {
            return Err(self.fail(Some(n), "q5_of", code::IR_REJECTED));
        };
        self.charge(Some(n), "denominator_nonzero", 0, 1, 0)?;
        self.charge(Some(n), "assert", 0, 0, 0)?;
        // The assertion consumes the predicate's Bool.
        self.meter.release(1);
        if matches!(d, Value::Int(z) if z.sign() == Sign::NoSign) {
            return Err(self.fail(Some(n), "assert", meter::code::POINTWISE_FAILED));
        }
        // The normalisation runs only after its row is admitted (DET.55).
        self.charge(Some(n), "q5_of", kernel, memory, 0)?;
        let result = value::q5_of(a, b, d).expect("priced integers with a nonzero denominator");
        self.settle(memory, result);
        let result = self.values.pop().expect("the result");
        for held in args {
            self.release(held);
        }
        self.values.push(result);
        Ok(())
    }

    /// The F2 and F5 operations (KN3.2; LP2 drafts; DET.55 names no row for them), one step each, with mistyped
    /// operands rejected before any charge. `to_f2` and `to_f5` take an integer of any size, so they are priced by it
    /// (DET.55: no unmetered big-integer operation): the integer row's price for adding the prime to it, one walk over
    /// its limbs. The others work on residues below 5 and cost no kernel units. `field_inverse` is guarded by a
    /// pointwise assertion (DET.55): its predicate `field_nonzero` reserves its `Bool`, the `assert` step consumes it,
    /// and only then does the inverse run. The inverse of zero stops with `LG4:input.pointwise_failed` after the
    /// `assert` step and keeps the admitted charges (chapter 13).
    fn finite_field(&mut self, n: NodeId, op: Prim, args: Vec<Held>) -> Result<(), Stop> {
        let name = match op {
            Prim::ToField(Field::F2) => "to_f2",
            Prim::ToField(Field::F5) => "to_f5",
            Prim::Residue => "residue",
            Prim::FieldAdd => "field_add",
            Prim::FieldSub => "field_sub",
            Prim::FieldNeg => "field_neg",
            Prim::FieldMul => "field_mul",
            _ => "field_inverse",
        };
        // An element of a field, with its residue below the prime; any other operand is mistyped.
        let element = |v: Option<&Value>| match v {
            Some(Value::Field(f, r)) if *r < f.modulus() => Some((*f, *r)),
            _ => None,
        };
        let (a, b) = (args.first().map(|h| &h.value), args.get(1).map(|h| &h.value));
        let (x, y) = (element(a), element(b));
        // The row is priced from the operands' metadata before any work.
        let (kernel, memory) = match (op, a, x, b, y) {
            (Prim::ToField(field), Some(Value::Int(i)), _, None, _) => {
                let prime = bits(u128::from(field.modulus()));
                (self.tariffs.integer(meter::IntOp::Add, i.bits() as u128, prime).0, 1)
            }
            // A residue below 5 is an integer of one byte.
            (Prim::Residue, _, Some(_), None, _) => (0, integer_size(8)),
            (Prim::FieldNeg | Prim::FieldInverse, _, Some(_), None, _) => (0, 1),
            (Prim::FieldAdd | Prim::FieldSub | Prim::FieldMul, _, Some((f, _)), _, Some((g, _))) if f == g => (0, 1),
            _ => return Err(self.fail(Some(n), name, code::IR_REJECTED)),
        };
        if op == Prim::FieldInverse {
            self.charge(Some(n), "field_nonzero", 0, 1, 0)?;
            self.charge(Some(n), "assert", 0, 0, 0)?;
            // The assertion consumes the predicate's Bool.
            self.meter.release(1);
            if x.is_some_and(|(_, r)| r == 0) {
                return Err(self.fail(Some(n), "assert", meter::code::POINTWISE_FAILED));
            }
        }
        self.charge(Some(n), name, kernel, memory, 0)?;
        let result = match (op, a, x, y) {
            (Prim::ToField(field), Some(Value::Int(i)), _, _) => Value::field(field, i),
            (Prim::Residue, _, Some((_, r)), _) => Value::int(r),
            (op, _, Some((f, r)), y) => {
                let (p, s) = (f.modulus(), y.map_or(0, |(_, s)| s));
                let residue = match op {
                    Prim::FieldAdd => (r + s) % p,
                    Prim::FieldSub => (r + p - s) % p,
                    Prim::FieldMul => r * s % p,
                    Prim::FieldNeg => (p - r) % p,
                    _ => (1..p).find(|t| r * t % p == 1).expect("a nonzero element's inverse"),
                };
                Value::Field(f, residue)
            }
            _ => unreachable!("a priced operation"),
        };
        self.settle(memory, result);
        let result = self.values.pop().expect("the result");
        for held in args {
            self.release(held);
        }
        self.values.push(result);
        Ok(())
    }

    /// `text_decode(b)` (DET.62; LP2 draft): one step with the input-byte tariff on every byte and a reservation of
    /// 4 bytes per input byte, the bound of a text of that many scalars, admitted before any byte is read. Invalid
    /// UTF-8 or a surrogate stops the run with `LG4:input.text_scalar_invalid`, binding no value and keeping the
    /// reservation until cleanup; a decoded text settles to its 4 bytes per scalar (DET.57).
    fn text_decode(&mut self, n: NodeId, args: Vec<Held>) -> Result<(), Stop> {
        let Some(Value::Bytes(data)) = args.first().map(|h| &h.value) else {
            return Err(self.fail(Some(n), "text_decode", code::IR_REJECTED));
        };
        let data = Arc::clone(data);
        let reserved = (data.len() as u128).saturating_mul(4);
        let kernel = self.tariffs.decode_per_byte.saturating_mul(data.len() as u128);
        self.charge(Some(n), "text_decode", kernel, reserved, 0)?;
        let Ok(text) = std::str::from_utf8(&data) else {
            return Err(self.fail(Some(n), "text_decode", meter::code::TEXT_SCALAR_INVALID));
        };
        self.settle(reserved, Value::text(text));
        let result = self.values.pop().expect("the result");
        for held in args {
            self.release(held);
        }
        self.values.push(result);
        Ok(())
    }

    fn record(&mut self, n: NodeId, names: &[Arc<Text>], args: Vec<Held>) -> Result<(), Stop> {
        let values: Vec<Value> = args.iter().map(|h| h.value.clone()).collect();
        let header = Record::header(&values);
        self.charge(Some(n), "value", 0, header, 0)?;
        let mut owners = self.own(header);
        for held in args {
            if held.origin == Origin::Owned {
                self.charge(Some(n), "visit", 0, 0, 0)?;
                owners.extend(held.owners);
            } else {
                let size = held.value.size();
                self.charge(Some(n), "alias", 0, size, 0)?;
                owners.extend(self.own(size));
            }
        }
        let record = Record::new(names.iter().cloned().zip(values).collect());
        self.push(Value::Record(Arc::new(record)), owners);
        Ok(())
    }

    /// A list, built as a record is: `value` for the header, then per item `visit` (a move) or `alias` (a borrow).
    fn list(&mut self, n: NodeId, args: Vec<Held>) -> Result<(), Stop> {
        let values: Vec<Value> = args.iter().map(|h| h.value.clone()).collect();
        let header = List::header(&values);
        self.charge(Some(n), "value", 0, header, 0)?;
        let mut owners = self.own(header);
        for held in args {
            if held.origin == Origin::Owned {
                self.charge(Some(n), "visit", 0, 0, 0)?;
                owners.extend(held.owners);
            } else {
                let size = held.value.size();
                self.charge(Some(n), "alias", 0, size, 0)?;
                owners.extend(self.own(size));
            }
        }
        self.push(Value::List(Arc::new(List::new(values))), owners);
        Ok(())
    }

    /// The list and word operations, one step each (LP2 drafts; DET.55 names no row for them). `length` reads
    /// retained metadata and reserves its integer. `index` lends or retains one element as a field read does; an
    /// index outside the list stops with `LG4:input.pointwise_failed` before the step. The others build a new value
    /// and are priced from the operands' lengths before it is built: `expand_per_letter` per letter of a built word
    /// or per letter a key scan reads, and [`LIST_PER_ELEMENT`] per element of a built list.
    fn sequence(&mut self, n: NodeId, op: Prim, mut args: Vec<Held>) -> Result<(), Stop> {
        let name = match op {
            Prim::Length => "length",
            Prim::Index => "index",
            Prim::Push => "push",
            Prim::Concat => "concat",
            Prim::Inverse => "inverse",
            Prim::Conjugate => "conjugate",
            Prim::Commutator => "commutator",
            Prim::Compose => "compose",
            Prim::Compress => "compress",
            Prim::Expand => "expand",
            Prim::TraceKey => "trace_key",
            _ => "word_key",
        };
        if op == Prim::Expand {
            return self.expand(n, name, args);
        }
        if op == Prim::Index {
            let index = args.pop().expect("an index");
            let list = args.pop().expect("a list");
            let element = match (&list.value, &index.value) {
                (Value::List(l), Value::Int(i)) => usize::try_from(&**i).ok().and_then(|i| l.items().get(i).cloned()),
                _ => return Err(self.fail(Some(n), name, code::IR_REJECTED)),
            };
            let Some(element) = element else {
                return Err(self.fail(Some(n), name, meter::code::POINTWISE_FAILED));
            };
            self.release(index);
            return self.take_part(n, name, 0, list, element);
        }
        // Only the word operations take menu words as words; `push` appends one to a list as it would any value.
        let builds = matches!(op, Prim::Concat | Prim::Inverse | Prim::Conjugate | Prim::Commutator | Prim::Compose);
        if builds && args.iter().any(|h| matches!(h.value, Value::MenuWord(_))) {
            return self.menu_sequence(n, op, name, args);
        }
        let letters = |h: &Held| match &h.value {
            Value::Word(w) => Some(w.clone()),
            _ => None,
        };
        let words: Option<Vec<Arc<[i32]>>> = args.iter().map(letters).collect();
        let per_letter = self.tariffs.expand_per_letter;
        let (kernel, memory, result): (u128, u128, Box<dyn FnOnce() -> Value>) = match (op, &words, &args[..]) {
            (Prim::Length, _, [h]) => {
                let result = match &h.value {
                    Value::Word(w) => Value::int(w.len()),
                    Value::MenuWord(w) => Value::int(w.letters().len()),
                    Value::List(l) | Value::Set(l) | Value::Map(l) => Value::int(l.items().len()),
                    // A compressed word's exact length is retained metadata: no part is expanded (LG2.16).
                    Value::Compressed(c) => Value::int(c.length().clone()),
                    _ => return Err(self.fail(Some(n), name, code::IR_REJECTED)),
                };
                (0, result.size(), Box::new(move || result))
            }
            (Prim::Compress, Some(ws), [_]) => {
                let w = ws[0].clone();
                let letters = w.len() as u128;
                let result = Value::Compressed(Arc::new(Compressed::leaf(w)));
                (per_letter.saturating_mul(letters), result.size(), Box::new(move || result))
            }
            // A run that keys menu words keys no Full word, so the keys it compares share one header (LG2.23).
            (Prim::WordKey, Some(ws), _) => {
                if self.menu.is_some_and(|m| m.item_matrices().is_some()) {
                    return Err(self.fail(Some(n), name, code::KEY_CONTRACT_MISMATCH));
                }
                let w = ws[0].clone();
                let read = w.len() as u128;
                (
                    per_letter.saturating_mul(read),
                    2 * read,
                    Box::new(move || Value::WordEqualityKey(free_reduction(&w).into())),
                )
            }
            // LB1.10's trace key under the run's pinned relation, priced by what its scan reads. Without a relation, or
            // under a jumbling one, no key of a Full word is built (LG2.23), and a letter outside the relation's
            // alphabet, such as a letter 0, is not of a Full word (LB1.3).
            (Prim::TraceKey, Some(ws), _) => {
                let w = ws[0].clone();
                let Some(relation) = self.relation.filter(|r| !r.is_menu()).cloned() else {
                    return Err(self.fail(Some(n), name, code::KEY_CONTRACT_MISMATCH));
                };
                if !w.iter().all(|&l| relation.holds(l)) {
                    return Err(self.fail(Some(n), name, code::INCOMPATIBLE_ALPHABET));
                }
                let (reads, size) = (relation.reads(&w), 2 * w.len() as u128);
                let build = move || {
                    let key = relation.key(&w).expect("letters of the relation's alphabet");
                    Value::TraceEqualityKey(Arc::new(key))
                };
                (per_letter.saturating_mul(reads), size, Box::new(build))
            }
            // A menu word's trace key needs a pinned `jumble-locality/1` relation that keys the run's menu alphabet
            // (LG2.23: one identity block, grip count and item count); a word of another alphabet is refused.
            (Prim::TraceKey, None, [h]) if matches!(h.value, Value::MenuWord(_)) => {
                let Value::MenuWord(w) = &h.value else { unreachable!("a menu word") };
                let menu = self.menu;
                let Some(relation) = self.relation.filter(|r| menu.is_some_and(|m| r.keys_menu(m))).cloned() else {
                    return Err(self.fail(Some(n), name, code::KEY_CONTRACT_MISMATCH));
                };
                let of_menu = menu.is_some_and(|m| w.identity() == m.identity());
                if !of_menu || !w.letters().iter().all(|&l| relation.holds_menu(l)) {
                    return Err(self.fail(Some(n), name, code::INCOMPATIBLE_ALPHABET));
                }
                let w = w.clone();
                let (reads, size) = (relation.menu_reads(w.letters()), 4 * w.letters().len() as u128);
                let build = move || {
                    let key = relation.menu_key(w.letters()).expect("letters of the relation's alphabet");
                    Value::TraceEqualityKey(Arc::new(key))
                };
                (per_letter.saturating_mul(reads), size, Box::new(build))
            }
            // A menu word's key needs the item matrices of the run's pinned menu (LG2.18); a word of another
            // alphabet is refused. The key is priced by its scan and by the draft price of its exact products.
            (Prim::WordKey, None, [h]) if matches!(h.value, Value::MenuWord(_)) => {
                let Value::MenuWord(w) = &h.value else { unreachable!("a menu word") };
                let Some(menu) = self.menu.filter(|m| m.item_matrices().is_some()).cloned() else {
                    return Err(self.fail(Some(n), name, code::KEY_CONTRACT_MISMATCH));
                };
                if w.identity() != menu.identity() || !w.letters().iter().all(|&l| menu.holds(l)) {
                    return Err(self.fail(Some(n), name, code::INCOMPATIBLE_ALPHABET));
                }
                let w = w.clone();
                let price = menu_key::price(&menu, w.letters(), self.draft.field_per_unit_pair);
                let (work, size) = price.expect("letters of the alphabet");
                let scan = per_letter.saturating_mul(w.letters().len() as u128);
                let build = move || {
                    let key = menu_key::key(&menu, w.letters()).expect("letters of the alphabet");
                    Value::MenuWordKey(Arc::new(key))
                };
                (scan.saturating_add(work), size, Box::new(build))
            }
            (Prim::Inverse | Prim::Conjugate | Prim::Commutator | Prim::Compose | Prim::Concat, Some(ws), _) => {
                let (x, y) = (ws[0].clone(), ws.get(1).cloned().unwrap_or_else(|| Arc::from(Vec::new())));
                let (lx, ly) = (x.len() as u128, y.len() as u128);
                let built = match op {
                    Prim::Inverse => lx,
                    Prim::Conjugate => 2 * lx + ly,
                    Prim::Commutator => 2 * lx + 2 * ly,
                    _ => lx + ly,
                };
                let build = move || {
                    let letters: Vec<i32> = match op {
                        Prim::Inverse => inverse(&x),
                        Prim::Conjugate => [&x[..], &y[..], &inverse(&x)].concat(),
                        Prim::Commutator => [&x[..], &y[..], &inverse(&x), &inverse(&y)].concat(),
                        _ => [&x[..], &y[..]].concat(),
                    };
                    Value::Word(letters.into())
                };
                (per_letter.saturating_mul(built), 2 * built, Box::new(build))
            }
            (Prim::Concat, None, [a, b]) => match (&a.value, &b.value) {
                (Value::List(x), Value::List(y)) => {
                    let (x, y) = (x.clone(), y.clone());
                    let count = (x.items().len() + y.items().len()) as u128;
                    let memory = a.value.size() + b.value.size() - 4;
                    let build = move || Value::List(Arc::new(List::new([x.items(), y.items()].concat())));
                    (LIST_PER_ELEMENT.saturating_mul(count), memory, Box::new(build))
                }
                // Two compressed words join without expansion. Whether they are one value decides whether the parts
                // of the second are kept again, so the step prices that comparison and reserves the larger result.
                (Value::Compressed(x), Value::Compressed(y)) => {
                    let (x, y) = (x.clone(), y.clone());
                    let compare = value::equal_price(&a.value, &b.value, &self.tariffs, &self.draft);
                    let bits = |c: &Compressed| c.length().bits() as u128;
                    let (add, _) = self.tariffs.integer(meter::IntOp::Add, bits(&x), bits(&y));
                    let parts = (x.parts() + y.parts() + 1) as u128;
                    let kernel = compare.saturating_add(add).saturating_add(LIST_PER_ELEMENT.saturating_mul(parts));
                    let memory = Compressed::concat_bound(&x, &y);
                    (kernel, memory, Box::new(move || Value::Compressed(Compressed::concat(&x, &y))))
                }
                _ => return Err(self.fail(Some(n), name, code::IR_REJECTED)),
            },
            (Prim::Push, _, [a, b]) => {
                let Value::List(x) = &a.value else {
                    return Err(self.fail(Some(n), name, code::IR_REJECTED));
                };
                let (x, e) = (x.clone(), b.value.clone());
                let count = x.items().len() as u128 + 1;
                let memory = a.value.size() + e.size() + if e.is_sequence() { 8 } else { 0 };
                let build = move || Value::List(Arc::new(List::new([x.items(), &[e]].concat())));
                (LIST_PER_ELEMENT.saturating_mul(count), memory, Box::new(build))
            }
            _ => return Err(self.fail(Some(n), name, code::IR_REJECTED)),
        };
        self.charge(Some(n), name, kernel, memory, 0)?;
        self.settle(memory, result());
        let result = self.values.pop().expect("the result");
        for held in args {
            self.release(held);
        }
        self.values.push(result);
        Ok(())
    }

    /// A word operation on menu words (LB1.8, LB1.9; LP2 draft), one step priced as on words: `expand_per_letter` per
    /// letter built and 4 bytes reserved per letter. Every operand is a menu word of the pinned alphabet: an operand of
    /// another kind stops the run with `LG4:input.ir_rejected` (the checker refuses the mix first), and a word of
    /// another alphabet, or a run with none, stops it with `LB1:input.incompatible_alphabet`, both before the step. An
    /// inverse reverses the letters, each at its own cap with the item of the inverse rotation in the pinned menu.
    fn menu_sequence(&mut self, n: NodeId, op: Prim, name: &str, args: Vec<Held>) -> Result<(), Stop> {
        let words: Option<Vec<Arc<MenuWord>>> = args
            .iter()
            .map(|h| match &h.value {
                Value::MenuWord(w) => Some(w.clone()),
                _ => None,
            })
            .collect();
        let builds = matches!(op, Prim::Concat | Prim::Inverse | Prim::Conjugate | Prim::Commutator | Prim::Compose);
        let Some(words) = words.filter(|_| builds) else {
            return Err(self.fail(Some(n), name, code::IR_REJECTED));
        };
        let Some(menu) = self.menu.filter(|m| words.iter().all(|w| w.identity() == m.identity())) else {
            return Err(self.fail(Some(n), name, code::INCOMPATIBLE_ALPHABET));
        };
        let (x, y) = (words[0].letters(), words.get(1).map_or(&[][..], |w| w.letters()));
        let (lx, ly) = (x.len() as u128, y.len() as u128);
        let built = match op {
            Prim::Inverse => lx,
            Prim::Conjugate => 2 * lx + ly,
            Prim::Commutator => 2 * lx + 2 * ly,
            _ => lx + ly,
        };
        let memory = 4 * built;
        self.charge(Some(n), name, self.tariffs.expand_per_letter.saturating_mul(built), memory, 0)?;
        // Every menu word of the run is a word of the pinned alphabet, so the inverse and the bound hold; the stop
        // below guards the invariant.
        let inverse = |w: &[[u32; 2]]| menu.inverse_word(w);
        let letters: Option<Vec<[u32; 2]>> = match op {
            Prim::Inverse => inverse(x),
            Prim::Conjugate => inverse(x).map(|xi| [x, y, &xi].concat()),
            Prim::Commutator => inverse(x).zip(inverse(y)).map(|(xi, yi)| [x, y, &xi, &yi].concat()),
            _ => Some([x, y].concat()),
        };
        let Some(word) = letters.and_then(|l| MenuWord::new(menu.shared_identity().clone(), &l)) else {
            return Err(self.fail(Some(n), name, code::INCOMPATIBLE_ALPHABET));
        };
        self.settle(memory, Value::MenuWord(Arc::new(word)));
        let result = self.values.pop().expect("the result");
        for held in args {
            self.release(held);
        }
        self.values.push(result);
        Ok(())
    }

    /// `expand(c, start, count)`, one step (LP2 draft): the letters of a compressed word at exact positions, priced
    /// before any part is read at `expand_per_letter` per letter out and [`LIST_PER_ELEMENT`] per part it may visit,
    /// 2·(height + count) + 1. A range past the word's end stops with `LG4:input.pointwise_failed` before the step; an
    /// admitted count the host cannot address stops with `KN7:resource_limit.host_exhausted`.
    fn expand(&mut self, n: NodeId, name: &str, args: Vec<Held>) -> Result<(), Stop> {
        let (Value::Compressed(c), Value::Int(start), Value::Int(count)) =
            (&args[0].value, &args[1].value, &args[2].value)
        else {
            return Err(self.fail(Some(n), name, code::IR_REJECTED));
        };
        if start.sign() == Sign::Minus || count.sign() == Sign::Minus || &(&**start + &**count) > c.length() {
            return Err(self.fail(Some(n), name, meter::code::POINTWISE_FAILED));
        }
        let (c, start) = (c.clone(), (**start).clone());
        let letters = u128::try_from(&**count).unwrap_or(u128::MAX);
        let visits = c.height().saturating_add(letters).saturating_mul(2).saturating_add(1);
        let per_letter = self.tariffs.expand_per_letter;
        let kernel = per_letter.saturating_mul(letters).saturating_add(LIST_PER_ELEMENT.saturating_mul(visits));
        let memory = letters.saturating_mul(2);
        self.charge(Some(n), name, kernel, memory, 0)?;
        let Ok(count) = usize::try_from(letters) else {
            return Err(self.fail(Some(n), name, code::HOST_EXHAUSTED));
        };
        let word = c.expand(&start, count).expect("a range within the word");
        self.settle(memory, Value::Word(word.into()));
        let result = self.values.pop().expect("the result");
        for held in args {
            self.release(held);
        }
        self.values.push(result);
        Ok(())
    }

    /// `word_stream(c, chunk)` (DET.63, KN2.10, LG4.6; LP2 draft). The schedule comes from the sealed inputs alone:
    /// chunks of `chunk` letters from position 0, the last one short. `word_open` is one step at the `stream_setup`
    /// tariff and reserves the cursor, the length as an exact integer, without debiting any chunk. Each chunk,
    /// located `chunk:<position>`, is one step that preflights its share before it expands a letter: `expand_per_letter`
    /// and `replay_per_letter` per letter, the vectors' partition of the full tariff (FR-B02), which includes the
    /// cursor's reads of the parts, and 2 bytes of scratch per letter. It then expands only its letters, hashes them
    /// into the literal digest and releases the scratch, so coverage advances only on completion and a denied chunk
    /// expands nothing. An interruption is observed before each step, so a cancelled stream stops at a chunk boundary
    /// whose locator names its coverage. A zero-length stream has its open row alone. The completed stream keeps the
    /// cursor's reservation as its owner. A chunk size of 0 is a pointwise failure before any step.
    fn word_stream(&mut self, n: NodeId, args: Vec<Held>) -> Result<(), Stop> {
        let name = "word_stream";
        let (Value::Compressed(c), Value::Int(size)) = (&args[0].value, &args[1].value) else {
            return Err(self.fail(Some(n), name, code::IR_REJECTED));
        };
        if size.sign() != Sign::Plus {
            return Err(self.fail(Some(n), name, meter::code::POINTWISE_FAILED));
        }
        let (c, size) = (Arc::clone(c), (**size).clone());
        let length = c.length().clone();
        let cursor = WordStream::cursor_size(&length);
        let setup = self.tariffs.stream_setup;
        self.charge(Some(n), "word_open", setup, cursor, 0)?;
        let per_letter = self.tariffs.expand_per_letter.saturating_add(self.tariffs.replay_per_letter);
        let mut letters = c.letters();
        let mut digest = c600_digest::Sha256::new();
        let mut scratch = Vec::new();
        let mut at = BigInt::from(0);
        while at < length {
            let share = (&length - &at).min(size.clone());
            let count = u128::try_from(&share).unwrap_or(u128::MAX);
            let memory = count.saturating_mul(2);
            self.charge(Some(n), &format!("chunk:{at}"), per_letter.saturating_mul(count), memory, 0)?;
            let Ok(count) = usize::try_from(count) else {
                return Err(self.fail(Some(n), name, code::HOST_EXHAUSTED));
            };
            scratch.clear();
            letters.next_chunk(count, &mut scratch);
            for letter in &scratch {
                digest.update(&letter.to_le_bytes());
            }
            self.meter.release(memory);
            at += share;
        }
        self.settle(cursor, Value::WordStream(Arc::new(WordStream::new(length, digest.finish()))));
        let result = self.values.pop().expect("the result");
        for held in args {
            self.release(held);
        }
        self.values.push(result);
        Ok(())
    }

    /// `stream_digest(s)` (DET.63, KI.13; LP2 draft): one step that reserves the digest's text of 64 hex digits and
    /// costs no kernel units, a fixed-width reading of the stream's summary.
    fn stream_digest(&mut self, n: NodeId, args: Vec<Held>) -> Result<(), Stop> {
        let Some(Value::WordStream(s)) = args.first().map(|h| &h.value) else {
            return Err(self.fail(Some(n), "stream_digest", code::IR_REJECTED));
        };
        let text = Value::text(&c600_digest::hex(s.digest()));
        let size = text.size();
        self.charge(Some(n), "stream_digest", 0, size, 0)?;
        self.settle(size, text);
        let result = self.values.pop().expect("the result");
        for held in args {
            self.release(held);
        }
        self.values.push(result);
        Ok(())
    }

    /// One part of a container, taken as a field read takes a field: a borrowed container lends it, and an owned one
    /// retains it with a charged owner and is released.
    fn take_part(&mut self, n: NodeId, name: &str, kernel: u128, container: Held, part: Value) -> Result<(), Stop> {
        match container.origin {
            Origin::Owned => {
                let size = part.size();
                self.charge(Some(n), name, kernel, size, 0)?;
                let owners = self.own(size);
                self.release(container);
                self.push(part, owners);
            }
            origin => {
                self.charge(Some(n), name, kernel, 0, 0)?;
                let origin = match origin {
                    Origin::Let { slot, .. } => Origin::Let { slot, whole: false },
                    other => other,
                };
                self.values.push(Held { value: part, owners: Vec::new(), origin });
            }
        }
        Ok(())
    }

    /// The draft price of `comparisons` key comparisons (LP2): each is priced as an exact equality walk over two keys
    /// of at most `widest` units, `compare_per_unit` per unit.
    fn order_price(&self, comparisons: u128, widest: u128) -> u128 {
        comparisons.saturating_mul(self.draft.compare_per_unit.saturating_mul(widest.saturating_mul(2)))
    }

    /// A set or a map from its written items, one step (LP2 draft). For n elements or entries it is priced before it
    /// builds at [`LIST_PER_ELEMENT`] per element and n·⌈log₂ n⌉ comparisons of the widest key, and reserves the size
    /// of every item written; the reservation then shrinks to the built value, which keeps equal keys once.
    fn keyed(&mut self, n: NodeId, map: bool, args: Vec<Held>) -> Result<(), Stop> {
        let name = if map { "map" } else { "set" };
        let values: Vec<Value> = args.iter().map(|h| h.value.clone()).collect();
        let step = if map { 2 } else { 1 };
        let count = (values.len() / step) as u128;
        let widest = values.iter().step_by(step).map(Value::size).max().unwrap_or(0);
        let order = self.order_price(count.saturating_mul(log2_ceil(count)), widest);
        let kernel = LIST_PER_ELEMENT.saturating_mul(count).saturating_add(order);
        // A map's entry is a `{ key, value }` record.
        let memory = if map {
            values.chunks(2).fold(4u128, |s, kv| s.saturating_add(List::size_of(kv)))
        } else {
            List::size_of(&values)
        };
        self.charge(Some(n), name, kernel, memory, 0)?;
        let value = if map {
            let mut items = values.into_iter();
            let mut entries = Vec::new();
            while let (Some(k), Some(v)) = (items.next(), items.next()) {
                entries.push((k, v));
            }
            Value::map(entries)
        } else {
            Value::set(values)
        };
        self.settle(memory, value);
        let result = self.values.pop().expect("the result");
        for held in args {
            self.release(held);
        }
        self.values.push(result);
        Ok(())
    }

    /// The set and map operations, one step each (LP2 drafts; DET.55 names no row for them), priced before they
    /// build. A search costs ⌈log₂(n + 1)⌉ comparisons of the probe for n elements or entries; an insertion adds
    /// [`LIST_PER_ELEMENT`] per element of the built collection, and `elements`, `keys` and `entries` the same per
    /// element of their list, in the named order (LG4.3). `contains` reserves its boolean; `lookup` lends or retains
    /// the value as a field read does, and a missing key stops with `LG4:input.pointwise_failed` before the step.
    fn keyed_op(&mut self, n: NodeId, op: Prim, mut args: Vec<Held>) -> Result<(), Stop> {
        let name = match op {
            Prim::Contains => "contains",
            Prim::Insert | Prim::InsertEntry => "insert",
            Prim::Lookup => "lookup",
            Prim::Elements(_) => "elements",
            Prim::Keys(_) => "keys",
            _ => "entries",
        };
        let (items, is_map) = match &args[0].value {
            Value::Set(c) => (c.clone(), false),
            Value::Map(c) => (c.clone(), true),
            _ => return Err(self.fail(Some(n), name, code::IR_REJECTED)),
        };
        let fits = match op {
            Prim::Contains => true,
            Prim::Insert | Prim::Elements(_) => !is_map,
            _ => is_map,
        };
        if !fits {
            return Err(self.fail(Some(n), name, code::IR_REJECTED));
        }
        let count = items.items().len() as u128;
        let search = |m: &Self, key: &Value| m.order_price(log2_ceil(count + 1), key.size());
        let (kernel, memory, result): (u128, u128, Box<dyn FnOnce() -> Value>) = match op {
            Prim::Elements(order) | Prim::Keys(order) | Prim::Entries(order) => {
                let keys = matches!(op, Prim::Keys(_));
                let memory = if keys {
                    let key = |e: &Value| entry_key(e).size() + if entry_key(e).is_sequence() { 8 } else { 0 };
                    items.items().iter().fold(4u128, |s, e| s.saturating_add(key(e)))
                } else {
                    args[0].value.size()
                };
                let build = move || {
                    let mut parts: Vec<Value> = match keys {
                        true => items.items().iter().map(|e| entry_key(e).clone()).collect(),
                        false => items.items().to_vec(),
                    };
                    if order == Order::Descending {
                        parts.reverse();
                    }
                    Value::list(parts)
                };
                (LIST_PER_ELEMENT.saturating_mul(count), memory, Box::new(build))
            }
            Prim::Contains => {
                let (collection, key) = (args[0].value.clone(), args[1].value.clone());
                let kernel = search(self, &key);
                (kernel, Value::Bool(false).size(), Box::new(move || Value::Bool(find(&collection, &key).is_some())))
            }
            Prim::Lookup => {
                let key = args.pop().expect("a key");
                let Some(i) = find(&args[0].value, &key.value) else {
                    return Err(self.fail(Some(n), name, meter::code::POINTWISE_FAILED));
                };
                let kernel = search(self, &key.value);
                self.release(key);
                let map = args.pop().expect("a map");
                return self.take_part(n, name, kernel, map, entry_value(&items.items()[i]).clone());
            }
            _ => {
                let key = args[1].value.clone();
                let value = args.get(2).map(|v| v.value.clone());
                let kernel = LIST_PER_ELEMENT.saturating_mul(count + 1).saturating_add(search(self, &key));
                // The built collection is at most the old one and the new element or `{ key, value }` entry.
                let added = match &value {
                    Some(v) => List::size_of(&[key.clone(), v.clone()]),
                    None => key.size() + if key.is_sequence() { 8 } else { 0 },
                };
                let build = move || match value {
                    Some(v) => map_insert(&items, key, v),
                    None => set_insert(&items, key),
                };
                (kernel, args[0].value.size().saturating_add(added), Box::new(build))
            }
        };
        self.charge(Some(n), name, kernel, memory, 0)?;
        self.settle(memory, result());
        let result = self.values.pop().expect("the result");
        for held in args {
            self.release(held);
        }
        self.values.push(result);
        Ok(())
    }

    /// A field of a record or a receipt: a borrowed container lends it; an owned one retains it with a charged owner
    /// and is released.
    fn field(&mut self, n: NodeId, held: Held, name: &str, op: &str) -> Result<(), Stop> {
        let field = match &held.value {
            Value::Record(r) | Value::Receipt(r) => r.get(name).cloned(),
            _ => None,
        };
        let Some(field) = field else {
            return Err(self.fail(Some(n), op, code::IR_REJECTED));
        };
        match held.origin {
            Origin::Owned => {
                let size = field.size();
                self.charge(Some(n), op, 0, size, 0)?;
                let owners = self.own(size);
                self.release(held);
                self.push(field, owners);
            }
            origin => {
                self.charge(Some(n), op, 0, 0, 0)?;
                let origin = match origin {
                    Origin::Let { slot, .. } => Origin::Let { slot, whole: false },
                    other => other,
                };
                self.values.push(Held { value: field, owners: Vec::new(), origin });
            }
        }
        Ok(())
    }

    fn read_receipt(&mut self, n: NodeId, held: Held, field: ReceiptField) -> Result<(), Stop> {
        let name = receipt_field(field);
        match &held.value {
            Value::Receipt(r) if r.get(name).is_none() => {
                self.charge(Some(n), "read_receipt", 0, 0, 0)?;
                Err(self.fail(Some(n), "read_receipt", meter::code::NOT_GRANTED))
            }
            Value::Receipt(_) => self.field(n, held, name, "read_receipt"),
            _ => Err(self.fail(Some(n), "read_receipt", code::IR_REJECTED)),
        }
    }

    /// A host row: a refused tariff costs the step only; an admitted one reserves its result bound, and the `sent`
    /// bytes it hands the host count as output, as a preview's request bytes do. The answer's menu words and trace keys
    /// are checked against the run's pinned alphabet and relation at the boundary, after the row.
    fn host_read(&mut self, n: NodeId, op: &str, request: Request, sent: u128) -> Result<(), Stop> {
        let tariff = match self.host.tariff(&request) {
            Ok(t) => t,
            Err(c) => {
                self.charge(Some(n), op, 0, 0, 0)?;
                return Err(self.fail(Some(n), op, c));
            }
        };
        self.charge(Some(n), op, tariff.kernel_cost, tariff.result_bound, sent)?;
        let value = match self.host.perform(&request) {
            Ok(v) => v,
            Err(c) => return Err(self.fail(Some(n), op, c)),
        };
        if value.size() > tariff.result_bound {
            return Err(self.fail(Some(n), op, code::HOST_EXHAUSTED));
        }
        if menu::check_words(&value, self.menu).is_err() {
            return Err(self.fail(Some(n), op, code::INCOMPATIBLE_ALPHABET));
        }
        if trace::check_keys(&value, self.relation).is_err() {
            return Err(self.fail(Some(n), op, code::KEY_CONTRACT_MISMATCH));
        }
        if menu_key::check_keys(&value, self.menu).is_err() {
            return Err(self.fail(Some(n), op, code::KEY_CONTRACT_MISMATCH));
        }
        self.settle(tariff.result_bound, value);
        Ok(())
    }

    /// A snapshot read whose query takes an argument (LP2 draft). DET.59's priced encoding pass gives the argument's
    /// canonical bytes, and a value with none, a function or a receipt among them, stops the run before the host is
    /// asked; the row is then a host row like a literal read, whose output is the bytes handed over.
    fn read_with(&mut self, n: NodeId, query: &str, ty: &QTy, held: Held) -> Result<(), Stop> {
        if is_approx(&held.value) {
            return Err(self.fail(Some(n), "kernel", code::IR_REJECTED));
        }
        let (bytes, buffer) = self.encode(Some(n), &held.value)?;
        let sent = bytes.len() as u128;
        self.host_read(n, "kernel", Request::SnapshotWith { query, arg: &bytes, ty }, sent)?;
        let answer = self.values.pop().expect("the read's answer");
        self.meter.drop_owner(buffer, true);
        self.release(held);
        self.values.push(answer);
        Ok(())
    }

    /// DET.59's output unit as one fused row `op`. The unit's canonical byte length is retained metadata of the value
    /// (a draft: every value comes from metered construction, which retains it), so one admission prices, in DET.56's
    /// order, the encoding work (`encode_per_byte` per byte), the buffer (memory) and the bytes (output). The draft
    /// computes that length on demand, by encoding, which changes no charge; the bytes reach the sink only after the
    /// row is admitted, and the caller releases the buffer after the publication. A value with no canonical form has
    /// no length: its row is charged its step and stops with `PS1:input.noncanonical_value`.
    fn unit(&mut self, n: Option<NodeId>, op: &str, value: &Value) -> Result<(Vec<u8>, OwnerId), Stop> {
        let mut bytes = Vec::new();
        if encode::encode(value, &mut bytes).is_err() {
            self.charge(n, op, 0, 0, 0)?;
            return Err(self.fail(n, op, code::NONCANONICAL_VALUE));
        }
        let len = bytes.len() as u128;
        self.charge(n, op, self.tariffs.encode_per_byte.saturating_mul(len), len, len)?;
        Ok((bytes, self.meter.own(len)))
    }

    /// DET.59's priced encoding pass, before a row that is not an output unit: its tariff and buffer reservation come
    /// from the value's retained bound, and the buffer shrinks to the exact length.
    fn encode(&mut self, n: Option<NodeId>, value: &Value) -> Result<(Vec<u8>, OwnerId), Stop> {
        let bound = value.bound().saturating_add(1);
        self.charge(n, "encode", self.tariffs.encode_per_byte.saturating_mul(bound), bound, 0)?;
        let mut bytes = Vec::new();
        if encode::encode(value, &mut bytes).is_err() {
            return Err(self.fail(n, "encode", code::NONCANONICAL_VALUE));
        }
        let len = bytes.len() as u128;
        self.meter.release(bound.checked_sub(len).expect("an encoding within its bound"));
        Ok((bytes, self.meter.own(len)))
    }

    fn output(&mut self, n: NodeId, sink: Sink, held: Held) -> Result<(), Stop> {
        let op = if matches!(sink, Sink::View) { "publish_view" } else { "emit" };
        let (bytes, buffer) = self.unit(Some(n), op, &held.value)?;
        if let Err(c) = self.host.sink(&sink, &held.value, &bytes) {
            return Err(self.fail(Some(n), op, c));
        }
        self.meter.drop_owner(buffer, true);
        self.release(held);
        self.push(Value::Unit, Vec::new());
        Ok(())
    }

    fn propose(&mut self, n: NodeId, mut args: Vec<Held>) -> Result<(), Stop> {
        let target = args.pop().expect("a target");
        let word = args.pop().expect("a word");
        let (Value::Word(_) | Value::MenuWord(_), Value::Target(t)) = (&word.value, &target.value) else {
            return Err(self.fail(Some(n), "invoke", code::IR_REJECTED));
        };
        let request = Value::record(vec![("target", target.value.clone()), ("word", word.value.clone())]);
        let (bytes, buffer) = self.encode(Some(n), &request)?;
        let request = match &word.value {
            Value::MenuWord(w) => {
                Request::ProposeMenu { letters: w.letters(), menu: w.identity(), target: t.as_str(), request: &bytes }
            }
            Value::Word(letters) => Request::Propose { word: letters, target: t.as_str(), request: &bytes },
            _ => return Err(self.fail(Some(n), "invoke", code::IR_REJECTED)),
        };
        let tariff = match self.host.tariff(&request) {
            Ok(t) => t,
            Err(c) => {
                self.charge(Some(n), "invoke", 0, 0, 0)?;
                return Err(self.fail(Some(n), "invoke", c));
            }
        };
        self.charge(Some(n), "invoke", tariff.kernel_cost, tariff.result_bound, bytes.len() as u128)?;
        let receipt = match self.host.perform(&request) {
            Ok(r @ Value::Receipt(_)) if r.size() <= tariff.result_bound => r,
            Ok(_) => return Err(self.fail(Some(n), "invoke", code::HOST_EXHAUSTED)),
            Err(c) => return Err(self.fail(Some(n), "invoke", c)),
        };
        self.settle(tariff.result_bound, receipt);
        let receipt = self.values.pop().expect("the receipt");
        self.meter.drop_owner(buffer, true);
        self.release(word);
        self.release(target);
        self.values.push(receipt);
        Ok(())
    }

    fn accounting(&mut self, n: NodeId, a: Accounting) -> Result<(), Stop> {
        if a == Accounting::Status {
            self.charge(Some(n), "accounting", 0, 4, 0)?;
            self.settle(4, Value::tag("running"));
            return Ok(());
        }
        let limit = self.meter.innermost().limits.steps;
        let reserve = integer_size(bits(limit));
        self.charge(Some(n), "accounting", 0, reserve, 0)?;
        let used = self.meter.innermost().used.steps;
        let v = if a == Accounting::UsedSteps { used } else { limit - used };
        self.settle(reserve, Value::int(v));
        Ok(())
    }

    fn draw(&mut self, n: NodeId) -> Result<(), Stop> {
        if self.generator.is_none() {
            self.charge(Some(n), "rng_draw", 0, 0, 0)?;
            return Err(self.fail(Some(n), "rng_draw", meter::code::NOT_GRANTED));
        }
        let reserve = integer_size(256);
        self.charge(Some(n), "rng_draw", self.tariffs.rng_block, reserve, 0)?;
        let Some(block) = self.generator.as_mut().expect("a generator").draw() else {
            return Err(self.fail(Some(n), "rng_draw", meter::code::GENERATOR_EXHAUSTED));
        };
        self.settle(reserve, Value::int(BigInt::from_bytes_be(Sign::Plus, &block)));
        Ok(())
    }

    /// `rng_init` (DET.61; LP2 draft): a generator on the run's seed, its stream path being the run's path, then the
    /// innermost open child's path, then the node's segments. It is priced as the run's own generator is; without a
    /// seed it is not granted, and a seed or path the stream key refuses stops the run after the charge.
    fn rng_init(&mut self, n: NodeId, segments: &[String]) -> Result<(), Stop> {
        let Some((seed, base)) = self.seed else {
            self.charge(Some(n), "rng_init", 0, 0, 0)?;
            return Err(self.fail(Some(n), "rng_init", meter::code::NOT_GRANTED));
        };
        let child = self.open.last().map(|o| o.path.clone()).unwrap_or_default();
        let path: Vec<&str> = base.iter().copied().chain(child.iter().chain(segments).map(String::as_str)).collect();
        let len = rng::key_input_len(seed.len(), &path);
        self.charge(Some(n), "rng_init", self.tariffs.rng_init_per_byte.saturating_mul(len), GENERATOR_SIZE, 0)?;
        let Some((key, _)) = rng::stream_key(seed, &path) else {
            return Err(self.fail(Some(n), "rng_init", meter::code::GENERATOR_INPUT));
        };
        self.settle(GENERATOR_SIZE, Value::Generator(Arc::new(Generator::new(Cursor::new(key)))));
        Ok(())
    }

    /// Enters the run, runs the entry function and emits its result as the top-level output unit.
    fn start(
        &mut self,
        function: usize,
        args: Vec<(Value, usize)>,
        seed: Option<(&[u8], &[&str])>,
    ) -> Result<(Value, Vec<u8>), Stop> {
        if let Some((seed, path)) = seed {
            let len = rng::key_input_len(seed.len(), path);
            self.charge(None, "rng_init", self.tariffs.rng_init_per_byte.saturating_mul(len), GENERATOR_SIZE, 0)?;
            let Some((key, _)) = rng::stream_key(seed, path) else {
                return Err(self.fail(None, "rng_init", meter::code::GENERATOR_INPUT));
            };
            self.meter.own(GENERATOR_SIZE);
            self.generator = Some(Cursor::new(key));
        }
        let mut held = Vec::new();
        for (value, len) in args {
            let size = value.size();
            self.charge(None, "read", self.tariffs.decode_per_byte.saturating_mul(len as u128), size, 0)?;
            let owners = self.own(size);
            held.push(Held { value, owners, origin: Origin::Owned });
        }
        self.call(None, self.loaded.functions[function].code, held, None)?;
        self.execute()?;
        let result = self.values.pop().expect("the entry's result");
        let (bytes, buffer) = self.unit(None, "emit", &result.value)?;
        self.meter.drop_owner(buffer, true);
        let value = result.value.clone();
        self.release(result);
        Ok((value, bytes))
    }
}

/// Whether a scrutinee equals a match pattern as an exact equality does. A menu-word pattern is a literal, which names
/// no alphabet of its own; every menu word of a run is a word of its pinned alphabet, so the letters decide (LP2
/// draft). An end test's verdict matches the status tag of its outcome, and no other pattern (LP2 draft).
fn pattern_matches(v: &Value, pattern: &Value) -> bool {
    match (v, pattern) {
        (Value::MenuWord(x), Value::MenuWord(y)) => x.letters() == y.letters(),
        (Value::EndTest(_, outcome), Value::Tag(tag)) => outcome.tag() == tag.as_str(),
        (Value::EndTest(..), _) => false,
        _ => value::equal(v, pattern),
    }
}

impl Loaded {
    /// Runs `module.function` with the given arguments. The arguments are captured input: each must have a canonical
    /// form, which prices its `read` (DET.57).
    pub fn run(
        &self,
        module: &str,
        function: &str,
        args: Vec<Value>,
        host: &mut dyn Host,
        options: &Options,
    ) -> Result<Run, Refusal> {
        let f =
            self.function(module, function).ok_or_else(|| rejected(format!("unknown entry {module}.{function}")))?;
        if self.functions[f].params != args.len() {
            return Err(rejected(format!("{module}.{function} takes {} arguments", self.functions[f].params)));
        }
        let mut input = Vec::new();
        for value in args {
            let mut bytes = Vec::new();
            if encode::encode(&value, &mut bytes).is_err() {
                return Err(refuse(code::NONCANONICAL_VALUE, "an argument has no canonical form"));
            }
            if let Err(why) = menu::check_words(&value, options.menu) {
                return Err(refuse(code::INCOMPATIBLE_ALPHABET, format!("an argument: {why}")));
            }
            input.push((value, bytes.len()));
        }
        let meter = if options.record { Meter::new(options.limits) } else { Meter::quiet(options.limits) };
        let mut machine = Machine {
            loaded: self,
            tariffs: options.tariffs,
            draft: options.draft,
            cancel: options.cancel,
            sources: options.sources,
            menu: options.menu,
            relation: options.relation,
            host,
            meter,
            frames: Vec::new(),
            values: Vec::new(),
            tasks: Vec::new(),
            generator: None,
            seed: options.seed,
            open: Vec::new(),
            children: 0,
        };
        let outcome = machine.start(f, input, options.seed);
        let Machine { meter, frames, values, .. } = machine;
        drop((frames, values));
        Ok(match outcome {
            Ok((value, bytes)) => Run { summary: meter.finish(None), value: Some(value), output: Some(bytes) },
            Err(stop) => Run { summary: meter.finish(Some(stop)), value: None, output: None },
        })
    }
}
