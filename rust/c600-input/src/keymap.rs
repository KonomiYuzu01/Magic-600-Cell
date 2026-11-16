//! The key map: recorded bindings, EV4.7's checks and the draft default map.
//!
//! A [`BindingSpec`] records each chord as a physical key code or, for DT-M1 Q2 (b)'s office chords only, as a
//! character with its modifiers. [`KeyMap::apply`] checks the specs under every declared layout and resolves them under
//! the active one; any finding refuses the whole map before it applies, so the router keeps its previous map.

use c600_kernel::json::{self, Value};

use crate::binding::{Arg, Binding, Destination, RoutingClass};
use crate::chord::{Chord, Mods};
use crate::layout::Layout;

/// The codes the key-map checks raise: EV4.3's three, each an active row of chapter 13.
pub mod code {
    /// Two bindings resolve to the same chord sequence in one routing class. As a draft reading, also a binding that
    /// conflicts with the keys its routing class takes itself (EV4.4): a text-class or capture-class binding, or a
    /// list-class binding to a row other than navigation, inspection and O9's row copy.
    pub const BINDING_CONFLICT: &str = "EV4:input.binding_conflict";
    /// A complete binding is also a prefix of another sequence in its routing class, or has no chord at all.
    pub const BINDING_PREFIX_CONFLICT: &str = "EV4:input.binding_prefix_conflict";
    /// A binding recorded by character does not resolve under a declared layout, or two bindings collide only through
    /// that resolution, or the active layout is not declared.
    pub const BINDING_LAYOUT_MISMATCH: &str = "EV4:input.binding_layout_mismatch";
}

/// The rows a list-class binding may name: navigation, inspection and O9's row copy (DT-M1 fix-first item 2).
pub const LIST_ROWS: [&str; 3] = ["list.move", "object.inspect", "list.row.copy"];

/// One chord of a recorded binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChordIntent {
    /// A physical chord, the portable form (K5).
    Physical(Chord),
    /// A character with its modifiers, resolved under a declared layout (Q2 (b)).
    Character { mods: Mods, character: char },
}

impl ChordIntent {
    /// Parses a physical chord (`Ctrl+Shift+Enter`) or a character intent (`Ctrl+'z'`).
    pub fn parse(text: &str) -> Option<ChordIntent> {
        match text.find('\'') {
            None => Chord::parse(text).map(ChordIntent::Physical),
            Some(at) => {
                let mods = Mods::parse(text[..at].strip_suffix('+').unwrap_or(&text[..at]))?;
                let mut chars = text[at..].strip_prefix('\'')?.strip_suffix('\'')?.chars();
                let character = chars.next()?;
                chars.next().is_none().then_some(ChordIntent::Character { mods, character })
            }
        }
    }

    pub fn resolve(&self, layout: &Layout) -> Option<Chord> {
        match self {
            ChordIntent::Physical(chord) => Some(*chord),
            ChordIntent::Character { mods, character } => layout.resolve(*mods, *character),
        }
    }
}

/// A binding as recorded, before a layout resolves it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingSpec {
    pub sequence: Vec<ChordIntent>,
    pub context: RoutingClass,
    pub command: String,
    pub args: Vec<(String, Arg)>,
    pub destination: Option<Destination>,
}

impl BindingSpec {
    /// A spec from chord texts ([`ChordIntent::parse`]); panics on a text that does not parse.
    pub fn new(sequence: &[&str], context: RoutingClass, command: &str, args: Vec<(&str, Arg)>) -> BindingSpec {
        BindingSpec {
            sequence: sequence
                .iter()
                .map(|text| ChordIntent::parse(text).unwrap_or_else(|| panic!("`{text}` is not a chord")))
                .collect(),
            context,
            command: command.to_owned(),
            args: args.into_iter().map(|(name, arg)| (name.to_owned(), arg)).collect(),
            destination: None,
        }
    }

