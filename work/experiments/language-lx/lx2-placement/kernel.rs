// A deliberately small experiment session, not the product persistence protocol.
use std::{fs::{File, OpenOptions}, io::Write, path::Path, sync::{Arc, Mutex}};
use crate::{engine::{Model, SLOTS}, json::{J, num, object, string}, sha256};

pub struct Snapshot {
    pub revision: u64,
    pub labels: Vec<i32>,
    pub bytes: Vec<u8>,
    pub hash: String,
}

impl Snapshot {
    fn new(revision: u64, labels: Vec<i32>) -> Arc<Self> {
        let bytes: Vec<u8> = labels.iter().flat_map(|v| v.to_le_bytes()).collect();
        let hash = sha256::hex(&bytes);
        Arc::new(Self { revision, labels, bytes, hash })
    }
}

#[derive(Clone)]
struct Preview { base: Arc<Snapshot>, after: Arc<Snapshot>, word: Vec<i32>, provenance: J, receipt: J }
struct Session {
    current: Arc<Snapshot>, pending: Option<Preview>, journal: File,
    history: Vec<(Vec<i32>, J)>, receipts: Vec<J>,
}
pub struct Kernel { pub model: Arc<Model>, pose: Vec<u8>, provenance: J, session: Mutex<Session> }

impl Kernel {
    pub fn new(model: Model, pose: Vec<u8>, provenance: J, journal: &Path) -> Result<Arc<Self>, String> {
        if pose.len() != 869_952 { return Err("wrong pose payload length".into()); }
        let file = OpenOptions::new().create_new(true).append(true).open(journal)
            .map_err(|_| "cannot create fresh journal")?;
        file.sync_all().map_err(|_| "cannot sync fresh journal")?;
        let current = Snapshot::new(0, model.ids.clone());
        Ok(Arc::new(Self { model: Arc::new(model), pose, provenance,
            session: Mutex::new(Session { current, pending: None, journal: file,
                history: Vec::new(), receipts: Vec::new() }) }))
    }

    // Acquired once at job admission. Runtime reads use this immutable snapshot;
    // they never hold a session lock while copying or transferring a payload.
    pub fn snapshot(&self) -> Arc<Snapshot> { self.session.lock().unwrap().current.clone() }

    fn word(&self, input: &J) -> Result<Vec<i32>, String> {
        let recipe = J::Array(vec![object([("kind", string("word")), ("moves", input.clone())])]);
        Ok(self.model.expand(&self.model.normalize(&recipe)?))
    }

    pub fn preview(&self, base: Arc<Snapshot>, input: &J, provenance: &J) -> Result<J, String> {
        if provenance != &self.provenance { return Err("provenance differs".into()); }
        let word = self.word(input)?;
        let mut labels = base.labels.clone();
        self.model.apply_word(&mut labels, &word, &mut Vec::new());
        let after = Snapshot::new(base.revision + 1, labels);
        let receipt = object([("base_revision", num(base.revision)),
            ("base_hash", string(&base.hash)), ("target_hash", string(&after.hash)),
            ("word_sha256", string(sha256::ints(&word))),
            ("provenance_sha256", string(sha256::hex(provenance.canonical().as_bytes())))]);
        let mut session = self.session.lock().unwrap();
        if session.current.revision != base.revision || session.current.hash != base.hash {
            return Err("stale preview".into());
        }
        if session.pending.is_some() { return Err("pending preview exists".into()); }
        session.pending = Some(Preview { base, after, word, provenance: provenance.clone(), receipt: receipt.clone() });
        Ok(receipt)
    }

    // The runtime's only kernel rows. Commit and undo are on the driver control
    // channel, never in this dispatch, and the runtime receives no journal path.
    pub fn call(&self, snapshot: &Arc<Snapshot>, request: &J) -> Result<Vec<u8>, String> {
        let result = match request.get("row")?.text()? {
            "read_labels" => return Ok(snapshot.bytes.clone()),
            "read_pose_bundle" => return Ok(self.pose.clone()),
            "check_word" => {
                let word = self.word(request.get("word")?)?;
                let (source, destination) = self.model.word_net(&word);
                object([("legal", J::Bool(true)), ("letters", num(word.len())),
                    ("net_sha256", string(sha256::pair(&source, &destination))),
                    ("moved_slots", num(source.len()))])
            },
            "preview" => self.preview(snapshot.clone(), request.get("word")?, request.get("provenance")?)?,
            _ => return Err("capability.denied".into()),
        };
        Ok(result.canonical().into_bytes())
    }

    // This reset is used only between benchmark samples, outside their clocks.
    pub fn clear_benchmark_preview(&self) { self.session.lock().unwrap().pending = None; }

    fn append(session: &mut Session, kind: &str, word: &[i32], provenance: &J,
              after: &Arc<Snapshot>) -> Result<J, String> {
        let row = object([("revision", num(after.revision)), ("kind", string(kind)),
            ("word", J::Array(word.iter().map(|&m| num(m)).collect())),
            ("previous_hash", string(&session.current.hash)), ("state_hash", string(&after.hash)),
            ("provenance", provenance.clone())]);
        let bytes = row.canonical() + "\n";
        session.journal.write_all(bytes.as_bytes()).map_err(|_| "journal write failed")?;
        session.journal.flush().map_err(|_| "journal flush failed")?;
        session.journal.sync_all().map_err(|_| "journal sync failed")?;
        // Publish state and receipt only after the durable append has succeeded.
        session.current = after.clone();
        let receipt = object([("status", string("committed")), ("revision", num(after.revision)),
            ("state_hash", string(&after.hash)), ("journal_row_sha256", string(sha256::hex(bytes.as_bytes())))]);
        session.receipts.push(receipt.clone());
        Ok(receipt)
    }

    pub fn commit(&self, expected: &J) -> Result<J, String> {
        let mut session = self.session.lock().unwrap();
        let preview = session.pending.as_ref().ok_or("no preview")?.clone();
        if expected != &preview.receipt || preview.base.revision != session.current.revision
            || preview.base.hash != session.current.hash { return Err("stale preview".into()); }
        let receipt = Self::append(&mut session, "commit", &preview.word, &preview.provenance, &preview.after)?;
        session.history.push((preview.word, preview.provenance));
        session.pending = None;
        Ok(receipt)
    }

    pub fn undo(&self) -> Result<J, String> {
        let mut session = self.session.lock().unwrap();
        let (word, provenance) = session.history.last().ok_or("nothing to undo")?.clone();
        let inverse: Vec<i32> = word.iter().rev().map(|m| -m).collect();
        let mut labels = session.current.labels.clone();
        self.model.apply_word(&mut labels, &inverse, &mut Vec::new());
        let after = Snapshot::new(session.current.revision + 1, labels);
        let receipt = Self::append(&mut session, "undo", &inverse, &provenance, &after)?;
        session.history.pop(); session.pending = None;
        Ok(receipt)
    }

    pub fn status(&self) -> J {
        let s = self.session.lock().unwrap();
        object([("revision", num(s.current.revision)), ("state_hash", string(&s.current.hash)),
            ("slots", num(SLOTS)), ("pending", s.pending.as_ref().map_or(J::Null, |p| p.receipt.clone())),
            ("receipts", J::Array(s.receipts.clone()))])
    }
}
