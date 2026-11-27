use super::*;

use super::super::helpers::paint_timing;
use super::super::picture::{stored_is_current, stream_picture, whole, PictureTiles};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_select::{EdgeMap, EdgePeak};

pub(super) fn magnetic_begin(mut view: Pin<&mut PictureView>, contrast: i32) -> bool {
    view.as_mut().rust_mut().begin_magnetic(contrast)
}

pub(super) fn magnetic_pen_begin(mut view: Pin<&mut PictureView>, contrast: i32) -> bool {
    view.as_mut()
        .rust_mut()
        .begin_magnetic_field(contrast, true)
}

impl super::super::state::PictureViewRust {
    /// Build and cache the edge field a magnetic lasso traces. False without a
    /// document, on a 32-bit document, or when another session owns the slot.
    pub(in crate::cxxqt_object) fn begin_magnetic(&mut self, contrast: i32) -> bool {
        self.begin_magnetic_field(contrast, false)
    }

    fn begin_magnetic_field(&mut self, contrast: i32, allow_32_bit: bool) -> bool {
        if !self.slot_allows(super::super::document_session::SessionKind::Lasso) {
            return false;
        }
        if !allow_32_bit && self.document_depth_bits() == 32 {
            return false;
        }
        let map = {
            let rust = &*self;
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            // The peak streams the picture by band and the costs read it a tile
            // at a time from a document clone, so no whole picture is built
            // (#1274). The clone shares every plane's tiles (refcounts), so it
            // costs the layer tree, not the pixels; the dense-layer heap guard
            // measures it.
            let t = std::time::Instant::now();
            let tiles = PictureTiles::new(
                std::sync::Arc::new(doc.clone()),
                stored_is_current(doc, rust),
            );
            paint_timing::record("magnetic_picture", t.elapsed());
            let t = std::time::Instant::now();
            let mut peak = EdgePeak::new(doc.width, doc.height);
            stream_picture(doc, rust, whole(doc), |_, band| peak.feed(&band));
            paint_timing::record("magnetic_edge_peak", t.elapsed());
            EdgeMap::from_tiles(
                doc.width,
                doc.height,
                contrast.clamp(1, 100) as u32,
                peak.peak(),
                std::sync::Arc::new(move |tx, ty| tiles.tile(tx, ty)),
            )
        };
        self.session
            .live
            .set_selection_gesture(super::super::document_session::SelectionGesture {
                edge_map: Some(map),
                ..Default::default()
            });
        true
    }

    /// The document's bit depth: the retained source depth when the document
    /// keeps one, else the working depth.
    pub(in crate::cxxqt_object) fn document_depth_bits(&self) -> i32 {
        let Some(doc) = self.session.doc.as_ref() else {
            return 0;
        };
        let depth = if doc.retains_source_depth() {
            doc.source.depth().unwrap_or(doc.depth)
        } else {
            doc.depth
        };
        match depth {
            pictura_core::BitDepth::Eight => 8,
            pictura_core::BitDepth::Sixteen => 16,
            pictura_core::BitDepth::ThirtyTwo => 32,
            pictura_core::BitDepth::One => 1,
        }
    }
}

pub(super) fn magnetic_trace(
    view: &PictureView,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    width: i32,
) -> Vec<i32> {
    let width = width.clamp(1, 256) as u32;
    let path = match view.rust().session.live.edge_map() {
        Some(map) => map.trace((x0, y0), (x1, y1), width),
        // An empty 0×0 field has no in-bounds pixel, so it always answers with
        // the straight segment.
        None => EdgeMap::from_buffer(&pictura_core::PixelBuffer::new(0, 0, 1), 1).trace(
            (x0, y0),
            (x1, y1),
            width,
        ),
    };
    path.into_iter().flat_map(|(x, y)| [x, y]).collect()
}

pub(super) fn magnetic_end(mut view: Pin<&mut PictureView>) {
    view.as_mut().rust_mut().session.live.clear_edge_map();
}
