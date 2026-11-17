//! A job, and the in-process executor that the child runs and that tests compare it with.
//!
//! [`run_in_process`] parses, lowers, admits, loads and runs one job against a host. The child process calls exactly
//! this function with its proxy host, so the compile stage is the same code on both sides and the comparison tests
//! hold the process boundary itself to account: transfer, forwarding and fault handling.

use std::collections::BTreeSet;

use c600_check::{admit, admit_built_in, Grant, Policy, Profile, Row};
use c600_ir::Capability;
use c600_run::interp::{load, Cancel, Host, Options, Run, SourceRanges};
use c600_run::menu::MenuAlphabet;
use c600_run::meter::{Amounts, DraftTariffs, SourceRange, Tariffs};
use c600_run::trace::Relation;
use c600_run::value::Value;
use c600_syntax::{lower, parse, Lowered};

/// How to run a job: the run options of `c600_run::interp::Options` that cross the boundary, and the admission
/// inputs of `c600_check::admit` or, for a built-in tool, `c600_check::admit_built_in`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobOptions {
    pub limits: Amounts,
    pub tariffs: Tariffs,
    pub draft: DraftTariffs,
    /// The generator's seed and stream path (DET.61).
    pub seed: Option<(Vec<u8>, Vec<String>)>,
    /// Whether the summary keeps one row per operation.
    pub record: bool,
    pub profile: Profile,
    pub grant: Vec<Capability>,
    /// The fixed declassifiers the boundary policy enables (BND.54); none under the interim policy.
    pub declassifiers: Vec<String>,
    /// Whether stops and rows carry source ranges (LG4.12, DET.56).
    pub source_ranges: bool,
    /// The built-in column of chapter 07 section 8 (LP2 draft; LP3's plan, section 2): the rows the built-in tool's
    /// reviewed class lists (BND.57). `Some` admits the entry with `c600_check::admit_built_in` instead of `admit`.
    /// The column is the solving profile's, so a job that names class rows in another profile is refused at the
    /// admission stage without a code. `None` admits the entry with `admit` in `profile`.
    pub built_in: Option<BTreeSet<Row>>,
    /// The menu alphabet the run pins for its menu words (LB1.8; LP2 draft): `c600_run::interp::Options::menu`.
    pub menu: Option<MenuAlphabet>,
    /// The independence relation the run pins for its trace keys (LB1.10; LP2 draft):
    /// `c600_run::interp::Options::relation`.
    pub relation: Option<Relation>,
}

impl JobOptions {
    /// Test tariffs, rows kept, no seed, the tool-authoring profile, no capability, the interim policy, source ranges
    /// on, no built-in column, no menu alphabet and no trace relation.
    pub fn new(limits: Amounts) -> JobOptions {
        JobOptions {
            limits,
            tariffs: Tariffs::TEST,
            draft: DraftTariffs::TEST,
            seed: None,
            record: true,
            profile: Profile::ToolAuthoring,
            grant: Vec::new(),
            declassifiers: Vec::new(),
            source_ranges: true,
            built_in: None,
            menu: None,
            relation: None,
        }
    }
}

/// One program run: the source bytes of its `.6cs` files, the entry and its arguments.
#[derive(Clone, Debug)]
pub struct Job {
    pub sources: Vec<Vec<u8>>,
    pub module: String,
    pub entry: String,
    pub args: Vec<Value>,
    pub options: JobOptions,
}

/// Where a job was refused before its first metered operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// A source file has syntax diagnostics.
    Parse,
    /// The parsed files do not lower to one IR program.
    Lower,
    /// The checker or the admission of the entry refused the program.
    Admit,
    /// The interpreter refused the IR (`c600_run::interp::load`).
    Load,
    /// The interpreter refused the entry or its arguments (`Loaded::run`).
    Start,
}

/// A refusal before any operation is metered. `codes` are the stable codes of the diagnostics or of the refusal;
/// a lowering error has none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub stage: Stage,
    pub codes: Vec<String>,
    pub detail: String,
}

