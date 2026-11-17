//! Map property checks over a run of camera poses (LB4.2) and the admission of a user chart (LB4.3).
//!
//! A [`Survey`] walks a chart through camera poses. At an exact pose it asks `c600-display`'s exact checks for every
//! declared property and keeps the verdicts; between exact poses it asks nothing exact and keeps only what a frame
//! draws, where domain membership is a display decision the caller may replace. The two halves have different types:
//! [`admit`] reads the verdicts and has no parameter a display decision could reach.

use c600_display::chart::{self, Admission, CameraPose, ChartCheck, Property, PropertyStatus, Region, Stereographic};
use c600_display::error::Error;
use c600_display::plan::{Chart, Coverage, DisplayPlan, StageKind};
use c600_display::reference::{self, Fixture, Point, PrecisionRule};
use c600_display::stage::{self, CheckedStage, StageDescription};

/// What a survey reads: the plan and its chart, the chart's exact declaration, the region as exact vertices, the
/// declared properties and the declared precision rule of the CPU reference.
#[derive(Clone, Copy, Debug)]
pub struct Subject<'a> {
    pub plan: &'a DisplayPlan,
    pub chart: &'a Chart,
    pub declaration: &'a Stereographic,
    pub region: &'a Region,
    pub properties: &'a [Property],
    pub rule: PrecisionRule,
}

/// One frame's display data at a camera pose: the vertices drawn, with binary32 coordinates, and the coverage. It
/// states no property of the chart.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    /// The pose's identity, or `approximate` and the pose's declared text.
    pub pose: String,
    pub exact_pose: bool,
    pub drawn: Vec<(u32, [f32; 4])>,
    pub coverage: Coverage,
}

/// A survey's result: the exact verdicts of the exact poses, and one frame of display data per pose.
#[derive(Clone, Debug, PartialEq)]
pub struct Survey {
    pub checks: Vec<ChartCheck>,
    pub frames: Vec<Frame>,
}

/// The CPU reference's own display decision between exact poses: a vertex is shown when the reference drew it, which
/// excludes its declared neighbourhood of the pole.
pub fn reference_decision(_: &CameraPose, _: u32, point: &Point) -> bool {
    matches!(point, Point::Drawn(_))
}

/// Surveys the subject over `poses`. `decide` is the display decision for one vertex at an approximate pose, given the
/// reference's point for it; it is never asked at an exact pose, where the reference's exact decisions stand, and its
/// answer reaches the frame only. A vertex it accepts is drawn only if the reference has finite coordinates for it: no
/// point is made up.
pub fn survey(
    subject: &Subject<'_>,
    poses: &[CameraPose],
    decide: &dyn Fn(&CameraPose, u32, &Point) -> bool,
) -> Result<Survey, Error> {
    let fixture = Fixture {
        plan: subject.plan,
        chart: subject.chart,
        declaration: subject.declaration,
        vertices: &subject.region.vertices,
        rule: subject.rule,
    };
    let identity = chart::identity(subject.chart);
    let mut checks = Vec::new();
    let mut frames = Vec::new();
    for pose in poses {
        let evaluation = reference::evaluate(&fixture, pose);
        match pose {
            CameraPose::Exact(_) => {
                for property in subject.properties {
                    let verdict = chart::check(subject.declaration, property, subject.region, pose)?;
                    checks.push(ChartCheck { chart: identity.clone(), property: property.name().to_owned(), verdict });
                }
                frames.push(Frame {
                    pose: evaluation.exact.pose.clone(),
                    exact_pose: true,
                    drawn: evaluation.draw(),
                    coverage: evaluation.coverage(),
                });
            }
            CameraPose::Approximate { .. } => {
                let mut drawn = Vec::new();
                let mut missing = Vec::new();
                for (i, point) in evaluation.points.iter().enumerate() {
                    match point {
                        Point::Drawn(y) if decide(pose, i as u32, point) => drawn.push((i as u32, *y)),
                        Point::Drawn(_) => missing.push(format!("vertex {i}: display")),
                        Point::Missing(m) => missing.push(format!("vertex {i}: {}", m.reason())),
                    }
                }
                let coverage = if missing.is_empty() { Coverage::Complete } else { Coverage::Partial { missing } };
                frames.push(Frame { pose: evaluation.exact.pose.clone(), exact_pose: false, drawn, coverage });
            }
        }
    }
    Ok(Survey { checks, frames })
}

/// `admit_chart` over a survey's exact verdicts (RPI.48): a claimed property is admitted only when its own check
/// established it at every exact pose and region the survey checked. The frames are not an input.
pub fn admit(chart: &Chart, survey: &Survey) -> (Admission, Vec<(String, PropertyStatus)>) {
    chart::admit_chart(chart, &survey.checks)
}

/// A user chart after the part of the stage-module admission that `c600-display` reaches (RPI.68, RPI.69, RPI.75):
/// its stage's declared outputs passed, and `admit_chart` answered for the properties the chart claims.
#[derive(Clone, Debug, PartialEq)]
pub struct UserChart {
    pub stage: CheckedStage,
    pub admission: Admission,
    pub statuses: Vec<(String, PropertyStatus)>,
}

/// Why a user chart is not admitted.
#[derive(Clone, Debug, PartialEq)]
pub enum UserChartError {
    /// The stage's declared outputs break the stage rules; the error is `c600-display`'s diagnostic.
    Stage(Error),
    /// The stage is a fragment stage: a chart maps positions, which a vertex stage returns (a draft).
    NotAVertexStage,
    /// No stage of the chart names the description's digest as its definition: the chart is not bound to this stage.
    Unbound,
}

/// Admits a user chart: the stage's outputs are checked first, the stage must be a vertex stage, the chart must name
/// the stage's digest as a stage definition, and then `admit_chart` reads `checks`. Passing the stage rules
/// establishes no property: with no established check for a claimed property the admission is `not_established`
/// (LB4.3).
pub fn admit_user_chart(
    description: &StageDescription,
    chart: &Chart,
    checks: &[ChartCheck],
) -> Result<UserChart, UserChartError> {
    let stage = stage::check_outputs(description).map_err(UserChartError::Stage)?;
    if stage.kind() != StageKind::Vertex {
        return Err(UserChartError::NotAVertexStage);
    }
    if !chart.stages.iter().any(|s| s.definition == stage.digest()) {
        return Err(UserChartError::Unbound);
    }
    let (admission, statuses) = chart::admit_chart(chart, checks);
    Ok(UserChart { stage, admission, statuses })
}
