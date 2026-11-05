// Independent Rust execution of LX-8's experiment type/effect/authority checks.
// The frozen catalogue is data only; no fixture outputs or Python calls are used.
use std::collections::{BTreeMap, BTreeSet};
use crate::json::{obj, strings, J};
use crate::Int;

type C<T> = Result<T, String>;
fn need(ok: bool, message: &str) -> C<()> { if ok { Ok(()) } else { Err(message.into()) } }
fn set(j: &J) -> BTreeSet<String> { j.a().iter().map(|x| x.s().to_string()).collect() }
fn array(values: &BTreeSet<String>) -> J { J::Arr(values.iter().map(|x| x.clone().into()).collect()) }
fn split_type(s: &str) -> C<(String, Vec<String>)> {
    let Some((head, tail)) = s.split_once('<') else { return Ok((s.into(), vec![])); };
    need(s.ends_with('>'), "unclosed type")?;
    let mut depth = 0i32; let mut begin = 0; let mut args = vec![];
    for (i, c) in tail[..tail.len()-1].char_indices() {
        match c {
            '<' => depth += 1, '>' => depth -= 1,
            ',' if depth == 0 => { args.push(tail[begin..i].into()); begin = i + 1; }, _ => (),
        }
        need(depth >= 0, "malformed type")?;
    }
    need(depth == 0, "malformed type")?; args.push(tail[begin..tail.len()-1].into()); Ok((head.into(), args))
}
fn check_type(s: &str, cat: &J) -> C<()> {
    let (head, args) = split_type(s)?;
    need(head != "UserActionToken", "user-action tokens are shell-only and not language values")?;
    let slots = cat["TYPE_SLOTS"].obj().get(&head).ok_or_else(|| format!("unknown type: {head}"))?.a();
    need(args.len() == slots.len(), &format!("wrong type arity: {s}"))?;
    for (arg, slot) in args.iter().zip(slots) {
        if slot.s() == "evidence" {
            check_type(arg, cat)?;
            need(["Evidence", "Certificate"].contains(&split_type(arg)?.0.as_str()), "verdict needs evidence payload")?;
        } else { need(cat["INDICES"][slot.s()].a().iter().any(|x| x.s() == arg), &format!("wrong {} index: {s}", slot.s()))?; }
    }
    if ["Word", "CompressedWord", "WordKey", "Recipe", "MenuLetter"].contains(&head.as_str()) {
        need(args == ["Full", "Retained"] || args == ["Jumble", "MenuA"], "word alphabet/model mismatch")?;
    }
    if ["Snapshot", "DerivedState"].contains(&head.as_str()) {
        need(args[..2] == ["Full", "Labels"] || args[..2] == ["Jumble", "Poses"], "state model/level mismatch")?;
    }
    Ok(())
}
fn effects(j: &J, cat: &J) -> C<J> {
    let found = set(j);
    need(j.a().len() == found.len(), "duplicate effect")?;
    need(found.is_subset(&set(&cat["EFFECTS"])), "unknown effect (commit is never an effect)")?;
    Ok(array(&found))
}
fn charges(j: &J) -> C<()> {
    let expected: BTreeSet<_> = ["steps", "kernel", "memory", "output"].into_iter().map(str::to_string).collect();
    need(j.obj().keys().cloned().collect::<BTreeSet<_>>() == expected, "wrong charge dimensions")?;
    need(j.obj().values().all(|v| match v {
        J::Str(s) => s == "0" || (!s.is_empty() && s.as_bytes()[0] >= b'1' && s.as_bytes()[0] <= b'9' && s.bytes().all(|c| c.is_ascii_digit())),
        _ => false,
    }), "charges must be exact nonnegative decimal strings")
}
fn charge(steps: &str) -> J { obj(&[("steps", steps.into()), ("kernel", "0".into()), ("memory", "0".into()), ("output", "0".into())]) }
struct Checker<'a> {
    parsed: &'a J, interfaces: &'a BTreeMap<String, J>, cat: &'a J,
    signatures: BTreeMap<String, J>, declarations: BTreeSet<String>, used_calls: BTreeMap<String, BTreeSet<String>>,
}
impl Checker<'_> {
    fn source(&self, node: &J) -> J { let mut s = node["source"].clone(); s.set("sha256", self.parsed["digest"].clone()); s }
    fn operation(&self, node: &J, op: &str, type_: &str, effects: J, charges: J, fields: &[(&str, J)]) -> C<J> {
        check_type(type_, self.cat)?;
        let mut out = obj(fields);
        for (key, value) in [("op", op.into()), ("type", type_.into()), ("effects", effects), ("charges", charges), ("source", self.source(node))] { out.set(key, value); }
        Ok(out)
    }
    fn expression(&self, node: &J, env: &BTreeMap<String, J>, calls: &mut BTreeSet<String>) -> C<J> {
        let kind = node["kind"].s();
        let literal_type = match kind { "int" => Some("Int"), "rat" => Some("Rat"), "q5" => Some("Q5"), "text" => Some("Text"), _ => None };
        if let Some(t) = literal_type {
            return self.operation(node, &format!("literal.{kind}"), t, vec![].into(), charge("1"), &[("value", node["value"].clone())]);
        }
        if kind == "approx" {
            return self.operation(node, "literal.approx", &format!("Approx<P{}>", node["value"]["precision_bits"].s()), vec![].into(), charge("1"), &[("value", node["value"].clone())]);
        }
        if kind == "word" || kind == "menu" {
            return self.operation(node, "literal.word", if kind == "word" { "Word<Full,Retained>" } else { "Word<Jumble,MenuA>" },
                vec![].into(), charge("1"), &[("letters", node["value"].clone()), ("expanded_length", node["value"].a().len().to_string().into())]);
        }
        if kind == "ref" {
            let name = node["name"].s(); let value = env.get(name).ok_or_else(|| format!("unbound name: {name}"))?;
            let mut fields = vec![("name", name.into())];
            if value.has("expanded_length") { fields.push(("expanded_length", value["expanded_length"].clone())); }
            return self.operation(node, "ref", value["type"].s(), vec![].into(), charge("0"), &fields);
        }
        if kind == "match" {
            let subject = self.expression(&node["subject"], env, calls)?;
            let (head, args) = split_type(subject["type"].s())?;
            need(head == "Verdict", "match requires a verdict, never an approximate boolean")?;
            let source_arms = node["arms"].a();
            let tags: BTreeSet<_> = source_arms.iter().map(|a| a["tag"].s()).collect();
            need(source_arms.len() == 3 && tags == ["yes", "no", "unknown"].into_iter().collect(), "verdict match must cover yes, no, unknown once")?;
            let mut arms = vec![]; let mut all_effects = set(&subject["effects"]); let mut types = BTreeSet::new();
            for arm in source_arms {
                let binder = arm["binder"].s(); need(!env.contains_key(binder), "case binder shadows a name")?;
                let payload = if arm["tag"].s() == "unknown" { "Text" } else { &args[0] };
                let mut arm_env = env.clone(); arm_env.insert(binder.into(), obj(&[("type", payload.into())]));
                let body = self.expression(&arm["body"], &arm_env, calls)?;
                types.insert(body["type"].s().to_string()); all_effects.extend(set(&body["effects"]));
                arms.push(obj(&[("tag", arm["tag"].clone()), ("binder", obj(&[("name", binder.into()), ("type", payload.into())])), ("body", body)]));
            }
            need(types.len() == 1, "case result types differ")?;
            return self.operation(node, "match.verdict", types.first().unwrap(), array(&all_effects), charge("1"), &[("subject", subject), ("arms", arms.into())]);
        }
        need(kind == "call", "recovery holes and unknown expressions cannot lower")?;
        let args: Vec<J> = node["args"].a().iter().map(|a| self.expression(a, env, calls)).collect::<C<_>>()?;
        let name = node["name"].s(); let mut extra = vec![];
        let signature = if self.cat["WORD_OPERATIONS"].a().iter().any(|w| w.s() == name) {
            let (result, length) = self.word_call(name, &args)?;
            if let Some(n) = length { extra.push(("expanded_length", n.into())); }
            obj(&[("result", result.into()), ("effects", vec![].into()), ("charges", charge("1"))])
        } else {
            let signature = self.signatures.get(name).ok_or_else(|| format!("unknown function or forbidden operation: {name}"))?;
            need(args.iter().map(|a| a["type"].clone()).collect::<Vec<_>>() == signature["params"].a(), &format!("argument types differ for {name}"))?;
            calls.insert(name.into()); signature.clone()
        };
        let mut all_effects = set(&signature["effects"]);
        for arg in &args { all_effects.extend(set(&arg["effects"])); }
        extra.extend([("target", name.into()), ("args", args.into())]);
        self.operation(node, "call", signature["result"].s(), array(&all_effects), signature["charges"].clone(), &extra)
    }
    fn word_call(&self, name: &str, args: &[J]) -> C<(String, Option<String>)> {
        need(args.len() == if name == "repeat" || name == "join" { 2 } else { 1 }, "wrong word-operation arity")?;
        let (head, indices) = split_type(args[0]["type"].s())?;
        need(head == "Word" || head == "CompressedWord", "word operations cannot consume equality keys")?;
        let suffix = format!("<{}>", indices.join(","));
        if name == "word_key" { need(head == "Word", "fixture key input must be literal")?; return Ok((format!("WordKey{suffix}"), None)); }
        if name == "word_length" { return Ok((format!("Cost<{},{}>", indices[0], if indices[0] == "Full" { "Generators" } else { "MenuLetters" }), None)); }
        if name == "repeat" {
            need(args[1]["type"].s() == "Int" && args[1]["op"].s() == "literal.int", "fixture repeat count must be an exact literal")?;
            let count = Int::parse(args[1]["value"].s()); need(count.sign >= 0, "negative repeat count")?;
            need(args[0].has("expanded_length"), "fixture compression needs a known length")?;
            return Ok((format!("CompressedWord{suffix}"), Some(Int::parse(args[0]["expanded_length"].s()).mul(&count).decimal())));
        }
        need(args[0]["type"] == args[1]["type"] && head == "CompressedWord", "join requires equal compressed alphabets")?;
        need(args.iter().all(|a| a.has("expanded_length")), "fixture join needs known lengths")?;
        Ok((format!("CompressedWord{suffix}"), Some(Int::parse(args[0]["expanded_length"].s()).add(&Int::parse(args[1]["expanded_length"].s())).decimal())))
    }
    fn prepare(&mut self) -> C<()> {
        let p = self.parsed;
        need(p["diagnostics"].a().is_empty() && p["holes"].a().is_empty(), "broken drafts cannot lower")?;
        need(p["edition"] == J::from("lx8-0") && p["module"] != J::Null, "missing or unsupported edition/module")?;
        effects(&p["requires"], self.cat)?;
        let mut aliases = BTreeSet::new();
        for import in p["imports"].a() {
            let alias = import["name"].s(); let digest = import["digest"].s();
            need(aliases.insert(alias), "duplicate module alias")?;
            need(digest.len() == 64 && digest.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) && self.interfaces.contains_key(digest), "dependency interface is not pinned")?;
            let interface = &self.interfaces[digest];
            need(interface["ir_version"] == self.cat["IR_VERSION"], "unsupported interface IR")?;
            for (export, signature) in interface["exports"].obj() {
                let name = format!("{alias}_{export}"); need(!self.signatures.contains_key(&name), "duplicate imported name")?;
                for t in signature["params"].a().iter().chain(std::iter::once(&signature["result"])) { check_type(t.s(), self.cat)?; }
                effects(&signature["effects"], self.cat)?; charges(&signature["charges"])?;
                self.signatures.insert(name, signature.clone());
            }
        }
        for d in p["declarations"].a() {
            let name = d["name"].s();
            need(!self.signatures.contains_key(name) && !self.declarations.contains(name) && !self.cat["WORD_OPERATIONS"].a().iter().any(|w| w.s() == name), &format!("duplicate declaration: {name}"))?;
            self.declarations.insert(name.into()); let params = d["params"].a();
            need(params.iter().map(|p| p["name"].s()).collect::<BTreeSet<_>>().len() == params.len(), "duplicate parameter")?;
            for t in params.iter().map(|p| &p["type"]).chain(std::iter::once(&d["result"])) { check_type(t.s(), self.cat)?; }
            let declared = effects(&d["effects"], self.cat)?;
            need(set(&declared).is_subset(&set(&p["requires"])), "module lacks a required capability")?; charges(&d["charges"])?;
            if d["kind"].s() == "function" {
                self.signatures.insert(name.into(), obj(&[("params", params.iter().map(|p| p["type"].clone()).collect::<Vec<_>>().into()),
                    ("result", d["result"].clone()), ("effects", declared), ("charges", d["charges"].clone())]));
            }
        }
        Ok(())
    }
    fn declaration(&mut self, d: &J) -> C<J> {
        let mut env: BTreeMap<String, J> = d["params"].a().iter().map(|p| (p["name"].s().into(), obj(&[("type", p["type"].clone())]))).collect();
        let mut bindings = vec![]; let mut all_effects = BTreeSet::new(); let mut calls = BTreeSet::new();
        for binding in d["bindings"].a() {
            let name = binding["name"].s(); need(!env.contains_key(name), "duplicate binding")?;
            let value = self.expression(&binding["value"], &env, &mut calls)?;
            bindings.push(obj(&[("name", name.into()), ("type", value["type"].clone()), ("value", value.clone())]));
            all_effects.extend(set(&value["effects"])); env.insert(name.into(), value);
        }
        let body = self.expression(&d["body"], &env, &mut calls)?; all_effects.extend(set(&body["effects"]));
        need(body["type"] == d["result"], &format!("return type differs: {}", d["name"].s()))?;
        let declared = effects(&d["effects"], self.cat)?; need(all_effects.is_subset(&set(&declared)), "annotation erases a callee effect")?;
        let kind = d["kind"].s(); let metadata = &d["metadata"];
        if kind == "view" {
            need(set(&declared).is_disjoint(&set(&strings(&["workspace", "preview", "file", "random", "clock"]))), "effect forbidden in a view")?;
            need(["View", "DisplayPlan"].contains(&split_type(body["type"].s())?.0.as_str()), "view needs display output")?;
        }
        if kind == "command" {
            let allowed: &[&str] = match metadata["kind"].s() {
                "query" => &["snapshot", "preview", "budget"], "workspace" => &["snapshot", "workspace", "budget"],
                "view" => &["snapshot", "view", "budget"], "file" => &["file", "snapshot", "budget"],
                _ => return Err("unsupported or committing command".into()),
            };
            need(set(&declared).is_subset(&set(&strings(allowed))), "command kind cannot grant these effects")?;
            need(!metadata["contexts"].a().is_empty() && set(&metadata["contexts"]).len() == metadata["contexts"].a().len(), "invalid command contexts")?;
            let parts: Vec<_> = metadata["row"].s().split('.').collect();
            need(parts.len() >= 2 && parts.iter().all(|s| !s.is_empty() && s.as_bytes()[0].is_ascii_lowercase() && s.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')), "invalid command row")?;
        }
        self.used_calls.insert(d["name"].s().into(), calls);
        Ok(obj(&[("kind", kind.into()), ("name", d["name"].clone()), ("params", d["params"].clone()), ("result", d["result"].clone()),
            ("declared_effects", declared), ("effects", array(&all_effects)), ("charges", d["charges"].clone()), ("metadata", metadata.clone()),
            ("bindings", bindings.into()), ("body", body), ("source", self.source(d))]))
    }
    fn requests(&self, name: &str, seen: &mut BTreeSet<String>) -> BTreeSet<String> {
        if seen.contains(name) { return BTreeSet::new(); }
        if let Some(p) = self.cat["PRIMITIVES"].obj().get(name) {
            return if p["effects"].a().is_empty() { BTreeSet::new() } else { [name.into()].into_iter().collect() };
        }
        seen.insert(name.into()); let mut result = BTreeSet::new();
        if let Some(calls) = self.used_calls.get(name) { for callee in calls { result.extend(self.requests(callee, seen)); } }
        result
    }
    fn run(&mut self) -> C<J> {
        self.prepare()?;
        let declarations = self.parsed["declarations"].a().iter().map(|d| self.declaration(d)).collect::<C<Vec<_>>>()?;
        for d in &declarations {
            if d["kind"].s() == "command" {
                need(self.requests(d["name"].s(), &mut BTreeSet::new()).is_subset(&set(&d["metadata"]["permits"])), "command does not declare every transitive kernel row")?;
            }
        }
        let imports = self.parsed["imports"].a().iter().map(|x| obj(&[("name", x["name"].clone()), ("digest", x["digest"].clone()), ("source", self.source(x))])).collect::<Vec<_>>();
        let mut requirements: Vec<J> = self.parsed["requires"].a().to_vec(); requirements.sort_by(|a,b| a.s().cmp(b.s()));
        Ok(obj(&[("ir_version", self.cat["IR_VERSION"].clone()), ("edition", self.parsed["edition"].clone()), ("module", self.parsed["module"].clone()),
            ("requires", requirements.into()), ("imports", imports.into()), ("declarations", declarations.into()),
            ("source", obj(&[("sha256", self.parsed["digest"].clone())]))]))
    }
}
pub fn lower(parsed: &J, interfaces: &BTreeMap<String, J>, cat: &J) -> C<J> {
    Checker { parsed, interfaces, cat, signatures: cat["PRIMITIVES"].obj().clone(), declarations: BTreeSet::new(), used_calls: BTreeMap::new() }.run()
}
