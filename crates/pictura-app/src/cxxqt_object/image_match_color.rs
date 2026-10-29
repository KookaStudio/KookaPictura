//! Image > Adjustments > Match Color (#352): statistics gathered over whole
//! layers and documents, a Lab mean/std transfer previewed on the canvas
//! through the shared adjustment preview machinery, and committed as one
//! "Match Color" state. Free functions over a [`PictureView`] (their own
//! bridge), like Replace Color.
//!
//! [`PictureView`]: super::qobject::PictureView

use super::impl_filters::{apply_op_active_region, cancel_filter_preview, ActiveOp};
use super::qobject::PictureView;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::PsdRect;
use pictura_paint::stamp::{layer_surface, surface_from_composite};
use pictura_render::{match_color_stats, Adjustment, MatchColorParams, MatchColorStats};

use super::picture::picture;

use self::ffi::QList_f64;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qlist.h");
        type QList_f64 = cxx_qt_lib::QList<f64>;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The target's Lab statistics — mean then standard deviation, three
        /// channels each — over the whole active layer, restricted to the
        /// selection when `use_selection` is set. Empty when there is no
        /// document or no pixel layer active.
        fn match_color_target_stats(view: &PictureView, use_selection: bool) -> Vec<f64>;

        /// A source document's statistics over its merged composite (an empty
        /// `path`) or one pixel layer (the panel path in `path`), restricted
        /// to that document's selection when `use_selection` is set. Empty when
        /// the document or layer is missing.
        fn match_color_source_stats(
            source: &PictureView,
            path: &QString,
            use_selection: bool,
        ) -> Vec<f64>;

        /// Preview Match Color on the active layer over the document rect
        /// `(x, y, w, h)` (the whole layer when `w` is not positive),
        /// re-applied from the pre-preview pixels; no history. `target` and
        /// `source` are the statistics vectors from the stats calls. False
        /// when refused.
        #[allow(clippy::too_many_arguments)]
        fn image_match_color_preview(
            view: Pin<&mut PictureView>,
            target: &QList_f64,
            source: &QList_f64,
            ignore_selection: bool,
            neutralize: bool,
            luminance: f64,
            color_intensity: f64,
            fade: f64,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
        ) -> bool;

        /// Apply Match Color to the whole active layer as one "Match Color"
        /// state (an open preview is replaced). False when refused.
        #[allow(clippy::too_many_arguments)]
        fn image_match_color_apply(
            view: Pin<&mut PictureView>,
            target: &QList_f64,
            source: &QList_f64,
            ignore_selection: bool,
            neutralize: bool,
            luminance: f64,
            color_intensity: f64,
            fade: f64,
        ) -> bool;
    }
}

/// The document-sized selection coverage plane of `view`, when its selection
/// matches the document. Match Color's statistics weight pixels by it.
fn selection_plane(view: &PictureView, use_selection: bool) -> Option<Vec<u8>> {
    if !use_selection {
        return None;
    }
    let rust = view.rust();
    let doc = rust.session.doc.as_ref()?;
    rust.session
        .selection
        .as_ref()
        .filter(|s| s.width == doc.width && s.height == doc.height)
        .map(|s| s.data.to_vec())
}

fn stats_vec(stats: MatchColorStats) -> Vec<f64> {
    stats.mean.iter().chain(stats.std.iter()).copied().collect()
}

fn match_color_target_stats(view: &PictureView, use_selection: bool) -> Vec<f64> {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return Vec::new();
    };
    // Statistics cover the whole layer being adjusted, never a preview
    // section, so the numbers cannot change as the view scrolls.
    let Some(path) = rust.layer_target.single() else {
        return Vec::new();
    };
    let Some(image) = layer_surface(doc, path) else {
        return Vec::new();
    };
    stats_vec(match_color_stats(
        &image.data,
        selection_plane(view, use_selection).as_deref(),
    ))
}

fn match_color_source_stats(source: &PictureView, path: &QString, use_selection: bool) -> Vec<f64> {
    let rust = source.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return Vec::new();
    };
    let image = if path.is_empty() {
        surface_from_composite(&picture(doc, rust))
    } else {
        match layer_surface(doc, &path.to_string()) {
            Some(image) => image,
            None => return Vec::new(),
        }
    };
    stats_vec(match_color_stats(
        &image.data,
        selection_plane(source, use_selection).as_deref(),
    ))
}

