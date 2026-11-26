//! The stage checker (RPI.68 to RPI.71, RPI.73, RPI.75; LG2.24, LG3.17, LG5.12; LP2 draft until the IR freeze).
//!
//! A stage module is checked here alone; the ordinary checker does not walk its bodies. [`crate::check`] applies the
//! structural rules, which need no limit, to every stage module of a program, in every profile alike. [`admit_stage`]
//! adds the rules that need the host's [`StageLimits`] (the enabled tiers, RM.38's weights and the fixed checker
//! bounds) and returns the only [`CheckedStage`] an adapter may take. Every refusal is
//! `LG2:type.stage_rule_violation`, apart from a base-type mismatch between two types a stage may hold, which is
//! `LG2:type.mismatch` (LG2.1).
//!
//! The structural rules:
//!
//! - manifest: the module declares no capability; each declared array is named once, has a length of at least 1 and
//!   names a parameter of type `List<Approx<24>>` of an exported function;
//! - every function is a plain `fn` with no effect in its annotation; its parameters and result are binary32 scalars
//!   (`Approx<24>`), `ApproxBool`, `Unit` or records of those, and only an exported function takes the declared
//!   arrays;
//! - a body uses literals of precision 24 that denote exact finite binary32 values, names, `let`, `display_if`, records and their
//!   fields, calls of functions of stage modules of the same kind, `repeat` with a literal count, `index` of a
//!   declared array at a literal position below its length, and the operation table: `approx_add`, `approx_less`,
//!   `approx_and`, `approx_not` and the [`StageOp`]s, `exclude` only in a vertex stage (RPI.75);
//! - nothing else: no exact number outside a loop count or an index, no `if` or `match` (ordinary control takes exact
//!   inputs), no closure, function value, collection or unresolved call, no clock (frame time is an entry parameter),
//!   and no effect, engine call, file, randomness, budget or receipt (RPI.69, LG3.17);
//! - no recursion: a call whose callee reaches the caller is refused at the call (calls are inlined).
//!
//! A stage module may export several functions, so a module of shared stage helpers is a stage module too. The
//! adapter names the entry: [`admit_stage`] refuses an entry that is not exported, takes not every declared array or
//! does not return its kind's position (`record { x, y, z }` for a vertex stage, `record { r, g, b }` for a fragment
//! stage, each of binary32).
//!
//! What the checker does not do: the raster bound (RPI.72), the CPU reference evaluator (RPI.74), the ID pass and the
//! host statuses (RPI.75, RPI.76), quarantine and compilation (RPI.77, RPI.78) belong to the adapter and the host.
//! Evaluation is total by the evaluator's coverage codes, not by a static rule: a division by zero or an overflow
//! gives `nonfinite`, and `exclude` gives `singular_neighbourhood` (LX-7's subset).

use std::collections::{BTreeMap, BTreeSet};

use c600_ir::{
    Callee, Expr, FnKind, Function, Lit, Module, ModuleKind, Prim, Program, QTy, StageDecl, StageKind, StageOp, Ty,
    EXEC_CONTRACT, IR_VERSION,
};

use crate::{check, path_text, type_text, Diagnostic, Found, IrError, Mismatch, IR_VERSION_UNSUPPORTED, TYPE_MISMATCH};

/// `LG2:type.stage_rule_violation` (LG2.24; RPI.69 to RPI.71).
pub const STAGE_RULE_VIOLATION: &str = "LG2:type.stage_rule_violation";

/// A type a stage may hold.
#[derive(Clone, Debug, PartialEq, Eq)]
enum STy {
    F32,
    Bool,
    Unit,
    /// Fields sorted by name.
    Record(Vec<(String, STy)>),
    /// A declared array input of this length.
    Array(u64),
}

impl STy {
    fn of(q: &QTy) -> Option<STy> {
        match &q.ty {
            Ty::Approx(24) => Some(STy::F32),
            Ty::ApproxBool => Some(STy::Bool),
            Ty::Unit => Some(STy::Unit),
            Ty::Record(fields) => {
                let mut out =
                    fields.iter().map(|(n, f)| STy::of(f).map(|t| (n.clone(), t))).collect::<Option<Vec<_>>>()?;
                out.sort_by(|a, b| a.0.cmp(&b.0));
                Some(STy::Record(out))
            }
            _ => None,
        }
    }

    fn text(&self) -> String {
        match self {
            STy::F32 => "Approx<P24>".into(),
            STy::Bool => "ApproxBool".into(),
            STy::Unit => "Unit".into(),
            STy::Record(fields) if fields.is_empty() => "record {}".into(),
            STy::Record(fields) => {
                let fields: Vec<String> = fields.iter().map(|(n, t)| format!("{n}: {}", t.text())).collect();
                format!("record {{ {} }}", fields.join(", "))
            }
            STy::Array(_) => "List<Approx<P24>>".into(),
        }
    }

    fn position(kind: StageKind) -> STy {
        let names: [&str; 3] = match kind {
            StageKind::Vertex => ["x", "y", "z"],
            StageKind::Fragment => ["b", "g", "r"],
        };
        STy::Record(names.iter().map(|n| (n.to_string(), STy::F32)).collect())
    }
}

