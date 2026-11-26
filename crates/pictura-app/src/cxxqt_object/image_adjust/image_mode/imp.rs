use super::*;

use super::super::super::channels::mode_name;
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::helpers_composite::{buffer_to_rgba_bytes, current_buffer};
use crate::cxxqt_object::impl_core::{document_engine_error, JobBar, Outcome};
use crate::cxxqt_object::save_format::SaveGifOptions;
use crate::cxxqt_object::PictureViewRust;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QByteArray, QString};
use pictura_codec::{ColorReduction, Dither, PaletteOptions, PatternPixels};
use pictura_core::{BitDepth, ColorMode, Document};
use pictura_render::{BitmapMethod, HalftoneScreen, HalftoneShape};

pub(super) const HISTORY_LABEL: &str = "Convert Mode";

pub(super) fn mode_from_name(name: &str) -> Option<ColorMode> {
    Some(match name {
        "bitmap" => ColorMode::Bitmap,
        "grayscale" => ColorMode::Grayscale,
        "duotone" => ColorMode::Duotone,
        "indexed" => ColorMode::Indexed,
        "rgb" => ColorMode::Rgb,
        "cmyk" => ColorMode::Cmyk,
        "lab" => ColorMode::Lab,
        "multichannel" => ColorMode::Multichannel,
        _ => return None,
    })
}

pub(super) fn depth_from_bits(bits: i32) -> Option<BitDepth> {
    Some(match bits {
        8 => BitDepth::Eight,
        16 => BitDepth::Sixteen,
        32 => BitDepth::ThirtyTwo,
        _ => return None,
    })
}

pub(crate) fn palette_options(
    palette: i32,
    colors: i32,
    dither: i32,
    amount: i32,
) -> PaletteOptions {
    let (reduction, colors) = match palette {
        0 => (ColorReduction::Adaptive, 256),
        1 => (ColorReduction::Restrictive, 216),
        2 => (ColorReduction::Perceptual, colors),
        3 => (ColorReduction::Selective, colors),
        _ => (ColorReduction::Adaptive, colors),
    };
    // Exact keeps every color as it is, so there is nothing to dither.
    let dither = match (palette, dither) {
        (0, _) => Dither::None,
        (_, 1) => Dither::Diffusion,
        (_, 2) => Dither::Pattern,
        (_, 3) => Dither::Noise,
        _ => Dither::None,
    };
    PaletteOptions {
        reduction,
        colors: colors.clamp(2, 256) as u16,
        dither,
        amount: amount.clamp(0, 100) as u8,
        transparency: false,
        mapped_transparency: false,
        matte: [255, 255, 255],
    }
}

pub(super) fn bitmap_method(method: i32) -> BitmapMethod<'static> {
    match method {
        1 => BitmapMethod::Pattern,
        2 => BitmapMethod::Diffusion,
        _ => BitmapMethod::Threshold,
    }
}

pub(super) fn image_mode(view: &PictureView) -> QString {
    let rust = view.rust();
    let name = rust.session.doc.as_ref().map_or("", |doc| {
        mode_name(pictura_render::document_color_mode(doc))
    });
    QString::from(name)
}

pub(super) fn image_depth_bits(view: &PictureView) -> i32 {
    match view
        .rust()
        .session
        .doc
        .as_ref()
        .map(pictura_render::document_bit_depth)
    {
        Some(BitDepth::One) => 1,
        Some(BitDepth::Eight) => 8,
        Some(BitDepth::Sixteen) => 16,
        Some(BitDepth::ThirtyTwo) => 32,
        None => 0,
    }
}

pub(super) fn image_resolution(view: &PictureView) -> QString {
    view.rust()
        .session
        .doc
        .as_ref()
        .map_or_else(QString::default, |doc| {
            QString::from(&resolution_label(pictura_codec::document_resolution(doc)))
        })
}

/// A file without ResolutionInfo reads as CS6's 72 ppi default.
pub(super) fn resolution_label(resolution: Option<pictura_codec::Resolution>) -> String {
    let format = |value: f64| {
        let text = format!("{value:.2}");
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    };
    match resolution {
        Some(r) if r.per_cm => format!("{} pixels/cm", format(r.ppi / 2.54)),
        Some(r) => format!("{} pixels/inch", format(r.ppi)),
        None => "72 pixels/inch".to_string(),
    }
}

