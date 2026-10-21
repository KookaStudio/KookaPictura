//! Image ▸ Mode: the color-mode and bit-depth conversions, the Indexed Color
//! dialog's live preview, and the menu's enabled/checked state. Free functions
//! over a [`PictureView`] (their own bridge). Ported from photorust's
//! `setColorMode` / `convertToIndexed` / `setBitDepth` bridge.
//!
//! Every conversion records one `Convert Mode` state (inferred CS6 label).
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::channels::mode_name;
use super::super::qobject::PictureView;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_codec::{ColorReduction, Dither, PaletteOptions};
use pictura_core::{BitDepth, ColorMode, Document};
use pictura_render::BitmapMethod;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The document's mode as the user sees it (`bitmap`, `grayscale`,
        /// `duotone`, `indexed`, `rgb`, `cmyk`, `lab`, `multichannel`); empty
        /// without a document.
        fn image_mode(view: &PictureView) -> QString;

        /// The document's bits per channel as the user sees it (1 for Bitmap);
        /// 0 without a document.
        fn image_depth_bits(view: &PictureView) -> i32;

        /// The document's resolution for the Properties panel, e.g. `"72
        /// pixels/inch"`; empty without a document.
        fn image_resolution(view: &PictureView) -> QString;

        /// Whether Image ▸ Mode ▸ `mode` is available (an `image_mode` name).
        fn image_mode_available(view: &PictureView, mode: &QString) -> bool;

        /// Whether Image ▸ Mode ▸ `bits` Bits/Channel is available.
        fn image_depth_available(view: &PictureView, bits: i32) -> bool;

        /// Convert to `grayscale`, `rgb`, `cmyk`, or `lab`. One "Convert Mode"
        /// state; false (nothing recorded) when the conversion is refused.
        fn convert_image_mode(view: Pin<&mut PictureView>, mode: &QString) -> bool;

        /// Convert between 8, 16, and 32 bits per channel (a 32-bit source
        /// goes through `convert_depth`'s HDR toning instead). One "Convert
        /// Mode" state; false when refused.
        fn convert_image_depth(view: Pin<&mut PictureView>, bits: i32) -> bool;

        /// Whether the image has at most 256 colors (the Exact palette).
        fn indexed_exact_available(view: &PictureView) -> bool;

        /// Image ▸ Mode ▸ Indexed Color. `palette`: 0 Exact, 1 Web, 2 Local
        /// (Perceptual), 3 Local (Selective), 4 Local (Adaptive); `dither`: 0
        /// None, 1 Diffusion, 2 Pattern, 3 Noise; `amount` 0–100. Flattens.
        /// Ends any preview; one "Convert Mode" state; false when refused.
        fn convert_to_indexed(
            view: Pin<&mut PictureView>,
            palette: i32,
            colors: i32,
            dither: i32,
            amount: i32,
        ) -> bool;

        /// Show the Indexed Color conversion on the canvas without recording
        /// history; each call converts the pre-preview document again.
        fn preview_indexed(
            view: Pin<&mut PictureView>,
            palette: i32,
            colors: i32,
            dither: i32,
            amount: i32,
        ) -> bool;

        /// Restore the document from before `preview_indexed`; false when no
        /// preview is shown.
        fn cancel_mode_preview(view: Pin<&mut PictureView>) -> bool;

        /// Image ▸ Mode ▸ Bitmap from 8-bit Grayscale. `method`: 0 50%
        /// Threshold, 1 Pattern Dither, 2 Diffusion Dither. Flattens to a flat
        /// image; one "Convert Mode" state; false when refused.
        fn convert_to_bitmap(view: Pin<&mut PictureView>, method: i32) -> bool;
    }
}

const HISTORY_LABEL: &str = "Convert Mode";

fn mode_from_name(name: &str) -> Option<ColorMode> {
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

fn depth_from_bits(bits: i32) -> Option<BitDepth> {
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
        matte: [255, 255, 255],
    }
}

fn bitmap_method(method: i32) -> BitmapMethod {
    match method {
        1 => BitmapMethod::Pattern,
        2 => BitmapMethod::Diffusion,
        _ => BitmapMethod::Threshold,
    }
}

fn image_mode(view: &PictureView) -> QString {
    let rust = view.rust();
    let name = rust.doc.as_ref().map_or("", |doc| {
        mode_name(pictura_render::document_color_mode(doc))
    });
    QString::from(name)
}

fn image_depth_bits(view: &PictureView) -> i32 {
    match view
        .rust()
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

fn image_resolution(view: &PictureView) -> QString {
    view.rust()
        .doc
        .as_ref()
        .map_or_else(QString::default, |doc| {
            QString::from(&resolution_label(pictura_codec::document_resolution(doc)))
        })
}

/// A file without ResolutionInfo reads as CS6's 72 ppi default.
fn resolution_label(resolution: Option<pictura_codec::Resolution>) -> String {
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

fn image_mode_available(view: &PictureView, mode: &QString) -> bool {
    let Some(mode) = mode_from_name(&mode.to_string()) else {
        return false;
    };
    view.rust()
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::can_convert_mode(doc, mode))
}

fn image_depth_available(view: &PictureView, bits: i32) -> bool {
    let Some(out) = depth_from_bits(bits) else {
        return false;
    };
    view.rust()
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::can_convert_depth(doc, out))
}

