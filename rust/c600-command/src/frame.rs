//! KN5's stage 5: the adapter's report of a presented frame (RM.26) and the frame a word commit's token binds (ONE.41).
//!
//! The display adapter reports each frame whose presentation it confirmed as a [`PresentedFrame`]: the frame's
//! [`FrameStamp`], the preview it shows ([`ShownPreview`], or none) and the chain that drew that preview
//! ([`FrameChain`]). [`crate::CommandLayer::present`] reads the reports in order. Only a frame that shows the pending
//! preview through the built-in chain presents it; a frame that shows no preview, another preview, or a preview drawn
//! through a user stage (RPI.76) ends its presentation, and the layer refuses it. A word commit's token binds the stamp
//! of the frame that presented its preview, and the commit is refused unless that frame belongs to the preview's
//! current presentation: the run of frames since the newest frame that started showing it, on one device.
//!
//! The adapter numbers its frames with a sequence that increases within a device epoch, and its device epochs increase
//! (`c600-display`'s host keeps one sequence for all its frames and a new epoch at each recovery). The layer depends on
//! the kernel only, so these types mirror the display's `PresentedFrame`, `FrameStamp` and `PreviewShown` and do not
//! name them; `c600-tools` maps one to the other.

use c600_kernel::json::{self, Value};

use crate::layer::{CommandLayer, Preview};
use crate::receipt::Error;
use crate::shell::Shell;
use crate::store::Store;
use crate::token::PreviewBinding;

/// What drew the frame's preview: the built-in chain, or a chain with user stages. A preview drawn through a user stage
/// is an extra display with no presented-preview identity (RPI.76), so it never presents a preview.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameChain {
    BuiltIn,
    UserStage,
}

/// The identity of one presented frame on the adapter's device (RM.24's device epoch, surface, surface generation and
/// sequence).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameStamp {
    pub device_epoch: u64,
    pub surface: u64,
    pub surface_generation: u64,
    /// Increasing within one device epoch.
    pub sequence: u64,
}

impl FrameStamp {
    /// The order of frame reports: by device epoch, then by sequence.
    pub fn order(&self) -> (u64, u64) {
        (self.device_epoch, self.sequence)
    }

    /// Whether two frames belong to one device epoch, surface and surface generation. A new device or a reopened
    /// surface needs a fresh report (RM.26).
    pub fn same_surface(&self, other: &FrameStamp) -> bool {
        self.device_epoch == other.device_epoch
            && self.surface == other.surface
            && self.surface_generation == other.surface_generation
    }

    pub fn to_json(&self) -> Value {
        let nat = |n: u64| Value::Int(n as i64);
        json::object([
            ("device_epoch", nat(self.device_epoch)),
            ("surface", nat(self.surface)),
            ("surface_generation", nat(self.surface_generation)),
            ("sequence", nat(self.sequence)),
        ])
    }
}

/// The preview a frame shows: its ID, its literal digest and its base state revision (RM.26's `PreviewShown`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShownPreview {
    pub id: u64,
    pub literal: String,
    pub base: u64,
}

impl ShownPreview {
    /// What a frame that shows `preview` reports of it.
    pub fn of(preview: &Preview) -> ShownPreview {
        ShownPreview { id: preview.id, literal: preview.literal.clone(), base: preview.deps.state }
    }

    /// What a frame that shows the preview a token binds reports of it.
    pub fn of_binding(binding: &PreviewBinding) -> ShownPreview {
        ShownPreview { id: binding.id, literal: binding.literal.clone(), base: binding.deps.state }
    }
}

/// RM.26's report of one frame whose presentation the adapter confirmed. `preview: None` is the explicit `no_preview`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentedFrame {
    pub preview: Option<ShownPreview>,
    pub stamp: FrameStamp,
    pub chain: FrameChain,
}

impl PresentedFrame {
    /// A frame that shows `preview` through the built-in chain.
    pub fn built_in(preview: &Preview, stamp: FrameStamp) -> PresentedFrame {
        PresentedFrame { preview: Some(ShownPreview::of(preview)), stamp, chain: FrameChain::BuiltIn }
    }

    /// Whether the frame presents `shown`: it shows that preview, drawn by the built-in chain.
    pub fn presents(&self, shown: &ShownPreview) -> bool {
        self.chain == FrameChain::BuiltIn && self.preview.as_ref() == Some(shown)
    }
}

/// A headless adapter's frames: one device epoch and one surface, with a sequence that counts up. The loop runners and
/// the tests present a preview with it as a display whose built-in chain drew the preview would report it.
#[derive(Clone, Debug)]
pub struct HeadlessFrames {
    stamp: FrameStamp,
}

impl Default for HeadlessFrames {
    fn default() -> Self {
        HeadlessFrames::new()
    }
}

impl HeadlessFrames {
    pub fn new() -> HeadlessFrames {
        HeadlessFrames { stamp: FrameStamp { device_epoch: 1, surface: 1, surface_generation: 1, sequence: 0 } }
    }

    /// Frames that continue after the newest frame reported to `layer`, on its device epoch and surface, or from the
    /// start when none was reported.
    pub fn after<S: Store>(layer: &CommandLayer<S>) -> HeadlessFrames {
        match layer.frame() {
            Some(frame) => HeadlessFrames { stamp: frame.stamp.clone() },
            None => HeadlessFrames::new(),
        }
    }

    /// The next frame's stamp.
    pub fn next(&mut self) -> FrameStamp {
        self.stamp.sequence += 1;
        self.stamp.clone()
    }

    /// The next frame, showing `preview` through the built-in chain.
    pub fn built_in(&mut self, preview: &Preview) -> PresentedFrame {
        PresentedFrame::built_in(preview, self.next())
    }

    /// The next frame, showing `preview` drawn through a user stage, or no preview at all (`None`).
    pub fn other(&mut self, preview: Option<&Preview>, chain: FrameChain) -> PresentedFrame {
        PresentedFrame { preview: preview.map(ShownPreview::of), stamp: self.next(), chain }
    }

    /// A new device epoch, as a recovery would start: the next frames belong to it.
    pub fn recover(&mut self) {
        self.stamp.device_epoch += 1;
    }

    /// A headless display's next frame: it shows the layer's pending preview through the built-in chain, or no preview
    /// when none is pending, and the shell reports it for preview `id` ([`Shell::present`]).
    pub fn present<S: Store>(&mut self, shell: &mut Shell, layer: &mut CommandLayer<S>, id: u64) -> Result<(), Error> {
        let frame = self.other(layer.preview(), FrameChain::BuiltIn);
        shell.present(layer, id, &frame)
    }
}
