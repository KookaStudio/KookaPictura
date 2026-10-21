//! The Channels panel's document queries and edits: the colour mode its rows
//! follow (the opened file's mode, which the working RGB / Gray data stands
//! in for), and the document's alpha channels (saved selections, named
//! "Alpha 1", "Alpha 2", … by position) — their thumbnails, New Channel, and
//! Delete Channel. Free functions over a [`PictureView`] (their own bridge).
//!
//! [`PictureView`]: super::qobject::PictureView

use super::qobject::PictureView;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{Channel, ColorMode};

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
        /// The mode the Channels panel lists: `"rgb"`, `"grayscale"`, `"cmyk"`, `"lab"`, `"multichannel"`, `"indexed"`, `"bitmap"`, or `"duotone"` — the opened file's mode when it was converted for editing. Empty without a document.
        fn channels_mode(view: &PictureView) -> QString;

        /// Alpha channel `index` scaled to fit `max_side` (one byte a pixel, row-major), its size in `width` / `height`; empty for a bad index.
        fn channels_alpha_thumbnail(
            view: &PictureView,
            index: i32,
            max_side: i32,
            width: &mut i32,
            height: &mut i32,
        ) -> Vec<u8>;

        /// Add an alpha channel, black (all masked) as CS6's New Channel makes it; one "New Channel" state. Returns its index, or -1 without a document.
        fn channels_new_alpha(view: Pin<&mut PictureView>) -> i32;

        /// Delete alpha channel `index`; one "Delete Channel" state. False for a bad index.
        fn channels_delete_alpha(view: Pin<&mut PictureView>, index: i32) -> bool;

        /// Load colour channel row `row` (0 = the composite) as the selection; `mask` is the visible RGB channels (bit 0 red, 1 green, 2 blue). False for a row outside the mode's colour rows.
        fn channels_load_color_selection(view: Pin<&mut PictureView>, row: i32, mask: i32) -> bool;

        /// The active layer's raster-mask channel name (`"<layer name> Mask"`), or
        /// empty when the active layer carries no raster mask.
        fn channels_active_mask_name(view: &PictureView) -> QString;

        /// The active layer's raster-mask coverage scaled to fit `max_side` (one
        /// byte a pixel, row-major), its size in `width` / `height`; empty without
        /// a mask. A disabled mask still shows its stored coverage.
        fn channels_active_mask_thumbnail(
            view: &PictureView,
            max_side: i32,
            width: &mut i32,
            height: &mut i32,
        ) -> Vec<u8>;
    }
}

pub(super) fn mode_name(mode: ColorMode) -> &'static str {
    match mode {
        ColorMode::Bitmap => "bitmap",
        ColorMode::Grayscale => "grayscale",
        ColorMode::Indexed => "indexed",
        ColorMode::Rgb => "rgb",
        ColorMode::Cmyk => "cmyk",
        ColorMode::Multichannel => "multichannel",
        ColorMode::Duotone => "duotone",
        ColorMode::Lab => "lab",
    }
}

fn channels_mode(view: &PictureView) -> QString {
    let name = view
        .rust()
        .session
        .doc
        .as_ref()
        .map_or("", |doc| mode_name(doc.source_mode.unwrap_or(doc.mode)));
    QString::from(name)
}

/// Nearest-neighbour shrink of a `width` x `height` plane to fit `max_side`.
fn shrink_plane(data: &[u8], width: u32, height: u32, max_side: u32) -> (Vec<u8>, u32, u32) {
    let longer = width.max(height);
    if max_side == 0 || longer <= max_side {
        return (data.to_vec(), width, height);
    }
    let w = ((width as u64 * max_side as u64) / longer as u64).max(1) as u32;
    let h = ((height as u64 * max_side as u64) / longer as u64).max(1) as u32;
    let mut out = Vec::with_capacity(w as usize * h as usize);
    for y in 0..h as usize {
        let sy = (y * height as usize / h as usize).min(height as usize - 1);
        for x in 0..w as usize {
            let sx = (x * width as usize / w as usize).min(width as usize - 1);
            out.push(data[sy * width as usize + sx]);
        }
    }
    (out, w, h)
}

fn channels_alpha_thumbnail(
    view: &PictureView,
    index: i32,
    max_side: i32,
    width: &mut i32,
    height: &mut i32,
) -> Vec<u8> {
    *width = 0;
    *height = 0;
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return Vec::new();
    };
    let Some(channel) = usize::try_from(index)
        .ok()
        .and_then(|i| doc.channels.get(i))
    else {
        return Vec::new();
    };
    if channel.data.len() != doc.width as usize * doc.height as usize {
        return Vec::new();
    }
    let (plane, w, h) = shrink_plane(&channel.data, doc.width, doc.height, max_side.max(0) as u32);
    *width = w as i32;
    *height = h as i32;
    plane
}

fn channels_new_alpha(mut view: Pin<&mut PictureView>) -> i32 {
    let index = {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.session.doc.as_mut() else {
            return -1;
        };
        let id = doc.channels.iter().map(|c| c.id).max().unwrap_or(0) + 1;
        doc.channels.push(Channel {
            id,
            data: vec![0u8; doc.width as usize * doc.height as usize].into(),
        });
        doc.channels.len() as i32 - 1
    };
    view.as_mut().edit("New Channel", |_| Edit::Record);
    index
}

fn channels_delete_alpha(mut view: Pin<&mut PictureView>, index: i32) -> bool {
    let removed = {
        let mut rust = view.as_mut().rust_mut();
        match (rust.session.doc.as_mut(), usize::try_from(index)) {
            (Some(doc), Ok(i)) if i < doc.channels.len() => {
                doc.channels.remove(i);
                true
            }
            _ => false,
        }
    };
    if removed {
        view.as_mut().edit("Delete Channel", |_| Edit::Record);
    }
    removed
}

fn channels_load_color_selection(view: Pin<&mut PictureView>, row: i32, mask: i32) -> bool {
    view.load_channel_selection(row, mask)
}

fn channels_active_mask_name(view: &PictureView) -> QString {
    let rust = view.rust();
    let (Some(doc), Some(path)) = (rust.session.doc.as_ref(), rust.layer_target.anchor()) else {
        return QString::default();
    };
    if !pictura_render::has_layer_mask(doc, path) {
        return QString::default();
    }
    match pictura_render::resolve_path(doc, path) {
        Some(layer) => QString::from(format!("{} Mask", layer.name)),
        None => QString::default(),
    }
}

fn channels_active_mask_thumbnail(
    view: &PictureView,
    max_side: i32,
    width: &mut i32,
    height: &mut i32,
) -> Vec<u8> {
    *width = 0;
    *height = 0;
    let Some((plane, w, h)) = view.rust().mask_channel_thumbnail(max_side.max(0) as u32) else {
        return Vec::new();
    };
    *width = w as i32;
    *height = h as i32;
    plane
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shrink_plane_fits_and_samples() {
        let data: Vec<u8> = (0..16).collect();
        let (small, w, h) = shrink_plane(&data, 8, 2, 4);
        assert_eq!((w, h), (4, 1));
        assert_eq!(small, [0, 2, 4, 6]);
        assert_eq!(shrink_plane(&data, 8, 2, 100).0.len(), 16);
        assert_eq!(mode_name(ColorMode::Lab), "lab");
    }
}
