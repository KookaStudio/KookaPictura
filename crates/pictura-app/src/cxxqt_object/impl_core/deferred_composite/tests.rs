use super::*;
use crate::cxxqt_object::document_session::DocumentSession;
use crate::cxxqt_object::helpers_composite::{composite_regions, RegionComposite};
use crate::cxxqt_object::impl_transform::translate_move_group;
use crate::cxxqt_object::picture::picture;
use crate::cxxqt_object::tests::{assert_display_current, pixel_layer, present_all};
use pictura_core::{BitDepth, ColorMode, Document, LayerStack};
use std::borrow::Cow;

fn view(side: u32) -> PictureViewRust {
    let mut doc = Document::new(side, side, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![
        pixel_layer("under", side, side, (120, 170, 220)),
        pixel_layer("over", side, side, (220, 90, 40)),
    ]);
    doc.composite = pictura_render::composite_rgba(&doc);
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    };
    view.reset_pyramid();
    present_all(&view);
    view.layer_target = LayerTarget::only("1");
    view.apply_edit("Open", Edit::Record);
    view
}

fn move_and_defer(
    view: &mut PictureViewRust,
    dx: i32,
    dy: i32,
) -> (CompositeJob, Vec<RegionComposite>) {
    let dirty = {
        let doc = view.session.doc.as_mut().expect("document");
        translate_move_group(doc, &["1".to_string()], dx, dy).expect("moved")
    };
    let edit = Edit::regions(vec![dirty], false);
    assert!(view.defer_move_composite("Move Layer", &edit));
    let job = view.pending_move_job.take().expect("deferred job");
    let regions = composite_regions(&job.doc, &job.rects, job.gpu_compute);
    (job, regions)
}

fn full_composite(view: &PictureViewRust) -> pictura_core::PixelBuffer {
    let doc = view.session.doc.as_ref().expect("document");
    pictura_render::composite_rgba(doc)
}

#[test]
fn a_landed_move_composite_matches_a_full_composite() {
    let mut view = view(64);
    let (job, regions) = move_and_defer(&mut view, 5, 7);
    let settled = view
        .land_move_composite(job.generation, regions)
        .expect("the current job lands");

    assert!(settled.blit.is_some());
    assert!(!view.move_composite_pending());
    assert_display_current(&view);
    assert_eq!(view.session.history.count(), 2);
    assert_eq!(view.session.history.label(1), "Move Layer");
}

#[test]
fn a_settled_composite_matches_a_full_composite() {
    let mut view = view(64);
    let (_job, _regions) = move_and_defer(&mut view, 3, 4);
    let settled = view.settle_move_composite().expect("settles");

    assert!(settled.blit.is_some());
    assert!(!view.move_composite_pending());
    assert_display_current(&view);
    assert_eq!(view.session.history.count(), 2);
    assert_eq!(view.session.history.label(1), "Move Layer");
}

#[test]
fn a_superseded_move_composite_is_dropped() {
    let mut view = view(64);
    let (job, regions) = move_and_defer(&mut view, 3, 4);
    view.settle_move_composite().expect("settles");
    let revision = view.canvas_revision;

    assert!(view.land_move_composite(job.generation, regions).is_none());
    assert_eq!(
        view.canvas_revision, revision,
        "the late result changes no tile"
    );
    assert_display_current(&view);
    assert_eq!(
        view.session.history.count(),
        2,
        "the late result adds nothing"
    );
}

#[test]
fn a_replaced_document_drops_the_move_composite() {
    let mut view = view(64);
    let (job, regions) = move_and_defer(&mut view, 3, 4);
    view.clear_document_scoped_state();
    assert!(!view.move_composite_pending());

    let replacement = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    let composite = replacement.composite.clone();
    view.session.doc = Some(replacement);
    let revision = view.canvas_revision;

    assert!(view.land_move_composite(job.generation, regions).is_none());
    assert_eq!(
        view.session.doc.as_ref().expect("document").composite,
        composite
    );
    assert_eq!(
        view.canvas_revision, revision,
        "the late result changes no tile"
    );
}

#[test]
fn a_settle_before_an_edit_keeps_one_state() {
    let mut view = view(64);
    let (_job, _regions) = move_and_defer(&mut view, 3, 4);
    view.settle_move_composite().expect("settles");
    let applied = view.apply_edit("Edit", Edit::Meta);

    assert_eq!(applied.declaration, Some(Edit::Meta));
    assert_eq!(view.session.history.count(), 3);
    assert_eq!(view.session.history.label(1), "Move Layer");
    assert_eq!(view.session.history.label(2), "Edit");
}

#[test]
fn can_undo_counts_a_pending_move() {
    let mut view = view(64);
    assert!(!view.can_undo());
    let (_job, _regions) = move_and_defer(&mut view, 3, 4);

    assert!(view.can_undo(), "the pending move is undoable");
    assert!(!view.can_redo());

    view.settle_move_composite().expect("settles");
    assert!(view.can_undo());
    assert!(!view.can_redo());
}

