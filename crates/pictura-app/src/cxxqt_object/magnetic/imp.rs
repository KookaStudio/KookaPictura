use super::*;

use super::super::picture::picture;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_select::EdgeMap;

pub(super) fn magnetic_begin(mut view: Pin<&mut PictureView>, contrast: i32) -> bool {
    view.as_mut().rust_mut().begin_magnetic(contrast)
}

impl super::super::state::PictureViewRust {
    /// Build and cache the edge field a magnetic lasso traces. False without a
    /// document, on a 32-bit document, or when another session owns the slot.
    pub(in crate::cxxqt_object) fn begin_magnetic(&mut self, contrast: i32) -> bool {
        if !self.slot_allows(super::super::document_session::SessionKind::Lasso) {
            return false;
        }
        if self.document_depth_bits() == 32 {
            return false;
        }
        let map = {
            let rust = &*self;
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            EdgeMap::from_buffer(&picture(doc, rust), contrast.clamp(1, 100) as u32)
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
            doc.source_depth.unwrap_or(doc.depth)
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
