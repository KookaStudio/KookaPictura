//! Image > Adjustments > Replace Color (#164): the dialog's sample-driven HSL
//! shift, previewed on the canvas through the Filter preview machinery and
//! committed as one "Replace Color" state. Free functions over a
//! [`PictureView`] (their own bridge), including the dialog's selection-mask
//! thumbnail.
//!
//! [`PictureView`]: super::qobject::PictureView

use super::helpers::active_pixel_layer;
use super::helpers_composite::current_buffer;
use super::impl_filters::{apply_op_active_region, cancel_filter_preview, ActiveOp};
use super::qobject::PictureView;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QImageFormat, QString};
use pictura_core::PsdRect;
use pictura_render::{replace_color_mask, Adjustment, ReplaceColorParams, ReplaceColorSample};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The Replace Color selection mask as a greyscale `QImage` fitted into
        /// a `size` box (white fully selected). Null when there is no document.
        fn image_replace_color_mask(
            view: &PictureView,
            samples: &QString,
            fuzziness: f64,
            localized: bool,
            size: i32,
        ) -> QImage;

        /// Preview Replace Color on the active layer over the document rect
        /// `(x, y, w, h)` (the whole layer when `w` is not positive), re-applied
        /// from the pre-preview pixels; no history. False when refused.
        #[allow(clippy::too_many_arguments)]
        fn image_replace_color_preview(
            view: Pin<&mut PictureView>,
            samples: &QString,
            fuzziness: f64,
            localized: bool,
            hue: f64,
            saturation: f64,
            lightness: f64,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
        ) -> bool;

        /// Apply Replace Color to the whole active layer as one "Replace Color"
        /// state (an open preview is replaced). False when refused.
        #[allow(clippy::too_many_arguments)]
        fn image_replace_color_apply(
            view: Pin<&mut PictureView>,
            samples: &QString,
            fuzziness: f64,
            localized: bool,
            hue: f64,
            saturation: f64,
            lightness: f64,
        ) -> bool;

        /// The Result swatch: `sample` (`0xRRGGBB`) shifted by `hue` degrees
        /// and `saturation` / `lightness` percent, as `0xRRGGBB`.
        fn image_replace_color_result(
            sample: u32,
            hue: f64,
            saturation: f64,
            lightness: f64,
        ) -> u32;

        /// The rounded `[hue, saturation, lightness]` that take `sample`
        /// closest to `result` (both `0xRRGGBB`).
        fn image_replace_color_shift_for(sample: u32, result: u32) -> Vec<i32>;
    }
}

fn rgb_of(c: u32) -> [u8; 3] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

fn image_replace_color_result(sample: u32, hue: f64, saturation: f64, lightness: f64) -> u32 {
    let [r, g, b] =
        pictura_render::replace_color_result(rgb_of(sample), hue, saturation, lightness);
    u32::from_be_bytes([0, r, g, b])
}

fn image_replace_color_shift_for(sample: u32, result: u32) -> Vec<i32> {
    let (hue, saturation, lightness) =
        pictura_render::replace_color_shift_for(rgb_of(sample), rgb_of(result));
    [hue, saturation, lightness]
        .map(|v| v.round() as i32)
        .to_vec()
}

