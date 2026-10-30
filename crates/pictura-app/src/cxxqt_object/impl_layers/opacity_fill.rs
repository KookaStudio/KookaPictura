//! Opacity and fill previews and commits: the display-only preview when the
//! bound is the canvas, the exact region/full path otherwise.

use super::super::display_preview::{commit_display_preview, PreviewKind};
use super::super::edit::Edit;
use super::super::helpers::{as_str_slice, list_of_strings};
use super::super::qobject;
use super::{changed_property_region, property_compositing};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QStringList;
use pictura_core::PsdRect;

impl qobject::PictureView {
    pub fn preview_layers_opacity(
        mut self: Pin<&mut Self>,
        paths: &QStringList,
        value: i32,
    ) -> i32 {
        let value = value.clamp(0, 255) as u8;
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let targets: Vec<String> = refs.iter().map(|path| (*path).to_string()).collect();
        let (changed, region, canvas) = {
            let mut rust = self.as_mut().rust_mut();
            rust.begin_display_preview(targets, PreviewKind::Opacity);
            let Some(doc) = rust.session.doc.as_mut() else {
                return 0;
            };
            let before: Vec<_> = refs
                .iter()
                .map(|path| property_compositing(doc, path))
                .collect();
            let changed = pictura_render::set_opacity_paths(doc, &refs, value);
            let region = if changed > 0 {
                changed_property_region(doc, &refs, &before)
            } else {
                Some(None)
            };
            let canvas = PsdRect {
                top: 0,
                left: 0,
                bottom: doc.height as i32,
                right: doc.width as i32,
            };
            (changed, region, canvas)
        };
        if changed == 0 {
            return 0;
        }
        let display = region.is_none() || matches!(region, Some(Some(rect)) if rect == canvas);
        if display {
            if let Some(rect) = self.as_mut().rust_mut().present_display_preview() {
                self.as_mut()
                    .region_damaged(rect.left, rect.top, rect.width(), rect.height());
                let mut rust = self.as_mut().rust_mut();
                rust.session.dirty = true;
                rust.session.content_revision = rust.session.content_revision.wrapping_add(1);
                rust.pending_previews.opacity_changed = true;
                return changed as i32;
            }
        }
        let saved = self.as_mut().rust_mut().display_preview.take();
        match region {
            Some(Some(rect)) => self.as_mut().refresh_region(rect),
            _ => self.as_mut().recomposite(),
        }
        self.as_mut().rust_mut().display_preview = saved;
        let mut rust = self.as_mut().rust_mut();
        rust.session.dirty = true;
        rust.session.content_revision = rust.session.content_revision.wrapping_add(1);
        rust.pending_previews.opacity_changed = true;
        changed as i32
    }

    pub fn commit_layers_opacity(mut self: Pin<&mut Self>, paths: &QStringList, value: i32) -> i32 {
        let value = value.clamp(0, 255) as u8;
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let pending = self.rust().pending_previews.opacity_changed;
        if self.rust().display_preview_active() {
            if let Some(doc) = self.as_mut().rust_mut().session.doc.as_mut() {
                pictura_render::set_opacity_paths(doc, &refs, value);
            }
            let recorded = commit_display_preview(self.as_mut(), "Opacity");
            self.as_mut().rust_mut().pending_previews.opacity_changed = false;
            return if matches!(recorded, Some(true)) {
                refs.len() as i32
            } else {
                0
            };
        }
        let mut changed = 0;
        self.as_mut().edit("Opacity", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let before: Vec<_> = refs
                .iter()
                .map(|path| property_compositing(doc, path))
                .collect();
            changed = pictura_render::set_opacity_paths(doc, &refs, value);
            if changed == 0 {
                return if pending { Edit::Record } else { Edit::None };
            }
            match changed_property_region(doc, &refs, &before) {
                Some(Some(rect)) => Edit::regions(vec![rect], true),
                _ => Edit::Full,
            }
        });
        self.as_mut().rust_mut().pending_previews.opacity_changed = false;
        changed as i32
    }

    pub fn preview_layers_fill(mut self: Pin<&mut Self>, paths: &QStringList, value: i32) -> i32 {
        let value = value.clamp(0, 255) as u8;
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let targets: Vec<String> = refs.iter().map(|path| (*path).to_string()).collect();
        let (changed, region, canvas) = {
            let mut rust = self.as_mut().rust_mut();
            rust.begin_display_preview(targets, PreviewKind::Fill);
            let Some(doc) = rust.session.doc.as_mut() else {
                return 0;
            };
            let before: Vec<_> = refs
                .iter()
                .map(|path| property_compositing(doc, path))
                .collect();
            let changed = pictura_render::set_fill_paths(doc, &refs, value);
            let region = if changed > 0 {
                changed_property_region(doc, &refs, &before)
            } else {
                Some(None)
            };
            let canvas = PsdRect {
                top: 0,
                left: 0,
                bottom: doc.height as i32,
                right: doc.width as i32,
            };
            (changed, region, canvas)
        };
        if changed == 0 {
            return 0;
        }
        let display = region.is_none() || matches!(region, Some(Some(rect)) if rect == canvas);
        if display {
            if let Some(rect) = self.as_mut().rust_mut().present_display_preview() {
                self.as_mut()
                    .region_damaged(rect.left, rect.top, rect.width(), rect.height());
                let mut rust = self.as_mut().rust_mut();
                rust.session.dirty = true;
                rust.session.content_revision = rust.session.content_revision.wrapping_add(1);
                rust.pending_previews.fill_changed = true;
                return changed as i32;
            }
        }
        let saved = self.as_mut().rust_mut().display_preview.take();
        match region {
            Some(Some(rect)) => self.as_mut().refresh_region(rect),
            _ => self.as_mut().recomposite(),
        }
        self.as_mut().rust_mut().display_preview = saved;
        let mut rust = self.as_mut().rust_mut();
        rust.session.dirty = true;
        rust.session.content_revision = rust.session.content_revision.wrapping_add(1);
        rust.pending_previews.fill_changed = true;
        changed as i32
    }

    pub fn commit_layers_fill(mut self: Pin<&mut Self>, paths: &QStringList, value: i32) -> i32 {
        let value = value.clamp(0, 255) as u8;
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let pending = self.rust().pending_previews.fill_changed;
        if self.rust().display_preview_active() {
            if let Some(doc) = self.as_mut().rust_mut().session.doc.as_mut() {
                pictura_render::set_fill_paths(doc, &refs, value);
            }
            let recorded = commit_display_preview(self.as_mut(), "Fill Opacity");
            self.as_mut().rust_mut().pending_previews.fill_changed = false;
            return if matches!(recorded, Some(true)) {
                refs.len() as i32
            } else {
                0
            };
        }
        let mut changed = 0;
        self.as_mut().edit("Fill Opacity", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let before: Vec<_> = refs
                .iter()
                .map(|path| property_compositing(doc, path))
                .collect();
            changed = pictura_render::set_fill_paths(doc, &refs, value);
            if changed == 0 {
                return if pending { Edit::Record } else { Edit::None };
            }
            match changed_property_region(doc, &refs, &before) {
                Some(Some(rect)) => Edit::regions(vec![rect], true),
                _ => Edit::Full,
            }
        });
        self.as_mut().rust_mut().pending_previews.fill_changed = false;
        changed as i32
    }
}
