use super::*;

use super::super::display_preview::{commit_display_preview, PreviewKind};
use super::super::edit::Edit;
use super::super::presentation::{Dirty, Presentation, Rebuild};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_core::{AdjustmentData, AutoKind, AutoOptions};
use super::ffi::AutoCorrectionSettings;
use pictura_render::ParamKind;

pub struct AutoCorrectionSource(Option<pictura_render::AutoHistogram>);

pub(super) fn adjustment_auto_source(view: &PictureView, path: &QString) -> Box<AutoCorrectionSource> {
    let source = view.rust().session.doc.as_ref().and_then(|doc| {
        pictura_render::adjustment_auto_source(doc, &path.to_string())
    });
    Box::new(AutoCorrectionSource(source))
}

fn auto_options(settings: &AutoCorrectionSettings) -> AutoOptions {
    let rgb = |color: u32| [(color >> 16) as u8, (color >> 8) as u8, color as u8];
    let percent = |value: f64| if value.is_finite() { value.clamp(0.0, 49.99) } else { 0.1 };
    AutoOptions {
        algorithm: match settings.algorithm {
            0 => AutoKind::Contrast,
            2 => AutoKind::Color,
            3 => AutoKind::BrightnessContrast,
            _ => AutoKind::Tone,
        },
        snap_midtones: settings.snap && settings.algorithm == 2,
        shadow_clip: percent(settings.shadow_clip),
        highlight_clip: percent(settings.highlight_clip),
        shadow: rgb(settings.shadow),
        midtone: rgb(settings.midtone),
        highlight: rgb(settings.highlight),
    }
}

pub(super) fn adjustment_auto_block(
    source: &AutoCorrectionSource,
    opening: &[u8],
    settings: &AutoCorrectionSettings,
) -> Vec<u8> {
    source.0.as_ref().and_then(|histogram| {
        pictura_render::auto_adjustment_block(&block_data(opening)?, histogram, &auto_options(settings))
    }).as_ref().map(block_bytes).unwrap_or_default()
}

pub(super) fn image_auto_configured(
    view: Pin<&mut PictureView>,
    kind: &QString,
    settings: &AutoCorrectionSettings,
) -> bool {
    super::super::image_adjust::configured_direct(view, &kind.to_string(), auto_options(settings))
}

pub(super) fn adjustment_data(view: &PictureView, path: &str) -> Option<AdjustmentData> {
    let doc = view.rust().session.doc.as_ref()?;
    let layer = pictura_render::resolve_path(doc, path)?;
    layer
        .has_adjustment_content()
        .then(|| layer.adjustment().cloned())
        .flatten()
}

fn block_bytes(data: &AdjustmentData) -> Vec<u8> {
    data.key
        .iter()
        .copied()
        .chain(data.data.iter().copied())
        .collect()
}

fn block_data(block: &[u8]) -> Option<AdjustmentData> {
    Some(AdjustmentData {
        key: block.get(..4)?.try_into().ok()?,
        data: block.get(4..)?.to_vec().into(),
    })
}

pub(super) fn adjustment_block(view: &PictureView, path: &QString) -> Vec<u8> {
    adjustment_data(view, &path.to_string())
        .as_ref()
        .map(block_bytes)
        .unwrap_or_default()
}

