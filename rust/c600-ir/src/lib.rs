//! The typed IR draft `c600-ir-draft-1` of 600 Cell Script (LP2, milestone M1).
//!
//! The crate holds data and short constructors only. The parser (`c600-syntax`, milestone M7), the checker
//! (`c600-check`) and the later interpreter depend on it and not on each other. The IR is a draft: it changes freely
//! until the IR freeze (chapter 12, PHS.49). Checker tests also build it by hand with [`build`]; a hand lowering is
//! no evidence for the parser.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// The IR draft this crate defines.
pub const IR_VERSION: &str = "c600-ir-draft-1";
/// The exactness and execution contract that the checker and the IR advertise: form 1 of LG2.5 (DET.54, LG2.21).
pub const EXEC_CONTRACT: &str = "lp1-exec-draft-1";
/// The identity of form 2 (LG2.21). Form 2 stays on record and is not built, so a program pinned to it is refused.
pub const FORM2_EXEC_CONTRACT: &str = "lp1-exec-region-draft-1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub ir_version: String,
    pub exec_contract: String,
    pub modules: Vec<Module>,
}

impl Program {
    /// A program pinned to this crate's IR draft and contract.
    pub fn new(modules: Vec<Module>) -> Self {
        Program { ir_version: IR_VERSION.into(), exec_contract: EXEC_CONTRACT.into(), modules }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    pub name: String,
    /// The module kind (LG5.12): an ordinary module or a stage module.
    pub kind: ModuleKind,
    /// The capabilities the module declares (LG3.9). A declaration requests authority and never supplies it.
    pub capabilities: BTreeSet<Capability>,
    pub functions: Vec<Function>,
}

impl Module {
    pub fn declaring(mut self, capabilities: &[Capability]) -> Self {
        self.capabilities.extend(capabilities.iter().copied());
        self
    }

    /// The stage declaration of a stage module.
    pub fn stage(&self) -> Option<&StageDecl> {
        match &self.kind {
            ModuleKind::Stage(decl) => Some(decl),
            ModuleKind::Program => None,
        }
    }
}

/// A module's kind (LG5.12). There is no fourth profile: a stage module is a kind of module, which any profile may use
/// (LG2.24, RPI.68).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ModuleKind {
    /// An ordinary module of 600 Cell Script.
    #[default]
    Program,
    /// A user shader stage (RPI.68 to RPI.76), checked by the stage checker alone and never called by a program.
    Stage(StageDecl),
}

/// What a stage module declares (LP2 draft until the IR freeze, PHS.49). Its entry is its one exported function, whose
/// parameters are the stage's inputs; frame time is one of them, never an ambient read (RPI.69).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageDecl {
    pub kind: StageKind,
    /// The declared static costs (RPI.71): weighted scalar operations for one vertex evaluation (`stage_ops`) or one
    /// fragment invocation (`fragment_ops`), and the allowed fragment invocations per surface pixel (`overdraw`).
    pub ops: u64,
    pub overdraw: u64,
    /// The entry's immutable array inputs, by parameter name, with their declared lengths. An index into one is checked
    /// statically (RPI.69).
    pub arrays: Vec<(String, u64)>,
}

/// The two stage kinds (RPI.68). There is no compute stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StageKind {
    /// A chart evaluator: it returns a position, or a typed coverage code where the map is undefined.
    Vertex,
    /// It returns an opaque RGB value, with no discard, depth, stencil or coverage change (RPI.75).
    Fragment,
}

impl StageKind {
    pub fn name(self) -> &'static str {
        match self {
            StageKind::Vertex => "vertex",
            StageKind::Fragment => "fragment",
        }
    }
}

/// The binary32 operations that only a stage module uses (RPI.70; LP2 draft): with `approx_add`, `approx_less`,
/// `approx_and` and `approx_not` they make the stage's operation table. A program never runs one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StageOp {
    Sub,
    Mul,
    Div,
    Neg,
    Abs,
    Sqrt,
    Sin,
    Cos,
    Exp2,
    Log2,
    /// `exclude(v, radius)`: in a vertex stage, the point is undefined (coverage `singular_neighbourhood`) where
    /// `abs(v) <= radius`, as in LX-7's subset; `radius` is a positive binary32 literal.
    Exclude,
}

impl StageOp {
    pub const ALL: [StageOp; 11] = [
        StageOp::Sub,
        StageOp::Mul,
        StageOp::Div,
        StageOp::Neg,
        StageOp::Abs,
        StageOp::Sqrt,
        StageOp::Sin,
        StageOp::Cos,
        StageOp::Exp2,
        StageOp::Log2,
        StageOp::Exclude,
    ];

