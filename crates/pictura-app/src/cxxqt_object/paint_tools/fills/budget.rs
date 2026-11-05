//! The memory gate the fill family passes before it allocates (#1207). An edit
//! estimates the bytes it will hold at its peak and is refused, instead of
//! being left to the kernel's OOM killer, when they exceed three quarters of
//! the memory available now: the rule the Image Size gate uses (#1156).

use pictura_core::{Document, PsdRect, BAND_ROWS};
use std::cell::Cell;

std::thread_local! {
    /// Whether the last fill was refused by this gate.
    static REFUSED: Cell<bool> = const { Cell::new(false) };
    /// The available memory a test pretends the host has.
    static AVAILABLE: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Forget the last refusal; every fill starts here.
pub(super) fn clear() {
    REFUSED.set(false);
}

/// Whether `bytes` more fit the memory available now. A refusal is remembered
/// for [`fill_refused_for_memory`].
pub(super) fn admit(bytes: u64) -> bool {
    let available = AVAILABLE.get().or_else(pictura_codec::available_memory);
    let fits = fits(bytes, available);
    if !fits {
        REFUSED.set(true);
    }
    fits
}

/// The gate's rule: three quarters of `available`, and no limit when the
/// host's memory cannot be read.
fn fits(bytes: u64, available: Option<u64>) -> bool {
    available.is_none_or(|available| bytes <= available / 4 * 3)
}

/// The pixels of `bounds`.
pub(super) fn area(bounds: PsdRect) -> u64 {
    bounds.width().max(0) as u64 * bounds.height().max(0) as u64
}

/// The bytes a write of `bounds` into the layer at `path` forks from history:
/// each 64-row band it crosses, of each colour and transparency plane, at the
/// layer's width.
///
/// ponytail: 8-bit planes only; a 16- or 32-bit native store written beside
/// them is not charged, so the gate under-counts a deep document's fill.
pub(super) fn fork_bytes(doc: &Document, path: &str, bounds: PsdRect) -> u64 {
    let Some(layer) = pictura_render::resolve_path(doc, path) else {
        return 0;
    };
    let rect = layer.rect;
    let Some(hit) = bounds.intersect(rect) else {
        return 0;
    };
    let band = BAND_ROWS as i32;
    let first = (hit.top - rect.top) / band * band;
    let last = ((hit.bottom - rect.top + band - 1) / band * band).min(rect.height());
    let planes = layer.channels.iter().filter(|c| c.id >= -1).count() as u64;
    planes * (last - first).max(0) as u64 * rect.width().max(0) as u64
}

/// Whether the last fill was refused because it did not fit the memory
/// available.
pub(super) fn fill_refused_for_memory() -> bool {
    REFUSED.get()
}

/// Pretend the host has `bytes` of memory available to the fills; a negative
/// value reads the host again.
pub(super) fn set_fill_available_memory_for_test(bytes: i64) {
    AVAILABLE.set(u64::try_from(bytes).ok());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gate_keeps_a_quarter_of_the_memory_free() {
        assert!(fits(750, Some(1000)));
        assert!(!fits(751, Some(1000)));
        assert!(fits(u64::MAX, None), "an unreadable host is not a refusal");
    }

    #[test]
    fn a_refusal_is_remembered_until_the_next_fill() {
        set_fill_available_memory_for_test(1000);
        clear();
        assert!(admit(700));
        assert!(!fill_refused_for_memory());
        assert!(!admit(800));
        assert!(fill_refused_for_memory());
        clear();
        assert!(!fill_refused_for_memory());
        set_fill_available_memory_for_test(-1);
    }

    #[test]
    fn a_write_forks_the_bands_it_crosses() {
        let mut doc = Document::new(
            100,
            300,
            pictura_core::ColorMode::Rgb,
            pictura_core::BitDepth::Eight,
        );
        doc.layers =
            pictura_core::LayerStack::from_layers(vec![crate::cxxqt_object::tests::pixel_layer(
                "layer",
                100,
                300,
                (0, 0, 0),
            )]);
        let planes = doc.layers[0].channels.iter().filter(|c| c.id >= -1).count() as u64;
        let rows = |top, bottom| PsdRect {
            top,
            left: 10,
            bottom,
            right: 20,
        };
        // Rows 70..140 cross bands 1 and 2: rows 64..192.
        assert_eq!(fork_bytes(&doc, "0", rows(70, 140)), planes * 128 * 100);
        // The last band is cut at the layer's bottom.
        assert_eq!(fork_bytes(&doc, "0", rows(290, 300)), planes * 44 * 100);
        assert_eq!(fork_bytes(&doc, "0", rows(400, 500)), 0, "off the layer");
        assert_eq!(fork_bytes(&doc, "9", rows(0, 10)), 0, "no such layer");
    }
}
