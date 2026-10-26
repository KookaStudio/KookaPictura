//! The zoomed-out filter dialog's canvas preview (#152): the visible region
//! reduced to about the canvas's device pixels and filtered there, as
//! photocraft previews large documents, so a parameter change costs what the
//! canvas shows rather than what the document holds. The document is untouched
//! until OK filters it at full resolution.

use super::super::helpers::active_mask_target;
use super::super::helpers_composite::selection_to_mask;
use super::super::state::{CanvasProxy, PictureViewRust};
#[cfg(test)]
use crate::layer_target::LayerTarget;
use cxx_qt_lib::{QList, QString, QStringList};
use pictura_core::{Document, LayerMask, PixelBuffer, PsdRect};

/// Resolve a stack: each kind takes the next `arity` slots of `params`.
pub(in crate::cxxqt_object) fn resolve_stack(
    kinds: &QStringList,
    params: &[f64],
) -> Option<Vec<pictura_filters::Filter>> {
    let mut at = 0;
    let mut filters = Vec::new();
    for kind in &QList::<QString>::from(kinds) {
        let kind = kind.to_string();
        if !crate::cxxqt_object::filter_map::is_filter_command(&kind) {
            return None;
        }
        let arity = crate::cxxqt_object::filter_map::filter_param_arity(&kind)?;
        let slots = params.get(at..at + arity)?;
        filters.push(crate::cxxqt_object::filter_map::filter_from_kind_params(
            &kind, slots,
        )?);
        at += arity;
    }
    (at == params.len()).then_some(filters)
}

/// A gallery stack's parameters with each kind's pixel-distance slots scaled for
/// a copy at `scale`; an unknown or malformed stack is returned unchanged so
/// `resolve_stack` can report it.
pub(in crate::cxxqt_object) fn scaled_stack_params(
    kinds: &QStringList,
    params: &QList<f64>,
    scale: f64,
) -> Vec<f64> {
    let params: Vec<f64> = params.into_iter().copied().collect();
    let mut scaled = Vec::with_capacity(params.len());
    let mut at = 0;
    for kind in &QList::<QString>::from(kinds) {
        let kind = kind.to_string();
        let Some(arity) = crate::cxxqt_object::filter_map::filter_param_arity(&kind) else {
            return params;
        };
        let Some(slots) = params.get(at..at + arity) else {
            return params;
        };
        scaled.extend(
            crate::cxxqt_object::filter_map::descriptor::scale_pixel_params(&kind, slots, scale),
        );
        at += arity;
    }
    if at != params.len() {
        return params;
    }
    scaled
}

/// [`super::ffi::filter_canvas_proxy`]'s render as straight sRGB, building or
/// reusing the cached reduced copy. `None` for a mask target, no active
/// layer, an empty region, or a refused filter.
pub(in crate::cxxqt_object) fn canvas_proxy_buffer(
    rust: &mut PictureViewRust,
    filter: &pictura_filters::Filter,
    region: PsdRect,
    scale: f64,
) -> Option<PixelBuffer> {
    if active_mask_target(rust).is_some() {
        return None;
    }
    let path = rust.layer_target.single().map(str::to_string)?;
    let key = (
        region,
        scale.to_bits(),
        rust.session.content_revision,
        rust.session.document_generation,
    );
    if rust
        .canvas_proxy
        .as_ref()
        .is_none_or(|cached| cached.key != key)
    {
        rust.canvas_proxy = None;
        let (source, selection) = pre_preview_sources(rust)?;
        let proxy = pictura_render::FilterProxy::new(&source, selection.as_ref(), region, scale)?;
        rust.canvas_proxy = Some(CanvasProxy { key, proxy });
    }
    let proxy = &rust.canvas_proxy.as_ref()?.proxy;
    let rendered = proxy
        .render(&path, std::slice::from_ref(filter), rust.gpu_compute)
        .ok()?;
    let doc = rust.session.doc.as_ref()?;
    Some(pictura_codec::buffer_to_srgb(doc, &rendered).into_owned())
}