    /// The operation's reserved name in source.
    pub fn name(self) -> &'static str {
        match self {
            StageOp::Sub => "approx_sub",
            StageOp::Mul => "approx_mul",
            StageOp::Div => "approx_div",
            StageOp::Neg => "approx_neg",
            StageOp::Abs => "approx_abs",
            StageOp::Sqrt => "approx_sqrt",
            StageOp::Sin => "approx_sin",
            StageOp::Cos => "approx_cos",
            StageOp::Exp2 => "approx_exp2",
            StageOp::Log2 => "approx_log2",
            StageOp::Exclude => "exclude",
        }
    }

    pub fn of_name(name: &str) -> Option<StageOp> {
        StageOp::ALL.into_iter().find(|op| op.name() == name)
    }

    pub fn arity(self) -> usize {
        match self {
            StageOp::Sub | StageOp::Mul | StageOp::Div | StageOp::Exclude => 2,
            _ => 1,
        }
    }

    /// RPI.70's tier: 1 for `+ - * /`, unary minus and `abs`, 2 for `sqrt`, 3 for `sin`, `cos`, `exp2` and `log2`.
    /// The draft puts `exclude` in tier 1, as a comparison.
    pub fn tier(self) -> u8 {
        match self {
            StageOp::Sqrt => 2,
            StageOp::Sin | StageOp::Cos | StageOp::Exp2 | StageOp::Log2 => 3,
            _ => 1,
        }
    }
}

/// A view is reactive and has no workspace or preview effect (LG3.5, LG3.6); a command is invoked by the solver. A
/// whole-state contract (LG4.10; LP2 draft until the IR freeze) takes no parameter, returns an exact `Bool` and reads
/// its revision's state and nothing else; it runs once per revision or as an agreement test, and no function calls or
/// holds it, so no call or loop runs it per call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FnKind {
    Helper,
    View,
    Command,
    Contract,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub kind: FnKind,
    pub exported: bool,
    pub params: Vec<Param>,
    pub result: QTy,
    /// An effect annotation. It restricts what checks successfully and cannot erase an inferred effect (LG3.2).
    pub effects: Option<EffectBound>,
    pub body: Expr,
}

impl Function {
    pub fn exported(mut self) -> Self {
        self.exported = true;
        self
    }

    pub fn with_effects(mut self, effects: EffectBound) -> Self {
        self.effects = Some(effects);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: QTy,
}

/// A base type with LG2.19's display qualifier. `Approx<p>` and `ApproxBool` carry it by definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QTy {
    pub ty: Ty,
    pub display: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ty {
    Unit,
    Bool,
    Nat,
    Int,
    /// An exact rational (LG1.6).
    Rat,
    /// An exact element of Q(√5): rational coefficients of 1 and √5 (LG1.6).
    Q5,
    Text,
    Word,
    Target,
    Budget,
    Status,
    Colour,
    /// `Approx<p>`: an approximate scalar of precision p.
    Approx(u32),
    /// The result of an approximate comparison (LG2.19).
    ApproxBool,
    Record(Vec<(String, QTy)>),
    Fn(Box<FnTy>),
    /// A preview receipt (BND.42).
    Receipt,
    /// A generic type parameter; substitution preserves the qualifier (LG2.19).
    Var(String),
    /// `List<T>`: a finite sequence in written order. Draft (LP2) until the IR freeze (PHS.49), like every
    /// collection and word form below.
    List(Box<QTy>),
    /// LG2.13's word equality key of a Full word, under LG2.18's free reduction. It is never a `Word` and never
    /// converts to one (CT-LG2-11).
    WordEqualityKey,
    /// `Set<T>`: distinct exact elements (LG2.20), kept in their canonical key order (DET.9).
    Set(Box<QTy>),
    /// `Map<K, V>`: values under distinct exact keys (LG2.20), kept in the keys' canonical key order (DET.9).
    Map(Box<QTy>, Box<QTy>),
    /// LG2.16's compressed word: a straight-line program over literal letters whose exact expanded length is known
    /// before any expansion (KN2.10). It is never a `Word`: its letters come out by an explicit, priced expansion.
    CompressedWord,
    /// LB1.8's literal word of jumbling menu letters (LP2 draft): letters `[cap, item]` in written order, a pole index
    /// of the model and an index in the pinned order of one menu, each counted from 0. The alphabet is the menu a run
    /// pins, named by its jumbling identity block (KN0.1); it is not part of the type, and a run pins at most one. A
    /// menu word is never a `Word`, and a mix of the two is refused with `LB1:input.incompatible_alphabet`.
    MenuWord,
    /// A verdict of one of LB3.2's end tests (LP2 draft): `holds`, `fails` or `unestablished` for that test alone,
    /// read from the engine row of the test's name. It is used through a `match` on its outcomes only, never as a
    /// `Bool` (LG2.9), and one test's verdict never stands for another's.
    EndTest(EndTest),
    /// LB1.10's trace equality key of a Full word (`trace-key/1`, LP2 draft; EV4.9's name): the word's letters in the
    /// least topological order of its trace under the independence relation a run pins, with that relation's header
    /// (LG2.23). It is never a `Word` or a `WordEqualityKey` and never converts to either.
    TraceEqualityKey,
    /// DET.61's generator (LP2 draft): the owner of one stream's cursor, the key of its sealed seed and path and its
    /// draw counter. `rng_init` starts one and `rng_draw` draws from it. A generator stays a bare value: no record,
    /// collection, closure capture or type variable holds one, and it has no equality, key order or canonical form.
    Generator,
    /// An element of the finite field F2 or F5, an exact number kind of KN3.2 and ADR-L03.2 (LP2 draft until the IR
    /// freeze, PHS.49). It has no literal (LG1.6): a value comes from `to_f2` or `to_f5` of an `Int`. It never widens
    /// to or from another number kind, and it has no canonical stored form (FP.6 wraps only int, rat and q5), so a
    /// program stores its `residue`.
    Field(Field),
}

/// LB3.2's end tests (LP2 draft): three independently named tests with different premises and conclusions (state
/// contract A5). A retained checkpoint implies a label checkpoint, and the sticker test does for a reachable input;
/// those implications are kernel facts, not conversions between the types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EndTest {
    /// A retained legal witness to the exact pose state.
    RetainedCheckpoint,
    /// The reachable-lattice premises and ν = 0 under the pinned C5 convention, certified without a witness word.
    LabelCheckpoint,
    /// N = 0 and every labelled slot at home.
    StickerTest,
}

