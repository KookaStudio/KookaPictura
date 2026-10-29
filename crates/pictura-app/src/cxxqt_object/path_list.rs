//! The Paths panel's document queries and edits: the saved paths and the Work
//! Path, which row is active (the path the Pen group and the path selection
//! tools edit, and the only one shown), New / Save / Rename / Duplicate /
//! Delete Path, and Fill Path, Stroke Path, and Load Path as a Selection. Free
//! functions over a [`PictureView`] (their own bridge).
//!
//! A row is named by index: `-1` the Work Path, `0..` a saved path, `-2` none.
//! Fill / Stroke / Load act on the path the path calls see (`paths::path`),
//! so the panel can point them at a shape layer's outline with
//! `path_set_layer_target`.
//!
//! [`PictureView`]: super::qobject::PictureView

use super::helpers::{combine_mode_from, feathered, rgba_from_argb};
use super::paint_tools::fills::apply;
use super::paths::{active_in, path, ActivePath};
use super::qobject::PictureView;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::path::{NamedPath, VectorPath};
use pictura_core::shape::path_coverage;
use pictura_core::PsdRect;
use pictura_paint::bucket::{self, BucketPaint};
use pictura_paint::spacing::SpacingMode;
use pictura_paint::{paint_stroke, Alpha, PaintMode, StrokeConfig, StrokeSample};
use pictura_select::Selection;
use std::borrow::Cow;

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
        /// Saved paths, in panel order; 0 without a document.
        fn paths_saved_count(view: &PictureView) -> i32;

        /// Saved path `i`'s name; empty when out of range.
        fn paths_saved_name(view: &PictureView, i: i32) -> QString;

        /// Whether the Work Path holds any subpath (CS6 lists it only then).
        fn paths_has_work_path(view: &PictureView) -> bool;

        /// The active row: -1 the Work Path, `0..` a saved path, -2 none.
        fn paths_active(view: &PictureView) -> i32;

        /// Make row `i` active (-2 hides every path). No history; emits `changed`. False when `i` names no row.
        fn paths_set_active(view: Pin<&mut PictureView>, i: i32) -> bool;

        /// Row `i`'s outline flattened for a thumbnail: per subpath its point count `n`, then `n` `x, y` pairs (a closed subpath ends on its first point). Empty when `i` names no row.
        fn paths_outline(view: &PictureView, i: i32) -> Vec<f64>;

        /// The first free default name, `"Path <n>"`.
        fn paths_next_name(view: &PictureView) -> QString;

        /// Append an empty saved path named `name` and make it active; records "New Path". Its index, or -1.
        fn paths_new(view: Pin<&mut PictureView>, name: &QString) -> i32;

        /// Save the Work Path as a saved path named `name`, which becomes active; records "Save Path". Its index, or -1 when there is no Work Path.
        fn paths_save_work(view: Pin<&mut PictureView>, name: &QString) -> i32;

        /// Rename saved path `i`; records "Rename Path". False for an empty or unchanged name.
        fn paths_rename(view: Pin<&mut PictureView>, i: i32, name: &QString) -> bool;

        /// Copy saved path `i` below itself as `"<name> copy"`, made active; records "Duplicate Path". Its index, or -1.
        fn paths_duplicate(view: Pin<&mut PictureView>, i: i32) -> i32;

        /// Delete row `i` (-1 the Work Path) and leave no row active; records "Delete Path".
        fn paths_delete(view: Pin<&mut PictureView>, i: i32) -> bool;

        /// Fill the targeted path's nonzero interior with `foreground` (`0xAARRGGBB`) on the active pixel layer, inside the selection; records "Fill Path". False when it encloses nothing or the layer cannot be painted.
        fn paths_fill(view: Pin<&mut PictureView>, foreground: u32) -> bool;

        /// Stroke every subpath of the targeted path with a round brush (`diameter` px, `hardness`, `opacity`, `flow` 0-100, `spacing` % of the diameter) in `foreground` on the active pixel layer; records "Stroke Path".
        fn paths_stroke(
            view: Pin<&mut PictureView>,
            foreground: u32,
            diameter: i32,
            hardness: i32,
            opacity: i32,
            flow: i32,
            spacing: i32,
        ) -> bool;

        /// Load the targeted path's nonzero interior as a selection, feathered by `feather` px and combined by `mode` (`"new"`, `"add"`, `"subtract"`, `"intersect"`); records "Make Selection". False when it encloses nothing.
        fn paths_make_selection(view: Pin<&mut PictureView>, feather: f64, mode: &QString) -> bool;
    }
}