/// The binary32 value an approximate literal denotes exactly, if it denotes one (LX-7's subset: a literal is checked
/// against its decimal rational, with no implicit rounding). The decimal is LG1.7's form (`is_decimal` in
/// `c600-syntax`): an optional sign, digits, an optional fraction and an optional exponent.
fn exact_binary32(decimal: &str) -> Option<f32> {
    // Rust's parse rounds correctly to nearest, ties to even; an overflow gives an infinity.
    let v: f32 = decimal.parse().ok()?;
    // A finite binary32 value has at most 149 fraction digits, so this expansion is exact.
    (v.is_finite() && normal_form(&format!("{:.150}", f64::from(v)))? == normal_form(decimal)?).then_some(v)
}

/// A decimal as its sign, its significant digits and the power of ten of the last one; zero is one form whatever
/// its sign.
fn normal_form(decimal: &str) -> Option<(bool, String, i64)> {
    let negative = decimal.starts_with('-');
    let s = decimal.strip_prefix(['-', '+']).unwrap_or(decimal);
    let (mantissa, exponent) = match s.find(['e', 'E']) {
        Some(i) => (&s[..i], s[i + 1..].parse::<i64>().ok()?),
        None => (s, 0),
    };
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if int.is_empty() || !int.bytes().chain(frac.bytes()).all(|b| b.is_ascii_digit()) {
        return None;
    }
    let digits = format!("{int}{frac}");
    let digits = digits.trim_start_matches('0');
    let trimmed = digits.trim_end_matches('0');
    if trimmed.is_empty() {
        return Some((false, String::new(), 0));
    }
    let power = exponent.checked_sub(i64::try_from(frac.len()).ok()?)?;
    let power = power.checked_add(i64::try_from(digits.len() - trimmed.len()).ok()?)?;
    Some((negative, trimmed.to_string(), power))
}

/// A positive exact binary32 literal (an `exclude` radius).
fn positive_literal(e: &Expr) -> bool {
    match e {
        Expr::Lit(Lit::Approx { decimal, precision: 24 }) => exact_binary32(decimal).is_some_and(|v| v > 0.0),
        _ => false,
    }
}

/// Whether `p` is the declared array input `name`: a parameter of that name of type `List<Approx<24>>`.
fn array_param(p: &c600_ir::Param, name: &str) -> bool {
    p.name == name && matches!(&p.ty.ty, Ty::List(e) if e.ty == Ty::Approx(24))
}

fn literal_nat(e: &Expr) -> Option<u64> {
    match e {
        Expr::Lit(Lit::Nat(n)) => Some(n.parse::<u64>().unwrap_or(u64::MAX)),
        _ => None,
    }
}

/// What an expression outside the stage's language is, for the refusal's note.
fn outside(e: &Expr) -> Option<&'static str> {
    Some(match e {
        Expr::If { .. } => "an ordinary `if` takes an exact condition; a stage branches with `display_if` (LG2.20)",
        Expr::Match { .. } => "a `match` takes an exact scrutinee; a stage branches with `display_if` (LG2.20)",
        Expr::Closure { .. } => "a closure allocates and is a callable",
        Expr::FnRef { .. } => "a function value is a callable; calls are inlined",
        Expr::Collection { .. } => "a collection allocates",
        Expr::ReadSnapshot { .. } | Expr::ReadSnapshotWith { .. } => "a snapshot read is an engine call",
        Expr::PublishView { .. } => "a view change is an effect",
        Expr::WriteWorkspace { .. } => "a workspace write is an effect",
        Expr::Propose { .. } => "a preview is an engine call",
        Expr::ReadReceipt { .. } => "a preview receipt comes from an engine call",
        Expr::UseExact { .. } => "an exact goal, budget or evidence sink is exact data",
        Expr::Accounting(_) => "display accounting reads the run's budgets",
        Expr::Declassified { .. } => "a declassified read is state-derived data",
        Expr::FileRead { .. } | Expr::FileWrite { .. } => "a file is no stage input",
        Expr::RandomDraw | Expr::RngInit { .. } => "randomness is an effect",
        Expr::TextDecodeHex { .. } => "a decoding allocates a text",
        Expr::ReadClock => "frame time is a declared host input, an entry parameter, never an ambient clock read",
        Expr::Child { .. } => "a child scope is a budget scope of a run",
        Expr::Construct { .. } | Expr::Case { .. } => "a sum type's value is no stage value (T1 draft)",
        Expr::Kernel { .. } => "a kernel row is an engine call",
        _ => return None,
    })
}

/// The structural stage rules over every stage module of `program`, as findings keyed in IR order.
pub(crate) fn structural(program: &Program) -> Result<Vec<Found>, IrError> {
    let mut c = Walker::new(program);
    for mi in 0..program.modules.len() {
        if program.modules[mi].stage().is_some() {
            c.module(mi)?;
        }
    }
    c.recursion();
    Ok(c.found)
}

struct Walker<'p> {
    program: &'p Program,
    modules: BTreeMap<&'p str, usize>,
    found: Vec<Found>,
    /// Calls between stage functions: caller, callee and the call's path in the caller.
    edges: Vec<((usize, usize), (usize, usize), Vec<u32>)>,
    cur: (usize, usize),
    path: Vec<u32>,
    env: Vec<(String, Option<STy>)>,
}

impl<'p> Walker<'p> {
    fn new(program: &'p Program) -> Self {
        let modules = program.modules.iter().enumerate().map(|(i, m)| (m.name.as_str(), i)).collect();
        Walker {
            program,
            modules,
            found: Vec::new(),
            edges: Vec::new(),
            cur: (0, 0),
            path: Vec::new(),
            env: Vec::new(),
        }
    }

