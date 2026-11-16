//! The jumbling half of the kernel API of 600 Cell Script (LP2, milestone M5).
//!
//! This crate is a candidate engine for the rigid jumbling puzzle (state contract 2026-10-09 with amendments A1 to
//! A4), built from `assets/` alone: nothing is read from the research reference J1 (PHS.18). The differential
//! harness of `research/jumbling/differential/` compares it with J1 step by step through the adapter binary
//! (`src/bin/adapter.rs`), and `tests/test_jumbling_rust_candidate.py` runs that comparison (CT-KN2-01, jumbling
//! half).
//!
//! # Selections of the integrator for M5 (LP2 plan, section 11)
//!
//! 1. Numbers are exact: Q(√5) over big integers, and no floating type is used. The stored pole normals are binary64
//!    values; each is read as its exact dyadic rational and matched to the unique (p + q√5)/2 with |q| ≤ 12 within
//!    10⁻⁹, decided by exact comparisons ([`model::pole_coordinate`]).
//! 2. Journal records of the families `menu`, `matrix` and `cayley` are read. Another family, or a float request
//!    (`params.requested`), is an adapter error and no refusal ([`twist::Twist::from_record`]).
//! 3. Free mode: a state started without a menu admits any exact rotation that fixes the grip's pole. Tests use it; no
//!    product command does.
//! 4. Net protection at pose level: a twist that would change the pose of a protected piece is refused with
//!    `KN2:protection.piece_would_move`. The report gives the level (poses), the mode (net), the verdict and the sorted
//!    IDs of the protected pieces whose pose would change, and no first refused prefix. There is no label-level
//!    substitution.
//! 5. Batches ([`observe`]) take a positive maximum chunk size and a budget. Cancellation is checked between chunks,
//!    and a batch that is cut off or cancelled reports half-open coverage and makes no total or final claim (JI.14).
//! 6. A classification budget counts exact evaluations. When it runs out the twist is `uncertain` and nothing changes;
//!    the outcome is deterministic.
//!
//! # Identity
//!
//! The jumbling identity block names the puzzle, the manifest's `model_id`, the menu identity (order-free), the
//! menu-order digest and the contract revision ([`Context::identity`]). A value with another identity is refused with
//! `KN1:input.identity_mismatch`, naming the first differing field (KN1.4); it is never reinterpreted.

#![forbid(unsafe_code)]

pub mod b64;
pub mod dd;
pub mod eval;
pub mod kplus;
pub mod levels;
pub mod model;
pub mod observe;
pub mod q5;
pub mod regions;
pub mod state;
pub mod twist;

use std::path::Path;
use std::sync::OnceLock;

use c600_kernel::{IdentityBlock, Puzzle, Refusal};

use crate::kplus::KPlus;
use crate::model::{Model, ModelError};
use crate::regions::Regions;
use crate::twist::Menu;

/// The state contract revision the candidate implements.
pub const CONTRACT_REVISION: &str = "state-contract 2026-10-09 A1-A4";

/// Chapter 13 codes the candidate uses.
pub mod codes {
    pub use c600_kernel::codes::{IDENTITY_MISMATCH, NAMED_OBJECT_INVALID};
    /// A twist outside the signed menu.
    pub const TWIST_OFF_MENU: &str = "KN2:input.twist_off_menu";
    /// A protected piece's pose would change (pose-level protection).
    pub const PIECE_WOULD_MOVE: &str = "KN2:protection.piece_would_move";
    /// A stored word has a refused prefix at the current input.
    pub const WORD_BLOCKED_PREFIX: &str = "KN2:input.word_blocked_prefix";
    /// A request or record header names a dimension other than [`super::DIMENSION`] (JI.3, JI.45).
    pub const UNSUPPORTED_DIMENSION: &str = "KN1:input.unsupported_dimension";
}

/// The one dimension this edition admits (JI.3).
pub const DIMENSION: i64 = 4;

/// The loaded model with its rotation group and exact piece regions.
pub struct Context {
    pub model: Model,
    pub kplus: KPlus,
    pub regions: Regions,
    /// The observation tables, built on first use ([`observe::tables`]).
    observe: OnceLock<Result<observe::Tables, String>>,
}

impl Context {
    /// Reads `assets/` and builds K⁺ and the exact regions; every structural fact the engine relies on is checked.
    pub fn load(assets: &Path) -> Result<Context, ModelError> {
        let model = Model::load(assets)?;
        let kplus = KPlus::build(&model)?;
        let regions = Regions::build(&model, &kplus)?;
        Ok(Context { model, kplus, regions, observe: OnceLock::new() })
    }

    /// The jumbling identity block of a state with this menu (free mode: no menu digests).
    pub fn identity(&self, menu: Option<&Menu>) -> IdentityBlock {
        IdentityBlock {
            puzzle: Puzzle::Jumbling,
            model_id: self.model.model_id.clone(),
            menu_id: menu.map(|m| m.identity.clone()),
            menu_order: menu.map(|m| m.order.clone()),
            contract_rev: Some(CONTRACT_REVISION.to_owned()),
        }
    }

    /// KN1.4: refuses a value whose identity block differs, naming the first differing field.
    pub fn check_identity(&self, menu: Option<&Menu>, block: &IdentityBlock) -> Result<(), Refusal> {
        let own = self.identity(menu);
        let field = if block.puzzle != own.puzzle {
            "puzzle"
        } else if block.model_id != own.model_id {
            "model_id"
        } else if block.menu_id != own.menu_id {
            "menu_id"
        } else if block.menu_order != own.menu_order {
            "menu_order"
        } else if block.contract_rev != own.contract_rev {
            "contract_rev"
        } else {
            return Ok(());
        };
        Err(Refusal {
            code: codes::IDENTITY_MISMATCH,
            message: format!("The identity block differs from the loaded model in {field}"),
        })
    }
}