/// Flattening tolerance for Stroke Path's brush samples, in document pixels.
const STROKE_TOLERANCE: f64 = 0.25;

fn saved_index(view: &PictureView, i: i32) -> Option<usize> {
    let count = view.rust().session.doc.as_ref()?.saved_paths.len();
    usize::try_from(i).ok().filter(|&i| i < count)
}

fn commit(mut view: Pin<&mut PictureView>, label: &str) {
    view.as_mut().edit(label, |_| Edit::Meta);
}

fn paths_saved_count(view: &PictureView) -> i32 {
    view.rust()
        .session
        .doc
        .as_ref()
        .map_or(0, |doc| doc.saved_paths.len() as i32)
}

fn paths_saved_name(view: &PictureView, i: i32) -> QString {
    saved_index(view, i)
        .and_then(|i| {
            Some(
                view.rust().session.doc.as_ref()?.saved_paths[i]
                    .name
                    .clone(),
            )
        })
        .map_or_else(QString::default, |name| QString::from(name.as_str()))
}

fn paths_has_work_path(view: &PictureView) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| !doc.work_path.is_empty())
}

fn paths_active(view: &PictureView) -> i32 {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return -2;
    };
    match active_in(doc, rust.active_path) {
        ActivePath::None => -2,
        ActivePath::Work if doc.work_path.is_empty() => -2,
        ActivePath::Work => -1,
        ActivePath::Saved(i) => i as i32,
    }
}

fn paths_set_active(mut view: Pin<&mut PictureView>, i: i32) -> bool {
    let active = match i {
        -2 => ActivePath::None,
        -1 if paths_has_work_path(&view) => ActivePath::Work,
        _ => match saved_index(&view, i) {
            Some(i) => ActivePath::Saved(i),
            None => return false,
        },
    };
    {
        let mut rust = view.as_mut().rust_mut();
        if rust.session.doc.is_none() {
            return false;
        }
        rust.path_on_layer = false;
        rust.active_path = active;
        // Switching rows ends any drawing session, so the Pen starts a new
        // component rather than extending the last one.
        if let Some(doc) = rust.session.doc.as_mut() {
            doc.work_path.finish_editing();
            doc.saved_paths
                .iter_mut()
                .for_each(|p| p.path.finish_editing());
        }
    }
    view.as_mut().changed();
    true
}

/// Flattening tolerance for [`paths_outline`] thumbnails, in document pixels.
const OUTLINE_TOLERANCE: f64 = 1.0;

fn paths_outline(view: &PictureView, i: i32) -> Vec<f64> {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return Vec::new();
    };
    let target = match i {
        -1 => &doc.work_path,
        _ => match saved_index(view, i) {
            Some(i) => &doc.saved_paths[i].path,
            None => return Vec::new(),
        },
    };
    flat_outline(target)
}

fn flat_outline(target: &VectorPath) -> Vec<f64> {
    let mut out = Vec::new();
    for (points, _) in target.flatten(OUTLINE_TOLERANCE) {
        out.push(points.len() as f64);
        out.extend(points.into_iter().flat_map(|(x, y)| [x, y]));
    }
    out
}

/// `"Path <n>"` for the smallest `n` not already taken.
fn next_name(names: &[NamedPath]) -> String {
    (1..)
        .map(|n| format!("Path {n}"))
        .find(|name| names.iter().all(|p| &p.name != name))
        .expect("an unbounded range has a free name")
}

fn paths_next_name(view: &PictureView) -> QString {
    let names = view
        .rust()
        .session
        .doc
        .as_ref()
        .map_or(&[][..], |doc| &doc.saved_paths[..]);
    QString::from(next_name(names).as_str())
}

/// Append `path` as a saved path named `name` (or the next default), make it
/// active, and record `label`. Its index, or -1 without a document.
fn push_saved(
    mut view: Pin<&mut PictureView>,
    name: &QString,
    path: VectorPath,
    label: &str,
) -> i32 {
    let index = {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.session.doc.as_mut() else {
            return -1;
        };
        let name = match name.to_string().trim() {
            "" => next_name(&doc.saved_paths),
            name => name.to_string(),
        };
        doc.saved_paths.push(NamedPath { name, path });
        let index = doc.saved_paths.len() - 1;
        rust.active_path = ActivePath::Saved(index);
        index
    };
    commit(view, label);
    index as i32
}

