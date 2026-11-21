//! The process host (LP3, C5's next piece, acceptance item 2): the 600-cell-Full tool's program runs in
//! `c600-runtime`'s child process, and the same boundary and host bridge answer it on the command layer. Each test
//! names the part it covers; no conformance ID runs whole here.

mod process_support;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

use c600_check::{Profile, NOT_GRANTED};
use c600_command::{Budget, Caller, Error as LayerError, HeadlessFrames, Status};
use c600_kernel::codes::RECIPE_SHAPE;
use c600_kernel::json::{self, Value as Json};
use c600_run::boundary::{Answer, Boundary, Store};
use c600_run::interp::code::NONCANONICAL_VALUE;
use c600_run::meter::code::STEPS;
use c600_run::meter::Outcome;
use c600_run::value::{equal, Value};
use c600_runtime::{Config, Ended, FaultKind, Runtime, Stage};
use c600_tools::bridge::{base, word_args, Bridge};
use c600_tools::process::{invoke_tool, job, runtime_beside, start_error, Cause, ProcessHost, IN_PROCESS, PROCESS};
use c600_tools::tool::{class_rows, limits, Tool, LIMITS, MODULE, ROW, SOURCE};
use process_support::{acknowledgement, fingerprint, process_host, report, repository, run_on, runtime, session};

/// Acceptance item 2, the process host. The tool's program runs in another operating-system process than the
/// command layer, and one child serves every run. For a word the layer admits, a word the layer refuses and a
/// longer word, the run on the process host has the same meter summary, value and output (the fixed
/// acknowledgement), the same shell record, the same single runtime invocation of `operation.preview` (command ID,
/// caller, epoch, arguments, no token) and the same receipt as the run on the in-process host; the report of the
/// solver's command that ran the tool, the preview the layer then builds and the commit the shell's chord makes are
/// the same, and so are the state, the revision and the journal length. On both hosts the run is that command's job
/// (finding B2 of the KN5 boundary attack, ONE.34).
///
/// RTE.21 and ONE.69 (the runtime in a process of its own), KN5.15 (the child holds no token: the envelope it causes
/// has none) and BND.42 (the fixed acknowledgement). CT-BND-11 and CT-BND-07 (part, the 600-cell-Full tool on the
/// process host: the program gets only the fixed acknowledgement whatever the layer answers).
#[test]
fn process_host_runs_the_tool_in_the_runtimes_child_with_the_same_bridge_and_receipts() {
    let tool = Tool::full_600_cell().unwrap();
    let mut host = process_host();
    let long: Vec<i32> = (1..=600).flat_map(|letter| [letter, -(letter % 7 + 1)]).collect();
    for (case, word) in [("admitted", &[1, -2][..]), ("refused by the layer", &[][..]), ("long", &long[..])] {
        let mut seen = Vec::new();
        for on_process in [false, true] {
            let (mut layer, mut shell) = session("process");
            let read = base(layer.revision());
            let before = fingerprint(&layer);
            let (ran, sent, command) = run_on(on_process.then_some(&mut host), &tool, &mut layer, &mut shell, word);
            let run = command.run();
            let run = run.as_str();
            assert_eq!(run, "job-1", "{case}: the run is the job of the solver's command");
            let ran = ran.unwrap_or_else(|stopped| panic!("{case}: the run ends: {:?}", stopped.cause));
            assert_eq!(ran.run.summary.outcome, Outcome::Completed, "{case}");
            assert_eq!(ran.run.output.as_deref(), Some(&acknowledgement(&read, run)[..]), "{case}");
            let [shown] = &ran.shell[..] else { panic!("{case}: one proposal") };
            assert_eq!((shown.id.as_str(), &shown.word[..], shown.target.as_str()), (&*format!("{run}/1"), word, ROW));
            let [one] = &sent[..] else { panic!("{case}: one invocation") };
            let invocation = &one.invocation;
            assert_eq!((invocation.command.as_str(), invocation.row.as_str()), (&*format!("{run}/1"), ROW), "{case}");
            assert_eq!(one.caller, Caller::Runtime { run: run.into() }, "{case}");
            assert!(invocation.token.is_none(), "{case}");
            assert_eq!(invocation.args.canonical(), word_args(word).canonical(), "{case}");
            assert_eq!(fingerprint(&layer), before, "{case}: a preview changes no state");
            let mut after = Vec::new();
            match &one.receipt.status {
                Status::Running(id) if case != "refused by the layer" => {
                    assert_eq!(layer.run_jobs(), 1, "{case}");
                    after.push(report(&mut layer, command).to_json().canonical());
                    HeadlessFrames::after(&layer).present(&mut shell, &mut layer, *id).unwrap();
                    after.push(layer.preview().expect("the presented preview").summary().canonical());
                    let committed = shell.press(&mut layer, "Ctrl+Shift+Enter", false).unwrap().unwrap();
                    assert!(matches!(committed.status, Status::Committed(_)), "{case}: {committed:?}");
                    after.push(committed.to_json().canonical());
                }
                Status::Refused(error) if case == "refused by the layer" => {
                    assert_eq!(error.code, RECIPE_SHAPE, "{case}");
                    assert!(matches!(&shown.answer, Answer::Refused(r) if r.code == RECIPE_SHAPE), "{case}");
                    after.push(report(&mut layer, command).to_json().canonical());
                }
                other => panic!("{case}: {other:?}"),
            }
            let value = ran.run.value.clone();
            let shell_record = format!("{:?}", ran.shell);
            let envelope = (invocation.command.clone(), invocation.epoch, invocation.args.canonical());
            seen.push((
                ran.run.summary,
                value,
                shell_record,
                envelope,
                one.receipt.clone(),
                after,
                fingerprint(&layer),
            ));
        }
        let [(summary, value, shell, envelope, receipt, after, end), process] = &seen[..] else { unreachable!() };
        assert_eq!(summary, &process.0, "{case}: the meter summaries differ between the hosts");
        match (value, &process.1) {
            (Some(a), Some(b)) => assert!(equal(a, b), "{case}: the values differ between the hosts"),
            (a, b) => assert!(a.is_none() && b.is_none(), "{case}: one host returned no value"),
        }
        assert_eq!(shell, &process.2, "{case}: the shell records differ between the hosts");
        assert_eq!(envelope, &process.3, "{case}: the invocations differ between the hosts");
        assert_eq!(receipt, &process.4, "{case}: the receipts differ between the hosts");
        assert_eq!(after, &process.5, "{case}: the previews or commits differ between the hosts");
        assert_eq!(end, &process.6, "{case}: the sessions end differently on the two hosts");
    }
    let child = host.child_id().expect("the child is alive between runs");
    assert_ne!(child, std::process::id(), "the program ran in another process");
    assert_eq!(host.spawns(), 1, "one child served every run");
}

