//! A layered document for the whole-picture reader tests (#1274): odd sizes,
//! so tiles and bands fold at the right and bottom edges; a half-covering
//! multiply layer over an opaque varied base; transparent margins; and an
//! Adobe RGB working profile, so the sRGB paths convert.

use super::super::document_session::DocumentSession;
use super::super::state::PictureViewRust;
use pictura_core::{
    BitDepth, Channel, ColorMode, Document, Layer, LayerStack, PixelBuffer, PsdRect,
};

pub(in crate::cxxqt_object) fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom,
        right,
    }
}

/// A layer over `r` whose planes vary with position and `seed`.
fn varied(name: &str, r: PsdRect, seed: u32) -> Layer {
    let (w, h) = (r.width() as u32, r.height() as u32);
    let mut layer = super::super::tests::pixel_layer(name, w, h, (0, 0, 0));
    layer.rect = r;
    for (c, channel) in layer.channels.iter_mut().enumerate() {
        let data: Vec<u8> = (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                let v = x * 7 + y * 13 + seed + c as u32 * 61;
                if channel.id == -1 {
                    (60 + v % 196) as u8
                } else {
                    (v % 251) as u8
                }
            })
            .collect();
        *channel = Channel {
            id: channel.id,
            data: data.into(),
        };
    }
    layer
}

/// The layered document, `w`×`h`, with a working profile when `profiled`.
pub(in crate::cxxqt_object) fn layered(w: u32, h: u32, profiled: bool) -> Document {
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    let (wi, hi) = (w as i32, h as i32);
    let mut base = varied("base", rect(11, 7, wi - 13, hi - 5), 3);
    for channel in &mut base.channels {
        if channel.id == -1 {
            let n = channel.data.len();
            channel.data = vec![255u8; n].into();
        }
    }
    let mut top = varied("top", rect(wi / 5, hi / 7, wi - 9, hi / 2 + 31), 41);
    top.blend = pictura_core::BlendMode::Multiply;
    doc.layers = LayerStack::from_layers(vec![base, top]);
    if profiled {
        doc.document_icc = Some(pictura_codec::Profile::adobe_rgb().to_icc().into());
    }
    doc
}

/// `doc` in a view, presented as the app shows it.
pub(in crate::cxxqt_object) fn view_of(doc: Document) -> PictureViewRust {
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    };
    view.composite_into_doc();
    view.reset_pyramid();
    view
}

/// The whole-composite reference for `view`'s document.
pub(in crate::cxxqt_object) fn full_picture(view: &PictureViewRust) -> PixelBuffer {
    pictura_render::composite_rgba(view.session.doc.as_ref().expect("document"))
}
