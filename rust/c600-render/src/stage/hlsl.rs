//! The HLSL generator of `m600-stage-hlsl/1` (RPI.73, RPI.78; LX-7's numerics; draft of 19 Nov 2026).
//!
//! The text, in this order: the header line `// m600-stage-hlsl/1 <kind> <digest>` with c600-display's
//! `describe(stage).digest`; the array buffer, only when the stage declares an array; the record structs `m600_r<k>`;
//! the reached functions `m600_f<module index>_<function index>`, each with its parameters and a trailing
//! `inout uint m600_cov`; and the fixed entry the core calls, `m600_vertex(...)` or `m600_fragment(...)`. Items are
//! separated by one empty line, lines end in LF and blocks indent by four spaces.
//!
//! Evaluation is LX-7's: left to right, postorder, each operation's result in its own `precise float` local (or `bool`
//! local), so there is no reassociation and no fused multiply-add, and after every arithmetic result
//! `if (m600_cov == 0u && !isfinite(v)) m600_cov = 2u;`. `exclude(v, r)` is
//! `if (m600_cov == 0u && abs(v) <= r) m600_cov = 1u;` and has the value `Unit`. The first event wins. A literal is
//! `asfloat(0x%08Xu)` of its exact binary32 value. A `let` is a new local, so shadowing renames. `display_if` is an
//! `if`/`else` statement with each branch's statements inside it, so an untaken branch runs nothing. `repeat` with the
//! literal count N is `[unroll] for (uint m600_i<k> = 0u; m600_i<k> < Nu; ++m600_i<k>) { ... acc = body; }`; a count
//! of 0 emits no loop (draft: `m600_i0 < 0u` is a constant comparison that `-WX` would refuse). `index(a, i)` reads
//! the buffer at the array's offset plus `i`, written as one literal.
//!
//! The fixed entries check the bound inputs and every element of the declared arrays first (draft: an array is an
//! input of the entry, so "inputs are checked at the entry" covers each of its elements; an input the stage omits is
//! not checked), call the stage's entry function and return its value only when the code is 0:
//!
//! - `uint m600_vertex(float x, float y, float z, float w, float t, out float3 position)`: the coverage code (0 a
//!   point, 1 `singular_neighbourhood`, 2 `nonfinite`); `position` is `float3(0, 0, 0)` unless the code is 0;
//! - `float3 m600_fragment(float class_, float shade, float x, float y, float z, float t)`: opaque RGB; a nonzero code
//!   gives `float3(0, 0, 0)` (draft: RPI.75 leaves a fragment's undefined value open).

use c600_check::CheckedStage;
use c600_display::{codes, Error, Form};
use c600_ir::{Expr, Lit, Module, Prim, StageKind, StageOp};

use super::{bind, callee, is_array, literal, nat, Binding, Bound, CONTRACT, T};

/// A stage's generated text with its binding and digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Generated {
    pub binding: Binding,
    /// `describe(stage).digest` of c600-display, which the header line names.
    pub digest: String,
    pub hlsl: String,
}

/// The text of `stage` under `m600-stage-hlsl/1`, or the binding's refusals, or a refusal of a form the text cannot
/// carry (draft: a record field whose name is no ASCII identifier, or a form outside the stage language, which the
/// stage checker does not admit).
pub fn generate(stage: &CheckedStage) -> Result<Generated, Vec<Error>> {
    let binding = bind(stage)?;
    let digest = c600_display::stage::describe(stage).digest;
    let defs = stage.definition();
    let mut g = Gen { defs, binding: &binding, structs: Vec::new() };
    let refuse = |(mi, fi): (usize, usize), why: String| {
        let (module, function) = (&defs[mi].name, &defs[mi].functions[fi].name);
        Error::new(codes::STAGE_RULE_VIOLATION, why)
            .in_form(Form::Diagnostic)
            .with("module", module)
            .with("function", function)
            .with("path", "body")
    };
    let mut functions = Vec::new();
    for &f in &binding.reached {
        functions.push(g.function(f).map_err(|why| vec![refuse(f, why)])?);
    }
    let entry = g.entry().map_err(|why| vec![refuse(binding.entry, why)])?;
    let mut out = format!("// {CONTRACT} {} {digest}\n", binding.kind.name());
    if !binding.arrays.is_empty() {
        out.push_str(&format!("\nStructuredBuffer<float> {} : register({});\n", binding.buffer(), binding.register()));
    }
    for (k, (_, fields)) in g.structs.iter().enumerate() {
        out.push_str(&format!("\nstruct m600_r{k}\n{{\n"));
        for (name, t) in fields.iter().filter(|(_, t)| !t.erased()) {
            out.push_str(&format!("    {} m_{name};\n", g.known(t)));
        }
        out.push_str("};\n");
    }
    for f in functions.iter().chain([&entry]) {
        out.push('\n');
        out.push_str(f);
    }
    Ok(Generated { digest, hlsl: out, binding })
}