#[test]
fn a_pending_composite_is_not_current() {
    let mut view = view(64);
    let (_job, _regions) = move_and_defer(&mut view, 3, 4);
    let expected = full_composite(&view);
    let doc = view.session.doc.as_ref().expect("document");
    let served = picture(doc, &view);

    assert!(matches!(served, Cow::Owned(_)));
    assert_eq!(served.into_owned(), expected);

    view.settle_move_composite().expect("settles");
    present_all(&view);
    let doc = view.session.doc.as_ref().expect("document");
    let before = crate::cxxqt_object::picture::composites_so_far();
    assert_eq!(picture(doc, &view).into_owned(), full_composite(&view));
    assert_eq!(
        crate::cxxqt_object::picture::composites_so_far(),
        before,
        "a settled composite is served from the tiles"
    );
}

#[test]
fn only_a_large_area_defers() {
    let mut view = PictureViewRust::default();
    view.session.doc = Some(Document::new(4000, 4000, ColorMode::Rgb, BitDepth::Eight));
    let whole = PsdRect {
        top: 0,
        left: 0,
        bottom: 4000,
        right: 4000,
    };
    let short = PsdRect {
        top: 0,
        left: 0,
        bottom: 4000,
        right: 3999,
    };

    assert!(view.should_defer_move(&Edit::regions(vec![whole], false)));
    assert!(!view.should_defer_move(&Edit::regions(vec![short], false)));

    view.session.doc = Some(Document::new(3999, 4000, ColorMode::Rgb, BitDepth::Eight));
    assert!(!view.should_defer_move(&Edit::regions(vec![whole], false)));
}

#[test]
fn a_layered_large_move_marks_tiles_instead_of_deferring() {
    let mut view = PictureViewRust::default();
    let side = 4200;
    let mut doc = Document::new(side, side, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("under", side, side, (120, 170, 220))]);
    view.session.doc = Some(doc);
    let whole = Edit::regions(
        vec![PsdRect {
            top: 0,
            left: 0,
            bottom: side as i32,
            right: side as i32,
        }],
        false,
    );

    assert!(
        !view.should_defer_move(&whole),
        "a layered RGB document composites its display tiles lazily"
    );
    view.move_composite_delay_for_test = 1;
    assert!(
        view.should_defer_move(&whole),
        "the test delay still forces the deferred path"
    );
}

#[test]
fn a_busy_display_does_not_defer() {
    let mut view = PictureViewRust::default();
    view.session.doc = Some(Document::new(4000, 4000, ColorMode::Rgb, BitDepth::Eight));
    let whole = Edit::regions(
        vec![PsdRect {
            top: 0,
            left: 0,
            bottom: 4000,
            right: 4000,
        }],
        false,
    );

    view.session.force_test_session();
    assert!(!view.should_defer_move(&whole));
    view.session.live = Default::default();

    view.pending_previews.opacity_changed = true;
    assert!(!view.should_defer_move(&whole));
    view.pending_previews.opacity_changed = false;

    view.mask_solo = Some("0".to_string());
    assert!(!view.should_defer_move(&whole));
}

#[test]
fn a_deferred_whole_landing_skips_the_display_conversion() {
    use crate::cxxqt_object::helpers_composite::DISPLAY_COPIES;
    // Over the synchronous tile limit (16 MP), with a profile whose display
    // conversion is not the identity. Its tiles are not presented at once:
    // the test helpers only do that for small documents.
    let side = 4100;
    let mut doc = Document::new(side, side, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![
        pixel_layer("under", side, side, (120, 170, 220)),
        pixel_layer("over", side, side, (220, 90, 40)),
    ]);
    doc.document_icc = Some(pictura_color::Profile::adobe_rgb().to_icc().into());
    doc.composite = pictura_render::composite_rgba(&doc);
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    };
    view.layer_target = LayerTarget::only("1");
    view.apply_edit("Open", Edit::Record);
    let copies = DISPLAY_COPIES.get();
    let (job, regions) = move_and_defer(&mut view, 5, 7);
    assert_eq!(regions.len(), 1);
    assert!(regions[0].srgb.is_none(), "the whole region was converted");
    // The landed region is the moved document's composite.
    let full = pictura_render::composite_rgba(&job.doc);
    let probe = |buf: &pictura_core::PixelBuffer| {
        let plane = (buf.width * buf.height) as usize;
        (0..4)
            .map(|c| buf.data.sample(c * plane + 2000 * side as usize + 2000))
            .collect::<Vec<_>>()
    };
    assert_eq!(probe(&regions[0].raw), probe(&full), "the region is stale");
    let revision = view.canvas_revision;
    view.land_move_composite(job.generation, regions)
        .expect("the current job lands");
    assert_eq!(DISPLAY_COPIES.get(), copies, "a display copy was made");
    assert_ne!(
        view.canvas_revision, revision,
        "the display was not invalidated"
    );
}
