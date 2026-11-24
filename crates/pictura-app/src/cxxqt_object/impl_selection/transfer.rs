//! One owner for selection coverage transfers (#1400): acquire a source
//! document's coverage as the engine's `Selection` and apply it to a
//! destination, so the shell never brokers dense pixels.

use super::super::edit::Edit;
use super::super::picture::{stream_picture, whole};
use super::super::state::PictureViewRust;
use super::channel_rows::channel_row_selection_from;
use super::quick::combine_selection;
use super::save_load::{
    document_matches, parse_selection_target, save_selection_shape, selection_from_target,
    SelectionTarget,
};
use pictura_core::{BitDepth, ColorMode, Document};
use pictura_select::{CombineMode, Selection};

impl PictureViewRust {
    /// The source coverage named by `spec`: empty for the live selection,
    /// `alpha:<n>`, `mask:<path>`, or `color:<row>` with the visible-channel
    /// `mask`. `alpha:` applies the channel's sense; `mask:` applies the
    /// mask's disabled/data-less rule.
    pub(in crate::cxxqt_object) fn transfer_source(
        &self,
        spec: &str,
        mask: u8,
    ) -> Option<Selection> {
        if spec.is_empty() {
            return self.session.selection.clone();
        }
        if let Some(row) = spec.strip_prefix("color:") {
            let row: i32 = row.parse().ok()?;
            let doc = self.session.doc.as_ref()?;
            return channel_row_selection_from(
                doc,
                |f| stream_picture(doc, self, whole(doc), f),
                row,
                mask,
            );
        }
        let doc = self.session.doc.as_ref()?;
        selection_from_target(doc, spec)
    }

    /// Save `shape` into `target` (`"new"`, `alpha:<n>`, or `mask:<path>`).
    /// Same-size only; the dialogs enforce it and this refuses a mismatch.
    pub(in crate::cxxqt_object) fn transfer_save(
        &mut self,
        shape: Selection,
        target: &str,
        name: &str,
        mode: CombineMode,
    ) -> Edit {
        if self.session.read_only() {
            return Edit::None;
        }
        if !document_matches(self, shape.width as i32, shape.height as i32) {
            return Edit::None;
        }
        let Some(target) = parse_selection_target(target) else {
            return Edit::None;
        };
        save_selection_shape(self, shape, target, name, mode)
    }

    /// Add `shape` as a new channel, fitted to this canvas and optionally
    /// inverted.
    pub(in crate::cxxqt_object) fn transfer_channel(
        &mut self,
        shape: Selection,
        name: &str,
        invert: bool,
    ) -> Edit {
        if self.session.read_only() {
            return Edit::None;
        }
        let Some(doc) = self.session.doc.as_ref() else {
            return Edit::None;
        };
        if !doc.can_add_channel() {
            return Edit::None;
        }
        let (width, height) = (doc.width, doc.height);
        let shape = if invert { shape.invert() } else { shape };
        let fitted = shape.fit_centered(width, height);
        save_selection_shape(
            self,
            fitted,
            SelectionTarget::NewChannel,
            name,
            CombineMode::New,
        )
    }

    /// Load `shape` into the selection, optionally inverted and fitted. A
    /// non-fitted load requires the document's dimensions.
    pub(in crate::cxxqt_object) fn transfer_load(
        &mut self,
        shape: Selection,
        invert: bool,
        fit: bool,
        mode: CombineMode,
    ) -> Edit {
        if self.session.read_only() {
            return Edit::None;
        }
        let Some((width, height)) = self.session.doc.as_ref().map(|doc| (doc.width, doc.height))
        else {
            return Edit::None;
        };
        if !fit && (shape.width != width || shape.height != height) {
            return Edit::None;
        }
        let shape = if invert { shape.invert() } else { shape };
        let shape = if fit {
            shape.fit_centered(width, height)
        } else {
            shape
        };
        combine_selection(self, shape, mode, false);
        Edit::Meta
    }

