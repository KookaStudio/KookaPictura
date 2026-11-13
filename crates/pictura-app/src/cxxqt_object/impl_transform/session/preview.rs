//! The images a transform previews: the moving layer and its backdrop.

use super::view_preview::{self, align, inner_aligned, MAX_LEVEL};
use super::*;
use pictura_core::PixelBuffer;
use pictura_render::FilterProxy;
use std::cell::Cell;

/// A plain layer whose box is over this many pixels previews its transform at
/// a reduced level (#1297): 64 MB of RGBA a preview image at most.
const TRANSFORM_PREVIEW_PIXELS: u64 = 16_000_000;

/// What building a transform preview holds at its peak, a preview pixel: the
/// moving image and its sRGB copy, and the backdrop patch's composite, sRGB
/// copy, and image (15 measured, 19 at the peak, #1297).
const PREVIEW_BYTES_PER_PIXEL: u64 = 20;

/// A whole-document backdrop, a canvas pixel: its composite, sRGB copy, and
/// image.
const WHOLE_BACKDROP_BYTES_PER_PIXEL: u64 = 12;

std::thread_local! {
    /// [`TRANSFORM_PREVIEW_PIXELS`] as a test pins it.
    static PREVIEW_PIXELS_FOR_TEST: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Pin this thread's transform preview threshold to `pixels`; a negative value
/// restores [`TRANSFORM_PREVIEW_PIXELS`].
pub(in crate::cxxqt_object) fn set_transform_preview_pixels_for_test(pixels: i64) {
    PREVIEW_PIXELS_FOR_TEST.set(u64::try_from(pixels).ok());
}

/// The level a transform preview of `path` is built at (#1297): 0 (full
/// resolution) unless the target is one plain pixel layer, not lifted, whose
/// box is over the threshold; then the shallowest level that brings it under.
/// "Plain" is the Move view preview's condition: the channels are the whole
/// appearance and the document reduces faithfully.
pub(super) fn transform_preview_level(
    doc: &pictura_core::Document,
    path: &str,
    members: &[String],
    bounded: bool,
) -> u32 {
    if members.len() > 1 || !bounded {
        return 0;
    }
    let Some(layer) = pictura_render::resolve_path(doc, path) else {
        return 0;
    };
    if needs_rendered_appearance(layer)
        || layer.smart_object.is_some()
        || !layer.channels.iter().any(|c| c.id == 0)
        || layer_has_effects(layer)
        || layer.is_group
        || clipping_group(doc, path).len() > 1
        || layer.rect.intersect(view_preview::canvas(doc)).is_none()
        || !FilterProxy::reduces_faithfully(doc)
    {
        return 0;
    }
    let area = view_preview::area(layer.rect).max(0) as u64;
    let threshold = PREVIEW_PIXELS_FOR_TEST
        .get()
        .unwrap_or(TRANSFORM_PREVIEW_PIXELS)
        .max(1);
    let mut level = 0;
    while level < MAX_LEVEL && area >> (2 * level) > threshold {
        level += 1;
    }
    level
}

/// The bytes building the transform preview of `path` (or the linked
/// `members`) over `orig_rect` holds at its peak, for the begin gate (#1297):
/// the box at its preview level, plus the whole document when the backdrop or
/// a linked set's image is rendered whole.
pub(super) fn transform_preview_bytes(
    doc: &pictura_core::Document,
    path: &str,
    members: &[String],
    orig_rect: PsdRect,
    bounded: bool,
) -> u64 {
    let level = transform_preview_level(doc, path, members, bounded);
    let preview = memory_gate::area(orig_rect) >> (2 * level);
    let linked = members.len() > 1;
    let hide = if linked {
        members.to_vec()
    } else {
        clipping_group(doc, path)
    };
    // A reduced preview's patch is its layer's box; only a full one reads the
    // content bounds.
    let whole = linked || !bounded || (level == 0 && backdrop_patch_rect(doc, &hide).is_none());
    let canvas = if whole {
        u64::from(doc.width) * u64::from(doc.height)
    } else {
        0
    };
    PREVIEW_BYTES_PER_PIXEL * preview + WHOLE_BACKDROP_BYTES_PER_PIXEL * canvas
}

/// The transform preview of the plain layer at `path` at `level` (#1297): the
/// moving image over the layer's rect aligned inside it to the level grid, and
/// the backdrop patch at the same level.
///
/// ponytail: built once at begin, so a view zoomed past the level shows the
/// coarser image; up to `2^level − 1` edge pixels of the layer are left out.
fn reduced_transform_preview(
    doc: &pictura_core::Document,
    path: &str,
    level: u32,
    gpu_compute: bool,
    opacity: i32,
) -> Option<TransformPreview> {
    let layer = pictura_render::resolve_path(doc, path)?;
    let scale = 1i32 << level;
    let region = inner_aligned(layer.rect, scale);
    let image = moving_layer_image(doc, path, region, level, gpu_compute)?;
    // The session trimmed the layer to its content, so its rect is the patch
    // `backdrop_patch_rect` would scan the planes for.
    let patch = align(layer.rect, scale).intersect(view_preview::canvas(doc))?;
    let base = view_preview::backdrop(doc, path, level, patch, gpu_compute)?;
    Some(TransformPreview {
        base,
        base_origin: Some((patch.left, patch.top)),
        image,
        x: region.left,
        y: region.top,
        opacity,
        level,
    })
}

/// The transform preview's moving image of `path` at `level`, and its
/// document-space origin: [`transform_preview_layer`] at level 0.
pub(in crate::cxxqt_object) fn transform_preview_layer_at(
    doc: &pictura_core::Document,
    path: &str,
    level: u32,
    gpu_compute: bool,
) -> Option<(QImage, i32, i32)> {
    if level == 0 {
        return transform_preview_layer(doc, path, gpu_compute);
    }
    let layer = pictura_render::resolve_path(doc, path)?;
    let region = inner_aligned(layer.rect, 1i32 << level);
    let image = moving_layer_image(doc, path, region, level, gpu_compute)?;
    Some((image, region.left, region.top))
}

/// Whether the layer's channels alone are not its whole appearance: a layer
/// mask, vector mask, layer style, fill adjustment, or type needs the
/// compositor to render the preview.
pub(super) fn needs_rendered_appearance(layer: &pictura_core::Layer) -> bool {
    layer.mask.is_some()
        || layer.vector_mask.is_some()
        || layer.adjustment.is_some()
        || layer.is_type()
        || pictura_render::is_shape_layer(layer)
        || pictura_render::decode_layer_fill(layer).is_some()
        || pictura_render::has_layer_style(layer)
}

/// The Move preview's moving image: the layer at `path` over `region`
/// (document coordinates, which may reach past the canvas), reduced by `level`.
/// A layer whose channels alone are not its appearance (a mask, vector mask,
/// style, fill, or type) is composited over transparency, with its opacity and
/// blend neutralized so the preview applies the layer opacity once itself.
/// Otherwise its raw channels are read, which is cheaper.
///
/// ponytail: an unlinked mask is previewed as moving with the layer, as the
/// transform preview does; the commit leaves the mask in place.
pub(super) fn moving_layer_image(
    doc: &pictura_core::Document,
    path: &str,
    region: PsdRect,
    level: u32,
    gpu_compute: bool,
) -> Option<QImage> {
    moving_layer_pixels(doc, path, region, level, gpu_compute)
        .map(|pixels| buffer_to_image(&pixels))
}

/// [`moving_layer_image`]'s straight sRGB pixels, which a lazy Move commit
/// also paints into the stale display tiles.
pub(super) fn moving_layer_pixels(
    doc: &pictura_core::Document,
    path: &str,
    region: PsdRect,
    level: u32,
    gpu_compute: bool,
) -> Option<PixelBuffer> {
    if region.width() <= 0 || region.height() <= 0 {
        return None;
    }
    let mut layer = pictura_render::resolve_path(doc, path)?.clone();
    let rendered = needs_rendered_appearance(&layer);
    if rendered {
        layer.opacity = 255;
        layer.blend = BlendMode::Normal;
    }
    shift_into_region(&mut layer, -region.left, -region.top);
    layer.visible = true;
    let mut single = doc.clone();
    single.width = u32::try_from(region.width()).ok()?;
    single.height = u32::try_from(region.height()).ok()?;
    single.layers = LayerStack::from_layers(vec![layer]);
    let whole = PsdRect {
        top: 0,
        left: 0,
        bottom: region.height(),
        right: region.width(),
    };
    if rendered {
        // A shape's fill and vector mask reduce with the document, but its
        // layer-effect sizes do not; scale them into the proxy's pixels so a
        // stroke keeps its document width. The descriptor rounds integer pixel
        // fields, so the preview is approximate; the commit stays exact.
        if level > 0 && pictura_render::is_shape_layer(&single.layers[0]) {
            pictura_render::scale_layer_effects(
                &mut single,
                &["0"],
                100.0 / f64::from(1u32 << level),
            );
        }
        // Level 0 composites the region-sized scratch document directly, so a
        // channel-less type or fill layer keeps its rect; a deeper level needs
        // the reduced copy.
        let reduced;
        let source = if level == 0 {
            &single
        } else {
            reduced = FilterProxy::new(&single, None, whole, 1.0 / f64::from(1u32 << level))?
                .into_document();
            &reduced
        };
        let (buffer, _) =
            pictura_render::composite(pictura_render::CompositeRequest::whole(source, gpu_compute));
        Some(pictura_codec::buffer_to_srgb(source, &buffer).into_owned())
    } else {
        let reduced =
            FilterProxy::new(&single, None, whole, 1.0 / f64::from(1u32 << level))?.into_document();
        layer_buffer(reduced.layers.iter().next()?)
    }
}

/// Shift everything the compositor reads in document coordinates by
/// `(dx, dy)`, so a layer moved into a region-sized scratch document keeps its
/// mask, vector mask, retained store, and type in place.
fn shift_into_region(layer: &mut pictura_core::Layer, dx: i32, dy: i32) {
    let shift = |rect: PsdRect| PsdRect {
        top: rect.top + dy,
        left: rect.left + dx,
        bottom: rect.bottom + dy,
        right: rect.right + dx,
    };
    layer.rect = shift(layer.rect);
    if let Some(store) = &mut layer.source_channels {
        store.rect = shift(store.rect);
    }
    if let Some(mask) = &mut layer.mask {
        mask.rect = shift(mask.rect);
    }
    if let Some(vector) = &mut layer.vector_mask {
        for point in vector.subpaths.iter_mut().flat_map(|s| &mut s.points) {
            point[0] = point[0].saturating_add(dx.saturating_mul(256));
            point[1] = point[1].saturating_add(dy.saturating_mul(256));
        }
        vector.rows = Default::default();
    }
    if let Some(tool) = layer.type_tool.as_mut() {
        if let Some(path) = tool.path.as_mut() {
            path.subpath.translate(f64::from(dx), f64::from(dy));
        }
        tool.transform[4] += f64::from(dx);
        tool.transform[5] += f64::from(dy);
    }
}

/// Render the layer alone over transparency and crop to its visible rect, so
/// the transform preview shows a mask, style, vector mask, fill, or type that
/// its raw channels do not carry. The layer's opacity and blend are
/// neutralized: the preview applies the layer opacity once itself.
pub(super) fn rendered_layer_image(
    doc: &pictura_core::Document,
    path: &str,
    gpu_compute: bool,
) -> Option<(QImage, i32, i32)> {
    let needs_materialize = {
        let layer = pictura_render::resolve_path(doc, path)?;
        !layer.channels.iter().any(|c| c.id == 0) && layer.smart_object.is_some()
    };
    let mut materialized = None;
    if needs_materialize {
        let mut clone = doc.clone();
        if !pictura_render::rasterize_smart_object(&mut clone, path) {
            return None;
        }
        materialized = Some(clone);
    }
    let source_doc = materialized.as_ref().unwrap_or(doc);
    let layer = pictura_render::resolve_path(source_doc, path)?;
    // A shape layer's rect is the whole canvas; it draws only within its
    // paint bounds (the outline box and its effects), so render just those.
    let rect = if pictura_render::is_shape_layer(layer) {
        pictura_render::layer_paint_bounds(source_doc, path)
            .and_then(|bounds| bounds.intersect(layer.rect))
            .unwrap_or(layer.rect)
    } else {
        layer.rect
    };
    let (x0, y0) = (rect.left.max(0), rect.top.max(0));
    let (x1, y1) = (
        rect.right.min(source_doc.width as i32),
        rect.bottom.min(source_doc.height as i32),
    );
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let mut source = layer.clone();
    source.opacity = 255;
    source.blend = BlendMode::Normal;
    // A clone shares the pixel planes; replacing the layer list keeps the
    // document's ICC profile, pattern library, and type stylesheet, which the
    // compositor needs to reproduce the layer's appearance.
    let mut scratch = source_doc.clone();
    scratch.layers = LayerStack::from_layers(vec![source]);
    let (rendered, _) = pictura_render::composite(pictura_render::CompositeRequest::region(
        &scratch,
        rect,
        gpu_compute,
    ));
    let image = buffer_to_image(&pictura_codec::buffer_to_srgb(&scratch, &rendered));
    Some((image, x0, y0))
}

/// The target path and the contiguous clipped layers directly above it in the
/// same parent. Hiding only the base would drop a clipped fill from the
/// preview, so the moving image carries the whole clipping group.
pub(super) fn clipping_group(doc: &pictura_core::Document, path: &str) -> Vec<String> {
    let mut group = vec![path.to_string()];
    let Some((parent, index)) = (match path.rsplit_once('/') {
        Some((parent, last)) => last
            .parse::<usize>()
            .ok()
            .map(|index| (Some(parent), index)),
        None => path.parse::<usize>().ok().map(|index| (None, index)),
    }) else {
        return group;
    };
    let siblings: Option<&[pictura_core::Layer]> = match parent {
        Some(parent) => {
            pictura_render::resolve_path(doc, parent).map(|layer| layer.children.as_slice())
        }
        None => Some(doc.layers.as_slice()),
    };
    let Some(siblings) = siblings else {
        return group;
    };
    let mut i = index + 1;
    while siblings
        .get(i)
        .is_some_and(|layer: &pictura_core::Layer| layer.clipping)
    {
        group.push(match parent {
            Some(parent) => format!("{parent}/{i}"),
            None => i.to_string(),
        });
        i += 1;
    }
    group
}

/// Render a clipping group (the target plus the clipped layers above it) alone
/// over transparency, cropped to the group's union rect. The target's opacity
/// and blend are neutralized; the clipped layers keep theirs.
///
/// ponytail: the preview moves the clipped layers with the base; the commit
/// transforms only the base, so the clipped pixels' final position can differ.
pub(super) fn rendered_clipping_group(
    doc: &pictura_core::Document,
    group: &[String],
    gpu_compute: bool,
) -> Option<(QImage, i32, i32)> {
    let mut layers = Vec::with_capacity(group.len());
    let mut bounds: Option<pictura_core::PsdRect> = None;
    for (i, path) in group.iter().enumerate() {
        let mut layer = pictura_render::resolve_path(doc, path)?.clone();
        if i == 0 {
            layer.opacity = 255;
            layer.blend = BlendMode::Normal;
        }
        bounds = Some(bounds.map_or(layer.rect, |rect| rect.union(layer.rect)));
        layers.push(layer);
    }
    let bounds = bounds?;
    let (x0, y0) = (bounds.left.max(0), bounds.top.max(0));
    let (x1, y1) = (
        bounds.right.min(doc.width as i32),
        bounds.bottom.min(doc.height as i32),
    );
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let mut scratch = doc.clone();
    scratch.layers = LayerStack::from_layers(layers);
    let rendered = current_buffer(&scratch, gpu_compute);
    let image = buffer_to_image(&pictura_codec::buffer_to_srgb(&scratch, &rendered));
    Some((image.copy(&QRect::new(x0, y0, x1 - x0, y1 - y0)), x0, y0))
}

/// The bounds of the transform targets at `paths`, when nothing they draw lands
/// outside them: the bound the commit's region refresh relies on too.
fn backdrop_patch_rect(
    doc: &pictura_core::Document,
    paths: &[String],
) -> Option<pictura_core::PsdRect> {
    let mut bounds: Option<pictura_core::PsdRect> = None;
    for path in paths {
        let layer = pictura_render::resolve_path(doc, path)?;
        // A shape layer reaches its outline box, its effects, and its clipped
        // run; hiding it changes nothing past that reach.
        let shape = pictura_render::is_shape_layer(layer) && layer.channels.is_empty();
        if !shape && (layer_has_effects(layer) || layer.adjustment.is_some() || layer.is_group) {
            return None;
        }
        // A painted layer's rect can be the whole canvas around a small mark.
        let drawn = if shape {
            pictura_render::damage::content_influence(doc, path)?
        } else {
            pictura_render::content_rect(doc, path).unwrap_or(layer.rect)
        };
        bounds = Some(bounds.map_or(drawn, |b| b.union(drawn)));
    }
    bounds.filter(|&rect| clamp_region(rect, doc.width, doc.height).is_some())
}

/// The transform overlay's images: see [`build_transform_preview`].
pub(super) struct TransformPreview {
    /// The document composited with the target hidden: the whole document, or
    /// the patch at `base_origin` that the live canvas lies under.
    pub(super) base: QImage,
    pub(super) base_origin: Option<(i32, i32)>,
    pub(super) image: QImage,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) opacity: i32,
    /// The images' level: `2^level` document pixels a preview pixel.
    pub(super) level: u32,
}

