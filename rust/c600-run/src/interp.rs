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
//! | set, map (LP2 drafts) | one step `set` or `map`: [`LIST_PER_ELEMENT`] per element and n·⌈log₂ n⌉ comparisons of the widest key, each `compare_per_unit` per unit of two such keys; it reserves every item written and settles to the built value |
//! | `contains`, `insert`, `lookup`, `elements`, `keys`, `entries` (LP2 drafts) | one step each: ⌈log₂(n + 1)⌉ comparisons of the probe for a search, [`LIST_PER_ELEMENT`] per element built; `lookup` lends or retains its value as a field read does, and a missing key stops with `LG4:input.pointwise_failed` before its step |
//! | exact arithmetic | `exact_add`, `exact_less`, `exact_eq`, `not`, `to_approx`, `approx_add`, `approx_less`, `approx_and`, `approx_not` |
//! | snapshot read, declassifier | `kernel` with the host row's tariff and result bound |
//! | file read, clock | `read` with the host row's tariff and result bound |
//! | view, workspace and file output | `encode` (DET.59's priced encoding pass), then `publish_view` or `emit` with the output bytes |
//! | `propose` | `encode` of the request, then `invoke` with the host row's tariff and the request bytes as output |
//! | receipt field | `read_receipt`; a field the host withheld stops with `LG3:capability.not_granted` (BND.42) |
//! | exact sink | `use_exact` |
//! | display accounting | `accounting`, read after its own step |
//! | random draw | `rng_draw` (DET.61) |
//!
//! A run is entered with `rng_init` when a seed is given, one `read` per argument, and the entry `call`. Its result
//! is the top-level output unit: `encode` and `emit` (DET.59).
//!
//! # Drafts (M2)
//!
//! The operation names, the stop locator text, the frame layout (parameters, captures, `let` slots and temporaries),
//! the per-capture `alias`, the receipt request record, [`DraftTariffs`], the `Status` accounting value `"running"`
//! and the JSON form of a word are drafts that change freely until the IR freeze. The IR has no child construct yet:
//! DET.60's children are exercised through the meter's traces only.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering as Atomic};
use std::sync::Arc;

use c600_ir::{self as ir, Accounting, Callee, ExactRole, Expr, FnKind, Order, Prim, QTy, ReceiptField, Ty};
use num_bigint::{BigInt, Sign};

use crate::approx;
use crate::encode;
use crate::meter::{
    self, frame_size, integer_size, Amounts, DraftTariffs, Interruption, Meter, OwnerId, SourceRange, Stop, Summary,
    Tariffs, GENERATOR_SIZE,
};
use crate::rng::{self, Cursor};
use crate::value::{
    self, entry_key, entry_value, find, free_reduction, inverse, map_insert, set_insert, Closure, List, Record, Text,
    Value,
};

/// The draft price of a built list (LP2): kernel units per element it holds. DET.55 names no row for it.
///
/// The list and word operations' names and prices, the word operations' reuse of `expand_per_letter`, the pointwise
/// stop of an index outside its list and a list's JSON form are LP2 drafts that change freely until the IR freeze
/// (PHS.49). So are the set and map operations, their prices (a comparison priced as an exact equality walk over two
/// keys of the bounding size), the canonical key order ([`value::key_order`]), the pointwise stop of a missing key
/// and their JSON forms.
pub const LIST_PER_ELEMENT: u128 = 1;

/// The codes the interpreter raises in addition to the meter's (chapter 13).
pub mod code {
    pub const IR_VERSION_UNSUPPORTED: &str = "LG4:input.ir_version_unsupported";
    pub const IR_REJECTED: &str = "LG4:input.ir_rejected";
    pub const NONCANONICAL_VALUE: &str = "PS1:input.noncanonical_value";
    pub const HOST_EXHAUSTED: &str = "KN7:resource_limit.host_exhausted";
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
}

