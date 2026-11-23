//! The recorder on a disk (LP4-9 draft; chapter 04 EV4.20, chapter 05 PS1.30 and PS1.44): the durable home of the
//! command layer's recordings and playback states, which `c600_command::Recorder` defines.
//!
//! [`DiskRecorder`] keeps one file, [`RECORDER`], next to the session's journal, in the journal's frames
//! ([`crate::journal::frame`]): each write is one frame, appended and synced, holding one canonical JSON record,
//! `{"record": "recording", "recording"}` with the recording in `c600_command::Recording::to_json`'s form or
//! `{"record": "playback", "name", "state"}` with the state in `c600_command::PlaybackState::to_json`'s form. The last
//! frame of a name wins. The file sits outside the journal's action order, so a write here takes no action number and
//! creates no undo target (EV4.20, EV4.21); the layer looks up a pending child's receipt in the journal first and
//! reconciles it at most once (KN5.23).
//!
//! A failed append that the recorder cuts back is rolled back; any other failure leaves the fate unknown until the
//! file is read again, which then decides it, as the journal's store does. A torn final frame is cut off at open. A
//! damaged frame, or a record that does not read, refuses the recorder (`PS1:input.persistence_format`) and every
//! later write, and nothing is cut. The recorder takes no session lock: the journal's store holds it, and the host
//! opens the recorder on a clone of that store's disk only after the store took the lock.
//!
//! The record shapes and the file are drafts (`rust/README.md`, LP4-9): PS1.12 lists a recording and a playback state
//! as workspace records under the journal's own order, and this file keeps them apart from it.

use std::io;

use c600_command::{Error, Fate, PlaybackState, Recorded, Recorder, Recording};
use c600_kernel::json::{self, Value};

use crate::disk::Disk;
use crate::journal::{self, Tail, MAX_PAYLOAD};
use crate::types::code;

/// The name of the recorder's file on the session's disk.
pub const RECORDER: &str = "recorder";

/// The recorder over a disk. See the module documentation.
pub struct DiskRecorder {
    disk: Box<dyn Disk>,
    /// The end of the last whole frame, which a failed append is cut back to.
    len: u64,
    held: Recorded,
    /// A write whose fate is unknown happened since the file was last read.
    unsettled: bool,
    /// Why the file does not read, if it does not.
    damaged: Option<Error>,
}

fn persistence(detail: impl Into<String>) -> Error {
    Error::new(code::PERSISTENCE_FORMAT, detail)
}

/// What one record does to what the recorder holds.
fn apply(held: &mut Recorded, payload: &[u8]) -> Result<(), Error> {
    let value = json::parse(payload).map_err(|_| persistence("a recorder frame is not JSON"))?;
    if value.canonical().as_bytes() != payload {
        return Err(persistence("a recorder frame is not canonical JSON"));
    }
    let fields = value.as_object().map_or(0, |fields| fields.len());
    match value.get("record").and_then(Value::as_str) {
        Some("recording") if fields == 2 => {
            let recording = value.get("recording").and_then(Recording::from_json);
            let recording = recording.ok_or_else(|| persistence("a recorder frame holds no recording"))?;
            held.recordings.insert(recording.name.clone(), recording);
        }
        Some("playback") if fields == 3 => {
            let name = value.get("name").and_then(Value::as_str);
            let state = value.get("state").and_then(PlaybackState::from_json);
            let (Some(name), Some(state)) = (name, state) else {
                return Err(persistence("a recorder frame holds no playback state"));
            };
            held.playbacks.insert(name.to_owned(), state);
        }
        _ => return Err(persistence("a recorder frame is neither a recording nor a playback state")),
    }
    Ok(())
}

