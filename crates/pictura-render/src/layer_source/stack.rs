//! Stack-mode frame composition and reduction for embedded smart objects.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use pictura_core::{Document, Layer, PixelBuffer, SmartObject, StackMode};

use super::{decode_payload, merged_source, MAX_EMBED_NESTING};

/// The per-band frame memory budget: `frames × rows × width × 4` bytes.
const FRAME_BUDGET: usize = 32 * 1024 * 1024;
/// The band never exceeds this many rows.
const MAX_BAND_ROWS: u32 = 256;
/// The frame-count cache is tiny; cap it so a long session cannot grow it.
const MAX_FRAME_COUNT_ENTRIES: usize = 256;

fn frame_counts() -> &'static Mutex<HashMap<u64, Option<usize>>> {
    static COUNTS: OnceLock<Mutex<HashMap<u64, Option<usize>>>> = OnceLock::new();
    COUNTS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) fn purge_frame_counts() {
    frame_counts()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
}

/// How many visible top-level frames the embedded payload holds, or `None`
/// when the object is not embedded, has an empty or unreadable payload, or is
/// nested past [`MAX_EMBED_NESTING`]. Cached by payload identity so the menu's
/// enablement checks do not decode the whole source on every refresh.
pub(crate) fn stack_frame_count(so: &SmartObject) -> Option<usize> {
    if so.kind != pictura_core::SmartObjectKind::Embedded || so.nesting >= MAX_EMBED_NESTING {
        return None;
    }
    so.payload().filter(|payload| !payload.is_empty())?;
    let key = so.payload_digest();
    let cached = frame_counts()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&key)
        .copied();
    if let Some(count) = cached {
        return count;
    }
    let count = decode_payload(so)
        .map(|embedded| embedded.layers.iter().filter(|layer| layer.visible).count());
    let mut counts = frame_counts().lock().unwrap_or_else(|e| e.into_inner());
    if counts.len() >= MAX_FRAME_COUNT_ENTRIES {
        counts.clear();
    }
    counts.insert(key, count);
    count
}

/// The reduced stack source when the object carries a mode and has at least two
/// visible frames; otherwise the ordinary merged source. The caller caches the
/// result under a key that includes the active mode.
pub(super) fn stack_or_merged(so: &SmartObject) -> Option<PixelBuffer> {
    let embedded = decode_payload(so)?;
    if let Some(mode) = so.stack_mode {
        if let Some(buffer) = reduce_document(&embedded, mode) {
            return Some(buffer);
        }
    }
    merged_source(&embedded)
}

fn reduce_document(doc: &Document, mode: StackMode) -> Option<PixelBuffer> {
    let frames: Vec<&Layer> = doc.layers.iter().filter(|layer| layer.visible).collect();
    if frames.len() < 2 || doc.width == 0 || doc.height == 0 {
        return None;
    }
    let band_rows = band_rows(doc.width, frames.len(), doc.height);
    let out_width = doc.width as usize;
    let out_plane = out_width * doc.height as usize;
    let mut out = PixelBuffer::new(doc.width, doc.height, 4);
    let mut y0 = 0;
    while y0 < doc.height {
        let rows = band_rows.min(doc.height - y0);
        let bands: Vec<PixelBuffer> = frames
            .iter()
            .map(|layer| {
                crate::composite_region::composite_layer_region(layer, doc, 0, y0, doc.width, rows)
            })
            .collect();
        let reduced = pictura_core::reduce_stack(&bands, mode);
        let band_plane = out_width * rows as usize;
        let out_data = out.data.as_mut_slice();
        let band_data = reduced.data.as_slice();
        for channel in 0..4 {
            for row in 0..rows as usize {
                let src = channel * band_plane + row * out_width;
                let dst = channel * out_plane + (y0 as usize + row) * out_width;
                out_data[dst..dst + out_width].copy_from_slice(&band_data[src..src + out_width]);
            }
        }
        y0 += rows;
    }
    Some(out)
}

fn band_rows(width: u32, frames: usize, height: u32) -> u32 {
    let per_row = width as usize * frames.max(1) * 4;
    let rows = (FRAME_BUDGET / per_row.max(1)).clamp(1, MAX_BAND_ROWS as usize);
    (rows as u32).min(height.max(1))
}