/// The document as it was before any open section preview, with the
/// selection as a canvas-sized coverage mask.
pub(in crate::cxxqt_object) fn pre_preview_sources(
    rust: &PictureViewRust,
) -> Option<(Document, Option<LayerMask>)> {
    let mut doc = rust.session.doc.clone()?;
    // An open section preview has written into the live layer; the work starts
    // from the pixels before it, as the commit does.
    if let Some(preview) = rust.session.live.filter_preview() {
        if let Some(layer) = pictura_render::resolve_path_mut(&mut doc, &preview.layer_path) {
            let original = preview.baseline.original(layer).into_owned();
            *layer = original;
        }
    }
    let mask = rust
        .session
        .selection
        .as_ref()
        .map(|s| selection_to_mask(s, &doc));
    Some((doc, mask))
}

/// The document rect `section` with `filter` applied to the active layer at
/// full resolution, as straight sRGB, leaving the document alone: the section
/// plus the filter's apron is copied at scale 1 and filtered there, so the
/// section matches a commit as a section preview does.
pub(in crate::cxxqt_object) fn section_buffer(
    rust: &mut PictureViewRust,
    filter: &pictura_filters::Filter,
    section: PsdRect,
) -> Option<PixelBuffer> {
    if active_mask_target(rust).is_some() {
        return None;
    }
    let path = rust.layer_target.single().map(str::to_string)?;
    let (source, selection) = pre_preview_sources(rust)?;
    let canvas = PsdRect {
        top: 0,
        left: 0,
        bottom: source.height as i32,
        right: source.width as i32,
    };
    let apron = filter.preview_apron();
    let region = clamp(
        PsdRect {
            top: section.top - apron,
            left: section.left - apron,
            bottom: section.bottom + apron,
            right: section.right + apron,
        },
        canvas,
    );
    let section = clamp(section, region);
    if section.width() <= 0 || section.height() <= 0 {
        return None;
    }
    let proxy = pictura_render::FilterProxy::new(&source, selection.as_ref(), region, 1.0)?;
    let rendered = proxy
        .render(&path, std::slice::from_ref(filter), rust.gpu_compute)
        .ok()?;
    let (rw, rh) = (rendered.width as usize, rendered.height as usize);
    let (w, h) = (section.width() as usize, section.height() as usize);
    let (ox, oy) = (
        (section.left - region.left) as usize,
        (section.top - region.top) as usize,
    );
    let mut crop = PixelBuffer::new(w as u32, h as u32, rendered.channels);
    for c in 0..rendered.channels as usize {
        for row in 0..h {
            let from = c * rw * rh + (oy + row) * rw + ox;
            let to = c * w * h + row * w;
            crop.data[to..to + w].copy_from_slice(&rendered.data[from..from + w]);
        }
    }
    Some(pictura_codec::buffer_to_srgb(&source, &crop).into_owned())
}