/// The six statistics doubles, or `None` (with `filter_error` set) when the
/// dialog handed over a malformed vector — it cannot happen through the
/// bridge's own stats calls.
fn parse_stats(values: &[f64]) -> Option<MatchColorStats> {
    if values.len() != 6 || values.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let mut mean = [0.0f64; 3];
    let mut std = [0.0f64; 3];
    mean.copy_from_slice(&values[..3]);
    std.copy_from_slice(&values[3..]);
    Some(MatchColorStats { mean, std })
}

#[allow(clippy::too_many_arguments)]
fn params(
    target: MatchColorStats,
    source: MatchColorStats,
    neutralize: bool,
    luminance: f64,
    color_intensity: f64,
    fade: f64,
) -> MatchColorParams {
    MatchColorParams {
        target,
        source,
        neutralize,
        luminance,
        color_intensity,
        fade,
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    mut view: Pin<&mut PictureView>,
    p: MatchColorParams,
    ignore_selection: bool,
    preview: Option<PsdRect>,
    label: Option<&str>,
) -> bool {
    let commit = label.is_some();
    let region = {
        let mut rust = view.as_mut().rust_mut();
        apply_op_active_region(
            &mut rust,
            &ActiveOp::Adjustment(Adjustment::MatchColor(p)),
            commit,
            preview,
            ignore_selection,
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

#[allow(clippy::too_many_arguments)]
fn image_match_color_preview(
    mut view: Pin<&mut PictureView>,
    target: &QList_f64,
    source: &QList_f64,
    ignore_selection: bool,
    neutralize: bool,
    luminance: f64,
    color_intensity: f64,
    fade: f64,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> bool {
    let target_v: Vec<f64> = target.into_iter().copied().collect();
    let source_v: Vec<f64> = source.into_iter().copied().collect();
    let (Some(target), Some(source)) = (parse_stats(&target_v), parse_stats(&source_v)) else {
        view.as_mut().rust_mut().filter_error =
            Some("the Match Color statistics are malformed".to_string());
        return false;
    };
    // The transfer is pointwise and the statistics cover the whole layer, so
    // restricting the preview to the visible section is exact.
    let section = (w > 0 && h > 0).then_some(PsdRect {
        top: y,
        left: x,
        bottom: y + h,
        right: x + w,
    });
    run(
        view,
        params(target, source, neutralize, luminance, color_intensity, fade),
        ignore_selection,
        section,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn image_match_color_apply(
    mut view: Pin<&mut PictureView>,
    target: &QList_f64,
    source: &QList_f64,
    ignore_selection: bool,
    neutralize: bool,
    luminance: f64,
    color_intensity: f64,
    fade: f64,
) -> bool {
    let target_v: Vec<f64> = target.into_iter().copied().collect();
    let source_v: Vec<f64> = source.into_iter().copied().collect();
    let (Some(target), Some(source)) = (parse_stats(&target_v), parse_stats(&source_v)) else {
        view.as_mut().rust_mut().filter_error =
            Some("the Match Color statistics are malformed".to_string());
        return false;
    };
    run(
        view,
        params(target, source, neutralize, luminance, color_intensity, fade),
        ignore_selection,
        None,
        Some("Match Color"),
    )
}

#[cfg(test)]
mod tests {
    use super::parse_stats;

    #[test]
    fn stats_vectors_round_trip() {
        let vec = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let stats = parse_stats(&vec).expect("six finite values parse");
        assert_eq!(stats.mean, [1.0, 2.0, 3.0]);
        assert_eq!(stats.std, [4.0, 5.0, 6.0]);
    }

    #[test]
    fn malformed_stats_are_refused() {
        assert!(parse_stats(&[]).is_none());
        assert!(parse_stats(&[0.0; 5]).is_none());
        assert!(parse_stats(&[0.0; 7]).is_none());
        assert!(parse_stats(&[f64::NAN, 0.0, 0.0, 0.0, 0.0, 0.0]).is_none());
        assert!(parse_stats(&[0.0, f64::INFINITY, 0.0, 0.0, 0.0, 0.0]).is_none());
    }
}
