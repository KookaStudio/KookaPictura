//! Move the pixels covered by a selection mask (design D5). Reuses the
//! `Layer via Copy`/`Layer via Cut` primitives and translates the resulting
//! layer's rectangle.

use pictura_core::{
    layer_move_locked, layer_pixel_locked, layer_transparency_locked, BlendMode, Channel,
    ColorLabel, Document, Layer, LayerMask, LockFlags, PsdRect,
};

use super::clipboard::{clear_layer, coverage_bounds, plane_coverage_bounds};
use super::merge::{merge_scope, MergeScope};
use super::paths::{format_segments, parse_path, resolve_path, resolve_path_mut};
use super::via::{layer_via_copy, layer_via_cut};
use crate::document_ops::canvas::offset_rect;

/// Lift the pixels covered by `coverage` (document-sized, `0..=255`) off the
/// layer at `source_path` for a transform: copy them into a new layer directly
/// above, sized to the coverage bounds (so memory follows the selection, not
/// the document), and clear them from the source as Edit > Clear does (a
/// Background clears to white). Unlike a move, a Background may be lifted.
/// Returns the new layer's path; empty when [`can_lift_selection`] refuses the
/// source, or the selection covers no visible pixel. Put it back with
/// [`merge_lifted`].
pub fn lift_selection(doc: &mut Document, source_path: &str, coverage: &[u8]) -> String {
    let Some(segments) = parse_path(source_path) else {
        return String::new();
    };
    let Some(source) = resolve_path(doc, source_path).filter(|layer| can_lift_selection(layer))
    else {
        return String::new();
    };
    let Some(floating) = lifted_copy(source, coverage, doc.width, doc.height) else {
        return String::new();
    };
    let Some((&index, parent)) = segments.split_last() else {
        return String::new();
    };
    if doc.layers.insert(parent, index + 1, floating).is_err() {
        return String::new();
    }
    // Clear only after the floating layer is safely in the stack: a refused
    // insert must not erase the source pixels.
    clear_layer(doc, source_path, Some(coverage));
    let mut path = segments;
    if let Some(last) = path.last_mut() {
        *last = index + 1;
    }
    format_segments(&path)
}

/// `source`'s pixels under `coverage`, as a layer at the coverage bounds
/// (clipped to the source) whose alpha is the source alpha scaled by the
/// coverage; `None` when no covered pixel is visible.
fn lifted_copy(source: &Layer, coverage: &[u8], doc_w: u32, doc_h: u32) -> Option<Layer> {
    let bounds = coverage_bounds(coverage, doc_w, doc_h)?;
    let rect = intersect(bounds, source.rect)?;
    let (w, h) = (rect.width() as usize, rect.height() as usize);
    let src = source.rect;
    let src_w = src.width() as usize;
    let src_alpha = source.channels.iter().find(|c| c.id == -1);
    let colour: Vec<&Channel> = source.channels.iter().filter(|c| c.id >= 0).collect();
    let mut planes = vec![vec![0u8; w * h]; colour.len()];
    let mut alpha = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = (rect.left as usize + x, rect.top as usize + y);
            let covered = coverage[dy * doc_w as usize + dx];
            if covered == 0 {
                continue;
            }
            let si = (dy - src.top as usize) * src_w + (dx - src.left as usize);
            let a = src_alpha.map_or(255, |c| c.data[si]);
            let out = ((a as u32 * covered as u32 + 127) / 255) as u8;
            if out == 0 {
                continue;
            }
            let di = y * w + x;
            alpha[di] = out;
            for (plane, channel) in planes.iter_mut().zip(&colour) {
                plane[di] = channel.data[si];
            }
        }
    }
    if alpha.iter().all(|a| *a == 0) {
        return None;
    }
    let mut channels: Vec<Channel> = colour
        .iter()
        .zip(planes)
        .map(|(channel, data)| Channel {
            id: channel.id,
            data: data.into(),
        })
        .collect();
    channels.push(Channel {
        id: -1,
        data: alpha.into(),
    });
    Some(Layer {
        name: format!("{} copy", source.name),
        rect,
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels,
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    })
}

fn intersect(a: PsdRect, b: PsdRect) -> Option<PsdRect> {
    let rect = PsdRect {
        top: a.top.max(b.top),
        left: a.left.max(b.left),
        bottom: a.bottom.min(b.bottom),
        right: a.right.min(b.right),
    };
    (rect.width() > 0 && rect.height() > 0).then_some(rect)
}