/// Whether `next` may replace `current`: both must resolve to the same
/// catalogue kind. Replace Color and Match Color resolve to no kind. Auto has a
/// catalogue kind but no layer default and is not saveable.
pub(super) fn same_adjustment_kind(current: &AdjustmentData, next: &AdjustmentData) -> bool {
    match (
        pictura_render::preset_kind(current),
        pictura_render::preset_kind(next),
    ) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

pub(super) fn adjustment_set_block(
    view: Pin<&mut PictureView>,
    path: &QString,
    block: &[u8],
) -> bool {
    let Some(next) = block_data(block) else {
        return false;
    };
    edit_adjustment(view, path, |current| {
        same_adjustment_kind(current, &next).then_some(next)
    })
}

pub(super) fn adjustment_page(view: &PictureView, path: &QString) -> QStringList {
    let cmyk = document_is_cmyk(view);
    adjustment_data(view, &path.to_string())
        .and_then(|data| {
            let editor = pictura_render::adjustment_editor_in_mode(&data, cmyk)?;
            Some(page_rows(&editor, data.key == *b"thrs"))
        })
        .unwrap_or_default()
}

fn document_is_cmyk(view: &PictureView) -> bool {
    view.rust().session.doc.as_ref().is_some_and(|doc| {
        pictura_render::document_color_mode(doc) == pictura_core::ColorMode::Cmyk
    })
}

/// An editor page as the tab-separated rows [`adjustment_page`] documents
/// (shared with the Image > Adjustments dialogs).
pub(in crate::cxxqt_object) fn page_rows(
    editor: &pictura_render::AdjustmentEditor,
    threshold_histogram: bool,
) -> QStringList {
    let mut rows = QStringList::default();
    let mut push = |row: String| rows.append(QString::from(row.as_str()));
    push(format!("title\t{}", editor.title));
    if threshold_histogram {
        push("threshold-histogram".into());
    }
    if let Some(note) = editor.note {
        push(format!("note\t{note}"));
    }
    if !editor.groups.is_empty() {
        push(format!("groups\t{}", editor.groups.join("|")));
    }
    if editor.curves {
        push("curves".into());
    }
    for p in &editor.params {
        let group = p.group.map_or(-1, |g| g as i64);
        let head = format!(
            "{}\t{}\t{group}\t{}\t{}",
            p.key, p.label, p.value, p.default
        );
        push(match &p.kind {
            ParamKind::Slider {
                min,
                max,
                decimals,
                unit,
            } => {
                let mut row = format!("slider\t{head}\t{min}\t{max}\t{decimals}");
                if !unit.is_empty() {
                    row.push('\t');
                    row.push_str(unit);
                }
                row
            }
            ParamKind::Check => format!("check\t{head}"),
            ParamKind::Color => format!("color\t{head}"),
            ParamKind::Choice(options) => format!("choice\t{head}\t{}", options.join("|")),
        });
    }
    rows
}

/// Replace the adjustment at `path` with `edit`'s result and refresh the area
/// it covers. False when the path is not an adjustment or `edit` refuses.
pub(super) fn edit_adjustment(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    edit: impl FnOnce(&AdjustmentData) -> Option<AdjustmentData>,
) -> bool {
    view.as_mut().settle_pending_composite();
    let path = path.to_string();
    {
        let mut rust = view.as_mut().rust_mut();
        rust.begin_display_preview(vec![path.clone()], PreviewKind::Adjustment);
        let Some(layer) = rust
            .session
            .doc
            .as_mut()
            .and_then(|doc| pictura_render::resolve_path_mut(doc, &path))
        else {
            return false;
        };
        if !layer.has_adjustment_content() {
            return false;
        }
        let Some(next) = layer.adjustment().and_then(edit) else {
            return false;
        };
        pictura_codec::edit_layer(layer, |parts| parts.adjustment = Some(next));
    }
    if let Some(rect) = view.as_mut().rust_mut().present_display_preview() {
        view.as_mut().present(Presentation {
            regions: vec![rect],
            rebuild: Rebuild::None,
            notify: false,
            dirty: Dirty::Set(true),
            revision: true,
            declaration: None,
        });
        return true;
    }
    // The exact path runs instead: keep the pre-gesture preview state so the
    // commit can still detect a no-op and record the change.
    let saved = view.as_mut().rust_mut().display_preview.take();
    let applied = view.as_mut().preview(|rust| {
        match rust
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::damage::influence(doc, &path))
        {
            Some(rect) => Edit::regions(vec![rect], false),
            None => Edit::Full,
        }
    });
    if let Some(mut saved) = saved {
        saved.exact_fallback = true;
        view.as_mut().rust_mut().display_preview = Some(saved);
    }
    !matches!(applied, Edit::None)
}