fn paths_new(view: Pin<&mut PictureView>, name: &QString) -> i32 {
    push_saved(view, name, VectorPath::default(), "New Path")
}

fn paths_save_work(mut view: Pin<&mut PictureView>, name: &QString) -> i32 {
    if !paths_has_work_path(&view) {
        return -1;
    }
    let mut work = {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.session.doc.as_mut() else {
            return -1;
        };
        std::mem::take(&mut doc.work_path)
    };
    work.finish_editing();
    push_saved(view, name, work, "Save Path")
}

fn paths_rename(mut view: Pin<&mut PictureView>, i: i32, name: &QString) -> bool {
    let name = name.to_string().trim().to_string();
    let Some(i) = saved_index(&view, i) else {
        return false;
    };
    {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.session.doc.as_mut() else {
            return false;
        };
        if name.is_empty() || doc.saved_paths[i].name == name {
            return false;
        }
        doc.saved_paths[i].name = name;
    }
    commit(view, "Rename Path");
    true
}

fn paths_duplicate(mut view: Pin<&mut PictureView>, i: i32) -> i32 {
    let Some(i) = saved_index(&view, i) else {
        return -1;
    };
    let copy = {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.session.doc.as_mut() else {
            return -1;
        };
        let mut copy = doc.saved_paths[i].clone();
        copy.name = format!("{} copy", copy.name);
        copy.path.finish_editing();
        doc.saved_paths.insert(i + 1, copy);
        rust.active_path = ActivePath::Saved(i + 1);
        i + 1
    };
    commit(view, "Duplicate Path");
    copy as i32
}

fn paths_delete(mut view: Pin<&mut PictureView>, i: i32) -> bool {
    let saved = saved_index(&view, i);
    if saved.is_none() && !(i == -1 && paths_has_work_path(&view)) {
        return false;
    }
    {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.session.doc.as_mut() else {
            return false;
        };
        match saved {
            Some(i) => {
                doc.saved_paths.remove(i);
            }
            None => doc.work_path = VectorPath::default(),
        }
        rust.active_path = ActivePath::None;
    }
    commit(view, "Delete Path");
    true
}

/// The targeted path's nonzero coverage, or `None` when it encloses nothing.
fn coverage(view: &PictureView) -> Option<Vec<u8>> {
    let target = path(view)?;
    let doc = view.rust().session.doc.as_ref()?;
    let mask = path_coverage(&target, doc.width, doc.height);
    mask.iter().any(|&v| v > 0).then_some(mask)
}

fn paths_fill(mut view: Pin<&mut PictureView>, foreground: u32) -> bool {
    let Some(mask) = coverage(&view) else {
        return false;
    };
    let paint = BucketPaint::Foreground(rgba_from_argb(foreground));
    let filled = apply(
        view.as_mut(),
        "Fill Path",
        |doc, layer, _rust, t, selection| {
            bucket::fill(
                doc,
                layer,
                &t.coverage(&mask),
                paint,
                PaintMode::Normal,
                1.0,
                selection,
                Alpha::Blend,
            )
        },
    );
    if filled {
        view.as_mut().changed();
    }
    filled
}

/// One brush sample list per subpath with at least two points (a closed
/// subpath's flattening already returns to its first point).
fn stroke_samples(target: &VectorPath) -> Vec<Vec<StrokeSample>> {
    target
        .flatten(STROKE_TOLERANCE)
        .into_iter()
        .filter(|(points, _)| points.len() >= 2)
        .map(|(points, _)| {
            points
                .into_iter()
                .map(|(x, y)| StrokeSample {
                    x: x as f32,
                    y: y as f32,
                    pressure: 1.0,
                })
                .collect()
        })
        .collect()
}

fn union(a: Option<PsdRect>, b: PsdRect) -> PsdRect {
    match a {
        None => b,
        Some(a) => PsdRect {
            top: a.top.min(b.top),
            left: a.left.min(b.left),
            bottom: a.bottom.max(b.bottom),
            right: a.right.max(b.right),
        },
    }
}

