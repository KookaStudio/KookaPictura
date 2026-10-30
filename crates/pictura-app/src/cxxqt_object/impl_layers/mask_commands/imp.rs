use super::*;

use super::super::super::helpers_composite::rgba_image;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QString};
use pictura_core::{Document, LayerMask};

/// The mask's coverage at document pixel `(x, y)` as the mask edit document
/// sees it: its data inside the rectangle, `default_color` outside or without
/// data. Disabled masks read the same: the coverage is what is edited.
pub(in crate::cxxqt_object) fn coverage_at(mask: &LayerMask, x: i32, y: i32) -> u8 {
    let (mw, mh) = (mask.rect.width(), mask.rect.height());
    let (mx, my) = (x - mask.rect.left, y - mask.rect.top);
    match &mask.data {
        Some(data) if mx >= 0 && my >= 0 && mx < mw && my < mh => data
            .get(my as usize * mw as usize + mx as usize)
            .copied()
            .unwrap_or(mask.default_color),
        _ => mask.default_color,
    }
}

/// The straight-alpha RGBA overlay of the mask of the layer at `path`, at most
/// `max_side` px on its long side, nearest-sampled: the rubylith colour at
/// `opacity × (255 − coverage) / 255`. `None` without a mask.
pub(super) fn overlay_rgba(
    doc: &Document,
    path: &str,
    (argb, opacity): (u32, u8),
    max_side: u32,
) -> Option<(u32, u32, Vec<u8>)> {
    let mask = pictura_render::resolve_path(doc, path)?.mask.as_ref()?;
    let longer = doc.width.max(doc.height).max(1);
    let scale = (max_side.max(1) as f64 / longer as f64).min(1.0);
    let w = ((doc.width as f64 * scale).round() as u32).max(1);
    let h = ((doc.height as f64 * scale).round() as u32).max(1);
    let [_, r, g, b] = argb.to_be_bytes();
    let opacity = u32::from(opacity.min(100));
    let columns: Vec<i32> = (0..w)
        .map(|x| (u64::from(x) * u64::from(doc.width) / u64::from(w)) as i32)
        .collect();
    let mut rgba = vec![0u8; w as usize * h as usize * 4];
    for (y, row) in rgba.chunks_mut(w as usize * 4).enumerate() {
        let dy = (y as u64 * doc.height as u64 / h as u64) as i32;
        for (&dx, px) in columns.iter().zip(row.chunks_mut(4)) {
            let hidden = 255 - u32::from(coverage_at(mask, dx, dy));
            px.copy_from_slice(&[r, g, b, ((hidden * opacity + 50) / 100) as u8]);
        }
    }
    Some((w, h, rgba))
}

pub(super) fn mask_overlay_toggle(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    let has_mask = view
        .rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::has_layer_mask(doc, &path));
    let mut rust = view.as_mut().rust_mut();
    let shown = has_mask && rust.mask_overlay.as_deref() != Some(path.as_str());
    rust.mask_overlay = shown.then_some(path);
    rust.mask_overlay_revision = rust.mask_overlay_revision.wrapping_add(1);
    shown
}

pub(super) fn mask_overlay_path(view: &PictureView) -> QString {
    QString::from(view.rust().mask_overlay.as_deref().unwrap_or_default())
}

pub(super) fn mask_overlay_revision(view: &PictureView) -> u64 {
    view.rust().mask_overlay_revision
}

pub(super) fn mask_display_color(view: &PictureView) -> u32 {
    view.rust().mask_display.0
}

pub(super) fn mask_display_opacity(view: &PictureView) -> i32 {
    i32::from(view.rust().mask_display.1)
}

pub(super) fn set_mask_display_options(mut view: Pin<&mut PictureView>, argb: u32, opacity: i32) {
    let mut rust = view.as_mut().rust_mut();
    rust.mask_display = (argb | 0xFF00_0000, opacity.clamp(0, 100) as u8);
    rust.mask_overlay_revision = rust.mask_overlay_revision.wrapping_add(1);
}

pub(super) fn mask_overlay_image(view: &PictureView, max_side: i32) -> QImage {
    let rust = view.rust();
    let (Some(doc), Some(path)) = (rust.session.doc.as_ref(), rust.mask_overlay.as_deref()) else {
        return QImage::default();
    };
    match overlay_rgba(doc, path, rust.mask_display, max_side.max(1) as u32) {
        Some((w, h, rgba)) => rgba_image(rgba, w as i32, h as i32),
        None => QImage::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::{coverage_at, overlay_rgba};
    use pictura_core::{BitDepth, ColorMode, Document, Layer, LayerMask, PsdRect};

    /// A 4×2 document whose one layer's mask covers the left 2×2, hiding its
    /// first column; the default colour (255) reveals the rest.
    fn doc() -> Document {
        let mut doc = Document::new(4, 2, ColorMode::Rgb, BitDepth::Eight);
        {
            let _ = doc.layers.push(Layer {
                mask: Some(LayerMask {
                    rect: PsdRect {
                        top: 0,
                        left: 0,
                        bottom: 2,
                        right: 2,
                    },
                    default_color: 255,
                    data: Some(vec![0u8, 128, 0, 128].into()),
                    ..Default::default()
                }),
                ..Default::default()
            });
        };
        doc
    }

    #[test]
    fn coverage_reads_the_rect_and_the_default() {
        let doc = doc();
        let mask = doc.layers[0].mask.as_ref().unwrap();
        let row: Vec<u8> = (-1..5).map(|x| coverage_at(mask, x, 1)).collect();
        assert_eq!(row, vec![255, 0, 128, 255, 255, 255]);
    }

    #[test]
    fn the_overlay_tints_hidden_coverage_at_the_opacity() {
        let (w, h, rgba) = overlay_rgba(&doc(), "0", (0xFFFF_0000, 50), 64).unwrap();
        assert_eq!((w, h), (4, 2));
        assert_eq!(&rgba[..4], &[255, 0, 0, 128], "hidden: red at 50%");
        assert_eq!(rgba[7], 64, "half coverage: half the opacity");
        assert_eq!(rgba[11], 0, "revealed: clear");
        let (_, _, blue) = overlay_rgba(&doc(), "0", (0xFF00_00FF, 100), 64).unwrap();
        assert_eq!(&blue[..4], &[0, 0, 255, 255]);
    }

    #[test]
    fn the_overlay_is_capped_on_its_long_side() {
        let (w, h, _) = overlay_rgba(&doc(), "0", (0xFFFF_0000, 50), 2).unwrap();
        assert_eq!((w, h), (2, 1));
    }
}