/// Build the transform overlay images for `path` (or, for a linked set, every
/// member in `members`): the backdrop with the targets hidden, the moving RGBA
/// image (materializing a channel-less embedded object into a clone), its
/// document-space origin, and opacity. `bounded` says the canvas shows the
/// document as it stands, so a patch of the targets' bounds can stand in for
/// the whole backdrop.
pub(super) fn build_transform_preview(
    doc: &pictura_core::Document,
    path: &str,
    members: &[String],
    orig_rect: PsdRect,
    gpu_compute: bool,
    bounded: bool,
) -> Option<TransformPreview> {
    let linked = members.len() > 1;
    let opacity = if linked {
        255
    } else {
        pictura_render::resolve_path(doc, path)?.opacity as i32
    };
    let level = transform_preview_level(doc, path, members, bounded);
    if level > 0 {
        return reduced_transform_preview(doc, path, level, gpu_compute, opacity);
    }
    let (image, x, y) = if linked {
        rendered_transform_group(doc, members, orig_rect, gpu_compute)?
    } else {
        transform_preview_layer(doc, path, gpu_compute)?
    };
    let hide = if linked {
        members.to_vec()
    } else {
        clipping_group(doc, path)
    };
    let patch = bounded.then(|| backdrop_patch(doc, &hide, gpu_compute));
    if let Some((base, origin)) = patch.flatten() {
        return Some(TransformPreview {
            base,
            base_origin: Some(origin),
            image,
            x,
            y,
            opacity,
            level: 0,
        });
    }
    let rendered = current_buffer(&hidden(doc, &hide), gpu_compute);
    Some(TransformPreview {
        base: buffer_to_image(&pictura_codec::buffer_to_srgb(doc, &rendered)),
        base_origin: None,
        image,
        x,
        y,
        opacity,
        level: 0,
    })
}

