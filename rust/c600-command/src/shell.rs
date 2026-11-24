//! The shell's half of the user-action path: command IDs, tokens, the presented preview and the key route.
//!
//! The shell holds the per-launch secret and mints one token per press (KN5.14). It resolves a key binding to a row and
//! absolute arguments before the envelope (EV4.3, EV4.4), so a key bound to a kernel row reaches the engine without
//! any interpreter step (ONE.48): this crate has no interpreter. A held key's auto-repeat reaches only rows whose
//! `repeat` flag is set, so a held commit chord commits once (R-18, KN5.17). A word commit's token binds the preview
//! of the newest frame that presented one, with that frame's stamp (ONE.41), which may differ from the engine's pending
//! preview (CT-ONE-04).
//!
//! After a command the shell reports its receipt and the display's refresh apart (KN5.22, R-02): a command that
//! applied is never reported as failed, and when the display's refresh fails the report says "Applied, display out of
//! date" ([`report`]). The display itself is the host's ([`Refresh`]).
//!
//! The key map is [`crate::keys`]' checked map, the one the input router of `c600-input` uses too (LP4-8): the draft
//! default map ([`crate::keys::default_specs`]) under the draft active layout, US QWERTY, declared with French AZERTY.
//! [`Shell::bind`] adds a binding, such as one of a checked user command, only after EV4.7's checks of the whole map
//! pass in each routing class and under every declared layout. The shell's key route ([`Shell::press`]) resolves a
//! sequence in the global routing class; the router adds the focus model, the text field and the held keys.
//!
//! The shell keeps the palette it read last ([`Shell::read_palette`], [`crate::palette`]): one read per revision and
//! generation, and a search of the kept palette for each keystroke, with no engine call (EV4.2, KN5.6).

use c600_kernel::json::{self, Value};

use crate::budget::Budget;
use crate::disclosure::Disclosure;
use crate::frame::{PresentedFrame, ShownPreview};
use crate::keys::{resolve_binding, Arg, BindingSpec, ChordIntent, Destination, KeyMap, Resolution, RoutingClass};
use crate::keys::{RoutingContext, DRAFT_LAYOUT};
use crate::layer::{Caller, CommandLayer, Invocation};
use crate::palette::Palette;
use crate::receipt::{code, ActionHint, CompletionKind, Direction, Error, Receipt, Revision, Status};
use crate::store::Store;
use crate::table::{PreviewRule, Row};
use crate::token::{args_digest, PreviewBinding, Secret, Token, TokenBinding};

/// The shell of one launch.
pub struct Shell {
    secret: Secret,
    prefix: String,
    nonce: u64,
    commands: u64,
    /// The engine epoch of the last envelope, which names the shell's command IDs after the first epoch (L10).
    epoch: u64,
    keymap: KeyMap,
    /// The input destination the key route resolves for (EV4.4; DT-M1 C4): twist keys twist the puzzle when it is
    /// live and append to the draft when it is the draft.
    pub destination: Destination,
    /// The editor's draft, `{"recipe": [...]}`.
    pub draft: Option<Value>,
    presented: Option<PreviewBinding>,
    /// The command ID of every invocation, in order.
    pub sent: Vec<String>,
    /// The budget the shell declares on an invocation of a row an entry serves ([`crate::budget`]; draft).
    pub budget: Option<Budget>,
    /// The palette read last ([`Shell::read_palette`]).
    palette: Option<Palette>,
    /// Which keys the keyboard and the help show as open (EV5.8; draft). The command layer never reads it, so no
    /// disclosure state locks a command ([`crate::disclosure`]).
    pub disclosure: Disclosure,
}

