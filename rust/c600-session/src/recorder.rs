//! The session's recorder (LP4-9 draft; chapter 04 EV4.20, chapter 05 PS1.16, PS1.30 and PS1.44, the format packet's
//! FP.37): the command layer's recordings and playback states as workspace records in the session's own journal,
//! outside its action order, through the store ([`crate::command_store::JournalStore::recorder`]).
//!
//! [`JournalRecorder`] shares its [`crate::command_store::JournalStore`]'s journal, lock and fate rules: each write is
//! one journal frame, `{"record": "command_workspace", "id", "kind", "value"}` ([`crate::command_store`]), appended
//! and synced before the layer keeps what it records. `kind` is `c600-recording-draft` or `c600-playback-draft`, `id`
//! is the recording's name, and `value` is the recording, in `c600_command::Recording::to_json`'s form, as a record
//! object that the frame cites (a recording is written once and may be large), or the playback state, in
//! `c600_command::PlaybackState::to_json`'s form, inline (it is small and written at every step, so a playback adds no
//! object per step). The last frame of an id and kind wins. A frame takes no action number, has no receipt and creates
//! no undo target (EV4.20, EV4.21, PS1.44: "outside the undo order"); the layer looks up a pending child's receipt in
//! the same journal first and reconciles it at most once (KN5.23).
//!
//! Because the records are the session journal's own frames, they belong to that session: a new session needs a disk
//! with no journal ([`crate::command_store::JournalStore::create`]), so no playback state, and no `actions` map of
//! action numbers, survives into another session, and a reopen reads them with the journal they were written in. A
//! failed append that the journal cuts back is rolled back; any other failure leaves the fate unknown until the
//! journal is read again. A damaged frame, or a frame that does not read, ends the journal's history there, as any
//! other record does, and the layer opens in read-only recovery.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use c600_command::recording::{PLAYBACK_FORMAT, RECORDING_FORMAT};
use c600_command::{Error, Fate, PlaybackState, Recorded, Recorder, Recording};
use c600_kernel::json::Value;

use crate::command_store::Inner;

/// The recorder over a session's journal. See the module documentation.
pub struct JournalRecorder(pub(crate) Arc<Mutex<Inner>>);

impl JournalRecorder {
    fn inner(&self) -> MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Applies one workspace record of `kind` to what the recorder holds; `false` when the value does not read as that
/// kind under that id, or the kind is neither.
pub(crate) fn keep(held: &mut Recorded, id: &str, kind: &str, value: &Value) -> bool {
    match kind {
        RECORDING_FORMAT => match Recording::from_json(value) {
            Some(recording) if recording.name == id && recording.to_json() == *value => {
                held.recordings.insert(id.to_owned(), recording);
                true
            }
            _ => false,
        },
        PLAYBACK_FORMAT => match PlaybackState::from_json(value) {
            Some(state) if state.to_json() == *value => {
                held.playbacks.insert(id.to_owned(), state);
                true
            }
            _ => false,
        },
        _ => false,
    }
}

impl Recorder for JournalRecorder {
    fn save_recording(&mut self, recording: &Recording) -> Fate {
        self.inner().keep(&recording.name, RECORDING_FORMAT, &recording.to_json(), true)
    }

    fn save_playback(&mut self, name: &str, state: &PlaybackState) -> Fate {
        self.inner().keep(name, PLAYBACK_FORMAT, &state.to_json(), false)
    }

    /// What the journal holds, read again first after a write whose fate was unknown. A journal that holds no session
    /// is refused with `KN4:input.unknown_session`, and one that cannot be read again with
    /// `KN4:resource_limit.disk_full`.
    fn load(&mut self) -> Result<Recorded, Error> {
        self.inner().kept()
    }
}
