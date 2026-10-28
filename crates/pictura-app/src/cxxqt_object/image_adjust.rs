//! Image > Adjustments: destructive adjustments of the active pixel layer,
//! within the selection. A dialog edits a scratch adjustment block (the 4-byte
//! key then the payload, the encoding an adjustment layer stores) through the
//! same descriptor pages the Properties panel uses, previews it on the canvas
//! through the Filter menu's preview machinery (cancelled by
//! `filter_preview_cancel`), and applies it as one state named for the
//! adjustment. Invert, Desaturate, Equalize, and the Auto commands apply
//! directly. Free functions over a [`PictureView`] (their own bridge).
//!
//! [`PictureView`]: super::qobject::PictureView

mod gradient_map;
mod image_mode;
mod image_ops;
mod image_size;
mod new_document;

use super::adjustment_edit::{format_points, page_rows, parse_points};
use super::impl_filters::{apply_op_active_region, cancel_filter_preview, ActiveOp};
use super::qobject::PictureView;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_core::{AdjustmentData, PsdRect};
use pictura_render::{Adjustment, AutoKind};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The block the `kind` dialog opens on (`"levels"`, `"curves"`, …; colours `0xRRGGBB` for Gradient Map), or empty for an unknown kind.
        fn image_adjustment_default(kind: &QString, foreground: u32, background: u32) -> Vec<u8>;

        /// `block`'s descriptor page (the Properties panel's row format), or empty.
        fn image_adjustment_page(block: &[u8]) -> QStringList;

        /// `block` with parameter `key` set to `value`, or empty when refused.
        fn image_adjustment_set(block: &[u8], key: &QString, value: f64) -> Vec<u8>;

        /// A Curves `block`'s `channel` points as `"x,y x,y …"`, or empty.
        fn image_adjustment_curve(block: &[u8], channel: i32) -> QString;

        /// A Curves `block` with `channel`'s points replaced, or empty when refused.
        fn image_adjustment_set_curve(block: &[u8], channel: i32, points: &QString) -> Vec<u8>;

        /// Preview `block` on the active layer over the document rect `(x, y, w, h)` (the whole layer when `w` is not positive), re-applied from the pre-preview pixels; no history. False when refused.
        fn image_adjust_preview(
            view: Pin<&mut PictureView>,
            block: &[u8],
            x: i32,
            y: i32,
            w: i32,
            h: i32,
        ) -> bool;

        /// Apply `block` to the whole active layer as one `label` state (an open preview is replaced). False when refused.
        fn image_adjust_apply(view: Pin<&mut PictureView>, block: &[u8], label: &QString) -> bool;

        /// Apply a dialog-less command — `"invert"`, `"desaturate"`, `"equalize"`, `"auto-tone"`, `"auto-contrast"`, `"auto-color"` — as one state named as CS6 names it. False when refused.
        fn image_adjust_direct(view: Pin<&mut PictureView>, kind: &QString) -> bool;
    }
}

fn to_block(data: Option<AdjustmentData>) -> Vec<u8> {
    data.map(|d| d.key.iter().copied().chain(d.data).collect())
        .unwrap_or_default()
}

fn from_block(block: &[u8]) -> Option<AdjustmentData> {
    Some(AdjustmentData {
        key: block.get(..4)?.try_into().ok()?,
        data: block.get(4..)?.to_vec(),
    })
}

fn rgb(c: u32) -> [u8; 3] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

fn image_adjustment_default(kind: &QString, foreground: u32, background: u32) -> Vec<u8> {
    to_block(pictura_render::default_adjustment_block(
        &kind.to_string(),
        rgb(foreground),
        rgb(background),
    ))
}

fn image_adjustment_page(block: &[u8]) -> QStringList {
    from_block(block)
        .as_ref()
        .and_then(pictura_render::adjustment_editor)
        .map(|editor| page_rows(&editor))
        .unwrap_or_default()
}

fn image_adjustment_set(block: &[u8], key: &QString, value: f64) -> Vec<u8> {
    to_block(
        from_block(block)
            .and_then(|d| pictura_render::set_adjustment_param(&d, &key.to_string(), value)),
    )
}

fn image_adjustment_curve(block: &[u8], channel: i32) -> QString {
    let points = from_block(block)
        .and_then(|d| pictura_render::curve_points(&d, usize::try_from(channel).ok()?));
    QString::from(
        points
            .map(|p| format_points(&p))
            .unwrap_or_default()
            .as_str(),
    )
}

fn image_adjustment_set_curve(block: &[u8], channel: i32, points: &QString) -> Vec<u8> {
    let edited = (|| {
        let points = parse_points(&points.to_string())?;
        let channel = usize::try_from(channel).ok()?;
        pictura_render::set_curve_points(&from_block(block)?, channel, &points)
    })();
    to_block(edited)
}

/// Run `adjustment` on the active layer (a preview over `region` when given)
/// and refresh the canvas; records `label` on a commit.
fn run(
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

fn decode(block: &[u8]) -> Option<Adjustment> {
    pictura_render::decode_adjustment(&from_block(block)?)
}

fn image_adjust_preview(
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

fn image_adjust_apply(view: Pin<&mut PictureView>, block: &[u8], label: &QString) -> bool {
    match decode(block) {
        Some(adjustment) => run(view, adjustment, None, Some(&label.to_string())),
        None => false,
    }
}

/// A dialog-less command's adjustment and history name.
fn direct(kind: &str) -> Option<(Adjustment, &'static str)> {
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

fn image_adjust_direct(view: Pin<&mut PictureView>, kind: &QString) -> bool {
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
