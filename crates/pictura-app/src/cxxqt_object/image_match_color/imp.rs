use super::*;

use super::super::impl_filters::{apply_op_active_region, cancel_filter_preview, ActiveOp};
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{Plane, PsdRect};
use pictura_paint::stamp::{layer_surface, surface_from_composite};
use pictura_render::{
    match_color_stats, Adjustment, MatchColorAccumulator, MatchColorParams, MatchColorStats,
};

use super::super::picture::{stream_picture, whole};

use super::ffi::QList_f64;

/// The document-sized selection coverage plane of `view`, when its selection
/// matches the document, as a tile-sharing clone. Match Color's statistics
/// weight pixels by it.
pub(super) fn selection_plane(view: &PictureView, use_selection: bool) -> Option<Plane<u8>> {
    if !use_selection {
        return None;
    }
    let rust = view.rust();
    let doc = rust.session.doc.as_ref()?;
    rust.session
        .selection
        .as_ref()
        .filter(|s| s.width == doc.width && s.height == doc.height)
        .map(|s| s.data.clone())
}

pub(super) fn stats_vec(stats: MatchColorStats) -> Vec<f64> {
    stats.mean.iter().chain(stats.std.iter()).copied().collect()
}

pub(super) fn match_color_target_stats(view: &PictureView, use_selection: bool) -> Vec<f64> {
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
    stats_vec(surface_stats(
        &image.data,
        selection_plane(view, use_selection).as_ref(),
    ))
}

pub(super) fn match_color_source_stats(
    source: &PictureView,
    path: &QString,
    use_selection: bool,
) -> Vec<f64> {
    let rust = source.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return Vec::new();
    };
    let selection = selection_plane(source, use_selection);
    if path.is_empty() {
        return stats_vec(picture_stats(doc, rust, selection.as_ref()));
    }
    let Some(image) = layer_surface(doc, &path.to_string()) else {
        return Vec::new();
    };
    stats_vec(surface_stats(&image.data, selection.as_ref()))
}

/// The coverage of `plane`'s samples `at..at + n`, clipped to its end, read
/// out of the tiles (#1268).
fn coverage_span(plane: &Plane<u8>, at: usize, n: usize) -> Vec<u8> {
    let mut out = vec![0u8; n.min(plane.len().saturating_sub(at))];
    plane.read_into(at, &mut out);
    out
}

/// [`match_color_stats`] of document-sized `pixels` under `selection`, the
/// coverage read a piece at a time: the pieces sum in pixel order, so the
/// result is bit-identical to one pass over a contiguous coverage.
fn surface_stats(pixels: &[[u8; 4]], selection: Option<&Plane<u8>>) -> MatchColorStats {
    const PIECE: usize = 1 << 16;
    let Some(plane) = selection else {
        return match_color_stats(pixels, None);
    };
    let mut stats = MatchColorAccumulator::default();
    for (k, piece) in pixels.chunks(PIECE).enumerate() {
        stats.add(piece, Some(&coverage_span(plane, k * PIECE, piece.len())));
    }
    stats.finish()
}

/// Match Color's statistics of the picture, `selection` its coverage: band by
/// band in pixel order, so they equal one pass over the whole picture bit for
/// bit and no whole picture is built (#1274).
pub(super) fn picture_stats(
    doc: &pictura_core::Document,
    view: &crate::cxxqt_object::state::PictureViewRust,
    selection: Option<&Plane<u8>>,
) -> MatchColorStats {
    let w = doc.width as usize;
    let mut stats = MatchColorAccumulator::default();
    stream_picture(doc, view, whole(doc), |offset, band| {
        let pixels = surface_from_composite(&band).data;
        let at = offset as usize * w;
        let coverage = selection.map(|plane| coverage_span(plane, at, pixels.len()));
        stats.add(&pixels, coverage.as_deref());
    });
    stats.finish()
}

/// The six statistics doubles, or `None` (with `filter_error` set) when the
/// dialog handed over a malformed vector — it cannot happen through the
/// bridge's own stats calls.
pub(super) fn parse_stats(values: &[f64]) -> Option<MatchColorStats> {
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
pub(super) fn params(
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
pub(super) fn run(
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
pub(super) fn image_match_color_preview(
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
pub(super) fn image_match_color_apply(
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

#[cfg(test)]
mod picture_stats_tests {
    use super::*;
    use crate::cxxqt_object::picture::readers_fixture::{full_picture, layered, view_of};

    /// The source statistics of the streamed picture equal one pass over the
    /// whole composite bit for bit, with and without a selection (#1274).
    #[test]
    fn match_color_stats_stream() {
        let view = view_of(layered(613, 541, false));
        let doc = view.session.doc.as_ref().expect("document");
        let whole_pixels = surface_from_composite(&full_picture(&view)).data;
        let coverage: Vec<u8> = (0..whole_pixels.len())
            .map(|i| (i * 7 % 256) as u8)
            .collect();
        let plane = Plane::from(coverage.clone());
        for (selection, plane) in [(None, None), (Some(&coverage[..]), Some(&plane))] {
            let streamed = picture_stats(doc, &view, plane);
            let want = match_color_stats(&whole_pixels, selection);
            assert_eq!(streamed.mean.map(f64::to_bits), want.mean.map(f64::to_bits));
            assert_eq!(streamed.std.map(f64::to_bits), want.std.map(f64::to_bits));
        }
    }
}
