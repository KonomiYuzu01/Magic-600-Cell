//! Profiles and the provenance that display content keeps (LG3.9, RPI.45, RPI.81, BND.22, BND.26, BND.27).
//!
//! Display content retains the policy and provenance of its producing profile (RPI.45), and a further view widens
//! nothing the session may show (RPI.81). The view model stores the block it is given with the installed output and
//! hands it on with every frame. It decides no label and refuses nothing on a block (BND.22, BND.26): whether
//! `display` refuses is LQ2's option set (BND.74, question 4), and the policy table is not modelled here.

use crate::model::ViewSource;

/// LG3.9's three profiles of the one language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Profile {
    Solving,
    ToolAuthoring,
    Research,
}

impl Profile {
    /// The three profiles.
    pub const ALL: [Profile; 3] = [Profile::Solving, Profile::ToolAuthoring, Profile::Research];
}

/// BND.27's boundary class: a field of a built-in tool's module manifest. The first five classes are the 2 October
/// boundary; the last two are the two exceptions that BND.69's interim reading keeps (setup search for a target the
/// solver names, and the 17 October repair list at a named cap).
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
    /// Every class of the interim reading.
    pub const ALL: [BoundaryClass; 7] = [
        BoundaryClass::Tracking,
        BoundaryClass::Analysis,
        BoundaryClass::Checking,
        BoundaryClass::Protection,
        BoundaryClass::ViewAid,
        BoundaryClass::SetupSearchNamedTarget,
        BoundaryClass::RepairList,
    ];
}

/// The block a display sink stores with installed output: the part of chapter 07's `ProvenanceBlock` this crate
/// keeps (draft).
///
/// The host fills it (BND.22). For a run of a further view's source it is the session's profile, the view's source
/// and the class the host declared when the view opened; a runner returns content only and cannot set it. A
/// standalone publication carries the block the host states, and the view model keeps it as given. The chart
/// evaluator a view draws with never changes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    /// The profile of the producing run (LG3.9).
    pub profile: Profile,
    /// What produced the output: a built-in source or an overlay row's checked module (RPI.83).
    pub producer: ViewSource,
    /// The class of a built-in tool's manifest (BND.27); `None` for a solver's own tool.
    pub class: Option<BoundaryClass>,
}
