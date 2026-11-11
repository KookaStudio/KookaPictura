//! Whole-document restores on a 16507×16196 document: what the history walk and
//! the display cost for each kind of edit an undo can cross (#1098).

use super::super::document_session::DocumentSession;
use super::super::edit::Edit;
use super::super::helpers_composite::planes_of;
use super::{ms, pixel_layer, LayerTarget, PictureViewRust};
use crate::history::Restored;
use pictura_core::{BitDepth, Channel, ColorMode, Document, LayerMask, LayerStack, PsdRect};
use pictura_render::TileSource;
use std::time::Instant;

const WIDTH: u32 = 16507;
const HEIGHT: u32 = 16196;

fn view() -> PictureViewRust {
    let mut doc = Document::new(WIDTH, HEIGHT, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![
        pixel_layer("under", WIDTH, HEIGHT, (120, 170, 220)),
        pixel_layer("over", WIDTH, HEIGHT, (120, 170, 220)),
    ]);
    doc.channels.push(Channel {
        id: 0,
        data: vec![0u8; WIDTH as usize * HEIGHT as usize].into(),
    });
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    };
    view.composite_into_doc();
    view.reset_pyramid();
    view.apply_edit("Open", Edit::Record);
    view
}

/// The viewport the #1216 budget is set for.
const VIEWPORT: (u32, u32) = (1920, 1080);

/// Present the fit view of `source` as the canvas does after a restore: the
/// GUI thread's crop (stand-ins for what is missing), then the worker's build
/// of the exact visible tiles, timed apart.
fn present_fit(label: &str, view: &PictureViewRust, source: TileSource) {
    let t = Instant::now();
    let (level, fit) = view.fit_view(VIEWPORT.0, VIEWPORT.1);
    let (_, missing) = view.tile_crop(source, level, fit);
    ms(
        &format!("{label} visible presentation (GUI thread)"),
        t.elapsed(),
    );
    let t = Instant::now();
    let built = view.run_tile_job(&missing);
    ms(
        &format!("{label} visible tiles ({built} at level {level}, worker)"),
        t.elapsed(),
    );
}

/// Undo, then present it the way `show_restored` does, timing each half.
fn undo_and_show(label: &str, view: &mut PictureViewRust) {
    let session = &mut view.session;
    let doc = session.doc.as_mut().expect("document");
    let t = Instant::now();
    let restored = session
        .history
        .undo_live(doc, &mut session.selection, &mut LayerTarget::default())
        .expect("a state to undo");
    ms(&format!("{label} history ({restored:?})"), t.elapsed());
    let t = Instant::now();
    match restored {
        Restored::Region(rect) | Restored::Composite(rect) => {
            view.refresh_from_composite(rect);
        }
        Restored::Everywhere => view.reset_pyramid(),
    }
    ms(&format!("{label} display"), t.elapsed());
}

#[test]
#[ignore = "16507x16196 restore profile; run explicitly with --ignored --nocapture"]
fn structural_restore_profile() {
    let mut new_layer = view();
    let doc = new_layer.session.doc.as_mut().expect("document");
    assert!(pictura_render::add_layer(doc, -1, "Layer") >= 0);
    new_layer.apply_edit("New Layer", Edit::Inserted);
    undo_and_show("new layer", &mut new_layer);
    drop(new_layer);

    let mut reorder = view();
    let doc = reorder.session.doc.as_mut().expect("document");
    assert!(doc.layers.move_node(&[0], &[], 2).is_ok());
    let t = Instant::now();
    reorder.apply_edit("Reorder", Edit::Structural { damage: None });
    ms("identical reorder capture", t.elapsed());
    undo_and_show("identical reorder", &mut reorder);
    drop(reorder);

    let mut channel = view();
    let doc = channel.session.doc.as_mut().expect("document");
    doc.channels[0].data.as_mut_slice()[3] = 9;
    channel.apply_edit("Channel", Edit::Record);
    undo_and_show("channel", &mut channel);
    drop(channel);

    let mut solo = view();
    let doc = solo.session.doc.as_mut().expect("document");
    doc.layers[0].mask = Some(LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 2048,
            right: 2048,
        },
        default_color: 255,
        data: Some(vec![128u8; 2048 * 2048].into()),
        ..LayerMask::default()
    });
    solo.mask_solo = Some("0".into());
    solo.rebuild_mask_solo_source();
    let t = Instant::now();
    solo.reset_pyramid();
    ms("solo mask restore display (GUI thread)", t.elapsed());
    present_fit("solo mask", &solo, TileSource::Mask);
    drop(solo);

    let mut resize = view();
    let doc = resize.session.doc.as_mut().expect("document");
    pictura_render::resize_document(
        doc,
        WIDTH / 2,
        HEIGHT / 2,
        pictura_render::Resample::Nearest,
    )
    .expect("resize");
    resize.apply_edit("Image Size", Edit::Full);
    undo_and_show("resize", &mut resize);

    present_fit("full size", &resize, TileSource::Composite);

    let session = &mut resize.session;
    let doc = session.doc.as_mut().expect("document");
    let t = Instant::now();
    let restored = session
        .history
        .jump_live(1, doc, &mut session.selection, &mut LayerTarget::default())
        .expect("a state to jump to");
    ms(&format!("jump history ({restored:?})"), t.elapsed());
    let t = Instant::now();
    resize.reset_pyramid();
    ms("jump display", t.elapsed());

    let session = &mut resize.session;
    let doc = session.doc.as_mut().expect("document");
    session
        .history
        .jump_live(0, doc, &mut session.selection, &mut LayerTarget::default())
        .expect("a state to jump to");
    let t = Instant::now();
    resize.reset_pyramid();
    ms("deferred display", t.elapsed());
    present_fit("restored", &resize, TileSource::Composite);
}

/// A soloed mask's display plane on an 8000² document: the same fill on one
/// core and on every core. Small enough not to swap on a 7 GiB machine.
#[test]
#[ignore = "8000x8000 solo mask profile; run explicitly with --ignored --nocapture"]
fn solo_mask_profile() {
    const SIDE: u32 = 8000;
    let mut doc = Document::new(SIDE, SIDE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("under", SIDE, SIDE, (120, 170, 220))]);
    doc.layers[0].mask = Some(LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 2048,
            right: 2048,
        },
        default_color: 255,
        data: Some(vec![128u8; 2048 * 2048].into()),
        ..LayerMask::default()
    });
    let mask = doc.layers[0].mask.as_ref().expect("mask");

    let one = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("a one-thread pool");
    let t = Instant::now();
    let serial = one.install(|| super::super::impl_core::mask_display_plane(mask, SIDE, SIDE));
    ms("solo mask plane (1 thread)", t.elapsed());
    drop(serial);

    let t = Instant::now();
    let plane = super::super::impl_core::mask_display_plane(mask, SIDE, SIDE);
    ms("solo mask plane (all cores)", t.elapsed());
    let t = Instant::now();
    let pyramid = pictura_render::ViewPyramid::rebuild(planes_of(&plane).expect("rgba"));
    ms("solo mask whole pyramid (the old display)", t.elapsed());
    drop((pyramid, plane));

    // The tile display: no document-sized plane, only the visible tiles.
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc.clone()),
        gpu_compute: false,
        ..Default::default()
    };
    view.composite_into_doc();
    view.mask_solo = Some("0".into());
    view.rebuild_mask_solo_source();
    let t = Instant::now();
    view.reset_pyramid();
    ms("solo mask restore display (GUI thread)", t.elapsed());
    present_fit("solo mask", &view, TileSource::Mask);
}
