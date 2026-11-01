//! Layer info stored as a document-level `Lr16`/`Lr32` block, which is where
//! Photoshop puts 16- and 32-bit layers, leaving the layer-info subsection empty.

use super::*;
use crate::filter_effects::next_block;

/// Parse the block keyed for `depth` out of the trailing `extra` blocks and
/// remove it, so a save does not write it beside the rewritten layers. `None`
/// when `extra` has no such block.
pub(super) fn read_tagged_layer_info(
    extra: &mut Vec<u8>,
    is_psb: bool,
    color_channels: usize,
    depth: u16,
    retain: bool,
    budget: &mut DecodeBudget,
) -> Result<Option<(LayerStack, Option<Compression>)>, PsdError> {
    let Some(wanted) = tagged_layer_key(depth) else {
        return Ok(None);
    };
    let mut blocks = Reader::new(extra);
    let found = loop {
        let Some((key, start, end, data)) = next_block(&mut blocks, is_psb) else {
            return Ok(None);
        };
        if key == *wanted {
            break (start..end, data);
        }
    };
    let (range, data) = found;
    let read = read_layer_info(
        &mut Reader::new(data),
        is_psb,
        data.len(),
        color_channels,
        depth,
        retain,
        budget,
    )?;
    extra.drain(range);
    Ok(Some(read))
}
