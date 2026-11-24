//! Shared layer-lock predicates (design D5).
//!
//! The canonical definitions live in `pictura-core` because they are
//! layer-model predicates that `pictura-paint` also needs, and paint does not
//! depend on this crate. This module re-exports them for renderer and app
//! callers.

pub use pictura_core::{layer_move_locked, layer_pixel_locked, layer_transparency_locked};

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{Layer, LockFlags};

    fn with(flag: u8) -> Layer {
        pictura_codec::layer_from_parts(pictura_core::LayerParts {
            lock: LockFlags::default().with(flag, true),
            ..Default::default()
        })
    }

    #[test]
    fn each_predicate_reads_only_its_flag() {
        assert!(layer_move_locked(&with(LockFlags::POSITION)));
        assert!(!layer_pixel_locked(&with(LockFlags::POSITION)));
        assert!(!layer_transparency_locked(&with(LockFlags::POSITION)));

        assert!(layer_pixel_locked(&with(LockFlags::PIXELS)));
        assert!(!layer_move_locked(&with(LockFlags::PIXELS)));

        assert!(layer_transparency_locked(&with(LockFlags::TRANSPARENCY)));
        assert!(!layer_pixel_locked(&with(LockFlags::TRANSPARENCY)));
    }

    #[test]
    fn unlocked_layer_passes_every_predicate() {
        let layer = Layer::default();
        assert!(!layer_move_locked(&layer));
        assert!(!layer_pixel_locked(&layer));
        assert!(!layer_transparency_locked(&layer));
    }
}
