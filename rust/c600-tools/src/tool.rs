//! The built-in 600-cell-Full tool: its source, its module manifest (BND.27, BND.37), the rows its class lists, its
//! admission in the solving profile (`c600_check::admit_built_in`, the built-in column of chapter 07 section 8) and
//! one run of it through the run-time boundary.

use std::collections::BTreeSet;

use c600_check::{admit_built_in, Admission, Grant, Policy, Row};
use c600_command::Budget;
use c600_digest::{sha256, sha256_hex};
use c600_ir::{Expr, Function, Lit, Program};
use c600_run::boundary::{Boundary, Capture, Context, Pins, Shown, Store};
use c600_run::interp::{load, Host, Loaded, Options, Refusal, Run};
use c600_run::meter::Amounts;
use c600_run::provenance::{self, Author, Pin, Producer, Profile};
use c600_run::value::Value;
use c600_syntax::{lower, parse};

/// The tool's source: edition `6cs-draft-1`, syntax candidate A, in the formatter's canonical layout.
pub const SOURCE: &str = include_str!("full_600_cell.6cs");
/// The tool's module.
pub const MODULE: &str = "full_600_cell";
/// The tool's entry: a command the solver invokes with the word to preview.
pub const ENTRY: &str = "preview_word";
/// The command-layer row the entry's target names.
pub const ROW: &str = "operation.preview";

/// The run limits of one run of a tool (DET.55): wide enough for a 100,000-letter word, 0.4's word limit. They are
/// every profile's ceiling ([`Budget::CEILING`], the owner's values of 21 November 2026), the budget a shell declares
/// by default.
pub const LIMITS: Amounts = Amounts { steps: 1 << 21, kernel_cost: 1 << 40, memory: 1 << 27, output: 1 << 24 };

/// The run limits of a program job's budget (KN7.7): the layer's [`Budget`] has the meter's four dimensions under the
/// same names, so each maps by name. A host runs a job's entry under them ([`crate::process::ToolCommand::limits`]).
pub fn limits(budget: Budget) -> Amounts {
    let Budget { steps, kernel_cost, memory, output } = budget;
    Amounts { steps, kernel_cost, memory, output }
}

/// The draft capability contract these tools' blocks name until the policy in force names one (FP.17).
const CONTRACT: &[u8] =
    b"c600-tools draft capability contract 1: capability separation (LG3.13) under the interim policy (BND.54)\n";

/// BND.27's boundary classes. A class grants nothing by itself and is reviewed when the tool ships (BM5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BoundaryClass {
    Tracking,
    Analysis,
    Checking,
    Protection,
    ViewAid,
    SetupSearchNamedTarget,
    RepairList,
}

impl BoundaryClass {
    pub const ALL: [BoundaryClass; 7] = [
        BoundaryClass::Tracking,
        BoundaryClass::Analysis,
        BoundaryClass::Checking,
        BoundaryClass::Protection,
        BoundaryClass::ViewAid,
        BoundaryClass::SetupSearchNamedTarget,
        BoundaryClass::RepairList,
    ];

    /// The class's name in BND.27.
    pub fn name(self) -> &'static str {
        match self {
            BoundaryClass::Tracking => "tracking",
            BoundaryClass::Analysis => "analysis",
            BoundaryClass::Checking => "checking",
            BoundaryClass::Protection => "protection",
            BoundaryClass::ViewAid => "view_aid",
            BoundaryClass::SetupSearchNamedTarget => "setup_search_named_target",
            BoundaryClass::RepairList => "repair_list",
        }
    }
}

/// The fields of a built-in tool's module manifest that this crate's tools need (BND.27; LG5.2 and PS1.15 define the
/// manifest). The IR draft and the syntax carry no manifest yet, so it is data here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub tool: &'static str,
    /// The LB6.5 candidate the tool belongs to.
    pub candidate: &'static str,
    pub module: &'static str,
    pub entry: &'static str,
    /// The declared class (BND.37): LB6.5's candidate classes for the candidate.
    pub classes: &'static [BoundaryClass],
    /// The command-layer rows the tool's proposals reach.
    pub rows: &'static [&'static str],
    /// The SHA-256 of the module's source, in hex: the digest a review record will name (CT-BND-15).
    pub source_sha256: String,
    /// Whether a `PrincipleRecord` exists (LB6.3). None does yet (LB6.5).
    pub principle_record: bool,
}