impl EndTest {
    /// The tests in their declared order, which the wire's kind byte counts.
    pub const ALL: [EndTest; 3] = [EndTest::RetainedCheckpoint, EndTest::LabelCheckpoint, EndTest::StickerTest];

    /// The engine row that answers the test (KN2.4), which is also its library entry's name (LB3.1).
    pub fn row(self) -> &'static str {
        match self {
            EndTest::RetainedCheckpoint => "retained_checkpoint",
            EndTest::LabelCheckpoint => "label_checkpoint",
            EndTest::StickerTest => "sticker_test",
        }
    }

    /// The test whose engine row is `row`, if any.
    pub fn of_row(row: &str) -> Option<EndTest> {
        EndTest::ALL.into_iter().find(|t| t.row() == row)
    }

    /// The name of the verdict's type in source. The test's own name stays the goal's (LG2.10), which waits for the
    /// IR's goals; a verdict is the test's answer for one input, not the predicate.
    pub fn type_name(self) -> &'static str {
        match self {
            EndTest::RetainedCheckpoint => "RetainedCheckpointVerdict",
            EndTest::LabelCheckpoint => "LabelCheckpointVerdict",
            EndTest::StickerTest => "StickerTestVerdict",
        }
    }

    /// The test whose verdict's type `name` names, if any.
    pub fn of_type_name(name: &str) -> Option<EndTest> {
        EndTest::ALL.into_iter().find(|t| t.type_name() == name)
    }
}

/// An end test's verdict (LP2 draft): KN0.5's yes, no or unknown, named by the status tags a `match` takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EndTestOutcome {
    Holds,
    Fails,
    /// Unknown, with no evidence either way, such as an imported table without its reachability record. It never
    /// reads as `Fails` (LG2.9).
    Unestablished,
}

impl EndTestOutcome {
    /// The outcomes in their declared order, which the wire's outcome byte counts.
    pub const ALL: [EndTestOutcome; 3] = [EndTestOutcome::Holds, EndTestOutcome::Fails, EndTestOutcome::Unestablished];

    /// The status tag that names the outcome in a `match` pattern.
    pub fn tag(self) -> &'static str {
        match self {
            EndTestOutcome::Holds => "holds",
            EndTestOutcome::Fails => "fails",
            EndTestOutcome::Unestablished => "unestablished",
        }
    }

    /// The outcome the status tag `tag` names, if any.
    pub fn of_tag(tag: &str) -> Option<EndTestOutcome> {
        EndTestOutcome::ALL.into_iter().find(|o| o.tag() == tag)
    }
}

/// The finite fields of KN3.2: F2 and F5 (LP2 draft).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
    F2,
    F5,
}

impl Field {
    /// The field's prime.
    pub fn modulus(self) -> u8 {
        match self {
            Field::F2 => 2,
            Field::F5 => 5,
        }
    }

    /// The field's type name in source.
    pub fn name(self) -> &'static str {
        match self {
            Field::F2 => "F2",
            Field::F5 => "F5",
        }
    }
}

impl Ty {
    /// The element types of a collection type, in order (a map's key, then its value); empty for other types.
    pub fn elements(&self) -> Vec<&QTy> {
        match self {
            Ty::List(e) | Ty::Set(e) => vec![&**e],
            Ty::Map(k, v) => vec![&**k, &**v],
            _ => Vec::new(),
        }
    }

    /// The collection type of the same kind with its element types replaced, in [`Ty::elements`] order.
    pub fn with_elements(&self, mut parts: Vec<QTy>) -> Ty {
        let mut next = || Box::new(parts.remove(0));
        match self {
            Ty::List(_) => Ty::List(next()),
            Ty::Set(_) => Ty::Set(next()),
            Ty::Map(..) => {
                let key = next();
                Ty::Map(key, next())
            }
            other => other.clone(),
        }
    }

    /// Whether two types are collections of the same kind.
    pub fn same_collection(&self, other: &Ty) -> bool {
        matches!((self, other), (Ty::List(_), Ty::List(_)) | (Ty::Set(_), Ty::Set(_)) | (Ty::Map(..), Ty::Map(..)))
    }
}