impl Shell {
    /// A shell with the launch's secret. `prefix` starts its command IDs, which are unique per launch.
    pub fn new(secret: Secret, prefix: &str) -> Shell {
        Shell {
            secret,
            prefix: prefix.to_owned(),
            nonce: 0,
            commands: 0,
            epoch: 1,
            keymap: KeyMap::default_for(DRAFT_LAYOUT).expect("the draft default map passes EV4.7's checks"),
            destination: Destination::Live,
            draft: None,
            presented: None,
            sent: Vec::new(),
            budget: Some(Budget::CEILING),
            palette: None,
            disclosure: Disclosure::new(),
        }
    }

    /// Replaces the secret after a restart; every earlier token stays invalid, and the palette is read again.
    pub fn restart(&mut self, secret: Secret) {
        self.secret = secret;
        self.presented = None;
        self.palette = None;
    }

    /// The checked key map ([`crate::keys::KeyMap`]).
    pub fn keymap(&self) -> &KeyMap {
        &self.keymap
    }

    /// Adds a binding of a key sequence in a routing class to a row with its arguments (EV4.3), so that a checked user
    /// command is invoked by key as well as from the palette. The whole map with the new binding is checked again
    /// (EV4.7; [`KeyMap::bind`]): a duplicate or prefix conflict in the binding's routing class, or a layout conflict
    /// under any declared layout, refuses it with the first finding's code (`EV4:input.binding_conflict`,
    /// `EV4:input.binding_prefix_conflict` or `EV4:input.binding_layout_mismatch`), naming every finding, and the map
    /// stays as it was. The palette shows each row's key sequences, so a binding makes a new read due.
    pub fn bind(&mut self, spec: BindingSpec) -> Result<(), Error> {
        self.keymap = self.keymap.bind(spec).map_err(|findings| {
            let detail: Vec<String> = findings.iter().map(|finding| finding.detail.clone()).collect();
            Error::new(findings[0].code, detail.join("; "))
        })?;
        self.palette = None;
        Ok(())
    }

    /// Replaces the key map with `keymap`, a map that passed EV4.7's checks when it was applied, such as the router's
    /// ([`KeyMap::apply`]). The palette is read again.
    pub fn apply_keymap(&mut self, keymap: KeyMap) {
        self.keymap = keymap;
        self.palette = None;
    }

    /// The palette (EV4.2, KN5.6): the kept read while it is current for `layer`, at the same revision and generation
    /// (KN5.25), and otherwise a new read ([`Palette::read`]). A search of it ([`Palette::find`]) reads it alone, so a
    /// keystroke costs no engine call.
    pub fn read_palette<S: Store>(&mut self, layer: &CommandLayer<S>) -> &Palette {
        if !self.palette.as_ref().is_some_and(|kept| kept.current(layer)) {
            self.palette = Some(Palette::read(layer, &self.keymap));
        }
        self.palette.as_ref().expect("a palette was kept or read")
    }

    /// The palette read last, which may be older than the layer's revision or generation.
    pub fn palette(&self) -> Option<&Palette> {
        self.palette.as_ref()
    }

    /// The preview the shell presented last.
    pub fn presented(&self) -> Option<&PreviewBinding> {
        self.presented.as_ref()
    }

    /// A fresh command ID: `prefix/n` in a session's first epoch, `prefix/epoch/n` after it, so a shell's IDs never
    /// repeat across a restart or a reopen (draft; L10).
    pub fn command_id(&mut self) -> String {
        self.commands += 1;
        match self.epoch {
            0 | 1 => format!("{}/{}", self.prefix, self.commands),
            epoch => format!("{}/{epoch}/{}", self.prefix, self.commands),
        }
    }

