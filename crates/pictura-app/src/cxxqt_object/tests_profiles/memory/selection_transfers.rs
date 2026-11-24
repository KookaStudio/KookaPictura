//! Heap guards for the coverage-transfer owner (#1400).
//!
//! The removed dense paths exported one 4250x4250 plane and rebuilt a second in
//! the receiver: at least 34.45 MiB simultaneous, independent of the retained
//! destination. The ignored reproducers model that removed handoff; they are
//! not measurements from a clean origin/master build. These guards bound the
//! owner instead: a tiled transfer stays below one retained
//! plane plus a small transport cushion, and a colour row below its one shared
//! derived/retained plane plus the band buffers.

use super::*;
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::picture::readers_fixture::{layered, view_of};
use pictura_core::{Channel, Plane};
use pictura_select::{CombineMode, Selection};

const MIB: usize = 1 << 20;

fn tiled_selection(side: u32) -> Selection {
    let data: Vec<u8> = (0..side as usize * side as usize)
        .map(|i| u8::from((i / 7) % 3 == 0) * 255)
        .collect();
    let mut plane = Plane::from(data);
    assert!(plane.tile(side as usize, side as usize));
    Selection {
        width: side,
        height: side,
        data: plane,
    }
}

fn opened(mut view: PictureViewRust) -> PictureViewRust {
    view.apply_edit("Open", Edit::Meta);
    view
}

/// Fail when `run` peaks at or above `output + allowance`, or materializes a
/// plane.
fn bounded_plane<T>(
    name: &str,
    output: usize,
    allowance: usize,
    setup: impl FnOnce() -> T + Send,
    run: impl FnOnce(&mut T) + Send,
) {
    if !alone_in_process(name) {
        return;
    }
    let (peak, materialized) = measure(setup, run);
    let limit = output + allowance;
    println!(
        "{name} at {GUARD_SIDE}²: heap peak {:.1} MiB ({peak} B), limit {:.1} MiB, \
         {materialized} materialization(s)",
        mib(peak),
        mib(limit)
    );
    assert!(
        peak < limit,
        "{name}: heap peak {peak} B reaches its output {output} B + {allowance} B"
    );
    assert_eq!(materialized, 0, "{name}: materialized a plane");
}

#[test]
fn same_document_save_holds_no_canvas_plane() {
    let plane = GUARD_SIDE as usize * GUARD_SIDE as usize;
    bounded_plane(
        "same_document_save",
        plane,
        MIB,
        || {
            let mut view = opened(view_of(layered(GUARD_SIDE, GUARD_SIDE, false)));
            view.session
                .set_selection(Some(tiled_selection(GUARD_SIDE)));
            view
        },
        |view| {
            let shape = view.transfer_source("", 0).expect("selection");
            let edit = view.transfer_save(shape, "new", "Guard", CombineMode::New);
            view.apply_edit("Save Selection", edit);
        },
    );
}

#[test]
fn tiled_transfer_heap_holds_no_canvas() {
    let plane = GUARD_SIDE as usize * GUARD_SIDE as usize;
    bounded_plane(
        "tiled_transfer",
        plane,
        MIB,
        || {
            let mut source = opened(view_of(layered(GUARD_SIDE, GUARD_SIDE, false)));
            source
                .session
                .set_selection(Some(tiled_selection(GUARD_SIDE)));
            let target = opened(view_of(layered(GUARD_SIDE, GUARD_SIDE, false)));
            (source, target)
        },
        |(source, target)| {
            let shape = source.transfer_source("", 0).expect("selection");
            let edit = target.transfer_channel(shape, "Guard", false);
            target.apply_edit("Duplicate Channel", edit);
        },
    );
}

