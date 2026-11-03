//! Whole-document restores on a 16507×16196 document: what the history walk and
//! the display cost for each kind of edit an undo can cross (#1098).

use super::super::document_session::DocumentSession;
use super::super::edit::Edit;
use super::super::helpers_composite::{level0_buffer, planes_of};
use super::{ms, pixel_layer, LayerTarget, PictureViewRust};
use crate::history::Restored;
use pictura_core::{BitDepth, Channel, ColorMode, Document, LayerStack};
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
    doc.composite = pictura_render::composite_rgba(&doc);
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    };
    view.reset_pyramid();
    view.apply_edit("Open", Edit::Record);
    view
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
    reorder.apply_edit("Reorder", Edit::Structural { damage: None });
    undo_and_show("identical reorder", &mut reorder);
    drop(reorder);

    let mut channel = view();
    let doc = channel.session.doc.as_mut().expect("document");
    doc.channels[0].data.as_mut_slice()[3] = 9;
    channel.apply_edit("Channel", Edit::Record);
    undo_and_show("channel", &mut channel);
    drop(channel);

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

    let t = Instant::now();
    let level0 = level0_buffer(resize.session.doc.as_ref().expect("document"));
    ms("full size level 0", t.elapsed());
    let t = Instant::now();
    let pyramid = pictura_render::ViewPyramid::rebuild(planes_of(&level0).expect("rgba"));
    ms("full size pyramid", t.elapsed());
    drop((level0, pyramid));

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
    let job = resize.reset_pyramid_deferred();
    ms("deferred display", t.elapsed());
    assert!(job.is_some(), "a large document defers");
}
