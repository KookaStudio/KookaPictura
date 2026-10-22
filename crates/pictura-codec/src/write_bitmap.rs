//! Bitmap write-back: decide whether a document read from a depth-1 Bitmap PSD
//! can be re-emitted byte-exactly as header mode Bitmap (0), and build the
//! image-data planes for both the Bitmap and RGB-fallback paths. Split from
//! `write.rs` to stay under the file-size cap.

use std::borrow::Cow;

use pictura_core::{BitDepth, ColorMode, Document};

use crate::error::PsdError;
use crate::write::composite_retained;

/// One channel's byte length at the depth-1 packed stride.
pub(crate) fn packed_plane_len(doc: &Document) -> usize {
    crate::depth::row_bytes(doc.width as usize, 1) * doc.height as usize
}

/// True when the document was read from a depth-1 Bitmap source.
pub(crate) fn is_bitmap_source(doc: &Document) -> bool {
    doc.source_mode == Some(ColorMode::Bitmap)
        && doc.source_planes.as_ref().map(|s| s.depth) == Some(BitDepth::One)
}

/// True when `doc` can be written back as a flat, unchanged Bitmap: a depth-1
/// Bitmap source with no layers whose retained packed plane still expands to the
/// current working RGB composite. Extra channels are allowed when they are all
/// packed at the depth-1 stride; they ride along verbatim. A Bitmap read expands
/// the packed bits to RGB, so an edit makes this false and the save falls back to
/// the working mode; no RGB-to-1-bit threshold is invented.
pub(crate) fn writes_bitmap(doc: &Document) -> bool {
    if !is_bitmap_source(doc)
        || doc.composite.channels != 3
        || !doc.merged_composite_present
        || !doc.layers.is_empty()
    {
        return false;
    }
    let packed = packed_plane_len(doc);
    if doc
        .channels
        .iter()
        .any(|channel| channel.data.len() != packed)
    {
        return false;
    }
    let Some(retained) = composite_retained(doc, 1, 0) else {
        return false;
    };
    let Some(retained) = retained.as_u8() else {
        return false;
    };
    // `get`-style length safety: a short `composite.data` on a public document
    // must fall back to RGB, not compare or slice out of bounds.
    crate::color_mode::bitmap_rows_to_rgb(retained, doc.width as usize, doc.height as usize)
        == doc.composite.data.as_ref()
}

/// Validate the extra channels for the selected output path. Bitmap output
/// requires the packed stride; an RGB fallback from a Bitmap source accepts
/// either packed or already-expanded 8-bit channels.
pub(crate) fn validate_extra_channels(doc: &Document, bitmap_mode: bool) -> Result<(), PsdError> {
    let packed = packed_plane_len(doc);
    let plane = doc.width as usize * doc.height as usize;
    for channel in &doc.channels {
        let len = channel.data.len();
        let ok = if bitmap_mode {
            len == packed
        } else if is_bitmap_source(doc) {
            len == packed || len == plane
        } else {
            len == plane
        };
        if !ok {
            return Err(PsdError::Invalid(format!(
                "document channel {} data length mismatch",
                channel.id
            )));
        }
    }
    Ok(())
}

/// The image-data planes for a Bitmap-mode write: the retained packed color
/// plane followed by the packed extras.
pub(crate) fn bitmap_planes(doc: &Document) -> Result<Vec<Cow<'_, [u8]>>, PsdError> {
    let packed = packed_plane_len(doc);
    let retained = doc
        .source_planes
        .as_ref()
        .and_then(|store| store.samples.as_u8())
        .ok_or_else(|| PsdError::Invalid("Bitmap packed plane missing".into()))?;
    // The read store holds every header channel at the packed stride, so the
    // color plane is its first `packed` bytes even when extras follow.
    let color = retained
        .get(..packed)
        .ok_or_else(|| PsdError::Invalid("Bitmap packed plane length mismatch".into()))?;
    let mut planes = Vec::with_capacity(1 + doc.channels.len());
    planes.push(Cow::Borrowed(color));
    for channel in &doc.channels {
        if channel.data.len() != packed {
            return Err(PsdError::Invalid(
                "Bitmap extra channel length mismatch".into(),
            ));
        }
        planes.push(Cow::Borrowed(channel.data.as_slice()));
    }
    Ok(planes)
}

/// Expand the extra channels for the RGB fallback. Packed channels become one
/// 8-bit sample per pixel; an already-expanded channel passes through.
pub(crate) fn fallback_extra_planes(doc: &Document) -> Result<Vec<Vec<u8>>, PsdError> {
    let packed = packed_plane_len(doc);
    let plane = doc.width as usize * doc.height as usize;
    let mut out = Vec::with_capacity(doc.channels.len());
    for channel in &doc.channels {
        if channel.data.len() == plane {
            out.push(channel.data.to_vec());
        } else if channel.data.len() == packed {
            let expanded = crate::color_mode::bitmap_rows_to_rgb(
                &channel.data,
                doc.width as usize,
                doc.height as usize,
            );
            out.push(expanded[..plane].to_vec());
        } else {
            return Err(PsdError::Invalid(format!(
                "document channel {} data length mismatch",
                channel.id
            )));
        }
    }
    Ok(out)
}