/// The 600-cell-Full tool's manifest.
pub fn manifest() -> Manifest {
    Manifest {
        tool: "600-cell-Full tool",
        candidate: "Word/recipe composer and full-effect inspector",
        module: MODULE,
        entry: ENTRY,
        classes: &[BoundaryClass::Analysis, BoundaryClass::Checking],
        rows: &[ROW],
        source_sha256: sha256_hex(SOURCE.as_bytes()),
        principle_record: false,
    }
}

/// Parses and lowers one module's source to the IR draft. A parse diagnostic or a lowering error is an `Err`.
pub fn lower_source(source: &str) -> Result<Program, String> {
    let file = parse(source.as_bytes());
    if !file.diagnostics.is_empty() || !file.holes.is_empty() {
        return Err(format!("the source does not parse: {:?}", file.diagnostics));
    }
    lower(&[file]).map(|lowered| lowered.program).map_err(|error| format!("the source does not lower: {error}"))
}

fn function<'p>(program: &'p Program, module: &str, entry: &str) -> Option<&'p Function> {
    program.modules.iter().filter(|m| m.name == module).flat_map(|m| &m.functions).find(|f| f.name == entry)
}

fn path_text(path: &[usize]) -> String {
    if path.is_empty() {
        "root".into()
    } else {
        path.iter().map(usize::to_string).collect::<Vec<_>>().join(".")
    }
}

/// The target texts of the proposals in an entry's body; a target that is not a literal is an `Err` with its path.
pub fn targets(program: &Program, module: &str, entry: &str) -> Result<BTreeSet<String>, String> {
    fn walk(e: &Expr, path: &mut Vec<usize>, out: &mut BTreeSet<String>) -> Result<(), String> {
        if let Expr::Propose { target, .. } = e {
            match &**target {
                Expr::Lit(Lit::Target(text)) => {
                    out.insert(text.clone());
                }
                _ => return Err(format!("the target at {} is not a literal", path_text(path))),
            }
        }
        for (i, child) in e.children().into_iter().enumerate() {
            path.push(i);
            walk(child, path, out)?;
            path.pop();
        }
        Ok(())
    }
    let f = function(program, module, entry).ok_or_else(|| format!("no function `{module}.{entry}`"))?;
    let mut out = BTreeSet::new();
    walk(&f.body, &mut Vec::new(), &mut out)?;
    Ok(out)
}

/// The rows a previewing built-in tool's class lists for the built-in column (BND.57): the prepare-a-preview row of
/// each command-layer row the manifest names, and nothing else. Draft (LP3): BND.57's rows by class do not exist as
/// data, so the class lists what the manifest's rows say the entry does; an entry that also reads state, publishes a
/// view or writes the workspace uses a row the class does not list.
pub fn class_rows(manifest: &Manifest) -> BTreeSet<Row> {
    manifest.rows.iter().map(|row| Row::Preview(row.to_string())).collect()
}

/// The draft capability contract's pin.
pub fn contract() -> Pin {
    Pin::of("c600-tools-draft-1", CONTRACT)
}

/// The checker's pin: `c600-check`'s source.
pub fn checker() -> Pin {
    Pin::of("c600-check-lp2-draft", include_bytes!("../../c600-check/src/lib.rs"))
}

/// The pins the policy in force holds for these runs.
pub fn pins() -> Pins {
    Pins { contracts: vec![contract()], checkers: vec![checker()] }
}

/// The block of a word the solver entered or named: a manual record under the draft contract (FP.17).
pub fn named_word_block() -> Vec<u8> {
    provenance::canonical(&provenance::manual(&contract()))
}

/// What one run left: the interpreter's run, what the shell shows for each proposal and any captures.
#[derive(Debug)]
pub struct Ran {
    pub run: Run,
    pub shell: Vec<Shown>,
    pub captures: Vec<Capture>,
}

