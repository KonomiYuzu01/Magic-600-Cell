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
//! The keymap is a draft: G6 has no keymap table yet. It follows 0.4 where 0.4 has a key (`Ctrl+Enter`,
//! `Ctrl+Shift+Enter`) and uses `Ctrl+Z`, `Ctrl+Y` and `Escape` for undo, redo and cancel.

use c600_kernel::json::{self, Value};

use crate::budget::Budget;
use crate::frame::{PresentedFrame, ShownPreview};
use crate::layer::{Caller, CommandLayer, Invocation};
use crate::receipt::{code, ActionHint, CompletionKind, Direction, Error, Receipt, Revision, Status};
use crate::store::Store;
use crate::table::{PreviewRule, Row};
use crate::token::{args_digest, PreviewBinding, Secret, Token, TokenBinding};

/// Where a bound argument comes from, resolved at the press.
#[derive(Clone, Debug, PartialEq)]
pub enum ArgSource {
    /// The editor's current draft.
    Draft,
    /// The ID of the preview the shell presented last.
    PresentedPreview,
    UndoHint,
    RedoHint,
    Fixed(Value),
}

/// One key binding of the draft keymap.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyBinding {
    pub sequence: String,
    /// `any` or a context name of H-08's vocabulary.
    pub context: String,
    pub row: String,
    pub args: Vec<(String, ArgSource)>,
}

impl KeyBinding {
    pub fn new(sequence: &str, context: &str, row: &str, args: &[(&str, ArgSource)]) -> KeyBinding {
        KeyBinding {
            sequence: sequence.to_owned(),
            context: context.to_owned(),
            row: row.to_owned(),
            args: args.iter().map(|(name, source)| ((*name).to_owned(), source.clone())).collect(),
        }
    }
}

/// The draft keymap.
pub fn default_keymap() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("Ctrl+Enter", "operation", "operation.preview", &[("draft", ArgSource::Draft)]),
        KeyBinding::new(
            "Ctrl+Shift+Enter",
            "operation",
            "operation.execute",
            &[("preview", ArgSource::PresentedPreview)],
        ),
        KeyBinding::new("Escape", "operation", "operation.preview.cancel", &[("preview", ArgSource::PresentedPreview)]),
        KeyBinding::new("Ctrl+Z", "any", "session.undo", &[("hint", ArgSource::UndoHint)]),
        KeyBinding::new("Ctrl+Y", "any", "session.redo", &[("hint", ArgSource::RedoHint)]),
    ]
}

/// The shell of one launch.
pub struct Shell {
    secret: Secret,
    prefix: String,
    nonce: u64,
    commands: u64,
    /// The engine epoch of the last envelope, which names the shell's command IDs after the first epoch (L10).
    epoch: u64,
    keymap: Vec<KeyBinding>,
    /// The focused context.
    pub context: String,
    /// The editor's draft, `{"recipe": [...]}`.
    pub draft: Option<Value>,
    presented: Option<PreviewBinding>,
    /// The command ID of every invocation, in order.
    pub sent: Vec<String>,
    /// The budget the shell declares on an invocation of a row an entry serves ([`crate::budget`]; draft).
    pub budget: Option<Budget>,
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
            keymap: default_keymap(),
            context: "operation".to_owned(),
            draft: None,
            presented: None,
            sent: Vec::new(),
            budget: Some(Budget::DRAFT_CEILING),
        }
    }

    /// Replaces the secret after a restart; every earlier token stays invalid.
    pub fn restart(&mut self, secret: Secret) {
        self.secret = secret;
        self.presented = None;
    }

    pub fn keymap(&self) -> &[KeyBinding] {
        &self.keymap
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
    /// an entry serves declares the shell's budget ([`Shell::budget`]: the draft ceiling until an owner reading sets
    /// the product's budgets; `None` declares none); an invocation of another row declares none.
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

    /// The key route: resolves the binding for `sequence` in the focused context and its arguments at the press, then
    /// invokes. An auto-repeat (`repeat`) of a row without the `repeat` flag is dropped (`Ok(None)`).
    pub fn press<S: Store>(
        &mut self,
        layer: &mut CommandLayer<S>,
        sequence: &str,
        repeat: bool,
    ) -> Result<Option<Receipt>, Error> {
        let binding = self
            .keymap
            .iter()
            .find(|b| b.sequence == sequence && (b.context == "any" || b.context == self.context))
            .cloned()
            .ok_or_else(|| Error::new(code::UNKNOWN_ROW, format!("`{sequence}` is not bound here")))?;
        let row = layer.table().row(&binding.row).cloned();
        let row = row.ok_or_else(|| Error::new(code::UNKNOWN_ROW, format!("no row `{}`", binding.row)))?;
        if repeat && !row.repeat {
            return Ok(None);
        }
        let mut args = Vec::new();
        for (name, source) in &binding.args {
            let value = match source {
                ArgSource::Draft => {
                    self.draft.clone().ok_or_else(|| Error::new(code::ARGUMENT_FORM, "the editor holds no draft"))?
                }
                ArgSource::PresentedPreview => match &self.presented {
                    Some(p) => Value::Int(p.id as i64),
                    None => return Err(Error::new(code::NO_FRESH_PREVIEW, "no presented preview")),
                },
                ArgSource::UndoHint | ArgSource::RedoHint => {
                    let direction = if *source == ArgSource::UndoHint { Direction::Undo } else { Direction::Redo };
                    let hint = layer.undo_hint(direction);
                    let hint = hint.ok_or_else(|| Error::new(code::UNDO_SUPERSEDED, "no eligible target"))?;
                    hint.to_json()
                }
                ArgSource::Fixed(value) => value.clone(),
            };
            args.push((name.clone(), value));
        }
        let args = Value::Object(args.into_iter().collect());
        self.invoke(layer, &binding.row, args).map(Some)
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
