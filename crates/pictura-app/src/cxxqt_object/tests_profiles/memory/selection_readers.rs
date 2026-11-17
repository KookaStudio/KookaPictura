//! The selection-readers guard (#1268): with a tiled soft selection on a
//! layered view, one operation from each migrated group repeats the bridge's
//! calls, and the materialization counter must not move. The exports also
//! return the selection's bytes and leave it tiled.

use super::*;
use crate::cxxqt_object::clipboard::{clip_selection, copy_of};
use crate::cxxqt_object::helpers_composite::selection_to_mask;
use crate::cxxqt_object::picture::readers_fixture::{layered, view_of};
use pictura_core::{Channel, Plane};
use pictura_paint::healing::{move_layer, patch_layer, MoveOptions, PatchOptions};
use pictura_select::Selection;

const W: u32 = 613;
const H: u32 = 541;

/// A seeded soft selection over the fixture: a solid block holding whole
/// uniform tiles, an unselected margin, and noise across tile edges, tiled.
fn soft_selection() -> Selection {
    let (w, h) = (W as usize, H as usize);
    let mut state = 0x1268_u64;
    let data: Vec<u8> = (0..w * h)
        .map(|i| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let (x, y) = (i % w, i / w);
            if (60..w - 40).contains(&x) && (50..h - 30).contains(&y) {
                255
            } else if x < 30 || y < 20 {
                0
            } else {
                (state >> 56) as u8
            }
        })
        .collect();
    let mut plane = Plane::from(data);
    assert!(plane.tile(w, h));
    Selection {
        width: W,
        height: H,
        data: plane,
    }
}

fn view_with_selection() -> PictureViewRust {
    let mut view = view_of(layered(W, H, false));
    view.session.set_selection(Some(soft_selection()));
    view
}

#[test]
fn selection_readers_do_not_materialize() {
    let name = "selection_readers_do_not_materialize";
    if !alone_in_process(name) {
        return;
    }
    let mut view = view_with_selection();
    let doc = view.session.doc.clone().expect("document");
    let selection = view.session.selection.clone().expect("a selection");
    let before = pictura_core::materializations();

    assert!(copy_of(&view, "0", false).is_some(), "Copy");
    assert!(copy_of(&view, "0", true).is_some(), "Copy Merged");
    let coverage = clip_selection(&view).expect("a document-sized selection");
    let mut cleared = doc.clone();
    assert!(pictura_render::clear_layer(
        &mut cleared,
        "0",
        Some(&coverage)
    ));
    let mut lifted = doc.clone();
    assert!(!pictura_render::lift_selection(&mut lifted, "0", &coverage).is_empty());

    let mask = selection_to_mask(&selection, &doc);
    let mut filtered = doc.clone();
    let layer = pictura_render::resolve_path_mut(&mut filtered, "0").expect("layer");
    let blur = pictura_filters::Filter::GaussianBlur { radius: 2.0 };
    pictura_render::apply_filter(layer, &blur, Some(&mask), false).expect("filters");

    let mut patched = doc.clone();
    let patch = PatchOptions {
        dx: 20,
        ..PatchOptions::default()
    };
    assert!(patch_layer(&mut patched, "0", &selection.data, patch).is_ok());
    let mut moved = doc.clone();
    let shift = MoveOptions {
        dx: 20,
        ..MoveOptions::default()
    };
    assert!(move_layer(&mut moved, "0", &selection.data, shift).is_ok());

    assert_eq!(view.selection_coverage_bytes().len(), (W * H) as usize);
    assert!(view.begin_selection_move());
    assert!(view.preview_selection_move(9, -4));
    assert!(view.commit_selection_move());

    let materialized = pictura_core::materializations() - before;
    println!("{name}: {materialized} materialization(s)");
    assert_eq!(materialized, 0, "a selection reader materialized a plane");
}

#[test]
fn coverage_exports_leave_no_mirror() {
    let mut view = view_with_selection();
    let selection = view.session.selection.clone().expect("a selection");
    let want = selection.coverage_bytes();
    let doc = view.session.doc.as_mut().expect("document");
    doc.channels.push(Channel {
        id: 0,
        data: selection.data.clone(),
    });

    let (live, channel) = pictura_core::assert_no_materialization(|| {
        (
            view.selection_coverage_bytes(),
            view.channel_coverage("alpha:0"),
        )
    });
    assert_eq!(live, want);
    assert_eq!(channel, want);
    let row = view.color_row_coverage(0, 0b111);
    assert_eq!(row.len(), want.len());
    let live = view.session.selection.as_ref().expect("a selection");
    assert!(live.data.is_tiled(), "the export left the selection tiled");
    assert!(view.session.doc.as_ref().expect("document").channels[0]
        .data
        .is_tiled());
}
