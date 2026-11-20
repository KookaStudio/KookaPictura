use super::*;

use super::super::helpers_composite::{
    level_display_image_data, premultiplied_display_image, rgba_image,
};
use super::super::picture::{
    preview_display, preview_level, preview_picture, stream_picture, whole,
};
use super::super::state::PictureViewRust;
use super::super::tiles::display_region;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QString};
use pictura_color::{GamutCheck, Profile};
use pictura_core::{
    localization_strength, spatial_sigma, BitDepth, ColorMode, Document, PixelBuffer, PsdRect,
    ReplaceColorSample,
};
use pictura_select::{color_range, ColorRangeSelect, ColorRangeSettings, CombineMode, Selection};

/// The dialog's Out Of Gamut item. The select crate has no such variant; the
/// app evaluates it against the proof setup.
const OUT_OF_GAMUT: i32 = 11;

/// The Color Range dialog's display-only selection preview.
pub(in crate::cxxqt_object) struct ColorRangePreviewState {
    select: i32,
    samples: Vec<ReplaceColorSample>,
    fuzziness: u32,
    localized: bool,
    range: u32,
    invert: bool,
    mode: i32,
    gamut: Option<GamutCheck>,
}

impl ColorRangePreviewState {
    pub(in crate::cxxqt_object) fn active(&self) -> bool {
        self.mode != 0
    }
}

pub(super) fn color_range_available(view: &PictureView) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| doc.source.depth().unwrap_or(doc.depth) != BitDepth::ThirtyTwo)
}

pub(super) fn color_range_out_of_gamut_available(view: &PictureView) -> bool {
    view.rust().session.doc.as_ref().is_some_and(is_rgb)
}

fn is_rgb(doc: &Document) -> bool {
    doc.document_color_mode() == ColorMode::Rgb
}

fn is_out_of_gamut(select: i32) -> bool {
    select == OUT_OF_GAMUT
}

fn engine_select(select: i32) -> Option<ColorRangeSelect> {
    ColorRangeSelect::from_index(select)
}

fn build_gamut_check(rust: &PictureViewRust) -> Option<GamutCheck> {
    let doc = rust.session.doc.as_ref()?;
    if !is_rgb(doc) {
        return None;
    }
    let source = doc
        .source
        .working_profile()
        .as_ref()
        .and_then(|icc| Profile::from_icc(&icc[..]).ok())
        .unwrap_or_else(Profile::srgb);
    let setup = super::super::proofing::setup(&rust.proof);
    GamutCheck::new(&source, &setup).ok()
}

/// `"x,y,r,g,b;…"`; malformed entries are skipped, so a stray separator
/// cannot discard colours the user picked.
pub(super) fn parse_samples(text: &str) -> Vec<ReplaceColorSample> {
    let mut out = Vec::new();
    for entry in text.split(';') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let parts: Vec<&str> = entry.split(',').collect();
        if parts.len() < 5 {
            continue;
        }
        let (Ok(x), Ok(y), Ok(r), Ok(g), Ok(b)) = (
            parts[0].trim().parse::<i32>(),
            parts[1].trim().parse::<i32>(),
            parts[2].trim().parse::<u8>(),
            parts[3].trim().parse::<u8>(),
            parts[4].trim().parse::<u8>(),
        ) else {
            continue;
        };
        out.push(ReplaceColorSample {
            x,
            y,
            rgb: [r, g, b],
        });
    }
    out
}

/// Nearest-neighbour shrink of a planar buffer so its longer side is at most
/// `max_side`; the preview needs the look, not every pixel.
pub(super) fn shrink(buffer: &PixelBuffer, max_side: u32) -> PixelBuffer {
    let longer = buffer.width.max(buffer.height);
    if max_side == 0 || longer <= max_side {
        return buffer.clone();
    }
    let scale = max_side as f64 / longer as f64;
    let w = ((buffer.width as f64 * scale).round() as u32).max(1);
    let h = ((buffer.height as f64 * scale).round() as u32).max(1);
    let (sw, sh) = (buffer.width as usize, buffer.height as usize);
    let plane = sw * sh;
    let mut data = vec![0u8; w as usize * h as usize * buffer.channels as usize];
    for c in 0..buffer.channels as usize {
        for y in 0..h as usize {
            let sy = (y * sh / h as usize).min(sh - 1);
            for x in 0..w as usize {
                let sx = (x * sw / w as usize).min(sw - 1);
                data[c * w as usize * h as usize + y * w as usize + x] =
                    buffer.data[c * plane + sy * sw + sx];
            }
        }
    }
    PixelBuffer {
        width: w,
        height: h,
        channels: buffer.channels,
        data: data.into(),
    }
}

