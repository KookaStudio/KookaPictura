//! The filter dialog's live preview.

use pictura_core::{Layer, PsdRect};
use rayon::prelude::*;

/// The pre-filter pixels backing a live filter preview.
pub(in crate::cxxqt_object) struct FilterPreview {
    /// Panel path of the layer being previewed.
    pub(in crate::cxxqt_object) layer_path: String,
    /// What the preview overwrote.
    pub(in crate::cxxqt_object) baseline: PreviewBaseline,
    /// The layer's rect before the preview grew it by the filter's apron (#1208).
    pub(in crate::cxxqt_object) rect: PsdRect,
}

/// What a filter preview overwrote, so the next one starts from the original.
pub(in crate::cxxqt_object) enum PreviewBaseline {
    /// The whole layer, before a whole-layer preview (sharing its planes).
    Layer(Box<Layer>),
    /// The pixels of `rect` (inside the layer) in each layer-sized channel,
    /// before a section preview, which writes nothing outside it. A copy of
    /// the section alone, so the live planes are written in place and never
    /// copied whole.
    Section {
        layer_rect: PsdRect,
        rect: PsdRect,
        planes: Vec<(i16, Vec<u8>)>,
    },
}

impl PreviewBaseline {
    /// Keep what a preview over `section` (the whole layer for `None`) is
    /// about to overwrite.
    pub(in crate::cxxqt_object) fn save(
        layer: &Layer,
        section: Option<pictura_core::PsdRect>,
    ) -> Self {
        let Some(rect) = section else {
            return Self::Layer(Box::new(layer.clone()));
        };
        let lrect = layer.rect;
        let lw = lrect.width() as usize;
        let full = lw * lrect.height() as usize;
        let (ox, oy) = (
            (rect.left - lrect.left) as usize,
            (rect.top - lrect.top) as usize,
        );
        let (cw, ch) = (rect.width() as usize, rect.height() as usize);
        let planes = layer
            .channels
            .iter()
            .filter(|c| c.data.len() == full)
            .map(|c| {
                let mut saved = vec![0u8; cw * ch];
                if cw > 0 {
                    saved.par_chunks_mut(cw).enumerate().for_each(|(row, out)| {
                        let from = (oy + row) * lw + ox;
                        out.copy_from_slice(&c.data[from..from + cw]);
                    });
                }
                (c.id, saved)
            })
            .collect();
        Self::Section {
            layer_rect: lrect,
            rect,
            planes,
        }
    }

    /// Put the saved pixels back into `layer`; returns the rect restored.
    pub(in crate::cxxqt_object) fn restore(&self, layer: &mut Layer) -> pictura_core::PsdRect {
        let (layer_rect, rect, planes) = match self {
            Self::Layer(original) => {
                *layer = (**original).clone();
                return layer.rect;
            }
            Self::Section {
                layer_rect,
                rect,
                planes,
            } => (*layer_rect, *rect, planes),
        };
        let (cw, ch) = (rect.width() as usize, rect.height() as usize);
        if layer.rect != layer_rect || cw == 0 || ch == 0 {
            return rect;
        }
        let lw = layer_rect.width() as usize;
        let (ox, oy) = (
            (rect.left - layer_rect.left) as usize,
            (rect.top - layer_rect.top) as usize,
        );
        for (id, saved) in planes {
            let Some(channel) = layer.channels.iter_mut().find(|c| c.id == *id) else {
                continue;
            };
            let rows = oy * lw..(oy + ch) * lw;
            channel.data.as_mut_slice()[rows]
                .par_chunks_mut(lw)
                .zip(saved.par_chunks(cw))
                .for_each(|(out, src)| out[ox..ox + cw].copy_from_slice(src));
        }
        rect
    }

    /// The layer as it was before the preview, given the live one.
    pub(in crate::cxxqt_object) fn original<'a>(
        &'a self,
        live: &'a Layer,
    ) -> std::borrow::Cow<'a, Layer> {
        match self {
            Self::Layer(original) => std::borrow::Cow::Borrowed(original.as_ref()),
            Self::Section { .. } => {
                let mut original = live.clone();
                self.restore(&mut original);
                std::borrow::Cow::Owned(original)
            }
        }
    }
}

/// A filter dialog's live preview: either on a layer's pixels or on a layer
/// mask's coverage.
///
/// The two are the same kind of session, so the slot holds one of them, never
/// both.
pub(in crate::cxxqt_object) enum FilterPreviewSession {
    /// A preview on a layer's pixels, with the baseline it overwrote.
    Layer(FilterPreview),
    /// A preview on a mask's coverage, with the coverage it overwrote.
    Mask {
        /// The panel path of the layer whose mask is previewed.
        path: String,
        /// The owning layer before the preview, including retained native channels.
        original: Box<pictura_core::Layer>,
    },
}