/// Whether [`lift_selection`] can take pixels from `layer`: a pixel layer or
/// Background without a pixel lock or a transparency lock on its alpha, and
/// not position-locked unless it is the Background (which always is).
pub fn can_lift_selection(layer: &pictura_core::Layer) -> bool {
    let has_alpha = layer.channels.iter().any(|c| c.id == -1);
    !layer.is_group
        && layer.adjustment.is_none()
        && layer.channels.iter().any(|c| c.id == 0)
        && !layer_pixel_locked(layer)
        && !(has_alpha && layer_transparency_locked(layer))
        && (layer.background || !layer_move_locked(layer))
}

/// Put a [`lift_selection`] layer at `lifted_path` back into its source (the
/// layer directly below) and remove it. The floating pixels are composited
/// over the source's own channels (Normal, full opacity), so the source keeps
/// everything else as it was: name, locks, Background status, opacity, blend,
/// mask, effects. A layer with alpha grows to hold pixels moved past its rect;
/// the alpha-less Background keeps its rect and clips them. False when the
/// path has no layer below it.
pub fn merge_lifted(doc: &mut Document, lifted_path: &str) -> bool {
    let Some(segments) = parse_path(lifted_path) else {
        return false;
    };
    let Some((&index, parent)) = segments.split_last() else {
        return false;
    };
    if index == 0 {
        return false;
    }
    // The floating layer is a temporary base for the run above it. Mark it
    // clipped for the removal so the run stays clipped to the layer below,
    // matching the pre-LayerStack lift/merge behavior.
    if let Some(floating) = resolve_path_mut(doc, lifted_path) {
        floating.clipping = true;
    }
    let Some(floating) = doc.layers.remove(&segments) else {
        return false;
    };
    let mut below = parent.to_vec();
    below.push(index - 1);
    let Some(base) = resolve_path_mut(doc, &format_segments(&below)) else {
        return false;
    };
    composite_over(base, &floating);
    true
}

/// `top`'s raster channels over `base`'s, in place, by straight-alpha "over".
fn composite_over(base: &mut Layer, top: &Layer) {
    let has_alpha = base.channels.iter().any(|c| c.id == -1);
    if has_alpha {
        grow_to(base, union(base.rect, top.rect));
    }
    let (rect, top_rect) = (base.rect, top.rect);
    let Some(area) = intersect(rect, top_rect) else {
        return;
    };
    let (base_w, top_w) = (rect.width() as usize, top_rect.width() as usize);
    let top_alpha = top.channels.iter().find(|c| c.id == -1);
    for y in area.top..area.bottom {
        for x in area.left..area.right {
            let ti = (y - top_rect.top) as usize * top_w + (x - top_rect.left) as usize;
            let fa = top_alpha.map_or(255, |c| c.data[ti]) as u32;
            if fa == 0 {
                continue;
            }
            let bi = (y - rect.top) as usize * base_w + (x - rect.left) as usize;
            let ba = base
                .channels
                .iter()
                .find(|c| c.id == -1)
                .map_or(255, |c| c.data[bi]) as u32;
            // Straight-alpha over, in 0..=255 fixed point.
            let rest = ba * (255 - fa) / 255;
            let out_a = fa + rest;
            for channel in base.channels.iter_mut() {
                if channel.id == -1 {
                    channel.data[bi] = out_a as u8;
                    continue;
                }
                let Some(fc) = top
                    .channels
                    .iter()
                    .find(|c| c.id == channel.id)
                    .map(|c| c.data[ti] as u32)
                else {
                    continue;
                };
                let bc = channel.data[bi] as u32;
                channel.data[bi] = ((fc * fa + bc * rest + out_a / 2) / out_a) as u8;
            }
        }
    }
}

fn union(a: PsdRect, b: PsdRect) -> PsdRect {
    PsdRect {
        top: a.top.min(b.top),
        left: a.left.min(b.left),
        bottom: a.bottom.max(b.bottom),
        right: a.right.max(b.right),
    }
}