    fn decl(&self) -> &'p StageDecl {
        let program = self.program;
        program.modules[self.cur.0].stage().expect("a stage module")
    }

    fn diagnostic(&self, code: &'static str, function: bool, path: String, note: String) -> Diagnostic {
        let program = self.program;
        let module = &program.modules[self.cur.0];
        Diagnostic {
            code,
            module: module.name.clone(),
            function: if function { module.functions[self.cur.1].name.clone() } else { String::new() },
            path,
            value_type: None,
            display_control: false,
            qualifier_erasure: false,
            mismatch: None,
            note,
        }
    }

    fn manifest(&mut self, note: String) {
        let d = self.diagnostic(STAGE_RULE_VIOLATION, false, "manifest".into(), note);
        self.found.push(Found { key: (self.cur.0, None, Vec::new()), diagnostic: d });
    }

    /// A finding at the current path extended by `at`.
    fn refuse_at(&mut self, code: &'static str, at: &[u32], note: String, mismatch: Option<Mismatch>) {
        let mut path = self.path.clone();
        path.extend_from_slice(at);
        let mut d = self.diagnostic(code, true, path_text(&path), note);
        d.mismatch = mismatch;
        self.found.push(Found { key: (self.cur.0, Some(self.cur.1), path), diagnostic: d });
    }

    fn refuse(&mut self, note: String) {
        self.refuse_at(STAGE_RULE_VIOLATION, &[], format!("{note} (RPI.69)"), None);
    }

    fn mismatch_at(&mut self, at: &[u32], expected: &STy, supplied: &STy, what: &str) {
        let (expected, supplied) = (expected.text(), supplied.text());
        let note = format!("{what}: a {supplied} where a {expected} is expected (LG2.1)");
        self.refuse_at(TYPE_MISMATCH, at, note, Some(Mismatch { expected, supplied }));
    }

    fn module(&mut self, mi: usize) -> Result<(), IrError> {
        let program = self.program;
        let module: &'p Module = &program.modules[mi];
        self.cur = (mi, 0);
        let decl = self.decl();
        if !module.capabilities.is_empty() {
            let caps: Vec<String> = module.capabilities.iter().map(|c| format!("{c:?}")).collect();
            self.manifest(format!(
                "a stage has only the `pure` effect and declares no capability; it requests {} (LG3.17, RPI.69)",
                caps.join(", ")
            ));
        }
        let mut seen = BTreeSet::new();
        for (name, len) in &decl.arrays {
            if !seen.insert(name.as_str()) {
                self.manifest(format!("the array input `{name}` is declared twice"));
            }
            if *len == 0 {
                self.manifest(format!("the array input `{name}` has length 0; a declared array holds a value"));
            }
            let exported = module.functions.iter().filter(|f| f.exported);
            if !exported.flat_map(|f| &f.params).any(|p| array_param(p, name)) {
                self.manifest(format!(
                    "the array input `{name}` names no parameter of type `List<Approx<24>>` of an exported function \
                     (RPI.69)"
                ));
            }
        }
        for fi in 0..module.functions.len() {
            self.function(mi, fi)?;
        }
        Ok(())
    }

    fn function(&mut self, mi: usize, fi: usize) -> Result<(), IrError> {
        let program = self.program;
        let f: &'p Function = &program.modules[mi].functions[fi];
        self.cur = (mi, fi);
        self.path.clear();
        self.env.clear();
        let decl = self.decl();
        if f.kind != FnKind::Helper {
            self.refuse(format!("a stage function is a plain `fn`, and `{}` is a {:?}", f.name, f.kind));
        }
        if let Some(bound) = &f.effects {
            if !bound.effects.is_empty() || !bound.capabilities.is_empty() || !bound.vars.is_empty() {
                self.refuse(format!(
                    "`{}` declares an effect, and a stage has only the `pure` effect (LG3.17)",
                    f.name
                ));
            }
        }
        for p in &f.params {
            // A declared array is a parameter of an exported function, which the host supplies; no call passes one.
            let array = decl.arrays.iter().find(|(n, _)| f.exported && array_param(p, n));
            let ty = match array {
                Some((_, len)) => Some(STy::Array(*len)),
                None => STy::of(&p.ty),
            };
            if ty.is_none() {
                self.refuse(format!(
                    "the parameter `{}` has type {}; a stage computes in binary32 only, and an array is a declared \
                     input of an exported function",
                    p.name,
                    type_text(&p.ty.ty)
                ));
            }
            self.env.push((p.name.clone(), ty));
        }
        let result = STy::of(&f.result);
        if result.is_none() {
            self.refuse(format!("the result has type {}; a stage computes in binary32 only", type_text(&f.result.ty)));
        }
        let body = self.expr(&f.body)?;
        if let (Some(r), Some(b)) = (&result, &body) {
            if r != b {
                self.mismatch_at(&[], r, b, "the result");
            }
        }
        Ok(())
    }

    /// Walk a child at index `i` of the current expression.
    fn at(&mut self, i: u32, e: &Expr) -> Result<Option<STy>, IrError> {
        self.path.push(i);
        let out = self.expr(e);
        self.path.pop();
        out
    }

    fn err(&self, message: String) -> IrError {
        let program = self.program;
        let (mi, fi) = self.cur;
        let m = &program.modules[mi];
        IrError(format!("{}.{} at {}: {message}", m.name, m.functions[fi].name, path_text(&self.path)))
    }

    fn expect(&mut self, i: u32, e: &Expr, want: &STy, what: &str) -> Result<(), IrError> {
        if let Some(t) = self.at(i, e)? {
            if &t != want {
                self.mismatch_at(&[i], want, &t, what);
            }
        }
        Ok(())
    }

    fn expr(&mut self, e: &Expr) -> Result<Option<STy>, IrError> {
        if let Some(why) = outside(e) {
            self.refuse(format!("{why}; a stage has only the `pure` effect, with no allocation, callable, engine call, file or renderer handle"));
            return Ok(None);
        }
        Ok(match e {
            Expr::Lit(lit) => match lit {
                Lit::Approx { decimal, precision: 24 } => {
                    if exact_binary32(decimal).is_none() {
                        self.refuse(format!(
                            "the literal {decimal} denotes no exact finite binary32 value; a stage literal is checked                              against its decimal rational, with no implicit rounding (LX-7)"
                        ));
                    }
                    Some(STy::F32)
                }
                Lit::Approx { precision, .. } => {
                    self.refuse(format!("a literal of precision {precision}: a stage computes in binary32 only"));
                    None
                }
                Lit::Unit => Some(STy::Unit),
                Lit::Nat(_) => {
                    self.refuse("an exact natural is only a loop count or an array position: no big integers".into());
                    None
                }
                other => {
                    self.refuse(format!("the literal {other:?} is no binary32 value"));
                    None
                }
            },
            Expr::Var(name) => {
                let found = self.env.iter().rev().find(|(n, _)| n == name).map(|(_, t)| t.clone());
                match found {
                    None => return Err(self.err(format!("unbound name `{name}`"))),
                    Some(Some(STy::Array(_))) => {
                        self.refuse(format!("the array input `{name}` is read only by `index` at a literal position"));
                        None
                    }
                    Some(t) => t,
                }
            }
            Expr::Let { name, value, body } => {
                let v = self.at(0, value)?;
                self.env.push((name.clone(), v));
                let b = self.at(1, body);
                self.env.pop();
                b?
            }
            Expr::DisplayIf { cond, then, els } => {
                self.expect(0, cond, &STy::Bool, "a `display_if` condition")?;
                let t = self.at(1, then)?;
                let f = self.at(2, els)?;
                match (t, f) {
                    (Some(t), Some(f)) if t != f => {
                        self.mismatch_at(&[2], &t, &f, "the other branch of a `display_if`");
                        None
                    }
                    (Some(t), Some(_)) => Some(t),
                    _ => None,
                }
            }
            Expr::Call { callee, args } => self.call(callee, args)?,
            Expr::Prim { op, args } => self.prim(*op, args)?,
            Expr::Record(fields) => {
                let mut out = Vec::new();
                let mut known = true;
                for (i, (name, f)) in fields.iter().enumerate() {
                    match self.at(i as u32, f)? {
                        Some(t) => out.push((name.clone(), t)),
                        None => known = false,
                    }
                }
                out.sort_by(|a, b| a.0.cmp(&b.0));
                known.then_some(STy::Record(out))
            }
            Expr::Field { value, field } => match self.at(0, value)? {
                Some(STy::Record(fields)) if fields.iter().any(|(n, _)| n == field) => {
                    fields.into_iter().find(|(n, _)| n == field).map(|(_, t)| t)
                }
                Some(other) => {
                    let want = format!("record {{ {field}: _, .. }}");
                    let note = format!("a field `{field}` of a {}: no such field (LG2.1)", other.text());
                    let mismatch = Mismatch { expected: want, supplied: other.text() };
                    self.refuse_at(TYPE_MISMATCH, &[0], note, Some(mismatch));
                    None
                }
                None => None,
            },
            Expr::Repeat { count, init, acc, result, body } => {
                if literal_nat(count).is_none() {
                    self.refuse_at(
                        STAGE_RULE_VIOLATION,
                        &[0],
                        "a loop's count is a literal natural: no loop without a static bound (RPI.69)".into(),
                        None,
                    );
                }
                let r = STy::of(result);
                if r.is_none() {
                    self.refuse(format!("a loop's value has type {}: binary32 only", type_text(&result.ty)));
                }
                if let (Some(i), Some(r)) = (self.at(1, init)?, &r) {
                    if &i != r {
                        self.mismatch_at(&[1], r, &i, "a loop's initial value");
                    }
                }
                self.env.push((acc.clone(), r.clone()));
                let b = self.at(2, body);
                self.env.pop();
                if let (Some(b), Some(r)) = (b?, &r) {
                    if &b != r {
                        self.mismatch_at(&[2], r, &b, "a loop's body");
                    }
                }
                r
            }
            _ => unreachable!("every other expression is outside the stage language"),
        })
    }

    fn call(&mut self, callee: &Callee, args: &[Expr]) -> Result<Option<STy>, IrError> {
        let (module, name) = match callee {
            Callee::Fn { module, name } => (module, name),
            Callee::Value(_) => {
                self.refuse("a call of a function value: a stage holds no callable, and calls are inlined".into());
                return Ok(None);
            }
            Callee::Unresolved { name, .. } => {
                self.refuse(format!("the unresolved call `{name}` is not pure (LG3.2, LG3.17)"));
                return Ok(None);
            }
        };
        let program = self.program;
        let mj = match module {
            None => self.cur.0,
            Some(m) => *self.modules.get(m.as_str()).ok_or_else(|| self.err(format!("unknown module `{m}`")))?,
        };
        let target = &program.modules[mj];
        let fj = target
            .functions
            .iter()
            .position(|f| &f.name == name)
            .ok_or_else(|| self.err(format!("unknown function `{name}`")))?;
        let f = &target.functions[fj];
        if mj != self.cur.0 && !f.exported {
            return Err(self.err(format!("`{}.{name}` is not exported", target.name)));
        }
        if args.len() != f.params.len() {
            return Err(self.err(format!("`{name}` takes {} arguments, not {}", f.params.len(), args.len())));
        }
        let kind = self.decl().kind;
        match target.stage() {
            None => {
                self.refuse(format!(
                    "`{}.{name}` is no stage function: a stage calls only functions of stage modules, which are \
                     checked alike and inlined, so no effect reaches it through a callee (LG3.17)",
                    target.name
                ));
                return Ok(None);
            }
            Some(d) if d.kind != kind => {
                self.refuse(format!(
                    "`{}.{name}` belongs to a {} stage, and this is a {} stage",
                    target.name,
                    d.kind.name(),
                    kind.name()
                ));
                return Ok(None);
            }
            Some(_) => {}
        }
        for (i, (a, p)) in args.iter().zip(&f.params).enumerate() {
            match STy::of(&p.ty) {
                Some(want) => self.expect(i as u32, a, &want, &format!("the argument `{}`", p.name))?,
                None => {
                    // The callee's parameter is refused at the callee; an array is passed by no call.
                    self.at(i as u32, a)?;
                }
            }
        }
        self.edges.push((self.cur, (mj, fj), self.path.clone()));
        Ok(STy::of(&f.result))
    }

    fn prim(&mut self, op: Prim, args: &[Expr]) -> Result<Option<STy>, IrError> {
        let arity = match op {
            Prim::ApproxAdd | Prim::ApproxLess | Prim::ApproxAnd | Prim::Index => 2,
            Prim::ApproxNot => 1,
            Prim::Stage(s) => s.arity(),
            other => {
                self.refuse(format!(
                    "`{other:?}` is outside the stage's operation table: binary32 only, with no big integers, \
                     allocation or effect (RPI.70)"
                ));
                return Ok(None);
            }
        };
        if args.len() != arity {
            return Err(self.err(format!("{op:?} takes {arity} operands, not {}", args.len())));
        }
        let what = format!("an operand of {op:?}");
        Ok(match op {
            Prim::ApproxAdd => {
                self.expect(0, &args[0], &STy::F32, &what)?;
                self.expect(1, &args[1], &STy::F32, &what)?;
                Some(STy::F32)
            }
            Prim::ApproxLess => {
                self.expect(0, &args[0], &STy::F32, &what)?;
                self.expect(1, &args[1], &STy::F32, &what)?;
                Some(STy::Bool)
            }
            Prim::ApproxAnd | Prim::ApproxNot => {
                for (i, a) in args.iter().enumerate() {
                    self.expect(i as u32, a, &STy::Bool, &what)?;
                }
                Some(STy::Bool)
            }
            Prim::Index => {
                let len = match &args[0] {
                    Expr::Var(name) => match self.env.iter().rev().find(|(n, _)| n == name) {
                        Some((_, Some(STy::Array(len)))) => Some(*len),
                        Some(_) => None,
                        None => return Err(self.err(format!("unbound name `{name}`"))),
                    },
                    _ => None,
                };
                let Some(len) = len else {
                    let note = "an index reads a declared array input, named directly (RPI.69)";
                    self.refuse_at(STAGE_RULE_VIOLATION, &[0], note.into(), None);
                    return Ok(None);
                };
                match literal_nat(&args[1]) {
                    Some(i) if i < len => {}
                    Some(i) => {
                        let note = format!("the index {i} is not below the declared length {len}: an index is checked statically (RPI.69)");
                        self.refuse_at(STAGE_RULE_VIOLATION, &[1], note, None);
                    }
                    None => {
                        let note =
                            "an index is a literal position, checked statically against the declared length (RPI.69)";
                        self.refuse_at(STAGE_RULE_VIOLATION, &[1], note.into(), None);
                    }
                }
                Some(STy::F32)
            }
            Prim::Stage(StageOp::Exclude) => {
                if self.decl().kind != StageKind::Vertex {
                    self.refuse(
                        "`exclude` changes coverage, and a fragment stage has no discard or coverage change (RPI.75)"
                            .into(),
                    );
                    return Ok(None);
                }
                self.expect(0, &args[0], &STy::F32, &what)?;
                if !positive_literal(&args[1]) {
                    let note = "an exclusion radius is a positive exact binary32 literal (RPI.69, LX-7)";
                    self.refuse_at(STAGE_RULE_VIOLATION, &[1], note.into(), None);
                }
                Some(STy::Unit)
            }
            Prim::Stage(_) => {
                for (i, a) in args.iter().enumerate() {
                    self.expect(i as u32, a, &STy::F32, &what)?;
                }
                Some(STy::F32)
            }
            _ => unreachable!("refused above"),
        })
    }

    /// Refuse every call whose callee reaches its caller: a stage has no recursion (calls are inlined, RPI.69).
    fn recursion(&mut self) {
        let mut next: BTreeMap<(usize, usize), BTreeSet<(usize, usize)>> = BTreeMap::new();
        for (from, to, _) in &self.edges {
            next.entry(*from).or_default().insert(*to);
        }
        let reaches = |start: (usize, usize), goal: (usize, usize)| {
            let mut seen = BTreeSet::new();
            let mut todo = vec![start];
            while let Some(n) = todo.pop() {
                if n == goal {
                    return true;
                }
                if seen.insert(n) {
                    todo.extend(next.get(&n).into_iter().flatten().copied());
                }
            }
            false
        };
        let cyclic: Vec<_> = self.edges.iter().filter(|(from, to, _)| reaches(*to, *from)).cloned().collect();
        for (from, to, path) in cyclic {
            self.cur = from;
            self.path = path;
            let callee = &self.program.modules[to.0].functions[to.1].name;
            self.refuse(format!("the call of `{callee}` reaches its caller again: a stage has no recursion"));
        }
        self.path.clear();
    }
}