/// A literal's text.
fn hex(v: f32) -> String {
    format!("asfloat(0x{:08X}u)", v.to_bits())
}

/// Whether a field name can be a member name: an ASCII identifier.
fn identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// A value in the text: its operand (none for an erased type) and its type.
#[derive(Clone, Debug)]
struct Val {
    text: Option<String>,
    ty: T,
}

impl Val {
    fn unit() -> Val {
        Val { text: None, ty: T::Unit }
    }
}

/// What a name in scope stands for.
#[derive(Clone, Debug)]
enum Name {
    Val(Val),
    /// A declared array input, at this offset in the buffer.
    Array(u64),
}

/// A function body being written.
struct Body {
    out: String,
    indent: usize,
    locals: u32,
    loops: u32,
    env: Vec<(String, Name)>,
}

impl Body {
    fn new() -> Body {
        Body { out: String::new(), indent: 1, locals: 0, loops: 0, env: Vec::new() }
    }

    fn line(&mut self, text: &str) {
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
        self.out.push_str(text);
        self.out.push('\n');
    }

    fn local(&mut self) -> String {
        let name = format!("m600_v{}", self.locals);
        self.locals += 1;
        name
    }

    fn lookup(&self, name: &str) -> Option<&Name> {
        self.env.iter().rev().find(|(n, _)| n == name).map(|(_, v)| v)
    }
}

struct Gen<'a> {
    defs: &'a [Module],
    binding: &'a Binding,
    /// Each record type by its key, with its fields in the order of its first occurrence.
    structs: Vec<(T, Vec<(String, T)>)>,
}

type R<T> = Result<T, String>;