    /// The same spec, serving one destination only.
    pub fn only(mut self, destination: Destination) -> BindingSpec {
        self.destination = Some(destination);
        self
    }

    /// Whether any chord is recorded by character.
    pub fn by_character(&self) -> bool {
        self.sequence.iter().any(|c| matches!(c, ChordIntent::Character { .. }))
    }

    fn resolve(&self, layout: &Layout) -> Option<Vec<Chord>> {
        self.sequence.iter().map(|intent| intent.resolve(layout)).collect()
    }

    fn label(&self) -> String {
        let chords: Vec<String> = self
            .sequence
            .iter()
            .map(|intent| match intent {
                ChordIntent::Physical(chord) => chord.to_string(),
                ChordIntent::Character { mods, character } => format!("{mods}'{character}'"),
            })
            .collect();
        format!("{} -> {}", chords.join(", "), self.command)
    }
}

/// One finding of the key-map checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub code: &'static str,
    /// The declared layout the finding holds under; `None` when it holds under every layout.
    pub layout: Option<&'static str>,
    /// The indices of the specs involved.
    pub specs: Vec<usize>,
    pub detail: String,
}

/// EV4.7's checks: the class rule, then, under every declared layout, resolution, duplicates and the prefix-code
/// property, each separately within one routing class.
pub fn check(specs: &[BindingSpec], declared: &[Layout], active: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut add = |finding: Finding| {
        if !findings.contains(&finding) {
            findings.push(finding);
        }
    };
    if !declared.iter().any(|layout| layout.id() == active) {
        add(Finding {
            code: code::BINDING_LAYOUT_MISMATCH,
            layout: None,
            specs: Vec::new(),
            detail: format!("the active layout `{active}` is not declared"),
        });
    }
    for (i, spec) in specs.iter().enumerate() {
        let refused = match spec.context {
            RoutingClass::Text | RoutingClass::Capture => true,
            RoutingClass::List => !LIST_ROWS.contains(&spec.command.as_str()),
            RoutingClass::Global => false,
        };
        if refused {
            add(Finding {
                code: code::BINDING_CONFLICT,
                layout: None,
                specs: vec![i],
                detail: format!("`{}` conflicts with the keys the {:?} class takes itself", spec.label(), spec.context),
            });
        }
        if spec.sequence.is_empty() {
            add(Finding {
                code: code::BINDING_PREFIX_CONFLICT,
                layout: None,
                specs: vec![i],
                detail: format!("`{}` has no chord, so it is a prefix of every sequence", spec.label()),
            });
        }
    }
    for layout in declared {
        let resolved: Vec<Option<Vec<Chord>>> = specs.iter().map(|spec| spec.resolve(layout)).collect();
        for (i, spec) in specs.iter().enumerate() {
            if resolved[i].is_none() {
                add(Finding {
                    code: code::BINDING_LAYOUT_MISMATCH,
                    layout: Some(layout.id()),
                    specs: vec![i],
                    detail: format!("`{}` does not resolve under {}", spec.label(), layout.id()),
                });
            }
        }
        for i in 0..specs.len() {
            for j in i + 1..specs.len() {
                let (a, b) = (&specs[i], &specs[j]);
                let (Some(x), Some(y)) = (&resolved[i], &resolved[j]) else { continue };
                if a.context != b.context || x.is_empty() || y.is_empty() {
                    continue;
                }
                let dependent = a.by_character() || b.by_character();
                let at = dependent.then_some(layout.id());
                if x == y {
                    let overlap = match (a.destination, b.destination) {
                        (Some(p), Some(q)) => p == q,
                        _ => true,
                    };
                    if !overlap {
                        continue;
                    }
                    let found = match a.sequence == b.sequence {
                        true => code::BINDING_CONFLICT,
                        false => code::BINDING_LAYOUT_MISMATCH,
                    };
                    add(Finding {
                        code: found,
                        layout: at,
                        specs: vec![i, j],
                        detail: format!("`{}` and `{}` take the same chords", a.label(), b.label()),
                    });
                } else if x.starts_with(y) || y.starts_with(x) {
                    add(Finding {
                        code: code::BINDING_PREFIX_CONFLICT,
                        layout: at,
                        specs: vec![i, j],
                        detail: format!("`{}` and `{}`: a complete binding is also a prefix", a.label(), b.label()),
                    });
                }
            }
        }
    }
    findings
}

