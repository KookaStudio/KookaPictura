//! The images a transform previews: the moving layer and its backdrop.

use super::*;

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
}

/// Build the transform overlay images for `path`: the backdrop with the target
/// hidden, the target's own RGBA image (materializing a channel-less embedded
/// object into a clone), its document-space origin, and opacity. `bounded` says
/// the canvas shows the document as it stands, so a patch of the target's bounds
/// can stand in for the whole backdrop.
pub(super) fn build_transform_preview(
    doc: &pictura_core::Document,
    path: &str,
    gpu_compute: bool,
    bounded: bool,
) -> Option<TransformPreview> {
    let opacity = pictura_render::resolve_path(doc, path)?.opacity as i32;
    let (image, x, y) = transform_preview_layer(doc, path, gpu_compute)?;
    let group = clipping_group(doc, path);
    let patch = bounded.then(|| backdrop_patch(doc, &group, gpu_compute));
    if let Some((base, origin)) = patch.flatten() {
        return Some(TransformPreview {
            base,
            base_origin: Some(origin),
            image,
            x,
            y,
            opacity,
        });
    }
    let rendered = current_buffer(&hidden(doc, &group), gpu_compute);
    Some(TransformPreview {
        base: buffer_to_image(&pictura_codec::buffer_to_srgb(doc, &rendered)),
        base_origin: None,
        image,
        x,
        y,
        opacity,
    })
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
