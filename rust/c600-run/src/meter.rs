//! The `lp1-exec-draft-1` reference meter (chapter 09, DET.55 to DET.60).
//!
//! The meter mirrors the frozen reference model `docs/architecture/1.0/language/contract-vectors/check_vectors.py`,
//! whose 40 traces `tests/traces.rs` replays. It covers:
//! - the four budget dimensions;
//! - one charge row per operation;
//! - owners and unowned reservations;
//! - child scopes with effective caps;
//! - cleanup at the end of a run.
//!
//! Amounts are `u128`, and every cost the interpreter computes saturates at `u128::MAX`. A saturated request can only be
//! refused, because a finite limit plus a current use never reaches `u128::MAX`.
//!
//! Interruption is checked before the step check (DET.56). A broken bookkeeping invariant is a bug in the interpreter,
//! not a program error, so it panics. Examples are an owner larger than its reservation or a release below zero.

/// The four budget dimensions in admission order (DET.56).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dim {
    Steps,
    KernelCost,
    Memory,
    Output,
}

impl Dim {
    pub const ALL: [Dim; 4] = [Dim::Steps, Dim::KernelCost, Dim::Memory, Dim::Output];

    pub fn name(self) -> &'static str {
        match self {
            Dim::Steps => "steps",
            Dim::KernelCost => "kernel_cost",
            Dim::Memory => "memory",
            Dim::Output => "output",
        }
    }

    /// The stop code of this dimension's exhaustion (chapter 09 section 8).
    pub fn code(self) -> &'static str {
        match self {
            Dim::Steps => code::STEPS,
            Dim::KernelCost => code::KERNEL_COST,
            Dim::Memory => code::MEMORY,
            Dim::Output => code::OUTPUT,
        }
    }
}

/// The stable stop codes the meter and the interpreter raise (chapter 09 section 8, chapter 13).
pub mod code {
    pub const STEPS: &str = "KN7:budget.steps";
    pub const KERNEL_COST: &str = "KN7:budget.kernel_cost";
    pub const MEMORY: &str = "KN7:budget.memory";
    pub const OUTPUT: &str = "KN7:budget.output";
    pub const CANCELLED_BY_USER: &str = "KN7:cancelled.by_user";
    pub const CANCELLED_REVISION_REPLACED: &str = "KN7:cancelled.revision_replaced";
    pub const POINTWISE_FAILED: &str = "LG4:input.pointwise_failed";
    pub const GENERATOR_INPUT: &str = "LG4:input.generator_input";
    pub const GENERATOR_EXHAUSTED: &str = "LG4:resource_limit.generator_exhausted";
    pub const TEXT_SCALAR_INVALID: &str = "LG4:input.text_scalar_invalid";
    pub const NOT_GRANTED: &str = "LG3:capability.not_granted";
}

/// One amount per dimension. Used for limits, use and requests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Amounts {
    pub steps: u128,
    pub kernel_cost: u128,
    pub memory: u128,
    pub output: u128,
}

impl Amounts {
    pub fn get(&self, dim: Dim) -> u128 {
        match dim {
            Dim::Steps => self.steps,
            Dim::KernelCost => self.kernel_cost,
            Dim::Memory => self.memory,
            Dim::Output => self.output,
        }
    }

    fn slot(&mut self, dim: Dim) -> &mut u128 {
        match dim {
            Dim::Steps => &mut self.steps,
            Dim::KernelCost => &mut self.kernel_cost,
            Dim::Memory => &mut self.memory,
            Dim::Output => &mut self.output,
        }
    }
}

/// A source range against the executed bytes (DET.56; HANDOFF H7's `SourceRange`): a source digest and a byte
/// interval.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRange {
    pub digest: String,
    pub start: usize,
    pub end: usize,
}

/// How a run or a child ended (KN7.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Completed,
    CutOff,
    Error,
    Cancelled,
}