impl<'a> Options<'a> {
    /// Test tariffs, rows kept, no seed, no interruption and no source ranges.
    pub fn new(limits: Amounts) -> Options<'a> {
        Options {
            limits,
            tariffs: Tariffs::TEST,
            draft: DraftTariffs::TEST,
            seed: None,
            record: true,
            cancel: None,
            sources: None,
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
    Closure { code: usize, captures: Vec<SlotRef> },
    Prim(Prim),
    Record(Vec<Arc<Text>>),
    Field(Box<str>),
    List,
    Set,
    Map,
    ReadSnapshot { query: String, ty: QTy },
    PublishView,
    WriteWorkspace { record: String, ty: QTy },
    Propose,
    ReadReceipt(ReceiptField),
    UseExact(ExactRole),
    Accounting(Accounting),
    Declassified { id: String, ty: Ty },
    FileRead { path: String, ty: Ty },
    FileWrite { path: String },
    RandomDraw,
    ReadClock,
}

#[derive(Debug)]
struct Node {
    kind: Kind,
    /// The operands, in evaluation order. A closure's body is not one: it runs in its own frame.
    kids: Vec<NodeId>,
    /// The parent and this node's index among the IR children, which give the node's path.
    parent: Option<(NodeId, u32)>,
    /// The named function whose body contains the node.
    function: usize,
}

/// A code unit: a named function's body or a closure's.
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
    closure: Option<NodeId>,
}