/// The named order of an ordered collection result (LG4.3): the canonical key order (DET.9) or its reverse. Draft.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    Ascending,
    Descending,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FnTy {
    pub params: Vec<QTy>,
    pub result: QTy,
    pub effects: EffectBound,
}

/// Effects, capability requirements and effect variables of a function type (LG3.1, LG3.2).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EffectBound {
    pub effects: BTreeSet<Effect>,
    pub capabilities: BTreeSet<Capability>,
    pub vars: BTreeSet<String>,
}

/// LG3.1's effect families. Pure is the empty set. There is no commit effect (LG3.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Effect {
    ReadSnapshot,
    ChangeView,
    ChangeWorkspace,
    PreparePreview,
    GrantedFile,
    SeededRandomness,
    Budgets,
}

/// Capabilities (LG3.8). Network, process and session-file access are never granted in 1.0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    File,
    Clock,
    Randomness,
    Network,
    Process,
    SessionFile,
}

/// A literal. Exact numbers are decimal text, never a host integer or float: an integer has no leading zeros and no
/// sign unless negative, and a ratio keeps the sign on its numerator and a nonzero denominator (LG1.6). Reducing a
/// ratio to lowest terms waits for the interpreter's big integers (M2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lit {
    Unit,
    Bool(bool),
    Nat(String),
    Int(String),
    Rat {
        num: String,
        den: String,
    },
    /// `one + sqrt5·√5`, each coefficient a ratio `(numerator, denominator)`.
    Q5 {
        one: (String, String),
        sqrt5: (String, String),
    },
    /// An approximate literal: its decimal text and declared precision in bits. It is never read as exact (LG1.7).
    Approx {
        decimal: String,
        precision: u32,
    },
    Text(String),
    /// A literal word in LX-8's retained alphabet: signed letters of magnitude 1 to 1200, in written order. The kernel
    /// interface gives them their meaning (M4).
    Word(Vec<i32>),
    Target(String),
    /// A status tag, such as a budget outcome code.
    Tag(String),
    Colour(String),
    /// A literal menu word (LP2 draft): letters `[cap, item]` in written order, each index counted from 0 and below
    /// [`MENU_INDEX_LIMIT`]. The run's pinned menu gives them their meaning (LB1.8); a letter outside it stops the
    /// run.
    MenuWord(Vec<[u32; 2]>),
}

