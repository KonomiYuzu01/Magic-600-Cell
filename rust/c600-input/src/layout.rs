//! Declared layouts, for the one layout-dependent intent the key map keeps (EV4.7; DT-M1 Q2 (b)).
//!
//! A binding stores physical key codes. Undo and redo keep their office characters (`Ctrl+Z`, `Ctrl+Y`) as an intent
//! recorded with its layout, which a [`Layout`] resolves to a physical chord before the map is applied. A layout here
//! is a table of the characters each key types with no modifier and with Shift; dead keys and AltGr levels are left
//! out. The platform layer, not this crate, reads labels and handles IME.

use crate::chord::{Chord, Key, Mods};

/// A declared keyboard layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    id: &'static str,
    base: &'static [(&'static str, char)],
    shifted: &'static [(&'static str, char)],
}

const US_BASE: &[(&str, char)] = &[
    ("KeyA", 'a'),
    ("KeyB", 'b'),
    ("KeyC", 'c'),
    ("KeyD", 'd'),
    ("KeyE", 'e'),
    ("KeyF", 'f'),
    ("KeyG", 'g'),
    ("KeyH", 'h'),
    ("KeyI", 'i'),
    ("KeyJ", 'j'),
    ("KeyK", 'k'),
    ("KeyL", 'l'),
    ("KeyM", 'm'),
    ("KeyN", 'n'),
    ("KeyO", 'o'),
    ("KeyP", 'p'),
    ("KeyQ", 'q'),
    ("KeyR", 'r'),
    ("KeyS", 's'),
    ("KeyT", 't'),
    ("KeyU", 'u'),
    ("KeyV", 'v'),
    ("KeyW", 'w'),
    ("KeyX", 'x'),
    ("KeyY", 'y'),
    ("KeyZ", 'z'),
    ("Digit1", '1'),
    ("Digit2", '2'),
    ("Digit3", '3'),
    ("Digit4", '4'),
    ("Digit5", '5'),
    ("Digit6", '6'),
    ("Digit7", '7'),
    ("Digit8", '8'),
    ("Digit9", '9'),
    ("Digit0", '0'),
    ("Minus", '-'),
    ("Equal", '='),
    ("BracketLeft", '['),
    ("BracketRight", ']'),
    ("Backslash", '\\'),
    ("Semicolon", ';'),
    ("Quote", '\''),
    ("Backquote", '`'),
    ("Comma", ','),
    ("Period", '.'),
    ("Slash", '/'),
    ("Space", ' '),
];

const US_SHIFTED: &[(&str, char)] = &[
    ("Digit1", '!'),
    ("Digit2", '@'),
    ("Digit3", '#'),
    ("Digit4", '$'),
    ("Digit5", '%'),
    ("Digit6", '^'),
    ("Digit7", '&'),
    ("Digit8", '*'),
    ("Digit9", '('),
    ("Digit0", ')'),
    ("Minus", '_'),
    ("Equal", '+'),
    ("BracketLeft", '{'),
    ("BracketRight", '}'),
    ("Backslash", '|'),
    ("Semicolon", ':'),
    ("Quote", '"'),
    ("Backquote", '~'),
    ("Comma", '<'),
    ("Period", '>'),
    ("Slash", '?'),
];

/// French AZERTY: A and Q, Z and W swap places, M sits right of L, and the unshifted top row types
/// `& é " ' ( - è _ ç à`, so a digit needs Shift (DT-M1 M10).
const FRENCH_BASE: &[(&str, char)] = &[
    ("KeyQ", 'a'),
    ("KeyB", 'b'),
    ("KeyC", 'c'),
    ("KeyD", 'd'),
    ("KeyE", 'e'),
    ("KeyF", 'f'),
    ("KeyG", 'g'),
    ("KeyH", 'h'),
    ("KeyI", 'i'),
    ("KeyJ", 'j'),
    ("KeyK", 'k'),
    ("KeyL", 'l'),
    ("Semicolon", 'm'),
    ("KeyN", 'n'),
    ("KeyO", 'o'),
    ("KeyP", 'p'),
    ("KeyA", 'q'),
    ("KeyR", 'r'),
    ("KeyS", 's'),
    ("KeyT", 't'),
    ("KeyU", 'u'),
    ("KeyV", 'v'),
    ("KeyZ", 'w'),
    ("KeyX", 'x'),
    ("KeyY", 'y'),
    ("KeyW", 'z'),
    ("Digit1", '&'),
    ("Digit2", 'é'),
    ("Digit3", '"'),
    ("Digit4", '\''),
    ("Digit5", '('),
    ("Digit6", '-'),
    ("Digit7", 'è'),
    ("Digit8", '_'),
    ("Digit9", 'ç'),
    ("Digit0", 'à'),
    ("Minus", ')'),
    ("Equal", '='),
    ("BracketRight", '$'),
    ("Quote", 'ù'),
    ("Backquote", '²'),
    ("Backslash", '*'),
    ("KeyM", ','),
    ("Comma", ';'),
    ("Period", ':'),
    ("Slash", '!'),
    ("IntlBackslash", '<'),
    ("Space", ' '),
];

const FRENCH_SHIFTED: &[(&str, char)] = &[
    ("Digit1", '1'),
    ("Digit2", '2'),
    ("Digit3", '3'),
    ("Digit4", '4'),
    ("Digit5", '5'),
    ("Digit6", '6'),
    ("Digit7", '7'),
    ("Digit8", '8'),
    ("Digit9", '9'),
    ("Digit0", '0'),
    ("Minus", '°'),
    ("Equal", '+'),
    ("BracketRight", '£'),
    ("Quote", '%'),
    ("Backslash", 'µ'),
    ("KeyM", '?'),
    ("Comma", '.'),
    ("Period", '/'),
    ("Slash", '§'),
    ("IntlBackslash", '>'),
];

impl Layout {
    /// US QWERTY.
    pub const US: Layout = Layout { id: "us-qwerty", base: US_BASE, shifted: US_SHIFTED };
    /// French AZERTY.
    pub const FRENCH: Layout = Layout { id: "fr-azerty", base: FRENCH_BASE, shifted: FRENCH_SHIFTED };

    pub fn id(&self) -> &'static str {
        self.id
    }

    /// The character the key types with no modifier, if this table has one.
    pub fn base(&self, key: Key) -> Option<char> {
        self.base.iter().find(|(code, _)| *code == key.code()).map(|&(_, c)| c)
    }

    /// The physical chord that types `character` with `mods` held: the key whose unshifted character it is, or the key
    /// whose shifted character it is with Shift added. A capital letter is its small letter with Shift.
    pub fn resolve(&self, mods: Mods, character: char) -> Option<Chord> {
        let shift = Mods { shift: true, ..Mods::NONE };
        let (mods, character) = match character.is_ascii_uppercase() {
            true => (mods.union(shift), character.to_ascii_lowercase()),
            false => (mods, character),
        };
        let find = |table: &[(&'static str, char)]| table.iter().find(|&&(_, c)| c == character).map(|&(code, _)| code);
        if let Some(code) = find(self.base) {
            return Some(Chord::new(mods, Key::parse(code)?));
        }
        let code = find(self.shifted)?;
        Some(Chord::new(mods.union(shift), Key::parse(code)?))
    }
}
