use crate::cxxqt_object::{helpers::active_mask_target, state::PictureViewRust};
use pictura_core::{rec601_luma, Coverage, Document, Layer, LayerStack, PsdRect};
use pictura_render::{composite_region, content_route, resolve_path, ContentRoute};
use pictura_select::quick_mask::working_value;

type Bins = [u32; 256];

pub(super) fn threshold_bins(rust: &PictureViewRust, path: &str) -> Bins {
    let mut bins = [0; 256];
    let Some(doc) = rust.session.doc.as_ref() else {
        return bins;
    };
    let Some(mut region) = (PsdRect {
        left: 0,
        top: 0,
        right: doc.width as i32,
        bottom: doc.height as i32,
    })
    .clip_to(doc.width, doc.height) else {
        return bins;
    };
    let mut selection = rust.session.selection.as_ref().map(|s| s.coverage());
    if selection.is_some() {
        let bounds = if path.is_empty() {
            rust.selection_mask_bounds()
        } else {
            rust.selection_bounds()
        };
        let Some(bounds) = bounds.and_then(|r| r.intersect(region)) else {
            return bins;
        };
        region = bounds;
        if !path.is_empty() {
            if let Some(&(dx, dy)) = rust.session.live.selection_move_delta() {
                let Some(translated) = dx.checked_neg().zip(dy.checked_neg()).and_then(|(x, y)| {
                    selection.as_ref().and_then(|coverage| coverage.rebased(x, y))
                }) else {
                    return bins;
                };
                selection = Some(translated);
            }
        }
    }
    let selection = selection.as_ref();
    if !path.is_empty() {
        if let Some(source) = below_threshold(doc, path) {
            for tile in sample_tiles(region) {
                let buffer = composite_region(&source, tile);
                let width = buffer.width as usize;
                let pixels = width * buffer.height as usize;
                add_samples(&mut bins, tile, selection, |x, y| {
                    let i = (y - tile.top) as usize * width + (x - tile.left) as usize;
                    let sample = |band| buffer.data.sample(i + band * pixels);
                    (sample(3)? != 0).then_some(())?;
                    Some(rec601_luma(sample(0)?, sample(1)?, sample(2)?))
                });
            }
        }
        return bins;
    }

    let quick_mask = &rust.session.quick_mask;
    if quick_mask.active {
        let coverage = quick_mask.coverage.as_ref().map(|s| s.coverage());
        for tile in sample_tiles(region) {
            add_samples(&mut bins, tile, selection, |x, y| {
                let value = coverage.as_ref().map_or(255, |mask| mask.sample(x, y));
                Some(working_value(value, quick_mask.selected))
            });
        }
    } else if let Some(path) = active_mask_target(rust) {
        if let Some(mask) = resolve_path(doc, &path).and_then(|layer| layer.mask.as_ref()) {
            // The filter's mask document presents a data-less mask as white.
            let coverage = match &mask.data {
                None => Some(Coverage::constant(255)),
                Some(_) => mask.raw_coverage(),
            };
            if let Some(coverage) = coverage {
                for tile in sample_tiles(region) {
                    add_samples(&mut bins, tile, selection, |x, y| Some(coverage.sample(x, y)));
                }
            }
        }
    } else if let Some(layer) = rust.layer_target.single().and_then(|p| resolve_path(doc, p)) {
        add_raster_samples(&mut bins, layer, region, selection);
    }
    bins
}

fn add_raster_samples(bins: &mut Bins, layer: &Layer, region: PsdRect, selection: Option<&Coverage>) {
    let Some(region) = region.intersect(layer.rect) else {
        return;
    };
    let plane = |id| layer.channels().iter().find(|c| c.id == id).map(|c| &c.data);
    let Some(red) = plane(0) else { return };
    let (green, blue) = match (plane(1), plane(2)) {
        (Some(g), Some(b)) => (g, b),
        (None, None) => (red, red),
        _ => return,
    };
    let alpha = plane(-1);
    let width = layer.rect.width() as usize;
    for tile in sample_tiles(region) {
        add_samples(bins, tile, selection, |x, y| {
            let i = (y - layer.rect.top) as usize * width + (x - layer.rect.left) as usize;
            if alpha.is_some_and(|a| a.sample(i) == Some(0)) {
                return None;
            }
            Some(rec601_luma(red.sample(i)?, green.sample(i)?, blue.sample(i)?))
        });
    }
}

fn add_samples(
    bins: &mut Bins,
    tile: PsdRect,
    selection: Option<&Coverage>,
    luma: impl Fn(i32, i32) -> Option<u8>,
) {
    for y in tile.top..tile.bottom {
        for x in tile.left..tile.right {
            let weight = selection.map_or(255, |coverage| coverage.sample(x, y));
            if weight == 0 {
                continue;
            }
            if let Some(luma) = luma(x, y) {
                bins[usize::from(luma)] += u32::from(weight);
            }
        }
    }
}

fn sample_tiles(region: PsdRect) -> impl Iterator<Item = PsdRect> {
    const TILE: i32 = 128;
    const GRID: i32 = 4;
    let axis = |origin: i32, span: i32| {
        let cells = ((span - 1) / TILE + 1).min(GRID);
        (0..cells).map(move |i| {
            let low = origin + i * span / cells;
            let high = origin + (i + 1) * span / cells;
            let side = (high - low).min(TILE);
            let start = low + (high - low - side) / 2;
            (start, start + side)
        })
    };
    // Center a tile in each cell so large sources have bounded work without
    // smoothing away tones. Sources at most 512 pixels per axis have no gaps.
    axis(region.top, region.height()).flat_map(move |(top, bottom)| {
        axis(region.left, region.width()).map(move |(left, right)| PsdRect {
            top,
            left,
            bottom,
            right,
        })
    })
}

fn below_threshold(doc: &Document, path: &str) -> Option<Document> {
    let layer = resolve_path(doc, path)?;
    if !layer.has_adjustment_content() || layer.adjustment()?.key != *b"thrs" {
        return None;
    }
    let indexes: Vec<usize> = path
        .split('/')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    let (_, ancestors) = indexes.split_last()?;
    let mut scope = doc.layers.as_slice();
    let mut branch = scope;
    let mut first_index = 0;
    for (depth, &index) in ancestors.iter().enumerate() {
        let group = branch.get(index)?;
        branch = group.children();
        if matches!(content_route(group), ContentRoute::IsolatedGroup) {
            scope = branch;
            first_index = depth + 1;
        }
    }
    let layers = prefix(scope, &indexes[first_index..])?;
    if layers.is_empty() {
        return None;
    }
    let mut source = doc.clone();
    source.layers = layers;
    Some(source)
}

/// Keep lower siblings at each pass-through level, including the partial group.
fn prefix(layers: &[Layer], indexes: &[usize]) -> Option<LayerStack> {
    let (&index, children) = indexes.split_first()?;
    let mut lower = layers.get(..index)?.to_vec();
    if !children.is_empty() {
        let mut group = layers.get(index)?.clone();
        let below = prefix(group.children(), children)?;
        if !below.is_empty() {
            *group.children_mut()? = below;
            lower.push(group);
        }
    }
    Some(LayerStack::from_layers(lower))
}

#[cfg(test)]
mod tests;
