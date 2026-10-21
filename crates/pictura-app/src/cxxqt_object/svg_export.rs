//! `File ▸ Export As…` with the SVG format: build the document's SVG text. Free
//! functions over a [`PictureView`] (their own bridge). SVG is text, so it does
//! not go through the Qt `QImageWriter` edge; the dialog writes the returned
//! string.
//!
//! [`PictureView`]: super::qobject::PictureView

use super::helpers_composite::buffer_to_rgba_bytes;
use super::qobject::PictureView;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::Document;

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
        /// The active document serialized to SVG text, or empty without one.
        /// Never changes the document, its path, dirty flag, or history.
        fn svg_from_view(view: &PictureView) -> QString;
    }
}

pub fn svg_from_view(view: &PictureView) -> QString {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return QString::default();
    };
    QString::from(svg_for_document(doc, rust.gpu_compute).as_str())
}

/// Serialize `doc` to SVG; each raster fallback is composited through the
/// active backend.
fn svg_for_document(doc: &Document, gpu_compute: bool) -> String {
    pictura_render::encode_document_svg(doc, |scratch: &Document| {
        let buffer = pictura_render::composite_active(scratch, gpu_compute).0;
        buffer_to_rgba_bytes(&pictura_codec::buffer_to_srgb(scratch, &buffer))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, ColorMode, Layer, PsdRect};

    fn pixel_layer(name: &str, w: u32, h: u32, rgb: (u8, u8, u8)) -> Layer {
        let mut layer = Layer {
            name: name.to_string(),
            rect: PsdRect {
                left: 0,
                top: 0,
                right: w as i32,
                bottom: h as i32,
            },
            ..Default::default()
        };
        let n = (w * h) as usize;
        layer.channels = vec![
            pictura_core::Channel {
                id: 0,
                data: vec![rgb.0; n].into(),
            },
            pictura_core::Channel {
                id: 1,
                data: vec![rgb.1; n].into(),
            },
            pictura_core::Channel {
                id: 2,
                data: vec![rgb.2; n].into(),
            },
            pictura_core::Channel {
                id: -1,
                data: vec![255; n].into(),
            },
        ];
        layer
    }

    #[test]
    fn a_raster_document_serializes_to_one_image() {
        let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        doc.layers =
            pictura_core::LayerStack::from_layers(vec![pixel_layer("bg", 2, 2, (10, 20, 30))]);
        let svg = svg_for_document(&doc, false);
        assert!(svg.starts_with("<svg "), "{svg}");
        assert!(svg.contains("viewBox=\"0 0 2 2\""), "{svg}");
        assert_eq!(svg.matches("<image ").count(), 1, "{svg}");
        assert!(svg.ends_with("</svg>\n"), "{svg}");
    }

    #[test]
    fn an_empty_document_is_just_the_header() {
        let doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        let svg = svg_for_document(&doc, false);
        assert!(!svg.contains("<image "), "{svg}");
        assert!(svg.ends_with("</svg>\n"), "{svg}");
    }

    #[test]
    fn a_hidden_layer_is_skipped() {
        let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        let mut hidden = pixel_layer("hidden", 2, 2, (1, 2, 3));
        hidden.visible = false;
        doc.layers = pictura_core::LayerStack::from_layers(vec![hidden]);
        assert!(!svg_for_document(&doc, false).contains("<image "));
    }
}
