//! Names and their scopes in one parsed file (EV1.3, EV1.5, EV1.6): every binding with the byte interval it is
//! visible in, and every name occurrence with what it resolves to. Complete declarations are read from the parsed
//! program; a recovery hole is read from what the parser recovered of it and from its tokens (EV1.2), and its
//! occurrences are marked as such.
//!
//! Resolution follows lowering's rules: a local binding of the same declaration whose scope holds the position, then
//! the module's declarations and import aliases, then the reserved names. A binding never shadows another in one
//! declaration, so equal names in different declarations or sibling scopes resolve apart.
//!
//! Each binding is also listed under its owner, and the module's names are looked up by name, so that resolving a
//! reference reads its own owner's bindings and not every binding of the file: on LX-4's L fixture (about 5,000
//! lines) a scan of every binding per reference cost about 20 ms of each completion.

use std::collections::HashMap;

use c600_syntax::ast::{BindingKind, Block, Decl, Expr, ExprKind, Item, ParsedFile, Pattern, TypeExpr};
use c600_syntax::{
    base_types, builtins, capability_names, constructors, effect_names, indexed_operations, keywords, orders,
    precisions, type_constructors, type_indices, Lexeme, LexemeKind, Span,
};

/// How a name is bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NameKind {
    /// A module function: `fn`, `view`, `command` or `contract`; or a constructor of a sum type the module declares
    /// (T1 draft).
    Declaration,
    /// An import alias.
    Import,
    Generic,
    Parameter,
    Let,
    ClosureParameter,
    MatchBinder,
}

impl From<BindingKind> for NameKind {
    fn from(kind: BindingKind) -> NameKind {
        match kind {
            BindingKind::Generic => NameKind::Generic,
            BindingKind::Parameter => NameKind::Parameter,
            BindingKind::Let => NameKind::Let,
            BindingKind::ClosureParameter => NameKind::ClosureParameter,
            BindingKind::MatchBinder => NameKind::MatchBinder,
        }
    }
}

/// Where a binding lives: the module, a complete declaration (by its position among the declarations) or a
/// recovery hole (by its position among the holes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Owner {
    Module,
    Decl(usize),
    Hole(usize),
}

#[derive(Clone, Debug)]
pub(crate) struct Binding {
    pub name: String,
    /// The binding name's range.
    pub span: Span,
    pub kind: NameKind,
    pub owner: Owner,
    /// The inclusive byte interval the name is visible in.
    pub scope: (usize, usize),
    /// A parameter's declared type.
    pub ty: Option<TypeExpr>,
    /// A complete declaration's position among the declarations.
    pub decl: Option<usize>,
    /// An import alias's module digest.
    pub digest: Option<String>,
}

/// What an occurrence resolves to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    Binding(usize),
    /// `alias.name`: a declaration of the module the alias binding imports.
    Member {
        alias: usize,
        name: String,
    },
    Builtin,
    Order,
    Keyword,
    Type,
    Effect,
    Capability,
    Unresolved,
}

#[derive(Clone, Debug)]
pub(crate) struct Ref {
    pub span: Span,
    pub name: String,
    pub target: Target,
    pub owner: Owner,
}

/// A name position: a binding site or a reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Occurrence {
    Binding(usize),
    Ref(usize),
}

pub(crate) struct Index<'a> {
    pub file: &'a ParsedFile,
    pub decls: Vec<&'a Decl>,
    pub bindings: Vec<Binding>,
    pub refs: Vec<Ref>,
    /// The positions in `bindings` of each owner's bindings, in binding order: the module's, each complete
    /// declaration's (by its position among the declarations) and each hole's (by its position among the holes).
    module_owned: Vec<usize>,
    decl_owned: Vec<Vec<usize>>,
    hole_owned: Vec<Vec<usize>>,
    /// The first module-level binding of each name, by kind: declarations and import aliases.
    first_declaration: HashMap<String, usize>,
    first_import: HashMap<String, usize>,
}

