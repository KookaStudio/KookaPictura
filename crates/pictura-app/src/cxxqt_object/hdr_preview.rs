use std::pin::Pin;

use cxx_qt::CxxQtType;
use cxx_qt_lib::QImage;
use pictura_codec::hdr_preview::{self, PreviewOptions};
use pictura_core::{Document, PixelBuffer, PsdRect};

use super::{edit::Edit, qobject::PictureView, PictureViewRust};

impl PictureViewRust {
    pub(super) fn hdr_preview_options(&self) -> Option<PreviewOptions> {
        let doc = self.session.doc.as_ref()?;
        if !pictura_render::supports_hdr_scene(doc) {
            return None;
        }
        Some(
            self.hdr_preview_draft
                .filter(|(generation, _)| *generation == self.session.document_generation)
                .map_or_else(|| hdr_preview::get(doc), |(_, options)| options),
        )
    }
}

pub(super) fn hdr_display_region(
    doc: &Document,
    rect: PsdRect,
    options: PreviewOptions,
    offset: i16,
    use_merged: bool,
) -> PixelBuffer {
    let Some(samples) = pictura_render::hdr_scene_region(doc, rect, use_merged) else {
        return super::tiles::display_region(doc, rect, false);
    };
    let n = samples.len();
    let mut out = vec![0; n * 4];
    for (i, [r, g, b, a]) in samples.into_iter().enumerate() {
        let mapped = options.map([r, g, b], f32::from(offset) / 16.0);
        for (c, value) in mapped.into_iter().chain([a]).enumerate() {
            out[c * n + i] = display_byte(value);
        }
    }
    let raw = PixelBuffer {
        width: rect.width() as u32,
        height: rect.height() as u32,
        channels: 4,
        data: out.into(),
    };
    super::helpers_composite::into_rgba_frame(pictura_codec::buffer_to_srgb(doc, &raw).into_owned())
}

fn display_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub fn hdr_preview_options(view: &PictureView) -> Vec<f64> {
    view.rust().hdr_preview_options().map_or_else(Vec::new, |o| {
        vec![f64::from(o.method), f64::from(o.exposure), f64::from(o.gamma)]
    })
}

/// Mode 0 drafts the display, mode 1 commits a metadata edit, and mode 2
/// discards the draft. The display invalidation affects presentation only.
pub fn set_hdr_preview_options(
    mut view: Pin<&mut PictureView>,
    method: i32,
    exposure: f64,
    gamma: f64,
    mode: i32,
) -> bool {
    if !matches!(mode, 0..=2) || view.rust().session.read_only() {
        return false;
    }
    let Some(options) = PreviewOptions::new(method, exposure as f32, gamma as f32) else {
        return false;
    };
    if view.rust().hdr_preview_options().is_none() {
        return false;
    }
    {
        let mut rust = view.as_mut().rust_mut();
        rust.hdr_preview_draft = (mode == 0).then_some((rust.session.document_generation, options));
        rust.refresh_hdr_tiles();
    }
    if mode == 1 {
        view.as_mut().edit("32-Bit Preview Options", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            if hdr_preview::get(doc) == options {
                return Edit::None;
            }
            hdr_preview::set(doc, options);
            Edit::Meta
        });
    }
    view.as_mut().pyramid_settled();
    true
}

pub fn hdr_scene_pixel(view: &PictureView, x: i32, y: i32) -> Vec<f64> {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return Vec::new();
    };
    let (Some(right), Some(bottom)) = (x.checked_add(1), y.checked_add(1)) else {
        return Vec::new();
    };
    let rect = PsdRect { top: y, left: x, bottom, right };
    pictura_render::hdr_scene_region(doc, rect, view.rust().hdr_merged_current && !view.rust().has_active_session() && !view.rust().display_preview_active()).map_or_else(Vec::new, |samples| {
        samples[0].into_iter().map(f64::from).collect()
    })
}

pub fn hdr_canvas_image(
    view: &PictureView,
    level: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    offset: i32,
) -> QImage {
    view.canvas_hdr_display_crop(level, x, y, w, h, None, offset.clamp(-160, 160) as i16)
}

pub fn hdr_swatch_argb(method: i32, exposure: f64, gamma: f64, r: f64, g: f64, b: f64) -> u32 {
    let exposure = (exposure as f32).clamp(-20.0, 20.0);
    let options = PreviewOptions::new(method, exposure, gamma as f32).unwrap_or_default();
    let [r, g, b] = options.map([r as f32, g as f32, b as f32], 0.0).map(display_byte);
    u32::from_be_bytes([255, r, g, b])
}