    /// Reports a presented frame to the layer ([`CommandLayer::present`], RM.26) and binds the preview it presents,
    /// with the frame's stamp (ONE.41). When the layer refuses the report, the shell keeps its binding only while the
    /// newest frame still presents that preview through the built-in chain, now with that frame's stamp, so a token may
    /// still name a preview the engine has replaced since (CT-ONE-04); otherwise the binding ends.
    pub fn present<S: Store>(
        &mut self,
        layer: &mut CommandLayer<S>,
        id: u64,
        frame: &PresentedFrame,
    ) -> Result<(), Error> {
        let answer = layer.present(id, frame).map(|p| PreviewBinding {
            id: p.id,
            literal: p.literal.clone(),
            deps: p.deps.clone(),
            frame: frame.stamp.clone(),
        });
        match answer {
            Ok(binding) => {
                self.presented = Some(binding);
                Ok(())
            }
            Err(error) => {
                let newest = layer.frame();
                match (self.presented.as_mut(), newest) {
                    (Some(bound), Some(newest)) if newest.presents(&ShownPreview::of_binding(bound)) => {
                        bound.frame = newest.stamp.clone();
                    }
                    _ => self.presented = None,
                }
                Err(error)
            }
        }
    }

    /// Reports to the layer that the adapter cannot confirm presentation ([`CommandLayer::presentation_unavailable`],
    /// RM.26), and ends the shell's preview binding, so no token names a preview while presentation is unavailable.
    pub fn presentation_unavailable<S: Store>(&mut self, layer: &mut CommandLayer<S>) {
        layer.presentation_unavailable();
        self.presented = None;
    }

    /// Step 2: mints the token of one press, or none for a row that takes none.
    fn mint<S: Store>(&mut self, layer: &CommandLayer<S>, command: &str, row: &Row, args: &Value) -> Option<Token> {
        if !row.takes_token() {
            return None;
        }
        self.nonce += 1;
        let revision = layer.revision();
        let required = row.preview == PreviewRule::Required;
        let preview = if required { self.presented.clone() } else { None };
        let hint = args.get("hint").and_then(ActionHint::from_json);
        let binding = TokenBinding {
            nonce: self.nonce,
            session: revision.session.clone(),
            epoch: revision.epoch,
            command: command.to_owned(),
            row: row.id.clone(),
            args: args_digest(args),
            identity: layer.identity_binding(),
            preview,
            revision: (!required).then_some((revision.state, revision.protection)),
            hint,
        };
        Some(Token::mint(&self.secret, binding))
    }

    /// Builds one press's invocation of `row` with absolute arguments, minting a token when the row takes one, without
    /// sending it. A word commit with no presented preview stops at the shell's gate (step 1). An invocation of a row
    /// an entry serves declares the shell's budget ([`Shell::budget`]: every profile's ceiling, [`Budget::CEILING`],
    /// by default; `None` declares none); an invocation of another row declares none.
    pub fn prepare<S: Store>(&mut self, layer: &CommandLayer<S>, row: &str, args: Value) -> Result<Invocation, Error> {
        let Some(found) = layer.table().row(row).cloned() else {
            return Err(Error::new(code::UNKNOWN_ROW, format!("no row `{row}`")));
        };
        if found.preview == PreviewRule::Required && self.presented.is_none() {
            return Err(Error::new(code::NO_FRESH_PREVIEW, "no presented preview"));
        }
        self.epoch = layer.revision().epoch;
        let command = self.command_id();
        let token = self.mint(layer, &command, &found, &args);
        self.sent.push(command.clone());
        let epoch = layer.revision().epoch;
        let budget = if found.overlay.is_some() { self.budget } else { None };
        Ok(Invocation { command, row: row.to_owned(), args, token, epoch, budget })
    }

    /// Sends a prepared invocation on the shell's channel, which stamps [`Caller::Shell`] (ruling 4). A commit that moves
    /// the state, or a completed cancel, ends the presented preview. A committed receipt that keeps the state, such as a
    /// record write's undo or redo (a workspace inverse), keeps it, as the layer keeps the preview (draft).
    pub fn send<S: Store>(&mut self, layer: &mut CommandLayer<S>, invocation: Invocation) -> Receipt {
        let cancel = invocation.row == "operation.preview.cancel";
        let state = layer.revision().state;
        let receipt = layer.invoke(&Caller::Shell, invocation);
        let moved = matches!(receipt.status, Status::Committed(_)) && layer.revision().state != state;
        if moved || (cancel && matches!(receipt.status, Status::Completed(_))) {
            self.presented = None;
        }
        receipt
    }

