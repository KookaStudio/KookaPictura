use super::{display_region, into_rgba_frame, mask_region_plane};
use pictura_core::{Document, LayerMask, PsdRect};
use pictura_render::{TileKey, TilePyramid, TileSource};
use std::collections::HashSet;
use std::sync::Arc;

/// Build `keys` into `local`, finer levels first, folding coarser tiles from
/// the finer ones at hand; returns every tile built, the levels folded
/// through included.
pub(super) fn build_keys(
    doc: &Document,
    mask: Option<&LayerMask>,
    painting: bool,
    preview: Option<&pictura_codec::MergedPreview>,
    hdr: Option<pictura_codec::hdr_preview::PreviewOptions>,
    hdr_merged: bool,
    local: &mut TilePyramid,
    keys: &[TileKey],
) -> Vec<(TileKey, Arc<[u8]>)> {
    let mut built = Vec::new();
    let sources: HashSet<_> = keys.iter().map(|key| key.source).collect();
    for source in sources {
        let keys: Vec<TileKey> = keys
            .iter()
            .copied()
            .filter(|k| k.source == source)
            .collect();
        if let TileSource::Hdr(offset) = source {
            if let Some(options) = hdr {
                let level0 = |rect| {
                    super::super::hdr_preview::hdr_display_region(
                        doc,
                        rect,
                        options,
                        offset,
                        hdr_merged && !painting,
                    )
                };
                built.extend(pictura_render::build_keys(local, &keys, &level0));
            }
            continue;
        }
        built.extend(match (source, mask, preview) {
            (TileSource::Mask, Some(mask), _) => {
                let level0 = |r: PsdRect| mask_region_plane(mask, r);
                pictura_render::build_keys(local, &keys, &level0)
            }
            (_, _, Some(preview)) if !painting => {
                let level0 = |r: PsdRect| {
                    into_rgba_frame(
                        pictura_codec::buffer_to_srgb(doc, &preview.region(r)).into_owned(),
                    )
                };
                pictura_render::build_keys(local, &keys, &level0)
            }
            _ => {
                let level0 = |r: PsdRect| display_region(doc, r, painting);
                pictura_render::build_keys(local, &keys, &level0)
            }
        });
    }
    built
}
