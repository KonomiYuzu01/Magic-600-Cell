//! The parsed program. Every node keeps its source range in a [`Loc`], which equality ignores, so two parses compare
//! equal when they hold the same program wherever its text sits (LG1.4). A parsed file also keeps its trivia and, for
//! each recovery hole, what the parser read of the broken declaration (EV1.2); neither is part of the program.

use crate::{Diagnostic, Span};

/// A node's source range. Equality ignores it.
#[derive(Clone, Copy, Debug)]
pub struct Loc(pub Span);

impl PartialEq for Loc {
    fn eq(&self, _: &Loc) -> bool {
        true
    }
}

impl Eq for Loc {}

/// One parsed file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParsedFile {
    /// SHA-256 of the pinned source bytes (LG1.3); absent when the bytes are refused.
    pub digest: Option<String>,
    /// The edition the header names, supported or not.
    pub edition: Option<String>,
    pub module: Option<ModuleHeader>,
    pub items: Vec<Item>,
    /// Declarations omitted after a syntax error. They are diagnostic values and never lower (LG1.5).
    pub holes: Vec<Span>,
    pub diagnostics: Vec<Diagnostic>,
    /// What the parser read of each hole's declaration before it stopped, one per hole and in the same order (EV1.2):
    /// a structure for completion and navigation to read, never an executable meaning.
    pub recovered: Vec<RecoveredDecl>,
    /// The comments and whitespace runs after the header's checks, in source order (EV1.2; M7 deferred them,
    /// PHS.21). They are not part of the program: [`ParsedFile::same_program`] and the canonical layout ignore them.
    pub trivia: Vec<Trivia>,
}

/// What a piece of trivia is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriviaKind {
    /// A run of spaces, tabs and newlines.
    Whitespace,
    /// A `#` comment, up to its line's end and without the newline.
    Comment,
}

/// Source text between tokens: a comment or a whitespace run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trivia {
    pub kind: TriviaKind,
    pub span: Span,
}

/// How a name inside a declaration is bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingKind {
    /// A type or effect parameter, `<T, E>`.
    Generic,
    Parameter,
    Let,
    ClosureParameter,
    /// A binder of a constructor pattern's arm, `ctor(a, b)` (T1 draft).
    MatchBinder,
}

/// A name bound in a declaration that became a recovery hole, as far as the parser read it (EV1.2). It is visible in
/// the byte interval `[from, to]`: from the end of its binding (after the `;` for a `let`) to the end of its scope,
/// or to the end of the hole where the scope did not close.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveredBinding {
    pub name: Name,
    pub kind: BindingKind,
    /// The declared type of a parameter.
    pub ty: Option<TypeExpr>,
    pub from: usize,
    pub to: usize,
}

/// What the parser read of a declaration before it stopped (EV1.2). The hole stays a diagnostic value: nothing here
/// lowers, and a draft with a hole has no layout (LG1.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveredDecl {
    /// The hole's range, as in [`ParsedFile::holes`].
    pub hole: Span,
    /// The token at which parsing stopped.
    pub stopped: Span,
    /// The declaration's kind, once its keyword was read; `None` for a broken import or an unknown start.
    pub kind: Option<DeclKind>,
    pub exported: bool,
    /// The declaration's name, once it was read.
    pub name: Option<Name>,
    /// The names bound so far, in source order.
    pub bindings: Vec<RecoveredBinding>,
}

impl ParsedFile {
    /// Whether two parses hold the same program: edition, module header and items, ignoring locations and digests.
    pub fn same_program(&self, other: &ParsedFile) -> bool {
        self.edition == other.edition && self.module == other.module && self.items == other.items
    }

    /// The declarations, in source order.
    pub fn declarations(&self) -> impl Iterator<Item = &Decl> {
        self.items.iter().filter_map(|item| match item {
            Item::Decl(d) => Some(&**d),
            Item::Import(_) | Item::Type(_) => None,
        })
    }