/// Acceptance item 2, what crosses to the child (KN5.15, RTE.21): the job is the admitted source's bytes, the
/// manifest's module and entry, the literal word as the one argument, the tool's limits, no grant and no
/// declassifier. The job runs in the solving profile and names the tool's class rows (`JobOptions::built_in`), so the
/// child admits the entry in the built-in column itself, with `c600_check::admit_built_in`, as `Tool::new` does in the
/// caller. The child's admission decides: the same job is refused at the child's admit stage, and never runs, without
/// the class rows (the solving column refuses every previewing program), with class rows that leave out
/// `operation.preview` (the class lists every row the entry uses), and with the class rows in the tool-authoring
/// profile (the built-in column is the solving profile's, so that refusal has no code). A source other than the one
/// the tool was admitted from is not sent at all.
///
/// ADR-L02 and LI.30 (part: the built-in column's admission, in the caller at `Tool::new` and in the child).
#[test]
fn process_host_job_carries_the_admitted_source_and_the_child_admits_it_in_the_built_in_column() {
    let tool = Tool::full_600_cell().unwrap();
    let built = job(&tool, SOURCE, &[1, -2]);
    assert_eq!(built.sources, vec![SOURCE.as_bytes().to_vec()]);
    assert_eq!((built.module.as_str(), built.entry.as_str()), (MODULE, tool.manifest.entry));
    assert!(matches!(&built.args[..], [word] if equal(word, &Value::word(&[1, -2]))));
    assert_eq!(built.options.profile, Profile::Solving);
    assert_eq!(built.options.built_in, Some(class_rows(&tool.manifest)));
    assert_eq!(built.options.limits, LIMITS);
    assert!(built.options.grant.is_empty() && built.options.declassifiers.is_empty() && built.options.seed.is_none());

    let (mut layer, mut shell) = session("process");
    let before = fingerprint(&layer);
    let mut runtime = Runtime::new(Config::new(process_support::runtime()));
    let rows = built.options.built_in.clone();
    let variants = [
        ("the job", Profile::Solving, rows.clone(), None),
        ("no class rows", Profile::Solving, None, Some("prepares no preview in a solving session")),
        ("no preview row", Profile::Solving, Some(BTreeSet::new()), Some("the class lists no row")),
        ("tool authoring", Profile::ToolAuthoring, rows, Some("the built-in column is the solving profile's")),
    ];
    for (case, profile, class, refusal) in variants {
        let mut asked = job(&tool, SOURCE, &[1, -2]);
        asked.options.profile = profile;
        asked.options.built_in = class;
        let context = tool.context("profiles", &base(layer.revision()));
        let mut bridge = Bridge::new(&mut layer, "profiles");
        let mut store = Store::default();
        let mut boundary = Boundary::new(&mut bridge, &mut store, context);
        let ended = runtime.run(&asked, &mut boundary, None).expect("the child answers");
        let proposals = boundary.shell.len();
        match (ended, refusal) {
            (Ended::Ran(run), None) => assert_eq!((run.summary.outcome, proposals), (Outcome::Completed, 1)),
            (Ended::Refused(refused), Some(why)) => {
                assert_eq!((refused.stage, proposals), (Stage::Admit, 0), "{case}: {refused:?}");
                assert!(refused.detail.contains(why), "{case}: {}", refused.detail);
                let codes = &refused.codes;
                match profile {
                    Profile::Solving => {
                        assert!(!codes.is_empty() && codes.iter().all(|c| c == NOT_GRANTED), "{case}: {codes:?}")
                    }
                    _ => assert!(codes.is_empty(), "{case}: {codes:?}"),
                }
                assert!(bridge.sent.is_empty(), "{case}");
            }
            (other, _) => panic!("{case}: {other:?}"),
        }
    }
    assert_eq!(fingerprint(&layer), before);

    let mut host = process_host();
    let other = SOURCE.replacen("preview_word", "preview_other", 1);
    assert_ne!(other, SOURCE);
    let command = invoke_tool(&mut shell, &mut layer, &tool, &[1, -2]).expect("the tool's row");
    let mut bridge = Bridge::new(&mut layer, &command.run());
    let stopped = host.run(&tool, &other, &command, &mut bridge, &mut Store::default()).unwrap_err();
    assert!(matches!(stopped.cause, Cause::OtherSource) && stopped.shell.is_empty() && bridge.sent.is_empty());
    assert_eq!((host.spawns(), host.child_id()), (0, None), "no child was started for another source");
}

