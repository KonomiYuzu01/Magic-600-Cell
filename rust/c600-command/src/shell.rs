//! The shell's half of the user-action path: command IDs, tokens, the presented preview and the key route.
//!
//! The shell holds the per-launch secret and mints one token per press (KN5.14). It resolves a key binding to a row and
//! absolute arguments before the envelope (EV4.3, EV4.4), so a key bound to a kernel row reaches the engine without
//! any interpreter step (ONE.48): this crate has no interpreter. A held key's auto-repeat reaches only rows whose
//! `repeat` flag is set, so a held commit chord commits once (R-18, KN5.17). A word commit's token binds the preview
//! the shell last presented (ONE.41), which may differ from the engine's pending one (CT-ONE-04).
//!
//! The keymap is a draft: G6 has no keymap table yet. It follows 0.4 where 0.4 has a key (`Ctrl+Enter`,
//! `Ctrl+Shift+Enter`) and uses `Ctrl+Z`, `Ctrl+Y` and `Escape` for undo, redo and cancel.

use c600_kernel::json::{self, Value};

use crate::layer::{Caller, CommandLayer, Invocation};
use crate::receipt::{code, ActionHint, Direction, Error, Receipt, Status};
use crate::store::Store;
use crate::table::{Kind, PreviewRule, Row};
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
    keymap: Vec<KeyBinding>,
    /// The focused context.
    pub context: String,
    /// The editor's draft, `{"recipe": [...]}`.
    pub draft: Option<Value>,
    presented: Option<PreviewBinding>,
    /// The command ID of every invocation, in order.
    pub sent: Vec<String>,
}

impl Shell {
    /// A shell with the launch's secret. `prefix` starts its command IDs, which are unique per launch.
    pub fn new(secret: Secret, prefix: &str) -> Shell {
        Shell {
            secret,
            prefix: prefix.to_owned(),
            nonce: 0,
            commands: 0,
            keymap: default_keymap(),
            context: "operation".to_owned(),
            draft: None,
            presented: None,
            sent: Vec::new(),
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

    /// A fresh command ID.
    pub fn command_id(&mut self) -> String {
        self.commands += 1;
        format!("{}/{}", self.prefix, self.commands)
    }

    /// Presents the engine's pending preview, as the built-in chain's frame report does (RM.26).
    pub fn present<S: Store>(&mut self, layer: &mut CommandLayer<S>, id: u64) -> Result<(), Error> {
        let p = layer.present(id)?;
        self.presented = Some(PreviewBinding { id: p.id, literal: p.literal.clone(), deps: p.deps.clone() });
        Ok(())
    }

    /// Step 2: mints the token of one press, or none for a row that takes none.
    fn mint<S: Store>(&mut self, layer: &CommandLayer<S>, command: &str, row: &Row, args: &Value) -> Option<Token> {
        if !row.takes_token() {
            return None;
        }
        self.nonce += 1;
        let revision = layer.revision();
        let preview = if row.preview == PreviewRule::Required { self.presented.clone() } else { None };
        let hint = args.get("hint").and_then(ActionHint::from_json);
        let plain_commit = row.kind == Kind::Commit && row.preview == PreviewRule::None && row.action_guard.is_none();
        let binding = TokenBinding {
            nonce: self.nonce,
            session: revision.session.clone(),
            epoch: revision.epoch,
            command: command.to_owned(),
            row: row.id.clone(),
            args: args_digest(args),
            identity: revision.identity.clone(),
            preview,
            revision: plain_commit.then_some((revision.state, revision.protection)),
            hint,
        };
        Some(Token::mint(&self.secret, binding))
    }

    /// Builds one press's invocation of `row` with absolute arguments, minting a token when the row takes one, without
    /// sending it. A word commit with no presented preview stops at the shell's gate (step 1).
    pub fn prepare<S: Store>(&mut self, layer: &CommandLayer<S>, row: &str, args: Value) -> Result<Invocation, Error> {
        let Some(found) = layer.table().row(row).cloned() else {
            return Err(Error::new(code::UNKNOWN_ROW, format!("no row `{row}`")));
        };
        if found.preview == PreviewRule::Required && self.presented.is_none() {
            return Err(Error::new(code::NO_FRESH_PREVIEW, "no presented preview"));
        }
        let command = self.command_id();
        let token = self.mint(layer, &command, &found, &args);
        self.sent.push(command.clone());
        let epoch = layer.revision().epoch;
        Ok(Invocation { command, row: row.to_owned(), args, token, caller: Caller::Shell, epoch })
    }

    /// Sends a prepared invocation. A commit or a completed cancel ends the presented preview.
    pub fn send<S: Store>(&mut self, layer: &mut CommandLayer<S>, invocation: Invocation) -> Receipt {
        let cancel = invocation.row == "operation.preview.cancel";
        let receipt = layer.invoke(invocation);
        if matches!(receipt.status, Status::Committed(_)) || (cancel && matches!(receipt.status, Status::Completed(_)))
        {
            self.presented = None;
        }
        receipt
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

/// The draft form of an inline recipe, `{"recipe": [...]}` (a draft `WordRef`).
pub fn word_draft(recipe: &Value) -> Value {
    json::object([("recipe", recipe.clone())])
}

/// The absolute arguments of `operation.preview` for an inline recipe, `{"draft": {"recipe": [...]}}`.
pub fn draft_args(recipe: &Value) -> Value {
    json::object([("draft", word_draft(recipe))])
}
