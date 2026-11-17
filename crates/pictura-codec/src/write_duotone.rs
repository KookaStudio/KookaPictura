//! Duotone/Multichannel write-back: decide whether a document read from a
//! Duotone or 1/3-channel Multichannel PSD can be re-emitted as that header
//! mode with its retained plates. Split from `write.rs` for the file-size cap.

use pictura_core::{BitDepth, ColorMode, Document};

use crate::common::{MODE_DUOTONE, MODE_MULTICHANNEL};
use crate::write::composite_retained;

/// `Some((mode_code, out_color_channels, retained color planes))` when `doc`
/// can be written back flat as Duotone (mode 8, one gray plate) or
/// Multichannel (mode 7, one or three plates): the matching `source_mode`, an
/// 8-bit source with no recorded source depth (or `Eight`), a 3-channel working
/// composite, no layers or document extra channels, a merged composite, and
/// retained plates that still forward-convert to the current working RGB.
/// Edited or layered documents fall back to the working RGB; no plate layout or
/// duotone curve is invented.
pub(crate) fn flat_source_mode(
    doc: &Document,
    depth: u16,
    plane: usize,
) -> Option<(u16, usize, Vec<u8>)> {
    let mode = doc.source.mode()?;
    if !matches!(mode, ColorMode::Duotone | ColorMode::Multichannel)
        || depth != 8
        || !matches!(doc.source.depth(), None | Some(BitDepth::Eight))
        || doc.composite.channels != 3
        || !doc.composite.merged_output()
        || !doc.layers.is_empty()
        || !doc.channels.is_empty()
    {
        return None;
    }
    let n = if mode == ColorMode::Duotone {
        1
    } else {
        // Multichannel plate count is header-authoritative: derive it from the
        // retained store (color planes only; document extras already rejected).
        let store = doc.source.planes().as_ref()?;
        if plane == 0 || store.samples.len() % plane != 0 {
            return None;
        }
        let n = store.samples.len() / plane;
        if n != 1 && n != 3 {
            return None;
        }
        n
    };
    let retained: Vec<Vec<u8>> = (0..n)
        .map(|i| composite_retained(doc, depth, i).map(|s| s.to_bytes()))
        .collect::<Option<_>>()?;
    // `get` rather than a slice: a short `composite.data` must fall back to
    // RGB, not panic before the length validation in `write_container`.
    let current = doc.composite.data.get(..3 * plane)?;
    let ok = match n {
        1 => crate::color_mode::gray_to_rgb(&retained[0]).as_slice() == current,
        _ => crate::color_mode::cmy_to_rgb(&retained.concat()).as_slice() == current,
    };
    if !ok {
        return None;
    }
    let code = if mode == ColorMode::Duotone {
        MODE_DUOTONE
    } else {
        MODE_MULTICHANNEL
    };
    Some((code, n, retained.concat()))
}
