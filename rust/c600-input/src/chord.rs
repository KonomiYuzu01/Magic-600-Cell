//! Physical chords (EV4.7; DT-M1 Outcome K5): a physical key code with its modifiers, never a label.
//!
//! Key codes take the W3C UI Events `code` names (`KeyQ`, `Digit7`, `Backquote`, `ArrowUp`), the notation 0.4 used
//! for most of its key entry points. A modifier key is not a key here; it belongs to a chord's [`Mods`].

use std::fmt;

/// The keys that type a character on some layout: letters, digits, punctuation and Space.
const CHARACTER_KEYS: [&str; 49] = [
    "KeyA",
    "KeyB",
    "KeyC",
    "KeyD",
    "KeyE",
    "KeyF",
    "KeyG",
    "KeyH",
    "KeyI",
    "KeyJ",
    "KeyK",
    "KeyL",
    "KeyM",
    "KeyN",
    "KeyO",
    "KeyP",
    "KeyQ",
    "KeyR",
    "KeyS",
    "KeyT",
    "KeyU",
    "KeyV",
    "KeyW",
    "KeyX",
    "KeyY",
    "KeyZ",
    "Digit0",
    "Digit1",
    "Digit2",
    "Digit3",
    "Digit4",
    "Digit5",
    "Digit6",
    "Digit7",
    "Digit8",
    "Digit9",
    "Minus",
    "Equal",
    "BracketLeft",
    "BracketRight",
    "Backslash",
    "Semicolon",
    "Quote",
    "Backquote",
    "Comma",
    "Period",
    "Slash",
    "IntlBackslash",
    "Space",
];

/// The named keys: editing, navigation and function keys.
const NAMED_KEYS: [&str; 27] = [
    "Enter",
    "Escape",
    "Tab",
    "Backspace",
    "Delete",
    "Insert",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "ContextMenu",
];

/// A physical key code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(&'static str);

impl Key {
    pub const ENTER: Key = Key("Enter");
    pub const ESCAPE: Key = Key("Escape");
    pub const TAB: Key = Key("Tab");

    /// The key with this `code` name, if it is one this crate knows.
    pub fn parse(code: &str) -> Option<Key> {
        CHARACTER_KEYS.iter().chain(NAMED_KEYS.iter()).find(|known| **known == code).map(|known| Key(known))
    }

    pub fn code(self) -> &'static str {
        self.0
    }

    /// Whether the key types a character on some layout.
    pub fn types_character(self) -> bool {
        CHARACTER_KEYS.contains(&self.0)
    }

    pub fn is_arrow(self) -> bool {
        self.0.starts_with("Arrow")
    }

    /// Whether the key edits or moves within text: the arrows, Backspace, Delete, Insert, Home, End, PageUp and
    /// PageDown. Enter is not one of them.
    pub fn is_editing(self) -> bool {
        self.is_arrow() || matches!(self.0, "Backspace" | "Delete" | "Insert" | "Home" | "End" | "PageUp" | "PageDown")
    }

    /// The digit of a `Digit0` to `Digit9` key.
    pub fn digit(self) -> Option<u32> {
        self.0.strip_prefix("Digit").and_then(|d| d.parse().ok())
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// The modifiers held with a key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mods {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

impl Mods {
    pub const NONE: Mods = Mods { ctrl: false, shift: false, alt: false, meta: false };

    /// Whether a modifier other than Shift is held, so that the key does not simply type its character.
    pub fn command(self) -> bool {
        self.ctrl || self.alt || self.meta
    }

    pub fn union(self, other: Mods) -> Mods {
        Mods {
            ctrl: self.ctrl || other.ctrl,
            shift: self.shift || other.shift,
            alt: self.alt || other.alt,
            meta: self.meta || other.meta,
        }
    }

    /// Parses a `+`-separated modifier list such as `Ctrl+Shift`; the empty text is no modifier.
    pub fn parse(text: &str) -> Option<Mods> {
        let mut mods = Mods::NONE;
        for name in text.split('+').filter(|name| !name.is_empty()) {
            let flag = match name {
                "Ctrl" => &mut mods.ctrl,
                "Shift" => &mut mods.shift,
                "Alt" => &mut mods.alt,
                "Meta" => &mut mods.meta,
                _ => return None,
            };
            if *flag {
                return None;
            }
            *flag = true;
        }
        Some(mods)
    }
}

impl fmt::Display for Mods {
    /// The modifiers in the fixed order Ctrl, Shift, Alt, Meta, each followed by `+`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (held, name) in [(self.ctrl, "Ctrl"), (self.shift, "Shift"), (self.alt, "Alt"), (self.meta, "Meta")] {
            if held {
                write!(f, "{name}+")?;
            }
        }
        Ok(())
    }
}

/// A physical chord: modifiers and one physical key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Chord {
    pub mods: Mods,
    pub key: Key,
}

impl Chord {
    pub fn new(mods: Mods, key: Key) -> Chord {
        Chord { mods, key }
    }

    /// Parses `Ctrl+Shift+Enter`, `KeyQ` or `Alt+ArrowRight`: modifiers in any order, then one key code.
    pub fn parse(text: &str) -> Option<Chord> {
        let (mods, key) = match text.rfind('+') {
            Some(at) => (Mods::parse(&text[..at])?, &text[at + 1..]),
            None => (Mods::NONE, text),
        };
        Some(Chord { mods, key: Key::parse(key)? })
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.mods, self.key)
    }
}
