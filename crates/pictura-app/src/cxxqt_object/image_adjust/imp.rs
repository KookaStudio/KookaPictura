use super::*;

use super::super::adjustment_edit::{format_points, page_rows, parse_points};
use super::super::helpers::{active_pixel_layer, mask_editing};
use super::super::helpers_composite::{level_display_image, sample_planar_argb};
use super::super::impl_filters::{apply_op_active_region, cancel_filter_preview, ActiveOp};
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QString, QStringList};
use pictura_core::{AdjustmentData, ColorMode, Layer, PsdRect};
use pictura_render::Adjustment;
use pictura_select::Selection;

pub(super) fn to_block(data: Option<AdjustmentData>) -> Vec<u8> {
    data.map(|d| {
        d.key
            .iter()
            .copied()
            .chain(d.data.iter().copied())
            .collect()
    })
    .unwrap_or_default()
}

pub(super) fn from_block(block: &[u8]) -> Option<AdjustmentData> {
    Some(AdjustmentData {
        key: block.get(..4)?.try_into().ok()?,
        data: block.get(4..)?.to_vec().into(),
    })
}

pub(super) fn rgb(c: u32) -> [u8; 3] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

pub(super) fn image_adjustment_default(
    kind: &QString,
    foreground: u32,
    background: u32,
    cmyk: bool,
) -> Vec<u8> {
    let Some(kind) = pictura_render::AdjustmentKind::from_name(&kind.to_string()) else {
        return Vec::new();
    };
    let ctx = pictura_render::Defaults {
        foreground: rgb(foreground),
        background: rgb(background),
        cmyk,
    };
    to_block(pictura_render::dialog_default(kind, ctx))
}

pub(super) fn image_adjustment_page(block: &[u8], cmyk: bool) -> QStringList {
    from_block(block)
        .and_then(|data| {
            let editor = pictura_render::adjustment_editor_in_mode(&data, cmyk)?;
            Some(page_rows(&editor, data.key == *b"thrs"))
        })
        .unwrap_or_default()
}

pub(super) fn image_adjustment_threshold_histogram(view: &PictureView, path: &QString) -> Vec<u32> {
    super::threshold_histogram::threshold_bins(view.rust(), &path.to_string()).to_vec()
}

pub(super) fn image_adjustment_set(block: &[u8], key: &QString, value: f64, cmyk: bool) -> Vec<u8> {
    to_block(from_block(block).and_then(|d| {
        pictura_render::set_adjustment_param_in_mode(&d, &key.to_string(), value, cmyk)
    }))
}

pub(super) fn image_adjust_sample(view: &PictureView, x: i32, y: i32, size: i32) -> u32 {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return 0;
    };
    let (width, height) = (doc.width as i32, doc.height as i32);
    if x < 0 || y < 0 || x >= width || y >= height {
        return 0;
    }
    let radius = (size.clamp(1, 101) - 1) / 2;
    let rect = PsdRect {
        top: (y - radius).max(0),
        left: (x - radius).max(0),
        bottom: (y + radius + 1).min(height),
        right: (x + radius + 1).min(width),
    };
    let pixels = super::super::picture::picture_region(doc, rust, rect);
    let mut weight = 0;
    let mut sum = [0; 3];
    for row in 0..rect.height() {
        for col in 0..rect.width() {
            let pixel = sample_planar_argb(&pixels, col, row);
            let alpha = pixel >> 24;
            weight += alpha;
            for (total, value) in sum.iter_mut().zip(rgb(pixel)) {
                *total += u32::from(value) * alpha;
            }
        }
    }
    if weight == 0 {
        return 0;
    }
    let [r, g, b] = sum.map(|value| value / weight);
    0xff000000 | (r << 16) | (g << 8) | b
}

pub(super) fn image_adjust_clipping_source(
    view: &PictureView,
    level: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> QImage {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return QImage::default();
    };
    if !(0..=30).contains(&level) || x < 0 || y < 0 || w <= 0 || h <= 0 || w > 1536 || h > 1536 {
        return QImage::default();
    }
    let (Some(right), Some(bottom)) = (x.checked_add(w), y.checked_add(h)) else {
        return QImage::default();
    };
    let rect = PsdRect {
        top: y,
        left: x,
        bottom,
        right,
    };
    let frame = pictura_render::composite_region_level(doc, rect, level as usize, &|chunk| {
        pictura_codec::buffer_to_srgb(doc, &chunk).into_owned()
    });
    level_display_image(&frame)
}

pub(super) fn image_adjustment_vibrance_auto(view: &PictureView) -> i32 {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return 0;
    };
    if doc.document_color_mode() != ColorMode::Rgb || mask_editing(rust) {
        return 0;
    }
    let Some(layer) = active_pixel_layer(doc, rust.layer_target.single()) else {
        return 0;
    };
    vibrance_auto_amount(layer, rust.session.selection.as_ref())
}