#[test]
#[ignore = "red baseline for the removed dense path; run explicitly"]
fn dense_path_exceeds_the_transfer_bound() {
    let name = "dense_path_exceeds_the_transfer_bound";
    if !alone_in_process(name) {
        return;
    }
    let plane = GUARD_SIDE as usize * GUARD_SIDE as usize;
    let (peak, _) = measure(
        || {
            let mut source = opened(view_of(layered(GUARD_SIDE, GUARD_SIDE, false)));
            source
                .session
                .set_selection(Some(tiled_selection(GUARD_SIDE)));
            let target = opened(view_of(layered(GUARD_SIDE, GUARD_SIDE, false)));
            (source, target)
        },
        |(source, target)| {
            let bytes = source.selection_coverage_bytes();
            let shape = Selection {
                width: GUARD_SIDE,
                height: GUARD_SIDE,
                data: Plane::from(bytes.clone()),
            };
            let edit = target.transfer_channel(shape, "Dense", false);
            target.apply_edit("Duplicate Channel", edit);
        },
    );
    println!(
        "{name} at {GUARD_SIDE}²: heap peak {:.1} MiB, bound {:.1} MiB",
        mib(peak),
        mib(plane + MIB)
    );
    assert!(
        peak > plane + MIB,
        "{name}: dense path peak {peak} B is not above the bound {} B",
        plane + MIB
    );
}

#[test]
fn colour_row_transfer_heap_is_bounded() {
    let plane = GUARD_SIDE as usize * GUARD_SIDE as usize;
    bounded_plane(
        "colour_row_transfer",
        plane,
        12 * MIB,
        || {
            let source = sc::new_document(GUARD_SIDE);
            let target = sc::new_document(GUARD_SIDE);
            (source, target)
        },
        |(source, target)| {
            let shape = source.transfer_source("color:1", 0x7).expect("colour row");
            let edit = target.transfer_channel(shape, "Guard", false);
            target.apply_edit("Duplicate Channel", edit);
        },
    );
}

#[test]
fn capped_transfer_refuses_before_resampling() {
    bounded_plane(
        "capped_transfer",
        0,
        MIB,
        || {
            let mut target = opened(view_of(layered(GUARD_SIDE, GUARD_SIDE, false)));
            {
                let doc = target.session.doc.as_mut().expect("document");
                while doc.can_add_channel() {
                    let id = doc
                        .channels
                        .iter()
                        .map(|channel| channel.id)
                        .max()
                        .unwrap_or(-1)
                        + 1;
                    doc.channels.push(Channel {
                        id,
                        data: vec![0u8; 1].into(),
                    });
                }
            }
            let shape = Selection::rect(16, 16, 0, 0, 16, 16);
            assert!(shape.data.is_tiled());
            (target, shape)
        },
        |(target, shape)| {
            assert_eq!(
                target.transfer_channel(shape.clone(), "Refused", false),
                Edit::None
            );
        },
    );
}

#[test]
#[ignore = "red baseline for the removed dense colour-row path; run explicitly"]
fn dense_colour_row_exceeds_the_transfer_bound() {
    let name = "dense_colour_row_exceeds_the_transfer_bound";
    if !alone_in_process(name) {
        return;
    }
    let plane = GUARD_SIDE as usize * GUARD_SIDE as usize;
    let (peak, _) = measure(
        || {
            let source = sc::new_document(GUARD_SIDE);
            let target = sc::new_document(GUARD_SIDE);
            (source, target)
        },
        |(source, target)| {
            let bytes = source.color_row_coverage(1, 0x7);
            let shape = Selection {
                width: GUARD_SIDE,
                height: GUARD_SIDE,
                data: Plane::from(bytes.clone()),
            };
            let edit = target.transfer_channel(shape, "Dense", false);
            target.apply_edit("Duplicate Channel", edit);
        },
    );
    println!(
        "{name} at {GUARD_SIDE}²: heap peak {:.1} MiB, bound {:.1} MiB",
        mib(peak),
        mib(plane + 12 * MIB)
    );
    assert!(
        peak > plane + 12 * MIB,
        "{name}: dense path peak {peak} B is not above the bound {} B",
        plane + 12 * MIB
    );
}