/// Extend `layer`'s channels to `rect` (a superset of its rect); the new area
/// is transparent.
fn grow_to(layer: &mut Layer, rect: PsdRect) {
    let old = layer.rect;
    if rect == old || old.width() <= 0 || old.height() <= 0 {
        return;
    }
    let (w, old_w) = (rect.width() as usize, old.width() as usize);
    let (x0, y0) = (
        (old.left - rect.left) as usize,
        (old.top - rect.top) as usize,
    );
    for channel in &mut layer.channels {
        let mut data = vec![0u8; w * rect.height() as usize];
        for y in 0..old.height() as usize {
            let at = (y0 + y) * w + x0;
            data[at..at + old_w].copy_from_slice(&channel.data[y * old_w..(y + 1) * old_w]);
        }
        channel.data = data.into();
    }
    layer.rect = rect;
}

/// Undo [`trim_to_content`]: pad the layer at `path` back out to `rect` with
/// transparent pixels, so a transform box shown and dismissed without a
/// transform leaves the layer's extent (and later filters' room to spread) as
/// it was. The composite is unchanged. False when the layer is gone, is a group,
/// has no alpha, or no longer lies inside `rect`.
pub fn untrim_to(doc: &mut Document, path: &str, rect: PsdRect) -> bool {
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    if layer.is_group
        || layer.rect == rect
        || union(layer.rect, rect) != rect
        || !layer.channels.iter().any(|c| c.id == -1)
    {
        return false;
    }
    grow_to(layer, rect);
    true
}

/// Trim the pixel layer at `path` to the bounds of its non-transparent pixels,
/// so a transform's box hugs the content as Photoshop's does. Only fully
/// transparent pixels are dropped, so the composite is unchanged. False (and
/// nothing changes) for a layer without alpha, with a mask, already tight, or
/// with no visible pixels, and for a type layer: its rect is set by its layout
/// (a paragraph layer's is its box) and its origin is read back against it.
pub fn trim_to_content(doc: &mut Document, path: &str) -> bool {
    let Some(bounds) = resolve_path(doc, path).and_then(trim_bounds) else {
        return false;
    };
    if let Some(layer) = resolve_path_mut(doc, path) {
        trim_layer(layer, bounds);
    }
    true
}

/// The rect a transform box on the layer at `path` hugs: what
/// [`trim_to_content`] would trim it to, else its rect. Changes nothing, so the
/// Move tool's controls box can show it without touching the layer; a shape
/// layer's is its outline box. `None` when the path does not resolve or the
/// layer has no visible pixels (no box, as in CS6).
pub fn content_rect(doc: &Document, path: &str) -> Option<PsdRect> {
    let layer = resolve_path(doc, path)?;
    if super::shape_layer::is_shape_layer(layer) {
        return super::shape_layer::shape_layer_rect(layer, doc.width, doc.height);
    }
    if let Some(bounds) = trim_bounds(layer) {
        return Some(bounds);
    }
    let empty = !layer.is_type()
        && layer
            .channels
            .iter()
            .find(|c| c.id == -1)
            .is_some_and(|alpha| alpha.data.bands().flatten().all(|a| *a == 0));
    (!empty).then_some(layer.rect)
}

/// The bounds [`trim_to_content`] trims `layer` to; `None` when it would not
/// trim (see there).
fn trim_bounds(layer: &Layer) -> Option<PsdRect> {
    if layer.mask.is_some() || layer.is_group || layer.is_type() {
        return None;
    }
    let alpha = layer.channels.iter().find(|c| c.id == -1)?;
    let rect = layer.rect;
    let (w, h) = (rect.width().max(0), rect.height().max(0));
    let local = plane_coverage_bounds(&alpha.data, w as u32, h as u32)?;
    if (local.left, local.top, local.right, local.bottom) == (0, 0, w, h) {
        return None;
    }
    Some(PsdRect {
        top: rect.top + local.top,
        left: rect.left + local.left,
        bottom: rect.top + local.bottom,
        right: rect.left + local.right,
    })
}

/// Crop `layer`'s channels from its rect to `bounds` (clamped inside it).
fn trim_layer(layer: &mut pictura_core::Layer, bounds: PsdRect) {
    let old = layer.rect;
    let new = PsdRect {
        top: bounds.top.max(old.top),
        left: bounds.left.max(old.left),
        bottom: bounds.bottom.min(old.bottom),
        right: bounds.right.min(old.right),
    };
    if new.width() <= 0 || new.height() <= 0 || new == old {
        return;
    }
    let old_w = old.width() as usize;
    let (w, x0) = (new.width() as usize, (new.left - old.left) as usize);
    for channel in &mut layer.channels {
        let mut data = Vec::with_capacity(w * new.height() as usize);
        for y in new.top..new.bottom {
            let row = (y - old.top) as usize * old_w + x0;
            data.extend_from_slice(&channel.data[row..row + w]);
        }
        channel.data = data.into();
    }
    layer.rect = new;
}