/// Parse `"x,y,r,g,b;…"`; malformed entries are skipped rather than failing the
/// whole list, so a stray separator cannot discard colours the user picked.
fn parse_samples(text: &str) -> Vec<ReplaceColorSample> {
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

/// The active layer's document origin. The renderer crops a layer to its own
/// rect before the kernel runs, so sample positions must be layer-local for
/// Localized Color Clusters to land where the user clicked.
fn layer_origin(view: &PictureView) -> (i32, i32) {
    let rust = view.rust();
    rust.session
        .doc
        .as_ref()
        .and_then(|doc| active_pixel_layer(doc, rust.layer_target.single()))
        .map_or((0, 0), |layer| (layer.rect.left, layer.rect.top))
}

fn parsed_local(view: &PictureView, samples: &QString) -> Vec<ReplaceColorSample> {
    let (ox, oy) = layer_origin(view);
    let mut parsed = parse_samples(&samples.to_string());
    for s in &mut parsed {
        s.x -= ox;
        s.y -= oy;
    }
    parsed
}

#[allow(clippy::too_many_arguments)]
fn params(
    samples: Vec<ReplaceColorSample>,
    fuzziness: f64,
    localized: bool,
    hue: f64,
    saturation: f64,
    lightness: f64,
) -> ReplaceColorParams {
    ReplaceColorParams {
        samples,
        fuzziness,
        localized,
        hue,
        saturation,
        lightness,
    }
}

fn run(
    mut view: Pin<&mut PictureView>,
    p: ReplaceColorParams,
    preview: Option<PsdRect>,
    label: Option<&str>,
) -> bool {
    let commit = label.is_some();
    let region = {
        let mut rust = view.as_mut().rust_mut();
        apply_op_active_region(
            &mut rust,
            &ActiveOp::Adjustment(Adjustment::ReplaceColor(p)),
            commit,
            preview,
            false,
        )
    };
    let Some(region) = region else {
        {
            let mut rust = view.as_mut().rust_mut();
            let _ = cancel_filter_preview(&mut rust);
        }
        view.as_mut().recomposite();
        return false;
    };
    let outcome = match region {
        Some(rect) => Edit::regions(vec![rect], false),
        None => Edit::Full,
    };
    match label {
        Some(label) => {
            view.as_mut().edit(label, |_| outcome);
        }
        None => {
            view.as_mut().preview(|_| outcome);
        }
    }
    true
}

/// Qt pads 8-bit scanlines to a 32-bit boundary; a tightly packed mask must be
/// stretched to that stride or the `QImage` skews and reads past the buffer
/// when `width` is not a multiple of four.
fn grayscale_with_qt_stride(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let stride = (width + 3) & !3;
    if stride == width {
        return data.to_vec();
    }
    let mut padded = vec![0u8; stride * height];
    for y in 0..height {
        padded[y * stride..y * stride + width].copy_from_slice(&data[y * width..(y + 1) * width]);
    }
    padded
}

fn image_replace_color_mask(
    view: &PictureView,
    samples: &QString,
    fuzziness: f64,
    localized: bool,
    size: i32,
) -> QImage {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return QImage::default();
    };
    let p = params(
        parse_samples(&samples.to_string()),
        fuzziness,
        localized,
        0.0,
        0.0,
        0.0,
    );
    let mask = replace_color_mask(
        &current_buffer(doc, rust.gpu_compute),
        &p,
        size.clamp(1, 512) as usize,
    );
    if mask.width == 0 || mask.height == 0 {
        return QImage::default();
    }
    let width = mask.width as usize;
    let height = mask.height as usize;
    let bytes = grayscale_with_qt_stride(mask.data.as_ref(), width, height);
    unsafe {
        QImage::from_raw_bytes(
            bytes,
            width as i32,
            height as i32,
            QImageFormat::Format_Grayscale8,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn image_replace_color_preview(
    view: Pin<&mut PictureView>,
    samples: &QString,
    fuzziness: f64,
    localized: bool,
    hue: f64,
    saturation: f64,
    lightness: f64,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> bool {
    let parsed = parsed_local(&view, samples);
    // Localized weights depend on layer-local coordinates, which a cropped
    // visible-section preview would shift, so run the whole layer then.
    let section = (!localized && w > 0 && h > 0).then_some(PsdRect {
        top: y,
        left: x,
        bottom: y + h,
        right: x + w,
    });
    run(
        view,
        params(parsed, fuzziness, localized, hue, saturation, lightness),
        section,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn image_replace_color_apply(
    view: Pin<&mut PictureView>,
    samples: &QString,
    fuzziness: f64,
    localized: bool,
    hue: f64,
    saturation: f64,
    lightness: f64,
) -> bool {
    let parsed = parsed_local(&view, samples);
    run(
        view,
        params(parsed, fuzziness, localized, hue, saturation, lightness),
        None,
        Some("Replace Color"),
    )
}

#[cfg(test)]
mod tests {
    use super::grayscale_with_qt_stride;

    #[test]
    fn grayscale_rows_are_padded_to_a_four_byte_stride() {
        let width = 154usize;
        let height = 2usize;
        let data: Vec<u8> = (0..width * height).map(|i| i as u8).collect();
        let padded = grayscale_with_qt_stride(&data, width, height);
        let stride = (width + 3) & !3;
        assert_eq!(stride, 156);
        assert_eq!(padded.len(), stride * height);
        for y in 0..height {
            assert_eq!(
                &padded[y * stride..y * stride + width],
                &data[y * width..(y + 1) * width]
            );
        }
    }

    #[test]
    fn an_already_aligned_width_is_not_padded() {
        let data: Vec<u8> = (0..12).collect();
        assert_eq!(grayscale_with_qt_stride(&data, 4, 3), data);
    }
}