impl Gen<'_> {
    /// The struct of a record type, registered at its first occurrence after the structs of its record fields.
    fn register(&mut self, t: &T) -> R<usize> {
        let key = t.key();
        if let Some(k) = self.structs.iter().position(|(k, _)| *k == key) {
            return Ok(k);
        }
        let T::Rec(fields) = t else { return Err(format!("{t:?} is no record")) };
        for (name, ft) in fields {
            if !identifier(name) {
                return Err(format!("the record field `{name}` is no ASCII identifier, so no member can carry it"));
            }
            if matches!(ft, T::Rec(_)) && !ft.erased() {
                self.register(ft)?;
            }
        }
        self.structs.push((key, fields.clone()));
        Ok(self.structs.len() - 1)
    }

    /// The text of a type registered earlier.
    fn known(&self, t: &T) -> String {
        match t {
            T::F32 => "float".into(),
            T::Bool => "bool".into(),
            _ => {
                let key = t.key();
                let k = self.structs.iter().position(|(k, _)| *k == key).unwrap_or(usize::MAX);
                format!("m600_r{k}")
            }
        }
    }

    /// The text of a parameter's or result's type.
    fn ty(&mut self, t: &T) -> R<String> {
        if let T::Rec(_) = t {
            self.register(t)?;
        }
        Ok(self.known(t))
    }

    /// The text of a local's type: `precise float` for binary32.
    fn local_ty(&mut self, t: &T) -> R<String> {
        Ok(match t {
            T::F32 => "precise float".into(),
            other => self.ty(other)?,
        })
    }

    fn function(&mut self, (mi, fi): (usize, usize)) -> R<String> {
        let defs = self.defs;
        let f = &defs[mi].functions[fi];
        let entry = (mi, fi) == self.binding.entry;
        let mut b = Body::new();
        let mut params = Vec::new();
        for (i, p) in f.params.iter().enumerate() {
            if entry && is_array(&p.ty) {
                let a = self.binding.arrays.iter().find(|a| a.name == p.name);
                let a = a.ok_or_else(|| format!("`{}` is no declared array input", p.name))?;
                b.env.push((p.name.clone(), Name::Array(a.offset)));
                continue;
            }
            let t = T::of(&p.ty).ok_or_else(|| format!("the parameter `{}` has no stage type", p.name))?;
            let text = if t.erased() {
                None
            } else {
                let name = format!("m600_a{i}");
                params.push(format!("{} {name}", self.ty(&t)?));
                Some(name)
            };
            b.env.push((p.name.clone(), Name::Val(Val { text, ty: t })));
        }
        params.push("inout uint m600_cov".into());
        let result = T::of(&f.result).ok_or("the result has no stage type")?;
        let ret = if result.erased() { "void".into() } else { self.ty(&result)? };
        let v = self.expr(&mut b, mi, &f.body)?;
        match (&v.text, result.erased()) {
            (Some(text), false) => b.line(&format!("return {text};")),
            (None, true) => {}
            _ => return Err("the body's value does not match the result".into()),
        }
        Ok(format!("{ret} m600_f{mi}_{fi}({})\n{{\n{}}}\n", params.join(", "), b.out))
    }

    /// The fixed entry the core calls.
    fn entry(&mut self) -> R<String> {
        let (mi, fi) = self.binding.entry;
        let defs = self.defs;
        let f = &defs[mi].functions[fi];
        let result = T::of(&f.result).ok_or("the entry's result has no stage type")?;
        let k = self.register(&result)?;
        let mut b = Body::new();
        b.line("uint m600_cov = 0u;");
        let mut args = Vec::new();
        for p in &self.binding.params {
            if let Bound::Input(input) = p.bound {
                b.line(&format!("if (m600_cov == 0u && !isfinite({})) m600_cov = 2u;", input.hlsl()));
                args.push(input.hlsl().to_string());
            }
        }
        let total = self.binding.total();
        if total > 0 {
            b.line(&format!("for (uint m600_j = 0u; m600_j < {total}u; ++m600_j)"));
            b.line("{");
            b.line(&format!("    if (m600_cov == 0u && !isfinite({}[m600_j])) m600_cov = 2u;", self.binding.buffer()));
            b.line("}");
        }
        args.push("m600_cov".into());
        b.line(&format!("m600_r{k} m600_p = m600_f{mi}_{fi}({});", args.join(", ")));
        let (signature, names, out) = match self.binding.kind {
            StageKind::Vertex => (
                "uint m600_vertex(float x, float y, float z, float w, float t, out float3 position)",
                ["x", "y", "z"],
                "position",
            ),
            StageKind::Fragment => (
                "float3 m600_fragment(float class_, float shade, float x, float y, float z, float t)",
                ["r", "g", "b"],
                "m600_rgb",
            ),
        };
        let value = names.map(|n| format!("m600_p.m_{n}")).join(", ");
        match self.binding.kind {
            StageKind::Vertex => b.line("position = float3(0.0f, 0.0f, 0.0f);"),
            StageKind::Fragment => b.line("float3 m600_rgb = float3(0.0f, 0.0f, 0.0f);"),
        }
        b.line("if (m600_cov == 0u)");
        b.line("{");
        b.line(&format!("    {out} = float3({value});"));
        b.line("}");
        match self.binding.kind {
            StageKind::Vertex => b.line("return m600_cov;"),
            StageKind::Fragment => b.line("return m600_rgb;"),
        }
        Ok(format!("{signature}\n{{\n{}}}\n", b.out))
    }

    /// A value's statements into `b`, in evaluation order, and its operand.
    fn expr(&mut self, b: &mut Body, mi: usize, e: &Expr) -> R<Val> {
        match e {
            Expr::Lit(Lit::Unit) => Ok(Val::unit()),
            Expr::Lit(_) => {
                let v = literal(e).ok_or("a literal that denotes no exact finite binary32 value")?;
                Ok(Val { text: Some(hex(v)), ty: T::F32 })
            }
            Expr::Var(name) => match b.lookup(name) {
                Some(Name::Val(v)) => Ok(v.clone()),
                Some(Name::Array(_)) => Err(format!("the array input `{name}` is read only by `index`")),
                None => Err(format!("the name `{name}` is unbound")),
            },
            Expr::Let { name, value, body } => {
                let v = self.expr(b, mi, value)?;
                let text = match &v.text {
                    Some(text) => {
                        let local = b.local();
                        let ty = self.local_ty(&v.ty)?;
                        b.line(&format!("{ty} {local} = {text};"));
                        Some(local)
                    }
                    None => None,
                };
                b.env.push((name.clone(), Name::Val(Val { text, ty: v.ty })));
                let out = self.expr(b, mi, body);
                b.env.pop();
                out
            }
            Expr::DisplayIf { cond, then, els } => {
                let c = self.expr(b, mi, cond)?.text.ok_or("a `display_if` condition with no value")?;
                let r = b.local();
                let saved = std::mem::take(&mut b.out);
                b.indent += 1;
                let t = self.branch(b, mi, then, &r);
                let then_text = std::mem::take(&mut b.out);
                let f = t.and_then(|t| self.branch(b, mi, els, &r).map(|_| t));
                let else_text = std::mem::replace(&mut b.out, saved);
                b.indent -= 1;
                let t = f?;
                if !t.ty.erased() {
                    let ty = self.local_ty(&t.ty)?;
                    b.line(&format!("{ty} {r};"));
                }
                b.line(&format!("if ({c})"));
                b.line("{");
                b.out.push_str(&then_text);
                b.line("}");
                b.line("else");
                b.line("{");
                b.out.push_str(&else_text);
                b.line("}");
                Ok(Val { text: (!t.ty.erased()).then_some(r), ty: t.ty })
            }
            Expr::Call { callee: c, args } => {
                let (mj, fj) = callee(self.defs, mi, c).ok_or("a call that names no function of the definition")?;
                let defs = self.defs;
                let f = &defs[mj].functions[fj];
                if args.len() != f.params.len() {
                    return Err(format!("`{}` takes {} arguments, not {}", f.name, f.params.len(), args.len()));
                }
                let mut texts = Vec::new();
                for a in args {
                    texts.extend(self.expr(b, mi, a)?.text);
                }
                texts.push("m600_cov".into());
                let call = format!("m600_f{mj}_{fj}({})", texts.join(", "));
                let ty = T::of(&f.result).ok_or("a callee's result has no stage type")?;
                if ty.erased() {
                    b.line(&format!("{call};"));
                    return Ok(Val { text: None, ty });
                }
                let local = b.local();
                let lt = self.local_ty(&ty)?;
                b.line(&format!("{lt} {local} = {call};"));
                Ok(Val { text: Some(local), ty })
            }
            Expr::Prim { op, args } => self.prim(b, mi, *op, args),
            Expr::Record(fields) => {
                let mut vals = Vec::new();
                for (name, e) in fields {
                    vals.push((name.clone(), self.expr(b, mi, e)?));
                }
                let ty = T::Rec(vals.iter().map(|(n, v)| (n.clone(), v.ty.clone())).collect());
                if ty.erased() {
                    return Ok(Val { text: None, ty });
                }
                let k = self.register(&ty)?;
                let local = b.local();
                b.line(&format!("m600_r{k} {local};"));
                for (name, ft) in self.structs[k].1.clone() {
                    if ft.erased() {
                        continue;
                    }
                    let v = vals.iter().find(|(n, _)| *n == name).and_then(|(_, v)| v.text.clone());
                    let v = v.ok_or_else(|| format!("the record has no value for its field `{name}`"))?;
                    b.line(&format!("{local}.m_{name} = {v};"));
                }
                Ok(Val { text: Some(local), ty })
            }
            Expr::Field { value, field } => {
                let v = self.expr(b, mi, value)?;
                let ft = v.ty.field(field).cloned().ok_or_else(|| format!("no field `{field}`"))?;
                if ft.erased() {
                    return Ok(Val { text: None, ty: ft });
                }
                let base = v.text.ok_or_else(|| format!("a field `{field}` of a value with no text"))?;
                Ok(Val { text: Some(format!("{base}.m_{field}")), ty: ft })
            }
            Expr::Repeat { count, init, acc, result, body } => {
                let n = nat(count).ok_or("a loop whose count is no literal natural")?;
                let i = self.expr(b, mi, init)?;
                let ty = T::of(result).ok_or("a loop's value has no stage type")?;
                let a = match &i.text {
                    Some(text) if !ty.erased() => {
                        let local = b.local();
                        let lt = self.local_ty(&ty)?;
                        b.line(&format!("{lt} {local} = {text};"));
                        Some(local)
                    }
                    _ => None,
                };
                if n == 0 {
                    return Ok(Val { text: a, ty });
                }
                let k = b.loops;
                b.loops += 1;
                b.line(&format!("[unroll] for (uint m600_i{k} = 0u; m600_i{k} < {n}u; ++m600_i{k})"));
                b.line("{");
                b.indent += 1;
                b.env.push((acc.clone(), Name::Val(Val { text: a.clone(), ty: ty.clone() })));
                let v = self.expr(b, mi, body);
                b.env.pop();
                if let (Some(a), Some(text)) = (&a, &v?.text) {
                    b.line(&format!("{a} = {text};"));
                }
                b.indent -= 1;
                b.line("}");
                Ok(Val { text: a, ty })
            }
            _ => Err("an expression outside the stage language".into()),
        }
    }

    /// A `display_if` branch: its statements, then the assignment of its value to the result `r`.
    fn branch(&mut self, b: &mut Body, mi: usize, e: &Expr, r: &str) -> R<Val> {
        let v = self.expr(b, mi, e)?;
        if let Some(text) = &v.text {
            b.line(&format!("{r} = {text};"));
        }
        Ok(v)
    }

    fn prim(&mut self, b: &mut Body, mi: usize, op: Prim, args: &[Expr]) -> R<Val> {
        match op {
            Prim::Index => {
                let Some(Expr::Var(name)) = args.first() else { return Err("an index of no named array".into()) };
                let Some(Name::Array(offset)) = b.lookup(name) else {
                    return Err(format!("`{name}` is no declared array input"));
                };
                let i = args.get(1).and_then(nat).ok_or("an index at no literal position")?;
                let at = offset.checked_add(i).ok_or("an index outside the buffer")?;
                let local = b.local();
                b.line(&format!("precise float {local} = {}[{at}u];", self.binding.buffer()));
                Ok(Val { text: Some(local), ty: T::F32 })
            }
            Prim::Stage(StageOp::Exclude) => {
                let v = match args.first() {
                    Some(e) => self.expr(b, mi, e)?.text.ok_or("an exclusion of no value")?,
                    None => return Err("an exclusion of no value".into()),
                };
                let r = args.get(1).and_then(literal).ok_or("an exclusion radius that is no literal")?;
                b.line(&format!("if (m600_cov == 0u && abs({v}) <= {}) m600_cov = 1u;", hex(r)));
                Ok(Val::unit())
            }
            _ => {
                let mut ops = Vec::new();
                for a in args {
                    ops.push(self.expr(b, mi, a)?.text.ok_or("an operand with no value")?);
                }
                let unary = |f: &str| format!("{f}({})", ops[0]);
                let (text, ty) = match (op, ops.len()) {
                    (Prim::ApproxAdd, 2) => (format!("{} + {}", ops[0], ops[1]), T::F32),
                    (Prim::Stage(StageOp::Sub), 2) => (format!("{} - {}", ops[0], ops[1]), T::F32),
                    (Prim::Stage(StageOp::Mul), 2) => (format!("{} * {}", ops[0], ops[1]), T::F32),
                    (Prim::Stage(StageOp::Div), 2) => (format!("{} / {}", ops[0], ops[1]), T::F32),
                    (Prim::Stage(StageOp::Neg), 1) => (format!("-{}", ops[0]), T::F32),
                    (Prim::Stage(StageOp::Abs), 1) => (unary("abs"), T::F32),
                    (Prim::Stage(StageOp::Sqrt), 1) => (unary("sqrt"), T::F32),
                    (Prim::Stage(StageOp::Sin), 1) => (unary("sin"), T::F32),
                    (Prim::Stage(StageOp::Cos), 1) => (unary("cos"), T::F32),
                    (Prim::Stage(StageOp::Exp2), 1) => (unary("exp2"), T::F32),
                    (Prim::Stage(StageOp::Log2), 1) => (unary("log2"), T::F32),
                    (Prim::ApproxLess, 2) => (format!("{} < {}", ops[0], ops[1]), T::Bool),
                    (Prim::ApproxAnd, 2) => (format!("{} && {}", ops[0], ops[1]), T::Bool),
                    (Prim::ApproxNot, 1) => (format!("!{}", ops[0]), T::Bool),
                    _ => return Err(format!("`{op:?}` with {} operands is outside the operation table", ops.len())),
                };
                let local = b.local();
                if ty == T::F32 {
                    b.line(&format!("precise float {local} = {text};"));
                    b.line(&format!("if (m600_cov == 0u && !isfinite({local})) m600_cov = 2u;"));
                } else {
                    b.line(&format!("bool {local} = {text};"));
                }
                Ok(Val { text: Some(local), ty })
            }
        }
    }
}
