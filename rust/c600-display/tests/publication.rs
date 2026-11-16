//! The host publication state machine (part H): CT-RM-03 with RM.32's supersession and latch vectors, RM.35's paired
//! ceiling vectors on the two sides of acceptance and RM.36's three device-loss contexts, and CT-RPI-04's late older
//! completion. A mock backend drives the host; every result is a source fixture outcome, not a backend run.

mod common;

use c600_display::codes;
use c600_display::host::ResourceSet;
use c600_display::pick::{PickMode, PickReply, Presented, PreviewShown, Requested};
use c600_display::plan::*;
use c600_display::receipt::{is_reserved, CancelReason, JobOutcome, Status};
use c600_display::validate::validate;
use c600_display::Form;

use common::*;

const V1: ViewId = ViewId(1);

fn shown(id: u64, base: &RevisionRef) -> PreviewShown {
    let display = preview(id, base);
    PreviewShown { preview: display.preview, literal: display.literal, base: base.state }
}

#[test]
fn ct_rm_03_pending_supersession_lets_only_the_newer_install() {
    let base = revision(12);
    let mut host = host(&base);
    assert_eq!(host.publish(CommandId(1), sheet(1, &base, 2)).status, Status::Accepted);
    let late = host.ticket(CommandId(1)).expect("A is prepared");
    assert_eq!(host.publish(CommandId(2), sheet_from(1, &base, 10, 3)).status, Status::Accepted);
    assert_eq!(status(&host, 1), Status::Cancelled(CancelReason::Superseded { by: CommandId(2) }));

    host.backend.finish(late);
    host.pump();
    assert!(host.installed(V1).is_none(), "A's late preparation yields no usable resources");
    finish(&mut host, 2);
    assert_eq!(host.installed(V1).expect("B").command, CommandId(2));
    assert_eq!(host.instances(V1), vec![InstanceId(10), InstanceId(11), InstanceId(12)]);
    assert_eq!(status(&host, 1), Status::Cancelled(CancelReason::Superseded { by: CommandId(2) }));
    one_terminal_each(&host);
}

#[test]
fn ct_rpi_04_an_older_completion_cannot_replace_a_newer_one() {
    let base = revision(12);
    let mut host = host(&base);
    host.publish(CommandId(1), sheet(1, &base, 2));
    let older = host.ticket(CommandId(1)).expect("prepared");
    install(&mut host, 2, sheet_from(1, &base, 10, 1));
    let newer = host.installed(V1).cloned().expect("installed");
    host.backend.finish(older);
    host.pump();
    assert_eq!(host.installed(V1), Some(&newer), "the newer publication stays installed");
    assert_eq!(host.view(V1).expect("view").publication_generation, 1);
    one_terminal_each(&host);
}

