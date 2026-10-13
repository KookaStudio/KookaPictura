//! Pictura Raw bridge commands.
//!
//! `apply_pictura_raw_filter` turns a plain raster layer into an embedded smart
//! object when needed, then bakes the 11 PV2012 Basic controls into its proxy and
//! attaches them, recording one "Pictura Raw" history state. A refusal
//! records nothing. `layer_pictura_raw_settings` reads the stored values back so the
//! dialog can prefill.

use super::helpers::*;
use super::qobject;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{CrsSettings, PicturaRawSettings};

impl qobject::PictureView {
    /// The Pictura Raw Basic settings attached to `path`, as 11 space-separated
    /// values in dialog order (temperature, tint, exposure, contrast,
    /// highlights, shadows, whites, blacks, clarity, vibrance, saturation), or
    /// empty when the layer has no Pictura Raw smart filter. Read-only.
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
        let Some(settings) = settings else {
            return QString::default();
        };
        let v = |value: Option<f64>| value.unwrap_or(0.0);
        let b = &settings.basic;
        QString::from(format!(
            "{} {} {} {} {} {} {} {} {} {} {}",
            v(b.temperature),
            v(b.tint),
            v(b.exposure),
            v(b.contrast),
            v(b.highlights),
            v(b.shadows),
            v(b.whites),
            v(b.blacks),
            v(b.clarity),
            v(b.vibrance),
            v(b.saturation),
        ))
    }

    /// `Filter > Pictura Raw…`: apply the 11 Basic controls to `path`,
    /// converting a plain raster pixel layer to an embedded smart object first.
    /// Records one "Pictura Raw" state on success; false (no state) without
    /// a document or target, or when the engine refuses.
    pub fn apply_pictura_raw_filter(
        mut self: Pin<&mut Self>,
        path: &QString,
        temperature: f64,
        tint: f64,
        exposure: f64,
        contrast: f64,
        highlights: f64,
        shadows: f64,
        whites: f64,
        blacks: f64,
        clarity: f64,
        vibrance: f64,
        saturation: f64,
    ) -> bool {
        let settings = PicturaRawSettings {
            basic: CrsSettings {
                temperature: Some(temperature),
                tint: Some(tint),
                exposure: Some(exposure),
                contrast: Some(contrast),
                highlights: Some(highlights),
                shadows: Some(shadows),
                whites: Some(whites),
                blacks: Some(blacks),
                clarity: Some(clarity),
                vibrance: Some(vibrance),
                saturation: Some(saturation),
            },
            ..Default::default()
        };
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
}
