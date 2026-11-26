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

pub(super) fn channels_multichannel_plates(view: &PictureView) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| doc.source.multichannel_plates())
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
    pictura_codec::append_channel_display_info(doc);
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
        if doc.source.multichannel_plates() {
            let preview = pictura_codec::multichannel_preview(doc);
            doc.composite.replace(preview).expect("valid plate preview");
            Edit::Full
        } else {
            Edit::Meta
        }
    });
    removed
}

pub(super) fn channels_load_color_selection(
    view: Pin<&mut PictureView>,
    row: i32,
    mask: i32,
    invert: bool,
    mode: &QString,
) -> bool {
    view.load_channel_selection(row, mask, invert, mode)
}

fn delete_alpha_with_name(doc: &mut Document, index: usize) -> bool {
    if index >= doc.channels.len()
        || (doc.source.multichannel_plates() && doc.channels.len() == 1)
    {
        return false;
    }
    let mut names = pictura_codec::alpha_names(doc);
    if doc.source.multichannel_plates() {
        if let Some(mut store) = doc.source.planes().clone() {
            let plane = doc.width as usize * doc.height as usize;
            let mut bytes = store.samples.to_bytes();
            let sample_width = if store.depth == pictura_core::BitDepth::Sixteen { 2 } else { 1 };
            let start = index * plane * sample_width;
            let end = start + plane * sample_width;
            if end <= bytes.len() {
                bytes.drain(start..end);
                store.samples = pictura_core::Samples::from_bytes(&bytes, store.depth);
                doc.source.replace_planes(Some(store)).expect("matching plate sample depth");
            }
        }
    }
    doc.channels.remove(index);
    pictura_codec::remove_channel_display_info(doc, index);
    if index < names.len() {
        names.remove(index);
    }
    names.truncate(doc.channels.len());
    pictura_codec::set_alpha_names(doc, &names);
    true
}

/// Whether extra channel `index` is `Selected Areas`. A missing entry and a
/// spot entry are `Masked Areas`.
pub(super) fn channels_selected_areas(view: &PictureView, index: i32) -> bool {
    let Some(index) = usize::try_from(index).ok() else {
        return false;
    };
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_codec::channel_selected_areas(doc, index))
}

/// Whether extra channel `index` is a spot channel from the PSD DisplayInfo.
pub(super) fn channels_alpha_is_spot(view: &PictureView, index: i32) -> bool {
    let Some(index) = usize::try_from(index).ok() else {
        return false;
    };
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_codec::spot_channel_indices(doc).contains(&index))
}

/// Toggle one channel's sense in place. Returns `Edit::None` when the sense is
/// already `selected`, the index is bad, or the channel is a spot entry. The
/// eligibility check happens before the plane is complemented.
pub(in crate::cxxqt_object) fn toggle_channel_sense(
    doc: &mut Document,
    index: usize,
    selected: bool,
) -> Edit {
    if index >= doc.channels.len() || pictura_codec::channel_selected_areas(doc, index) == selected
    {
        return Edit::None;
    }
    if !pictura_codec::set_channel_selected_areas(doc, index, selected) {
        return Edit::None;
    }
    for byte in doc.channels[index].data.as_mut_slice() {
        *byte = 255 - *byte;
    }
    Edit::Meta
}

/// Set extra channel `index` to `selected`; one `Channel Options` state when it
/// changes. False for a bad index, a spot channel, or no change.
pub(super) fn channels_set_sense(
    mut view: Pin<&mut PictureView>,
    index: i32,
    selected: bool,
) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let outcome = view.as_mut().edit("Channel Options", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        toggle_channel_sense(doc, index, selected)
    });
    !matches!(outcome, Edit::None)
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

    #[test]
    fn new_channel_defaults_to_masked() {
        let mut doc = doc();
        let index = new_alpha(&mut doc).expect("new channel");
        assert!(!pictura_codec::channel_selected_areas(&doc, index));
        assert!(doc.channels[index].data.iter().all(|&byte| byte == 0));
    }

    #[test]
    fn new_channel_keeps_other_senses() {
        let mut doc = doc();
        assert!(pictura_codec::set_channel_selected_areas(&mut doc, 1, true));
        let index = new_alpha(&mut doc).expect("new channel");
        assert!(pictura_codec::channel_selected_areas(&doc, 1));
        assert!(!pictura_codec::channel_selected_areas(&doc, index));
        assert_eq!(pictura_codec::display_info(&doc).len(), 4);
    }

    #[test]
    fn delete_channel_shifts_display_info() {
        let mut doc = doc();
        pictura_codec::set_display_info(
            &mut doc,
            &[
                pictura_codec::ChannelDisplayInfo {
                    opacity: 10,
                    mode: pictura_codec::SELECTED_AREAS_MODE,
                    ..Default::default()
                },
                pictura_codec::ChannelDisplayInfo {
                    opacity: 20,
                    mode: pictura_codec::MASKED_AREAS_MODE,
                    ..Default::default()
                },
                pictura_codec::ChannelDisplayInfo {
                    opacity: 30,
                    mode: pictura_codec::SPOT_CHANNEL_MODE,
                    ..Default::default()
                },
            ],
        );
        assert!(delete_alpha_with_name(&mut doc, 0));
        let infos = pictura_codec::display_info(&doc);
        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0].opacity, 20);
        assert_eq!(infos[1].opacity, 30);
    }
}
