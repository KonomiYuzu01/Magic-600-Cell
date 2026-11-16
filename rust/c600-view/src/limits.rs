//! Limit profiles and the output allowance (RM.11 to RM.13, RM.38, RM.39, RPI.50, RPI.82).
//!
//! A [`LimitProfile`] is a versioned set of finite ceilings, one for every field of RM.11's table, each stated with
//! its unit. A missing field or unit makes the profile invalid; nothing defaults to unlimited (RM.11). Profiles here
//! are fixtures: no value is measured, and nothing in this crate advertises support (RPI.82, RM.40). The admission
//! ceiling is the componentwise minimum of the host/profile and backend ceilings (RM.12); a count equal to its
//! ceiling passes, and one above it is `EV3:resource_limit.display_plan_ceiling` naming field, requested count and
//! ceiling.
//!
//! [`OutputAllowance`] is RM.13's separately charged output allowance: a complete publication is charged its
//! canonical uncompressed length before installation, the terminal reserve never carries user output, a denied unit is
//! not charged and earlier charges are not refunded.

use crate::error::{code, Context, Error, Form};

/// The ceiling fields of RM.11, in the order of its table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
    Nodes,
    Plans,
    Instances,
    Maps,
    MapEntries,
    Charts,
    Stages,
    StageOps,
    FragmentOps,
    Overdraw,
    FrameWork,
    ViewVertexEvals,
    LayoutVertices,
    LayoutIndices,
    OverlayItems,
    InputBytes,
    SourceBytes,
    CanonicalOutputBytes,
    AdapterCpuBytes,
    AdapterGpuBytes,
}

impl Field {
    /// Every field, in RM.11's table order. Checks report the first excess in this order (draft, RM.12).
    pub const ALL: [Field; 20] = [
        Field::Nodes,
        Field::Plans,
        Field::Instances,
        Field::Maps,
        Field::MapEntries,
        Field::Charts,
        Field::Stages,
        Field::StageOps,
        Field::FragmentOps,
        Field::Overdraw,
        Field::FrameWork,
        Field::ViewVertexEvals,
        Field::LayoutVertices,
        Field::LayoutIndices,
        Field::OverlayItems,
        Field::InputBytes,
        Field::SourceBytes,
        Field::CanonicalOutputBytes,
        Field::AdapterCpuBytes,
        Field::AdapterGpuBytes,
    ];

    /// The field's name in RM.11.
    pub fn name(self) -> &'static str {
        match self {
            Field::Nodes => "nodes",
            Field::Plans => "plans",
            Field::Instances => "instances",
            Field::Maps => "maps",
            Field::MapEntries => "map_entries",
            Field::Charts => "charts",
            Field::Stages => "stages",
            Field::StageOps => "stage_ops",
            Field::FragmentOps => "fragment_ops",
            Field::Overdraw => "overdraw",
            Field::FrameWork => "frame_work",
            Field::ViewVertexEvals => "view_vertex_evals",
            Field::LayoutVertices => "layout_vertices",
            Field::LayoutIndices => "layout_indices",
            Field::OverlayItems => "overlay_items",
            Field::InputBytes => "input_bytes",
            Field::SourceBytes => "source_bytes",
            Field::CanonicalOutputBytes => "canonical_output_bytes",
            Field::AdapterCpuBytes => "adapter_cpu_bytes",
            Field::AdapterGpuBytes => "adapter_gpu_bytes",
        }
    }

    /// The unit a profile must state for the field. RM.11 states units in prose; these tokens are a draft.
    pub fn unit(self) -> &'static str {
        match self {
            Field::Nodes
            | Field::Plans
            | Field::Instances
            | Field::Maps
            | Field::MapEntries
            | Field::Charts
            | Field::Stages
            | Field::LayoutVertices
            | Field::LayoutIndices
            | Field::OverlayItems => "records",
            Field::StageOps | Field::FragmentOps | Field::FrameWork => "weighted_ops",
            Field::Overdraw => "invocations_per_pixel",
            Field::ViewVertexEvals => "vertex_evals_per_frame",
            Field::InputBytes | Field::SourceBytes | Field::CanonicalOutputBytes => "bytes",
            Field::AdapterCpuBytes | Field::AdapterGpuBytes => "resident_bytes",
        }
    }

    /// The field with this RM.11 name.
    pub fn from_name(name: &str) -> Option<Field> {
        Field::ALL.into_iter().find(|field| field.name() == name)
    }

    fn index(self) -> usize {
        Field::ALL.iter().position(|field| *field == self).expect("every field is listed")
    }
}

