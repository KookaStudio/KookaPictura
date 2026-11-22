//! Pictura Raw: typed read/write of the on-disk camera-raw smart filter.
//!
//! The on-disk form is the reference's standard camera-raw smart filter
//! (`filterID 2683`, name `"Camera Raw Filter"`) inside `SoLd.filterFX`, so
//! the reference can render it; "Pictura Raw" is only our user-facing name.
//!
//! This module is the Camera Raw adapter: the `Fltr` key map, the settings
//! codec, and the filter record. The smart-filter config edit protocol (block
//! discovery, framing, padding, typed-view sync) lives in the
//! `crate::smart_filter` edit module.
//!
//! ponytail: behavioural parity only. The `Fltr` short keys and their value
//! types are grounded on a reference Photoshop descriptor; see
//! `docs/dev/camera-raw-cc-notes.md` §Fltr key map. Document-level
//! `FXid`/`FEid`/`FMsk` render caches are not authored (the baked proxy carries
//! the pixels).

use pictura_core::{Layer, PicturaRawSettings, SmartFilter};

use crate::error::PsdError;

mod codec;
mod text;
mod xmp;

pub use codec::{decode_pictura_raw_settings, encode_pictura_raw_fltr};
pub use text::{pictura_raw_from_text, pictura_raw_to_text};
pub use xmp::{pictura_raw_from_xmp, pictura_raw_to_xmp, raw_settings_key};

#[cfg(test)]
mod tests;

// The two constants below, the `Fltr` class ID `Adobe Camera Raw Filter`, the
// `Nm  ` value, and `filterID 2683` are the reference's on-disk identifiers for the
// camera-raw smart filter. They are kept byte-for-byte for the reference
// compatibility and used nominatively; see NOTICE.md.
/// The PSD smart-filter id of the camera-raw filter.
pub const CAMERA_RAW_FILTER_ID: i32 = 2683;
/// The filter's display name as stored in `filterFXList[].Nm  `.
pub const CAMERA_RAW_FILTER_NAME: &str = "Camera Raw Filter";

/// Attach the camera-raw smart filter carrying `settings` to `layer`.
///
/// A thin wrapper over [`crate::attach_smart_filter`] with the camera-raw
/// filter id.
pub fn attach_pictura_raw_filter(
    layer: &mut Layer,
    settings: &PicturaRawSettings,
) -> Result<(), PsdError> {
    crate::smart_filter::attach_smart_filter(layer, new_filter(settings))
}

fn new_filter(settings: &PicturaRawSettings) -> SmartFilter {
    SmartFilter {
        filter_id: CAMERA_RAW_FILTER_ID,
        name: CAMERA_RAW_FILTER_NAME.to_string(),
        enabled: true,
        options: crate::descriptor::write_descriptor(&encode_pictura_raw_fltr(settings)).into(),
    }
}