    /// Build a grayscale document holding `shape` as its named extra channel.
    pub(in crate::cxxqt_object) fn transfer_document(
        &mut self,
        shape: Selection,
        name: &str,
        invert: bool,
    ) -> bool {
        let shape = if invert { shape.invert() } else { shape };
        let mut doc = Document::new(
            shape.width,
            shape.height,
            ColorMode::Grayscale,
            BitDepth::Eight,
        );
        doc.channels.push(shape.to_channel(0));
        let label = if name.trim().is_empty() {
            "Alpha 1".to_string()
        } else {
            name.trim().to_string()
        };
        pictura_codec::set_alpha_names(&mut doc, &[label]);
        self.replace_document(
            Some(doc),
            crate::history::History::default(),
            Default::default(),
            None,
        );
        let target = self.layer_target.clone();
        if let Some(doc) = self.session.doc.as_mut() {
            self.session
                .history
                .capture_live(doc, &None, &target, "New");
        }
        self.path = None;
        self.session.mark_saved();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cxxqt_object::open_jobs::compatibility::NativeCompatibility;
    use crate::cxxqt_object::picture::readers_fixture::{layered, view_of};
    use pictura_core::{Channel, LayerMask, Plane, PsdRect};

    fn plain(width: u32, height: u32) -> PictureViewRust {
        view_of(Document::new(
            width,
            height,
            ColorMode::Rgb,
            BitDepth::Eight,
        ))
    }

    fn layered_view(width: u32, height: u32) -> PictureViewRust {
        view_of(layered(width, height, false))
    }

    fn add_alpha(view: &mut PictureViewRust, data: Vec<u8>) -> usize {
        let doc = view.session.doc.as_mut().expect("document");
        let id = doc
            .channels
            .iter()
            .map(|channel| channel.id)
            .max()
            .unwrap_or(0)
            + 1;
        doc.channels.push(Channel {
            id,
            data: data.into(),
        });
        doc.channels.len() - 1
    }

    fn flat_selection(width: u32, height: u32, value: u8) -> Selection {
        Selection {
            width,
            height,
            data: Plane::from(vec![value; width as usize * height as usize]),
        }
    }

    fn selection_bytes(view: &PictureViewRust) -> Vec<u8> {
        view.session
            .selection
            .as_ref()
            .expect("a selection")
            .coverage_bytes()
    }

    fn channel_bytes(view: &PictureViewRust, index: usize) -> Vec<u8> {
        view.session.doc.as_ref().expect("document").channels[index]
            .data
            .as_slice()
            .to_vec()
    }

    #[test]
    fn same_document_duplicate_matches_direct_apply() {
        let mut owner = plain(64, 64);
        let mut direct = plain(64, 64);
        let data: Vec<u8> = (0..64 * 64).map(|i| (i % 251) as u8).collect();
        add_alpha(&mut owner, data.clone());
        add_alpha(&mut direct, data);

        let shape = owner.transfer_source("alpha:0", 0).expect("channel");
        let edit = owner.transfer_channel(shape, "Copy", false);
        owner.apply_edit("Duplicate Channel", edit);

        let shape = direct.transfer_source("alpha:0", 0).expect("channel");
        let edit = save_selection_shape(
            &mut direct,
            shape,
            SelectionTarget::NewChannel,
            "Copy",
            CombineMode::New,
        );
        direct.apply_edit("Duplicate Channel", edit);

        assert_eq!(channel_bytes(&owner, 1), channel_bytes(&direct, 1));
        assert_eq!(owner.session.history.count(), 1);
        assert_eq!(direct.session.history.count(), 1);
        assert_eq!(owner.session.history.label(0), "Duplicate Channel");
    }

    #[test]
    fn cross_document_transfer_matches_direct_apply() {
        let mut source = plain(64, 64);
        source
            .session
            .set_selection(Some(Selection::rect(64, 64, 4, 4, 40, 40)));
        let shape = source.transfer_source("", 0).expect("selection");

        let mut owner = plain(64, 64);
        let mut direct = plain(64, 64);

        let edit = owner.transfer_load(shape.clone(), false, false, CombineMode::New);
        owner.apply_edit("Load Selection", edit);
        combine_selection(&mut direct, shape, CombineMode::New, false);
        direct.apply_edit("Load Selection", Edit::Meta);

        assert_eq!(selection_bytes(&owner), selection_bytes(&direct));
        assert_eq!(owner.session.history.count(), 1);
        assert_eq!(direct.session.history.count(), 1);
    }

    #[test]
    fn colour_row_load_applies_in_place() {
        let mut view = layered_view(64, 64);
        view.session.set_selection(Some(Selection::all(64, 64)));
        let shape = view.transfer_source("color:1", 0x7).expect("colour row");
        assert_eq!((shape.width, shape.height), (64, 64));
        let edit = view.transfer_load(shape, false, false, CombineMode::New);
        view.apply_edit("Load Selection", edit);
        let selection = view.session.selection.as_ref().expect("selection");
        assert_eq!((selection.width, selection.height), (64, 64));
        assert!(selection.coverage_bytes().iter().any(|value| *value > 0));
        assert_eq!(view.session.history.count(), 1);
        assert_eq!(view.session.history.label(0), "Load Selection");
    }

    #[test]
    fn new_document_holds_source_coverage() {
        let mut source = plain(64, 64);
        source
            .session
            .set_selection(Some(Selection::rect(64, 64, 4, 4, 20, 20)));
        let shape = source.transfer_source("", 0).expect("selection");
        let want = shape.coverage_bytes();

        let mut target = PictureViewRust::default();
        assert!(target.transfer_document(shape, "Hair", false));

        let doc = target.session.doc.as_ref().expect("document");
        assert_eq!((doc.width, doc.height), (64, 64));
        assert_eq!(doc.mode, ColorMode::Grayscale);
        assert_eq!(doc.channels.len(), 1);
        assert_eq!(doc.channels[0].data.as_slice(), want.as_slice());
        assert_eq!(target.session.history.count(), 1);
        assert_eq!(target.session.history.label(0), "New");
    }

    #[test]
    fn cross_document_save_combines() {
        let mut source = plain(32, 32);
        source
            .session
            .set_selection(Some(flat_selection(32, 32, 64)));
        let shape = source.transfer_source("", 0).expect("selection");

        let mut target = plain(32, 32);
        assert_eq!(add_alpha(&mut target, vec![128u8; 32 * 32]), 0);

        for (mode, expected) in [
            (CombineMode::Add, 128u8),
            (CombineMode::Subtract, 64),
            (CombineMode::Intersect, 32),
        ] {
            target.session.doc.as_mut().expect("document").channels[0].data =
                vec![128u8; 32 * 32].into();
            let edit = target.transfer_save(shape.clone(), "alpha:0", "", mode);
            target.apply_edit("Save Selection", edit);
            let channel = &target.session.doc.as_ref().expect("document").channels[0];
            assert_eq!(channel.data.as_slice()[0], expected);
        }
    }

    #[test]
    fn transfer_records_one_destination_state() {
        let shape = Selection::rect(32, 32, 4, 4, 16, 16);

        let mut channel = plain(32, 32);
        let edit = channel.transfer_channel(shape.clone(), "C", false);
        channel.apply_edit("Duplicate Channel", edit);
        assert_eq!(channel.session.history.count(), 1);
        assert_eq!(channel.session.history.label(0), "Duplicate Channel");

        let mut save = plain(32, 32);
        let edit = save.transfer_save(shape.clone(), "new", "S", CombineMode::New);
        save.apply_edit("Save Selection", edit);
        assert_eq!(save.session.history.count(), 1);
        assert_eq!(save.session.history.label(0), "Save Selection");

        let mut load = plain(32, 32);
        let edit = load.transfer_load(shape.clone(), false, false, CombineMode::New);
        load.apply_edit("Load Selection", edit);
        assert_eq!(load.session.history.count(), 1);
        assert_eq!(load.session.history.label(0), "Load Selection");

        let mut document = PictureViewRust::default();
        assert!(document.transfer_document(shape, "D", false));
        assert_eq!(document.session.history.count(), 1);
        assert_eq!(document.session.history.label(0), "New");
    }

    #[test]
    fn transfer_leaves_source_unchanged() {
        let mut source = plain(32, 32);
        let data: Vec<u8> = (0..32 * 32).map(|i| (i % 199) as u8).collect();
        add_alpha(&mut source, data.clone());
        let source_selection = Selection::all(32, 32);
        source.session.set_selection(Some(source_selection.clone()));
        let source_history = source.session.history.count();

        let shape = source.transfer_source("alpha:0", 0).expect("channel");
        let mut target = plain(32, 32);
        let edit = target.transfer_channel(shape, "Copy", false);
        target.apply_edit("Duplicate Channel", edit);

        assert_eq!(channel_bytes(&source, 0), data);
        assert_eq!(selection_bytes(&source), source_selection.coverage_bytes());
        assert_eq!(source.session.history.count(), source_history);
    }

    #[test]
    fn same_size_is_identity_and_different_size_fits() {
        let shape = Selection::rect(32, 32, 4, 4, 16, 16);

        let mut same = plain(32, 32);
        let edit = same.transfer_load(shape.clone(), false, false, CombineMode::New);
        same.apply_edit("Load Selection", edit);
        assert_eq!(selection_bytes(&same), shape.coverage_bytes());

        let mut bigger = plain(48, 40);
        let edit = bigger.transfer_load(shape.clone(), false, true, CombineMode::New);
        bigger.apply_edit("Load Selection", edit);
        assert_eq!(
            selection_bytes(&bigger),
            shape.fit_centered(48, 40).coverage_bytes()
        );
    }

    #[test]
    fn new_channel_shares_source_tiles() {
        let shape = Selection::rect(64, 64, 4, 4, 40, 40);
        assert!(shape.data.is_tiled());
        let before = pictura_core::materializations();

        let mut target = plain(64, 64);
        let edit = target.transfer_channel(shape.clone(), "Copy", false);
        target.apply_edit("Duplicate Channel", edit);

        let channel = &target.session.doc.as_ref().expect("document").channels[0];
        assert!(channel.data.is_tiled());
        for tile in 0..shape.data.tile_count() {
            assert!(
                channel.data.shares_tile(tile, &shape.data),
                "tile {tile} is not shared"
            );
        }
        assert_eq!(pictura_core::materializations(), before);
    }

    #[test]
    fn same_size_load_keeps_tiles() {
        let shape = Selection::rect(64, 64, 4, 4, 40, 40);
        let before = pictura_core::materializations();
        let mut view = plain(64, 64);
        view.transfer_load(shape.clone(), false, false, CombineMode::New);

        let selection = view.session.selection.as_ref().expect("selection");
        assert!(selection.data.is_tiled());
        for tile in 0..shape.data.tile_count() {
            assert!(
                selection.data.shares_tile(tile, &shape.data),
                "tile {tile} is not shared"
            );
        }
        assert_eq!(selection.coverage_bytes(), shape.coverage_bytes());
        assert_eq!(pictura_core::materializations(), before);
    }

    #[test]
    fn fitted_transfer_is_tiled() {
        let shape = Selection::rect(32, 32, 4, 4, 16, 16);
        let mut view = plain(48, 40);
        view.transfer_load(shape.clone(), false, true, CombineMode::New);
        let fitted = view.session.selection.as_ref().expect("selection");
        assert!(fitted.data.is_tiled());
        assert_eq!(
            fitted.coverage_bytes(),
            shape.fit_centered(48, 40).coverage_bytes()
        );
    }

    #[test]
    fn selected_areas_channel_duplicates_canonical_coverage() {
        let mut view = plain(64, 64);
        let plane: Vec<u8> = (0..64 * 64).map(|i| (i % 200) as u8).collect();
        let index = add_alpha(&mut view, plane.clone());
        assert!(pictura_codec::set_channel_selected_areas(
            view.session.doc.as_mut().expect("document"),
            index,
            true
        ));
        let canonical: Vec<u8> = plane.iter().map(|value| 255 - value).collect();

        let shape = view.transfer_source("alpha:0", 0).expect("channel");
        assert_eq!(shape.coverage_bytes(), canonical);
        let edit = view.transfer_channel(shape, "Copy", false);
        view.apply_edit("Duplicate Channel", edit);

        let doc = view.session.doc.as_ref().expect("document");
        assert!(!pictura_codec::channel_selected_areas(doc, 1));
        assert_eq!(doc.channels[1].data.as_slice(), canonical.as_slice());
        assert_eq!(doc.channels[0].data.as_slice(), plane.as_slice());
    }

    #[test]
    fn disabled_mask_load_and_save_semantics() {
        let mut view = layered_view(64, 64);
        {
            let doc = view.session.doc.as_mut().expect("document");
            let layer = pictura_render::resolve_path_mut(doc, "0").expect("layer");
            let mask = LayerMask {
                rect: PsdRect {
                    top: 0,
                    left: 0,
                    bottom: 64,
                    right: 64,
                },
                data: Some(vec![0u8; 64 * 64].into()),
                disabled: true,
                ..LayerMask::default()
            };
            layer.mask = Some(mask);
        }

        let source = view.transfer_source("mask:0", 0).expect("mask");
        assert_eq!(source.coverage_bytes(), vec![255u8; 64 * 64]);

        let half = flat_selection(64, 64, 128);
        let edit = view.transfer_save(half, "mask:0", "", CombineMode::Add);
        assert!(!matches!(edit, Edit::None));
        let doc = view.session.doc.as_ref().expect("document");
        let mask = pictura_render::resolve_path(doc, "0")
            .and_then(|layer| layer.mask.as_ref())
            .expect("mask");
        let stored = mask.data.as_ref().expect("stored plane");
        assert_eq!(stored.as_slice()[0], 128);
    }

    #[test]
    fn read_only_destination_refuses_unharmed() {
        let mut view = plain(32, 32);
        view.session
            .set_selection(Some(Selection::rect(32, 32, 0, 0, 8, 8)));
        let before = selection_bytes(&view);
        let history = view.session.history.count();
        view.session
            .set_compatibility(NativeCompatibility::read_only_for_test());

        let shape = Selection::rect(32, 32, 0, 0, 32, 32);
        assert_eq!(
            view.transfer_load(shape.clone(), false, false, CombineMode::New),
            Edit::None
        );
        assert_eq!(view.transfer_channel(shape.clone(), "C", false), Edit::None);
        assert_eq!(
            view.transfer_save(shape, "new", "S", CombineMode::New),
            Edit::None
        );
        assert_eq!(selection_bytes(&view), before);
        assert_eq!(view.session.history.count(), history);
    }

    #[test]
    fn transfer_refusals_leave_no_state() {
        let shape = Selection::rect(32, 32, 4, 4, 16, 16);

        let mut empty = PictureViewRust::default();
        assert!(empty.transfer_source("", 0).is_none());
        assert_eq!(
            empty.transfer_channel(shape.clone(), "C", false),
            Edit::None
        );
        assert_eq!(
            empty.transfer_save(shape.clone(), "new", "S", CombineMode::New),
            Edit::None
        );
        assert_eq!(
            empty.transfer_load(shape.clone(), false, false, CombineMode::New),
            Edit::None
        );

        let mut view = plain(32, 32);
        assert!(view.transfer_source("", 0).is_none());
        let before = view.session.history.count();
        assert_eq!(
            view.transfer_save(shape.clone(), "bogus", "S", CombineMode::New),
            Edit::None
        );
        assert_eq!(
            view.transfer_save(shape.clone(), "mask:missing", "S", CombineMode::New),
            Edit::None
        );
        {
            let doc = view.session.doc.as_mut().expect("document");
            while doc.can_add_channel() {
                doc.channels.push(Channel {
                    id: doc.channels.len() as i16,
                    data: vec![0u8; 1].into(),
                });
            }
        }
        assert_eq!(view.transfer_channel(shape, "C", false), Edit::None);
        assert_eq!(view.session.history.count(), before);
    }

    #[test]
    fn transfer_adapters_do_not_export_dense_coverage() {
        for source in [
            include_str!("../../../cpp/frame_selection_dialogs.cpp"),
            include_str!("../../../cpp/file_drop_router.cpp"),
            include_str!("../../../cpp/panels/channels_panel.cpp"),
        ] {
            let compact: String = source.chars().filter(|ch| !ch.is_whitespace()).collect();
            for reader in [
                "selection_coverage_bytes(",
                "channel_coverage(",
                "color_row_coverage(",
            ] {
                assert!(
                    !compact.contains(reader),
                    "{reader} remains in a transfer adapter"
                );
            }
        }
    }

    #[test]
    fn invert_precedes_fit_and_combine() {
        let shape = Selection {
            width: 2,
            height: 2,
            data: Plane::from(vec![1u8, 2, 3, 4]),
        };
        let expected = shape.invert().fit_centered(3, 3);
        let mut fitted = plain(3, 3);
        let edit = fitted.transfer_load(shape, true, true, CombineMode::New);
        fitted.apply_edit("Load Selection", edit);
        assert_eq!(selection_bytes(&fitted), expected.coverage_bytes());

        let mut view = plain(4, 4);
        view.session.set_selection(Some(flat_selection(4, 4, 128)));
        let edit = view.transfer_load(flat_selection(4, 4, 64), true, false, CombineMode::Add);
        view.apply_edit("Load Selection", edit);
        assert_eq!(selection_bytes(&view), vec![191u8; 4 * 4]);
    }
}