/// Acceptance item 2, a word both hosts refuse alike. A word with a letter outside 1 to 1200 in magnitude has no
/// transfer encoding (C3's wire: a canonical word), so on the process host the parent refuses its job before any child
/// is started, and in process `run_in_process` refuses the run before the tool runs, with the same check. Both stop at
/// the start stage with the interpreter's non-canonical value code: no proposal, no invocation, no change. The host
/// reports the solver's command that ran the tool as failed with that code. These are O07's steps 0, 1, 2 and 8.
///
/// CT-LB2-03 (part: a word the transfer cannot carry is refused whole, never altered or split).
#[test]
fn both_hosts_refuse_a_word_with_no_transfer_encoding_at_the_start_stage() {
    let tool = Tool::full_600_cell().unwrap();
    let mut host = process_host();
    for word in [&[0][..], &[1201], &[-1201], &[1, 0], &[i32::MIN], &[i32::MAX]] {
        for on_process in [false, true] {
            let (mut layer, mut shell) = session("process");
            let before = fingerprint(&layer);
            let (ran, sent, command) = run_on(on_process.then_some(&mut host), &tool, &mut layer, &mut shell, word);
            let stopped = ran.err().unwrap_or_else(|| panic!("{word:?} ran (process host: {on_process})"));
            assert!(stopped.cause.no_transfer(), "{word:?}: {:?}", stopped.cause);
            let Cause::Refused(refused) = &stopped.cause else { unreachable!() };
            assert_eq!(refused.stage, Stage::Start);
            assert!(stopped.shell.is_empty() && sent.is_empty(), "{word:?}: nothing reached the boundary or the layer");
            let error = start_error(&stopped.cause).expect("the start-stage refusal's error");
            let reported = command.finish(&mut layer, Err(error)).expect("the layer holds the tool's job");
            assert_eq!(reported.status.code(), Some(NONCANONICAL_VALUE), "{word:?}: {reported:?}");
            assert_eq!(fingerprint(&layer), before);
        }
    }
    assert_eq!(host.spawns(), 0, "a job with no transfer encoding starts no child");
}