    /// [`Shell::send`], then the report with the display's status ([`report`]).
    pub fn send_reported<S: Store>(
        &mut self,
        layer: &mut CommandLayer<S>,
        invocation: Invocation,
        display: &mut impl Refresh,
    ) -> Report {
        let receipt = self.send(layer, invocation);
        report(layer, receipt, display)
    }

    /// The command route: [`Shell::prepare`], then [`Shell::send`].
    pub fn invoke<S: Store>(&mut self, layer: &mut CommandLayer<S>, row: &str, args: Value) -> Result<Receipt, Error> {
        let invocation = self.prepare(layer, row, args)?;
        Ok(self.send(layer, invocation))
    }

    /// EV4.20's step action: plays the user step that the playback of `recording` waits at as the solver's own press.
    /// The shell takes the recorded row and arguments ([`CommandLayer::step_arguments`]), fills in what only the press
    /// fixes (the current hint of the step's history for an undo or redo, the presented preview for a word commit, the
    /// current revision for a `RevisionRef` argument, and `confirm` when the solver confirms, since a recording stores no
    /// confirmation), chooses the command ID and mints
    /// the token ([`Shell::prepare`]); the layer checks the step and writes `pending` ([`CommandLayer::begin_step`]);
    /// then the ordinary row is sent. A refusal before the send changes nothing but what `begin_step` says it stops.
    pub fn step<S: Store>(
        &mut self,
        layer: &mut CommandLayer<S>,
        recording: &str,
        confirm: bool,
    ) -> Result<Receipt, Error> {
        let (row, recorded) = layer.step_arguments(recording)?;
        let mut args = recorded.as_object().cloned().unwrap_or_default();
        let hint = |hint: Option<ActionHint>| {
            hint.map(|hint| hint.to_json()).ok_or_else(|| Error::new(code::UNDO_SUPERSEDED, "no eligible target"))
        };
        match row.as_str() {
            "session.undo" => args.insert("hint".to_owned(), hint(layer.undo_hint(Direction::Undo))?),
            "session.redo" => args.insert("hint".to_owned(), hint(layer.undo_hint(Direction::Redo))?),
            crate::layer::LAYOUT_UNDO_ROW => args.insert("hint".to_owned(), hint(layer.layout_hint(Direction::Undo))?),
            crate::layer::LAYOUT_REDO_ROW => args.insert("hint".to_owned(), hint(layer.layout_hint(Direction::Redo))?),
            "operation.execute" => match &self.presented {
                Some(p) => args.insert("preview".to_owned(), Value::Int(p.id as i64)),
                None => return Err(Error::new(code::NO_FRESH_PREVIEW, "no presented preview")),
            },
            _ => None,
        };
        if confirm {
            args.insert("confirm".to_owned(), Value::Bool(true));
        }
        // EV4.18: the current revision where the row names one, never the recorded one.
        if let Some(found) = layer.table().row(&row) {
            for (name, value) in args.iter_mut() {
                let ty = found.arguments.get(name).map(String::as_str).unwrap_or_default();
                if ty.strip_suffix('?').unwrap_or(ty) == "RevisionRef" {
                    *value = layer.revision().to_json();
                }
            }
        }
        let invocation = self.prepare(layer, &row, Value::Object(args))?;
        layer.begin_step(recording, &invocation)?;
        Ok(self.send(layer, invocation))
    }