fn vibrance_auto_amount(layer: &Layer, selection: Option<&Selection>) -> i32 {
    const SAMPLE_SIDE: i64 = 128;
    const TARGET_SATURATION: f64 = 0.50;
    const MAX_VIBRANCE: f64 = 35.0;

    let rect = match selection {
        Some(selection) => selection.bounds().and_then(|rect| layer.rect.intersect(rect)),
        None => (!layer.rect.is_empty()).then_some(layer.rect),
    };
    let Some(rect) = rect else {
        return 0;
    };
    let coverage = selection.map(Selection::coverage);
    let plane = |id| layer.channels().iter().find(|c| c.id == id).map(|c| &c.data);
    let (Some(red), Some(green), Some(blue)) = (plane(0), plane(1), plane(2)) else {
        return 0;
    };
    let alpha = plane(-1);
    let stride = layer.rect.width() as usize;
    let width = i64::from(rect.width());
    let height = i64::from(rect.height());
    let cols = width.min(SAMPLE_SIDE);
    let rows = height.min(SAMPLE_SIDE);
    let mut weighted_saturation = 0.0;
    let mut total_weight = 0.0;
    for row in 0..rows {
        let y = rect.top + (((2 * row + 1) * height) / (2 * rows)) as i32;
        for col in 0..cols {
            let x = rect.left + (((2 * col + 1) * width) / (2 * cols)) as i32;
            let index = (i64::from(y) - i64::from(layer.rect.top)) as usize * stride
                + (i64::from(x) - i64::from(layer.rect.left)) as usize;
            let selected = coverage.as_ref().map_or(255, |mask| mask.sample(x, y));
            let opacity = alpha.map_or(255, |plane| plane.sample(index).unwrap_or(0));
            let weight = f64::from(selected) * f64::from(opacity);
            if weight == 0.0 {
                continue;
            }
            let (Some(r), Some(g), Some(b)) =
                (red.sample(index), green.sample(index), blue.sample(index))
            else {
                continue;
            };
            let low = r.min(g).min(b);
            let high = r.max(g).max(b);
            let chroma = high - low;
            if chroma < 4 {
                continue;
            }
            let hsl_denominator = 255.0 - (f64::from(high) + f64::from(low) - 255.0).abs();
            weighted_saturation += weight * f64::from(chroma) / hsl_denominator;
            total_weight += weight;
        }
    }
    if total_weight == 0.0 {
        return 0;
    }
    let mean = weighted_saturation / total_weight;
    if mean >= TARGET_SATURATION {
        return 0;
    }
    // Fit the existing Vibrance operator's nominal gain before its hue weighting.
    (100.0 * (TARGET_SATURATION - mean) / (1.0 - mean))
        .round()
        .min(MAX_VIBRANCE) as i32
}

pub(super) fn image_adjustment_curve(block: &[u8], channel: i32) -> QString {
    let points = from_block(block)
        .and_then(|d| pictura_render::curve_points(&d, usize::try_from(channel).ok()?));
    QString::from(
        points
            .map(|p| format_points(&p))
            .unwrap_or_default()
            .as_str(),
    )
}

pub(super) fn image_adjustment_set_curve(block: &[u8], channel: i32, points: &QString) -> Vec<u8> {
    let edited = (|| {
        let points = parse_points(&points.to_string())?;
        let channel = usize::try_from(channel).ok()?;
        pictura_render::set_curve_points(&from_block(block)?, channel, &points)
    })();
    to_block(edited)
}

/// Run `adjustment` on the active layer (a preview over `region` when given)
/// and refresh the canvas; records `label` on a commit.
pub(super) fn run(
    mut view: Pin<&mut PictureView>,
    adjustment: Adjustment,
    preview: Option<PsdRect>,
    label: Option<&str>,
) -> bool {
    let commit = label.is_some();
    let fade_capture = commit
        .then(|| crate::cxxqt_object::fade::capture(view.rust()))
        .flatten();
    let region = {
        let mut rust = view.as_mut().rust_mut();
        apply_op_active_region(
            &mut rust,
            &ActiveOp::Adjustment(adjustment),
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
            if let Some(capture) = fade_capture {
                crate::cxxqt_object::fade::record(&mut view.as_mut().rust_mut(), capture, label);
            }
        }
        None => {
            view.as_mut().preview(|_| outcome);
        }
    }
    true
}

pub(super) fn decode(block: &[u8]) -> Option<Adjustment> {
    pictura_render::decode_adjustment(&from_block(block)?)
}