/// The bound of a menu letter's indices (LB1.8; LP2 draft): a cap is a pole index of the jumbling model, which has 600
/// poles (`c600_jumble::model::POLES`), and the draft bounds a menu's items alike.
pub const MENU_INDEX_LIMIT: u32 = 600;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prim {
    /// Exact arithmetic and comparison on `Nat`, `Int` or `Budget`.
    ExactAdd,
    ExactLess,
    ExactEq,
    /// Short-circuit exact booleans (LG2.20).
    And,
    Or,
    Not,
    /// Explicit exact-to-approximate conversion; it names the precision (LG2.19, CT-LG2-04).
    ToApprox(u32),
    /// An attempted scalar conversion back to exact. It exists to be refused (LG2.19, LG2.22).
    ToExact,
    ApproxAdd,
    ApproxLess,
    ApproxAnd,
    ApproxNot,
    // Collections and words (LP2 drafts until the IR freeze, PHS.49).
    /// `length(x)`: the element count of a list or a set, the entry count of a map, or the exact length of a word, a
    /// menu word or a compressed word (LG2.16), as `Nat`. A compressed word's length needs no expansion.
    Length,
    /// `index(list, i)`: the element at exact position `i`, counted from 0 (LG2.20).
    Index,
    /// `push(list, x)`: the list with `x` appended.
    Push,
    /// `concat(a, b)`: two words, two menu words, two lists or two compressed words joined in order; compressed words
    /// join without expansion.
    Concat,
    /// `inverse(w)`: the letters in reverse order, each negated (LG2.15). A menu word's letters are reversed, each at
    /// its own cap with the item of the inverse rotation in the run's pinned menu (LB1.8, LB1.9).
    Inverse,
    /// `conjugate(x, y)`: x · y · x⁻¹ (LG2.15), on two words or two menu words.
    Conjugate,
    /// `commutator(x, y)`: x · y · x⁻¹ · y⁻¹ (LG2.15), on two words or two menu words.
    Commutator,
    /// `compose(x, y)`: the action of x, then of y; its letters are x's, then y's, in chronological order (LG2.15). On
    /// two words or two menu words.
    Compose,
    /// `word_key(w)`: LG2.13's key of a Full word, LG2.18's stack free reduction of adjacent inverse pairs.
    WordKey,
    /// `contains(c, k)`: whether a set holds the element `k`, or a map the key `k`. `k` is exact (LG2.20).
    Contains,
    /// `insert(s, x)`: the set with the element `x` added.
    Insert,
    /// `insert(m, k, v)`: the map with `v` under the key `k`, replacing a value under an equal key.
    InsertEntry,
    /// `lookup(m, k)`: the value under the key `k`; a missing key is a pointwise failure.
    Lookup,
    /// `elements(s, order)`: a set's elements as a `List` in the named order (LG4.3).
    Elements(Order),
    /// `keys(m, order)`: a map's keys as a `List` in the named order (LG4.3).
    Keys(Order),
    /// `entries(m, order)`: a map's entries as a `List` of `{ key, value }` records in the named order (LG4.3).
    Entries(Order),
    /// `compress(w)`: the word `w` as a compressed word of one part (LG2.16).
    Compress,
    /// `expand(c, start, count)`: the `count` letters of the compressed word `c` from the exact position `start`,
    /// counted from 0, as a `Word` in chronological order (LG4.6). A range past the word's end is a pointwise failure.
    Expand,
    /// `trace_key(w)`: LB1.10's trace key of a Full word under the run's pinned independence relation (LP2 draft):
    /// its lexicographically least topological word, every occurrence kept.
    TraceKey,
    /// `exact_mul(a, b)`: the exact product of two numbers, widened along `Nat`, `Int`, `Rat` and `Q5` as a sum is
    /// (LP2 draft; DET.57 prices it). A budget is a cost, and a product of costs is none, so it takes no `Budget`.
    ExactMul,
    /// `assert(p)`: a pointwise assertion (LP2 draft; DET.55, LG4.10), of value `Unit`. The predicate `p` is evaluated
    /// first with ordinary operations; the assertion is one step, and a false predicate ends the run with
    /// `LG4:input.pointwise_failed` before the guarded operation, which follows the assertion in the program.
    Assert,
    /// `rng_draw(g)`: the next 256-bit block of the generator `g`'s stream as `Nat` (LP2 draft; DET.61). It advances
    /// `g`'s own cursor, which a borrow of `g` shares and an alias of `g` copies.
    RngDraw,
    // Finite fields (KN3.2; LP2 drafts until the IR freeze, PHS.49). They take operands of one field only and stay
    // apart from `ExactAdd`'s widening.
    /// `to_f2(n)`, `to_f5(n)`: the residue of the `Int` (or `Nat`) `n` modulo the field's prime, as a field element.
    ToField(Field),
    /// `residue(x)`: the field element's representative in `0..p` as a `Nat`.
    Residue,
    /// `field_add(x, y)`: the sum in the field.
    FieldAdd,
    /// `field_sub(x, y)`: the difference in the field.
    FieldSub,
    /// `field_neg(x)`: the additive inverse in the field.
    FieldNeg,
    /// `field_mul(x, y)`: the product in the field.
    FieldMul,
    /// `field_inverse(x)`: the multiplicative inverse in the field. Zero has none: the operation is guarded by a
    /// pointwise assertion that `x` is not zero (DET.55), and its failure is `LG4:input.pointwise_failed`.
    FieldInverse,
    /// A stage operation (RPI.70; LP2 draft). Only a stage module uses one; the checker refuses it elsewhere.
    Stage(StageOp),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Callee {
    Fn {
        module: Option<String>,
        name: String,
    },
    Value(Box<Expr>),
    /// A callable the lowering could not resolve; never admitted as exact or pure (LG2.19, LG3.2).
    Unresolved {
        name: String,
        result: QTy,
    },
}

/// The fields of a preview receipt. Under capability separation a previewing program in the solving profile
/// receives only the ID and the base (BND.42).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptField {
    Id,
    Base,
    Effect,
    Protection,
    EndState,
    N,
    Error,
}

/// Exact sinks that approximate control cannot steer (CT-LG2-05).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExactRole {
    Budget,
    Goal,
    Evidence,
}

/// Display accounting: the host's observations of a display evaluation (LG2.20).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accounting {
    UsedSteps,
    UnusedSteps,
    Status,
}