/// An applied key map: the bindings resolved under the active layout, after [`check`] found nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyMap {
    layout: Layout,
    declared: Vec<&'static str>,
    bindings: Vec<Binding>,
}

impl KeyMap {
    /// Checks `specs` under every declared layout and resolves them under `active`. Any finding refuses the map.
    pub fn apply(specs: &[BindingSpec], declared: &[Layout], active: &str) -> Result<KeyMap, Vec<Finding>> {
        let findings = check(specs, declared, active);
        if !findings.is_empty() {
            return Err(findings);
        }
        let layout = *declared.iter().find(|layout| layout.id() == active).expect("the check saw the active layout");
        let bindings = specs
            .iter()
            .map(|spec| Binding {
                sequence: spec.resolve(&layout).expect("the check resolved every spec"),
                context: spec.context,
                command: spec.command.clone(),
                args: spec.args.clone(),
                layout: spec.by_character().then_some(layout.id()),
                destination: spec.destination,
            })
            .collect();
        Ok(KeyMap { layout, declared: declared.iter().map(Layout::id).collect(), bindings })
    }

    /// The draft default map under `active`, declared for US QWERTY and French AZERTY.
    pub fn default_for(active: &str) -> Result<KeyMap, Vec<Finding>> {
        KeyMap::apply(&default_specs(), &[Layout::US, Layout::FRENCH], active)
    }

    /// The active layout.
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn declared(&self) -> &[&'static str] {
        &self.declared
    }

    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }
}

fn int(n: i64) -> Value {
    Value::Int(n)
}

/// A twist key's rotation: a placeholder until the twist rows take A6's exact requests (KN3.7).
fn rotation(axis: i64, inverse: bool) -> Value {
    json::object([("axis", int(axis)), ("inverse", Value::Bool(inverse))])
}

fn camera(axis: &str, step: i64) -> Value {
    json::object([(axis, int(step))])
}