pub(super) fn image_adjust_preview(
    view: Pin<&mut PictureView>,
    block: &[u8],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> bool {
    let Some(adjustment) = decode(block) else {
        return false;
    };
    let section = (w > 0 && h > 0).then_some(PsdRect {
        top: y,
        left: x,
        bottom: y + h,
        right: x + w,
    });
    run(view, adjustment, section, None)
}

pub(super) fn image_adjust_apply(
    view: Pin<&mut PictureView>,
    block: &[u8],
    label: &QString,
) -> bool {
    match decode(block) {
        Some(adjustment) => run(view, adjustment, None, Some(&label.to_string())),
        None => false,
    }
}

/// A dialog-less command's adjustment and history name. Both come from the
/// catalogue: the adjustment from the row's direct constructor, the label from
/// the row's title.
pub(super) fn direct(kind: &str) -> Option<(Adjustment, String)> {
    let kind = pictura_render::AdjustmentKind::from_name(kind)?;
    let adjustment = pictura_render::direct_adjustment(kind)?;
    let title = pictura_render::entry_of(kind)?.title.to_string();
    Some((adjustment, title))
}

pub(super) fn image_adjust_direct(view: Pin<&mut PictureView>, kind: &QString) -> bool {
    match direct(&kind.to_string()) {
        Some((adjustment, label)) => run(view, adjustment, None, Some(&label)),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_round_trip_and_edit() {
        let block = to_block(pictura_render::dialog_default(
            pictura_render::AdjustmentKind::Threshold,
            pictura_render::Defaults::rgb(),
        ));
        assert_eq!(&block[..4], b"thrs");
        let edited = image_adjustment_set(&block, &QString::from("level"), 200.0, false);
        assert_eq!(decode(&edited), Some(Adjustment::Threshold(200)));
        assert!(image_adjustment_set(&block, &QString::from("nope"), 1.0, false).is_empty());
        assert!(from_block(b"thr").is_none());
        assert_eq!(rgb(0x12_34_56), [0x12, 0x34, 0x56]);
        assert!(direct("equalize").is_some() && direct("match-color").is_none());
    }

    fn vibrance_sample_layer(vivid_alpha: u8) -> pictura_core::Layer {
        let mut layer = pictura_core::Layer::default();
        layer.rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 1,
            right: 2,
        };
        layer.set_channels(
            [(0, [105, 5]), (1, [125, 240]), (2, [135, 5]), (-1, [255, vivid_alpha])]
                .into_iter()
                .map(|(id, data)| pictura_core::Channel {
                    id,
                    data: data.to_vec().into(),
                })
                .collect(),
        );
        layer
    }

    fn vibrance_sample_selection(vivid_coverage: u8) -> pictura_select::Selection {
        let channel = pictura_core::Channel {
            id: -1,
            data: vec![255, vivid_coverage].into(),
        };
        pictura_select::Selection::from_channel(&channel, 2, 1).unwrap()
    }

    #[test]
    fn vibrance_auto_excludes_transparent_and_unselected() {
        let opaque = vibrance_sample_layer(255);
        let transparent = vibrance_sample_layer(0);
        let muted_only = vibrance_sample_selection(0);
        assert_eq!(vibrance_auto_amount(&opaque, None), 0);
        assert_eq!(vibrance_auto_amount(&transparent, None), 35);
        assert_eq!(vibrance_auto_amount(&opaque, Some(&muted_only)), 35);
    }

    #[test]
    fn vibrance_auto_weights_alpha_and_selection() {
        let opaque = vibrance_sample_layer(255);
        let translucent = vibrance_sample_layer(64);
        let feathered = vibrance_sample_selection(64);
        let alpha_amount = vibrance_auto_amount(&translucent, None);
        assert!((1..35).contains(&alpha_amount));
        assert_eq!(vibrance_auto_amount(&opaque, Some(&feathered)), alpha_amount);
        assert!(vibrance_auto_amount(&translucent, Some(&feathered)) > alpha_amount);
    }

    #[test]
    fn channel_mixer_default_in_cmyk_document() {
        let block = image_adjustment_default(&QString::from("channel-mixer"), 0, 0xffffff, true);
        let Some(Adjustment::ChannelMixer(p)) = decode(&block) else {
            panic!("a channel-mixer block decodes");
        };
        assert_eq!(
            p.ink,
            [
                [100.0, 0.0, 0.0, 0.0],
                [0.0, 100.0, 0.0, 0.0],
                [0.0, 0.0, 100.0, 0.0],
                [0.0, 0.0, 0.0, 100.0],
            ]
        );
        assert_eq!(p.ink_constant, [0.0; 4]);
        assert_eq!(p.red, [100.0, 0.0, 0.0], "the RGB view is also identity");
        let page = image_adjustment_page(&block, true);
        assert!(page.iter().any(|row| row
            .to_string()
            .starts_with("groups\tCyan|Magenta|Yellow|Black")));
    }
}