impl Outcome {
    pub fn name(self) -> &'static str {
        match self {
            Outcome::Completed => "completed",
            Outcome::CutOff => "cut_off",
            Outcome::Error => "error",
            Outcome::Cancelled => "cancelled",
        }
    }
}

/// Where a run stopped: the operation path, the failed phase (a dimension, `effect` or `interruption`), the exhausted
/// or failing scope and the source range of the operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    pub at: String,
    pub phase: &'static str,
    pub scope: String,
    pub source: Option<SourceRange>,
}

/// A stop: a budget cut-off, an effect error or an interruption.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stop {
    pub code: &'static str,
    pub outcome: Outcome,
    pub position: Position,
    /// The scope the stop ends. A child scope absorbs its own stops; any other stop goes on to the enclosing scope.
    scope: usize,
    /// The root's live charge when the stop was raised, before any cleanup (DET.58).
    live: Option<u128>,
}

/// Why a run was interrupted (KN7.6). The interpreter observes it before each operation (DET.56).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interruption {
    ByUser,
    RevisionReplaced,
}

/// One meter: the root or a child scope (DET.60).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    pub name: String,
    pub limits: Amounts,
    /// Cumulative steps, kernel cost and output, and the peak live memory.
    pub used: Amounts,
    /// Current live memory: owners plus unowned reservations.
    pub live: u128,
    /// Admitted memory not yet turned into an owner or released.
    pub reserved: u128,
}

impl Scope {
    fn new(name: &str, limits: Amounts) -> Scope {
        Scope { name: name.to_string(), limits, used: Amounts::default(), live: 0, reserved: 0 }
    }

    fn current(&self, dim: Dim) -> u128 {
        if dim == Dim::Memory {
            self.live
        } else {
            self.used.get(dim)
        }
    }
}

/// One charge row: the operation path, its admitted step and requests, and what it released (the vectors' columns).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub at: String,
    pub steps: u128,
    pub kernel_cost: u128,
    pub memory: u128,
    pub output: u128,
    pub released: u128,
}

/// A live owner: a charged payload and the meters that count it. A dropped owner's slot is reused under a new
/// generation, so an old identifier never names a later owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OwnerId {
    index: usize,
    generation: u64,
}

#[derive(Clone, Debug)]
struct Owner {
    size: u128,
    meters: Vec<usize>,
}

#[derive(Clone, Debug)]
struct Slot {
    generation: u64,
    owner: Option<Owner>,
}

/// An open child scope, returned by [`Meter::enter_child`] and closed by [`Meter::leave_child`].
#[derive(Debug)]
pub struct Child {
    scope: usize,
    at: String,
    requested: Amounts,
    effective: Amounts,
}

/// A child's ledger entry (DET.60).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChildRecord {
    pub at: String,
    pub requested: Amounts,
    pub effective: Amounts,
    pub outcome: Outcome,
    pub code: Option<&'static str>,
    pub stop: Option<Position>,
    pub used: Amounts,
    pub returned_memory: u128,
    pub live_after_return: u128,
}

/// The terminal record of a run (DET.58, DET.59).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    pub outcome: Outcome,
    pub code: Option<&'static str>,
    pub stop: Option<Position>,
    pub used: Amounts,
    pub live_before_cleanup: u128,
    pub live_after_cleanup: u128,
    pub rows: Vec<Row>,
    pub children: Vec<ChildRecord>,
}

/// The meter of one run.
#[derive(Clone, Debug)]
pub struct Meter {
    scopes: Vec<Scope>,
    active: Vec<usize>,
    owners: Vec<Slot>,
    free: Vec<usize>,
    /// Whether charge rows are kept. A quiet meter keeps none; its counters and stops are the same.
    record: bool,
    rows: Vec<Row>,
    children: Vec<ChildRecord>,
}

const ROOT: usize = 0;

