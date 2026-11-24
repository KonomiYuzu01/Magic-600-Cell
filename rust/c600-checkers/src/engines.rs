//! The exact engines, owned (LP5 plan, piece R8): the 600-cell-Full kernel and the jumbling engine loaded from a model
//! folder, for a host that runs checks and seals their run records without an edge of its own to the engine crates.
//! Loading checks every structural fact the engines rely on; nothing here checks a certificate, and nothing here is
//! part of a checker's identity.

use std::path::Path;
use std::sync::Arc;

use c600_jumble::twist::Menu;
use c600_jumble::Context;
use c600_kernel::json;
use c600_kernel::{IdentityBlock, Kernel};

use crate::report::{codes, Refusal};
use crate::Engines;

/// The two exact engines, loaded once and shared.
#[derive(Clone)]
pub struct Loaded {
    kernel: Arc<Kernel>,
    jumble: Arc<Context>,
}

impl Loaded {
    /// Loads both engines from the model folder `assets`. A model that does not load is a refusal with
    /// `KN6:evidence.no_checker`: no check can run without its engines.
    pub fn load(assets: &Path) -> Result<Loaded, Refusal> {
        let refuse = |engine: &str, error: String| Refusal {
            code: codes::NO_CHECKER,
            message: format!("the {engine} does not load: {error}"),
        };
        let kernel = Kernel::load(assets).map_err(|error| refuse("600-cell-Full kernel", error.to_string()))?;
        let jumble = Context::load(assets).map_err(|error| refuse("jumbling engine", error.to_string()))?;
        Ok(Loaded { kernel: Arc::new(kernel), jumble: Arc::new(jumble) })
    }

    /// The engines a check reads.
    pub fn engines(&self) -> Engines<'_> {
        Engines { kernel: Some(self.kernel.as_ref()), jumble: Some(self.jumble.as_ref()) }
    }

    /// The kernel, shared: a session store over the same model opens with it.
    pub fn kernel(&self) -> &Arc<Kernel> {
        &self.kernel
    }

    /// KN0.1's identity block of a 600-cell-Full input.
    pub fn full_identity(&self) -> IdentityBlock {
        self.kernel.identity()
    }

    /// KN0.1's identity block of a jumbling input under the menu record `menu`: the model, the order-free menu
    /// identity, the menu-order digest and the contract revision, as the engine reads the record. A record the engine
    /// does not read is refused with `KN6:evidence.certificate_rejected`.
    pub fn jumbling_identity(&self, menu: &[u8]) -> Result<IdentityBlock, Refusal> {
        let refuse =
            |error: String| Refusal { code: codes::CERTIFICATE_REJECTED, message: format!("the menu: {error}") };
        let doc = json::parse(menu).map_err(|error| refuse(error.to_string()))?;
        let menu = Menu::from_record(&self.jumble, &doc).map_err(refuse)?;
        Ok(self.jumble.identity(Some(&menu)))
    }
}

/// The kernel's build string: its crate version and the SHA-256 of a listing of its sources (DET.3's kernel API
/// identity of a run that reads it).
pub fn kernel_build() -> String {
    c600_kernel::build()
}
