//! The Move tool's view preview (#1148). A layer too large to bake whole is
//! previewed from what the canvas shows: the backdrop under the layer's
//! on-screen part and the moving pixels around the view, both at the level the
//! canvas presents. The work follows the view, not the document.

use super::super::super::helpers::layer_has_effects;
use super::super::super::helpers_composite::{buffer_to_image, current_buffer, layer_image};
use cxx_qt_lib::QImage;
use pictura_core::{Document, LayerStack, PsdRect};
use pictura_render::FilterProxy;

/// Content above this many pixels is previewed from the view. Below it the
/// whole-content patch stays cheap and keeps full resolution.
pub(in crate::cxxqt_object) const VIEW_PREVIEW_PIXELS: i64 = 4_000_000;

/// The deepest level a view preview is built at; deeper levels do not exist.
const MAX_LEVEL: u32 = 16;

const EMPTY: PsdRect = PsdRect {
    top: 0,
    left: 0,
    bottom: 0,
    right: 0,
};

/// What a view preview was built for and what it covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::cxxqt_object) struct MoveView {
    pub(in crate::cxxqt_object) level: u32,
    /// The document rect the canvas presents, clamped to the canvas.
    pub(in crate::cxxqt_object) visible: PsdRect,
    /// The document rect the moving image covers; empty when none of the
    /// layer is near the view.
    pub(in crate::cxxqt_object) region: PsdRect,
    /// The moving layer's content bounds, which the marching ants box.
    pub(in crate::cxxqt_object) content: PsdRect,
}

impl MoveView {
    /// Document pixels per preview pixel.
    pub(in crate::cxxqt_object) fn scale(&self) -> i32 {
        1 << self.level
    }
}

pub(super) struct ViewPreview {
    /// The backdrop under the layer's on-screen part; null when none of the
    /// layer is on screen.
    pub(super) base: QImage,
    pub(super) base_origin: (i32, i32),
    /// The layer over the view's region; null when the region is empty.
    pub(super) layer: QImage,
    pub(super) view: MoveView,
}

/// The view preview of `paths` with the canvas presenting `level` over
/// `visible`, or `None` when the whole-content preview should run: more than
/// one layer, a layer that is not plain pixels or has content under
/// [`VIEW_PREVIEW_PIXELS`], no presented view, or (above level 0) a document a
/// reduced copy would not draw faithfully.
pub(super) fn view_preview(
    doc: &Document,
    paths: &[String],
    level: i32,
    visible: PsdRect,
    gpu_compute: bool,
) -> Option<ViewPreview> {
    let [path] = paths else {
        return None;
    };
    let layer = pictura_render::resolve_path(doc, path)?;
    if pictura_render::is_shape_layer(layer)
        || layer_has_effects(layer)
        || layer.is_group
        || layer.adjustment.is_some()
        || layer.is_type()
        || layer.smart_object.is_some()
    {
        return None;
    }
    let content = pictura_render::content_rect(doc, path)?;
    if area(content) <= VIEW_PREVIEW_PIXELS {
        return None;
    }
    let level = u32::try_from(level)
        .ok()
        .filter(|&level| level <= MAX_LEVEL)?;
    let visible = visible.intersect(canvas(doc))?;
    if level > 0 && !FilterProxy::reduces_faithfully(doc) {
        return None;
    }
    let scale = 1i32 << level;
    let (base, base_origin) = match content.intersect(visible) {
        Some(patch) => {
            let patch = align(patch, scale).intersect(canvas(doc))?;
            let base = backdrop(doc, path, level, patch, gpu_compute)?;
            (base, (patch.left, patch.top))
        }
        None => (QImage::default(), (0, 0)),
    };
    // A layer far off screen starts with no moving image; [`follow`] builds
    // one when a drag brings it into view.
    let (image, region) = match moving_region(layer.rect, visible, scale) {
        Some(region) => (moving_image(doc, path, level, region)?, region),
        None => (QImage::default(), EMPTY),
    };
    Some(ViewPreview {
        base,
        base_origin,
        layer: image,
        view: MoveView {
            level,
            visible,
            region,
            content,
        },
    })
}

/// The moving image again for a drag that has carried the view `(dx, dy)`
/// across the layer, when the part of the layer now on screen leaves the
/// region it covers; `None` while the current image still covers it.
pub(super) fn follow(
    doc: &Document,
    path: &str,
    view: MoveView,
    dx: i32,
    dy: i32,
) -> Option<(QImage, MoveView)> {
    let layer = pictura_render::resolve_path(doc, path)?;
    let shown = offset(view.visible, -dx, -dy);
    let needed = shown.intersect(layer.rect)?;
    if contains(view.region, needed) {
        return None;
    }
    let region = moving_region(layer.rect, shown, view.scale())?;
    let image = moving_image(doc, path, view.level, region)?;
    Some((image, MoveView { region, ..view }))
}

