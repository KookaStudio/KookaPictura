//! The Properties panel's adjustment editing: the page descriptor of the
//! adjustment layer at `path`, live parameter / curve / reset edits that
//! recomposite without a history state, and the commit that records one
//! "Modify … Layer" state per gesture. Free functions over a [`PictureView`]
//! (their own bridge, so the `PictureView` declaration list does not grow).
//!
//! [`PictureView`]: super::qobject::PictureView

use super::edit::Edit;
use super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_core::AdjustmentData;
use pictura_render::ParamKind;

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
        /// The Properties page for the adjustment layer at `path`, one tab-separated row each: `title`, `note`, `groups` (`|`-joined), `curves`, then a `slider` (key, label, group, value, default, min, max, decimals), `check` / `color` (key, label, group, value, default), or `choice` (… options `|`-joined) per control; group -1 shows in every group. Empty for anything else.
        fn adjustment_page(view: &PictureView, path: &QString) -> QStringList;

        /// Set parameter `key` of the adjustment at `path` and recomposite, without a history state. False when refused.
        fn adjustment_set(
            view: Pin<&mut PictureView>,
            path: &QString,
            key: &QString,
            value: f64,
        ) -> bool;

        /// A Curves layer's `channel` (0 composite, 1-3 red, green, blue) points as `"x,y x,y …"`, or empty.
        fn adjustment_curve(view: &PictureView, path: &QString, channel: i32) -> QString;

        /// Replace a Curves layer's `channel` points (`"x,y x,y …"`) and recomposite, without a history state.
        fn adjustment_set_curve(
            view: Pin<&mut PictureView>,
            path: &QString,
            channel: i32,
            points: &QString,
        ) -> bool;

        /// Every parameter of the adjustment at `path` back to its default, without a history state.
        fn adjustment_reset(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Record the edits since the last state as one "Modify `title` Layer" state.
        fn adjustment_commit(view: Pin<&mut PictureView>, title: &QString);
    }
}

fn adjustment_data(view: &PictureView, path: &str) -> Option<AdjustmentData> {
    let doc = view.rust().session.doc.as_ref()?;
    pictura_render::resolve_path(doc, path)?.adjustment.clone()
}

fn adjustment_page(view: &PictureView, path: &QString) -> QStringList {
    adjustment_data(view, &path.to_string())
        .as_ref()
        .and_then(pictura_render::adjustment_editor)
        .map(|editor| page_rows(&editor))
        .unwrap_or_default()
}

/// An editor page as the tab-separated rows [`adjustment_page`] documents
/// (shared with the Image > Adjustments dialogs).
pub(super) fn page_rows(editor: &pictura_render::AdjustmentEditor) -> QStringList {
    let mut rows = QStringList::default();
    let mut push = |row: String| rows.append(QString::from(row.as_str()));
    push(format!("title\t{}", editor.title));
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
            ParamKind::Slider { min, max, decimals } => {
                format!("slider\t{head}\t{min}\t{max}\t{decimals}")
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
fn edit_adjustment(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    edit: impl FnOnce(&AdjustmentData) -> Option<AdjustmentData>,
) -> bool {
    let path = path.to_string();
    let applied = view.as_mut().preview(|rust| {
        let Some(layer) = rust
            .session
            .doc
            .as_mut()
            .and_then(|doc| pictura_render::resolve_path_mut(doc, &path))
        else {
            return Edit::None;
        };
        let Some(next) = layer.adjustment.as_ref().and_then(edit) else {
            return Edit::None;
        };
        layer.adjustment = Some(next);
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
    !matches!(applied, Edit::None)
}

fn adjustment_set(view: Pin<&mut PictureView>, path: &QString, key: &QString, value: f64) -> bool {
    let key = key.to_string();
    edit_adjustment(view, path, |data| {
        pictura_render::set_adjustment_param(data, &key, value)
    })
}

pub(super) fn format_points(points: &[(u8, u8)]) -> String {
    points
        .iter()
        .map(|(x, y)| format!("{x},{y}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn parse_points(text: &str) -> Option<Vec<(u8, u8)>> {
    text.split_whitespace()
        .map(|pair| {
            let (x, y) = pair.split_once(',')?;
            Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
        })
        .collect()
}

fn adjustment_curve(view: &PictureView, path: &QString, channel: i32) -> QString {
    let points = adjustment_data(view, &path.to_string())
        .and_then(|data| pictura_render::curve_points(&data, usize::try_from(channel).ok()?));
    QString::from(
        points
            .map(|p| format_points(&p))
            .unwrap_or_default()
            .as_str(),
    )
}

fn adjustment_set_curve(
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

fn adjustment_reset(view: Pin<&mut PictureView>, path: &QString) -> bool {
    edit_adjustment(view, path, pictura_render::reset_adjustment)
}

fn adjustment_commit(mut view: Pin<&mut PictureView>, title: &QString) {
    let label = format!("Modify {title} Layer");
    view.as_mut().edit(&label, |_| Edit::Record);
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
}