/// A built-in tool, parsed, lowered, admitted in the solving profile and loaded.
pub struct Tool {
    pub manifest: Manifest,
    pub program: Program,
    pub admission: Admission,
    /// The SHA-256 of the module's source: the producer's module digest.
    module: [u8; 32],
    loaded: Loaded,
}

impl Tool {
    /// A built-in tool from its module's source and its manifest. It fails if the manifest names another source, if
    /// the source does not parse, lower, check or load, if the built-in column (`c600_check::admit_built_in`, with the
    /// manifest's [`class_rows`]) does not admit the entry, or if the rows the entry's proposals name are not the
    /// manifest's rows.
    pub fn new(source: &str, manifest: Manifest) -> Result<Tool, String> {
        if manifest.source_sha256 != sha256_hex(source.as_bytes()) {
            return Err("the manifest names another source".into());
        }
        let program = lower_source(source)?;
        let (module, entry) = (manifest.module, manifest.entry);
        let admission =
            admit_built_in(&program, module, entry, &class_rows(&manifest), &Grant::none(), &Policy::interim())
                .map_err(|error| error.to_string())?;
        if !admission.is_admitted() {
            return Err(format!("the tool is not admitted: {:?}", admission.diagnostics));
        }
        let rows: BTreeSet<String> = manifest.rows.iter().map(|row| row.to_string()).collect();
        if targets(&program, module, entry)? != rows {
            return Err(format!("the entry's proposals do not name the manifest's rows {:?}", manifest.rows));
        }
        let loaded = load(&program).map_err(|refusal| format!("{}: {}", refusal.code, refusal.detail))?;
        Ok(Tool { manifest, program, admission, module: sha256(source.as_bytes()), loaded })
    }

    /// The 600-cell-Full tool.
    pub fn full_600_cell() -> Result<Tool, String> {
        Tool::new(SOURCE, manifest())
    }

    /// The producer of everything a run stores or proposes (BND.22).
    pub fn producer(&self) -> Producer {
        Producer {
            contract: contract(),
            checker: Some(checker()),
            author: Author::BuiltIn,
            module: self.module,
            entry: self.manifest.entry.into(),
            profile: Profile::Solving,
        }
    }

    /// The context of one run in the solving profile (BND.17, BND.22). `name` names the run in the proposal IDs and
    /// `base` is the revision the run reads: the acknowledgement's `base`. The word argument's block is that of a word
    /// the solver named.
    pub fn context(&self, name: &str, base: &str) -> Context {
        Context {
            run: name.into(),
            base: base.into(),
            producer: self.producer(),
            reads_state: self.admission.separation.reads_state,
            inputs: vec![named_word_block()],
            pins: pins(),
        }
    }

    /// Runs the entry on a literal word, the entry's pinned absolute argument, through a boundary onto `engine`, under
    /// [`LIMITS`].
    pub fn run_in(
        &self,
        word: &[i32],
        context: Context,
        engine: &mut dyn Host,
        store: &mut Store,
    ) -> Result<Ran, Refusal> {
        self.run_under(word, context, LIMITS, engine, store)
    }

    /// Runs the entry on a literal word through a boundary onto `engine` under `limits`: a job's run is under its
    /// budget's ([`limits`]). The run keeps no per-operation rows (`Options::record`): the meter does not charge them
    /// to the memory limit, so they would grow with the run's steps, and a quiet run has the same counters and stops.
    pub fn run_under(
        &self,
        word: &[i32],
        context: Context,
        limits: Amounts,
        engine: &mut dyn Host,
        store: &mut Store,
    ) -> Result<Ran, Refusal> {
        let mut boundary = Boundary::new(engine, store, context);
        let (module, entry) = (self.manifest.module, self.manifest.entry);
        let options = Options { record: false, ..Options::new(limits) };
        let run = self.loaded.run(module, entry, vec![Value::word(word)], &mut boundary, &options)?;
        Ok(Ran { run, shell: boundary.shell, captures: boundary.captures })
    }

    /// Runs the entry on a literal word in the solving profile with [`Tool::context`].
    pub fn run(
        &self,
        word: &[i32],
        name: &str,
        base: &str,
        engine: &mut dyn Host,
        store: &mut Store,
    ) -> Result<Ran, Refusal> {
        self.run_in(word, self.context(name, base), engine, store)
    }
}
