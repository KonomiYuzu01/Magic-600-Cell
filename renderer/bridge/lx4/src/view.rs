//! The view whose stale state LX-4 times (RTE.49; EV1.7, EV3.6). Each sample gets a c600-view view model with one
//! overlay view of the edited module's checked version, the edit's baseline, whose output is installed before the
//! sample's first input. The editor's draft reaches the model by source digest ([`ViewModel::edit_source`]), and the
//! view's `source_lag` in the frame input says whether the frame shows the view stale: the view model decides the
//! state, by a host comparison of digests (RPI.13), not this crate. Nothing runs and nothing is fenced on an edit.
//!
//! The view's one run is a fixed publication, as in c600-view's own tests: LX-4 times the stale state, not a view's
//! output, and running a view on the session's state is C9 product's host call (rerun, RTE.50, not built). The
//! revision, the engine bundle and the limits are generous fixture values, not product ones.

use c600_view::{
    BundleCandidate, BundlePart, CommandId, Content, Declared, Depends, Digest, Field, LimitProfile, MapDescriptor,
    Mode, Origin, OutputAllowance, PlanDescriptor, Profile, Revision, RunOutcome, SourceLag, ViewId, ViewModel,
    ViewSource, ViewStatus, Workload,
};

/// Every fixture limit but `overdraw`, which stays 1, as in c600-view's tests.
const GENEROUS: u64 = 1_000_000;

/// The command ID of the view's `view.open`, a solver's command.
const OPEN: CommandId = CommandId::Caller(GENEROUS);

/// The view model of one sample and its view of the baseline.
pub struct StaleView {
    model: ViewModel,
    view: ViewId,
    version: Digest,
    lag: Option<SourceLag>,
}

impl StaleView {
    /// A view model whose one overlay view of module version `version` (the baseline's source digest) is current.
    pub fn open(version: Digest) -> Result<StaleView, String> {
        let revision = Revision::new(1, 1, 1, 1, [1; 32]);
        let entries: Vec<(&str, u64, &str)> = Field::ALL
            .iter()
            .map(|field| (field.name(), if *field == Field::Overdraw { 1 } else { GENEROUS }, field.unit()))
            .collect();
        let profile = LimitProfile::from_entries("lx4-fixture-1", &entries).map_err(|e| format!("{e:?}"))?;
        let parts = Mode::JumbleOff
            .required()
            .iter()
            .map(|kind| {
                let mut digest = [0u8; 32];
                digest[0] = *kind as u8;
                BundlePart { kind: *kind, revision, digest }
            })
            .collect();
        let bundle = BundleCandidate { revision, mode: Mode::JumbleOff, parts };
        let mut model = ViewModel::new(Profile::Solving, profile, &bundle, Workload::default(), Workload::default())
            .map_err(|e| format!("the view model does not open: {e:?}"))?;
        let view = ViewId::new("lx4");
        let source = ViewSource::Overlay { row: "lx4".to_owned(), module: version, arguments: vec![] };
        let workload = Workload { vertex_evals: 1, cpu_bytes: 1, gpu_bytes: 1, ..Workload::default() };
        let declared =
            Declared { depends: Depends::STATE, workload, output: OutputAllowance::new(GENEROUS, 16), class: None };
        model
            .open_view(OPEN, Origin::Solver, view.clone(), source, declared)
            .map_err(|e| format!("the view does not open: {e:?}"))?;
        model.pump(|_| RunOutcome::Published(publication()));
        match model.status(&view) {
            Some(ViewStatus::Current { .. }) => Ok(StaleView { model, view, version, lag: None }),
            other => Err(format!("the view is not current after its run: {other:?}")),
        }
    }

    /// The editor holds `draft` (its buffer's source digest) as the baseline's source; the baseline's own digest
    /// clears the lag. Then reads the view's lag from the frame input, as a frame draws it.
    pub fn edit(&mut self, draft: Digest) {
        self.model.edit_source(self.version, draft);
        self.lag = self.model.frame_input().views.into_iter().find(|v| v.view == self.view).and_then(|v| v.source_lag);
    }

    /// The view's source lag in the last frame input: the frame shows the view stale.
    pub fn lag(&self) -> Option<SourceLag> {
        self.lag
    }
}

/// The view's one publication: one plan with one instance, as in c600-view's tests.
fn publication() -> Content {
    Content {
        nodes: vec![vec![1, 2, 3]],
        plans: vec![PlanDescriptor { plan: 1, source_map: 7, instances: 1, ..PlanDescriptor::default() }],
        maps: vec![MapDescriptor { map: 7, entries: 1 }],
        source_bytes: 12,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_draft_makes_the_view_stale_and_the_baseline_clears_it() {
        let (version, draft) = ([0xa0; 32], [0xd0; 32]);
        let mut view = StaleView::open(version).expect("the view opens current");
        assert_eq!(view.lag(), None);
        view.edit(version);
        assert_eq!(view.lag(), None);
        view.edit(draft);
        assert_eq!(view.lag(), Some(SourceLag { producer: version, draft }));
        assert!(matches!(view.model.status(&view.view), Some(ViewStatus::Stale { source_lag: Some(_), .. })));
        view.edit(version);
        assert_eq!(view.lag(), None);
        assert!(matches!(view.model.status(&view.view), Some(ViewStatus::Current { .. })));
    }
}