/// A child scope's requested caps, one per budget dimension (DET.60; LP2 draft). An omitted cap resolves, when the
/// child starts, to what the enclosing scopes have left in that dimension, so it creates no allowance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChildCaps {
    pub steps: Option<u128>,
    pub kernel_cost: Option<u128>,
    pub memory: Option<u128>,
    pub output: Option<u128>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Lit(Lit),
    Var(String),
    FnRef {
        module: Option<String>,
        name: String,
    },
    Let {
        name: String,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    If {
        cond: Box<Expr>,
        then: Box<Expr>,
        els: Box<Expr>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<(Lit, Expr)>,
        default: Box<Expr>,
    },
    /// Form 1's display conditional: an `ApproxBool` condition and lazy pure display branches (LG2.20).
    DisplayIf {
        cond: Box<Expr>,
        then: Box<Expr>,
        els: Box<Expr>,
    },
    Call {
        callee: Callee,
        args: Vec<Expr>,
    },
    /// A closure; its captures are the free variables of its body (LG2.19).
    Closure {
        params: Vec<Param>,
        result: QTy,
        body: Box<Expr>,
    },
    Prim {
        op: Prim,
        args: Vec<Expr>,
    },
    Record(Vec<(String, Expr)>),
    Field {
        value: Box<Expr>,
        field: String,
    },
    /// A collection of type `ty` from its items, evaluated in written order. Draft. A `List` keeps that order; a `Set`
    /// keeps each element once; a `Map`'s items alternate keys and values, and a later key's value replaces an
    /// earlier one's. Sets and maps take their canonical key order (DET.9).
    Collection {
        ty: Ty,
        items: Vec<Expr>,
    },
    /// A snapshot or workspace read. The type carries the stored field's display qualifier: decoding retains it
    /// (LG2.19, storage row).
    ReadSnapshot {
        query: String,
        ty: QTy,
    },
    /// A snapshot read whose query takes an argument (LP2 draft): the engine's row `query` for the argument's value,
    /// such as a word's protection. It is always an engine read, never a workspace record, and the argument crosses to
    /// the host as its canonical bytes (DET.59), so it is exact data with a canonical form.
    ReadSnapshotWith {
        query: String,
        arg: Box<Expr>,
        ty: QTy,
    },
    PublishView {
        value: Box<Expr>,
    },
    WriteWorkspace {
        record: String,
        ty: QTy,
        value: Box<Expr>,
    },
    /// Prepare a literal-word preview (KN5.9); it returns a receipt and never commits (LG3.4).
    Propose {
        word: Box<Expr>,
        target: Box<Expr>,
    },
    ReadReceipt {
        receipt: Box<Expr>,
        field: ReceiptField,
    },
    UseExact {
        role: ExactRole,
        value: Box<Expr>,
    },
    Accounting(Accounting),
    /// A fixed declassifier of chapter 07, identified by its ID. A program cannot declare one (LG3.11).
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
        value: Box<Expr>,
    },
    RandomDraw,
    ReadClock,
    /// `rng_init("segment", ...)` (LP2 draft; DET.61): a new generator on the run's granted seed and sealed stream
    /// path, the run's assigned path followed by these literal segments. The path is fixed before execution, so the
    /// same seed and path give the same stream again.
    RngInit {
        path: Vec<String>,
    },
    /// `child` (LP2 draft; DET.60): `body` runs as a serial subrun under `caps`, in a call frame of its own, and
    /// borrows the names in scope. When it completes, its value is the expression's. When it stops on a cap or an
    /// error of its own, it is unwound and `otherwise` runs in the parent, in a call frame of its own, with `status`,
    /// when it names one, a `Status` bound to the stop's outcome, `cut_off` or `error`. A limit of an enclosing scope
    /// or an interruption stops the enclosing run.
    Child {
        caps: ChildCaps,
        result: QTy,
        body: Box<Expr>,
        status: Option<String>,
        otherwise: Box<Expr>,
    },
    /// `repeat` (LP2 draft; DET.55, LG2.20): a bounded loop over an accumulator. `count`, an exact natural, bounds it;
    /// `init` is bound to `acc`, and `body` runs `count` times, each time with `acc` bound to the value the previous
    /// iteration left. The loop's value is the last one, `init` itself when `count` is 0. Every test of the count
    /// costs one step, the final false one included, and the body runs in the enclosing call frame.
    Repeat {
        count: Box<Expr>,
        init: Box<Expr>,
        acc: String,
        result: QTy,
        body: Box<Expr>,
    },
}

impl Expr {
    /// The child expressions in path order. A diagnostic's path is the chain of child indices from a function body,
    /// and the parser's source map uses the same paths (LG4.12).
    pub fn children(&self) -> Vec<&Expr> {
        match self {
            Expr::Lit(_)
            | Expr::Var(_)
            | Expr::FnRef { .. }
            | Expr::ReadSnapshot { .. }
            | Expr::Accounting(_)
            | Expr::Declassified { .. }
            | Expr::FileRead { .. }
            | Expr::RandomDraw
            | Expr::ReadClock
            | Expr::RngInit { .. } => Vec::new(),
            Expr::Let { value, body, .. } => vec![&**value, &**body],
            Expr::If { cond, then, els } | Expr::DisplayIf { cond, then, els } => vec![&**cond, &**then, &**els],
            Expr::Match { scrutinee, arms, default } => {
                let mut out = vec![&**scrutinee];
                out.extend(arms.iter().map(|(_, arm)| arm));
                out.push(&**default);
                out
            }
            Expr::Call { callee, args } => {
                let mut out: Vec<&Expr> = args.iter().collect();
                if let Callee::Value(f) = callee {
                    out.push(&**f);
                }
                out
            }
            Expr::Closure { body, .. } => vec![&**body],
            Expr::Child { body, otherwise, .. } => vec![&**body, &**otherwise],
            Expr::Repeat { count, init, body, .. } => vec![&**count, &**init, &**body],
            Expr::Prim { args, .. } | Expr::Collection { items: args, .. } => args.iter().collect(),
            Expr::Record(fields) => fields.iter().map(|(_, f)| f).collect(),
            Expr::Field { value, .. }
            | Expr::PublishView { value }
            | Expr::WriteWorkspace { value, .. }
            | Expr::UseExact { value, .. }
            | Expr::FileWrite { value, .. } => vec![&**value],
            Expr::ReadSnapshotWith { arg, .. } => vec![&**arg],
            Expr::Propose { word, target } => vec![&**word, &**target],
            Expr::ReadReceipt { receipt, .. } => vec![&**receipt],
        }
    }
}