#[test]
fn ct_rm_03_completed_supersession_keeps_both_completions_in_order() {
    let base = revision(12);
    let mut host = host(&base);
    install(&mut host, 1, sheet(1, &base, 2));
    let first = host.receipt(CommandId(1)).expect("A");
    let resources = host.installed(V1).expect("A").resources;
    install(&mut host, 2, sheet_from(1, &base, 10, 1));
    assert_eq!(host.receipt(CommandId(1)), Some(first.clone()), "A's terminal never changes");
    let generations: Vec<u64> = host
        .events()
        .iter()
        .map(|(_, status)| match status {
            Status::Completed(completion) => completion.result.covered.publication_generation,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(generations, vec![1, 2]);
    assert_eq!(host.events().iter().map(|(c, _)| c.0).collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(host.backend.retired, vec![resources], "A's resources retire after B installs");
    assert_eq!(host.installed(V1).expect("B").command, CommandId(2));
    one_terminal_each(&host);
}

#[test]
fn ct_rm_03_rejected_replacement_supersedes_nothing() {
    let base = revision(12);
    let mut ctx = context();
    ctx.limits.instances = 4;
    let mut host = host_with(&base, ctx);
    host.publish(CommandId(1), sheet(1, &base, 2));

    let mut malformed = sheet_from(1, &base, 10, 2);
    malformed.plans[0].source_map = MapId(9);
    assert_eq!(refused(&host.publish(CommandId(2), malformed)).code, codes::PLAN_INVALID);
    let error = refused(&host.publish(CommandId(3), sheet_from(1, &base, 10, 5)));
    assert_eq!(error.code, codes::PLAN_CEILING);
    assert_eq!(
        (error.context["field"].as_str(), error.context["requested"].as_str(), error.context["ceiling"].as_str()),
        ("instances", "5", "4")
    );
    assert_eq!(status(&host, 1), Status::Accepted, "A stays pending");
    assert!(host.installed(V1).is_none(), "nothing from B installs");

    finish(&mut host, 1);
    assert_eq!(host.installed(V1).expect("A").command, CommandId(1));
    install(&mut host, 4, sheet_from(1, &base, 10, 4));
    assert_eq!(host.instances(V1).len(), 4, "four instances pass the ceiling");
    one_terminal_each(&host);
}

#[test]
fn ct_rm_03_rejected_scheduled_output_fails_its_original_invocation() {
    let base = revision(12);
    let mut ctx = context();
    ctx.limits.instances = 4;
    let mut host = host_with(&base, ctx);
    host.publish(CommandId(1), sheet(1, &base, 2));
    let running = host.schedule(CommandId(2), V1, u64::MAX);
    let Status::Running(job) = running.status else { panic!("{:?}", running.status) };
    assert_eq!(status(&host, 1), Status::Cancelled(CancelReason::Superseded { by: CommandId(2) }));

    let receipt = host.deliver(CommandId(2), sheet_from(1, &base, 10, 5)).expect("a receipt");
    let error = failed(&receipt);
    assert_eq!(error.code, codes::PLAN_CEILING);
    assert_eq!(
        (error.context["field"].as_str(), error.context["requested"].as_str(), error.context["ceiling"].as_str()),
        ("instances", "5", "4")
    );
    assert_eq!(
        (receipt.job, receipt.request.as_str()),
        (Some(job), running.request.as_str()),
        "the original invocation"
    );
    assert_eq!(host.job_outcome(CommandId(2)), Some(&JobOutcome::Error(error.clone())));
    assert_eq!(host.deliver(CommandId(2), sheet_from(1, &base, 10, 4)), Some(receipt), "no second admission");
    host.pump();
    assert!(host.installed(V1).is_none());
    assert_eq!(status(&host, 1), Status::Cancelled(CancelReason::Superseded { by: CommandId(2) }));
    one_terminal_each(&host);
}

#[test]
fn ct_rm_03_dependency_change_cancels_or_refuses_old_work() {
    let base = revision(12);
    let mut host = host(&base);
    install(&mut host, 1, sheet(1, &base, 2));
    host.publish(CommandId(2), sheet_from(1, &base, 10, 2));
    let pending = host.ticket(CommandId(2)).expect("prepared");

    host.set_dependency("state", "13");
    assert_eq!(status(&host, 2), Status::Cancelled(CancelReason::ReplacedRevision));
    assert!(matches!(status(&host, 1), Status::Completed(_)), "installed success keeps its receipt");
    assert!(host.stale(V1), "the installed output read revision 12");
    host.backend.finish(pending);
    host.pump();
    assert_eq!(host.installed(V1).expect("A").command, CommandId(1));

    let error = refused(&host.publish(CommandId(3), sheet_from(1, &base, 20, 1)));
    assert_eq!(error.code, codes::STALE_PLAN);
    let running = host.schedule(CommandId(4), V1, u64::MAX);
    assert!(matches!(running.status, Status::Running(_)));
    let receipt = host.deliver(CommandId(4), sheet_from(1, &base, 20, 1)).expect("a receipt");
    assert_eq!(receipt.status, Status::Cancelled(CancelReason::ReplacedRevision));
    assert_eq!(host.job_outcome(CommandId(4)), Some(&JobOutcome::Cancelled(CancelReason::ReplacedRevision)));

    install(&mut host, 5, sheet_from(1, &revision(13), 30, 1));
    assert!(!host.stale(V1));
    one_terminal_each(&host);
}

#[test]
fn ct_rm_03_budget_exhaustion_cancels_with_budget_and_installs_nothing() {
    let base = revision(12);
    let payload = sheet(1, &base, 3);
    let bytes = validate(&payload, &context()).expect("valid").output_bytes;
    let mut host = host(&base);

    host.schedule(CommandId(1), V1, bytes - 1);
    let receipt = host.deliver(CommandId(1), payload.clone()).expect("a receipt");
    assert_eq!(receipt.status, Status::Cancelled(CancelReason::Budget));
    let Some(JobOutcome::CutOff(error)) = host.job_outcome(CommandId(1)) else { panic!("cut off") };
    assert_eq!((error.code, error.form), (codes::OUTPUT, Form::Stop));
    assert_eq!(error.context["requested"], bytes.to_string());
    assert_eq!(error.context["remaining"], (bytes - 1).to_string());
    assert!(!is_reserved(error), "never the reserved producer plan budget");
    assert!(host.ticket(CommandId(1)).is_none(), "nothing was prepared");
    host.pump();
    assert!(host.installed(V1).is_none());

    host.schedule(CommandId(2), V1, bytes);
    assert!(matches!(host.deliver(CommandId(2), payload).expect("a receipt").status, Status::Running(_)));
    finish(&mut host, 2);
    assert!(matches!(status(&host, 2), Status::Completed(_)), "an allowance equal to the bytes admits them");
    one_terminal_each(&host);
}

#[test]
fn ct_rm_03_renderer_loss_before_acceptance_is_a_refusal() {
    let base = revision(12);
    let mut host = host(&base);
    host.publish(CommandId(1), sheet(1, &base, 2));
    host.backend.lose_device();
    let error = refused(&host.publish(CommandId(2), sheet_from(1, &base, 10, 2)));
    assert_eq!(error.code, codes::RENDERER_UNAVAILABLE);
    assert_eq!(status(&host, 1), Status::Accepted, "B's refusal does not supersede A");

    host.pump();
    let error = failed(&host.receipt(CommandId(1)).expect("A"));
    assert_eq!(error.code, codes::RENDERER_UNAVAILABLE, "A's own loss is handled under RM.22");
    assert!(host.installed(V1).is_none());
    one_terminal_each(&host);
}

#[test]
fn ct_rm_03_renderer_loss_after_acceptance_fails_the_running_job() {
    let base = revision(12);
    let mut host = host(&base);
    let running = host.schedule(CommandId(1), V1, u64::MAX);
    host.backend.lose_device();
    let receipt = host.deliver(CommandId(1), sheet(1, &base, 2)).expect("a receipt");
    let error = failed(&receipt);
    assert_eq!(error.code, codes::RENDERER_UNAVAILABLE);
    assert_eq!(receipt.job, running.job);
    assert_eq!(host.job_outcome(CommandId(1)), Some(&JobOutcome::Error(error)));

    let mut host = common::host(&base);
    host.schedule(CommandId(1), V1, u64::MAX);
    host.deliver(CommandId(1), sheet(1, &base, 2));
    assert!(host.ticket(CommandId(1)).is_some(), "prepared and pending");
    host.backend.lose_device();
    host.pump();
    let error = failed(&host.receipt(CommandId(1)).expect("a receipt"));
    assert_eq!(error.code, codes::RENDERER_UNAVAILABLE);
    assert_eq!(host.job_outcome(CommandId(1)), Some(&JobOutcome::Error(error)));
    assert!(host.installed(V1).is_none());
    one_terminal_each(&host);
}

#[test]
fn ct_rm_03_renderer_loss_after_installation_keeps_completed() {
    let base = revision(12);
    let mut host = host(&base);
    install(&mut host, 1, sheet(1, &base, 2));
    let receipt = host.receipt(CommandId(1)).expect("B");
    let old = PickReply { binding: host.pick_binding(V1).expect("current"), instance: InstanceId(1) };
    host.show_preview(V1, Some(preview(1, &base))).expect("an engine preview");
    host.frame(V1).expect("a frame");
    host.read_presented();
    assert_eq!(host.presented_preview(), Some(shown(1, &base)));

    host.backend.lose_device();
    host.pump();
    assert_eq!(host.receipt(CommandId(1)), Some(receipt.clone()), "B stays completed(view_published)");
    assert_eq!(host.outcome(CommandId(1), &receipt.request), Some(receipt.clone()));
    assert!(!host.installed(V1).expect("retained").available);
    assert_eq!(host.pick_binding(V1), Err("device_epoch"));
    assert_eq!(host.presented_preview(), None, "old reports clear");
    assert_eq!(host.frame(V1).expect_err("drawing is unavailable").code, codes::RENDERER_UNAVAILABLE);
    let error = host.resolve_pick(&old, PickMode::Explicit, Requested::Occurrence, &Unasked).expect_err("stale");
    assert_eq!((error.code, error.context["field"].as_str()), (codes::STALE_PICK, "device_epoch"));

    host.recover().expect("recovers");
    assert!(host.installed(V1).expect("retained").available);
    assert_eq!(host.resolve_pick(&old, PickMode::Passive, Requested::Occurrence, &Unasked), Ok(None));
    let error = host.resolve_pick(&old, PickMode::Explicit, Requested::Occurrence, &Unasked).expect_err("stale");
    assert_eq!(error.context["field"], "device_epoch", "no old pick is accepted after recovery");
    let fresh = PickReply { binding: host.pick_binding(V1).expect("current"), instance: InstanceId(1) };
    assert!(host.resolve_pick(&fresh, PickMode::Explicit, Requested::Occurrence, &Unasked).expect("current").is_some());
    assert_eq!(host.receipt(CommandId(1)), Some(receipt), "recovery creates no retry");
    one_terminal_each(&host);
}

#[test]
fn ct_rm_03_preview_latch_names_only_a_confirmed_preview() {
    let base = revision(12);
    let mut host = host(&base);
    install(&mut host, 1, sheet(1, &base, 1));
    for _ in 0..39 {
        host.frame(V1).expect("a frame");
    }
    assert_eq!(host.read_presented().len(), 39);
    assert_eq!(host.presented_preview(), None);

    host.show_preview(V1, Some(preview(1, &base))).expect("P1");
    assert_eq!(host.frame(V1).expect("frame 40").sequence, 40);
    host.read_presented();
    assert_eq!(host.presented_preview(), Some(shown(1, &base)));

    host.show_preview(V1, Some(preview(2, &base))).expect("P2");
    host.backend.hold(41);
    assert_eq!(host.frame(V1).expect("frame 41").sequence, 41);
    assert!(host.read_presented().is_empty(), "frame 41 is submitted, not confirmed");
    assert_eq!(host.presented_preview(), Some(shown(1, &base)), "ONE.41 still names P1");
    host.backend.release(41);
    let confirmed = host.read_presented();
    assert_eq!(confirmed[0].presented, Presented::Previews(vec![shown(2, &base)]));
    assert_eq!(host.presented_preview(), Some(shown(2, &base)));

    host.show_preview(V1, Some(preview(3, &base))).expect("P3");
    host.backend.hold(42);
    host.frame(V1).expect("frame 42");
    host.show_preview(V1, Some(preview(4, &base))).expect("P4");
    host.frame(V1).expect("frame 43");
    host.read_presented();
    assert_eq!(host.presented_preview(), Some(shown(4, &base)));
    host.backend.release(42);
    assert_eq!(host.read_presented().len(), 1);
    assert_eq!(host.presented_preview(), Some(shown(4, &base)), "an older confirmation never replaces a newer one");

    host.show_preview(V1, None).expect("no preview");
    host.frame(V1).expect("frame 44");
    host.read_presented();
    assert_eq!(host.presented_preview(), None, "a no_preview report clears it");

    host.show_preview(V1, Some(preview(5, &base))).expect("P5");
    host.frame(V1).expect("frame 45");
    host.read_presented();
    assert_eq!(host.presented_preview(), Some(shown(5, &base)));
    host.backend.lose_device();
    assert!(host.read_presented().is_empty());
    assert_eq!(host.presented_preview(), None, "unavailable confirmation clears it");
}

#[test]
fn ct_rm_03_receipts_replay_and_refuse_reuse() {
    let base = revision(12);
    let mut host = host(&base);
    let first = host.publish(CommandId(1), sheet(1, &base, 2));
    assert_eq!(host.publish(CommandId(1), sheet(1, &base, 2)), first, "the same command and request");
    let reused = refused(&host.publish(CommandId(1), sheet(1, &base, 3)));
    assert_eq!(reused.code, codes::COMMAND_ID_REUSED);
    assert_eq!(status(&host, 1), Status::Accepted);
    assert_eq!(host.open_view(V1).expect_err("open").code, codes::VIEW_ID_IN_USE);
    let error = refused(&host.publish(CommandId(2), sheet(9, &base, 2)));
    assert_eq!(error.code, codes::PLAN_INVALID);
    assert!(error.summary.contains("not open"), "{}", error.summary);
    finish(&mut host, 1);
    assert_eq!(host.installed(V1).expect("A").resources, ResourceSet(host.ticket(CommandId(1)).expect("t").0));
    one_terminal_each(&host);
}