    /// The sum type declarations, in source order (T1 draft).
    pub fn types(&self) -> impl Iterator<Item = &TypeDecl> {
        self.items.iter().filter_map(|item| match item {
            Item::Type(t) => Some(&**t),
            Item::Import(_) | Item::Decl(_) => None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Name {
    pub text: String,
    pub loc: Loc,
}

/// `module name requires [capabilities];`, or a stage module's `stage kind name ops n overdraw n inputs [a: n];`
/// (LG5.12, RPI.68; LP2 draft), which requires no capability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleHeader {
    pub name: Name,
    pub requires: Vec<Name>,
    pub stage: Option<StageHeader>,
    pub loc: Loc,
}

/// A stage module's kind and declared costs (RPI.71) and its array inputs, each a parameter name of the entry with a
/// length (RPI.69). The `inputs` list is optional; numbers are canonical naturals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageHeader {
    /// `vertex` or `fragment`.
    pub kind: Name,
    pub ops: String,
    pub overdraw: String,
    pub inputs: Vec<(Name, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    Import(Import),
    Decl(Box<Decl>),
    /// A sum type declaration (LG2.1; T1 draft until the IR freeze, PHS.49).
    Type(Box<TypeDecl>),
}

/// `type Shape { circle(Nat); empty(); }` or `export type ...` in column 1 (LG2.1; T1 draft until the IR freeze,
/// PHS.49): constructors with fixed payload types, written as parameter types, each constructor ending with `;`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDecl {
    pub exported: bool,
    pub name: Name,
    pub ctors: Vec<CtorDecl>,
    pub loc: Loc,
}

/// One constructor of a declared sum type, `circle(Nat)` or nullary `empty()` (T1 draft).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CtorDecl {
    pub name: Name,
    pub payload: Vec<TypeExpr>,
    pub loc: Loc,
}

/// `import alias "<sha256>";`: a module named by the digest of its source, not by a path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Import {
    pub alias: Name,
    pub digest: String,
    pub loc: Loc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclKind {
    Fn,
    View,
    Command,
    /// A whole-state contract (LG4.10; LP2 draft).
    Contract,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decl {
    pub exported: bool,
    pub kind: DeclKind,
    pub name: Name,
    /// Type and effect parameters, `<T, E>`.
    pub generics: Vec<Name>,
    pub params: Vec<ParamDecl>,
    pub result: TypeExpr,
    pub effects: Vec<Name>,
    /// `charge [steps, kernel, memory, output]` in canonical decimal text.
    pub charge: Option<Vec<String>>,
    pub metadata: Option<Metadata>,
    pub body: Block,
    pub loc: Loc,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamDecl {
    pub name: Name,
    pub ty: TypeExpr,
}

/// A command's table row: `row "s" kind k contexts [..] permits [..]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub row: String,
    pub kind: Name,
    pub contexts: Vec<Name>,
    pub permits: Vec<Name>,
}

/// `{ let a = e; ... return e; }`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub lets: Vec<LetStmt>,
    pub result: Expr,
    pub loc: Loc,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LetStmt {
    pub name: Name,
    pub value: Expr,
    pub loc: Loc,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeExpr {
    /// `Name` or `Name<A,B>`, including `Display<T>`, `Approx<P53>` and the identity tower's `State<Jumbling>`; or
    /// `alias.Name`, a sum type an imported module declares (T1 draft). The index of a tower type, a puzzle, a level,
    /// a goal or a cost unit, is written as a type argument without arguments of its own.
    Named { module: Option<Name>, name: Name, args: Vec<TypeExpr>, loc: Loc },
    /// `fn(A, B) -> R effects [..]`
    Fn { params: Vec<TypeExpr>, result: Box<TypeExpr>, effects: Vec<Name>, loc: Loc },
    /// `record { a: T, b: U }`
    Record { fields: Vec<(Name, TypeExpr)>, loc: Loc },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expr {
    pub kind: ExprKind,
    pub loc: Loc,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExprKind {
    /// An integer in canonical decimal text (LG1.6).
    Int(String),
    /// A ratio in canonical decimal text, the sign on the numerator and a nonzero denominator; not reduced.
    Rat {
        num: String,
        den: String,
    },
    /// `q5(a, b)`: the rational coefficients of 1 and √5.
    Q5 {
        one: (String, String),
        sqrt5: (String, String),
    },
    /// `~p"decimal"`: an approximate literal and its precision in bits (LG1.7).
    Approx {
        precision: u32,
        decimal: String,
    },
    Text(String),
    Bool(bool),
    Unit,
    /// `word[+1, -2]`: signed letters of magnitude 1 to 1200.
    Word(Vec<i32>),
    /// `menu[2:5, 0:1]`: letters of the run's pinned menu (LB1.8; LP2 draft), each a cap and an item, naturals
    /// below 600 counted from 0; the item is kept as its canonical decimal text. LX-8's fixture letters
    /// `menu[2:spin]`, a grip below 120 and a rotation name, still parse and do not lower.
    Menu(Vec<(u32, String)>),
    Name(String),
    /// `f(a)`, `f<T>(a)`, `alias.f(a)` or a call of any callable value. Type arguments follow names only.
    Call {
        callee: Box<Expr>,
        type_args: Vec<TypeExpr>,
        args: Vec<Expr>,
    },
    Field {
        value: Box<Expr>,
        field: Name,
    },
    /// `record { a = e }`
    Record(Vec<(Name, Expr)>),
    /// `fn(x: T) -> R { ... }`
    Closure {
        params: Vec<ParamDecl>,
        result: TypeExpr,
        body: Box<Block>,
    },
    /// `match e { pattern => e; ... }`: literal arms and a final `_` arm, or constructor arms and an optional final
    /// `_` arm (T1 draft).
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<Arm>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arm {
    pub pattern: Pattern,
    pub body: Expr,
    pub loc: Loc,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pattern {
    /// `_`
    Wildcard,
    /// `ctor(a, b)` or nullary `ctor()`: a constructor of the scrutinee's sum type and one binder per payload value,
    /// a name or `_` (LG2.1; T1 draft).
    Ctor { ctor: Name, binders: Vec<Name> },
    /// A literal, such as `3` or `tag("KN7:budget.output")`.
    Lit(Expr),
}