/// Short constructors for hand-built IR.
pub mod build {
    use super::*;

    /// The precision the hand lowerings use.
    pub const P: u32 = 53;

    pub fn exact(ty: Ty) -> QTy {
        QTy { ty, display: false }
    }
    pub fn display(ty: Ty) -> QTy {
        QTy { ty, display: true }
    }
    pub fn approx() -> Ty {
        Ty::Approx(P)
    }
    pub fn param(name: &str, ty: QTy) -> Param {
        Param { name: name.into(), ty }
    }
    pub fn var(name: &str) -> Expr {
        Expr::Var(name.into())
    }
    pub fn unit() -> Expr {
        Expr::Lit(Lit::Unit)
    }
    pub fn nat(n: u64) -> Expr {
        Expr::Lit(Lit::Nat(n.to_string()))
    }
    pub fn word(letters: &[i32]) -> Expr {
        Expr::Lit(Lit::Word(letters.to_vec()))
    }
    /// A literal menu word of letters `[cap, item]` (LP2 draft).
    pub fn menu_word(letters: &[[u32; 2]]) -> Expr {
        Expr::Lit(Lit::MenuWord(letters.to_vec()))
    }
    pub fn target(t: &str) -> Expr {
        Expr::Lit(Lit::Target(t.into()))
    }
    pub fn colour(c: &str) -> Expr {
        Expr::Lit(Lit::Colour(c.into()))
    }
    pub fn tag(t: &str) -> Lit {
        Lit::Tag(t.into())
    }
    pub fn let_(name: &str, value: Expr, body: Expr) -> Expr {
        Expr::Let { name: name.into(), value: Box::new(value), body: Box::new(body) }
    }
    pub fn if_(cond: Expr, then: Expr, els: Expr) -> Expr {
        Expr::If { cond: Box::new(cond), then: Box::new(then), els: Box::new(els) }
    }
    pub fn display_if(cond: Expr, then: Expr, els: Expr) -> Expr {
        Expr::DisplayIf { cond: Box::new(cond), then: Box::new(then), els: Box::new(els) }
    }
    pub fn match_(scrutinee: Expr, arms: Vec<(Lit, Expr)>, default: Expr) -> Expr {
        Expr::Match { scrutinee: Box::new(scrutinee), arms, default: Box::new(default) }
    }
    pub fn prim(op: Prim, args: Vec<Expr>) -> Expr {
        Expr::Prim { op, args }
    }
    pub fn to_approx(e: Expr) -> Expr {
        prim(Prim::ToApprox(P), vec![e])
    }
    pub fn approx_less(a: Expr, b: Expr) -> Expr {
        prim(Prim::ApproxLess, vec![a, b])
    }
    pub fn exact_add(a: Expr, b: Expr) -> Expr {
        prim(Prim::ExactAdd, vec![a, b])
    }
    pub fn exact_mul(a: Expr, b: Expr) -> Expr {
        prim(Prim::ExactMul, vec![a, b])
    }
    /// A generator on the run's stream path followed by `path` (LP2 draft; DET.61).
    pub fn rng_init(path: &[&str]) -> Expr {
        Expr::RngInit { path: path.iter().map(|s| s.to_string()).collect() }
    }
    pub fn rng_draw(g: Expr) -> Expr {
        prim(Prim::RngDraw, vec![g])
    }
    /// A child scope (LP2 draft; DET.60): `body` under `caps`, and `otherwise` after its stop, with `status` bound
    /// when it is named.
    pub fn child(caps: ChildCaps, result: QTy, body: Expr, status: Option<&str>, otherwise: Expr) -> Expr {
        let status = status.map(str::to_string);
        Expr::Child { caps, result, body: Box::new(body), status, otherwise: Box::new(otherwise) }
    }
    /// A bounded loop (LP2 draft; DET.55): `body` runs `count` times with `acc` bound to the value so far, from `init`.
    pub fn repeat(count: Expr, init: Expr, acc: &str, result: QTy, body: Expr) -> Expr {
        Expr::Repeat { count: Box::new(count), init: Box::new(init), acc: acc.into(), result, body: Box::new(body) }
    }
    pub fn to_field(field: Field, e: Expr) -> Expr {
        prim(Prim::ToField(field), vec![e])
    }
    pub fn call(name: &str, args: Vec<Expr>) -> Expr {
        Expr::Call { callee: Callee::Fn { module: None, name: name.into() }, args }
    }
    pub fn call_in(module: &str, name: &str, args: Vec<Expr>) -> Expr {
        Expr::Call { callee: Callee::Fn { module: Some(module.into()), name: name.into() }, args }
    }
    pub fn call_value(f: Expr, args: Vec<Expr>) -> Expr {
        Expr::Call { callee: Callee::Value(Box::new(f)), args }
    }
    pub fn fn_ref(name: &str) -> Expr {
        Expr::FnRef { module: None, name: name.into() }
    }
    pub fn closure(params: Vec<Param>, result: QTy, body: Expr) -> Expr {
        Expr::Closure { params, result, body: Box::new(body) }
    }
    pub fn record(fields: Vec<(&str, Expr)>) -> Expr {
        Expr::Record(fields.into_iter().map(|(n, e)| (n.to_string(), e)).collect())
    }