impl Meter {
    /// A meter that keeps one row per operation, as the contract vectors list them.
    pub fn new(limits: Amounts) -> Meter {
        Meter {
            scopes: vec![Scope::new("root", limits)],
            active: vec![ROOT],
            owners: Vec::new(),
            free: Vec::new(),
            record: true,
            rows: Vec::new(),
            children: Vec::new(),
        }
    }

    /// A meter that keeps no rows. A long run's bookkeeping then stays proportional to its live owners.
    pub fn quiet(limits: Amounts) -> Meter {
        Meter { record: false, ..Meter::new(limits) }
    }

    pub fn root(&self) -> &Scope {
        &self.scopes[ROOT]
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The innermost active scope.
    pub fn innermost(&self) -> &Scope {
        &self.scopes[*self.active.last().expect("the root is always active")]
    }

    /// Admits one operation (DET.56). It adds a row and checks the step in every active scope, root first; a failed
    /// step debits nothing. It then preflights kernel cost, memory (live plus the reservation) and output, each root
    /// first. Equality is admitted. A failed request keeps the step and debits none of the three. On success, all
    /// three are debited together.
    pub fn charge(
        &mut self,
        at: &str,
        source: Option<&SourceRange>,
        kernel_cost: u128,
        memory: u128,
        output: u128,
    ) -> Result<(), Stop> {
        self.charge_at(|| (at.to_string(), source.cloned()), kernel_cost, memory, output)
    }

    /// [`Meter::charge`] with a locator that is called only when the operation's path is needed: for its row, or for
    /// its stop.
    pub fn charge_at(
        &mut self,
        locate: impl FnOnce() -> (String, Option<SourceRange>),
        kernel_cost: u128,
        memory: u128,
        output: u128,
    ) -> Result<(), Stop> {
        let mut locate = Some(locate);
        let mut located = None;
        if self.record {
            let (at, source) = (locate.take().expect("the locator is called once"))();
            self.rows.push(Row { at: at.clone(), steps: 0, kernel_cost: 0, memory: 0, output: 0, released: 0 });
            located = Some((at, source));
        }
        let requests = [(Dim::Steps, 1), (Dim::KernelCost, kernel_cost), (Dim::Memory, memory), (Dim::Output, output)];
        for (dim, amount) in requests {
            let failed = self.active.iter().copied().find(|&s| {
                let scope = &self.scopes[s];
                scope.current(dim).saturating_add(amount) > scope.limits.get(dim)
            });
            if let Some(s) = failed {
                let (at, source) = match located {
                    Some(located) => located,
                    None => (locate.take().expect("the locator is called once"))(),
                };
                let position = Position { at, phase: dim.name(), scope: self.scopes[s].name.clone(), source };
                let live = Some(self.scopes[ROOT].live);
                return Err(Stop { code: dim.code(), outcome: Outcome::CutOff, position, scope: s, live });
            }
            if dim == Dim::Steps {
                if let Some(row) = self.last_row() {
                    row.steps = 1;
                }
                for &s in &self.active {
                    self.scopes[s].used.steps += 1;
                }
            }
        }
        if let Some(row) = self.last_row() {
            (row.kernel_cost, row.memory, row.output) = (kernel_cost, memory, output);
        }
        for &s in &self.active {
            let scope = &mut self.scopes[s];
            scope.used.kernel_cost += kernel_cost;
            scope.used.output += output;
            scope.live += memory;
            scope.reserved += memory;
            scope.used.memory = scope.used.memory.max(scope.live);
        }
        Ok(())
    }

    /// The stop of an interruption observed before an operation (DET.56, KN7.6). It ends the whole run.
    pub fn interrupted(&self, why: Interruption, at: &str, source: Option<&SourceRange>) -> Stop {
        let code = match why {
            Interruption::ByUser => code::CANCELLED_BY_USER,
            Interruption::RevisionReplaced => code::CANCELLED_REVISION_REPLACED,
        };
        let position = Position {
            at: at.to_string(),
            phase: "interruption",
            scope: self.innermost().name.clone(),
            source: source.cloned(),
        };
        Stop { code, outcome: Outcome::Cancelled, position, scope: ROOT, live: Some(self.scopes[ROOT].live) }
    }

    /// The domain error of an admitted operation (DET.56: it keeps its charges). The innermost scope absorbs it.
    pub fn error(&self, code: &'static str, at: &str, source: Option<&SourceRange>) -> Stop {
        let scope = *self.active.last().expect("the root is always active");
        let position = Position {
            at: at.to_string(),
            phase: "effect",
            scope: self.scopes[scope].name.clone(),
            source: source.cloned(),
        };
        Stop { code, outcome: Outcome::Error, position, scope, live: None }
    }

    /// Turns `size` units of the admitted reservation into an owner, counted by every active scope (DET.58).
    pub fn own(&mut self, size: u128) -> OwnerId {
        for &s in &self.active {
            let scope = &mut self.scopes[s];
            scope.reserved = scope.reserved.checked_sub(size).expect("owner exceeds admitted reservation");
        }
        let owner = Some(Owner { size, meters: self.active.clone() });
        match self.free.pop() {
            Some(index) => {
                let slot = &mut self.owners[index];
                slot.generation += 1;
                slot.owner = owner;
                OwnerId { index, generation: slot.generation }
            }
            None => {
                self.owners.push(Slot { generation: 0, owner });
                OwnerId { index: self.owners.len() - 1, generation: 0 }
            }
        }
    }

    fn owner(&self, owner: OwnerId) -> Option<&Owner> {
        self.owners.get(owner.index).filter(|s| s.generation == owner.generation).and_then(|s| s.owner.as_ref())
    }

    /// The payload size of a live owner.
    pub fn size(&self, owner: OwnerId) -> u128 {
        self.owner(owner).expect("live owner").size
    }

    /// Whether an owner is still live. An unwound child drops its owners.
    pub fn is_live(&self, owner: OwnerId) -> bool {
        self.owner(owner).is_some()
    }

    /// Releases `size` units of unowned reservation in every active scope, recorded on the last row (DET.56: an excess
    /// result reservation shrinks to the actual size; scratch is released after use).
    pub fn release(&mut self, size: u128) {
        for &s in &self.active {
            let scope = &mut self.scopes[s];
            scope.reserved = scope.reserved.checked_sub(size).expect("release exceeds unowned reservation");
            scope.live = scope.live.checked_sub(size).expect("negative live charge");
        }
        if let Some(row) = self.last_row() {
            row.released += size;
        }
    }

    /// Drops an owner (DET.58). A drop inside the program is recorded on the last row; cleanup is not.
    pub fn drop_owner(&mut self, owner: OwnerId, record: bool) {
        let slot = &mut self.owners[owner.index];
        assert_eq!(slot.generation, owner.generation, "a dropped owner is not dropped again");
        let dropped = slot.owner.take().expect("an owner is dropped once");
        self.free.push(owner.index);
        for s in dropped.meters {
            let scope = &mut self.scopes[s];
            scope.live = scope.live.checked_sub(dropped.size).expect("negative live charge");
        }
        if record {
            if let Some(row) = self.last_row() {
                row.released += dropped.size;
            }
        }
    }

    /// Opens a child scope (DET.60). The `at/child` step is charged in the enclosing scopes. Each requested cap is then
    /// cut to what every active scope has left: its limit less its cumulative use, or less its live memory.
    pub fn enter_child(&mut self, at: &str, source: Option<&SourceRange>, requested: Amounts) -> Result<Child, Stop> {
        self.charge(&format!("{at}/child"), source, 0, 0, 0)?;
        let mut effective = requested;
        for dim in Dim::ALL {
            for &s in &self.active {
                let scope = &self.scopes[s];
                let left = scope.limits.get(dim) - scope.current(dim);
                *effective.slot(dim) = effective.get(dim).min(left);
            }
        }
        self.scopes.push(Scope::new(at, effective));
        let scope = self.scopes.len() - 1;
        self.active.push(scope);
        Ok(Child { scope, at: at.to_string(), requested, effective })
    }

    /// Closes a child scope with its body's result: the size of the owner it returned to the parent, or its stop.
    /// - On success, owners that survive (the returned result) leave the child's count.
    /// - On a stop, the child is unwound without charge: its unowned reservations and owners are released in every
    ///   scope that counts them.
    /// - The child absorbs its own stop and resumes the parent; a stop of an enclosing scope goes on.
    pub fn leave_child(&mut self, child: Child, result: Result<u128, Stop>) -> Result<(), Stop> {
        let c = child.scope;
        assert_eq!(self.active.last(), Some(&c), "children close in order");
        let (outcome, code, stop, returned_memory, propagate) = match result {
            Ok(returned) => {
                let mut freed = 0;
                for owner in self.owners.iter_mut().filter_map(|slot| slot.owner.as_mut()) {
                    if owner.meters.contains(&c) {
                        freed += owner.size;
                        owner.meters.retain(|&s| s != c);
                    }
                }
                self.scopes[c].live -= freed;
                (Outcome::Completed, None, None, returned, None)
            }
            Err(stop) => {
                let reserved = self.scopes[c].reserved;
                for &s in &self.active {
                    let scope = &mut self.scopes[s];
                    scope.reserved = scope.reserved.checked_sub(reserved).expect("child reservation");
                    scope.live = scope.live.checked_sub(reserved).expect("negative live charge");
                }
                for index in 0..self.owners.len() {
                    let slot = &self.owners[index];
                    if slot.owner.as_ref().is_some_and(|o| o.meters.contains(&c)) {
                        self.drop_owner(OwnerId { index, generation: slot.generation }, false);
                    }
                }
                let propagate = (stop.scope != c).then(|| stop.clone());
                (stop.outcome, Some(stop.code), Some(stop.position), 0, propagate)
            }
        };
        self.active.pop();
        let scope = &self.scopes[c];
        self.children.push(ChildRecord {
            at: child.at,
            requested: child.requested,
            effective: child.effective,
            outcome,
            code,
            stop,
            used: scope.used,
            returned_memory,
            live_after_return: scope.live,
        });
        propagate.map_or(Ok(()), Err)
    }

    /// Ends the run (DET.58). It captures the live charge before cleanup and drops every owner without charge. It also
    /// abandons the reservations of an admitted operation that ended in an error.
    pub fn finish(mut self, stop: Option<Stop>) -> Summary {
        assert_eq!(self.active, [ROOT], "every child is closed");
        let live_before_cleanup = stop.as_ref().and_then(|s| s.live).unwrap_or(self.scopes[ROOT].live);
        for index in 0..self.owners.len() {
            let slot = &self.owners[index];
            if slot.owner.is_some() {
                self.drop_owner(OwnerId { index, generation: slot.generation }, false);
            }
        }
        let root = &mut self.scopes[ROOT];
        root.live = root.live.checked_sub(root.reserved).expect("negative live charge");
        root.reserved = 0;
        let (outcome, code, position) = match stop {
            None => (Outcome::Completed, None, None),
            Some(stop) => (stop.outcome, Some(stop.code), Some(stop.position)),
        };
        Summary {
            outcome,
            code,
            stop: position,
            used: root.used,
            live_before_cleanup,
            live_after_cleanup: root.live,
            rows: self.rows,
            children: self.children,
        }
    }

    /// The row of the last admitted operation, when rows are kept.
    fn last_row(&mut self) -> Option<&mut Row> {
        self.record.then(|| self.rows.last_mut().expect("a charge precedes every release"))
    }

    /// The root's current live memory (owners plus unowned reservations).
    pub fn live(&self) -> u128 {
        self.scopes[ROOT].live
    }
}

/// The tariffs of DET.55 to DET.63. Production tariffs are an open contract (PHS.55, LG4.8), so a run takes them as
/// an input. [`Tariffs::TEST`] holds the contract vectors' values, which are test-only and unmeasured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tariffs {
    pub kernel_base: u128,
    pub kernel_per_item: u128,
    pub limb_bits: u128,
    pub add_per_limb: u128,
    pub mul_per_limb_pair: u128,
    pub text_per_scalar: u128,
    pub decode_per_byte: u128,
    pub encode_per_byte: u128,
    pub rng_init_per_byte: u128,
    pub rng_block: u128,
    pub stream_setup: u128,
    pub expand_per_letter: u128,
    pub replay_per_letter: u128,
}