pub(super) fn image_mode_available(view: &PictureView, mode: &QString) -> bool {
    let Some(mode) = mode_from_name(&mode.to_string()) else {
        return false;
    };
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::can_convert_mode(doc, mode))
}

pub(super) fn image_depth_available(view: &PictureView, bits: i32) -> bool {
    let Some(out) = depth_from_bits(bits) else {
        return false;
    };
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::can_convert_depth(doc, out))
}

/// Run `convert` on the document and, when it succeeds, recomposite and record
/// one state. A flattening conversion leaves at most a Background, so the
/// active layer is re-pointed at it (or cleared for a flat Bitmap).
pub(super) fn commit(
    view: Pin<&mut PictureView>,
    convert: impl FnOnce(&mut Document) -> Result<(), pictura_ops::OpsError> + Send + 'static,
) -> bool {
    commit_mode_edit(view, HISTORY_LABEL, move |doc| convert(doc).map(|()| true))
}

fn commit_mode_edit(
    mut view: Pin<&mut PictureView>,
    label: &'static str,
    edit: impl FnOnce(&mut Document) -> Result<bool, pictura_ops::OpsError> + Send + 'static,
) -> bool {
    if view.rust().modal_transform_blocks_edit() {
        return view.document_preflight_refusal(pictura_error::ErrorCode::Capability);
    }
    let restored = {
        let mut rust = view.as_mut().rust_mut();
        let base = rust.session.live.take_mode_preview();
        let restored = base.is_some();
        if let Some(base) = base {
            rust.session.doc.set(Some(base));
        }
        restored
    };
    // The canvas leaves the preview before the conversion, which may run as a
    // job that a cancel ends with the restored document.
    if restored {
        view.as_mut().recomposite();
    }
    view.as_mut().document_op(
        label,
        JobBar::Busy,
        move |doc| {
            edit(doc)
                .map(|changed| changed.then(Outcome::default))
                .map_err(document_engine_error)
        },
        |rust, _| {
            refresh_active_layer(rust);
            Edit::Full
        },
    )
}

pub(super) fn refresh_active_layer(rust: &mut PictureViewRust) {
    let Some(count) = rust.session.doc.as_ref().map(|doc| doc.layers.len()) else {
        return;
    };
    let target = {
        let mut target = rust.layer_target.clone();
        if let Some(doc) = rust.session.doc.as_ref() {
            target.retain_resolving(doc);
        }
        target
    };
    rust.set_layer_target(target);
    if rust.layer_target.is_empty() && count > 0 {
        rust.select_only_layer(&(count - 1).to_string());
    }
}

pub(super) fn convert_image_mode(view: Pin<&mut PictureView>, mode: &QString) -> bool {
    match mode_from_name(&mode.to_string()) {
        Some(
            mode @ (ColorMode::Grayscale
            | ColorMode::Rgb
            | ColorMode::Cmyk
            | ColorMode::Lab
            | ColorMode::Multichannel),
        ) => {
            let (icc, options) = {
                let rust = view.rust();
                (rust.working_cmyk.clone(), rust.conversion_options)
            };
            commit(view, move |doc| {
                let profile = pictura_codec::Profile::from_icc(&icc).ok();
                pictura_render::convert_mode_with_profiles(doc, mode, profile.as_ref(), options)
            })
        }
        Some(_) => view.document_preflight_refusal(pictura_error::ErrorCode::Capability),
        None => view.document_preflight_refusal(pictura_error::ErrorCode::InvalidArg),
    }
}

pub(super) fn convert_image_depth(view: Pin<&mut PictureView>, bits: i32) -> bool {
    match depth_from_bits(bits) {
        Some(out) => commit(view, move |doc| pictura_render::convert_bit_depth(doc, out)),
        None => view.document_preflight_refusal(pictura_error::ErrorCode::InvalidArg),
    }
}

pub(super) fn indexed_exact_available(view: &PictureView) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(pictura_render::indexed_exact_available)
}

