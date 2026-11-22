//! CT-RPI-32 (source, part: the headless part; not run: compiling a stage and compositing on a device, which need a
//! backend, on the owner's machine (C9)). The headless part runs on stages that LP2's stage checker admits
//! (`c600_check::admit_stage`) and the display takes up (`stage::take_up`): a fragment stage writes opaque RGB only,
//! with no discard, depth, stencil or coverage output; host statuses composite after user stages; a preview shown
//! through a user stage has no presented-preview identity and binds no commit token (RPI.68, RPI.69, RPI.75, RPI.76).
//! Every refusal the checker gives for a stage used here comes back with the checker's code. One test runs the
//! display's own output rule (`stage::check_outputs`) on stage descriptions built directly, the rule that
//! `c600-repr`'s chart path also calls. The host still reports F05 and F26 unsupported, so no user stage reaches its
//! frames.

mod common;

use c600_check::{IR_VERSION_UNSUPPORTED, STAGE_RULE_VIOLATION, TYPE_MISMATCH};
use c600_digest::sha256_hex;
use c600_display::pick::{Presented, PreviewShown};
use c600_display::plan::*;
use c600_display::stage::{self, Chain, CheckedStage, HostStatus, Layer, Output, StageDescription};
use c600_display::{codes, Form};
use c600_ir::build::{approx_less, display_if, let_, program, stage_op, var};
use c600_ir::{Program, StageKind as Kind, StageOp};
use common::stages::*;
use common::*;

/// The checker's refusal of `module.main` in `p`, taken up: nothing reaches the display, and every finding comes back
/// in the checker's order with the checker's code, naming its module, function, path and mismatch. Returns each
/// error's code, form and path.
fn refusals(p: &Program, module: &str) -> Vec<(&'static str, Form, String)> {
    let admission = admission(p, module);
    assert!(admission.stage.is_none() && !admission.diagnostics.is_empty(), "{admission:#?}");
    let errors = stage::take_up(&admission).expect_err("a refused stage is not taken up");
    assert_eq!(errors.iter().map(|e| e.code).collect::<Vec<_>>(), admission.codes());
    for (error, finding) in errors.iter().zip(&admission.diagnostics) {
        assert_eq!(error.summary, finding.note);
        assert_eq!(error.context["module"], finding.module);
        assert_eq!(error.context["path"], finding.path);
        assert_eq!(error.context.get("function").map(String::as_str).unwrap_or(""), finding.function);
        let mismatch = finding.mismatch.as_ref().map(|m| (m.expected.clone(), m.supplied.clone()));
        let context = error.context.get("expected").map(|e| (e.clone(), error.context["supplied"].clone()));
        assert_eq!(context, mismatch);
    }
    errors.into_iter().map(|e| (e.code, e.form, e.context["path"].clone())).collect()
}

fn diagnostic(code: &'static str, path: &str) -> (&'static str, Form, String) {
    (code, Form::Diagnostic, path.to_owned())
}

/// A stage description built directly, not taken up from the checker, named by the digest of `name`.
fn describe(kind: StageKind, name: &str, outputs: &[Output]) -> StageDescription {
    StageDescription { kind, digest: sha256_hex(name.as_bytes()), outputs: outputs.to_vec() }
}

/// A description that the display's output rule accepts.
fn checked(kind: StageKind, name: &str, outputs: &[Output]) -> CheckedStage {
    stage::check_outputs(&describe(kind, name, outputs)).expect("a well-declared stage")
}

/// A description that the display's output rule refuses with `LG2:type.stage_rule_violation`, a diagnostic.
fn violation(stage: &StageDescription) -> c600_display::Error {
    let error = stage::check_outputs(stage).expect_err("outside the stage rules");
    assert_eq!((error.code, error.form), (codes::STAGE_RULE_VIOLATION, Form::Diagnostic), "{error:?}");
    error
}

fn shown(base: &RevisionRef) -> PreviewShown {
    let preview = preview(7, base);
    PreviewShown { preview: preview.preview, literal: preview.literal, base: base.state }
}

