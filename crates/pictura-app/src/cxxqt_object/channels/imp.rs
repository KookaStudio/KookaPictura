use super::*;

use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_core::{Channel, ColorMode, Document};

pub(in crate::cxxqt_object) fn mode_name(mode: ColorMode) -> &'static str {
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

pub(super) fn channels_mode(view: &PictureView) -> QString {
    let name = view
        .rust()
        .session
        .doc
        .as_ref()
        .map_or("", |doc| mode_name(doc.source.mode().unwrap_or(doc.mode)));
    QString::from(name)
}

/// Nearest-neighbour shrink of a `width` x `height` plane to fit `max_side`.
pub(super) fn shrink_plane(
    data: &[u8],
    width: u32,
    height: u32,
    max_side: u32,
) -> (Vec<u8>, u32, u32) {
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

pub(super) fn channels_alpha_thumbnail(
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

pub(super) fn channels_max() -> i32 {
    i32::from(pictura_core::channels::MAX_CHANNELS)
}

pub(super) fn channels_can_add(view: &PictureView) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| doc.can_add_channel())
}

fn new_alpha(doc: &mut pictura_core::Document) -> Option<usize> {
    if !doc.can_add_channel() {
        return None;
    }
    let id = doc.channels.iter().map(|c| c.id).max().unwrap_or(0) + 1;
    doc.channels.push(Channel {
        id,
        data: vec![0u8; doc.width as usize * doc.height as usize].into(),
    });
    Some(doc.channels.len() - 1)
}

pub(super) fn channels_new_alpha(mut view: Pin<&mut PictureView>) -> i32 {
    let mut index = -1;
    view.as_mut().edit("New Channel", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        let Some(i) = new_alpha(doc) else {
            return Edit::None;
        };
        index = i as i32;
        Edit::Meta
    });
    index
}

pub(super) fn channels_delete_alpha(mut view: Pin<&mut PictureView>, index: i32) -> bool {
    let mut removed = false;
    view.as_mut().edit("Delete Channel", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        let Ok(i) = usize::try_from(index) else {
            return Edit::None;
        };
        if !delete_alpha_with_name(doc, i) {
            return Edit::None;
        }
        removed = true;
        Edit::Meta
    });
    removed
}

pub(super) fn channels_load_color_selection(
    view: Pin<&mut PictureView>,
    row: i32,
    mask: i32,
    mode: &QString,
) -> bool {
    view.load_channel_selection(row, mask, mode)
}

fn delete_alpha_with_name(doc: &mut Document, index: usize) -> bool {
    if index >= doc.channels.len() {
        return false;
    }
    let mut names = pictura_codec::alpha_names(doc);
    doc.channels.remove(index);
    if index < names.len() {
        names.remove(index);
    }
    names.truncate(doc.channels.len());
    pictura_codec::set_alpha_names(doc, &names);
    true
}

fn alpha_display_names(doc: &Document) -> Vec<String> {
    let names = pictura_codec::alpha_names(doc);
    (0..doc.channels.len())
        .map(|index| {
            let stored = names.get(index).map(String::as_str).unwrap_or("");
            if stored.trim().is_empty() {
                format!("Alpha {}", index + 1)
            } else {
                stored.to_string()
            }
        })
        .collect()
}

pub(super) fn channels_alpha_names(view: &PictureView) -> QStringList {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return QStringList::default();
    };
    alpha_display_names(doc)
        .into_iter()
        .map(QString::from)
        .collect()
}

pub(super) fn channels_alpha_name(view: &PictureView, index: i32) -> QString {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return QString::default();
    };
    usize::try_from(index)
        .ok()
        .and_then(|index| alpha_display_names(doc).into_iter().nth(index))
        .map(QString::from)
        .unwrap_or_default()
}

pub(super) fn channels_active_mask_name(view: &PictureView) -> QString {
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

pub(super) fn channels_active_mask_thumbnail(
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
    use pictura_core::BitDepth;

    #[test]
    fn new_alpha_refuses_at_cap() {
        use pictura_core::{BitDepth, ColorMode, Document};

        let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        for i in 0..53 {
            doc.channels.push(Channel {
                id: i,
                data: vec![0u8; 4].into(),
            });
        }
        assert!(!doc.can_add_channel());
        assert_eq!(new_alpha(&mut doc), None);
        assert_eq!(doc.channels.len(), 53);
    }

    #[test]
    fn shrink_plane_fits_and_samples() {
        let data: Vec<u8> = (0..16).collect();
        let (small, w, h) = shrink_plane(&data, 8, 2, 4);
        assert_eq!((w, h), (4, 1));
        assert_eq!(small, [0, 2, 4, 6]);
        assert_eq!(shrink_plane(&data, 8, 2, 100).0.len(), 16);
        assert_eq!(mode_name(ColorMode::Lab), "lab");
    }

    fn doc() -> Document {
        let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        doc.channels = (0..3)
            .map(|id| Channel {
                id,
                data: vec![0u8; 4].into(),
            })
            .collect();
        doc
    }

    #[test]
    fn channel_name_falls_back_to_alpha_n() {
        let mut doc = doc();
        assert_eq!(
            alpha_display_names(&doc),
            vec!["Alpha 1", "Alpha 2", "Alpha 3"]
        );
        pictura_codec::set_alpha_names(&mut doc, &["Hair".to_string()]);
        assert_eq!(
            alpha_display_names(&doc),
            vec!["Hair", "Alpha 2", "Alpha 3"]
        );
    }

    #[test]
    fn channel_name_positions_follow_add_and_delete() {
        let mut doc = doc();
        pictura_codec::set_alpha_names(
            &mut doc,
            &["A".to_string(), "B".to_string(), "C".to_string()],
        );
        assert!(delete_alpha_with_name(&mut doc, 1));
        assert_eq!(alpha_display_names(&doc), vec!["A", "C"]);
        doc.channels.push(Channel {
            id: 9,
            data: vec![0u8; 4].into(),
        });
        assert_eq!(alpha_display_names(&doc), vec!["A", "C", "Alpha 3"]);
    }
}