pub(super) fn adjustment_set(
    view: Pin<&mut PictureView>,
    path: &QString,
    key: &QString,
    value: f64,
) -> bool {
    let cmyk = document_is_cmyk(&view);
    let key = key.to_string();
    edit_adjustment(view, path, move |data| {
        pictura_render::set_adjustment_param_in_mode(data, &key, value, cmyk)
    })
}

pub(in crate::cxxqt_object) fn format_points(points: &[(u8, u8)]) -> String {
    points
        .iter()
        .map(|(x, y)| format!("{x},{y}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(in crate::cxxqt_object) fn parse_points(text: &str) -> Option<Vec<(u8, u8)>> {
    text.split_whitespace()
        .map(|pair| {
            let (x, y) = pair.split_once(',')?;
            Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
        })
        .collect()
}

pub(super) fn adjustment_curve(view: &PictureView, path: &QString, channel: i32) -> QString {
    let points = adjustment_data(view, &path.to_string())
        .and_then(|data| pictura_render::curve_points(&data, usize::try_from(channel).ok()?));
    QString::from(
        points
            .map(|p| format_points(&p))
            .unwrap_or_default()
            .as_str(),
    )
}

pub(super) fn adjustment_set_curve(
    view: Pin<&mut PictureView>,
    path: &QString,
    channel: i32,
    points: &QString,
) -> bool {
    let (Ok(channel), Some(points)) = (usize::try_from(channel), parse_points(&points.to_string()))
    else {
        return false;
    };
    edit_adjustment(view, path, |data| {
        pictura_render::set_curve_points(data, channel, &points)
    })
}

pub(super) fn adjustment_reset(view: Pin<&mut PictureView>, path: &QString) -> bool {
    let cmyk = document_is_cmyk(&view);
    edit_adjustment(view, path, move |data| {
        pictura_render::reset_adjustment_in_mode(data, cmyk)
    })
}

pub(super) fn adjustment_commit(mut view: Pin<&mut PictureView>, title: &QString) {
    let label = format!("Modify {title} Layer");
    if commit_display_preview(view.as_mut(), &label).is_none() {
        // A fallback preview already composited the exact frame; capture it
        // without recompositing.
        view.as_mut().edit(&label, |_| Edit::Record);
    }
}

pub(super) fn adjustment_cancel(mut view: Pin<&mut PictureView>) {
    let exact_fallback = view.as_mut().rust_mut().cancel_display_preview();
    if exact_fallback {
        view.as_mut().preview(|_| Edit::Full);
    }
    view.as_mut().changed();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_points_round_trip_as_text() {
        let points = [(0, 0), (128, 160), (255, 255)];
        assert_eq!(format_points(&points), "0,0 128,160 255,255");
        assert_eq!(
            parse_points("0,0 128,160 255,255").as_deref(),
            Some(&points[..])
        );
        assert_eq!(parse_points("0,0 300,1"), None);
        assert_eq!(parse_points("bad"), None);
    }

    #[test]
    fn same_kind_guard_rejects_different_or_undecodable_blocks() {
        let ctx = pictura_render::Defaults::rgb();
        let threshold =
            pictura_render::dialog_default(pictura_render::AdjustmentKind::Threshold, ctx)
                .expect("threshold default");
        let posterize =
            pictura_render::dialog_default(pictura_render::AdjustmentKind::Posterize, ctx)
                .expect("posterize default");
        let garbage = AdjustmentData {
            key: *b"zzzz",
            data: vec![1, 2, 3].into(),
        };
        assert!(same_adjustment_kind(&threshold, &threshold));
        assert!(!same_adjustment_kind(&threshold, &posterize));
        assert!(!same_adjustment_kind(&threshold, &garbage));
        assert!(!same_adjustment_kind(&garbage, &garbage));
    }
}
