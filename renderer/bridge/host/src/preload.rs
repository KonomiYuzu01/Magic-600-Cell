//! Charter E3's startup (a draft): the shipped model's loads, started on threads of their own before the shell has a
//! core context ([`crate::m6h_preload`]), so that they run while the shell opens the core's surface, and beside each
//! other. The host joins each load where it would otherwise make it: [`crate::m6h_create_preloaded`] the kernel and
//! the verified geometry, [`crate::m6h_open_session`] the exact model.
//!
//! - A load on its own thread gives the result and the error it gives on the caller's thread; a load that panics
//!   panics again where it is joined. A load whose thread did not start runs where it is joined.
//! - Dropping a load waits for its thread, so that no load runs after its preload or handle is freed (the shell frees
//!   the library after that).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use c600_display::produce::GeometryInput;
use c600_jumble::model::ModelError;
use c600_jumble::Context as Exact;
use c600_kernel::parallel::Background;
use c600_kernel::{Kernel, LoadError};

/// One load from the assets folder: running on a thread of its own, or still to run where it is joined.
pub type Load<T> = Background<T>;

/// `load` of `assets` on a new thread named `name`.
fn started<T: Send + 'static>(name: &str, assets: &Path, load: fn(&Path) -> T) -> Load<T> {
    let path = assets.to_path_buf();
    Background::start(name, move || load(&path))
}

/// `load` of `assets`, run where it is joined.
fn deferred<T: Send + 'static>(assets: &Path, load: fn(&Path) -> T) -> Load<T> {
    let path = assets.to_path_buf();
    Background::deferred(move || load(&path))
}

/// The exact model's load, which a session's twist menu reads ([`crate::turns`]).
pub type ExactLoad = Load<Result<Exact, ModelError>>;

/// What [`crate::m6h_preload`] hands out and [`crate::m6h_create_preloaded`] takes.
pub struct Preload {
    pub(crate) assets: PathBuf,
    pub(crate) kernel: Load<Result<Kernel, LoadError>>,
    pub(crate) geometry: Load<Result<GeometryInput, LoadError>>,
    pub(crate) exact: Option<ExactLoad>,
}

impl Preload {
    /// Starts the kernel's and the geometry's loads from `assets`, and with `session` the exact model's, each on its
    /// own thread.
    pub fn start(assets: &Path, session: bool) -> Preload {
        Preload {
            assets: assets.to_path_buf(),
            kernel: started("c600-shell-load-kernel", assets, Kernel::load),
            geometry: started("c600-shell-load-geometry", assets, GeometryInput::load),
            exact: session.then(|| started("c600-shell-load-exact", assets, Exact::load)),
        }
    }

    /// The loads of [`crate::m6h_create`]: the kernel's and the geometry's, each run where it is joined, on the
    /// caller's thread and in that order; no exact model's.
    pub(crate) fn deferred(assets: &Path) -> Preload {
        Preload {
            assets: assets.to_path_buf(),
            kernel: deferred(assets, Kernel::load),
            geometry: deferred(assets, GeometryInput::load),
            exact: None,
        }
    }
}

/// The exact model of `assets` for a session, as [`crate::turns::Turns::open`] loads it.
pub fn load_exact(assets: &Path) -> Result<Arc<Exact>, String> {
    exact_result(Exact::load(assets))
}

/// The exact model from a load's result, with the session's error text.
pub fn exact_result(loaded: Result<Exact, ModelError>) -> Result<Arc<Exact>, String> {
    loaded.map(Arc::new).map_err(|error| format!("the exact model: {error:?}"))
}