/// The host's limits for admitting a stage (RPI.70, RPI.71, RM.38). None of their values is set yet: the error rules
/// that enable a tier are open (RPI.36 [O]), and RM.38's weights and the fixed checker bounds come from measurement
/// (RPI.79). So the host supplies them, and the checker invents none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StageLimits {
    /// The highest tier of RPI.70 whose error rule is stated; 0 enables none. `approx_add`, `approx_less`,
    /// `approx_and`, `approx_not`, `display_if`, `index` and `exclude` are tier 1.
    pub tier: u8,
    /// RM.38's weights relative to `+`, by operation name: `approx_add`, `approx_less`, `approx_and`, `approx_not`,
    /// the [`StageOp`] names, `display_if` and `index`. An operation without a weight is refused.
    pub weights: BTreeMap<String, u64>,
    /// The fixed checker bound on a declared `stage_ops` or `fragment_ops`.
    pub max_ops: u64,
    /// The fixed checker bound on a declared `overdraw`.
    pub max_overdraw: u64,
    /// The fixed checker bound on a loop's count.
    pub max_loop_count: u64,
    /// The fixed checker bound on loop nesting in the expanded chain.
    pub max_loop_depth: u32,
}

/// A stage that passed the stage checker under a host's limits: the only form in which a stage reaches the adapter
/// (RPI.68: a checked definition, compiled by the host, never a callable or shader text from a program). It has no
/// public constructor; [`admit_stage`] makes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedStage {
    module: String,
    entry: String,
    decl: StageDecl,
    counted_ops: u64,
    definition: Vec<Module>,
}

