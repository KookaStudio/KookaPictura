use super::*;

use super::super::helpers_composite::buffer_to_rgba_bytes;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::Document;

pub fn svg_from_view(view: &PictureView) -> QString {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return QString::default();
    };
    QString::from(svg_for_document(doc, rust.gpu_compute).as_str())
}

/// The top-level layer at `index` serialized to a document-sized SVG, or empty
/// without that layer.
pub fn svg_from_layer(view: &PictureView, index: i32) -> QString {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return QString::default();
    };
    let Ok(index) = usize::try_from(index) else {
        return QString::default();
    };
    QString::from(svg_for_layer(doc, index, rust.gpu_compute).as_str())
}

/// The testable core of [`svg_from_layer`]: serialize one top-level layer on
/// the document canvas.
pub(crate) fn svg_for_layer(doc: &Document, index: usize, gpu_compute: bool) -> String {
    let Some(layer) = doc.layers.get(index) else {
        return String::new();
    };
    let mut scratch = doc.clone();
    scratch.layers = pictura_core::LayerStack::from_layers(vec![layer.clone()]);
    svg_for_document(&scratch, gpu_compute)
}

/// Serialize `doc` to SVG; each raster fallback is composited through the
/// active backend.
pub(crate) fn svg_for_document(doc: &Document, gpu_compute: bool) -> String {
    pictura_render::encode_document_svg(doc, |scratch: &Document| {
        let buffer = pictura_render::composite(pictura_render::CompositeRequest::whole(
            scratch,
            gpu_compute,
        ))
        .0;
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

    #[test]
    fn a_layer_serializes_to_a_document_sized_svg() {
        let mut doc = Document::new(4, 3, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![
            pixel_layer("bottom", 4, 3, (10, 20, 30)),
            pixel_layer("top", 4, 3, (40, 50, 60)),
        ]);
        let svg = svg_for_layer(&doc, 1, false);
        assert!(svg.contains("viewBox=\"0 0 4 3\""), "{svg}");
        assert_eq!(svg.matches("<image ").count(), 1, "{svg}");
        let whole = svg_for_document(&doc, false);
        assert_eq!(whole.matches("<image ").count(), 2, "{whole}");
    }
}
