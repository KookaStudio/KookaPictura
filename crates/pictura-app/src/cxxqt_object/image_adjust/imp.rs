use super::*;

use super::super::adjustment_edit::{format_points, page_rows, parse_points};
use super::super::impl_filters::{apply_op_active_region, cancel_filter_preview, ActiveOp};
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_core::{AdjustmentData, PsdRect};
use pictura_render::{Adjustment, AutoKind};

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
) -> Vec<u8> {
    to_block(pictura_render::default_adjustment_block(
        &kind.to_string(),
        rgb(foreground),
        rgb(background),
    ))
}

pub(super) fn image_adjustment_page(block: &[u8]) -> QStringList {
    from_block(block)
        .as_ref()
        .and_then(pictura_render::adjustment_editor)
        .map(|editor| page_rows(&editor))
        .unwrap_or_default()
}

pub(super) fn image_adjustment_set(block: &[u8], key: &QString, value: f64) -> Vec<u8> {
    to_block(
        from_block(block)
            .and_then(|d| pictura_render::set_adjustment_param(&d, &key.to_string(), value)),
    )
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

/// A dialog-less command's adjustment and history name.
pub(super) fn direct(kind: &str) -> Option<(Adjustment, &'static str)> {
    Some(match kind {
        "invert" => (Adjustment::Invert, "Invert"),
        "desaturate" => (Adjustment::Desaturate, "Desaturate"),
        "equalize" => (Adjustment::Equalize, "Equalize"),
        "auto-tone" => (Adjustment::Auto(AutoKind::Tone), "Auto Tone"),
        "auto-contrast" => (Adjustment::Auto(AutoKind::Contrast), "Auto Contrast"),
        "auto-color" => (Adjustment::Auto(AutoKind::Color), "Auto Color"),
        _ => return None,
    })
}

pub(super) fn image_adjust_direct(view: Pin<&mut PictureView>, kind: &QString) -> bool {
    match direct(&kind.to_string()) {
        Some((adjustment, label)) => run(view, adjustment, None, Some(label)),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_round_trip_and_edit() {
        let block = to_block(pictura_render::default_adjustment_block(
            "threshold",
            [0; 3],
            [255; 3],
        ));
        assert_eq!(&block[..4], b"thrs");
        let edited = image_adjustment_set(&block, &QString::from("level"), 200.0);
        assert_eq!(decode(&edited), Some(Adjustment::Threshold(200)));
        assert!(image_adjustment_set(&block, &QString::from("nope"), 1.0).is_empty());
        assert!(from_block(b"thr").is_none());
        assert_eq!(rgb(0x12_34_56), [0x12, 0x34, 0x56]);
        assert!(direct("equalize").is_some() && direct("match-color").is_none());
    }
}
