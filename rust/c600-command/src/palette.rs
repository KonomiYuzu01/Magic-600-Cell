//! The palette's data (EV4.2, KN5.6; DT-M1 C3; ADR-D01.18; LP4-4).
//!
//! The palette lists every command of the table: H-08's stored rows and the language overlay rows that checked modules
//! registered (KN5.3, EV4.1). Each entry gives the row's ID, label and kind, whether a language module defined it, the
//! key sequences the shell's keymap binds to it, and its answer in KN5.6's availability table.
//!
//! - [`Palette::read`] reads the table and its availability at one moment, as one [`Availability`] that the layer
//!   computes from its stored rows, the overlay rows' stored metadata and its own state, with no language code running
//!   and no program started (KN5.5, KN5.7). The shell reads it once per revision and generation (KN5.25;
//!   [`crate::Shell::read_palette`]).
//! - [`Palette::find`] answers a search from that read alone: it takes no layer, so a keystroke costs no engine call
//!   (EV4.2).
//! - An unavailable entry keeps its place in the entries and in the matches, with its typed reason (KN5.6).
//! - Each answer is the row's argument-independent gates ([`Binding::Computed`]): whether the command can run now. Like
//!   every hint it grants nothing. An invocation from the palette goes through [`crate::Shell::invoke`], the path a key
//!   press takes once it has resolved its binding, so the same gates and preview rule apply (EV4.2).
//!
//! Drafts, since EV4.2 leaves the match rule open:
//! - A query matches an entry when each of its whitespace-separated words is a substring, ignoring ASCII case, of the
//!   entry's label, its row ID or one of its key sequences. An empty query matches every entry.
//! - Matches keep the table's order, by row ID. Nothing ranks them. ADR-D01.18's ranking by the solver's own recency,
//!   frequency and pins, the glossary terms of EV4.2's search, aliases (`palette.alias`) and the selection filter
//!   (`command.find`'s `object`) are not built.

use crate::availability::Availability;
use crate::keys::KeyMap;
use crate::layer::CommandLayer;
use crate::receipt::Revision;
use crate::store::Store;
use crate::table::{Binding, Kind};

/// H-08's row that opens the palette over a query (EV4.2; F1 in 0.4). The layer admits it and changes nothing
/// ([`CommandLayer::invoke`]); the palette's data is the shell's read.
pub const FIND_ROW: &str = "command.find";

/// One command in the palette.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteEntry {
    /// H-08's stable dotted row ID.
    pub row: String,
    /// The row's label, or its ID when it has none.
    pub label: String,
    pub kind: Kind,
    /// Whether a checked language module registered the row (KN5.3); a stored row that an entry serves is not one.
    pub language_defined: bool,
    /// The key sequences the shell's key map binds to the row, in key map order, as resolved under its active layout:
    /// each chord in [`crate::keys::Chord`]'s form, chords of a prefix group separated by a space. They are shown
    /// before disclosure (EV4.2).
    pub keys: Vec<String>,
    /// The row's answer in the availability table the palette read: `Ok(())`, or the typed reason it is unavailable.
    pub availability: Result<(), &'static str>,
}

impl PaletteEntry {
    /// Whether `word`, already in ASCII lower case, is a substring of the label, the row ID or a key sequence.
    fn matches(&self, word: &str) -> bool {
        let found = |text: &str| text.to_ascii_lowercase().contains(word);
        found(&self.label) || found(&self.row) || self.keys.iter().any(|key| found(key))
    }
}

/// The palette read at one revision and generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette {
    /// The revision the availability table was computed at.
    pub revision: Revision,
    /// KN5.25's generation at the same moment.
    pub generation: u64,
    /// The availability table's digest of the context and effective grants.
    pub context: String,
    entries: Vec<PaletteEntry>,
}

impl Palette {
    /// Reads every row of `layer`'s table with its answer in one availability table
    /// ([`CommandLayer::availability_table`], a computed binding per row) and the key sequences `keymap` binds to it.
    /// It starts no program, queues no job and changes nothing in the layer.
    pub fn read<S: Store>(layer: &CommandLayer<S>, keymap: &KeyMap) -> Palette {
        let computed = Binding::Computed;
        let table = layer.table();
        let availability = layer.availability_table(table.rows().map(|row| (row.id.as_str(), &computed)));
        let entries = table
            .rows()
            .map(|row| PaletteEntry {
                row: row.id.clone(),
                label: row.label.clone().unwrap_or_else(|| row.id.clone()),
                kind: row.kind,
                language_defined: table.language_defined(&row.id),
                keys: keymap
                    .bindings()
                    .iter()
                    .filter(|binding| binding.command == row.id)
                    .map(|binding| binding.sequence.iter().map(ToString::to_string).collect::<Vec<_>>().join(" "))
                    .collect(),
                availability: availability.answer(&row.id, &computed).expect("the table answers every row asked"),
            })
            .collect();
        let Availability { revision, generation, context, .. } = availability;
        Palette { revision, generation, context, entries }
    }

    /// Whether this read is still current for `layer`: the same revision and generation (KN5.25).
    pub fn current<S: Store>(&self, layer: &CommandLayer<S>) -> bool {
        self.generation == layer.generation() && self.revision == *layer.revision()
    }

    /// Every entry, in the table's order.
    pub fn entries(&self) -> &[PaletteEntry] {
        &self.entries
    }

    /// The entry of `row`, if the table held it at the read.
    pub fn entry(&self, row: &str) -> Option<&PaletteEntry> {
        self.entries.iter().find(|entry| entry.row == row)
    }

    /// The entries that match `query`, in the table's order, unavailable ones in their place. It reads this palette
    /// alone.
    pub fn find(&self, query: &str) -> Vec<&PaletteEntry> {
        let words: Vec<String> = query.split_whitespace().map(str::to_ascii_lowercase).collect();
        self.entries.iter().filter(|entry| words.iter().all(|word| entry.matches(word))).collect()
    }
}