pub(super) fn convert_to_indexed(
    view: Pin<&mut PictureView>,
    palette: i32,
    colors: i32,
    dither: i32,
    amount: i32,
) -> bool {
    convert_to_indexed_with_options(
        view,
        palette,
        colors,
        dither,
        amount,
        false,
        &QByteArray::default(),
    )
}

pub(super) fn preview_indexed(
    view: Pin<&mut PictureView>,
    palette: i32,
    colors: i32,
    dither: i32,
    amount: i32,
) -> bool {
    preview_indexed_with_options(
        view,
        palette,
        colors,
        dither,
        amount,
        false,
        &QByteArray::default(),
    )
}

fn forced_colors(rgb: &QByteArray) -> Option<Vec<[u8; 3]>> {
    let (colors, trailing) = rgb.as_slice().as_chunks::<3>();
    (colors.len() <= 256 && trailing.is_empty()).then(|| colors.to_vec())
}

pub(super) fn indexed_exact_available_with_options(
    view: &PictureView,
    transparency: bool,
    forced_rgb: &QByteArray,
) -> bool {
    let Some(forced) = forced_colors(forced_rgb) else {
        return false;
    };
    let mut options = palette_options(0, 256, 0, 0);
    options.transparency = transparency;
    let rust = view.rust();
    rust.session
        .live
        .mode_preview()
        .or(rust.session.doc.as_ref())
        .is_some_and(|doc| pictura_render::document_ops::indexed_exact_fits(doc, options, &forced))
}

pub(super) fn convert_to_indexed_with_options(
    view: Pin<&mut PictureView>,
    palette: i32,
    colors: i32,
    dither: i32,
    amount: i32,
    transparency: bool,
    forced_rgb: &QByteArray,
) -> bool {
    let Some(forced) = forced_colors(forced_rgb) else {
        return view.document_preflight_refusal(pictura_error::ErrorCode::InvalidArg);
    };
    let mut options = palette_options(palette, colors, dither, amount);
    options.transparency = transparency;
    commit(view, move |doc| {
        pictura_render::document_ops::convert_to_indexed_with_forced(
            doc,
            options,
            &forced,
            palette == 0,
        )
    })
}

pub(super) fn preview_indexed_with_options(
    mut view: Pin<&mut PictureView>,
    palette: i32,
    colors: i32,
    dither: i32,
    amount: i32,
    transparency: bool,
    forced_rgb: &QByteArray,
) -> bool {
    let Some(forced) = forced_colors(forced_rgb) else {
        return false;
    };
    let mut options = palette_options(palette, colors, dither, amount);
    options.transparency = transparency;
    let previewed = {
        let mut rust = view.as_mut().rust_mut();
        let Some(mut converted) = rust.mode_preview_base() else {
            return false;
        };
        let ok = pictura_render::document_ops::convert_to_indexed_with_forced(
            &mut converted,
            options,
            &forced,
            palette == 0,
        )
        .is_ok();
        if ok {
            rust.session.doc.set(Some(converted));
        }
        ok
    };
    if previewed {
        view.as_mut().recomposite();
    }
    previewed
}

fn pack_table(table: pictura_codec::ColorTable) -> QByteArray {
    let bytes: Vec<_> = table
        .colors
        .iter()
        .enumerate()
        .flat_map(|(i, &[r, g, b])| {
            [
                r,
                g,
                b,
                if table.transparent == Some(i as u8) {
                    0
                } else {
                    255
                },
            ]
        })
        .collect();
    QByteArray::from(bytes.as_slice())
}

fn unpack_table(rgba: &QByteArray) -> Option<pictura_codec::ColorTable> {
    let (entries, trailing) = rgba.as_slice().as_chunks::<4>();
    if !trailing.is_empty() || !(1..=256).contains(&entries.len()) {
        return None;
    }
    let mut table = pictura_codec::ColorTable::default();
    for (i, &[r, g, b, alpha]) in entries.iter().enumerate() {
        match alpha {
            255 => {}
            0 if table.transparent.is_none() => table.transparent = Some(i as u8),
            _ => return None,
        }
        table.colors.push([r, g, b]);
    }
    Some(table)
}

