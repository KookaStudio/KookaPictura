//! Render a channel-less type layer's raster into its channels.

use super::{render_type, type_layer_spec, type_layers_editable};
use pictura_core::Document;

/// Render a channel-less type layer's raster into its channels, so a Move can
/// measure and preview its ink instead of rendering the whole paragraph box on
/// every press. False for a non-type layer, one that already has pixels, a run
/// set the renderer cannot reproduce, or a document that is not 8-bit RGB.
///
/// The pixels are the compositor's own type render, so the appearance does not
/// change; this is a cache, not an edit.
pub fn materialize_type_layer(doc: &mut Document, path: &str) -> bool {
    if !type_layers_editable(doc) {
        return false;
    }
    let Some(layer) = crate::resolve_path(doc, path) else {
        return false;
    };
    if layer.type_tool().is_none() || layer.channels().iter().any(|c| c.id == 0) {
        return false;
    }
    if !layer
        .type_tool()
        .is_some_and(pictura_codec::type_tool_has_uniform_runs)
    {
        return false;
    }
    let Some((rect, buffer)) = type_layer_spec(layer).and_then(|spec| render_type(&spec)) else {
        return false;
    };
    let Some(layer) = crate::resolve_path_mut(doc, path) else {
        return false;
    };
    layer.rect = rect;
    layer.set_channels(crate::type_channels::type_channels(&buffer));
    layer.discard_raw_channels();
    layer.source_channels = None;
    true
}