impl CheckedStage {
    pub fn module(&self) -> &str {
        &self.module
    }
    pub fn entry(&self) -> &str {
        &self.entry
    }
    pub fn kind(&self) -> StageKind {
        self.decl.kind
    }
    /// The declared costs, which admission charges (RPI.71).
    pub fn declaration(&self) -> &StageDecl {
        &self.decl
    }
    /// The weighted operations the checker counted for one evaluation of the expanded chain (RM.38).
    pub fn counted_ops(&self) -> u64 {
        self.counted_ops
    }
    /// The checked definition: the stage module, then each stage module it calls, in program order.
    pub fn definition(&self) -> &[Module] {
        &self.definition
    }
}

/// The stage checker's answer for one stage entry under the host's limits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageAdmission {
    /// The structural findings of the stage modules the entry reaches, in IR order, then the refusals at the entry,
    /// in the bodies and at the manifest; or the program's version refusal.
    pub diagnostics: Vec<Diagnostic>,
    /// Present exactly when there is no diagnostic.
    pub stage: Option<CheckedStage>,
}

impl StageAdmission {
    pub fn codes(&self) -> Vec<&'static str> {
        self.diagnostics.iter().map(|d| d.code).collect()
    }
}

/// Admit the function `entry` of the stage module `module` under the host's limits (RPI.68 to RPI.71). The answer
/// does not depend on the profile: a stage is the same checked definition under each of the three (LG2.24, RPI.68).
/// An unknown module or function, or a module that is no stage, is an [`IrError`].
///
/// It keeps the structural findings of the stage modules the entry reaches and refuses, with
/// `LG2:type.stage_rule_violation`:
///
/// - at `entry`: an entry that is not exported, whose result is not its kind's (`record { x, y, z }` of binary32 for
///   a vertex stage, `record { r, g, b }` for a fragment stage), or that does not take every declared array input;
/// - in a body: an operation of a tier above `limits.tier` or without a weight, a loop whose count is above
///   `limits.max_loop_count` or that is nested deeper than `limits.max_loop_depth` in the expanded chain (calls
///   inlined);
/// - at `manifest`: a declared `ops` above `limits.max_ops` or below the weighted count, and a declared `overdraw`
///   above `limits.max_overdraw`.
///
/// The count is RM.38's: the sum of weights over the expanded chain, a loop at its count, a `display_if` at its
/// condition, its own weight and its dearer branch. The body and cost rules apply only to a stage with no structural
/// finding, whose bodies the counter can walk.
pub fn admit_stage(
    program: &Program,
    module: &str,
    entry: &str,
    limits: &StageLimits,
) -> Result<StageAdmission, IrError> {
    let report = check(program)?;
    if program.ir_version != IR_VERSION || program.exec_contract != EXEC_CONTRACT {
        let diagnostics = report.diagnostics.into_iter().filter(|d| d.code == IR_VERSION_UNSUPPORTED).collect();
        return Ok(StageAdmission { diagnostics, stage: None });
    }
    let mi = program
        .modules
        .iter()
        .position(|m| m.name == module)
        .ok_or_else(|| IrError(format!("no module `{module}`")))?;
    let decl = program.modules[mi].stage().ok_or_else(|| IrError(format!("`{module}` is no stage module")))?;
    let fi = program.modules[mi]
        .functions
        .iter()
        .position(|f| f.name == entry)
        .ok_or_else(|| IrError(format!("no function `{module}.{entry}`")))?;
    let f = &program.modules[mi].functions[fi];
    let mut modules = BTreeSet::from([mi]);
    reached(program, (mi, fi), &mut modules);
    let names: BTreeSet<&str> = modules.iter().map(|&m| program.modules[m].name.as_str()).collect();
    let mut diagnostics: Vec<Diagnostic> =
        report.diagnostics.iter().filter(|d| names.contains(d.module.as_str())).cloned().collect();
    let structural = diagnostics.is_empty();
    let refusal = |function: &str, path: &str, note: String| Diagnostic {
        code: STAGE_RULE_VIOLATION,
        module: module.to_string(),
        function: function.to_string(),
        path: path.into(),
        value_type: None,
        display_control: false,
        qualifier_erasure: false,
        mismatch: None,
        note,
    };
    if !f.exported {
        diagnostics.push(refusal(
            entry,
            "entry",
            "the adapter takes an exported function as a stage's entry (RPI.68)".into(),
        ));
    }
    let want = STy::position(decl.kind);
    if STy::of(&f.result).is_some_and(|r| r != want) {
        let note =
            format!("a {} stage's entry returns {}, not {}", decl.kind.name(), want.text(), type_text(&f.result.ty));
        diagnostics.push(refusal(entry, "entry", note));
    }
    for (name, _) in &decl.arrays {
        if !f.params.iter().any(|p| array_param(p, name)) {
            let note =
                format!("the entry takes no declared array input `{name}`, which the host supplies to it (RPI.69)");
            diagnostics.push(refusal(entry, "entry", note));
        }
    }
    let mut counted = 0;
    if structural {
        let mut counter = Counter {
            program,
            limits,
            memo: BTreeMap::new(),
            found: Vec::new(),
            refused: BTreeSet::new(),
            depth: 0,
            cur: (mi, fi),
            path: Vec::new(),
        };
        counted = counter.function(mi, fi);
        counter.found.sort_by(|a, b| a.key.cmp(&b.key));
        diagnostics.extend(counter.found.into_iter().map(|f| f.diagnostic));
        let ops = match decl.kind {
            StageKind::Vertex => "stage_ops",
            StageKind::Fragment => "fragment_ops",
        };
        if decl.ops > limits.max_ops {
            let note =
                format!("the declared {ops} {} is over the checker's bound {} (RPI.71)", decl.ops, limits.max_ops);
            diagnostics.push(refusal("", "manifest", note));
        }
        if decl.ops < counted {
            let note = format!(
                "the declared {ops} {} is below the checked weighted count {counted} (RPI.71, RM.38)",
                decl.ops
            );
            diagnostics.push(refusal("", "manifest", note));
        }
        if decl.overdraw > limits.max_overdraw {
            let note = format!(
                "the declared overdraw {} is over the checker's bound {} (RPI.71)",
                decl.overdraw, limits.max_overdraw
            );
            diagnostics.push(refusal("", "manifest", note));
        }
    }
    let stage = diagnostics.is_empty().then(|| CheckedStage {
        module: module.to_string(),
        entry: entry.to_string(),
        decl: decl.clone(),
        counted_ops: counted,
        definition: modules.iter().map(|&m| program.modules[m].clone()).collect(),
    });
    Ok(StageAdmission { diagnostics, stage })
}