pub(super) fn indexed_color_table(view: &PictureView) -> QByteArray {
    view.rust()
        .session
        .doc
        .as_ref()
        .and_then(pictura_render::document_ops::indexed_color_table)
        .map_or_else(QByteArray::default, pack_table)
}

pub(super) fn indexed_color_table_minimum(view: &PictureView) -> i32 {
    view.rust()
        .session
        .doc
        .as_ref()
        .and_then(pictura_render::document_ops::indexed_color_table_minimum)
        .map_or(0, |count| count as i32)
}

pub(super) fn apply_indexed_color_table(view: Pin<&mut PictureView>, rgba: &QByteArray) -> bool {
    let Some(table) = unpack_table(rgba) else {
        return view.document_preflight_refusal(pictura_error::ErrorCode::InvalidArg);
    };
    if view
        .rust()
        .session
        .doc
        .as_ref()
        .and_then(pictura_render::document_ops::indexed_color_table)
        .is_some_and(|current| current == table)
    {
        return false;
    }
    commit_mode_edit(view, "Color Table", move |doc| {
        pictura_render::document_ops::replace_indexed_color_table(doc, table)
    })
}

pub(super) fn decode_indexed_act(bytes: &QByteArray) -> QByteArray {
    pictura_codec::parse_act(bytes.as_slice()).map_or_else(QByteArray::default, pack_table)
}

pub(super) fn encode_indexed_act(rgba: &QByteArray) -> QByteArray {
    unpack_table(rgba).map_or_else(QByteArray::default, |table| {
        QByteArray::from(pictura_codec::encode_act(&table).as_slice())
    })
}

pub(super) fn preview_gif_indexed(
    mut view: Pin<&mut PictureView>,
    palette: i32,
    colors: i32,
    dither: i32,
    amount: i32,
) -> bool {
    let options = SaveGifOptions {
        palette,
        colors,
        dither,
        amount,
        interlaced: false,
    }
    .palette_options();
    let previewed = {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.mode_preview_base() else {
            return false;
        };
        let buffer = current_buffer(&doc, rust.gpu_compute);
        let srgb = pictura_codec::buffer_to_srgb(&doc, &buffer);
        let rgba = buffer_to_rgba_bytes(&srgb);
        let indexed = pictura_codec::quantize(&rgba, doc.width, doc.height, options);
        let plane = doc.width as usize * doc.height as usize;
        if indexed.indices.len() < plane {
            return false;
        }
        let mut preview = Vec::with_capacity(plane * 4);
        for &index in &indexed.indices[..plane] {
            let Some(entry) = indexed.palette.get(index as usize) else {
                return false;
            };
            preview.extend_from_slice(&[entry[0], entry[1], entry[2], 255]);
        }
        rust.session.doc.set(Some(Document::from_rgba(
            "GIF Preview",
            doc.width,
            doc.height,
            &preview,
        )));
        true
    };
    if previewed {
        view.as_mut().recomposite();
    }
    previewed
}

impl super::super::super::state::PictureViewRust {
    /// The document a mode preview converts from: the first base, kept so an
    /// update re-converts the original instead of the last preview.
    pub(in crate::cxxqt_object) fn mode_preview_base(&mut self) -> Option<pictura_core::Document> {
        if !self.slot_allows(super::super::super::document_session::SessionKind::ModePreview) {
            return None;
        }
        if self.session.live.mode_preview().is_none() {
            if let Some(base) = self.session.doc.snapshot() {
                self.session.live.set_mode_preview(base);
            }
        }
        self.session.live.mode_preview().cloned()
    }

    /// Drop the live mode preview, restoring the pre-preview document.
    pub(in crate::cxxqt_object) fn cancel_mode_preview(&mut self) -> bool {
        let Some(base) = self.session.live.take_mode_preview() else {
            return false;
        };
        self.session.doc.set(Some(base));
        true
    }
}

pub(super) fn cancel_mode_preview(mut view: Pin<&mut PictureView>) -> bool {
    if !view.as_mut().rust_mut().cancel_mode_preview() {
        return false;
    }
    view.as_mut().recomposite();
    true
}