impl Tariffs {
    /// The tariffs of the frozen contract vectors (`tariff_status`: "test-only; unmeasured"). Never production values.
    pub const TEST: Tariffs = Tariffs {
        kernel_base: 1,
        kernel_per_item: 2,
        limb_bits: 8,
        add_per_limb: 1,
        mul_per_limb_pair: 1,
        text_per_scalar: 1,
        decode_per_byte: 1,
        encode_per_byte: 1,
        rng_init_per_byte: 1,
        rng_block: 3,
        stream_setup: 1,
        expand_per_letter: 1,
        replay_per_letter: 2,
    };

    /// A kernel row over `items` input items.
    pub fn kernel(&self, items: u128) -> u128 {
        self.kernel_base.saturating_add(items.saturating_mul(self.kernel_per_item))
    }

    /// The kernel cost and the result reservation of an exact integer addition or multiplication, from the operands'
    /// bit lengths. The cost counts limbs of `limb_bits` bits. The reservation covers any result of those sizes in
    /// DET.57's integer layout.
    pub fn integer(&self, op: IntOp, a_bits: u128, b_bits: u128) -> (u128, u128) {
        let (a_bits, b_bits) = (a_bits.max(1), b_bits.max(1));
        let limbs = |bits: u128| bits.div_ceil(self.limb_bits.max(1));
        let (cost, bound_bits) = match op {
            IntOp::Add => {
                (limbs(a_bits).saturating_add(limbs(b_bits)).saturating_mul(self.add_per_limb), a_bits.max(b_bits) + 1)
            }
            IntOp::Mul => (
                limbs(a_bits).saturating_mul(limbs(b_bits)).saturating_mul(self.mul_per_limb_pair),
                a_bits.saturating_add(b_bits),
            ),
        };
        (cost, 1u128.saturating_add(bound_bits.div_ceil(8)))
    }
}