/// The modules of the stage functions that `start` calls, directly or not, added to `out`.
fn reached(program: &Program, start: (usize, usize), out: &mut BTreeSet<usize>) {
    let mut seen = BTreeSet::new();
    let mut todo = vec![start];
    while let Some((mi, fi)) = todo.pop() {
        if !seen.insert((mi, fi)) {
            continue;
        }
        out.insert(mi);
        let mut stack = vec![&program.modules[mi].functions[fi].body];
        while let Some(e) = stack.pop() {
            if let Expr::Call { callee: Callee::Fn { module, name }, .. } = e {
                let mj = match module {
                    None => Some(mi),
                    Some(m) => program.modules.iter().position(|x| &x.name == m),
                };
                let target = mj.and_then(|mj| {
                    let m = &program.modules[mj];
                    let fj = m.functions.iter().position(|f| &f.name == name)?;
                    matches!(m.kind, ModuleKind::Stage(_)).then_some((mj, fj))
                });
                todo.extend(target);
            }
            stack.extend(e.children());
        }
    }
}

/// A limit the counter checks. A function walked again (called twice, or inside a loop) refuses each limit once at
/// each place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Limit {
    Tier,
    Weight,
    LoopCount,
    LoopDepth,
}

struct Counter<'p> {
    program: &'p Program,
    limits: &'p StageLimits,
    /// The weighted count of each function's body, with no enclosing loop.
    memo: BTreeMap<(usize, usize), u64>,
    found: Vec<Found>,
    refused: BTreeSet<((usize, Option<usize>, Vec<u32>), Limit)>,
    depth: u32,
    cur: (usize, usize),
    path: Vec<u32>,
}

