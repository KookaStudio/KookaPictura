//! Save/load target helpers: target parsing, channel and mask combination,
//! coverage transfer, and mask target listing.

use super::*;
use crate::cxxqt_object::state::PictureViewRust;
use pictura_core::{Document, LayerMask};

#[derive(Clone, Debug)]
pub(super) enum SelectionTarget {
    NewChannel,
    Channel(usize),
    Mask(String),
}

pub(super) fn parse_selection_target(target: &str) -> Option<SelectionTarget> {
    if target == "new" {
        return Some(SelectionTarget::NewChannel);
    }
    if let Some(index) = target.strip_prefix("alpha:") {
        return index.parse().ok().map(SelectionTarget::Channel);
    }
    target
        .strip_prefix("mask:")
        .map(|path| SelectionTarget::Mask(path.to_string()))
}

fn default_channel_name(index: usize) -> String {
    format!("Alpha {}", index + 1)
}

pub(super) fn document_matches(rust: &PictureViewRust, width: i32, height: i32) -> bool {
    rust.session
        .doc
        .as_ref()
        .is_some_and(|doc| doc.width == width as u32 && doc.height == height as u32)
}

#[cfg(test)]
pub(super) fn save_selection_channel(rust: &mut PictureViewRust) -> Edit {
    let Some(selection) = rust.transfer_source("", 0) else {
        return Edit::None;
    };
    rust.transfer_save(selection, "new", "", CombineMode::New)
}

pub(super) fn save_selection_shape(
    rust: &mut PictureViewRust,
    selection: Selection,
    target: SelectionTarget,
    name: &str,
    mode: CombineMode,
) -> Edit {
    let Some(doc) = rust.session.doc.as_mut() else {
        return Edit::None;
    };
    match target {
        SelectionTarget::NewChannel => {
            if !doc.can_add_channel() {
                return Edit::None;
            }
            let id = doc.channels.iter().map(|c| c.id).max().unwrap_or(0) + 1;
            let index = doc.channels.len();
            doc.channels.push(selection.to_channel(id));
            pictura_codec::append_channel_display_info(doc);
            let mut names = pictura_codec::alpha_names(doc);
            names.resize(doc.channels.len(), String::new());
            for (i, stored) in names.iter_mut().enumerate() {
                if stored.trim().is_empty() {
                    *stored = default_channel_name(i);
                }
            }
            names[index] = if name.trim().is_empty() {
                default_channel_name(index)
            } else {
                name.trim().to_string()
            };
            pictura_codec::set_alpha_names(doc, &names);
            Edit::Meta
        }
        SelectionTarget::Channel(index) => {
            let Some(channel) = doc.channels.get(index) else {
                return Edit::None;
            };
            let Ok(base) = Selection::from_channel(channel, doc.width, doc.height) else {
                return Edit::None;
            };
            let id = channel.id;
            let selected = pictura_codec::channel_selected_areas(doc, index);
            let mut base = if selected { base.invert() } else { base };
            if base.combine(&selection, mode.into()).is_err() {
                return Edit::None;
            }
            let written = if selected { base.invert() } else { base };
            doc.channels[index] = written.to_channel(id);
            Edit::Meta
        }
        SelectionTarget::Mask(path) => {
            let Some(layer) = pictura_render::resolve_path(doc, &path) else {
                return Edit::None;
            };
            let Some(mask) = layer.mask.as_ref() else {
                return Edit::None;
            };
            let mut base = stored_mask_selection(mask, doc.width, doc.height);
            if base.combine(&selection, mode.into()).is_err() {
                return Edit::None;
            }
            let mut written = mask.clone();
            written.rect = PsdRect {
                top: 0,
                left: 0,
                bottom: doc.height as i32,
                right: doc.width as i32,
            };
            written.default_color = 0;
            written.data = Some(base.data);
            let mask_rect = written.rect;
            if !pictura_render::replace_layer_mask(doc, &path, written) {
                return Edit::None;
            }
            match pictura_render::damage::influence(doc, &path) {
                Some(rect) => Edit::Regions {
                    rects: vec![rect],
                    // The write replaces the whole mask plane; the composite
                    // influence is smaller than the changed mask tiles.
                    hint: Some(vec![mask_rect]),
                    notify: true,
                },
                None => Edit::Full,
            }
        }
    }
}

pub(super) fn selection_from_target(doc: &Document, target: &str) -> Option<Selection> {
    match parse_selection_target(target)? {
        SelectionTarget::NewChannel => None,
        SelectionTarget::Channel(index) => {
            let channel = doc.channels.get(index)?;
            let selection = Selection::from_channel(channel, doc.width, doc.height).ok()?;
            Some(if pictura_codec::channel_selected_areas(doc, index) {
                selection.invert()
            } else {
                selection
            })
        }
        SelectionTarget::Mask(path) => {
            let layer = pictura_render::resolve_path(doc, &path)?;
            let mask = layer.mask.as_ref()?;
            Some(pictura_select::selection_from_mask(
                mask, doc.width, doc.height,
            ))
        }
    }
}

/// The raw stored coverage of a mask, ignoring its disabled flag. This is the
/// save-combine base; Load uses `selection_from_mask` so it matches the
/// Channels/Layers Ctrl-click behaviour.
fn stored_mask_selection(mask: &LayerMask, width: u32, height: u32) -> Selection {
    let mut data = vec![0u8; width as usize * height as usize];
    if let Some(pixels) = mask.data.as_ref() {
        let mw = mask.rect.width();
        let mh = mask.rect.height();
        let slice = pixels.as_slice();
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let mx = x - mask.rect.left;
                let my = y - mask.rect.top;
                let value = if mw > 0 && mh > 0 && mx >= 0 && my >= 0 && mx < mw && my < mh {
                    slice
                        .get(my as usize * mw as usize + mx as usize)
                        .copied()
                        .unwrap_or(mask.default_color)
                } else {
                    mask.default_color
                };
                data[y as usize * width as usize + x as usize] = value;
            }
        }
    } else {
        data.fill(mask.default_color);
    }
    Selection {
        width,
        height,
        data: data.into(),
    }
}

pub(super) fn coverage_selection(coverage: &[u8], width: i32, height: i32) -> Option<Selection> {
    if width < 1 || height < 1 {
        return None;
    }
    if coverage.len() != width as usize * height as usize {
        return None;
    }
    Some(Selection {
        width: width as u32,
        height: height as u32,
        data: coverage.to_vec().into(),
    })
}

pub(super) fn mask_targets(doc: &Document) -> Vec<String> {
    pictura_render::flatten_rows(doc)
        .into_iter()
        .filter_map(|(path, _)| {
            let layer = pictura_render::resolve_path(doc, &path)?;
            layer
                .mask
                .as_ref()
                .map(|_| format!("{path}|{} Mask", layer.name))
        })
        .collect()
}
