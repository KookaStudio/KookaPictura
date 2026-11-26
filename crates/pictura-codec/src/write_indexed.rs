//! Indexed write-back: decide whether a document read from an Indexed PSD can be
//! re-emitted byte-exactly, and supply each pixel layer's retained index plane.
//! Split from `write.rs` to stay under the file-size cap.

use pictura_core::{ColorMode, Document, Layer};

use crate::write::{composite_retained, layer_retained};

/// The three working RGB planes a layer's color channels carry, when the layer
/// has exactly one each of ids `0`/`1`/`2` of equal length. `None` otherwise.
fn layer_rgb_planes(layer: &Layer) -> Option<Vec<&[u8]>> {
    let positions: Vec<usize> = layer
        .channels()
        .iter()
        .enumerate()
        .filter(|(_, c)| c.id >= 0 && c.id < 3)
        .map(|(i, _)| i)
        .collect();
    if positions.len() != 3 {
        return None;
    }
    let plane = layer.channels()[positions[0]].data.len();
    if positions
        .iter()
        .any(|&i| layer.channels()[i].data.len() != plane)
    {
        return None;
    }
    Some(
        positions
            .iter()
            .map(|&i| layer.channels()[i].data.as_slice())
            .collect(),
    )
}

/// True when every non-group layer that carries RGB color can be rebuilt from
/// its retained index plane. A group recurses; a layer with no RGB color
/// channels (an adjustment or empty layer) does not participate.
fn indexed_layers_unchanged(layers: &[Layer], palette: &[u8; 768], depth: u16, clear: Option<u8>) -> bool {
    layers.iter().all(|layer| {
        if layer.is_group() {
            return indexed_layers_unchanged(layer.children(), palette, depth, clear);
        }
        let Some(rgb) = layer_rgb_planes(layer) else {
            return true;
        };
        let Some(index) = layer_retained(layer, depth, 0).map(|s| s.to_bytes()) else {
            return false;
        };
        if crate::color_mode::indexed_to_rgb(&index, palette) != rgb.concat() {
            return false;
        }
        if let Some(clear) = clear {
            let alpha = layer.channels().iter().find(|channel| channel.id == -1);
            if index.iter().enumerate().any(|(i, &value)| {
                value == clear && alpha.is_none_or(|channel| channel.data.get(i) != Some(&0))
            }) {
                return false;
            }
        }
        true
    })
}

/// True when `doc` can be written back byte-exactly as Indexed: an 8-bit Indexed
/// source whose retained palette and every retained index plane (the composite's
/// when a merged composite is present, and each pixel layer's) still expand to
/// the current working RGB. Any edit, an added color layer, or a dropped palette
/// makes this false, so the save falls back to the working RGB rather than
/// inventing an RGB-to-palette quantization.
fn writes_indexed_impl(doc: &Document, depth: u16, plane: usize, allow_opaque_rgba: bool) -> bool {
    if depth != 8 || doc.source.depth().is_some() || doc.source.mode() != Some(ColorMode::Indexed) {
        return false;
    }
    let clear = crate::indexed_metadata(doc).1;
    let composite_channels = doc.composite.channels;
    if composite_channels != 3 && !((allow_opaque_rgba || clear.is_some()) && composite_channels == 4) {
        return false;
    }
    let Some(palette) = doc.source.palette() else {
        return false;
    };
    // A file with "Maximize Compatibility" off carries no merged composite, so
    // there is no composite plane to check; the layers still must reconstruct.
    if doc.composite.merged_output() {
        let Some(index) = composite_retained(doc, depth, 0).map(|s| s.to_bytes()) else {
            return false;
        };
        // `get` rather than a slice: `write_psd` is public and a short
        // `composite.data` must fall back to RGB, not panic before the length
        // validation in `write_container`.
        let Some(current) = doc.composite.data.get(..3 * plane) else {
            return false;
        };
        if (0..3).any(|c| {
            current[c * plane..(c + 1) * plane].iter().zip(&index).any(|(&rgb, &i)| {
                Some(i) != clear && rgb != palette[c * 256 + usize::from(i)]
            })
        }) {
            return false;
        }
        if composite_channels == 4 {
            let Some(alpha) = doc.composite.data.get(3 * plane..4 * plane) else {
                return false;
            };
            if alpha.iter().zip(&index).any(|(&value, &i)| {
                value != if Some(i) == clear { 0 } else { 255 }
            }) {
                return false;
            }
        } else if clear.is_some_and(|i| index.contains(&i)) {
            return false;
        }
    }
    indexed_layers_unchanged(&doc.layers, &palette, depth, clear)
}

/// An 8-bit Indexed source whose retained palette and index planes still
/// expand to the current composite and layers. A transparent index uses RGBA.
pub(crate) fn writes_indexed(doc: &Document, depth: u16, plane: usize) -> bool {
    writes_indexed_impl(doc, depth, plane, false)
}

/// Whether the document's retained Indexed palette and index plane still
/// expand to its current composite, so a save can re-emit them byte-exactly.
/// An edit or a dropped palette makes this false.
pub fn indexed_source_is_current(doc: &Document) -> bool {
    writes_indexed_impl(doc, 8, doc.width as usize * doc.height as usize, false)
}

/// Whether the retained Indexed palette and index plane still expand to the
/// current composite when the app's full refresh has stored the working RGBA
/// frame instead of the source's three planes. The RGB planes must match and
/// the alpha plane must agree with the retained transparent index.
pub fn gif_indexed_source_is_current(doc: &Document) -> bool {
    writes_indexed_impl(doc, 8, doc.width as usize * doc.height as usize, true)
}

/// The retained index plane to emit for an Indexed output's pixel layer, or
/// `None` when the layer carries no RGB color (a group/adjustment) or has no
/// retained index channel. [`writes_indexed`] already guaranteed the retained
/// plane still expands to the layer's RGB.
pub(crate) fn index_layer_plane(layer: &Layer, depth: u16) -> Option<Vec<u8>> {
    layer_rgb_planes(layer)?;
    layer_retained(layer, depth, 0).map(|s| s.to_bytes())
}
