//! Camera raw opens (#400): File ▸ Open of a DNG or CR2 decodes on a worker,
//! leaves the decoded source on the view for the Pictura Raw dialog, then
//! develops it at full size on a worker into a new 16-bit RGB document.

mod imp;
mod store;
#[cfg(test)]
mod store_tests;
#[cfg(test)]
mod tests;

use super::super::qobject::PictureView;
pub(in crate::cxxqt_object) use imp::PendingRaw;
pub use imp::{
    open_raw_develop_start, open_raw_start, raw_as_shot_temperature, raw_as_shot_tint, raw_preview,
    raw_save_settings, raw_seed_settings, raw_sniff_path,
};