// ponytail: Stroke Path paints with a round brush from the given settings,
// not CS6's choice of any painting tool, and (like Kooka's brush strokes)
// ignores the selection.
fn paths_stroke(
    mut view: Pin<&mut PictureView>,
    foreground: u32,
    diameter: i32,
    hardness: i32,
    opacity: i32,
    flow: i32,
    spacing: i32,
) -> bool {
    let Some(target) = path(&view) else {
        return false;
    };
    let runs = stroke_samples(&target);
    if runs.is_empty() {
        return false;
    }
    let cfg = StrokeConfig {
        color: rgba_from_argb(foreground),
        diameter: diameter.clamp(1, 5000) as u32,
        hardness: hardness.clamp(0, 100) as u8,
        opacity: opacity.clamp(0, 100) as u8,
        flow: flow.clamp(0, 100) as u8,
        spacing: SpacingMode::Fixed(spacing.clamp(1, 1000) as u16),
        ..StrokeConfig::default()
    }
    .sanitized();
    let stroked = apply(
        view.as_mut(),
        "Stroke Path",
        |doc, layer, _rust, t, _selection| {
            let (ox, oy) = (t.origin.0 as f32, t.origin.1 as f32);
            runs.iter().fold(None, |dirty, samples| {
                // A mask's document sits at its rectangle, so a path drawn in
                // document space moves by the target's origin.
                let local: Cow<[StrokeSample]> = if t.is_mask {
                    Cow::Owned(
                        samples
                            .iter()
                            .map(|s| StrokeSample {
                                x: s.x - ox,
                                y: s.y - oy,
                                pressure: s.pressure,
                            })
                            .collect(),
                    )
                } else {
                    Cow::Borrowed(samples.as_slice())
                };
                match paint_stroke(doc, layer, &cfg, &local) {
                    Some(rect) => Some(union(dirty, rect)),
                    None => dirty,
                }
            })
        },
    );
    if stroked {
        view.as_mut().changed();
    }
    stroked
}

fn paths_make_selection(mut view: Pin<&mut PictureView>, feather: f64, mode: &QString) -> bool {
    let Some(mask) = coverage(&view) else {
        return false;
    };
    let Some((width, height)) = view
        .rust()
        .session
        .doc
        .as_ref()
        .map(|d| (d.width, d.height))
    else {
        return false;
    };
    let shape = feathered(
        Selection {
            width,
            height,
            data: mask.into(),
        },
        feather,
    );
    let mode = combine_mode_from(&mode.to_string());
    view.as_mut()
        .apply_selection_labeled(shape, mode, "Make Selection")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &str) -> NamedPath {
        NamedPath {
            name: name.to_string(),
            path: VectorPath::default(),
        }
    }

    #[test]
    fn next_name_takes_the_first_free_number() {
        assert_eq!(next_name(&[]), "Path 1");
        assert_eq!(next_name(&[named("Path 1"), named("Path 3")]), "Path 2");
        assert_eq!(next_name(&[named("Outline")]), "Path 1");
    }

    #[test]
    fn flat_outline_prefixes_each_subpath_with_its_point_count() {
        let mut target = VectorPath::default();
        for (x, y) in [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0)] {
            target.append_corner(x, y);
        }
        target.close_active_subpath();
        target.finish_editing();
        target.append_corner(9.0, 9.0);
        target.finish_editing();
        assert_eq!(
            flat_outline(&target),
            vec![4.0, 0.0, 0.0, 4.0, 0.0, 4.0, 4.0, 0.0, 0.0, 1.0, 9.0, 9.0],
            "a closed subpath ends on its first point once"
        );
    }

    #[test]
    fn stroke_samples_close_a_closed_subpath_and_skip_lone_anchors() {
        let mut target = VectorPath::default();
        for (x, y) in [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)] {
            target.append_corner(x, y);
        }
        target.close_active_subpath();
        target.finish_editing();
        target.append_corner(30.0, 30.0);
        target.finish_editing();
        let runs = stroke_samples(&target);
        assert_eq!(runs.len(), 1, "a one-anchor subpath has nothing to stroke");
        let run = &runs[0];
        assert_eq!((run[0].x, run[0].y), (0.0, 0.0));
        let last = run.last().unwrap();
        assert_eq!((last.x, last.y), (0.0, 0.0), "closed back to the start");
    }
}