    /// The fresh whole-word preview of the word step that the playback of `recording` waits at (EV4.18): an ordinary
    /// `operation.preview` of the stored word ([`CommandLayer::step_draft`]), never the recorded preview. It does not
    /// move the cursor; [`Shell::step`] commits it once it is presented.
    pub fn step_preview<S: Store>(&mut self, layer: &mut CommandLayer<S>, recording: &str) -> Result<Receipt, Error> {
        let draft = layer.step_draft(recording)?;
        self.invoke(layer, "operation.preview", json::object([("draft", draft)]))
    }

    /// The key route: resolves the binding of `sequence` in the global routing class for the shell's destination under
    /// the key map's active layout ([`resolve_binding`]), resolves its relative names at the press, then invokes.
    /// `sequence` is one or more chords separated by spaces, each a physical chord (`Ctrl+KeyZ`) or a character intent
    /// (`Ctrl+'z'`) that the active layout resolves. An auto-repeat (`repeat`) of a row without the `repeat` flag is
    /// dropped (`Ok(None)`). A sequence that binds nothing, or that is only a prefix group, is refused with
    /// `KN5:input.unknown_row`. The shell resolves the draft, the presented preview, the hints of both histories and the
    /// revision; a relative name that needs the router's focus, grip or menu (a list, an entry, a grip slot, a menu
    /// item, a phase or a scrub position) is refused with `KN5:input.argument_form` before any envelope.
    pub fn press<S: Store>(
        &mut self,
        layer: &mut CommandLayer<S>,
        sequence: &str,
        repeat: bool,
    ) -> Result<Option<Receipt>, Error> {
        let unbound = || Error::new(code::UNKNOWN_ROW, format!("`{sequence}` is not bound here"));
        let layout = *self.keymap.layout();
        let chords: Option<Vec<_>> = sequence
            .split_whitespace()
            .map(|text| ChordIntent::parse(text).and_then(|intent| intent.resolve(&layout)))
            .collect();
        let chords = chords.ok_or_else(unbound)?;
        let context = RoutingContext { class: RoutingClass::Global, destination: self.destination };
        let binding = match resolve_binding(&chords, context, self.keymap.bindings(), layout.id()) {
            Resolution::Bound(binding) => binding.clone(),
            Resolution::Prefix | Resolution::Unhandled => return Err(unbound()),
        };
        let row = layer.table().row(&binding.command).cloned();
        let row = row.ok_or_else(|| Error::new(code::UNKNOWN_ROW, format!("no row `{}`", binding.command)))?;
        if repeat && !row.repeat {
            return Ok(None);
        }
        let mut args = Vec::new();
        for (name, arg) in &binding.args {
            let hint = |hint: Option<ActionHint>| {
                hint.map(|hint| hint.to_json()).ok_or_else(|| Error::new(code::UNDO_SUPERSEDED, "no eligible target"))
            };
            let value = match arg {
                Arg::Fixed(value) => value.clone(),
                Arg::Draft => {
                    self.draft.clone().ok_or_else(|| Error::new(code::ARGUMENT_FORM, "the editor holds no draft"))?
                }
                Arg::PresentedPreview => match &self.presented {
                    Some(p) => Value::Int(p.id as i64),
                    None => return Err(Error::new(code::NO_FRESH_PREVIEW, "no presented preview")),
                },
                Arg::UndoHint => hint(layer.undo_hint(Direction::Undo))?,
                Arg::RedoHint => hint(layer.undo_hint(Direction::Redo))?,
                Arg::LayoutUndoHint => hint(layer.layout_hint(Direction::Undo))?,
                Arg::LayoutRedoHint => hint(layer.layout_hint(Direction::Redo))?,
                Arg::Revision => layer.revision().to_json(),
                other => {
                    let detail = format!("`{}`: the shell's key route resolves no {other:?}", binding.command);
                    return Err(Error::new(code::ARGUMENT_FORM, detail));
                }
            };
            args.push((name.clone(), value));
        }
        let args = Value::Object(args.into_iter().collect());
        self.invoke(layer, &binding.command, args).map(Some)
    }
}