#[test]
fn ct_rpi_32_headless_fragment_stage_writes_opaque_rgb_only() {
    // Admitted stages: a fragment stage describes opaque RGB only, a vertex stage a position and a coverage code.
    let p = program(vec![vertex(), fragment("f")]);
    let fragment_stage = admitted(&p, "f");
    assert_eq!(fragment_stage.kind(), StageKind::Fragment);
    let description = stage::describe(admission(&p, "f").stage.as_ref().expect("admitted"));
    assert_eq!((description.kind, description.outputs.clone()), (StageKind::Fragment, vec![Output::Rgb]));
    assert_eq!(description.digest, fragment_stage.digest());
    let vertex_stage = stage::describe(admission(&p, "v").stage.as_ref().expect("admitted"));
    assert_eq!(vertex_stage.outputs, vec![Output::Position, Output::CoverageCode]);
    assert_ne!(vertex_stage.digest, description.digest);

    // A fragment entry with alpha, depth, stencil or coverage beside RGB, or a position, is refused at the entry.
    for extra in ["a", "depth", "stencil", "coverage"] {
        let names = ["r", "g", "b", extra];
        let wide = stage_with("f", Kind::Fragment, record_ty(&names), record_of(&names, var("t")));
        assert_eq!(refusals(&program(vec![wide]), "f"), vec![diagnostic(STAGE_RULE_VIOLATION, "entry")], "{extra}");
    }
    let placed = stage_with("f", Kind::Fragment, record_ty(&["x", "y", "z"]), position());
    assert_eq!(refusals(&program(vec![placed]), "f"), vec![diagnostic(STAGE_RULE_VIOLATION, "entry")]);
    // A fragment never discards: `exclude` is for a vertex stage only (RPI.75).
    let cut = let_("c", stage_op(StageOp::Exclude, vec![var("t"), c600_ir::build::f32_lit("1")]), var("t"));
    let discard = stage_with("f", Kind::Fragment, record_ty(&["r", "g", "b"]), record_of(&["r", "g", "b"], cut));
    assert_eq!(refusals(&program(vec![discard]), "f"), vec![diagnostic(STAGE_RULE_VIOLATION, "0.0")]);
    // A vertex entry that returns a colour, and a body whose display branches have two types.
    let coloured = stage_with("v", Kind::Vertex, record_ty(&["r", "g", "b"]), record_of(&["r", "g", "b"], var("t")));
    assert_eq!(refusals(&program(vec![coloured]), "v"), vec![diagnostic(STAGE_RULE_VIOLATION, "entry")]);
    let mixed = display_if(approx_less(var("t"), zero()), zero(), approx_less(var("t"), zero()));
    let typed = stage_with("f", Kind::Fragment, record_ty(&["r", "g", "b"]), record_of(&["r", "g", "b"], mixed));
    assert_eq!(refusals(&program(vec![typed]), "f"), vec![diagnostic(TYPE_MISMATCH, "0.2")]);
    // A program of another IR version is refused in that code's registered form, `refusal`.
    let mut old = program(vec![fragment("f")]);
    old.ir_version = "c600-ir-draft-0".to_owned();
    assert_eq!(refusals(&old, "f"), vec![(IR_VERSION_UNSUPPORTED, Form::Refusal, "program".to_owned())]);

    // A description widened by hand past what `stage::describe` gives is refused by the display's own output rule.
    for output in [Output::Discard, Output::Depth, Output::Stencil, Output::Coverage, Output::Alpha] {
        let mut wider = description.clone();
        wider.outputs.push(output);
        let error = stage::check_outputs(&wider).expect_err("outside the stage rules");
        assert_eq!((error.code, error.form), (codes::STAGE_RULE_VIOLATION, Form::Diagnostic));
        assert_eq!(error.context["output"], output.name());
    }

    // The digest names the checked definition: the same under other weights, different for another body.
    let mut doubled = stage_limits();
    doubled.weights.values_mut().for_each(|w| *w = 2);
    let again = c600_check::admit_stage(&p, "v", "main", &doubled).expect("a stage module");
    assert_eq!(stage::take_up(&again).expect("admitted").digest(), vertex_stage.digest);
    let other = stage_with("v", Kind::Vertex, record_ty(&["x", "y", "z"]), record_of(&["x", "y", "z"], var("x")));
    assert_ne!(admitted(&program(vec![other]), "v").digest(), vertex_stage.digest);
}