impl Counter<'_> {
    fn refuse(&mut self, limit: Limit, note: String) {
        let key = (self.cur.0, Some(self.cur.1), self.path.clone());
        if !self.refused.insert((key.clone(), limit)) {
            return;
        }
        let program = self.program;
        let m = &program.modules[self.cur.0];
        let d = Diagnostic {
            code: STAGE_RULE_VIOLATION,
            module: m.name.clone(),
            function: m.functions[self.cur.1].name.clone(),
            path: path_text(&self.path),
            value_type: None,
            display_control: false,
            qualifier_erasure: false,
            mismatch: None,
            note,
        };
        self.found.push(Found { key, diagnostic: d });
    }

    /// The weight of an operation of tier `tier`, refused when its tier is not enabled or it has no weight.
    fn weight(&mut self, name: &str, tier: u8) -> u64 {
        if tier > self.limits.tier {
            self.refuse(
                Limit::Tier,
                format!(
                    "`{name}` is in tier {tier}, and the tiers enabled are those up to {} (RPI.70, RPI.36)",
                    self.limits.tier
                ),
            );
        }
        match self.limits.weights.get(name) {
            Some(w) => *w,
            None => {
                self.refuse(Limit::Weight, format!("`{name}` has no measured weight (RM.38)"));
                0
            }
        }
    }

    fn function(&mut self, mi: usize, fi: usize) -> u64 {
        // A function counted outside every loop is counted once; inside a loop its loops nest deeper, so it is walked
        // again to check the depth.
        if self.depth == 0 {
            if let Some(&n) = self.memo.get(&(mi, fi)) {
                return n;
            }
        }
        let saved = (self.cur, std::mem::take(&mut self.path));
        self.cur = (mi, fi);
        let program = self.program;
        let n = self.expr(&program.modules[mi].functions[fi].body);
        (self.cur, self.path) = saved;
        if self.depth == 0 {
            self.memo.insert((mi, fi), n);
        }
        n
    }

    fn at(&mut self, i: u32, e: &Expr) -> u64 {
        self.path.push(i);
        let n = self.expr(e);
        self.path.pop();
        n
    }

    fn expr(&mut self, e: &Expr) -> u64 {
        match e {
            Expr::Lit(_) | Expr::Var(_) => 0,
            Expr::Let { value, body, .. } => self.at(0, value).saturating_add(self.at(1, body)),
            Expr::DisplayIf { cond, then, els } => {
                let w = self.weight("display_if", 1);
                let c = self.at(0, cond);
                let (t, f) = (self.at(1, then), self.at(2, els));
                w.saturating_add(c).saturating_add(t.max(f))
            }
            Expr::Record(fields) => {
                fields.iter().enumerate().fold(0u64, |n, (i, (_, f))| n.saturating_add(self.at(i as u32, f)))
            }
            Expr::Field { value, .. } => self.at(0, value),
            Expr::Call { callee: Callee::Fn { module, name }, args } => {
                let program = self.program;
                let mj = match module {
                    None => self.cur.0,
                    Some(m) => program.modules.iter().position(|x| &x.name == m).unwrap_or(self.cur.0),
                };
                let fj = program.modules[mj].functions.iter().position(|f| &f.name == name).unwrap_or(0);
                let a = args.iter().enumerate().fold(0u64, |n, (i, a)| n.saturating_add(self.at(i as u32, a)));
                a.saturating_add(self.function(mj, fj))
            }
            Expr::Prim { op, args } => {
                let (name, tier) = match op {
                    Prim::ApproxAdd => ("approx_add", 1),
                    Prim::ApproxLess => ("approx_less", 1),
                    Prim::ApproxAnd => ("approx_and", 1),
                    Prim::ApproxNot => ("approx_not", 1),
                    Prim::Index => ("index", 1),
                    Prim::Stage(s) => (s.name(), s.tier()),
                    _ => return 0,
                };
                let w = self.weight(name, tier);
                if matches!(op, Prim::Index | Prim::Stage(StageOp::Exclude)) {
                    // A literal position or radius costs nothing; the array is read by the load itself.
                    let first = if *op == Prim::Index { 0 } else { self.at(0, &args[0]) };
                    return w.saturating_add(first);
                }
                args.iter().enumerate().fold(w, |n, (i, a)| n.saturating_add(self.at(i as u32, a)))
            }
            Expr::Repeat { count, init, body, .. } => {
                let k = literal_nat(count).unwrap_or(0);
                if k > self.limits.max_loop_count {
                    self.path.push(0);
                    self.refuse(
                        Limit::LoopCount,
                        format!(
                            "the loop's count {k} is over the checker's bound {} (RPI.71)",
                            self.limits.max_loop_count
                        ),
                    );
                    self.path.pop();
                }
                self.depth += 1;
                if self.depth > self.limits.max_loop_depth {
                    self.refuse(
                        Limit::LoopDepth,
                        format!(
                            "loops nest {} deep in the expanded chain, over the checker's bound {} (RPI.71)",
                            self.depth, self.limits.max_loop_depth
                        ),
                    );
                }
                let i = self.at(1, init);
                let b = self.at(2, body);
                self.depth -= 1;
                i.saturating_add(b.saturating_mul(k))
            }
            _ => 0,
        }
    }
}
