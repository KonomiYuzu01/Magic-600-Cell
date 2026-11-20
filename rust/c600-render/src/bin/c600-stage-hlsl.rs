//! `c600-stage-hlsl --source <stage.6cs> --limits <limits.json> --out <dir> [--entry <name>] [--name <name>]`
//!
//! Reads one stage module in source and the host's limits, parses and lowers it with c600-syntax, admits it with
//! `c600_check::admit_stage` and takes it up with `c600_display::stage::take_up`, then writes `<out>/<name>.hlsl`
//! (`m600-stage-hlsl/1`) and `<out>/<name>.json` (the record of `c600_render::stage::record`). `--entry` defaults to
//! `main` and `--name` to the source's file stem (draft). The stage module is the source's one module of kind stage.
//!
//! Exit codes (draft): 0 written; 1 the stage is refused (a parse finding, a lowering error, a checker or binding
//! refusal), each printed to stderr as `<code>: <summary>` with its context; 2 a usage, limits or file error.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use c600_display::Error;
use c600_ir::ModuleKind;
use c600_render::stage::{generate, parse_limits, record};

struct Args {
    source: PathBuf,
    limits: PathBuf,
    out: PathBuf,
    entry: String,
    name: Option<String>,
}

const USAGE: &str =
    "usage: c600-stage-hlsl --source <stage.6cs> --limits <limits.json> --out <dir> [--entry <name>] [--name <name>]";

fn args() -> Result<Args, String> {
    let mut source = None;
    let mut limits = None;
    let mut out = None;
    let mut entry = None;
    let mut name = None;
    let mut it = std::env::args_os().skip(1);
    while let Some(flag) = it.next() {
        let flag = flag.into_string().map_err(|_| "an argument is no UTF-8 text".to_string())?;
        let slot = match flag.as_str() {
            "--source" | "--limits" | "--out" | "--entry" | "--name" => flag,
            other => return Err(format!("unknown argument `{other}`")),
        };
        let value = it.next().ok_or_else(|| format!("`{slot}` needs a value"))?;
        let previous = match slot.as_str() {
            "--source" => source.replace(PathBuf::from(value)).is_some(),
            "--limits" => limits.replace(PathBuf::from(value)).is_some(),
            "--out" => out.replace(PathBuf::from(value)).is_some(),
            _ => {
                let text = value.into_string().map_err(|_| format!("`{slot}` is no UTF-8 text"))?;
                if slot == "--entry" { entry.replace(text) } else { name.replace(text) }.is_some()
            }
        };
        if previous {
            return Err(format!("`{slot}` is given twice"));
        }
    }
    Ok(Args {
        source: source.ok_or("`--source` is missing")?,
        limits: limits.ok_or("`--limits` is missing")?,
        out: out.ok_or("`--out` is missing")?,
        entry: entry.unwrap_or_else(|| "main".into()),
        name,
    })
}

fn print_error(error: &Error) {
    let context: Vec<String> = error.context.iter().map(|(k, v)| format!("{k}={v}")).collect();
    if context.is_empty() {
        eprintln!("{error}");
    } else {
        eprintln!("{error} [{}]", context.join(", "));
    }
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
}

fn run() -> Result<ExitCode, String> {
    let args = args().map_err(|e| format!("{e}\n{USAGE}"))?;
    let limits = parse_limits(&read(&args.limits)?)?;
    let file = c600_syntax::parse(&read(&args.source)?);
    if !file.diagnostics.is_empty() {
        for d in &file.diagnostics {
            match d.span {
                Some(s) => eprintln!("{}: {}:{}: {}", d.code, s.line, s.column, d.message),
                None => eprintln!("{}: {}", d.code, d.message),
            }
        }
        return Ok(ExitCode::from(1));
    }
    let lowered = match c600_syntax::lower(std::slice::from_ref(&file)) {
        Ok(lowered) => lowered,
        Err(e) => {
            eprintln!("lowering (no code): {e}");
            return Ok(ExitCode::from(1));
        }
    };
    let stages: Vec<&str> = lowered
        .program
        .modules
        .iter()
        .filter(|m| matches!(m.kind, ModuleKind::Stage(_)))
        .map(|m| m.name.as_str())
        .collect();
    let [module] = stages.as_slice() else {
        return Err(format!("the source holds {} stage modules, and one is needed", stages.len()));
    };
    let admission =
        c600_check::admit_stage(&lowered.program, module, &args.entry, &limits).map_err(|e| e.to_string())?;
    if let Err(errors) = c600_display::stage::take_up(&admission) {
        errors.iter().for_each(print_error);
        return Ok(ExitCode::from(1));
    }
    let Some(stage) = &admission.stage else { return Err("the admission holds no stage".into()) };
    let generated = match generate(stage) {
        Ok(generated) => generated,
        Err(errors) => {
            errors.iter().for_each(print_error);
            return Ok(ExitCode::from(1));
        }
    };
    let name = match args.name {
        Some(name) => name,
        None => args
            .source
            .file_stem()
            .and_then(|s| s.to_str())
            .map(str::to_string)
            .ok_or("the source's file stem is no UTF-8 text; give `--name`")?,
    };
    if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
        return Err(format!("`{name}` is no plain file name"));
    }
    std::fs::create_dir_all(&args.out).map_err(|e| format!("cannot create {}: {e}", args.out.display()))?;
    let hlsl = args.out.join(format!("{name}.hlsl"));
    let json = args.out.join(format!("{name}.json"));
    std::fs::write(&hlsl, generated.hlsl.as_bytes()).map_err(|e| format!("cannot write {}: {e}", hlsl.display()))?;
    std::fs::write(&json, record(stage, &generated)).map_err(|e| format!("cannot write {}: {e}", json.display()))?;
    println!("{}", hlsl.display());
    println!("{}", json.display());
    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    run().unwrap_or_else(|e| {
        eprintln!("c600-stage-hlsl: {e}");
        ExitCode::from(2)
    })
}
