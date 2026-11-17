//! The parsed program. Every node keeps its source range in a [`Loc`], which equality ignores, so two parses compare
//! equal when they hold the same program wherever its text sits (LG1.4).

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
            Item::Import(_) => None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Name {
    pub text: String,
    pub loc: Loc,
}

/// `module name requires [capabilities];`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleHeader {
    pub name: Name,
    pub requires: Vec<Name>,
    pub loc: Loc,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    Import(Import),
    Decl(Box<Decl>),
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
    /// `Name` or `Name<A,B>`, including `Display<T>` and `Approx<P53>`.
    Named { name: Name, args: Vec<TypeExpr>, loc: Loc },
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
    /// `match e { pattern => e; ... }`
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
    /// `tag(binder)`: LX-8's verdict arms.
    Binder { tag: Name, binder: Name },
    /// A literal, such as `3` or `tag("KN7:budget.output")`.
    Lit(Expr),
}