/// The run's budget (KN7.7; LP2's pull request 348). The solver's command declares the budget its job runs under, and
/// on both hosts the tool's entry runs under the run limits of the job's budget, not under the ceiling: with a
/// budget of one step the run is cut off with the meter's steps code before it proposes anything, alike on the two
/// hosts. The host reports the job as failed with that code, and the session is unchanged. The shell's default
/// budget, every profile's ceiling, gives the tool's limits.
#[test]
fn both_hosts_run_the_tool_under_the_budget_its_command_declared() {
    assert_eq!(limits(Budget::CEILING), LIMITS, "the shell's default budget gives the tool's limits");
    let tool = Tool::full_600_cell().unwrap();
    let mut host = process_host();
    let one = Budget { steps: 1, ..Budget::CEILING };
    let mut summaries = Vec::new();
    for on_process in [false, true] {
        let (mut layer, mut shell) = session("budget");
        shell.budget = Some(one);
        let before = fingerprint(&layer);
        let (ran, sent, command) = run_on(on_process.then_some(&mut host), &tool, &mut layer, &mut shell, &[1, -2]);
        assert_eq!((command.job.budget, command.limits()), (one, limits(one)), "the job runs under its budget");
        let ran =
            ran.unwrap_or_else(|stopped| panic!("the run ends (process host: {on_process}): {:?}", stopped.cause));
        let summary = ran.run.summary;
        assert_eq!((summary.outcome, summary.code), (Outcome::CutOff, Some(STEPS)), "{summary:?}");
        assert_eq!(summary.used.steps, 1, "the run used its one step");
        assert!(ran.shell.is_empty() && sent.is_empty(), "the run proposed nothing (process host: {on_process})");
        let error = LayerError::new(STEPS, "the run was cut off at its budget's steps");
        let reported = command.finish(&mut layer, Err(error)).expect("the layer holds the tool's job");
        assert!(matches!(reported.status, Status::Failed(_)), "the job failed: {reported:?}");
        assert_eq!(reported.status.code(), Some(STEPS), "{reported:?}");
        assert!(layer.take_program_job().is_none(), "no job is left queued (process host: {on_process})");
        assert_eq!(fingerprint(&layer), before, "a cut-off run changes nothing");
        summaries.push(summary);
    }
    assert_eq!(summaries[0], summaries[1], "the meter summaries differ between the hosts");
}

/// KN7.13 (part, the tool's host): a runtime that cannot be started leaves no run, no invocation and no change, and
/// the caller gets the fault, not a result.
#[test]
fn process_host_reports_a_runtime_that_does_not_start() {
    let tool = Tool::full_600_cell().unwrap();
    let missing = runtime_beside(&repository().join("rust").join("c600-tools").join("no-such-binary"));
    assert!(!missing.exists());
    let mut host = ProcessHost::new(missing);
    let (mut layer, mut shell) = session("process");
    let before = fingerprint(&layer);
    let (ran, sent, _command) = run_on(Some(&mut host), &tool, &mut layer, &mut shell, &[1, -2]);
    let stopped = ran.expect_err("no run without a runtime");
    assert!(matches!(&stopped.cause, Cause::Lost(fault) if fault.kind == FaultKind::Spawn), "{:?}", stopped.cause);
    assert!(stopped.shell.is_empty() && sent.is_empty());
    assert_eq!(fingerprint(&layer), before);
}

fn source_route(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_c600-lp3-source")).args(arguments).output().expect("the runner starts")
}

fn read_json(path: &PathBuf) -> Json {
    json::parse(&std::fs::read(path).expect("the file was written")).expect("the file is JSON")
}

