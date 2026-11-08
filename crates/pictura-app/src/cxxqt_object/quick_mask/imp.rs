use super::*;
use crate::cxxqt_object::{
    document_session::{SessionKind, StrokeSession},
    edit::Edit,
    helpers::mask_stroke_config,
    state::PictureViewRust,
};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QImageFormat};
use pictura_core::{BitDepth, Channel, ColorMode, Document, Layer, LockFlags, PsdRect};
use pictura_paint::{Stroke, StrokeConfig, StrokeSample};
use pictura_select::{
    quick_mask::{overlay, working_value},
    Selection,
};

#[derive(Clone)]
pub(in crate::cxxqt_object) struct QuickMaskState {
    pub active: bool,
    pub coverage: Option<Selection>,
    pub deselected: Option<Selection>,
    pub selected: bool,
    pub gray: bool,
    pub color: u32,
    pub opacity: u8,
    pub pending: Option<(bool, u32, u8)>,
    pub move_origin: Option<(Document, crate::layer_target::LayerTarget)>,
}
impl Default for QuickMaskState {
    fn default() -> Self {
        Self {
            active: false,
            coverage: None,
            deselected: None,
            selected: false,
            gray: false,
            color: 0xffff0000,
            opacity: 50,
            pending: None,
            move_origin: None,
        }
    }
}
pub(in crate::cxxqt_object) struct QuickMaskStroke {
    pub document: Document,
    pub stroke: Stroke,
    pub baseline: Option<Selection>,
}

pub(in crate::cxxqt_object) fn quick_mask_coverage(view: &PictureView, x: i32, y: i32) -> i32 {
    i32::from(view.rust().quick_mask_coverage(x, y))
}
pub(in crate::cxxqt_object) fn quick_mask_active(view: &PictureView) -> bool {
    view.rust().session.quick_mask.active
}
pub(in crate::cxxqt_object) fn quick_mask_selected_areas(view: &PictureView) -> bool {
    view.rust().session.quick_mask.selected
}
pub(in crate::cxxqt_object) fn quick_mask_color(view: &PictureView) -> u32 {
    view.rust().session.quick_mask.color
}
pub(in crate::cxxqt_object) fn quick_mask_opacity(view: &PictureView) -> i32 {
    i32::from(view.rust().session.quick_mask.opacity)
}
pub(in crate::cxxqt_object) fn quick_mask_grayscale(view: &PictureView) -> bool {
    view.rust().session.quick_mask.gray
}
pub(in crate::cxxqt_object) fn quick_mask_set(
    mut view: Pin<&mut PictureView>,
    active: bool,
) -> bool {
    if view.rust().session.doc.is_none() {
        return false;
    }
    if view.rust().quick_mask_stroke_active() {
        view.as_mut().end_paint();
    }
    if view.rust().session.quick_mask.active == active {
        return true;
    }
    // A content preview is cancelled before its selection becomes coverage.
    view.as_mut().cancel_active_session();
    view.as_mut().clear_transform_preview();
    if let Some((doc, target)) = view
        .as_mut()
        .rust_mut()
        .session
        .quick_mask
        .move_origin
        .take()
    {
        let mut rust = view.as_mut().rust_mut();
        rust.session.doc = Some(doc);
        rust.layer_target = target;
        rust.reset_pyramid();
    }
    let changed = view.as_mut().rust_mut().set_quick_mask_mode(active);
    if changed {
        view.as_mut().changed();
    }
    changed
}
pub(in crate::cxxqt_object) fn quick_mask_set_options(
    mut view: Pin<&mut PictureView>,
    selected: bool,
    color: u32,
    opacity: i32,
) -> bool {
    let mut rust = view.as_mut().rust_mut();
    let values = (selected, color | 0xff000000, opacity.clamp(0, 100) as u8);
    if rust.quick_mask_stroke_active() {
        rust.session.quick_mask.pending = Some(values);
        return true;
    }
    rust.apply_quick_mask_options(values);
    view.as_mut().changed();
    true
}
pub(in crate::cxxqt_object) fn quick_mask_set_grayscale(
    mut view: Pin<&mut PictureView>,
    gray: bool,
) -> bool {
    if !quick_mask_active(&view) {
        return false;
    }
    let mut rust = view.as_mut().rust_mut();
    rust.session.quick_mask.gray = gray;
    rust.invalidate_quick_mask();
    view.as_mut().changed();
    true
}
pub(in crate::cxxqt_object) fn quick_mask_thumbnail(view: &PictureView, side: i32) -> QImage {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return QImage::default();
    };
    if !rust.session.quick_mask.active || doc.width == 0 || doc.height == 0 {
        return QImage::default();
    }
    let scale = (side.clamp(1, 512) as f64 / doc.width.max(doc.height) as f64).min(1.0);
    let (w, h) = (
        (doc.width as f64 * scale).round().max(1.0) as i32,
        (doc.height as f64 * scale).round().max(1.0) as i32,
    );
    let mut bytes = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let c = working_value(
                rust.quick_mask_coverage(
                    (x as u64 * doc.width as u64 / w as u64) as i32,
                    (y as u64 * doc.height as u64 / h as u64) as i32,
                ),
                rust.session.quick_mask.selected,
            );
            bytes.extend_from_slice(&[c, c, c, 255]);
        }
    }
    rgba_image(bytes, w, h)
}
fn rgba_image(bytes: Vec<u8>, w: i32, h: i32) -> QImage {
    // SAFETY: each caller emits exactly w*h premultiplied RGBA samples.
    unsafe { QImage::from_raw_bytes(bytes, w, h, QImageFormat::Format_RGBA8888_Premultiplied) }
}

