//! The editor's buffer and the sample it is in, driven by lx4.py's messages (its docstring is the protocol): Reset
//! loads an edit's baseline, Select sets the selection in UTF-8 bytes, a WM_CHAR unit types over the selection, and
//! VK_BACK deletes the selection or the character before the caret. Automatic insertions are off, so after the last
//! input the buffer equals the script's result.

use crate::fixtures::Loaded;

/// The sample the editor is in: the edit Reset named and how many of its inputs arrived.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sample {
    pub edit: usize,
    pub id: u64,
    /// Selects and typed keys the edit has (Reset not counted).
    pub expected: usize,
    pub received: usize,
    /// Why the sample can no longer be timed. A failed sample writes no row, so the harness times it out.
    pub failure: Option<String>,
    /// Whether a typed key has changed the buffer.
    pub typed: bool,
}

impl Sample {
    /// Every input arrived and none failed.
    pub fn complete(&self) -> bool {
        self.failure.is_none() && self.received == self.expected
    }
}

/// The buffer, its selection and its sample.
#[derive(Clone, Debug, Default)]
pub struct Editor {
    pub text: String,
    /// The selection `[start, end)` in UTF-8 bytes. Typing replaces it; the caret is at `end`.
    pub start: usize,
    pub end: usize,
    /// Counts the buffer's changes; the analysis is tagged with it.
    pub revision: u64,
    pub sample: Option<Sample>,
    /// A UTF-16 high surrogate waiting for its low half.
    high: Option<u16>,
}

/// What a WM_CHAR unit did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Typed {
    /// A counted input of the sample.
    Counted,
    /// A control character the editor ignores (the WM_CHAR 0x08 that Godot's TranslateMessage makes of VK_BACK).
    Ignored,
}

impl Editor {
    /// Starts sample `id` of edit `edit`: the buffer becomes the edit's baseline, whose digest is checked again.
    pub fn reset(&mut self, loaded: &Loaded, edit: usize, id: u64) -> Result<(), String> {
        self.high = None;
        self.start = 0;
        self.end = 0;
        self.revision += 1;
        let Some(script) = loaded.edits.get(edit) else {
            self.text.clear();
            self.sample = Some(Sample { edit, id, expected: 0, received: 0, failure: None, typed: false });
            return Err(self.fail(format!("Reset names edit {edit}; the script has {}", loaded.edits.len())));
        };
        let module = loaded.fixtures.get(&script.target).expect("load checked every edit's target");
        self.text.clone_from(&module.source);
        self.sample = Some(Sample { edit, id, expected: script.inputs(), received: 0, failure: None, typed: false });
        if module.sha256() != script.base_sha256 {
            return Err(self.fail(format!("{}'s baseline digest differs from the edit's", script.target)));
        }
        Ok(())
    }

    /// Marks the sample failed and gives the reason back.
    pub fn fail(&mut self, reason: String) -> String {
        if let Some(sample) = &mut self.sample {
            sample.failure.get_or_insert_with(|| reason.clone());
        }
        reason
    }

    /// Counts one input of the sample. Past the edit's last input, or after a failure, nothing is applied.
    fn count(&mut self) -> Result<(), String> {
        let Some(sample) = &mut self.sample else {
            return Err("an input came before any Magic600.LX4.Reset".to_owned());
        };
        if let Some(failure) = &sample.failure {
            return Err(format!("sample {} failed already: {failure}", sample.id));
        }
        sample.received += 1;
        if sample.received > sample.expected {
            let reason = format!("more inputs than edit {}'s {}", sample.edit, sample.expected);
            return Err(self.fail(reason));
        }
        Ok(())
    }

    pub fn select(&mut self, start: usize, end: usize) -> Result<(), String> {
        self.count()?;
        if self.high.is_some() {
            return Err(self.fail("a Select came between the halves of a surrogate pair".to_owned()));
        }
        if start > end || !self.text.is_char_boundary(start) || !self.text.is_char_boundary(end) {
            let reason = format!("Select [{start}, {end}) is no selection of the {}-byte buffer", self.text.len());
            return Err(self.fail(reason));
        }
        self.start = start;
        self.end = end;
        Ok(())
    }

    /// One WM_CHAR unit: 13 (Enter) types a newline, a surrogate pair types one character, and other C0 controls and
    /// DEL are ignored and not counted.
    pub fn char(&mut self, unit: u16) -> Result<Typed, String> {
        if (unit < 0x20 && unit != 9 && unit != 13) || unit == 0x7F {
            return Ok(Typed::Ignored);
        }
        self.count()?;
        let c = match unit {
            0xD800..=0xDBFF => {
                if self.high.replace(unit).is_some() {
                    return Err(self.fail("two high surrogates in a row".to_owned()));
                }
                return Ok(Typed::Counted);
            }
            0xDC00..=0xDFFF => {
                let Some(high) = self.high.take() else {
                    return Err(self.fail("a low surrogate without its high half".to_owned()));
                };
                char::from_u32(0x10000 + ((u32::from(high) - 0xD800) << 10) + (u32::from(unit) - 0xDC00))
            }
            13 => Some('\n'),
            _ => char::from_u32(u32::from(unit)),
        };
        if self.high.is_some() {
            return Err(self.fail("a high surrogate without its low half".to_owned()));
        }
        let c = c.expect("every unit outside the surrogates is a character, and every pair is one");
        let mut bytes = [0u8; 4];
        let typed = c.encode_utf8(&mut bytes);
        self.text.replace_range(self.start..self.end, typed);
        self.start += typed.len();
        self.end = self.start;
        self.changed();
        Ok(Typed::Counted)
    }

    /// VK_BACK's key-down: deletes the selection, or the character before the caret when the selection is empty.
    pub fn backspace(&mut self) -> Result<(), String> {
        self.count()?;
        if self.high.is_some() {
            return Err(self.fail("a VK_BACK came between the halves of a surrogate pair".to_owned()));
        }
        if self.start == self.end {
            match self.text[..self.start].chars().next_back() {
                Some(c) => self.start -= c.len_utf8(),
                None => return Ok(()),
            }
        }
        self.text.replace_range(self.start..self.end, "");
        self.end = self.start;
        self.changed();
        Ok(())
    }

    fn changed(&mut self) {
        self.revision += 1;
        if let Some(sample) = &mut self.sample {
            sample.typed = true;
        }
    }

    /// The caret's 1-based line and column (in characters).
    pub fn caret(&self) -> (usize, usize) {
        let head = &self.text[..self.end];
        let line_start = head.rfind('\n').map_or(0, |i| i + 1);
        (head.matches('\n').count() + 1, head[line_start..].chars().count() + 1)
    }
}