impl DiskRecorder {
    /// The recorder on `disk`: its file is read, a torn final frame is cut off and the rest is synced. A file that
    /// does not exist holds nothing. A disk that cannot be read is refused with `KN4:resource_limit.disk_full`; a
    /// damaged file opens, and [`Recorder::load`] refuses it.
    pub fn open(disk: Box<dyn Disk>) -> Result<DiskRecorder, Error> {
        let mut recorder = DiskRecorder { disk, len: 0, held: Recorded::default(), unsettled: false, damaged: None };
        recorder.reread().map_err(|e| Error::new(code::DISK_FULL, format!("the recorder's file: {e}")))?;
        Ok(recorder)
    }

    /// Reads the file again and settles it: what it holds is what the recorder holds.
    fn reread(&mut self) -> io::Result<()> {
        let Some(bytes) = self.disk.read(RECORDER)? else {
            self.len = 0;
            self.held = Recorded::default();
            self.unsettled = false;
            return Ok(());
        };
        let scan = journal::scan(&bytes);
        let mut held = Recorded::default();
        let mut damaged = match scan.tail {
            Tail::Damaged(at) => Some(persistence(format!("the recorder's file is damaged at byte {at}"))),
            Tail::Torn | Tail::Clean => None,
        };
        for payload in &scan.payloads {
            if damaged.is_some() {
                break;
            }
            damaged = apply(&mut held, payload).err();
        }
        if damaged.is_none() {
            if scan.tail == Tail::Torn {
                self.disk.truncate(RECORDER, scan.end)?;
            }
            self.disk.sync(RECORDER)?;
        }
        self.len = scan.end;
        self.held = held;
        self.damaged = damaged;
        self.unsettled = false;
        Ok(())
    }

    /// Appends one record as a frame and syncs it.
    fn write(&mut self, record: Value, apply_held: impl FnOnce(&mut Recorded)) -> Fate {
        if self.unsettled && self.reread().is_err() {
            return Fate::Unknown;
        }
        if let Some(error) = &self.damaged {
            return Fate::RolledBack(error.clone());
        }
        let payload = record.canonical();
        if payload.len() > MAX_PAYLOAD {
            return Fate::RolledBack(Error::new(code::DISK_FULL, "a recorder record exceeds the frame limit"));
        }
        let bytes = journal::frame(payload.as_bytes());
        match self.disk.append(RECORDER, &bytes).and_then(|()| self.disk.sync(RECORDER)) {
            Ok(()) => {
                self.len += bytes.len() as u64;
                apply_held(&mut self.held);
                Fate::Durable
            }
            Err(_) => match self.disk.truncate(RECORDER, self.len).and_then(|()| self.disk.sync(RECORDER)) {
                Ok(()) => Fate::RolledBack(Error::new(code::DISK_FULL, "the recorder's record was not written")),
                Err(_) => {
                    self.unsettled = true;
                    Fate::Unknown
                }
            },
        }
    }
}

impl Recorder for DiskRecorder {
    fn save_recording(&mut self, recording: &Recording) -> Fate {
        let record = json::object([("record", Value::String("recording".into())), ("recording", recording.to_json())]);
        let recording = recording.clone();
        self.write(record, |held| {
            held.recordings.insert(recording.name.clone(), recording);
        })
    }

    fn save_playback(&mut self, name: &str, state: &PlaybackState) -> Fate {
        let record = json::object([
            ("record", Value::String("playback".into())),
            ("name", Value::String(name.to_owned())),
            ("state", state.to_json()),
        ]);
        let state = state.clone();
        self.write(record, |held| {
            held.playbacks.insert(name.to_owned(), state);
        })
    }

    /// What the file holds; after a write whose fate was unknown, the file is read again first. A damaged file is
    /// refused with `PS1:input.persistence_format`, and a file that cannot be read with
    /// `KN4:resource_limit.disk_full`.
    fn load(&mut self) -> Result<Recorded, Error> {
        if self.unsettled {
            self.reread().map_err(|e| Error::new(code::DISK_FULL, format!("the recorder's file: {e}")))?;
        }
        match &self.damaged {
            Some(error) => Err(error.clone()),
            None => Ok(self.held.clone()),
        }
    }
}