/// Tariffs the interpreter needs beyond the vectors' set, for rows the frozen vectors do not price: comparisons of
/// variable-size values, rational and Q(√5) arithmetic, and conversion to an approximation. They are M2 drafts with
/// test-only values; LG4.8 owns the production tariffs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DraftTariffs {
    /// Per byte of both operands of an integer order or a variable-size equality.
    pub compare_per_unit: u128,
    /// Per pair of widened operand bytes of a rational or Q(√5) addition or order.
    pub field_per_unit_pair: u128,
    /// Per pair of bytes of the operand and the result of a conversion to `Approx<p>`.
    pub convert_per_unit_pair: u128,
}

impl DraftTariffs {
    /// Test-only values.
    pub const TEST: DraftTariffs =
        DraftTariffs { compare_per_unit: 1, field_per_unit_pair: 1, convert_per_unit_pair: 1 };
}

/// The two native integer rows the vectors price.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntOp {
    Add,
    Mul,
}

/// DET.57's logical size of an unbounded integer of the given bit length: a sign byte and at least one magnitude byte.
pub fn integer_size(bits: u128) -> u128 {
    1 + bits.div_ceil(8).max(1)
}

/// DET.57's function frame: 16 bytes plus 8 per slot.
pub fn frame_size(slots: u128) -> u128 {
    16u128.saturating_add(slots.saturating_mul(8))
}

/// DET.57's generator owner: the 32-byte key, the 8-byte counter and the exhausted flag.
pub const GENERATOR_SIZE: u128 = 41;