/// Render a linked transform group over transparency, cropped to `rect` (the
/// members' union), for the transform preview. Hidden members transform with
/// the set but are not drawn, so they are skipped here. Each member keeps its
/// own opacity and blend; the preview applies no extra opacity.
///
/// ponytail: a member whose blend mode reads the layers below previews as if
/// over transparency, and a member's clipping run is not carried; the commit
/// transforms every member's own pixels correctly.
pub(in crate::cxxqt_object) fn rendered_transform_group(
    doc: &pictura_core::Document,
    members: &[String],
    rect: PsdRect,
    gpu_compute: bool,
) -> Option<(QImage, i32, i32)> {
    let (x0, y0) = (rect.left.max(0), rect.top.max(0));
    let (x1, y1) = (
        rect.right.min(doc.width as i32),
        rect.bottom.min(doc.height as i32),
    );
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let mut layers = Vec::with_capacity(members.len());
    // The compositor wants bottom-first; the panel list is topmost-first.
    for path in members.iter().rev() {
        let layer = pictura_render::resolve_path(doc, path)?;
        if layer.visible {
            layers.push(layer.clone());
        }
    }
    if layers.is_empty() {
        return Some((QImage::default(), x0, y0));
    }
    let mut scratch = doc.clone();
    scratch.layers = LayerStack::from_layers(layers);
    let rendered = current_buffer(&scratch, gpu_compute);
    let image = buffer_to_image(&pictura_codec::buffer_to_srgb(&scratch, &rendered));
    Some((image.copy(&QRect::new(x0, y0, x1 - x0, y1 - y0)), x0, y0))
}