/// R-02's message for a command that applied while the display's refresh failed (ONE.12).
pub const DISPLAY_OUT_OF_DATE: &str = "Applied, display out of date";

/// The display's side of the refresh after a command (KN5.22, R-02): the host adopts the engine's bundle of `revision`
/// into LP3's view model and presents it, and an error says why it could not. This crate depends on the kernel only,
/// so it names neither the view model nor the renderer.
pub trait Refresh {
    fn refresh(&mut self, revision: &Revision) -> Result<(), Error>;
}

/// The display's status after a command, which the shell reports apart from the command's receipt (KN5.22).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DisplayStatus {
    /// No refresh was due: the command did not apply ([`Report::applied`]).
    NotDue,
    /// The display adopted `revision`, the head.
    Current(Revision),
    /// The display's refresh to `revision`, the head, failed with `error`: "Applied, display out of date" (R-02). The
    /// command's receipt keeps its terminal success. The continuation refreshes the display ([`refresh_display`]) and
    /// never sends the command again.
    OutOfDate { revision: Revision, error: Error },
}

/// A command's report: its receipt as the layer returned it, and the display's status (KN5.22).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub receipt: Receipt,
    pub display: DisplayStatus,
}

impl Report {
    /// Whether the command applied: it committed, or completed as a workspace write, a view publication or a file
    /// publication (KN5.22). It follows the receipt alone, whatever the display, as 0.4's send route did.
    pub fn applied(&self) -> bool {
        applied(&self.receipt.status)
    }

    /// R-02's message, when the command applied and the display's refresh failed. It offers no retry (ONE.12).
    pub fn message(&self) -> Option<&'static str> {
        matches!(self.display, DisplayStatus::OutOfDate { .. }).then_some(DISPLAY_OUT_OF_DATE)
    }
}

/// Whether a receipt says that its command applied (KN5.22). A query's answer and a preview's creation change nothing
/// that a refresh adopts; a preview reaches the display through the frames ([`Shell::present`]).
fn applied(status: &Status) -> bool {
    match status {
        Status::Committed(_) => true,
        Status::Completed(completion) => {
            matches!(completion.kind, CompletionKind::Workspace | CompletionKind::ViewPublished | CompletionKind::File)
        }
        _ => false,
    }
}

/// Reports `receipt` with the display's status (KN5.22, R-02). When its command applied, the display refreshes to the
/// head ([`refresh_display`]); otherwise no refresh is due. A refresh failure never changes the receipt: a committed
/// command is never reported as failed, the layer's outcome lookup returns the same receipt, and the pending preview
/// and the jobs are as after any commit (ONE.70). `receipt` may come from any route: a send, a key press, or a program
/// job's terminal receipt.
pub fn report<S: Store>(layer: &CommandLayer<S>, receipt: Receipt, display: &mut impl Refresh) -> Report {
    let display = if applied(&receipt.status) { refresh_display(layer, display) } else { DisplayStatus::NotDue };
    Report { receipt, display }
}

/// Refreshes the display to the head: the continuation of "Applied, display out of date", as RPI.47's recovery reads
/// the current bundle. It sends no command, so it can never commit again (ONE.71).
pub fn refresh_display<S: Store>(layer: &CommandLayer<S>, display: &mut impl Refresh) -> DisplayStatus {
    let revision = layer.revision().clone();
    match display.refresh(&revision) {
        Ok(()) => DisplayStatus::Current(revision),
        Err(error) => DisplayStatus::OutOfDate { revision, error },
    }
}

/// The draft form of an inline recipe, `{"recipe": [...]}` (a draft `WordRef`).
pub fn word_draft(recipe: &Value) -> Value {
    json::object([("recipe", recipe.clone())])
}

/// The absolute arguments of `operation.preview` for an inline recipe, `{"draft": {"recipe": [...]}}`.
pub fn draft_args(recipe: &Value) -> Value {
    json::object([("draft", word_draft(recipe))])
}
