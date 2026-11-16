//! The process host (LP3, C5's next piece, acceptance item 2): the 600-cell-Full tool's program runs in
//! `c600-runtime`'s child process, and the same boundary and host bridge answer it on the command layer. Each test
//! names the part it covers; no conformance ID runs whole here.

mod process_support;

use std::path::PathBuf;
use std::process::Command;

use c600_check::Profile;
use c600_command::{Caller, Status};
use c600_kernel::codes::RECIPE_SHAPE;
use c600_kernel::json::{self, Value as Json};
use c600_run::boundary::{Answer, Boundary, Store};
use c600_run::meter::Outcome;
use c600_run::value::{equal, Value};
use c600_runtime::{Config, Ended, FaultKind, Runtime, Stage};
use c600_tools::bridge::{base, word_args, Bridge};
use c600_tools::process::{job, runtime_beside, Cause, ProcessHost, IN_PROCESS, PROCESS};
use c600_tools::tool::{Tool, LIMITS, MODULE, ROW, SOURCE};
use process_support::{acknowledgement, fingerprint, process_host, repository, run_on, runtime, session};

/// Acceptance item 2, the process host. The tool's program runs in another operating-system process than the
/// command layer, and one child serves every run. For a word the layer admits, a word the layer refuses and a
/// longer word, the run on the process host has the same meter summary, value and output (the fixed
/// acknowledgement), the same shell record, the same single runtime invocation of `operation.preview` (command ID,
/// caller, epoch, arguments, no token) and the same receipt as the run on the in-process host; the preview the layer
/// then builds and the commit the shell's chord makes are the same, and so are the state, the revision and the
/// journal length.
///
/// RTE.21 and ONE.69 (the runtime in a process of its own), KN5.15 (the child holds no token: the envelope it causes
/// has none) and BND.42 (the fixed acknowledgement). CT-BND-11 and CT-BND-07 (part, the 600-cell-Full tool on the
/// process host: the program gets only the fixed acknowledgement whatever the layer answers).
#[test]
fn process_host_runs_the_tool_in_the_runtimes_child_with_the_same_bridge_and_receipts() {
    let tool = Tool::full_600_cell().unwrap();
    let mut host = process_host();
    let long: Vec<i32> = (1..=600).flat_map(|letter| [letter, -(letter % 7 + 1)]).collect();
    for (case, word, run) in
        [("admitted", &[1, -2][..], "run-a"), ("refused by the layer", &[][..], "run-b"), ("long", &long[..], "run-c")]
    {
        let mut seen = Vec::new();
        for on_process in [false, true] {
            let (mut layer, mut shell) = session("process");
            let read = base(layer.revision());
            let before = fingerprint(&layer);
            let (ran, sent) = run_on(on_process.then_some(&mut host), &tool, &mut layer, word, run);
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
                    shell.present(&mut layer, *id).unwrap();
                    after.push(layer.preview().expect("the presented preview").summary().canonical());
                    let committed = shell.press(&mut layer, "Ctrl+Shift+Enter", false).unwrap().unwrap();
                    assert!(matches!(committed.status, Status::Committed(_)), "{case}: {committed:?}");
                    after.push(committed.to_json().canonical());
                }
                Status::Refused(error) if case == "refused by the layer" => {
                    assert_eq!(error.code, RECIPE_SHAPE, "{case}");
                    assert!(matches!(&shown.answer, Answer::Refused(r) if r.code == RECIPE_SHAPE), "{case}");
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
/// declassifier. The job's profile is tool authoring, because the child applies `c600_check::admit`, which has no
/// built-in column: the same job in the solving profile is refused at the child's admit stage and never runs, so no
/// proposal is made. A source other than the one the tool was admitted from is not sent at all.
///
/// ADR-L02 and LI.30 (part: the built-in column's admission is the parent's, at `Tool::new`; the child's own
/// admission of the built-in tool is not built, see the crate's drafts).
#[test]
fn process_host_job_carries_the_admitted_source_and_the_child_admits_it_as_tool_authoring() {
    let tool = Tool::full_600_cell().unwrap();
    let built = job(&tool, SOURCE, &[1, -2]);
    assert_eq!(built.sources, vec![SOURCE.as_bytes().to_vec()]);
    assert_eq!((built.module.as_str(), built.entry.as_str()), (MODULE, tool.manifest.entry));
    assert!(matches!(&built.args[..], [word] if equal(word, &Value::word(&[1, -2]))));
    assert_eq!(built.options.profile, Profile::ToolAuthoring);
    assert_eq!(built.options.limits, LIMITS);
    assert!(built.options.grant.is_empty() && built.options.declassifiers.is_empty() && built.options.seed.is_none());

    let (mut layer, _) = session("process");
    let before = fingerprint(&layer);
    let mut runtime = Runtime::new(Config::new(process_support::runtime()));
    for (profile, runs) in [(Profile::Solving, false), (Profile::ToolAuthoring, true)] {
        let mut asked = job(&tool, SOURCE, &[1, -2]);
        asked.options.profile = profile;
        let context = tool.context("profiles", &base(layer.revision()));
        let mut bridge = Bridge::new(&mut layer, "profiles");
        let mut store = Store::default();
        let mut boundary = Boundary::new(&mut bridge, &mut store, context);
        let ended = runtime.run(&asked, &mut boundary, None).expect("the child answers");
        let proposals = boundary.shell.len();
        match ended {
            Ended::Ran(run) if runs => assert_eq!((run.summary.outcome, proposals), (Outcome::Completed, 1)),
            Ended::Refused(refused) if !runs => {
                assert_eq!((refused.stage, proposals), (Stage::Admit, 0), "{refused:?}");
                assert!(!refused.codes.is_empty());
                assert!(bridge.sent.is_empty());
            }
            other => panic!("{profile:?}: {other:?}"),
        }
    }
    assert_eq!(fingerprint(&layer), before);

    let mut host = process_host();
    let other = SOURCE.replacen("preview_word", "preview_other", 1);
    assert_ne!(other, SOURCE);
    let read = base(layer.revision());
    let mut bridge = Bridge::new(&mut layer, "other");
    let stopped = host.run(&tool, &other, &[1, -2], "other", &read, &mut bridge, &mut Store::default()).unwrap_err();
    assert!(matches!(stopped.cause, Cause::OtherSource) && stopped.shell.is_empty() && bridge.sent.is_empty());
    assert_eq!((host.spawns(), host.child_id()), (0, None), "no child was started for another source");
}

/// Acceptance item 2, the one place the hosts differ. A word with a letter outside 1 to 1200 in magnitude has no
/// transfer encoding (C3's wire: a canonical word), so on the process host its job is refused at the start stage
/// with the interpreter's non-canonical value code before any child is started: no proposal, no invocation, no
/// receipt, no change. On the in-process host the same word runs, the tool proposes it, and the command layer
/// refuses it with the kernel's recipe-shape code. Either way the word previews nothing; these are O07's steps 0, 1,
/// 2 and 8.
///
/// CT-LB2-03 (part: a word the transfer cannot carry is refused whole, never altered or split).
#[test]
fn process_host_refuses_a_word_with_no_transfer_encoding_at_the_start_stage() {
    let tool = Tool::full_600_cell().unwrap();
    let mut host = process_host();
    for word in [&[0][..], &[1201], &[-1201], &[1, 0]] {
        let (mut layer, _) = session("process");
        let before = fingerprint(&layer);
        let (ran, sent) = run_on(Some(&mut host), &tool, &mut layer, word, "start");
        let stopped = ran.err().unwrap_or_else(|| panic!("{word:?} ran on the process host"));
        assert!(stopped.cause.no_transfer(), "{word:?}: {:?}", stopped.cause);
        let Cause::Refused(refused) = &stopped.cause else { unreachable!() };
        assert_eq!(refused.stage, Stage::Start);
        assert!(stopped.shell.is_empty() && sent.is_empty(), "{word:?}: nothing reached the boundary or the layer");
        assert_eq!(fingerprint(&layer), before);

        let (ran, sent) = run_on(None, &tool, &mut layer, word, "start");
        assert_eq!(ran.unwrap().run.summary.outcome, Outcome::Completed);
        let [one] = &sent[..] else { panic!("{word:?}: one invocation in process") };
        assert_eq!(one.receipt.status.code(), Some(RECIPE_SHAPE), "{word:?}");
        assert_eq!(fingerprint(&layer), before);
    }
    assert_eq!(host.spawns(), 0, "a job with no transfer encoding starts no child");
}

/// KN7.13 (part, the tool's host): a runtime that cannot be started leaves no run, no invocation and no change, and
/// the caller gets the fault, not a result.
#[test]
fn process_host_reports_a_runtime_that_does_not_start() {
    let tool = Tool::full_600_cell().unwrap();
    let missing = runtime_beside(&repository().join("rust").join("c600-tools").join("no-such-binary"));
    assert!(!missing.exists());
    let mut host = ProcessHost::new(missing);
    let (mut layer, _) = session("process");
    let before = fingerprint(&layer);
    let (ran, sent) = run_on(Some(&mut host), &tool, &mut layer, &[1, -2], "lost");
    let stopped = ran.err().expect("no run without a runtime");
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
/// and the receipts are the same but for O07's four words with no transfer encoding, which leave the kernel's
/// refusal of the recipe and the start-stage refusal in the step record instead of a refusal receipt. A host name
/// the runner does not know, and `--runtime` without the process host, are usage errors.
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
    let mut start_refused = Vec::new();
    for (a, b) in cases(in_receipts).iter().zip(cases(process_receipts)) {
        let case = a.get("case_id").and_then(Json::as_str).unwrap().to_owned();
        assert_eq!(a.get("host").and_then(Json::as_str), Some(IN_PROCESS));
        assert_eq!(b.get("host").and_then(Json::as_str), Some(PROCESS));
        assert_eq!(a.get("tail").map(Json::canonical), b.get("tail").map(Json::canonical));
        let list = |value: &Json, name: &str| value.get(name).and_then(Json::as_array).expect(name).to_vec();
        let (mut ia, mut ib) = (0, 0);
        for (ra, rb) in list(a, "steps").iter().zip(list(&b, "steps")) {
            let field = |record: &Json, name: &str| record.get(name).cloned().unwrap_or_else(|| panic!("{name}"));
            for name in ["index", "literal", "letters", "expansion_refusal", "draft", "case_draft", "rebased_result"] {
                assert_eq!(field(ra, name).canonical(), field(&rb, name).canonical(), "{case}: {name}");
            }
            let given = !matches!(field(ra, "literal"), Json::Null);
            assert_eq!(field(ra, "host").as_str(), given.then_some(IN_PROCESS), "{case}");
            assert_eq!(field(&rb, "host").as_str(), given.then_some(PROCESS), "{case}");
            assert!(matches!(field(ra, "start_refusal"), Json::Null), "{case}: in process every word runs");
            let (na, nb) = (field(ra, "receipts").as_i64().unwrap(), field(&rb, "receipts").as_i64().unwrap());
            let (ea, eb) = (ia + na as usize, ib + nb as usize);
            let (receipts_a, receipts_b) = (&list(a, "receipts")[ia..ea], &list(&b, "receipts")[ib..eb]);
            let (callers_a, callers_b) = (&list(a, "callers")[ia..ea], &list(&b, "callers")[ib..eb]);
            match field(&rb, "start_refusal") {
                Json::Null => {
                    let same = |x: &[Json], y: &[Json]| {
                        x.len() == y.len() && x.iter().zip(y).all(|(x, y)| x.canonical() == y.canonical())
                    };
                    assert!(same(receipts_a, receipts_b), "{case}: the receipts differ between the hosts");
                    assert!(same(callers_a, callers_b), "{case}: the callers differ between the hosts");
                }
                start => {
                    // The in-process host's one receipt is the layer's refusal with the kernel's refusal of the recipe.
                    assert_eq!((na, nb), (1, 0), "{case}");
                    assert_eq!(start.get("stage").and_then(Json::as_str), Some("start"));
                    let kernel = start.get("recipe_refusal").expect("the kernel's refusal of the recipe");
                    let error = receipts_a[0].get("status").and_then(|status| status.get("error"));
                    assert_eq!(error.map(Json::canonical), Some(kernel.canonical()), "{case}");
                    start_refused.push((case.clone(), field(ra, "index").as_i64().unwrap()));
                }
            }
            (ia, ib) = (ea, eb);
        }
    }
    let expected: Vec<(String, i64)> = [0, 1, 2, 8].iter().map(|&index| ("O07".to_owned(), index)).collect();
    assert_eq!(start_refused, expected, "only O07's words with no transfer encoding differ");

    for arguments in [
        &["trace", "--assets", "x", "--host", "elsewhere", "a", "b"][..],
        &["trace", "--assets", "x", "--runtime", "y", "a", "b"],
    ] {
        let out = source_route(arguments);
        assert_eq!(out.status.code(), Some(2), "{arguments:?}");
        assert!(String::from_utf8_lossy(&out.stderr).contains("usage:"), "{arguments:?}");
    }
}