enum Step<'e> {
    Visit { expr: &'e Expr, parent: Option<(NodeId, u32)>, attach: bool },
    Bind { name: &'e str, slot: usize },
    Unbind,
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
    for (mi, m) in program.modules.iter().enumerate() {
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
    let exported: Vec<bool> = program.modules.iter().flat_map(|m| m.functions.iter().map(|f| f.exported)).collect();
    let mut compiler = Compiler {
        loaded: Loaded { modules, functions, codes: Vec::new(), nodes: Vec::new() },
        names,
        contexts: Vec::new(),
        next_id: 0,
    };
    let mut fi = 0;
    for (mi, m) in program.modules.iter().enumerate() {
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
        | Prim::ExactLess
        | Prim::ExactEq
        | Prim::And
        | Prim::Or
        | Prim::ApproxAdd
        | Prim::ApproxLess
        | Prim::ApproxAnd => Some(2),
        Prim::Not | Prim::ApproxNot | Prim::ToApprox(_) => Some(1),
        Prim::ToExact => None,
        Prim::Length | Prim::Inverse | Prim::WordKey => Some(1),
        Prim::Index | Prim::Push | Prim::Concat | Prim::Conjugate | Prim::Commutator | Prim::Compose => Some(2),
        Prim::Elements(_) | Prim::Keys(_) | Prim::Entries(_) => Some(1),
        Prim::Contains | Prim::Insert | Prim::Lookup => Some(2),
        Prim::InsertEntry => Some(3),
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
                    closure: Some(id),
                });
                steps.push(Step::Leave);
                steps.push(Step::Visit { expr: body, parent: Some((id, 0)), attach: false });
                Kind::Closure { code, captures: Vec::new() }
            }
            Expr::Prim { op, args } => {
                match (arity(*op), op) {
                    (None, _) => return Err(rejected("a conversion to exact is never run (LG2.19)")),
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
        };
        self.loaded.nodes[id].kind = kind;
        Ok(())
    }

    /// Closes a closure body: its captures take declaration order, and the closure node reads each from the
    /// enclosing frame.
    fn leave(&mut self) {
        let ctx = self.contexts.pop().expect("a closure context");
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
                _ => unreachable!("only variables and closures name captures"),
            }
        }
        let outer: Vec<SlotRef> = order.iter().map(|&i| ctx.captures[i].outer).collect();
        let code = &mut self.loaded.codes[ctx.code];
        code.captures = outer.len();
        code.lets = ctx.lets;
        let node = ctx.closure.expect("a closure context");
        if outer.iter().any(|s| matches!(s, SlotRef::Capture(_))) {
            self.contexts.last_mut().expect("an enclosing context").uses.push(node);
        }
        if let Kind::Closure { captures, .. } = &mut self.loaded.nodes[node].kind {
            *captures = outer;
        }
    }

    /// Sizes every frame (DET.57): parameters, captures, `let` slots and the temporaries its body needs at once.
    fn frames(&mut self) {
        let nodes = &self.loaded.nodes;
        let mut temps = vec![0u128; nodes.len()];
        for id in (0..nodes.len()).rev() {
            let kids = &nodes[id].kids;
            let t = |k: NodeId| temps[k];
            temps[id] = match &nodes[id].kind {
                Kind::Closure { .. } => 1,
                _ if kids.is_empty() => 1,
                Kind::Let(_) => t(kids[0]).max(t(kids[1])),
                Kind::If | Kind::DisplayIf | Kind::Match(_) | Kind::Prim(Prim::And | Prim::Or) => {
                    kids.iter().map(|&k| t(k)).max().unwrap_or(1)
                }
                _ => kids
                    .iter()
                    .enumerate()
                    .map(|(i, &k)| i as u128 + t(k))
                    .max()
                    .unwrap_or(0)
                    .max(kids.len() as u128 + 1),
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
}

#[derive(Clone, Copy, Debug)]
enum Task {
    Eval(NodeId),
    Apply(NodeId),
    Branch(NodeId),
    Bind(NodeId),
    ScopeExit(NodeId),
    Return,
}

struct Machine<'a> {
    loaded: &'a Loaded,
    tariffs: Tariffs,
    draft: DraftTariffs,
    cancel: Option<&'a dyn Cancel>,
    sources: Option<&'a dyn SourceRanges>,
    host: &'a mut dyn Host,
    meter: Meter,
    frames: Vec<Frame>,
    values: Vec<Held>,
    tasks: Vec<Task>,
    generator: Option<Cursor>,
}

/// The stop locator of an operation: the call path from the entry (each caller's call site), then the operation's
/// node path and name; `entry#…` for the operations around the entry call. The source range is the node's.
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
    for site in frames.iter().filter_map(|f| f.site) {
        at.push_str(&format!("{}:{} > ", loaded.functions[loaded.nodes[site].function].label, loaded.path(site)));
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
            match task {
                Task::Eval(n) => self.eval(n)?,
                Task::Apply(n) => self.apply(n)?,
                Task::Branch(n) => self.branch(n)?,
                Task::Bind(n) => self.bind(n)?,
                Task::ScopeExit(n) => self.scope_exit(n)?,
                Task::Return => self.ret()?,
            }
        }
        Ok(())
    }

    fn eval(&mut self, n: NodeId) -> Result<(), Stop> {
        let loaded = self.loaded;
        let node = &loaded.nodes[n];
        match &node.kind {
            Kind::Lit(v) => {
                let size = v.size();
                self.charge(Some(n), "value", 0, size, 0)?;
                let owners = self.own(size);
                self.push(v.clone(), owners);
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
            Kind::ReadSnapshot { query, ty } => self.host_read(n, "kernel", Request::Snapshot { query, ty })?,
            Kind::Declassified { id, ty } => self.host_read(n, "kernel", Request::Declassified { id, ty })?,
            Kind::FileRead { path, ty } => self.host_read(n, "read", Request::FileRead { path, ty })?,
            Kind::ReadClock => self.host_read(n, "read", Request::Clock)?,
            Kind::Accounting(a) => self.accounting(n, *a)?,
            Kind::RandomDraw => self.draw(n)?,
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
        let owners = if held.origin == Origin::Owned {
            self.charge(Some(n), "visit", 0, 0, 0)?;
            held.owners
        } else {
            let size = held.value.size();
            self.charge(Some(n), "alias", 0, size, 0)?;
            self.own(size)
        };
        self.frames.last_mut().expect("a frame").lets[slot] = Some((held.value, owners));
        self.tasks.push(Task::ScopeExit(n));
        self.tasks.push(Task::Eval(self.loaded.nodes[n].kids[1]));
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
                let kernel = arms
                    .iter()
                    .fold(0u128, |s, a| s.saturating_add(value::equal_price(v, a, &self.tariffs, &self.draft)));
                self.charge(Some(n), "branch", kernel, 0, 0)?;
                let arm = arms.iter().position(|a| value::equal(v, a)).unwrap_or(arms.len());
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
            Kind::UseExact(role) => {
                let held = args.pop().expect("a value");
                if is_approx(&held.value) {
                    return Err(self.fail(Some(n), "use_exact", code::IR_REJECTED));
                }
                self.charge(Some(n), "use_exact", 0, 0, 0)?;
                if let Err(c) = self.host.sink(&Sink::Exact { role: *role }, &held.value, &[]) {
                    return Err(self.fail(Some(n), "use_exact", c));
                }
                self.release(held);
                self.push(Value::Unit, Vec::new());
                Ok(())
            }
            _ => unreachable!("an operation with operands"),
        }
    }

    /// The `call` operation and the callee's frame. Arguments are borrowed; owned temporaries wait for the return.
    fn call(&mut self, site: Option<NodeId>, code: usize, args: Vec<Held>, callee: Option<Held>) -> Result<(), Stop> {
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
        let mut captures = Vec::new();
        if let Some(h) = callee {
            if let Value::Closure(closure) = &h.value {
                captures = closure.captures().to_vec();
            }
            temps.extend(h.owners);
        }
        self.frames.push(Frame { code, site, params, captures, lets: vec![None; c.lets], owner, temps });
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
            | Prim::WordKey => return self.sequence(n, op, args),
            Prim::Contains
            | Prim::Insert
            | Prim::InsertEntry
            | Prim::Lookup
            | Prim::Elements(_)
            | Prim::Keys(_)
            | Prim::Entries(_) => return self.keyed_op(n, op, args),
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
            (Prim::ExactLess, a, Some(b)) => value::compare_price(a, b, &d).map(|k| (k, 1)),
            (Prim::ExactEq, a, Some(b)) if !is_approx(a) && !is_approx(b) => {
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
            _ => "word_key",
        };
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
        let letters = |h: &Held| match &h.value {
            Value::Word(w) => Some(w.clone()),
            _ => None,
        };
        let words: Option<Vec<Arc<[i32]>>> = args.iter().map(letters).collect();
        let per_letter = self.tariffs.expand_per_letter;
        let (kernel, memory, result): (u128, u128, Box<dyn FnOnce() -> Value>) = match (op, &words, &args[..]) {
            (Prim::Length, _, [h]) => {
                let count = match &h.value {
                    Value::Word(w) => w.len(),
                    Value::List(l) | Value::Set(l) | Value::Map(l) => l.items().len(),
                    _ => return Err(self.fail(Some(n), name, code::IR_REJECTED)),
                };
                let result = Value::int(count);
                (0, result.size(), Box::new(move || result))
            }
            (Prim::WordKey, Some(ws), _) => {
                let w = ws[0].clone();
                let read = w.len() as u128;
                (
                    per_letter.saturating_mul(read),
                    2 * read,
                    Box::new(move || Value::WordEqualityKey(free_reduction(&w).into())),
                )
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
            (Prim::Concat, None, [a, b]) => {
                let (Value::List(x), Value::List(y)) = (&a.value, &b.value) else {
                    return Err(self.fail(Some(n), name, code::IR_REJECTED));
                };
                let (x, y) = (x.clone(), y.clone());
                let count = (x.items().len() + y.items().len()) as u128;
                let memory = a.value.size() + b.value.size() - 4;
                let build = move || Value::List(Arc::new(List::new([x.items(), y.items()].concat())));
                (LIST_PER_ELEMENT.saturating_mul(count), memory, Box::new(build))
            }
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

    /// A host row: a refused tariff costs the step only; an admitted one reserves its result bound.
    fn host_read(&mut self, n: NodeId, op: &str, request: Request) -> Result<(), Stop> {
        let tariff = match self.host.tariff(&request) {
            Ok(t) => t,
            Err(c) => {
                self.charge(Some(n), op, 0, 0, 0)?;
                return Err(self.fail(Some(n), op, c));
            }
        };
        self.charge(Some(n), op, tariff.kernel_cost, tariff.result_bound, 0)?;
        let value = match self.host.perform(&request) {
            Ok(v) => v,
            Err(c) => return Err(self.fail(Some(n), op, c)),
        };
        if value.size() > tariff.result_bound {
            return Err(self.fail(Some(n), op, code::HOST_EXHAUSTED));
        }
        self.settle(tariff.result_bound, value);
        Ok(())
    }

    /// DET.59's priced encoding pass: its tariff and buffer reservation come from the value's retained bound, and the
    /// buffer shrinks to the exact length.
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
        let (bytes, buffer) = self.encode(Some(n), &held.value)?;
        let op = if matches!(sink, Sink::View) { "publish_view" } else { "emit" };
        self.charge(Some(n), op, 0, 0, bytes.len() as u128)?;
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
        let (Value::Word(letters), Value::Target(t)) = (&word.value, &target.value) else {
            return Err(self.fail(Some(n), "invoke", code::IR_REJECTED));
        };
        let request = Value::record(vec![("target", target.value.clone()), ("word", word.value.clone())]);
        let (bytes, buffer) = self.encode(Some(n), &request)?;
        let request = Request::Propose { word: letters, target: t.as_str(), request: &bytes };
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
        let (bytes, buffer) = self.encode(None, &result.value)?;
        self.charge(None, "emit", 0, 0, bytes.len() as u128)?;
        self.meter.drop_owner(buffer, true);
        let value = result.value.clone();
        self.release(result);
        Ok((value, bytes))
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
            input.push((value, bytes.len()));
        }
        let meter = if options.record { Meter::new(options.limits) } else { Meter::quiet(options.limits) };
        let mut machine = Machine {
            loaded: self,
            tariffs: options.tariffs,
            draft: options.draft,
            cancel: options.cancel,
            sources: options.sources,
            host,
            meter,
            frames: Vec::new(),
            values: Vec::new(),
            tasks: Vec::new(),
            generator: None,
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