/// Run `convert` on the document and, when it succeeds, recomposite and record
/// one state. A flattening conversion leaves at most a Background, so the
/// active layer is re-pointed at it (or cleared for a flat Bitmap).
fn commit(mut view: Pin<&mut PictureView>, convert: impl FnOnce(&mut Document) -> bool) -> bool {
    let converted = {
        let mut rust = view.as_mut().rust_mut();
        if let Some(base) = rust.mode_preview.take() {
            rust.doc = Some(base);
        }
        let Some(doc) = rust.doc.as_mut() else {
            return false;
        };
        convert(doc)
    };
    if !converted {
        view.as_mut().recomposite();
        return false;
    }
    refresh_active_layer(view.as_mut());
    view.as_mut().edit(HISTORY_LABEL, |_| Edit::Full);
    true
}

fn refresh_active_layer(mut view: Pin<&mut PictureView>) {
    let mut rust = view.as_mut().rust_mut();
    let Some(count) = rust.doc.as_ref().map(|doc| doc.layers.len()) else {
        return;
    };
    let target = {
        let mut target = rust.layer_target.clone();
        if let Some(doc) = rust.doc.as_ref() {
            target.retain_resolving(doc);
        }
        target
    };
    rust.set_layer_target(target);
    if rust.layer_target.is_empty() && count > 0 {
        rust.select_only_layer(&(count - 1).to_string());
    }
}

fn convert_image_mode(view: Pin<&mut PictureView>, mode: &QString) -> bool {
    match mode_from_name(&mode.to_string()) {
        Some(mode @ (ColorMode::Grayscale | ColorMode::Rgb | ColorMode::Cmyk | ColorMode::Lab)) => {
            commit(view, |doc| pictura_render::convert_mode(doc, mode).is_ok())
        }
        _ => false,
    }
}

fn convert_image_depth(view: Pin<&mut PictureView>, bits: i32) -> bool {
    match depth_from_bits(bits) {
        Some(out) => commit(view, |doc| {
            pictura_render::convert_bit_depth(doc, out).is_ok()
        }),
        None => false,
    }
}

fn indexed_exact_available(view: &PictureView) -> bool {
    view.rust()
        .doc
        .as_ref()
        .is_some_and(pictura_render::indexed_exact_available)
}

fn convert_to_indexed(
    view: Pin<&mut PictureView>,
    palette: i32,
    colors: i32,
    dither: i32,
    amount: i32,
) -> bool {
    let options = palette_options(palette, colors, dither, amount);
    commit(view, |doc| {
        pictura_render::convert_to_indexed(doc, options).is_ok()
    })
}

fn preview_indexed(
    mut view: Pin<&mut PictureView>,
    palette: i32,
    colors: i32,
    dither: i32,
    amount: i32,
) -> bool {
    let options = palette_options(palette, colors, dither, amount);
    let previewed = {
        let mut rust = view.as_mut().rust_mut();
        let Some(mut converted) = rust.mode_preview_base() else {
            return false;
        };
        let ok = pictura_render::convert_to_indexed(&mut converted, options).is_ok();
        if ok {
            rust.doc = Some(converted);
        }
        ok
    };
    if previewed {
        view.as_mut().recomposite();
    }
    previewed
}

impl super::super::state::PictureViewRust {
    /// The document a mode preview converts from: the first base, kept so an
    /// update re-converts the original instead of the last preview.
    pub(in crate::cxxqt_object) fn mode_preview_base(&mut self) -> Option<pictura_core::Document> {
        if !self.slot_allows(super::super::document_session::SessionKind::ModePreview) {
            return None;
        }
        if self.mode_preview.is_none() {
            self.mode_preview = self.doc.clone();
        }
        self.mode_preview.clone()
    }

    /// Drop the live mode preview, restoring the pre-preview document.
    pub(in crate::cxxqt_object) fn cancel_mode_preview(&mut self) -> bool {
        let Some(base) = self.mode_preview.take() else {
            return false;
        };
        self.doc = Some(base);
        true
    }
}

fn cancel_mode_preview(mut view: Pin<&mut PictureView>) -> bool {
    if !view.as_mut().rust_mut().cancel_mode_preview() {
        return false;
    }
    view.as_mut().recomposite();
    true
}

fn convert_to_bitmap(view: Pin<&mut PictureView>, method: i32) -> bool {
    let method = bitmap_method(method);
    commit(view, |doc| {
        pictura_render::convert_to_bitmap(doc, method).is_ok()
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
