//! CT-RPI-32 (source), the headless part: a fragment stage has no discard, depth, stencil or coverage output; host
//! statuses composite after user stages; a preview shown through a user stage has no presented-preview identity and
//! binds no commit token (RPI.75, RPI.76). It runs over fixture stage descriptions (RPI.68 to RPI.80 say what a stage
//! declares). Not built, so CT-RPI-32 stays partly run: reading the declared outputs from a stage module needs the
//! stage language's parser and checker, and compositing on a device needs a backend; the host still reports F05 and
//! F26 unsupported, so no user stage reaches its frames.

mod common;

use c600_digest::sha256_hex;
use c600_display::pick::{Presented, PreviewShown};
use c600_display::plan::*;
use c600_display::stage::{self, Chain, CheckedStage, HostStatus, Layer, Output, StageDescription};
use c600_display::{codes, Form};
use common::*;

fn describe(kind: StageKind, name: &str, outputs: &[Output]) -> StageDescription {
    StageDescription { kind, digest: sha256_hex(name.as_bytes()), outputs: outputs.to_vec() }
}

fn checked(kind: StageKind, name: &str, outputs: &[Output]) -> CheckedStage {
    stage::check_outputs(&describe(kind, name, outputs)).expect("a well-declared stage")
}

fn violation(stage: &StageDescription) -> c600_display::Error {
    let error = stage::check_outputs(stage).expect_err("outside the stage rules");
    assert_eq!((error.code, error.form), (codes::STAGE_RULE_VIOLATION, Form::Diagnostic), "{error:?}");
    error
}

fn user_chain() -> Chain {
    Chain::User(vec![
        checked(StageKind::Vertex, "vertex stage", &[Output::Position, Output::CoverageCode]),
        checked(StageKind::Fragment, "fragment stage", &[Output::Rgb]),
    ])
}

fn shown(base: &RevisionRef) -> PreviewShown {
    let preview = preview(7, base);
    PreviewShown { preview: preview.preview, literal: preview.literal, base: base.state }
}

#[test]
fn ct_rpi_32_headless_fragment_stage_writes_opaque_rgb_only() {
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