impl PictureView {
    pub(in crate::cxxqt_object) fn begin_quick_mask_paint(
        mut self: Pin<&mut Self>,
        cfg: StrokeConfig,
        label: &str,
    ) -> bool {
        self.as_mut().rust_mut().begin_quick_mask_paint(cfg, label)
    }
}
impl PictureViewRust {
    pub(in crate::cxxqt_object) fn set_quick_mask_mode(&mut self, active: bool) -> bool {
        if self.session.doc.is_none() {
            return false;
        }
        if self.session.quick_mask.active == active {
            return true;
        }
        if active {
            self.session.quick_mask.coverage = self.session.take_selection();
            self.session.quick_mask.deselected = self.session.deselected_selection.take();
            if let Some(coverage) = &mut self.session.quick_mask.coverage {
                coverage
                    .data
                    .tile(coverage.width as usize, coverage.height as usize);
            }
        } else {
            let coverage = self.session.quick_mask.coverage.take();
            self.session.set_selection(coverage);
            self.session.deselected_selection = self.session.quick_mask.deselected.take();
        }
        self.session.quick_mask.active = active;
        self.session.quick_mask.gray = false;
        self.session.touch_selection();
        self.invalidate_quick_mask();
        true
    }
    pub(in crate::cxxqt_object) fn quick_mask_content_allowed(&self) -> bool {
        !self.session.quick_mask.active
    }
    pub(in crate::cxxqt_object) fn invalidate_quick_mask(&mut self) {
        self.canvas_revision = self.canvas_revision.wrapping_add(1);
        self.frame_revision = self.frame_revision.wrapping_add(1);
    }
    pub(in crate::cxxqt_object) fn apply_quick_mask_options(&mut self, values: (bool, u32, u8)) {
        let q = &mut self.session.quick_mask;
        (q.selected, q.color, q.opacity) = values;
        self.invalidate_quick_mask();
    }
    pub(in crate::cxxqt_object) fn quick_mask_stroke_active(&self) -> bool {
        self.session
            .live
            .stroke_session()
            .is_some_and(|s| s.quick_mask.is_some())
    }
    pub(in crate::cxxqt_object) fn quick_mask_coverage(&self, x: i32, y: i32) -> u8 {
        let Some(doc) = self.session.doc.as_ref() else {
            return 0;
        };
        if x < 0 || y < 0 || x >= doc.width as i32 || y >= doc.height as i32 {
            return 0;
        }
        let at = y as usize * doc.width as usize + x as usize;
        if let Some(stroke) = self
            .session
            .live
            .stroke_session()
            .and_then(|s| s.quick_mask.as_ref())
        {
            return stroke.document.layers[0].channels[0]
                .data
                .sample(at)
                .unwrap_or(255);
        }
        self.session
            .quick_mask
            .coverage
            .as_ref()
            .and_then(|s| s.data.sample(at))
            .unwrap_or(255)
    }
    pub(in crate::cxxqt_object) fn quick_mask_document(&self, working: bool) -> Option<Document> {
        let real = self.session.doc.as_ref()?;
        let bytes = u64::from(real.width) * u64::from(real.height);
        if !crate::cxxqt_object::helpers::memory_gate::admit(bytes * 4) {
            return None;
        }
        let mut coverage = self
            .session
            .quick_mask
            .coverage
            .clone()
            .unwrap_or_else(|| Selection::all(real.width, real.height));
        let (w, h) = (real.width as usize, real.height as usize);
        coverage.data.tile(w, h);
        if working && self.session.quick_mask.selected {
            invert(&mut coverage.data, w, h);
        }
        let mut document = Document::new(0, 0, ColorMode::Grayscale, BitDepth::Eight);
        document.width = real.width;
        document.height = real.height;
        document
            .layers
            .push(Layer {
                name: "Quick Mask".into(),
                rect: PsdRect {
                    left: 0,
                    top: 0,
                    right: real.width as i32,
                    bottom: real.height as i32,
                },
                channels: vec![Channel {
                    id: 0,
                    data: coverage.data,
                }],
                lock: LockFlags::default().with(LockFlags::TRANSPARENCY, true),
                ..Default::default()
            })
            .ok()?;
        Some(document)
    }
    pub(in crate::cxxqt_object) fn adopt_quick_mask_document(
        &mut self,
        document: Document,
        working: bool,
    ) -> bool {
        let mut data = document.layers[0].channels[0].data.clone();
        if working && self.session.quick_mask.selected {
            let (w, h) = (document.width as usize, document.height as usize);
            data.tile(w, h);
            invert(&mut data, w, h);
        }
        let selection = Selection {
            width: document.width,
            height: document.height,
            data,
        };
        let changed = match self.session.quick_mask.coverage.as_ref() {
            Some(before) => before != &selection,
            None => !selection.data.all_eq(255),
        };
        if changed {
            self.session.quick_mask.coverage = Some(selection);
            self.invalidate_quick_mask();
        }
        changed
    }
    pub(in crate::cxxqt_object) fn begin_quick_mask_paint(
        &mut self,
        cfg: StrokeConfig,
        label: &str,
    ) -> bool {
        if !self.session.quick_mask.active
            || self.layer_target.is_empty()
            || !self.slot_allows(SessionKind::MaskPaint)
            || self.session.live.kind().is_some()
        {
            return false;
        }
        let Some(document) = self.quick_mask_document(false) else {
            return false;
        };
        let Ok(mut stroke) = Stroke::begin_at(&document, "0", mask_stroke_config(cfg)) else {
            return false;
        };
        stroke.set_inverse_grayscale(self.session.quick_mask.selected);
        stroke.set_coverage_clip(self.session.selection.as_ref().map(|s| s.data.clone()));
        let session = StrokeSession {
            quick_mask: Some(QuickMaskStroke {
                document,
                stroke,
                baseline: self.session.quick_mask.coverage.clone(),
            }),
            stroke_label: label.to_string(),
            ..Default::default()
        };
        self.session.live.set_stroke_session(session);
        true
    }
    pub(in crate::cxxqt_object) fn sample_quick_mask(
        &mut self,
        x: f64,
        y: f64,
        pressure: f64,
        hold: bool,
    ) -> bool {
        let Some(ctx) = self
            .session
            .live
            .stroke_session_mut()
            .and_then(|s| s.quick_mask.as_mut())
        else {
            return false;
        };
        let changed = if hold {
            ctx.stroke.hold(&mut ctx.document)
        } else {
            ctx.stroke.sample(
                &mut ctx.document,
                StrokeSample {
                    x: x as f32,
                    y: y as f32,
                    pressure: pressure as f32,
                },
            )
        };
        if changed {
            self.canvas_revision = self.canvas_revision.wrapping_add(1);
        }
        changed
    }
    pub(in crate::cxxqt_object) fn finish_quick_mask_paint(&mut self) -> bool {
        let Some(mut session) = self.session.live.take_stroke_session() else {
            return false;
        };
        let Some(ctx) = session.quick_mask.take() else {
            return false;
        };
        let changed = ctx.stroke.finish().is_some();
        if changed {
            self.session.quick_mask.coverage = Some(Selection {
                width: ctx.document.width,
                height: ctx.document.height,
                data: ctx.document.layers[0].channels[0].data.clone(),
            });
        }
        if let Some(values) = self.session.quick_mask.pending.take() {
            self.apply_quick_mask_options(values);
        }
        self.invalidate_quick_mask();
        changed
    }
    pub(in crate::cxxqt_object) fn quick_mask_crop(&self, level: i32, rect: PsdRect) -> QImage {
        use crate::cxxqt_object::helpers_composite::planes_of;
        if level < 0 || level as usize >= self.pyramid.level_count() {
            return QImage::default();
        }
        let Some(planes) = self.level0.as_ref().and_then(planes_of) else {
            return QImage::default();
        };
        let base = self.pyramid.crop(planes, level as usize, rect);
        let q = &self.session.quick_mask;
        let scale = 1i64.checked_shl(level as u32).unwrap_or(i64::MAX);
        let doc = self.session.doc.as_ref().unwrap();
        if base.width() == 0 || base.height() == 0 {
            return QImage::default();
        }
        let coverage = self
            .session
            .live
            .stroke_session()
            .and_then(|s| s.quick_mask.as_ref())
            .map(|s| &s.document.layers[0].channels[0].data)
            .or_else(|| q.coverage.as_ref().map(|s| &s.data));
        let mut bytes = base.data().to_vec();
        for (oy, row) in bytes
            .chunks_exact_mut(base.width() as usize * 4)
            .enumerate()
        {
            let y = (rect.top as i64 + oy as i64) * scale;
            if y < 0 || y >= i64::from(doc.height) {
                continue;
            }
            let mask = coverage.and_then(|p| p.row(y as usize, doc.width as usize));
            let mask = mask.as_deref();
            for (ox, pixel) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                let x = (rect.left as i64 + ox as i64) * scale;
                if x < 0 || x >= i64::from(doc.width) {
                    continue;
                }
                let c = mask.and_then(|r| r.get(x as usize)).copied().unwrap_or(255);
                let color = if q.gray {
                    let v = working_value(c, q.selected);
                    [v, v, v, 255]
                } else {
                    overlay(*pixel, c, q.selected, q.color, q.opacity)
                };
                *pixel = color;
            }
        }
        rgba_image(bytes, base.width() as i32, base.height() as i32)
    }
}
#[path = "filter.rs"]
mod filter;
#[path = "paint.rs"]
mod paint;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// Invert a `w × h` coverage plane through its tiles, so no contiguous copy is
/// built.
fn invert(plane: &mut pictura_core::Plane, w: usize, h: usize) {
    plane.for_each_tile_mut(0..w, 0..h, w, |tile| {
        for c in tile.data.iter_mut() {
            *c = 255 - *c;
        }
    });
}