    /// `List<elem>` with the given items (draft).
    pub fn list(elem: QTy, items: Vec<Expr>) -> Expr {
        Expr::Collection { ty: Ty::List(Box::new(elem)), items }
    }

    /// `Set<elem>` with the given items (draft).
    pub fn set(elem: QTy, items: Vec<Expr>) -> Expr {
        Expr::Collection { ty: Ty::Set(Box::new(elem)), items }
    }

    /// `Map<key, value>` with the given entries in written order (draft).
    pub fn map(key: QTy, value: QTy, entries: Vec<(Expr, Expr)>) -> Expr {
        let items = entries.into_iter().flat_map(|(k, v)| [k, v]).collect();
        Expr::Collection { ty: Ty::Map(Box::new(key), Box::new(value)), items }
    }
    pub fn field(value: Expr, name: &str) -> Expr {
        Expr::Field { value: Box::new(value), field: name.into() }
    }
    pub fn read_snapshot(query: &str, ty: QTy) -> Expr {
        Expr::ReadSnapshot { query: query.into(), ty }
    }
    pub fn read_snapshot_with(query: &str, arg: Expr, ty: QTy) -> Expr {
        Expr::ReadSnapshotWith { query: query.into(), arg: Box::new(arg), ty }
    }
    pub fn publish_view(value: Expr) -> Expr {
        Expr::PublishView { value: Box::new(value) }
    }
    pub fn write_workspace(record: &str, ty: QTy, value: Expr) -> Expr {
        Expr::WriteWorkspace { record: record.into(), ty, value: Box::new(value) }
    }
    pub fn propose(word: Expr, target: Expr) -> Expr {
        Expr::Propose { word: Box::new(word), target: Box::new(target) }
    }
    pub fn read_receipt(receipt: Expr, field: ReceiptField) -> Expr {
        Expr::ReadReceipt { receipt: Box::new(receipt), field }
    }
    pub fn use_exact(role: ExactRole, value: Expr) -> Expr {
        Expr::UseExact { role, value: Box::new(value) }
    }
    pub fn bound(effects: &[Effect], vars: &[&str]) -> EffectBound {
        EffectBound {
            effects: effects.iter().copied().collect(),
            capabilities: BTreeSet::new(),
            vars: vars.iter().map(|v| v.to_string()).collect(),
        }
    }
    pub fn fn_ty(params: Vec<QTy>, result: QTy, effects: EffectBound) -> Ty {
        Ty::Fn(Box::new(FnTy { params, result, effects }))
    }
    pub fn function(kind: FnKind, name: &str, params: Vec<Param>, result: QTy, body: Expr) -> Function {
        Function { name: name.into(), kind, exported: false, params, result, effects: None, body }
    }
    pub fn helper(name: &str, params: Vec<Param>, result: QTy, body: Expr) -> Function {
        function(FnKind::Helper, name, params, result, body)
    }
    pub fn view(name: &str, params: Vec<Param>, result: QTy, body: Expr) -> Function {
        function(FnKind::View, name, params, result, body)
    }
    pub fn command(name: &str, params: Vec<Param>, result: QTy, body: Expr) -> Function {
        function(FnKind::Command, name, params, result, body)
    }
    /// A whole-state contract with the result `result` (LG4.10; LP2 draft): no parameter.
    pub fn contract(name: &str, result: QTy, body: Expr) -> Function {
        function(FnKind::Contract, name, Vec::new(), result, body)
    }
    pub fn module(name: &str, functions: Vec<Function>) -> Module {
        Module { name: name.into(), kind: ModuleKind::Program, capabilities: BTreeSet::new(), functions }
    }
    /// A stage module with no array inputs.
    pub fn stage(name: &str, kind: StageKind, ops: u64, overdraw: u64, functions: Vec<Function>) -> Module {
        let decl = StageDecl { kind, ops, overdraw, arrays: Vec::new() };
        Module { name: name.into(), kind: ModuleKind::Stage(decl), capabilities: BTreeSet::new(), functions }
    }
    /// A binary32 scalar, `Approx<24>` (RPI.69).
    pub fn f32() -> QTy {
        display(Ty::Approx(24))
    }
    /// A binary32 literal.
    pub fn f32_lit(decimal: &str) -> Expr {
        Expr::Lit(Lit::Approx { decimal: decimal.into(), precision: 24 })
    }
    pub fn stage_op(op: StageOp, args: Vec<Expr>) -> Expr {
        Expr::Prim { op: Prim::Stage(op), args }
    }
    pub fn program(modules: Vec<Module>) -> Program {
        Program::new(modules)
    }
}