#[derive(Clone, Copy)]
enum Namespace {
    Value,
    Type,
    Effect,
    /// A token in a hole, whose namespace the parser did not establish.
    Any,
}

impl<'a> Index<'a> {
    /// The index of a parsed file and its tokens.
    pub fn build(file: &'a ParsedFile, lexemes: &[Lexeme]) -> Index<'a> {
        let decls: Vec<&Decl> = file.declarations().collect();
        let mut index = Index {
            file,
            module_owned: Vec::new(),
            decl_owned: vec![Vec::new(); decls.len()],
            hole_owned: vec![Vec::new(); file.recovered.len()],
            first_declaration: HashMap::new(),
            first_import: HashMap::new(),
            decls,
            bindings: Vec::new(),
            refs: Vec::new(),
        };
        let all = (0, usize::MAX);
        let mut position = 0;
        for item in &file.items {
            match item {
                Item::Import(i) => index.push(Binding {
                    name: i.alias.text.clone(),
                    span: i.alias.loc.0,
                    kind: NameKind::Import,
                    owner: Owner::Module,
                    scope: all,
                    ty: None,
                    decl: None,
                    digest: Some(i.digest.clone()),
                }),
                Item::Decl(d) => {
                    index.push(Binding {
                        name: d.name.text.clone(),
                        span: d.name.loc.0,
                        kind: NameKind::Declaration,
                        owner: Owner::Module,
                        scope: all,
                        ty: None,
                        decl: Some(position),
                        digest: None,
                    });
                    position += 1;
                }
                // A sum type's constructors are module names that a call resolves to (T1 draft).
                Item::Type(t) => {
                    for c in &t.ctors {
                        index.push(Binding {
                            name: c.name.text.clone(),
                            span: c.name.loc.0,
                            kind: NameKind::Declaration,
                            owner: Owner::Module,
                            scope: all,
                            ty: None,
                            decl: None,
                            digest: None,
                        });
                    }
                }
            }
        }
        for r in &file.recovered {
            if let Some(name) = &r.name {
                index.push(Binding {
                    name: name.text.clone(),
                    span: name.loc.0,
                    kind: NameKind::Declaration,
                    owner: Owner::Module,
                    scope: all,
                    ty: None,
                    decl: None,
                    digest: None,
                });
            }
        }
        for n in 0..index.decls.len() {
            index.declaration(n);
        }
        for h in 0..file.recovered.len() {
            index.hole(h, lexemes);
        }
        index
    }

    /// Adds a binding and lists it under its owner and, at the module level, under its name.
    fn push(&mut self, binding: Binding) {
        let i = self.bindings.len();
        match binding.owner {
            Owner::Module => {
                self.module_owned.push(i);
                let first = match binding.kind {
                    NameKind::Declaration => Some(&mut self.first_declaration),
                    NameKind::Import => Some(&mut self.first_import),
                    _ => None,
                };
                if let Some(first) = first {
                    first.entry(binding.name.clone()).or_insert(i);
                }
            }
            Owner::Decl(n) => self.decl_owned[n].push(i),
            Owner::Hole(h) => self.hole_owned[h].push(i),
        }
        self.bindings.push(binding);
    }

    /// The positions of `owner`'s bindings, in binding order.
    fn owned(&self, owner: Owner) -> &[usize] {
        let list = match owner {
            Owner::Module => Some(&self.module_owned),
            Owner::Decl(n) => self.decl_owned.get(n),
            Owner::Hole(h) => self.hole_owned.get(h),
        };
        list.map_or(&[], Vec::as_slice)
    }

    fn bind(&mut self, name: &c600_syntax::ast::Name, kind: NameKind, owner: Owner, scope: (usize, usize)) {
        self.push(Binding {
            name: name.text.clone(),
            span: name.loc.0,
            kind,
            owner,
            scope,
            ty: None,
            decl: None,
            digest: None,
        });
    }

    fn declaration(&mut self, n: usize) {
        let d = self.decls[n];
        let owner = Owner::Decl(n);
        let body = (d.body.loc.0.start, d.body.loc.0.end);
        for g in &d.generics {
            self.bind(g, NameKind::Generic, owner, (d.loc.0.start, d.loc.0.end));
        }
        for p in &d.params {
            self.ty(&p.ty, owner);
            self.bind(&p.name, NameKind::Parameter, owner, body);
            if let Some(b) = self.bindings.last_mut() {
                b.ty = Some(p.ty.clone());
            }
        }
        self.ty(&d.result, owner);
        for e in &d.effects {
            self.reference(&e.text, e.loc.0, owner, Namespace::Effect);
        }
        self.block(&d.body, owner);
    }

    fn block(&mut self, b: &Block, owner: Owner) {
        for l in &b.lets {
            self.expr(&l.value, owner);
            self.bind(&l.name, NameKind::Let, owner, (l.loc.0.end, b.loc.0.end));
        }
        self.expr(&b.result, owner);
    }

    fn expr(&mut self, e: &Expr, owner: Owner) {
        match &e.kind {
            ExprKind::Name(n) => self.reference(n, e.loc.0, owner, Namespace::Value),
            ExprKind::Call { callee, type_args, args } => {
                self.expr(callee, owner);
                for t in type_args {
                    self.ty(t, owner);
                }
                for a in args {
                    self.expr(a, owner);
                }
            }
            ExprKind::Field { value, field } => {
                if let ExprKind::Name(alias) = &value.kind {
                    let target = self.resolve(alias, value.loc.0.start, owner, Namespace::Value);
                    if let Target::Binding(i) = target {
                        if self.bindings[i].kind == NameKind::Import {
                            self.refs.push(Ref { span: value.loc.0, name: alias.clone(), target, owner });
                            let member = Target::Member { alias: i, name: field.text.clone() };
                            self.refs.push(Ref { span: field.loc.0, name: field.text.clone(), target: member, owner });
                            return;
                        }
                    }
                }
                self.expr(value, owner);
            }
            ExprKind::Record(fields) => {
                for (_, v) in fields {
                    self.expr(v, owner);
                }
            }
            ExprKind::Closure { params, result, body } => {
                for p in params {
                    self.ty(&p.ty, owner);
                    self.bind(&p.name, NameKind::ClosureParameter, owner, (body.loc.0.start, body.loc.0.end));
                    if let Some(b) = self.bindings.last_mut() {
                        b.ty = Some(p.ty.clone());
                    }
                }
                self.ty(result, owner);
                self.block(body, owner);
            }
            ExprKind::Match { scrutinee, arms } => {
                self.expr(scrutinee, owner);
                for arm in arms {
                    match &arm.pattern {
                        Pattern::Ctor { binders, .. } => {
                            for binder in binders.iter().filter(|b| b.text != "_") {
                                let scope = (binder.loc.0.end, arm.body.loc.0.end);
                                self.bind(binder, NameKind::MatchBinder, owner, scope);
                            }
                        }
                        Pattern::Lit(e) => self.expr(e, owner),
                        Pattern::Wildcard => {}
                    }
                    self.expr(&arm.body, owner);
                }
            }
            _ => {}
        }
    }

    fn ty(&mut self, t: &TypeExpr, owner: Owner) {
        match t {
            TypeExpr::Named { name, args, .. } => {
                self.reference(&name.text, name.loc.0, owner, Namespace::Type);
                for a in args {
                    self.ty(a, owner);
                }
            }
            TypeExpr::Fn { params, result, effects, .. } => {
                for p in params {
                    self.ty(p, owner);
                }
                self.ty(result, owner);
                for e in effects {
                    self.reference(&e.text, e.loc.0, owner, Namespace::Effect);
                }
            }
            TypeExpr::Record { fields, .. } => {
                for (_, t) in fields {
                    self.ty(t, owner);
                }
            }
        }
    }

    /// A hole's recovered bindings and the name tokens inside it.
    fn hole(&mut self, h: usize, lexemes: &[Lexeme]) {
        let r = &self.file.recovered[h];
        let owner = Owner::Hole(h);
        for b in &r.bindings {
            self.bind(&b.name, b.kind.into(), owner, (b.from, b.to));
            if let Some(last) = self.bindings.last_mut() {
                last.ty = b.ty.clone();
            }
        }
        let sites: Vec<usize> =
            r.bindings.iter().map(|b| b.name.loc.0.start).chain(r.name.iter().map(|n| n.loc.0.start)).collect();
        let inside: Vec<&Lexeme> =
            lexemes.iter().filter(|l| l.span.start >= r.hole.start && l.span.end <= r.hole.end).collect();
        for (k, l) in inside.iter().enumerate() {
            if l.kind != LexemeKind::Name || sites.contains(&l.span.start) {
                continue;
            }
            let previous = k.checked_sub(1).map(|p| inside[p]);
            if previous.is_some_and(|p| p.text == ".") {
                let alias = k.checked_sub(2).map(|p| inside[p]).filter(|a| a.kind == LexemeKind::Name);
                if let Some(a) = alias {
                    if let Target::Binding(i) = self.resolve(&a.text, a.span.start, owner, Namespace::Value) {
                        if self.bindings[i].kind == NameKind::Import {
                            let member = Target::Member { alias: i, name: l.text.clone() };
                            self.refs.push(Ref { span: l.span, name: l.text.clone(), target: member, owner });
                        }
                    }
                }
                continue;
            }
            self.reference(&l.text, l.span, owner, Namespace::Any);
        }
    }

    fn reference(&mut self, name: &str, span: Span, owner: Owner, namespace: Namespace) {
        let target = self.resolve(name, span.start, owner, namespace);
        self.refs.push(Ref { span, name: name.to_owned(), target, owner });
    }

    /// What `name` at byte `at` of `owner` resolves to.
    fn resolve(&self, name: &str, at: usize, owner: Owner, namespace: Namespace) -> Target {
        if let Some(i) = self.local(name, at, owner) {
            let generic = self.bindings[i].kind == NameKind::Generic;
            if matches!(namespace, Namespace::Value | Namespace::Any) || generic {
                return Target::Binding(i);
            }
        }
        let value = || {
            if let Some(i) = self.module_binding(name) {
                Some(Target::Binding(i))
            } else if builtins().iter().chain(indexed_operations()).chain(constructors()).any(|b| *b == name) {
                // The tower's indexed operations and the built-in constructors are no reserved names; unbound, they
                // name the operation (T1 drafts).
                Some(Target::Builtin)
            } else if orders().contains(&name) {
                Some(Target::Order)
            } else if keywords().contains(&name) {
                Some(Target::Keyword)
            } else {
                None
            }
        };
        let ty = || {
            let known = base_types().iter().chain(type_constructors()).chain(precisions()).any(|t| *t == name)
                || type_indices().contains(&name);
            known.then_some(Target::Type)
        };
        let effect = || {
            if effect_names().contains(&name) {
                Some(Target::Effect)
            } else if capability_names().contains(&name) {
                Some(Target::Capability)
            } else {
                None
            }
        };
        let found = match namespace {
            Namespace::Value => value(),
            Namespace::Type => ty(),
            Namespace::Effect => effect(),
            Namespace::Any => value().or_else(ty).or_else(effect),
        };
        found.unwrap_or(Target::Unresolved)
    }

    /// The innermost local binding of `name` that `owner` sees at byte `at`; of equally inner ones, the last bound.
    pub fn local(&self, name: &str, at: usize, owner: Owner) -> Option<usize> {
        self.owned(owner)
            .iter()
            .copied()
            .filter(|&i| {
                let b = &self.bindings[i];
                b.name == name && b.scope.0 <= at && at <= b.scope.1
            })
            .max_by_key(|&i| self.bindings[i].scope.0)
    }

    /// The module-level binding of `name`, a declaration before an alias; of each kind, the first bound.
    pub fn module_binding(&self, name: &str) -> Option<usize> {
        self.first_declaration.get(name).or_else(|| self.first_import.get(name)).copied()
    }

    /// Who owns byte `at`: the complete declaration holding it, else the hole, else the module.
    pub fn owner_at(&self, at: usize) -> Owner {
        if let Some(n) = self.decls.iter().position(|d| d.loc.0.start <= at && at < d.loc.0.end) {
            return Owner::Decl(n);
        }
        if let Some(h) = self.file.recovered.iter().position(|r| r.hole.start <= at && at <= r.hole.end) {
            return Owner::Hole(h);
        }
        Owner::Module
    }

    /// The bindings `owner` sees at byte `at`: its locals whose scope holds the position, then the module's names.
    pub fn visible(&self, at: usize, owner: Owner) -> Vec<usize> {
        let locals = self
            .bindings
            .iter()
            .enumerate()
            .filter(|(_, b)| b.owner == owner && owner != Owner::Module && b.scope.0 <= at && at <= b.scope.1);
        let module = self.bindings.iter().enumerate().filter(|(_, b)| b.owner == Owner::Module);
        locals.chain(module).map(|(i, _)| i).collect()
    }

    /// The name occurrence that byte `at` touches: inside it or at either end. A binding site comes first.
    pub fn occurrence(&self, at: usize) -> Option<Occurrence> {
        let touches = |s: &Span| s.start <= at && at <= s.end;
        if let Some(i) = self.bindings.iter().position(|b| touches(&b.span)) {
            return Some(Occurrence::Binding(i));
        }
        self.refs.iter().position(|r| touches(&r.span)).map(Occurrence::Ref)
    }

    /// The binding an occurrence names, if it names one of this file's bindings.
    pub fn binding_of(&self, occurrence: Occurrence) -> Option<usize> {
        match occurrence {
            Occurrence::Binding(i) => Some(i),
            Occurrence::Ref(r) => match self.refs[r].target {
                Target::Binding(i) => Some(i),
                _ => None,
            },
        }
    }

    /// The references that resolve to a binding.
    pub fn references(&self, binding: usize) -> impl Iterator<Item = &Ref> {
        self.refs.iter().filter(move |r| r.target == Target::Binding(binding))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// `local` as it was before the bindings were listed by owner: a scan of every binding of the file.
    fn local_scan(index: &Index, name: &str, at: usize, owner: Owner) -> Option<usize> {
        index
            .bindings
            .iter()
            .enumerate()
            .filter(|(_, b)| b.owner == owner && b.name == name && b.scope.0 <= at && at <= b.scope.1)
            .max_by_key(|(_, b)| b.scope.0)
            .map(|(i, _)| i)
    }

    /// `module_binding` as it was before the module's names were mapped: a scan of every binding of the file.
    fn module_binding_scan(index: &Index, name: &str) -> Option<usize> {
        let module =
            |kind| index.bindings.iter().position(|b| b.owner == Owner::Module && b.kind == kind && b.name == name);
        module(NameKind::Declaration).or_else(|| module(NameKind::Import))
    }

    /// Two declarations named `f` and an import alias `f`; generics, parameters, lets, closures and match binders
    /// that reuse names, a closure whose two parameters share a name and a scope start, and a broken command, whose
    /// recovery hole has bindings and names a declaration that is also declared complete after it.
    fn source() -> String {
        let (a, b) = ("a".repeat(64), "b".repeat(64));
        format!(
            "edition \"6cs-draft-1\";\nmodule tool requires [];\nimport lib \"{a}\";\nimport f \"{b}\";\n\n\
             fn f<T>(x: Nat, t: T) -> Nat effects [] {{\n    let y = exact_add(x, 1);\n    \
             let g = fn(x: Nat, x: Nat) -> Nat {{ let y = x; return y; }};\n    \
             let k = match tag(\"x\") {{ holds(y) => g(y, x); }};\n    return k;\n}}\n\n\
             fn f(x: Nat) -> Nat effects [] {{\n    return lib.double(x);\n}}\n\n\
             export command go(w: Word, n: Nat) -> Nat effects [] {{\n    let m = f(n);\n    \
             let x = repeat(n, 0, fn(acc: Nat) -> Nat {{ return acc; }});\n    return m(\n\n\
             fn go(x: Nat) -> Nat effects [] {{\n    let x = f(x);\n    return x;\n}}\n"
        )
    }

    /// The owner lists, the name maps and therefore every lookup answer as the scans did, at every position, for
    /// every owner and every name the file binds or references, so resolution's rules and its tie-breaks are kept.
    #[test]
    fn indexed_lookups_answer_as_a_scan_of_every_binding() {
        let source = source();
        let file = c600_syntax::parse(source.as_bytes());
        let (lexemes, _) = c600_syntax::lexemes(&source);
        let index = Index::build(&file, &lexemes);
        assert_eq!(file.recovered.len(), 1, "the broken command is a recovery hole");
        assert!(!file.recovered[0].bindings.is_empty(), "the hole has bindings");
        assert_eq!(index.decls.len(), 3);

        let mut owners = vec![Owner::Module];
        owners.extend((0..index.decls.len()).map(Owner::Decl));
        owners.extend((0..file.recovered.len()).map(Owner::Hole));
        for &owner in &owners {
            let scan: Vec<usize> = (0..index.bindings.len()).filter(|&i| index.bindings[i].owner == owner).collect();
            assert_eq!(index.owned(owner), scan.as_slice(), "{owner:?}");
        }
        assert!(index.owned(Owner::Decl(index.decls.len())).is_empty());
        assert!(index.owned(Owner::Hole(file.recovered.len())).is_empty());

        let mut names: BTreeSet<&str> = index.bindings.iter().map(|b| b.name.as_str()).collect();
        names.extend(index.refs.iter().map(|r| r.name.as_str()));
        names.extend(["exact_add", "Nat", "nothing_binds_this"]);
        owners.extend([Owner::Decl(index.decls.len()), Owner::Hole(file.recovered.len())]);
        for name in &names {
            assert_eq!(index.module_binding(name), module_binding_scan(&index, name), "{name}");
            for &owner in &owners {
                for at in 0..=source.len() + 1 {
                    let (found, scan) = (index.local(name, at, owner), local_scan(&index, name, at, owner));
                    assert_eq!(found, scan, "{name} at {at} in {owner:?}");
                }
            }
        }

        // The cases whose order rules matter are present: a declaration before an alias of the same name, the first
        // of two declarations, and two parameters with the same scope, of which the later one is found.
        let f = index.module_binding("f").unwrap();
        assert_eq!((index.bindings[f].kind, index.bindings[f].decl), (NameKind::Declaration, Some(0)));
        let go = index.module_binding("go").unwrap();
        assert_eq!(index.bindings[go].decl, Some(2), "a complete declaration comes before a hole's name");
        let tied: Vec<usize> = index
            .owned(Owner::Decl(0))
            .iter()
            .copied()
            .filter(|&i| index.bindings[i].kind == NameKind::ClosureParameter && index.bindings[i].name == "x")
            .collect();
        assert_eq!(tied.len(), 2);
        assert_eq!(index.bindings[tied[0]].scope, index.bindings[tied[1]].scope);
        let inside = index.bindings[tied[0]].scope.0 + 1;
        assert_eq!(index.local("x", inside, Owner::Decl(0)), Some(tied[1]));
    }
}