/// Acceptance item 2, the runner's flag and records. `c600-lp3-source trace --host process` replays O06 (protected
/// orbits, undo and redo) and O07 (invalid inputs) with the tool in the runtime's child: the traces are
/// byte-identical to the in-process host's, the summary line, each case and each step given to a host name the host,
/// and the receipts files are the same but for the host's name. On both hosts O07's four words with no canonical
/// form are refused at the run's start stage: the step record keeps the kernel's refusal of the recipe and the
/// start-stage refusal, and the step's two receipts are the solver's command that ran the tool and its report,
/// failed with the non-canonical value code. A host name the runner does not know, and `--runtime` without the
/// process host, are usage errors.
#[test]
fn process_host_flag_replays_cases_with_the_same_traces_and_names_the_host_in_every_record() {
    let assets = repository().join("assets");
    let inputs = repository().join("work").join("experiments").join("language-lx").join("inputs").join("oracle");
    let folder = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("c5-process-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let text = |path: PathBuf| path.to_str().expect("a UTF-8 path").to_owned();
    let runtime = text(runtime());
    let mut outputs = Vec::new();
    for host in [IN_PROCESS, PROCESS] {
        let receipts = folder.join(format!("{host}-receipts.json"));
        let mut arguments = vec!["trace".to_owned(), "--assets".into(), text(assets.clone()), "--host".into()];
        arguments.push(host.into());
        if host == PROCESS {
            arguments.extend(["--runtime".to_owned(), runtime.clone()]);
        }
        arguments.extend(["--receipts".to_owned(), text(receipts.clone())]);
        let mut traces = Vec::new();
        for case in ["O06", "O07"] {
            let trace = folder.join(format!("{host}-{case}.trace.json"));
            arguments.extend([text(inputs.join(format!("{case}.json.gz"))), text(trace.clone())]);
            traces.push(trace);
        }
        let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
        let out = source_route(&arguments);
        assert!(out.status.success(), "{host}: {}", String::from_utf8_lossy(&out.stderr));
        let summary = json::parse(out.stdout.trim_ascii_end()).expect("one JSON summary line");
        assert_eq!(summary.get("host").and_then(Json::as_str), Some(host));
        let spawns = summary.get("runtime_spawns").and_then(Json::as_i64);
        assert_eq!(spawns, (host == PROCESS).then_some(1), "{host}: one child serves every case");
        let traces: Vec<Vec<u8>> = traces.iter().map(|path| std::fs::read(path).unwrap()).collect();
        outputs.push((traces, read_json(&receipts)));
    }
    let _ = std::fs::remove_dir_all(&folder);
    let [(in_traces, in_receipts), (process_traces, process_receipts)] = &outputs[..] else { unreachable!() };
    assert_eq!(in_traces, process_traces, "the traces differ between the hosts");

    let cases = |receipts: &Json| receipts.get("cases").and_then(Json::as_array).expect("cases").to_vec();
    let codes = Json::Array(vec![Json::String(NONCANONICAL_VALUE.into())]).canonical();
    let mut start_refused = Vec::new();
    for (a, b) in cases(in_receipts).iter().zip(cases(process_receipts)) {
        let case = a.get("case_id").and_then(Json::as_str).unwrap().to_owned();
        assert_eq!(a.get("host").and_then(Json::as_str), Some(IN_PROCESS));
        assert_eq!(b.get("host").and_then(Json::as_str), Some(PROCESS));
        for name in ["case_id", "receipts", "callers", "tail"] {
            assert_eq!(a.get(name).map(Json::canonical), b.get(name).map(Json::canonical), "{case}: {name}");
        }
        let list = |value: &Json, name: &str| value.get(name).and_then(Json::as_array).expect(name).to_vec();
        let (steps_a, steps_b) = (list(a, "steps"), list(&b, "steps"));
        assert_eq!(steps_a.len(), steps_b.len(), "{case}");
        for (ra, rb) in steps_a.iter().zip(&steps_b) {
            let field = |record: &Json, name: &str| record.get(name).cloned().unwrap_or_else(|| panic!("{name}"));
            for name in [
                "index",
                "receipts",
                "literal",
                "letters",
                "expansion_refusal",
                "start_refusal",
                "draft",
                "case_draft",
                "rebased_result",
            ] {
                assert_eq!(field(ra, name).canonical(), field(rb, name).canonical(), "{case}: {name}");
            }
            let given = !matches!(field(ra, "literal"), Json::Null);
            assert_eq!(field(ra, "host").as_str(), given.then_some(IN_PROCESS), "{case}");
            assert_eq!(field(rb, "host").as_str(), given.then_some(PROCESS), "{case}");
            let start = field(ra, "start_refusal");
            if !matches!(start, Json::Null) {
                // The solver's command and its failed report: the run stopped before its proposal, and the record
                // keeps the kernel's refusal.
                assert_eq!(field(ra, "receipts").as_i64(), Some(2), "{case}");
                assert_eq!(start.get("stage").and_then(Json::as_str), Some("start"), "{case}");
                assert_eq!(start.get("codes").map(Json::canonical), Some(codes.clone()), "{case}");
                let kernel = start.get("recipe_refusal").and_then(|kernel| kernel.get("code"));
                assert!(kernel.and_then(Json::as_str).is_some_and(|code| code.starts_with("KN2:")), "{case}");
                start_refused.push((case.clone(), field(ra, "index").as_i64().unwrap()));
            }
        }
    }
    let expected: Vec<(String, i64)> = [0, 1, 2, 8].iter().map(|&index| ("O07".to_owned(), index)).collect();
    assert_eq!(start_refused, expected, "only O07's words with no transfer encoding stop at the start stage");

    for arguments in [
        &["trace", "--assets", "x", "--host", "elsewhere", "a", "b"][..],
        &["trace", "--assets", "x", "--runtime", "y", "a", "b"],
    ] {
        let out = source_route(arguments);
        assert_eq!(out.status.code(), Some(2), "{arguments:?}");
        assert!(String::from_utf8_lossy(&out.stderr).contains("usage:"), "{arguments:?}");
    }
}