/// `rect` clamped to `bounds`.
fn clamp(rect: PsdRect, bounds: PsdRect) -> PsdRect {
    PsdRect {
        top: rect.top.clamp(bounds.top, bounds.bottom),
        left: rect.left.clamp(bounds.left, bounds.right),
        bottom: rect.bottom.clamp(bounds.top, bounds.bottom),
        right: rect.right.clamp(bounds.left, bounds.right),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cxxqt_object::impl_filters::apply_filter_active_region;
    use crate::cxxqt_object::tests::pixel_layer;
    use pictura_core::{BitDepth, ColorMode, Document};
    use pictura_filters::Filter;

    fn rect(top: i32, left: i32, bottom: i32, right: i32) -> PsdRect {
        PsdRect {
            top,
            left,
            bottom,
            right,
        }
    }

    /// A 128×128 layer of diagonal stripes, so a blur changes it everywhere.
    fn striped() -> PictureViewRust {
        let mut doc = Document::new(128, 128, ColorMode::Rgb, BitDepth::Eight);
        let mut layer = pixel_layer("px", 128, 128, (0, 0, 0));
        for ch in layer.channels.iter_mut().filter(|c| c.id >= 0) {
            ch.data = (0..128 * 128)
                .map(|i| {
                    if (i % 128 + i / 128) % 8 < 4 {
                        30u8
                    } else {
                        220
                    }
                })
                .collect::<Vec<_>>()
                .into();
        }
        doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        PictureViewRust {
            session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
            layer_target: LayerTarget::only("0"),
            gpu_compute: false,
            ..Default::default()
        }
    }

    fn red(rust: &PictureViewRust) -> Vec<u8> {
        rust.session.doc.as_ref().unwrap().layers[0].channels[0]
            .data
            .to_vec()
    }

    #[test]
    fn the_proxy_filters_a_reduced_copy_and_leaves_the_document() {
        let mut rust = striped();
        let before = red(&rust);
        let blur = Filter::GaussianBlur { radius: 2.0 };
        let whole = rect(0, 0, 128, 128);
        let shown = canvas_proxy_buffer(&mut rust, &blur, whole, 0.25).expect("proxy");
        assert_eq!((shown.width, shown.height), (32, 32));
        assert_eq!(red(&rust), before, "the document is untouched");
        let unfiltered = rust
            .canvas_proxy
            .as_ref()
            .unwrap()
            .proxy
            .render("0", &[], false)
            .unwrap();
        assert_ne!(shown.data, unfiltered.data, "the copy is filtered");
    }

    #[test]
    fn the_copy_is_kept_until_its_key_changes() {
        let mut rust = striped();
        let blur = Filter::GaussianBlur { radius: 2.0 };
        let whole = rect(0, 0, 128, 128);
        let first = canvas_proxy_buffer(&mut rust, &blur, whole, 0.5).expect("proxy");
        // A change the key does not see is not picked up: the copy was reused.
        rust.session.doc.as_mut().unwrap().layers[0].channels[0]
            .data
            .as_mut_slice()
            .fill(0);
        let again = canvas_proxy_buffer(&mut rust, &blur, whole, 0.5).expect("proxy");
        assert_eq!(again.data, first.data);
        // A new document revision rebuilds it.
        rust.session.content_revision += 1;
        let rebuilt = canvas_proxy_buffer(&mut rust, &blur, whole, 0.5).expect("proxy");
        assert_ne!(rebuilt.data, first.data);
    }

    #[test]
    fn an_open_section_preview_does_not_leak_into_the_copy() {
        let blur = Filter::GaussianBlur { radius: 2.0 };
        let whole = rect(0, 0, 128, 128);
        let mut fresh = striped();
        let expected = canvas_proxy_buffer(&mut fresh, &blur, whole, 0.5).expect("proxy");

        let mut rust = striped();
        apply_filter_active_region(
            &mut rust,
            "gaussian-blur",
            &[6.0],
            false,
            Some(rect(40, 40, 80, 80)),
        )
        .expect("section preview");
        let shown = canvas_proxy_buffer(&mut rust, &blur, whole, 0.5).expect("proxy");
        assert_eq!(shown.data, expected.data);
    }

    #[test]
    fn a_mask_target_has_no_proxy() {
        let mut rust = striped();
        let layer = &mut rust.session.doc.as_mut().unwrap().layers[0];
        layer.mask = Some(pictura_core::LayerMask {
            rect: rect(0, 0, 128, 128),
            default_color: 255,
            disabled: false,
            flags: 0,
            data: Some(vec![255u8; 128 * 128].into()),
            extra: Vec::new(),
            ..Default::default()
        });
        rust.mask_edit_target = Some("0".to_string());
        let blur = Filter::GaussianBlur { radius: 2.0 };
        assert!(canvas_proxy_buffer(&mut rust, &blur, rect(0, 0, 128, 128), 0.5).is_none());
    }
}
