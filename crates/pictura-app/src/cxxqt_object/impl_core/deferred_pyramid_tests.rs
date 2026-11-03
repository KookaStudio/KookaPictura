use super::super::super::document_session::DocumentSession;
use super::super::super::tests::pixel_layer;
use super::*;
use pictura_core::{BitDepth, ColorMode, Document, LayerMask, LayerStack, PsdRect};

const SIDE: u32 = 4200;

fn view(side: u32) -> PictureViewRust {
    let mut doc = Document::new(side, side, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("px", side, side, (9, 120, 240))]);
    doc.composite = pictura_render::composite_rgba(&doc);
    let n = side as usize * side as usize;
    for i in (0..n).step_by(7) {
        doc.composite.data[i] = (i % 251) as u8;
    }
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    };
    view.reset_pyramid();
    view
}

fn assert_exact(view: &PictureViewRust) {
    let exact = ViewPyramid::rebuild(planes_of(view.level0.as_ref().unwrap()).unwrap());
    assert_eq!(view.pyramid.level_count(), exact.level_count());
    for level in 1..exact.level_count() {
        let (ours, theirs) = (
            view.pyramid.level_view(level).unwrap(),
            exact.level_view(level).unwrap(),
        );
        assert!(ours.data() == theirs.data(), "level {level} differs");
    }
}

fn built(job: &PyramidJob) -> ViewPyramid {
    ViewPyramid::rebuild(planes_of(&job.level0).unwrap())
}

#[test]
fn a_landed_pyramid_equals_a_rebuild() {
    let mut view = view(SIDE);
    let job = view
        .reset_pyramid_deferred()
        .expect("a large document defers");
    let coarse = view.deferred_pyramid.expect("deferred").level;
    assert!(coarse > 0);
    let revision = view.canvas_revision;
    assert!(view.land_pyramid(job.generation, built(&job)));
    assert_eq!(view.deferred_pyramid, None);
    assert_ne!(view.canvas_revision, revision, "the canvas drops its crop");
    assert_exact(&view);
}

#[test]
fn a_superseded_pyramid_is_dropped() {
    let mut view = view(SIDE);
    let first = view.reset_pyramid_deferred().expect("deferred");
    let second = view.reset_pyramid_deferred().expect("deferred");
    assert!(!view.land_pyramid(first.generation, built(&first)));
    assert!(
        view.deferred_pyramid.is_some(),
        "the later build is pending"
    );
    assert!(view.land_pyramid(second.generation, built(&second)));
}

#[test]
fn a_synchronous_reset_supersedes_the_build() {
    let mut view = view(SIDE);
    let job = view.reset_pyramid_deferred().expect("deferred");
    view.reset_pyramid();
    assert!(!view.land_pyramid(job.generation, built(&job)));
    assert_exact(&view);
}

#[test]
fn a_write_settles_the_deferred_pyramid() {
    let mut view = view(SIDE);
    let job = view.reset_pyramid_deferred().expect("deferred");
    view.update_pyramid(PsdRect {
        top: 0,
        left: 0,
        bottom: 64,
        right: 64,
    });
    assert_eq!(view.deferred_pyramid, None);
    assert_exact(&view);
    assert!(!view.land_pyramid(job.generation, built(&job)));
}

#[test]
fn a_small_document_does_not_defer() {
    let mut view = view(512);
    assert!(view.reset_pyramid_deferred().is_none());
    assert_eq!(view.deferred_pyramid, None);
    assert_exact(&view);
}

#[test]
fn a_composite_restore_rebuilds_the_soloed_mask() {
    let mut view = view(512);
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: 512,
        right: 512,
    };
    let doc = view.session.doc.as_mut().expect("document");
    doc.layers[0].mask = Some(LayerMask {
        rect,
        default_color: 255,
        data: Some(vec![200u8; 512 * 512].into()),
        ..LayerMask::default()
    });
    view.mask_solo = Some("0".into());
    view.rebuild_mask_solo_source();
    let before = view.mask_level0.clone().expect("solo source");

    let doc = view.session.doc.as_mut().expect("document");
    let mask = doc.layers[0].mask.as_mut().expect("mask");
    mask.data.as_mut().expect("data").as_mut_slice().fill(10);
    let frame = view.frame_revision;
    view.refresh_mask_solo();

    assert!(view.mask_level0.as_ref() != Some(&before));
    assert_ne!(
        view.frame_revision, frame,
        "the canvas takes the new source"
    );
}
