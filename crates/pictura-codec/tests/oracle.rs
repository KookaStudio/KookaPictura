//! Differential tests against an independent oracle.
//!
//! The fixtures in `tests/fixtures/` are authored by the Python `psd-tools`
//! library (`scripts/generate-fixtures.py`), not by this crate. Anything the
//! Rust codec reads here proves it agrees with a separate implementation of
//! the PSD format rather than only with its own writer.
//!
//! Regenerate the fixtures with `python3 scripts/generate-fixtures.py`.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use pictura_codec::{read_descriptor, read_psd, write_psb, write_psd, DescValue};
use pictura_core::{
    BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Compression, Document, LayerBlock,
    LayerMask, LockFlags, PsdRect, SmartObject, SmartObjectKind,
};

#[path = "oracle/support.rs"]
mod support;

pub use support::*;

#[path = "oracle/channels.rs"]
mod channels;
#[path = "oracle/effects.rs"]
mod effects;
#[path = "oracle/fills.rs"]
mod fills;
#[path = "oracle/fixtures.rs"]
mod fixtures;
#[path = "oracle/image_resources.rs"]
mod image_resources;
#[path = "oracle/imagemagick.rs"]
mod imagemagick;
#[path = "oracle/layers.rs"]
mod layers;
#[path = "oracle/opaque.rs"]
mod opaque;
#[path = "oracle/pictura_raw.rs"]
mod pictura_raw;
#[path = "oracle/smart_object.rs"]
mod smart_object;
#[path = "oracle/zip_prediction.rs"]
mod zip_prediction;

#[path = "oracle/bevel.rs"]
mod bevel;
#[path = "oracle/color_overlay.rs"]
mod color_overlay;
#[path = "oracle/gradient_overlay.rs"]
mod gradient_overlay;
#[path = "oracle/inner_glow.rs"]
mod inner_glow;
#[path = "oracle/inner_shadow.rs"]
mod inner_shadow;
#[path = "oracle/legacy.rs"]
mod legacy;
#[path = "oracle/outer_glow.rs"]
mod outer_glow;
#[path = "oracle/pattern_overlay.rs"]
mod pattern_overlay;
#[path = "oracle/satin.rs"]
mod satin;
#[path = "oracle/stroke.rs"]
mod stroke;
#[path = "oracle/stroke_gradient.rs"]
mod stroke_gradient;
#[path = "oracle/stroke_pattern.rs"]
mod stroke_pattern;
#[path = "oracle/type_engine_data.rs"]
mod type_engine_data;