/// Why a profile is invalid (RM.11). An invalid profile supports nothing; it never means unlimited.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileError {
    /// The profile has no version.
    MissingVersion,
    /// An entry names no field of RM.11.
    Unknown(String),
    /// A field occurs twice.
    Duplicate(Field),
    /// A field is absent.
    Missing(Field),
    /// A field's unit is empty.
    MissingUnit(Field),
    /// A field's unit is not the one RM.11's counting rule uses.
    WrongUnit { field: Field, unit: String },
}

/// A versioned set of finite ceilings (RM.11).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitProfile {
    version: String,
    values: [u64; 20],
}

impl LimitProfile {
    /// A profile from `(field, value, unit)` entries. Every field of RM.11 must occur once with its unit.
    pub fn from_entries(version: &str, entries: &[(&str, u64, &str)]) -> Result<LimitProfile, ProfileError> {
        if version.is_empty() {
            return Err(ProfileError::MissingVersion);
        }
        let mut values = [None; 20];
        for (name, value, unit) in entries {
            let field = Field::from_name(name).ok_or_else(|| ProfileError::Unknown(name.to_string()))?;
            if values[field.index()].is_some() {
                return Err(ProfileError::Duplicate(field));
            }
            if unit.is_empty() {
                return Err(ProfileError::MissingUnit(field));
            }
            if *unit != field.unit() {
                return Err(ProfileError::WrongUnit { field, unit: unit.to_string() });
            }
            values[field.index()] = Some(*value);
        }
        let mut complete = [0; 20];
        for field in Field::ALL {
            complete[field.index()] = values[field.index()].ok_or(ProfileError::Missing(field))?;
        }
        Ok(LimitProfile { version: version.to_string(), values: complete })
    }

    /// The profile's version.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The ceiling of one field.
    pub fn get(&self, field: Field) -> u64 {
        self.values[field.index()]
    }

    /// RM.12: the componentwise minimum of this host/profile ceiling and a backend support ceiling.
    pub fn admission_ceiling(&self, backend: &LimitProfile) -> LimitProfile {
        let mut values = self.values;
        for (value, other) in values.iter_mut().zip(backend.values.iter()) {
            *value = (*value).min(*other);
        }
        LimitProfile { version: format!("min({}, {})", self.version, backend.version), values }
    }

    /// RM.12: equality passes; an excess is `EV3:resource_limit.display_plan_ceiling` in the given form.
    pub fn check(&self, field: Field, requested: u128, form: Form) -> Result<(), Error> {
        let ceiling = u128::from(self.get(field));
        if requested <= ceiling {
            return Ok(());
        }
        Err(Error {
            code: code::DISPLAY_PLAN_CEILING,
            form,
            context: Context::Ceiling { field: field.name(), requested, ceiling },
        })
    }

    /// Checks several counts and reports the first excess in RM.11's table order.
    pub fn check_all(&self, counts: &[(Field, u128)], form: Form) -> Result<(), Error> {
        let mut sorted = counts.to_vec();
        sorted.sort_by_key(|(field, _)| *field);
        sorted.into_iter().try_for_each(|(field, requested)| self.check(field, requested, form))
    }
}

/// RM.13's output allowance of one declared run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputAllowance {
    /// Canonical bytes the run may still emit as user output.
    pub remaining: u64,
    /// The terminal reserve: it survives exhaustion and never carries user output (LI.28).
    pub reserve: u64,
}

impl OutputAllowance {
    /// An allowance of `remaining` user bytes and a terminal reserve.
    pub fn new(remaining: u64, reserve: u64) -> OutputAllowance {
        OutputAllowance { remaining, reserve }
    }

    /// Charges one complete unit of `required` canonical bytes. At equality it passes. Above, it returns the
    /// `KN7:budget.output` stop and charges nothing for the denied unit; the reserve is never touched.
    pub fn charge(&mut self, required: u64) -> Result<(), Error> {
        if required > self.remaining {
            return Err(Error::stop(code::OUTPUT, Context::Output { required, remaining: self.remaining }));
        }
        self.remaining -= required;
        Ok(())
    }
}