/// Move the pixels covered by `mask` from the layer at `source_path` by
/// `(dx, dy)`. With `duplicate` the source stays intact and a new layer sits
/// above it; otherwise the source region is cut and the moved pixels merge back
/// down, so the layer count is unchanged. Returns false on the same refusals as
/// [`layer_via_copy`] / [`layer_via_cut`].
pub fn move_selection_content(
    doc: &mut Document,
    source_path: &str,
    mask: &LayerMask,
    dx: i32,
    dy: i32,
    duplicate: bool,
) -> bool {
    // A locked source cannot contribute pixels: a position lock refuses the
    // move, a pixel lock refuses any mutation, and a transparency lock refuses
    // the cut that clears the source alpha (a duplicate leaves it intact).
    let Some(source) = resolve_path(doc, source_path) else {
        return false;
    };
    if layer_move_locked(source)
        || layer_pixel_locked(source)
        || (!duplicate && layer_transparency_locked(source))
    {
        return false;
    }
    let new_path = if duplicate {
        layer_via_copy(doc, source_path, mask)
    } else {
        layer_via_cut(doc, source_path, mask)
    };
    if new_path.is_empty() {
        return false;
    }
    if let Some(layer) = resolve_path_mut(doc, &new_path) {
        layer.rect = offset_rect(layer.rect, dx, dy);
        if let Some(layer_mask) = &mut layer.mask {
            layer_mask.rect = offset_rect(layer_mask.rect, dx, dy);
        }
    }
    if !duplicate {
        // The cut copy is a `Normal` layer and this merge composites it as one;
        // the original's blend mode reaches the backdrop through the merged
        // node, so giving the cut copy that blend would apply it twice. The
        // merged node also inherits the original's opacity — see #813.
        let _ = merge_scope(doc, MergeScope::Down(&new_path));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{
        BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer, LayerMask, LockFlags,
        PsdRect,
    };

    fn rect(w: i32, h: i32) -> PsdRect {
        PsdRect {
            top: 0,
            left: 0,
            bottom: h,
            right: w,
        }
    }

    fn pixel_layer(name: &str, w: u32, h: u32, value: u8) -> Layer {
        let n = w as usize * h as usize;
        Layer {
            name: name.into(),
            rect: rect(w as i32, h as i32),
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            lock: LockFlags::default(),
            color: ColorLabel::None,
            clipping: false,
            visible: true,
            mask: None,
            adjustment: None,
            channels: vec![
                Channel {
                    id: 0,
                    data: vec![value; n].into(),
                },
                Channel {
                    id: 1,
                    data: vec![value; n].into(),
                },
                Channel {
                    id: 2,
                    data: vec![value; n].into(),
                },
                Channel {
                    id: -1,
                    data: vec![255; n].into(),
                },
            ],
            children: pictura_core::LayerStack::default(),
            is_group: false,
            background: false,
            ..Default::default()
        }
    }

    fn doc_with(layers: Vec<Layer>) -> Document {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(layers);
        doc
    }

    fn selection_mask(data: Vec<u8>) -> LayerMask {
        LayerMask {
            rect: rect(4, 4),
            default_color: 0,
            disabled: false,
            flags: 0,
            data: Some(data.into()),
            ..Default::default()
        }
    }

    fn alpha(layer: &Layer) -> &[u8] {
        &layer.channels.iter().find(|c| c.id == -1).unwrap().data
    }

    fn square_coverage() -> Vec<u8> {
        let mut coverage = vec![0; 16];
        for i in [5, 6, 9, 10] {
            coverage[i] = 255;
        }
        coverage
    }

    fn channel(layer: &Layer, id: i16) -> &[u8] {
        &layer.channels.iter().find(|c| c.id == id).unwrap().data
    }

    #[test]
    fn lift_copies_the_covered_pixels_trimmed_and_clears_them() {
        let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
        let lifted = lift_selection(&mut doc, "0", &square_coverage());
        assert_eq!(lifted, "1");
        let layer = &doc.layers[1];
        assert_eq!(
            layer.rect,
            PsdRect {
                top: 1,
                left: 1,
                bottom: 3,
                right: 3
            },
            "trimmed to the selection"
        );
        assert_eq!(channel(layer, 0), &[40; 4]);
        assert_eq!(alpha(layer), &[255; 4]);
        let source = alpha(&doc.layers[0]);
        assert_eq!((source[5], source[10], source[0]), (0, 0, 255));

        assert!(merge_scope(&mut doc, MergeScope::Down(&lifted)).is_ok());
        assert_eq!(doc.layers.len(), 1);
        assert_eq!(
            alpha(&doc.layers[0])[5],
            255,
            "an untransformed lift merges back"
        );
    }

    #[test]
    fn a_locked_background_can_be_lifted_and_stays_the_background() {
        let mut background = pixel_layer("Background", 4, 4, 40);
        background.channels.retain(|c| c.id != -1);
        background.background = true;
        background.lock = LockFlags::default()
            .with(LockFlags::TRANSPARENCY, true)
            .with(LockFlags::POSITION, true);
        let mut doc = doc_with(vec![background]);
        let lifted = lift_selection(&mut doc, "0", &square_coverage());
        assert_eq!(lifted, "1");
        assert_eq!(
            channel(&doc.layers[0], 0)[5],
            255,
            "the hole clears to white"
        );
        assert_eq!(channel(&doc.layers[0], 0)[0], 40);

        assert!(merge_lifted(&mut doc, &lifted));
        assert_eq!(doc.layers.len(), 1);
        assert!(doc.layers[0].background);
        assert!(doc.layers[0].lock.contains(LockFlags::POSITION));
        assert!(!doc.layers[0].channels.iter().any(|c| c.id == -1));
        assert_eq!(channel(&doc.layers[0], 0)[5], 40);
    }

    #[test]
    fn a_white_selection_on_a_white_background_lifts() {
        let mut background = pixel_layer("Background", 4, 4, 255);
        background.channels.retain(|c| c.id != -1);
        background.background = true;
        let mut doc = doc_with(vec![background]);
        assert_eq!(lift_selection(&mut doc, "0", &square_coverage()), "1");
        assert_eq!(doc.layers.len(), 2);
    }

    #[test]
    fn lift_then_merge_back_restores_the_layer_exactly() {
        let mut layer = pixel_layer("half", 4, 4, 0);
        for channel in layer.channels.iter_mut() {
            let id = channel.id as i32;
            channel.data = (0..16)
                .map(|i| (i * 13 + id * 7) as u8)
                .collect::<Vec<_>>()
                .into();
        }
        layer.opacity = 128;
        layer.blend = BlendMode::Multiply;
        layer.mask = Some(selection_mask(vec![200; 16]));
        let mut doc = doc_with(vec![layer]);
        let before = doc.clone();
        let lifted = lift_selection(&mut doc, "0", &square_coverage());
        assert_eq!(doc.layers[1].rect.width(), 2, "sized to the selection");
        assert!(merge_lifted(&mut doc, &lifted));
        assert_eq!(doc, before, "opacity, blend, mask, and pixels survive");
    }

    #[test]
    fn moved_pixels_grow_a_layer_with_alpha() {
        let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
        let lifted = lift_selection(&mut doc, "0", &square_coverage());
        if let Some(layer) = resolve_path_mut(&mut doc, &lifted) {
            layer.rect = offset_rect(layer.rect, 3, 0);
        }
        assert!(merge_lifted(&mut doc, &lifted));
        let layer = &doc.layers[0];
        assert_eq!(layer.rect.right, 6, "grown to hold the moved pixels");
        let w = layer.rect.width() as usize;
        assert_eq!(alpha(layer)[w + 4], 255, "moved pixel");
        assert_eq!(alpha(layer)[w + 1], 0, "vacated pixel");
        assert_eq!(channel(layer, 0)[w + 4], 40);
    }

    #[test]
    fn lift_refuses_empty_coverage_and_pixel_and_position_locks() {
        let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
        assert!(lift_selection(&mut doc, "0", &[0; 16]).is_empty());
        doc.layers[0].lock = LockFlags::default().with(LockFlags::PIXELS, true);
        assert!(lift_selection(&mut doc, "0", &square_coverage()).is_empty());
        doc.layers[0].lock = LockFlags::default().with(LockFlags::POSITION, true);
        assert!(lift_selection(&mut doc, "0", &square_coverage()).is_empty());
        assert_eq!(doc.layers.len(), 1);
    }

    #[test]
    fn trim_to_content_hugs_the_opaque_pixels() {
        let mut layer = pixel_layer("square", 4, 4, 40);
        let alpha = layer.channels.iter_mut().find(|c| c.id == -1).unwrap();
        alpha.data = square_coverage().into();
        let mut doc = doc_with(vec![layer]);
        assert!(trim_to_content(&mut doc, "0"));
        assert_eq!(
            doc.layers[0].rect,
            PsdRect {
                top: 1,
                left: 1,
                bottom: 3,
                right: 3
            }
        );
        assert_eq!(channel(&doc.layers[0], 0), &[40; 4]);
        assert!(!trim_to_content(&mut doc, "0"), "already tight");

        let mut empty = doc_with(vec![pixel_layer("empty", 4, 4, 0)]);
        empty.layers[0].channels[3].data = vec![0; 16].into();
        assert!(!trim_to_content(&mut empty, "0"));
        assert_eq!(empty.layers[0].rect, rect(4, 4));
        assert_eq!(
            content_rect(&empty, "0"),
            None,
            "an empty layer shows no box"
        );
    }

    #[test]
    fn content_rect_matches_a_per_pixel_scan() {
        let (w, h) = (9usize, 7usize);
        let patterns: [&dyn Fn(usize, usize) -> u8; 4] = [
            &|x, y| u8::from(x == 3 && y == 2),
            &|x, y| u8::from((2..7).contains(&x) && (1..6).contains(&y)) * 9,
            &|x, y| u8::from(x == 0 && y == 6 || x == 8 && y == 0),
            &|x, y| u8::from((x + y) % 5 == 4 && x > 1 && y < 6),
        ];
        for pattern in patterns {
            let mut data = vec![0u8; w * h];
            let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
            for y in 0..h {
                for x in 0..w {
                    data[y * w + x] = pattern(x, y);
                    if data[y * w + x] > 0 {
                        (x0, y0) = (x0.min(x), y0.min(y));
                        (x1, y1) = (x1.max(x + 1), y1.max(y + 1));
                    }
                }
            }
            let mut layer = pixel_layer("pattern", w as u32, h as u32, 40);
            layer.channels.iter_mut().find(|c| c.id == -1).unwrap().data = data.into();
            let found = content_rect(&doc_with(vec![layer]), "0").unwrap();
            let expected = (y0 as i32, x0 as i32, y1 as i32, x1 as i32);
            assert_eq!((found.top, found.left, found.bottom, found.right), expected);
        }
    }

    #[test]
    fn content_rect_does_not_materialize_a_diverged_plane() {
        let (w, h) = (8u32, 200u32);
        let mut layer = pixel_layer("banded", w, h, 40);
        let alpha = layer.channels.iter_mut().find(|c| c.id == -1).unwrap();
        alpha.data = vec![255; (w * h) as usize].into();
        alpha.data.fracture(w as usize);
        let held = alpha.data.clone();
        alpha.data.write_band(0).expect("band")[0] = 0;
        let doc = doc_with(vec![layer]);
        let before = pictura_core::materializations();
        assert!(content_rect(&doc, "0").is_some());
        assert_eq!(pictura_core::materializations(), before);
        drop(held);
    }

    #[test]
    fn untrim_restores_the_extent_without_changing_pixels() {
        let mut layer = pixel_layer("square", 4, 4, 40);
        layer.channels.iter_mut().find(|c| c.id == -1).unwrap().data = square_coverage().into();
        let mut doc = doc_with(vec![layer]);
        let before = doc.layers[0].clone();
        let hugged = content_rect(&doc, "0").unwrap();
        assert_eq!(
            doc.layers[0].rect,
            rect(4, 4),
            "content_rect changes nothing"
        );
        assert!(trim_to_content(&mut doc, "0"));
        assert_eq!(
            doc.layers[0].rect, hugged,
            "the box shown is the box trimmed to"
        );
        assert!(untrim_to(&mut doc, "0", rect(4, 4)));
        assert_eq!(doc.layers[0].rect, rect(4, 4));
        assert_eq!(alpha(&doc.layers[0]), alpha(&before));
        assert_eq!(
            channel(&doc.layers[0], 0)[5],
            40,
            "the content keeps its place"
        );
        assert!(
            !untrim_to(&mut doc, "0", rect(4, 4)),
            "already at the extent"
        );
        assert!(!untrim_to(&mut doc, "9", rect(4, 4)), "no such layer");
    }

    #[test]
    fn trim_leaves_type_layers() {
        let mut layer = pixel_layer("type", 4, 4, 40);
        layer.channels.iter_mut().find(|c| c.id == -1).unwrap().data = square_coverage().into();
        layer.extra_blocks.push(pictura_core::LayerBlock {
            key: *b"TySh",
            data: Vec::new().into(),
        });
        let mut doc = doc_with(vec![layer]);
        assert!(!trim_to_content(&mut doc, "0"));
        assert_eq!(doc.layers[0].rect, rect(4, 4));
    }

    #[test]
    fn move_cuts_translates_and_merges_without_changing_layer_count() {
        let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
        let moved =
            move_selection_content(&mut doc, "0", &selection_mask(vec![255; 16]), 2, 0, false);

        assert!(moved);
        assert_eq!(doc.layers.len(), 1, "the moved copy merges back down");
        let data = alpha(&doc.layers[0]);
        for y in 0..4 {
            for x in 0..4 {
                if x < 2 {
                    assert_eq!(data[y * 4 + x], 0, "source ({x},{y}) cleared");
                } else {
                    assert_eq!(data[y * 4 + x], 255, "dest ({x},{y}) has the pixels");
                }
            }
        }
    }

    #[test]
    fn move_duplicate_keeps_source_and_adds_a_layer() {
        let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
        let moved =
            move_selection_content(&mut doc, "0", &selection_mask(vec![255; 16]), 2, 0, true);

        assert!(moved);
        assert_eq!(doc.layers.len(), 2, "the duplicate is a new layer");
        assert!(
            alpha(&doc.layers[0]).iter().all(|&v| v == 255),
            "source intact"
        );
        let copy = &doc.layers[1];
        assert_eq!(copy.name, "base copy");
        assert_eq!(
            copy.rect,
            PsdRect {
                top: 0,
                left: 2,
                bottom: 4,
                right: 6,
            },
            "the copy rect translated by dx"
        );
        assert!(
            alpha(copy).iter().all(|&v| v == 255),
            "copy carries the pixels"
        );
    }

    #[test]
    fn empty_mask_refuses_and_zero_coverage_cuts_nothing() {
        let no_data = LayerMask {
            rect: rect(4, 4),
            default_color: 0,
            disabled: false,
            flags: 0,
            data: None,
            ..Default::default()
        };
        let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
        let before = doc.clone();
        assert!(!move_selection_content(
            &mut doc, "0", &no_data, 2, 0, false
        ));
        assert_eq!(doc, before, "no mask data refuses without mutation");

        // `layer_via_cut` only refuses a missing data plane, so a present but
        // all-zero mask still cuts nothing: the call reports success and the
        // merge-down leaves the document byte-identical.
        let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
        let before = doc.clone();
        assert!(move_selection_content(
            &mut doc,
            "0",
            &selection_mask(vec![0; 16]),
            2,
            0,
            false
        ));
        assert_eq!(doc, before, "zero coverage moves nothing");
    }

    #[test]
    fn locked_source_refuses_content_move() {
        let mask = selection_mask(vec![255; 16]);

        for flag in [LockFlags::PIXELS, LockFlags::POSITION] {
            let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
            doc.layers[0].lock = LockFlags::default().with(flag, true);
            let before = doc.clone();
            assert!(
                !move_selection_content(&mut doc, "0", &mask, 2, 0, false),
                "flag {flag} refuses the move"
            );
            assert_eq!(doc, before, "refusal leaves the document unchanged");
        }

        let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
        doc.layers[0].lock = LockFlags::default().with(LockFlags::TRANSPARENCY, true);
        let before = doc.clone();
        assert!(
            !move_selection_content(&mut doc, "0", &mask, 2, 0, false),
            "a transparency lock refuses the cut"
        );
        assert_eq!(doc, before);
        assert!(
            move_selection_content(&mut doc, "0", &mask, 2, 0, true),
            "a duplicate leaves the source alpha intact"
        );
    }
}
