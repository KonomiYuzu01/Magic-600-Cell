use std::{io::{Read, Write}, sync::{Arc, mpsc}, time::Instant};
use crate::{faults, json::{self, J, num, object, string}, kernel::{Kernel, Snapshot}, sha256, wire};

pub trait KernelPort { fn call(&mut self, request: &J) -> Result<Vec<u8>, String>; }

pub struct DirectPort {
    pub kernel: Arc<Kernel>, pub snapshot: Arc<Snapshot>, pub benchmark: bool,
    pub ready: mpsc::Sender<J>, pub go: mpsc::Receiver<()>,
}

pub fn dispatch(kernel: &Kernel, snapshot: &Arc<Snapshot>, request: &J, benchmark: bool)
    -> Result<Vec<u8>, String> {
    if benchmark && request.get("row")?.text()? == "_clear_benchmark_preview" {
        kernel.clear_benchmark_preview(); return Ok(Vec::new());
    }
    kernel.call(snapshot, request)
}

impl KernelPort for DirectPort {
    fn call(&mut self, request: &J) -> Result<Vec<u8>, String> {
        if request.get("row")?.text()? == "_armed" {
            self.ready.send(request.get("prefix")?.clone()).map_err(|_| "ready channel")?;
            self.go.recv().map_err(|_| "release channel")?;
            return Ok(Vec::new());
        }
        dispatch(&self.kernel, &self.snapshot, request, self.benchmark)
    }
}

pub struct PipePort<R: Read, W: Write> { pub input: R, pub output: W }
impl<R: Read, W: Write> KernelPort for PipePort<R, W> {
    fn call(&mut self, request: &J) -> Result<Vec<u8>, String> {
        wire::write(&mut self.output, request.canonical().as_bytes())?;
        wire::response(&mut self.input)
    }
}

fn request(row: &str, spec: &J) -> Result<J, String> {
    Ok(match row {
        "preview" => object([("row", string(row)), ("word", spec.get("word")?.clone()),
            ("provenance", spec.get("provenance")?.clone())]),
        "check_word" => object([("row", string(row)), ("word", spec.get("word")?.clone())]),
        _ => object([("row", string(row))]),
    })
}

fn prefix(port: &mut impl KernelPort, spec: &J) -> Result<J, String> {
    let labels = port.call(&request("read_labels", spec)?)?;
    let pose = port.call(&request("read_pose_bundle", spec)?)?;
    let check = json::parse(&port.call(&request("check_word", spec)?)?)?;
    let preview = json::parse(&port.call(&request("preview", spec)?)?)?;
    let refusal = port.call(&request("commit", spec)?).unwrap_err();
    if labels.len() != 1_039_200 || pose.len() != 869_952 || refusal != "capability.denied" {
        return Err("runtime prefix differs".into());
    }
    Ok(object([("labels_bytes", num(labels.len())), ("labels_sha256", string(sha256::hex(&labels))),
        ("pose_bytes", num(pose.len())), ("pose_sha256", string(sha256::hex(&pose))),
        ("check_word", check), ("preview", preview), ("runtime_commit_refusal", string(refusal))]))
}

fn measure(port: &mut impl KernelPort, spec: &J) -> Result<J, String> {
    let mut rows = Vec::new();
    // Smoke first across every row, then three distinct series of 100 per row.
    for series in 0..=3 {
        for row in ["read_labels", "read_pose_bundle", "check_word", "preview"] {
            let req = request(row, spec)?;
            let samples = if series == 0 { 1 } else { 100 };
            let mut times = Vec::new(); let mut expected: Option<Vec<u8>> = None;
            for _ in 0..samples {
                if row == "preview" { port.call(&request("_clear_benchmark_preview", spec)?)?; }
                let start = Instant::now();
                let reply = port.call(&req)?;
                let elapsed = start.elapsed().as_nanos();
                // Complete response comparison is outside the measurement interval.
                if let Some(bytes) = &expected {
                    if &reply != bytes { return Err("benchmark reply changed".into()); }
                } else { expected = Some(reply); }
                times.push(num(elapsed));
            }
            let bytes = expected.unwrap();
            rows.push(object([("row", string(row)), ("series", num(series)),
                ("request_bytes", num(req.canonical().len())), ("reply_bytes", num(bytes.len())),
                ("reply_sha256", string(sha256::hex(&bytes))), ("samples_ns", J::Array(times))]));
        }
    }
    Ok(object([("status", string("ok")), ("code", string("measured")), ("rows", J::Array(rows))]))
}

pub fn script(port: &mut impl KernelPort, spec: &J) -> Result<J, String> {
    let benchmark = spec.get("mode")?.text()? == "benchmark";
    let prefix = if benchmark { J::Null } else { prefix(port, spec)? };
    port.call(&object([("row", string("_armed")), ("prefix", prefix)]))?;
    if benchmark { measure(port, spec) } else { Ok(faults::run(spec.get("fault")?.text()?)) }
}