/// How a job ended inside the runtime.
#[derive(Debug)]
pub enum Ended {
    /// The run's meter record, and the value and output bytes when it completed.
    Ran(Run),
    Refused(Refused),
}

struct Ranges<'a>(&'a Lowered);

impl SourceRanges for Ranges<'_> {
    fn range(&self, module: &str, function: &str, path: &str) -> Option<SourceRange> {
        let map = self.0.source(module)?;
        let span = map.span(function, path)?;
        Some(SourceRange { digest: map.digest.clone(), start: span.start, end: span.end })
    }
}

fn refused(stage: Stage, codes: Vec<String>, detail: String) -> Ended {
    Ended::Refused(Refused { stage, codes, detail })
}

/// Parses, lowers, admits, loads and runs a job against `host` in this process.
pub fn run_in_process(job: &Job, host: &mut dyn Host, cancel: Option<&dyn Cancel>) -> Ended {
    let mut files = Vec::with_capacity(job.sources.len());
    for source in &job.sources {
        let file = parse(source);
        if !file.diagnostics.is_empty() {
            let codes = file.diagnostics.iter().map(|d| d.code.to_string()).collect();
            let detail = file.diagnostics.iter().map(|d| format!("{}: {}", d.code, d.message)).collect::<Vec<_>>();
            return refused(Stage::Parse, codes, detail.join("\n"));
        }
        files.push(file);
    }
    let lowered = match lower(&files) {
        Ok(lowered) => lowered,
        Err(e) => return refused(Stage::Lower, Vec::new(), e.to_string()),
    };
    let o = &job.options;
    let declassifiers: Vec<&str> = o.declassifiers.iter().map(String::as_str).collect();
    let policy = Policy::enabling(&declassifiers);
    let grant = Grant::of(&o.grant);
    let admission = match &o.built_in {
        None => admit(&lowered.program, &job.module, &job.entry, o.profile, &grant, &policy),
        Some(_) if o.profile != Profile::Solving => {
            let detail = "the built-in column is the solving profile's: a job that names class rows runs in the \
                          solving profile (chapter 07 section 8)";
            return refused(Stage::Admit, Vec::new(), detail.into());
        }
        Some(rows) => admit_built_in(&lowered.program, &job.module, &job.entry, rows, &grant, &policy),
    };
    let admission = match admission {
        Ok(admission) => admission,
        Err(e) => return refused(Stage::Admit, Vec::new(), e.to_string()),
    };
    if !admission.is_admitted() {
        let codes = admission.diagnostics.iter().map(|d| d.code.to_string()).collect();
        let detail = admission
            .diagnostics
            .iter()
            .map(|d| format!("{}: {}.{} at {}: {}", d.code, d.module, d.function, d.path, d.note))
            .collect::<Vec<_>>();
        return refused(Stage::Admit, codes, detail.join("\n"));
    }
    let loaded = match load(&lowered.program) {
        Ok(loaded) => loaded,
        Err(r) => return refused(Stage::Load, vec![r.code.to_string()], r.detail),
    };
    let ranges = Ranges(&lowered);
    let path: Vec<&str> = o.seed.as_ref().map(|(_, p)| p.iter().map(String::as_str).collect()).unwrap_or_default();
    let options = Options {
        limits: o.limits,
        tariffs: o.tariffs,
        draft: o.draft,
        seed: o.seed.as_ref().map(|(seed, _)| (seed.as_slice(), path.as_slice())),
        record: o.record,
        cancel,
        sources: if o.source_ranges { Some(&ranges) } else { None },
        menu: o.menu.as_ref(),
        relation: o.relation.as_ref(),
    };
    match loaded.run(&job.module, &job.entry, job.args.clone(), host, &options) {
        Ok(run) => Ended::Ran(run),
        Err(r) => refused(Stage::Start, vec![r.code.to_string()], r.detail),
    }
}