pub(super) fn convert_to_bitmap(view: Pin<&mut PictureView>, method: i32) -> bool {
    let method = bitmap_method(method);
    commit(view, move |doc| {
        pictura_render::convert_to_bitmap(doc, method)
    })
}

pub(super) fn convert_to_bitmap_halftone(
    view: Pin<&mut PictureView>,
    frequency: f64,
    per_cm: bool,
    angle: f64,
    shape: i32,
) -> bool {
    let shapes = [
        HalftoneShape::Round,
        HalftoneShape::Diamond,
        HalftoneShape::Ellipse,
        HalftoneShape::Line,
        HalftoneShape::Square,
        HalftoneShape::Cross,
    ];
    let Some(&shape) = usize::try_from(shape).ok().and_then(|i| shapes.get(i)) else {
        return false;
    };
    let options = HalftoneScreen {
        frequency,
        per_cm,
        angle,
        shape,
    };
    commit(view, move |doc| {
        pictura_render::convert_to_bitmap(doc, BitmapMethod::HalftoneScreen(options))
    })
}

fn bitmap_pattern(doc: &Document, id: &str, native: &[u8]) -> Option<PatternPixels> {
    if id.is_empty() {
        return None;
    }
    if native.is_empty() {
        return pictura_codec::decode_patterns(doc)
            .into_iter()
            .find(|pattern| pattern.pattern_id == id)
            .or_else(|| pictura_render::builtin_pattern_by_id(id));
    }

    use pictura_codec::preset_library::{decode, document_pattern_id, Format, Kind, Payload};

    let mut entries = decode(Kind::Patterns, Format::Native, native)
        .ok()?
        .library
        .entries
        .into_iter();
    let Payload::Pattern(pattern) = entries.next()?.payload else {
        return None;
    };
    (entries.next().is_none() && document_pattern_id(&pattern) == id).then_some(pattern)
}

pub(super) fn convert_to_bitmap_pattern(
    view: Pin<&mut PictureView>,
    pattern_id: &QString,
    native: &QByteArray,
) -> bool {
    let id = pattern_id.to_string();
    let native = native.as_slice().to_vec();
    commit(view, move |doc| {
        let pattern = bitmap_pattern(doc, &id, &native);
        pictura_render::convert_to_bitmap(doc, BitmapMethod::CustomPattern(pattern.as_ref()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_ignores_the_dither_and_web_ignores_the_count() {
        let exact = palette_options(0, 12, 1, 75);
        assert_eq!((exact.colors, exact.dither), (256, Dither::None));
        let web = palette_options(1, 12, 2, 40);
        assert_eq!(web.reduction, ColorReduction::Restrictive);
        assert_eq!((web.dither, web.amount), (Dither::Pattern, 40));
        let local = palette_options(3, 1, 9, 400);
        assert_eq!(
            (local.reduction, local.colors, local.dither, local.amount),
            (ColorReduction::Selective, 2, Dither::None, 100)
        );
    }

    #[test]
    fn resolution_labels_follow_the_display_unit() {
        use pictura_codec::Resolution;
        assert_eq!(resolution_label(None), "72 pixels/inch");
        let inch = Resolution {
            ppi: 300.0,
            per_cm: false,
        };
        assert_eq!(resolution_label(Some(inch)), "300 pixels/inch");
        let cm = Resolution {
            ppi: 72.0,
            per_cm: true,
        };
        assert_eq!(resolution_label(Some(cm)), "28.35 pixels/cm");
    }

    #[test]
    fn names_and_depths_map_both_ways() {
        for mode in [
            ColorMode::Grayscale,
            ColorMode::Indexed,
            ColorMode::Cmyk,
            ColorMode::Lab,
        ] {
            assert_eq!(mode_from_name(mode_name(mode)), Some(mode));
        }
        assert_eq!(mode_from_name("hsv"), None);
        assert_eq!(depth_from_bits(16), Some(BitDepth::Sixteen));
        assert_eq!(depth_from_bits(1), None);
        assert_eq!(bitmap_method(2), BitmapMethod::Diffusion);
    }
}
