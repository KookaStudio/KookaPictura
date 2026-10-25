//! The Vanishing Point planes in a saved file: the project's own image
//! resource (`pictura_codec::VANISHING_POINT`), which PSD, PSB, TIFF and JPEG
//! carry. The session's planes stay the live copy; a save writes them out and
//! installing a document reads them back.

use pictura_core::Document;
use pictura_filters::VpPlanes;

/// The planes `doc`'s file kept, when they decode and fit its canvas.
pub(in crate::cxxqt_object) fn saved_planes(doc: &Document) -> Option<VpPlanes> {
    pictura_codec::vanishing_point_data(doc)
        .and_then(|data| VpPlanes::from_bytes(&data))
        .filter(|planes| !planes.is_empty() && planes.size() == (doc.width, doc.height))
}

/// The resource data to save `planes` with `doc`; `None` when there are
/// none or they were drawn on a canvas of another size, so a save drops a
/// block the planes no longer back.
pub(in crate::cxxqt_object) fn store_planes(
    planes: Option<&VpPlanes>,
    doc: &Document,
) -> Option<Vec<u8>> {
    planes
        .filter(|p| !p.is_empty() && p.size() == (doc.width, doc.height))
        .map(VpPlanes::to_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, ColorMode};

    #[test]
    fn saved_planes_must_fit_the_canvas() {
        let mut doc = Document::new(100, 80, ColorMode::Rgb, BitDepth::Eight);
        assert!(saved_planes(&doc).is_none());
        let mut planes = VpPlanes::new(100, 80);
        assert_eq!(
            store_planes(Some(&planes), &doc),
            None,
            "no planes, no block"
        );
        planes.add([(10.0, 10.0), (90.0, 10.0), (90.0, 70.0), (10.0, 70.0)]);
        let data = store_planes(Some(&planes), &doc);
        pictura_codec::set_vanishing_point_data(&mut doc, data.as_deref());
        assert_eq!(saved_planes(&doc).as_ref(), Some(&planes));
        let mut resized = Document::new(120, 80, ColorMode::Rgb, BitDepth::Eight);
        resized.image_resources = doc.image_resources.clone();
        assert!(
            saved_planes(&resized).is_none(),
            "planes for another canvas size"
        );
        assert_eq!(
            store_planes(Some(&planes), &resized),
            None,
            "not saved after a resize"
        );
    }
}