/// Map document positions through the preview level and the shrink grid.
fn preview_samples(
    samples: &[ReplaceColorSample],
    level: usize,
    level_w: u32,
    level_h: u32,
    w: u32,
    h: u32,
) -> Vec<ReplaceColorSample> {
    let lw = level_w.max(1) as i64;
    let lh = level_h.max(1) as i64;
    samples
        .iter()
        .map(|s| {
            if s.x < 0 || s.y < 0 {
                return *s;
            }
            let lx = (s.x as i64) >> level;
            let ly = (s.y as i64) >> level;
            ReplaceColorSample {
                x: (lx * w as i64 / lw) as i32,
                y: (ly * h as i64 / lh) as i32,
                rgb: s.rgb,
            }
        })
        .collect()
}

/// Multiply `coverage` by the live selection at document coordinates. The
/// preview maps each coverage pixel to `doc_x` / `doc_y` through `map`.
fn intersect_selection(
    rust: &PictureViewRust,
    coverage: &mut [u8],
    w: usize,
    h: usize,
    map: impl Fn(usize, usize) -> (i32, i32),
) {
    let Some(selection) = rust.session.selection.as_ref() else {
        return;
    };
    let cov = selection.coverage();
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = map(x, y);
            let s = cov.sample(dx, dy) as u16;
            let i = y * w + x;
            coverage[i] = ((coverage[i] as u16 * s + 127) / 255) as u8;
        }
    }
}

fn gamut_coverage(gamut: &GamutCheck, buffer: &PixelBuffer, invert: bool) -> Vec<u8> {
    let n = buffer.width as usize * buffer.height as usize;
    let mut rgb = Vec::with_capacity(n * 3);
    for i in 0..n {
        rgb.push(buffer.data.sample(i).unwrap_or(0));
        rgb.push(buffer.data.sample(n + i).unwrap_or(0));
        rgb.push(buffer.data.sample(2 * n + i).unwrap_or(0));
    }
    let mask = gamut.mask(&rgb).unwrap_or_else(|_| vec![false; n]);
    let mut out = vec![0u8; n];
    for (i, slot) in out.iter_mut().enumerate() {
        let transparent = buffer.channels == 4 && buffer.data.sample(3 * n + i).unwrap_or(255) == 0;
        let v = if transparent {
            0
        } else if mask.get(i).copied().unwrap_or(false) {
            255
        } else {
            0
        };
        *slot = if invert { 255 - v } else { v };
    }
    out
}

fn level_coverage(
    select: ColorRangeSelect,
    samples: &[ReplaceColorSample],
    fuzziness: u32,
    localization: f64,
    sigma: f32,
    invert: bool,
    buffer: &PixelBuffer,
) -> Vec<u8> {
    let settings = ColorRangeSettings {
        select,
        samples: samples.to_vec(),
        fuzziness,
        localization,
        spatial_sigma: sigma,
        invert,
        origin: (0, 0),
    };
    color_range(buffer, &settings).coverage_bytes()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn color_range_preview(
    view: &PictureView,
    select: i32,
    samples: &QString,
    fuzziness: i32,
    localized: bool,
    range: i32,
    invert: bool,
    max_side: i32,
    width: &mut i32,
    height: &mut i32,
) -> Vec<u8> {
    *width = 0;
    *height = 0;
    if !color_range_available(view) {
        return Vec::new();
    }
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return Vec::new();
    };
    if is_out_of_gamut(select) && !is_rgb(doc) {
        return Vec::new();
    }
    let samples = parse_samples(&samples.to_string());
    let side = max_side.max(1) as u32;
    let level = preview_level(doc.width, doc.height, side);
    let (level_w, level_h) = pictura_render::level_size(doc.width, doc.height, level);
    let buffer = shrink(&preview_picture(doc, rust, side), side);
    let (w, h) = (buffer.width, buffer.height);
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let mapped = preview_samples(&samples, level, level_w, level_h, w, h);
    let localization = if localized {
        localization_strength(range.clamp(0, 100) as u32)
    } else {
        0.0
    };
    let sigma = spatial_sigma(((w * w + h * h) as f32).sqrt());
    let mut coverage = if is_out_of_gamut(select) {
        let Some(gamut) = build_gamut_check(rust) else {
            return Vec::new();
        };
        gamut_coverage(&gamut, &buffer, invert)
    } else {
        let Some(select) = engine_select(select) else {
            return Vec::new();
        };
        level_coverage(
            select,
            &mapped,
            fuzziness.max(0) as u32,
            localization,
            sigma,
            invert,
            &buffer,
        )
    };
    intersect_selection(rust, &mut coverage, w as usize, h as usize, |x, y| {
        (
            (x as u64 * doc.width as u64 / w as u64) as i32,
            (y as u64 * doc.height as u64 / h as u64) as i32,
        )
    });
    *width = w as i32;
    *height = h as i32;
    coverage
}

