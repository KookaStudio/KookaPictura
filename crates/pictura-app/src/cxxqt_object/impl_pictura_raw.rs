//! Pictura Raw bridge commands.
//!
//! `apply_pictura_raw_filter` turns a plain raster layer into an embedded smart
//! object when needed, then bakes the full settings into its proxy and attaches
//! them, recording one "Pictura Raw" history state. A refusal records nothing.
//! `layer_pictura_raw_settings` reads the stored settings back as bridge text so
//! the dialog can prefill, and `pictura_raw_preview` renders a downscaled copy
//! for the dialog's live preview.

use super::helpers::*;
use super::helpers_composite::buffer_to_image;
use super::qobject;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QString};

impl qobject::PictureView {
    /// The full Pictura Raw settings attached to `path` in the bridge text form,
    /// or empty when the layer has no Pictura Raw smart filter. Read-only.
    pub fn layer_pictura_raw_settings(&self, path: &QString) -> QString {
        let settings = self
            .rust()
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::resolve_path(doc, &path.to_string()))
            .and_then(|layer| layer.smart_object.as_ref())
            .and_then(|so| {
                so.smart_filters
                    .iter()
                    .find(|filter| filter.filter_id == pictura_codec::CAMERA_RAW_FILTER_ID)
            })
            .map(|filter| pictura_codec::decode_pictura_raw_settings(&filter.options));
        match settings {
            Some(settings) => QString::from(pictura_codec::pictura_raw_to_text(&settings)),
            None => QString::default(),
        }
    }

    /// `Filter > Pictura Raw…`: apply `settings_text` to `path`, converting a
    /// plain raster pixel layer to an embedded smart object first. Records one
    /// "Pictura Raw" state on success; false (no state) without a document or
    /// target, or when the engine refuses.
    pub fn apply_pictura_raw_filter(
        mut self: Pin<&mut Self>,
        path: &QString,
        settings_text: &QString,
    ) -> bool {
        let settings = pictura_codec::pictura_raw_from_text(&settings_text.to_string());
        let path = path.to_string();
        let applied = {
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.doc.as_mut() else {
                return false;
            };
            if pictura_render::can_convert_to_smart_object(doc, &path)
                && !pictura_render::convert_to_smart_object(doc, &path)
            {
                return false;
            }
            pictura_render::apply_pictura_raw(doc, &path, &settings)
        };
        if applied {
            self.as_mut().clear_link_sets();
            let region = self
                .rust()
                .doc
                .as_ref()
                .and_then(|doc| pictura_render::resolve_path(doc, &path))
                .and_then(layer_visibility_region);
            match region {
                Some(rect) => self.as_mut().refresh_region(rect),
                None => self.as_mut().recomposite(),
            }
            self.as_mut().record("Pictura Raw");
        }
        applied
    }

    /// A downscaled preview of the layer at `path` rendered with `settings_text`,
    /// for the dialog's live preview. Read-only: the layer is never converted.
    /// A null image when there is no document or the target is ineligible.
    pub fn pictura_raw_preview(
        &self,
        path: &QString,
        settings_text: &QString,
        max_dim: i32,
    ) -> QImage {
        let settings = pictura_codec::pictura_raw_from_text(&settings_text.to_string());
        let rust = self.rust();
        let Some(doc) = rust.doc.as_ref() else {
            return QImage::default();
        };
        let Some(buffer) =
            pictura_render::preview_pictura_raw(doc, &path.to_string(), &settings, max_dim)
        else {
            return QImage::default();
        };
        // The layer is in the document working space; convert to sRGB for display.
        buffer_to_image(&pictura_codec::buffer_to_srgb(doc, &buffer))
    }
}