/// `doc` with the layers at `paths` hidden.
fn hidden(doc: &pictura_core::Document, paths: &[String]) -> pictura_core::Document {
    let mut out = doc.clone();
    for path in paths {
        if let Some(target) = pictura_render::resolve_path_mut(&mut out, path) {
            target.visible = false;
        }
    }
    out
}

/// The backdrop under the layers at `paths` as a patch of their bounds and its
/// document-space origin, for a preview drawn over the live canvas; `None` when
/// the canvas cannot lie under it, so the whole backdrop has to be built.
pub(super) fn backdrop_patch(
    doc: &pictura_core::Document,
    paths: &[String],
    gpu_compute: bool,
) -> Option<(QImage, (i32, i32))> {
    let rect = backdrop_patch_rect(doc, paths)?;
    let (rendered, _) = pictura_render::composite(pictura_render::CompositeRequest::region(
        &hidden(doc, paths),
        rect,
        gpu_compute,
    ));
    Some((
        buffer_to_image(&pictura_codec::buffer_to_srgb(doc, &rendered)),
        (rect.left.max(0), rect.top.max(0)),
    ))
}

/// The moving pixels of [`build_transform_preview`] and their document-space
/// origin: the layer (or its clipping group, or its rendered appearance).
pub(in crate::cxxqt_object) fn transform_preview_layer(
    doc: &pictura_core::Document,
    path: &str,
    gpu_compute: bool,
) -> Option<(QImage, i32, i32)> {
    let layer = pictura_render::resolve_path(doc, path)?;
    let group = clipping_group(doc, path);
    let image = if group.len() > 1 {
        rendered_clipping_group(doc, &group, gpu_compute)?
    } else if needs_rendered_appearance(layer) {
        rendered_layer_image(doc, path, gpu_compute)?
    } else {
        match layer_image(layer) {
            Some(image) => (image, layer.rect.left, layer.rect.top),
            None => {
                let mut clone = doc.clone();
                if !pictura_render::rasterize_smart_object(&mut clone, path) {
                    return None;
                }
                let rendered = pictura_render::resolve_path(&clone, path)?;
                (
                    layer_image(rendered)?,
                    rendered.rect.left,
                    rendered.rect.top,
                )
            }
        }
    };
    Some(image)
}