pub(super) fn color_range_preview_image(view: &PictureView, max_side: i32) -> QImage {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return QImage::default();
    };
    let side = max_side.max(1) as u32;
    let painting = rust.session.live.stroke().is_some();
    match preview_display(doc, painting, side) {
        Some(level) => level_display_image_data(
            level.data().to_vec(),
            level.width() as i32,
            level.height() as i32,
        ),
        None => premultiplied_display_image(&display_region(doc, whole(doc), painting)),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn color_range_preview_set(
    mut view: Pin<&mut PictureView>,
    select: i32,
    samples: &QString,
    fuzziness: i32,
    localized: bool,
    range: i32,
    invert: bool,
    mode: i32,
) -> bool {
    let state = {
        let rust = view.rust();
        let Some(doc) = rust.session.doc.as_ref() else {
            return false;
        };
        if is_out_of_gamut(select) && !is_rgb(doc) {
            return false;
        }
        let gamut = if is_out_of_gamut(select) {
            build_gamut_check(rust)
        } else {
            None
        };
        ColorRangePreviewState {
            select,
            samples: parse_samples(&samples.to_string()),
            fuzziness: fuzziness.max(0) as u32,
            localized,
            range: range.clamp(0, 100) as u32,
            invert,
            mode: mode.clamp(0, 4),
            gamut,
        }
    };
    view.as_mut().rust_mut().color_range_preview = Some(state);
    view.as_mut().changed();
    true
}

pub(super) fn color_range_preview_clear(mut view: Pin<&mut PictureView>) -> bool {
    let had = view
        .as_mut()
        .rust_mut()
        .color_range_preview
        .take()
        .is_some();
    if had {
        view.as_mut().changed();
    }
    had
}

#[allow(clippy::too_many_arguments)]
pub(super) fn color_range_apply(
    view: Pin<&mut PictureView>,
    select: i32,
    samples: &QString,
    fuzziness: i32,
    localized: bool,
    range: i32,
    invert: bool,
) -> bool {
    let (shape, refine) = {
        let rust = view.rust();
        let Some(doc) = rust.session.doc.as_ref() else {
            return false;
        };
        if !color_range_available(&view) {
            return false;
        }
        if !is_out_of_gamut(select) && engine_select(select).is_none() {
            return false;
        }
        if is_out_of_gamut(select) && !is_rgb(doc) {
            return false;
        }
        let samples = parse_samples(&samples.to_string());
        let w = doc.width as usize;
        let mut data = vec![0u8; w * doc.height as usize];
        let localization = if localized {
            localization_strength(range.clamp(0, 100) as u32)
        } else {
            0.0
        };
        let sigma =
            spatial_sigma(((doc.width as f64).powi(2) + (doc.height as f64).powi(2)).sqrt() as f32);
        if is_out_of_gamut(select) {
            let Some(gamut) = build_gamut_check(rust) else {
                return false;
            };
            stream_picture(doc, rust, whole(doc), |offset, band| {
                let coverage = gamut_coverage(&gamut, &band, invert);
                let at = offset as usize * w;
                data[at..at + coverage.len()].copy_from_slice(&coverage);
            });
        } else {
            let Some(select) = engine_select(select) else {
                return false;
            };
            stream_picture(doc, rust, whole(doc), |offset, band| {
                let part = color_range(
                    &band,
                    &ColorRangeSettings {
                        select,
                        samples: samples.clone(),
                        fuzziness: fuzziness.max(0) as u32,
                        localization,
                        spatial_sigma: sigma,
                        invert,
                        origin: (0, offset as i32),
                    },
                );
                let at = offset as usize * w;
                part.data.read_into(0, &mut data[at..at + part.data.len()]);
            });
        }
        (
            Selection {
                width: doc.width,
                height: doc.height,
                data: data.into(),
            },
            rust.session.selection.is_some(),
        )
    };
    // Run on a live selection, Color Range narrows it to a subset (CS6).
    let mode = if refine {
        CombineMode::Intersect
    } else {
        CombineMode::New
    };
    view.apply_selection_labeled(shape, mode, "Color Range")
}

impl PictureViewRust {
    /// The composite crop of `level` over `rect` with the Color Range
    /// Selection Preview rendered over it.
    pub(in crate::cxxqt_object) fn color_range_crop(
        &self,
        level: i32,
        rect: PsdRect,
        state: &ColorRangePreviewState,
    ) -> (QImage, Vec<pictura_render::TileKey>) {
        if !self.level_meets(level, rect) {
            return (QImage::default(), Vec::new());
        }
        let (base, missing) = self.tile_crop(pictura_render::TileSource::Composite, level, rect);
        let Some(doc) = self.session.doc.as_ref() else {
            return (QImage::default(), missing);
        };
        let w = base.width() as usize;
        let h = base.height() as usize;
        if w == 0 || h == 0 {
            return (QImage::default(), missing);
        }
        let scale = 1i64.checked_shl(level as u32).unwrap_or(i64::MAX);
        let (level_w, level_h) =
            pictura_render::level_size(doc.width, doc.height, level.max(0) as usize);
        let src = base.data();
        let mut straight = vec![0u8; w * h * 4];
        for i in 0..w * h {
            let a = src[i * 4 + 3] as u32;
            if a == 0 {
                continue;
            }
            for c in 0..3 {
                straight[c * w * h + i] =
                    ((src[i * 4 + c] as u32 * 255 + a / 2) / a).min(255) as u8;
            }
            straight[3 * w * h + i] = a as u8;
        }
        let buffer = PixelBuffer {
            width: w as u32,
            height: h as u32,
            channels: 4,
            data: straight.clone().into(),
        };
        let samples: Vec<ReplaceColorSample> = state
            .samples
            .iter()
            .map(|s| {
                if s.x < 0 || s.y < 0 {
                    *s
                } else {
                    ReplaceColorSample {
                        x: (s.x as i64 >> level.max(0) as u32) as i32,
                        y: (s.y as i64 >> level.max(0) as u32) as i32,
                        rgb: s.rgb,
                    }
                }
            })
            .collect();
        let localization = if state.localized {
            localization_strength(state.range)
        } else {
            0.0
        };
        let sigma = spatial_sigma(((level_w * level_w + level_h * level_h) as f32).sqrt());
        let mut coverage = if is_out_of_gamut(state.select) {
            match state.gamut.as_ref() {
                Some(gamut) => gamut_coverage(gamut, &buffer, state.invert),
                None => vec![0u8; w * h],
            }
        } else if let Some(select) = engine_select(state.select) {
            color_range(
                &buffer,
                &ColorRangeSettings {
                    select,
                    samples,
                    fuzziness: state.fuzziness,
                    localization,
                    spatial_sigma: sigma,
                    invert: state.invert,
                    origin: (rect.left, rect.top),
                },
            )
            .coverage_bytes()
        } else {
            vec![0u8; w * h]
        };
        if let Some(selection) = self.session.selection.as_ref() {
            let cov = selection.coverage();
            for oy in 0..h {
                let dy = ((rect.top as i64 + oy as i64) * scale) as i32;
                for ox in 0..w {
                    let dx = ((rect.left as i64 + ox as i64) * scale) as i32;
                    let s = cov.sample(dx, dy) as u16;
                    coverage[oy * w + ox] = ((coverage[oy * w + ox] as u16 * s + 127) / 255) as u8;
                }
            }
        }
        let mut bytes = src.to_vec();
        let quick = self.session.quick_mask.color;
        let (qr, qg, qb) = ((quick >> 16) & 0xff, (quick >> 8) & 0xff, quick & 0xff);
        let opacity = self.session.quick_mask.opacity as u32;
        for i in 0..w * h {
            let c = coverage[i] as u32;
            let [r, g, b, a] = match state.mode {
                1 => [c as u8, c as u8, c as u8, 255],
                2 => {
                    let blend = |v: u32| ((v * c + 127) / 255) as u8;
                    [
                        blend(straight[i] as u32),
                        blend(straight[w * h + i] as u32),
                        blend(straight[2 * w * h + i] as u32),
                        255,
                    ]
                }
                3 => {
                    let blend = |v: u32| ((v * c + 255 * (255 - c) + 127) / 255) as u8;
                    [
                        blend(straight[i] as u32),
                        blend(straight[w * h + i] as u32),
                        blend(straight[2 * w * h + i] as u32),
                        255,
                    ]
                }
                4 => {
                    let t = (255 - c) * opacity / 100;
                    let mix = |v: u32, q: u32| ((v * (255 - t) + q * t + 127) / 255) as u8;
                    [
                        mix(straight[i] as u32, qr),
                        mix(straight[w * h + i] as u32, qg),
                        mix(straight[2 * w * h + i] as u32, qb),
                        255,
                    ]
                }
                _ => [src[i * 4], src[i * 4 + 1], src[i * 4 + 2], src[i * 4 + 3]],
            };
            let a = a as u32;
            bytes[i * 4] = ((r as u32 * a + 127) / 255) as u8;
            bytes[i * 4 + 1] = ((g as u32 * a + 127) / 255) as u8;
            bytes[i * 4 + 2] = ((b as u32 * a + 127) / 255) as u8;
            bytes[i * 4 + 3] = a as u8;
        }
        (rgba_image(bytes, w as i32, h as i32), missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cxxqt_object::document_session::DocumentSession;
    use crate::cxxqt_object::proofing::ProofState;
    use crate::cxxqt_object::state::PictureViewRust;
    use pictura_core::{BitDepth, ColorMode, Document};

    fn view_of(doc: Document) -> PictureViewRust {
        PictureViewRust {
            session: DocumentSession::with_document(doc),
            ..Default::default()
        }
    }

    #[test]
    fn color_range_gamut_follows_the_proof_setup() {
        let rgb = view_of(Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight));
        let check = build_gamut_check(&rgb).expect("RGB document has a gamut check");
        let mask = check.mask(&[0, 255, 0, 128, 128, 128]).expect("two pixels");
        assert!(mask[0], "saturated green is outside the Working CMYK gamut");
        assert!(!mask[1], "neutral grey is in gamut");

        let cmyk = view_of(Document::new(2, 2, ColorMode::Cmyk, BitDepth::Eight));
        assert!(build_gamut_check(&cmyk).is_none(), "CMYK has no OOG mode");
    }

    #[test]
    fn color_range_gamut_uses_the_working_profile() {
        let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        doc.source
            .set_working_profile(Some(Profile::adobe_rgb().to_icc().into()));
        let view = view_of(doc);
        let check = build_gamut_check(&view).expect("gamut check");
        let setup = crate::cxxqt_object::proofing::setup(&ProofState::default());
        let expected = GamutCheck::new(&Profile::adobe_rgb(), &setup).expect("reference check");
        let samples = [255, 0, 0, 0, 255, 0, 0, 0, 255, 128, 128, 128];
        assert_eq!(
            check.mask(&samples).expect("mask"),
            expected.mask(&samples).expect("reference mask")
        );
    }

    #[test]
    fn shrink_keeps_aspect_and_samples_nearest() {
        let (w, h) = (8u32, 4u32);
        let n = (w * h) as usize;
        let mut data = vec![0u8; 3 * n];
        for (i, v) in data[..n].iter_mut().enumerate() {
            *v = (i % w as usize) as u8 * 10;
        }
        let buffer = PixelBuffer {
            width: w,
            height: h,
            channels: 3,
            data: data.into(),
        };
        let small = shrink(&buffer, 4);
        assert_eq!((small.width, small.height), (4, 2));
        assert_eq!(&small.data[..4], &[0, 20, 40, 60]);
        assert_eq!(shrink(&buffer, 100).width, 8, "no upscaling");
    }

    #[test]
    fn parses_samples_and_skips_malformed() {
        let parsed = parse_samples("3,4,10,20,30; bad; 5,6,1,2");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].x, 3);
        assert_eq!(parsed[0].y, 4);
        assert_eq!(parsed[0].rgb, [10, 20, 30]);
    }

    #[test]
    fn localized_samples_stay_in_document_space() {
        // A 100-row band at document row 256. The sample sits at document
        // row 280 and the red pixel at the band's row 24: the band origin must
        // keep both in document space.
        let n = 100usize;
        let mut data = vec![0u8; 4 * n];
        data[24] = 255;
        data[3 * n + 24] = 255;
        let band = PixelBuffer {
            width: 1,
            height: n as u32,
            channels: 4,
            data: data.into(),
        };
        let settings = ColorRangeSettings {
            select: ColorRangeSelect::Sampled,
            samples: vec![ReplaceColorSample {
                x: 0,
                y: 280,
                rgb: [255, 0, 0],
            }],
            fuzziness: 40,
            localization: 1.0,
            spatial_sigma: spatial_sigma(356.0),
            invert: false,
            origin: (0, 256),
        };
        let mask = color_range(&band, &settings);
        assert_eq!(mask.data[24], 255, "the sample is at the document row");
    }
}