/// The document without the layer at `path`, composited over `patch` at
/// `level`. Level 0 composites the patch exactly; a deeper level composites a
/// reduced copy of it.
fn backdrop(
    doc: &Document,
    path: &str,
    level: u32,
    patch: PsdRect,
    gpu_compute: bool,
) -> Option<QImage> {
    let mut hidden = doc.clone();
    let target = pictura_render::resolve_path_mut(&mut hidden, path)?;
    target.visible = false;
    // A hidden layer draws nothing; without its planes the reduction skips it.
    target.channels.clear();
    let (buffer, source) = if level == 0 {
        let (buffer, _) = pictura_render::composite_region_active(&hidden, patch, gpu_compute);
        (buffer, hidden)
    } else {
        let reduced =
            FilterProxy::new(&hidden, None, patch, 1.0 / f64::from(1u32 << level))?.into_document();
        (current_buffer(&reduced, gpu_compute), reduced)
    };
    Some(buffer_to_image(&pictura_codec::buffer_to_srgb(
        &source, &buffer,
    )))
}

/// The layer at `path` over `region` at `level`, with no opacity applied (the
/// preview applies it once). The region may reach past the canvas: a drag
/// carries off-canvas pixels into view.
fn moving_image(doc: &Document, path: &str, level: u32, region: PsdRect) -> Option<QImage> {
    let mut layer = pictura_render::resolve_path(doc, path)?.clone();
    layer.rect = offset(layer.rect, -region.left, -region.top);
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
    let reduced =
        FilterProxy::new(&single, None, whole, 1.0 / f64::from(1u32 << level))?.into_document();
    layer_image(reduced.layers.iter().next()?)
}

/// The part of `layer_rect` within a quarter view of `visible`, on the level
/// grid. The margin lets a drag run a quarter view before [`follow`] rebuilds.
fn moving_region(layer_rect: PsdRect, visible: PsdRect, scale: i32) -> Option<PsdRect> {
    let (mx, my) = (visible.width() / 4, visible.height() / 4);
    let grown = PsdRect {
        top: visible.top.saturating_sub(my),
        left: visible.left.saturating_sub(mx),
        bottom: visible.bottom.saturating_add(my),
        right: visible.right.saturating_add(mx),
    };
    // Aligned inside the layer, so each preview pixel is a whole footprint.
    let inner = PsdRect {
        top: ceil_to(layer_rect.top, scale),
        left: ceil_to(layer_rect.left, scale),
        bottom: floor_to(layer_rect.bottom, scale),
        right: floor_to(layer_rect.right, scale),
    };
    let inner = if inner.width() > 0 && inner.height() > 0 {
        inner
    } else {
        layer_rect
    };
    align(grown.intersect(layer_rect)?, scale).intersect(inner)
}

fn align(rect: PsdRect, scale: i32) -> PsdRect {
    PsdRect {
        top: floor_to(rect.top, scale),
        left: floor_to(rect.left, scale),
        bottom: ceil_to(rect.bottom, scale),
        right: ceil_to(rect.right, scale),
    }
}

fn floor_to(v: i32, scale: i32) -> i32 {
    v.div_euclid(scale) * scale
}

fn ceil_to(v: i32, scale: i32) -> i32 {
    -(-v).div_euclid(scale) * scale
}

fn canvas(doc: &Document) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: i32::try_from(doc.height).unwrap_or(i32::MAX),
        right: i32::try_from(doc.width).unwrap_or(i32::MAX),
    }
}

fn area(rect: PsdRect) -> i64 {
    i64::from(rect.width().max(0)) * i64::from(rect.height().max(0))
}

fn offset(rect: PsdRect, dx: i32, dy: i32) -> PsdRect {
    PsdRect {
        top: rect.top + dy,
        left: rect.left + dx,
        bottom: rect.bottom + dy,
        right: rect.right + dx,
    }
}

fn contains(outer: PsdRect, inner: PsdRect) -> bool {
    inner.left >= outer.left
        && inner.top >= outer.top
        && inner.right <= outer.right
        && inner.bottom <= outer.bottom
}

#[cfg(test)]
#[path = "view_preview_tests.rs"]
mod tests;