/// The draft default map. G6 has no keymap table yet; each choice here is a draft (see the crate's drafts).
pub fn default_specs() -> Vec<BindingSpec> {
    use Arg::*;
    use RoutingClass::{Global, List as ListClass};
    let view = || Fixed(Value::String("global".into()));
    let mut specs = vec![
        // A focused list or view: the arrows, Enter to inspect and O9's number keys (EV4.4). Tab is focus movement
        // inside the shell and has no row.
        BindingSpec::new(&["ArrowUp"], ListClass, "list.move", vec![("list", List), ("entry", ListEntry(-1))]),
        BindingSpec::new(&["ArrowLeft"], ListClass, "list.move", vec![("list", List), ("entry", ListEntry(-1))]),
        BindingSpec::new(&["ArrowDown"], ListClass, "list.move", vec![("list", List), ("entry", ListEntry(1))]),
        BindingSpec::new(&["ArrowRight"], ListClass, "list.move", vec![("list", List), ("entry", ListEntry(1))]),
        BindingSpec::new(&["Enter"], ListClass, "object.inspect", vec![("object", Selected)]),
        // The global map.
        BindingSpec::new(&["Ctrl+Enter"], Global, "operation.preview", vec![("draft", Draft)]),
        BindingSpec::new(&["Ctrl+Shift+Enter"], Global, "operation.execute", vec![("preview", PresentedPreview)]),
        BindingSpec::new(
            &["Ctrl+Shift+Backspace"],
            Global,
            "operation.preview.cancel",
            vec![("preview", PresentedPreview)],
        ),
        BindingSpec::new(&["Ctrl+'z'"], Global, "session.undo", vec![("hint", UndoHint)]),
        BindingSpec::new(&["Ctrl+'y'"], Global, "session.redo", vec![("hint", RedoHint)]),
        BindingSpec::new(&["Escape"], Global, "transient.close", vec![]),
        BindingSpec::new(&["F9"], Global, "keyboard.toggle", vec![]),
        BindingSpec::new(&["Ctrl+KeyG", "KeyF"], Global, "display.frame.toggle", vec![]),
        BindingSpec::new(&["Ctrl+KeyG", "KeyI"], Global, "twist.inverse.toggle", vec![]),
        BindingSpec::new(
            &["ArrowLeft"],
            Global,
            "camera.rotate",
            vec![("view", view()), ("rotation", Fixed(camera("yaw", -1)))],
        ),
        BindingSpec::new(
            &["ArrowRight"],
            Global,
            "camera.rotate",
            vec![("view", view()), ("rotation", Fixed(camera("yaw", 1)))],
        ),
        BindingSpec::new(
            &["ArrowUp"],
            Global,
            "camera.rotate",
            vec![("view", view()), ("rotation", Fixed(camera("pitch", -1)))],
        ),
        BindingSpec::new(
            &["ArrowDown"],
            Global,
            "camera.rotate",
            vec![("view", view()), ("rotation", Fixed(camera("pitch", 1)))],
        ),
        BindingSpec::new(
            &["Equal"],
            Global,
            "camera.zoom",
            vec![("view", view()), ("scale", Fixed(camera("step", 1)))],
        ),
        BindingSpec::new(
            &["Minus"],
            Global,
            "camera.zoom",
            vec![("view", view()), ("scale", Fixed(camera("step", -1)))],
        ),
        BindingSpec::new(
            &["Alt+ArrowLeft"],
            Global,
            "view.preview.scrub",
            vec![("preview", PresentedPreview), ("prefix", Scrub(-1)), ("revision", Revision)],
        ),
        BindingSpec::new(
            &["Alt+ArrowRight"],
            Global,
            "view.preview.scrub",
            vec![("preview", PresentedPreview), ("prefix", Scrub(1)), ("revision", Revision)],
        ),
    ];
    for (n, key) in ["Digit1", "Digit2", "Digit3", "Digit4", "Digit5", "Digit6", "Digit7", "Digit8", "Digit9", "Digit0"]
        .iter()
        .enumerate()
    {
        specs.push(BindingSpec::new(
            &[*key],
            ListClass,
            "list.row.copy",
            vec![("result", ListResult), ("object", ListRow(n + 1))],
        ));
    }
    for (slot, key) in ["KeyQ", "KeyW", "KeyE", "KeyR"].iter().enumerate() {
        specs.push(BindingSpec::new(
            &[*key],
            Global,
            "grip.select",
            vec![("cap", SlotCap(slot)), ("frame", SlotFrame(slot))],
        ));
    }
    for (axis, key) in ["KeyJ", "KeyK", "KeyL"].iter().enumerate() {
        for inverse in [false, true] {
            let chord = if inverse { format!("Shift+{key}") } else { (*key).to_owned() };
            let r = Fixed(rotation(axis as i64, inverse));
            specs.push(
                BindingSpec::new(
                    &[chord.as_str()],
                    Global,
                    "twist.apply",
                    vec![("cap", GripCap), ("rotation", r.clone())],
                )
                .only(Destination::Live),
            );
            specs.push(
                BindingSpec::new(
                    &[chord.as_str()],
                    Global,
                    "operation.phase.append_twist",
                    vec![("cap", GripCap), ("rotation", r), ("phase", Phase)],
                )
                .only(Destination::Draft),
            );
        }
    }
    specs
}