/// CT-RPI-32 (source), the headless part on stage descriptions: it runs the display's own output rule on a stage
/// description built directly (`stage::check_outputs`; RPI.68, RPI.69, RPI.75), the rule that `c600-repr`'s chart
/// path also calls. A fragment stage declares opaque RGB only: an alpha, discard, depth, stencil or coverage output
/// after or before `Rgb`, no output, a repeated `Rgb` and a `Position` are refused. A vertex stage declares a position
/// and a coverage code, in either order, and nothing else. A digest that is not 64 lowercase hex digits is refused.
/// Each refusal is `LG2:type.stage_rule_violation` in its registered form, a diagnostic.
#[test]
fn ct_rpi_32_headless_descriptions_follow_the_output_rule() {
    let fragment = checked(StageKind::Fragment, "fragment stage", &[Output::Rgb]);
    assert_eq!(fragment.kind(), StageKind::Fragment);
    for output in [Output::Discard, Output::Depth, Output::Stencil, Output::Coverage, Output::Alpha] {
        let error = violation(&describe(StageKind::Fragment, "fragment stage", &[Output::Rgb, output]));
        assert_eq!(error.context["output"], output.name());
        let error = violation(&describe(StageKind::Fragment, "fragment stage", &[output, Output::Rgb]));
        assert_eq!(error.context["output"], output.name());
    }
    violation(&describe(StageKind::Fragment, "fragment stage", &[]));
    violation(&describe(StageKind::Fragment, "fragment stage", &[Output::Rgb, Output::Rgb]));
    violation(&describe(StageKind::Fragment, "fragment stage", &[Output::Position]));

    // A vertex stage returns a position or a coverage code, declares both and nothing else.
    checked(StageKind::Vertex, "vertex stage", &[Output::CoverageCode, Output::Position]);
    violation(&describe(StageKind::Vertex, "vertex stage", &[Output::Position]));
    violation(&describe(StageKind::Vertex, "vertex stage", &[Output::Position, Output::CoverageCode, Output::Rgb]));
    violation(&describe(StageKind::Vertex, "vertex stage", &[Output::Position, Output::CoverageCode, Output::Depth]));

    // A stage is named by its digest, never by text.
    let mut unnamed = describe(StageKind::Fragment, "fragment stage", &[Output::Rgb]);
    unnamed.digest = "fragment stage".to_owned();
    violation(&unnamed);
}

#[test]
fn ct_rpi_32_headless_host_statuses_composite_after_user_stages() {
    let base = revision(5);
    let statuses = [HostStatus::Selection, HostStatus::Stale, HostStatus::Preview, HostStatus::ProtectionBreakthrough];
    for preview in [None, Some(shown(&base))] {
        let chain = user_chain();
        let drawn = preview.as_ref().map(|p| (p, &chain));
        let composition = stage::compose(&user_chain(), drawn, &statuses);
        let last_user = composition.layers.iter().rposition(|l| matches!(l, Layer::User(_) | Layer::ExtraPreview));
        let first_status = composition.layers.iter().position(|l| matches!(l, Layer::Status(_)));
        assert!(last_user.expect("user stages") < first_status.expect("statuses"), "{:?}", composition.layers);
        let order: Vec<HostStatus> = composition
            .layers
            .iter()
            .filter_map(|l| match l {
                Layer::Status(s) => Some(*s),
                _ => None,
            })
            .collect();
        assert_eq!(
            order,
            vec![HostStatus::Preview, HostStatus::Stale, HostStatus::ProtectionBreakthrough, HostStatus::Selection]
        );
    }
}

#[test]
fn ct_rpi_32_headless_preview_through_a_user_stage_has_no_identity() {
    let base = revision(5);
    let shown = shown(&base);

    // Through a user stage: an extra display that the presented report does not name and a commit cannot bind to.
    let user = user_chain();
    let composition = stage::compose(&Chain::BuiltIn, Some((&shown, &user)), &[HostStatus::Preview]);
    assert_eq!(composition.presented, Presented::NoPreview);
    assert_eq!(composition.commit_binding(), None);
    assert!(composition.layers.contains(&Layer::ExtraPreview));
    assert!(!composition.layers.iter().any(|l| matches!(l, Layer::Preview(_))));

    // Through the built-in chain: the preview has its presented identity, and a commit may bind to it.
    let composition = stage::compose(&user, Some((&shown, &Chain::BuiltIn)), &[HostStatus::Preview]);
    assert_eq!(composition.presented, Presented::Previews(vec![shown.clone()]));
    assert_eq!(composition.commit_binding(), Some(&shown));

    // The host's own frames use the built-in chain only, and report the same identity; no user stage can install.
    let mut host = host(&base);
    host.show_preview(ViewId(1), Some(preview(7, &base))).expect("an engine preview");
    host.frame(ViewId(1)).expect("a frame");
    host.read_presented();
    assert_eq!(host.presented_preview(), Some(shown));
    for row in ["F05", "F26"] {
        let error = context().support.feature(row).expect_err("user stages are unsupported");
        assert_eq!(error.code, codes::UNSUPPORTED);
    }
}
