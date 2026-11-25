use super::*;

const MIB: i64 = 1024 * 1024;

#[test]
fn a_refusal_is_remembered_until_the_next_fill() {
    // 1 GiB available leaves 768 MiB of room.
    set_available_memory_for_test(1024 * MIB);
    clear();
    assert!(admit_resident(700 * MIB as u64).admitted());
    assert!(!edit_refused_for_memory());
    assert!(!admit_resident(800 * MIB as u64).admitted());
    assert!(edit_refused_for_memory());
    clear();
    assert!(!edit_refused_for_memory());
    set_available_memory_for_test(-1);
}

// [mga_admits]
#[test]
fn a_fitting_edit_is_admitted() {
    set_available_memory_for_test(1024 * MIB);
    set_scratch_available_for_test(64 * MIB);
    let outcome = admit_resident(700 * MIB as u64);
    let refusal = last_refusal();
    set_available_memory_for_test(-1);
    set_scratch_available_for_test(-1);
    assert_eq!(outcome, Admission::Admitted { spill: false });
    assert_eq!(refusal, None, "an admitted edit records no refusal");
    assert!(!edit_refused_for_memory());
    assert!(!edit_refused_for_scratch());
    assert!(!edit_refused_for_scratch_full());
}

// [mga_refuses]
#[test]
fn an_edit_beyond_the_room_is_refused() {
    set_available_memory_for_test(1024 * MIB);
    let outcome = admit_resident(800 * MIB as u64);
    let memory = edit_refused_for_memory();
    let scratch = edit_refused_for_scratch();
    let scratch_full = edit_refused_for_scratch_full();
    set_available_memory_for_test(-1);
    assert_eq!(outcome, Admission::Refused(Refusal::Memory));
    assert!(memory);
    assert!(!scratch, "not an unreadable-tile refusal");
    assert!(!scratch_full, "not a scratch-full refusal");
}

#[test]
fn a_scratch_refusal_supersedes_an_earlier_memory_refusal() {
    set_available_memory_for_test(1024 * MIB);
    assert_eq!(
        admit_resident(800 * MIB as u64),
        Admission::Refused(Refusal::Memory)
    );
    assert!(edit_refused_for_memory());
    refuse_for_scratch();
    assert_eq!(last_refusal(), Some(Refusal::ScratchUnreadable));
    assert!(edit_refused_for_scratch());
    assert!(!edit_refused_for_memory());
    assert!(!edit_refused_for_scratch_full());
    clear();
    memory::record_refusal("an edit", 1);
    assert!(edit_refused_for_memory());
    clear();
    set_available_memory_for_test(-1);
}

// [mga_scratch_full]
#[test]
fn a_scratch_full_commit_is_refused_with_its_own_reason() {
    // 1 GiB available leaves 768 MiB of room: 100 MiB of working fits, but
    // 2 GiB of spillable output does not fit the room with no scratch free.
    set_available_memory_for_test(1024 * MIB);
    set_scratch_available_for_test(0);
    let cost = CommitCost {
        working: 100 * MIB as u64,
        spillable: 2048 * MIB as u64,
    };
    let outcome = admit_commit(&cost);
    let scratch_full = edit_refused_for_scratch_full();
    set_available_memory_for_test(-1);
    set_scratch_available_for_test(-1);
    assert_eq!(outcome, Admission::Refused(Refusal::ScratchFull));
    assert!(scratch_full, "the reason names the scratch disks");
}

// [mga_spill_marked]
#[test]
fn an_admitted_commit_reports_whether_it_spills() {
    // 1 GiB available leaves 768 MiB of room.
    set_available_memory_for_test(1024 * MIB);
    set_scratch_available_for_test(4096 * MIB);
    let spilling = admit_commit(&CommitCost {
        working: 100 * MIB as u64,
        spillable: 1024 * MIB as u64,
    });
    let fitting = admit_commit(&CommitCost {
        working: 100 * MIB as u64,
        spillable: 200 * MIB as u64,
    });
    set_available_memory_for_test(-1);
    set_scratch_available_for_test(-1);
    assert_eq!(spilling, Admission::Admitted { spill: true });
    assert_eq!(fitting, Admission::Admitted { spill: false });
}

// [mga_quick_mask_clean]
#[test]
fn a_quick_mask_gate_clears_a_prior_refusal() {
    set_available_memory_for_test(1024 * MIB);
    // An earlier edit is refused, then the room comes back.
    assert_eq!(
        admit_resident(800 * MIB as u64),
        Admission::Refused(Refusal::Memory)
    );
    assert!(edit_refused_for_memory());
    // The Quick Mask gate runs no other gate, so it must not inherit that
    // refusal. Drive the real gate, not another admission.
    let mut doc = Document::new(40, 30, pictura_core::ColorMode::Rgb, BitDepth::Eight);
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![crate::cxxqt_object::tests::pixel_layer(
            "layer",
            40,
            30,
            (0, 0, 0),
        )]);
    let rust = crate::cxxqt_object::PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        ..Default::default()
    };
    let filtered = rust.quick_mask_document(false).is_some();
    let stale = edit_refused_for_memory();
    set_available_memory_for_test(-1);
    assert!(filtered, "the Quick Mask gate has room again");
    assert!(
        !stale,
        "the Quick Mask gate left the earlier refusal readable"
    );
}

// [mga_no_stale]
#[test]
fn an_admitted_edit_records_no_refusal() {
    set_available_memory_for_test(1024 * MIB);
    // A refusal first, then room again.
    assert_eq!(
        admit_resident(800 * MIB as u64),
        Admission::Refused(Refusal::Memory)
    );
    assert!(edit_refused_for_memory());
    // An admitted edit makes no refusal of its own.
    let outcome = admit_resident(100 * MIB as u64);
    let after_admission = edit_refused_for_memory();
    set_available_memory_for_test(-1);
    assert_eq!(outcome, Admission::Admitted { spill: false });
    assert!(!after_admission, "the refusal did not outlive its edit");

    // An edit that makes no admission call at all starts by clearing, so it
    // must not inherit the earlier refusal either.
    set_available_memory_for_test(1024 * MIB);
    assert_eq!(
        admit_resident(800 * MIB as u64),
        Admission::Refused(Refusal::Memory)
    );
    assert!(edit_refused_for_memory());
    clear();
    let after_clear = edit_refused_for_memory();
    set_available_memory_for_test(-1);
    assert!(
        !after_clear,
        "a non-admitting edit inherited an earlier refusal"
    );
}

#[test]
fn a_write_forks_the_tiles_it_crosses() {
    let mut doc = Document::new(
        100,
        300,
        pictura_core::ColorMode::Rgb,
        pictura_core::BitDepth::Eight,
    );
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![crate::cxxqt_object::tests::pixel_layer(
            "layer",
            100,
            300,
            (0, 0, 0),
        )]);
    let planes = doc.layers[0]
        .channels()
        .iter()
        .filter(|c| c.id >= -1)
        .count() as u64;
    let rows = |top, bottom| PsdRect {
        top,
        left: 10,
        bottom,
        right: 20,
    };
    // Rows 70..140 lie in tile row 0: 256 rows, clipped to the 100 columns.
    assert_eq!(fork_bytes(&doc, "0", rows(70, 140)), planes * 256 * 100);
    // The last tile row is cut at the layer's bottom: rows 256..300.
    assert_eq!(fork_bytes(&doc, "0", rows(290, 300)), planes * 44 * 100);
    // A box off the layer grows it to the union: 100×500, rows 256..500.
    assert_eq!(fork_bytes(&doc, "0", rows(400, 500)), planes * 244 * 100);
    assert_eq!(fork_bytes(&doc, "9", rows(0, 10)), 0, "no such layer");
}

// [tp_gate]
/// An RGB/8 document of `side`² whose one layer covers the canvas, sized
/// without its planes: the cost reads only the rect and the channel ids.
fn full_canvas(side: u32) -> Document {
    let mut doc = Document::new(1, 1, pictura_core::ColorMode::Rgb, BitDepth::Eight);
    let mut layer = crate::cxxqt_object::tests::pixel_layer("layer", 1, 1, (0, 0, 0));
    layer.rect = PsdRect {
        top: 0,
        left: 0,
        bottom: side as i32,
        right: side as i32,
    };
    doc.width = side;
    doc.height = side;
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    doc
}

fn whole(side: u32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: side as i32,
        right: side as i32,
    }
}

// [ocf_gate_spills]
#[test]
fn a_large_fill_spills_its_tiles() {
    let doc = full_canvas(17_000);
    let bounds = whole(17_000);
    let cost = fill_cost(&doc, "0", bounds, 0);
    // 2 GiB available leaves 1.75 GiB of room, under the old RAM charge.
    let room = (2048 - 256) * MIB as u64;
    assert!(fork_bytes(&doc, "0", bounds) + refresh_bytes(bounds) > room);
    assert_eq!(cost.working, 0);
    set_available_memory_for_test(2048 * MIB);
    set_scratch_available_for_test(64 * 1024 * MIB);
    clear();
    let admitted = admit_spilling(cost.working, cost.spillable);
    let refused = edit_refused_for_memory();
    set_available_memory_for_test(-1);
    set_scratch_available_for_test(-1);
    assert!(admitted);
    assert!(!refused);
}

/// Gate an Image Size of a 17000² full-canvas layer to 150 % on a host with
/// 1.75 GiB of room and `scratch` bytes free: whether it was admitted,
/// whether it expects to spill, and whether a refusal was scratch-full.
fn gate_large_image_size(scratch: i64) -> (bool, bool, bool) {
    let doc = full_canvas(17_000);
    let cost = image_size_cost(&doc, 25_500, 25_500);
    set_available_memory_for_test(2048 * MIB);
    set_scratch_available_for_test(scratch);
    clear();
    let admitted = admit_spilling(cost.working, cost.spillable);
    let spills = expects_spill(&cost);
    let scratch_full = edit_refused_for_scratch_full();
    set_available_memory_for_test(-1);
    set_scratch_available_for_test(-1);
    (admitted, spills, scratch_full)
}

/// A 17000² document with a Background and one full-canvas layer.
fn background_and_layer(side: u32) -> Document {
    let mut doc = full_canvas(side);
    let layer = doc.layers[0].clone();
    let mut background = layer.clone();
    background.background = true;
    doc.layers = pictura_core::LayerStack::from_layers(vec![background, layer]);
    doc
}

/// Gate each of the three canvas commands at 1.75 GiB of room and
/// `scratch` bytes of scratch: (admitted, expected to spill, scratch-full).
fn gate_large_canvas_commands(scratch: i64) -> Vec<(bool, bool, bool)> {
    let doc = background_and_layer(17_000);
    let costs = [
        canvas_size_cost(&doc, 25_500, 25_500, pictura_ops::Anchor::Center),
        orient_cost(&doc, 1),
        rotate_cost(&doc, 15.0).0,
    ];
    set_available_memory_for_test(2048 * MIB);
    set_scratch_available_for_test(scratch);
    let gated = costs
        .iter()
        .map(|cost| {
            clear();
            let admitted = admit_spilling(cost.working, cost.spillable);
            (
                admitted,
                expects_spill(cost),
                edit_refused_for_scratch_full(),
            )
        })
        .collect();
    set_available_memory_for_test(-1);
    set_scratch_available_for_test(-1);
    gated
}

// [ocg_gate_spills]
#[test]
fn large_canvas_commands_are_admitted_when_scratch_holds_them() {
    let doc = background_and_layer(17_000);
    let planes = doc.layers[0].channels().len() as u64;
    let canvas = canvas_size_cost(&doc, 25_500, 25_500, pictura_ops::Anchor::Center);
    assert_eq!(
        canvas.spillable,
        planes * 25_500 * 25_500,
        "only the Background is rewritten"
    );
    assert_eq!(orient_cost(&doc, 1).spillable, 2 * planes * 17_000 * 17_000);
    let (rotate, (w, h)) = rotate_cost(&doc, 15.0);
    assert!(w > 17_000 && h > 17_000, "the canvas grows: {w}×{h}");
    assert_eq!(rotate.spillable, 2 * planes * u64::from(w) * u64::from(h));
    for (i, (admitted, spills, _)) in gate_large_canvas_commands(64 * 1024 * MIB)
        .into_iter()
        .enumerate()
    {
        assert!(admitted, "command {i}");
        assert!(spills, "command {i}: the planes exceed the room");
    }
}

// [ocg_gate_scratch_full]
#[test]
fn full_scratch_disks_refuse_large_canvas_commands() {
    for (i, (admitted, _, scratch_full)) in gate_large_canvas_commands(0).into_iter().enumerate() {
        assert!(!admitted, "command {i}");
        assert!(scratch_full, "command {i}");
    }
}

// [ocr_gate_spills]
#[test]
fn a_large_image_size_is_admitted_when_scratch_holds_it() {
    let doc = full_canvas(17_000);
    let cost = image_size_cost(&doc, 25_500, 25_500);
    let planes = doc.layers[0].channels().len() as u64;
    assert_eq!(cost.spillable, planes * 25_500 * 25_500);
    assert!(
        cost.working >= 4 * TILE as u64 * 25_500,
        "one strip is charged"
    );
    let (admitted, spills, _) = gate_large_image_size(64 * 1024 * MIB);
    assert!(admitted);
    assert!(spills, "the planes exceed the room");
}

// [ocr_gate_scratch_full]
#[test]
fn full_scratch_disks_refuse_a_large_image_size() {
    let (admitted, _, scratch_full) = gate_large_image_size(0);
    assert!(!admitted);
    assert!(scratch_full);
}

// [mcr_transform_spill_job] [mcr_transform_fits_sync]
#[test]
fn a_commit_beyond_the_room_expects_to_spill() {
    let cost = CommitCost {
        working: 100 * MIB as u64,
        spillable: 200 * MIB as u64,
    };
    // 256 MiB of headroom is kept under the pinned available memory.
    set_available_memory_for_test((256 + 200) * MIB);
    set_scratch_available_for_test(64 * 1024 * MIB);
    clear();
    let admitted = admit_spilling(cost.working, cost.spillable);
    let spills = expects_spill(&cost);
    set_available_memory_for_test((256 + 300) * MIB);
    let fits = expects_spill(&cost);
    set_available_memory_for_test(-1);
    set_scratch_available_for_test(-1);
    assert!(admitted, "the scratch disks hold the rest");
    assert!(spills, "300 MiB over 200 MiB of room");
    assert!(!fits, "the room holds all of it");
}

// [ocf_small_refresh]
#[test]
fn a_small_document_fill_charges_its_refresh() {
    let doc = full_canvas(2000);
    let bounds = whole(2000);
    let cost = fill_cost(&doc, "0", bounds, 0);
    assert_eq!(cost.working, refresh_bytes(bounds));
    assert_eq!(cost.spillable, fork_bytes(&doc, "0", bounds));
    // A large document's small region is composited densely too.
    let doc = full_canvas(17_000);
    let small = whole(1000);
    assert_eq!(refresh_working(&doc, small), refresh_bytes(small));
}

#[test]
fn fork_bytes_counts_tiles() {
    for (depth, bytes) in [
        (pictura_core::BitDepth::Eight, 1),
        (pictura_core::BitDepth::Sixteen, 2),
        (pictura_core::BitDepth::ThirtyTwo, 4),
    ] {
        let mut doc = Document::new(1000, 1000, pictura_core::ColorMode::Rgb, depth);
        doc.layers =
            pictura_core::LayerStack::from_layers(vec![crate::cxxqt_object::tests::pixel_layer(
                "layer",
                1000,
                1000,
                (0, 0, 0),
            )]);
        let planes = doc.layers[0]
            .channels()
            .iter()
            .filter(|c| c.id >= -1)
            .count() as u64;
        let inside = PsdRect {
            top: 300,
            left: 300,
            bottom: 310,
            right: 310,
        };
        let one_tile = (TILE * TILE) as u64;
        assert_eq!(fork_bytes(&doc, "0", inside), planes * one_tile * bytes);
    }
}

#[test]
fn a_fill_that_grows_the_layer_forks_the_grown_tiles() {
    let mut doc = Document::new(
        100,
        300,
        pictura_core::ColorMode::Rgb,
        pictura_core::BitDepth::Eight,
    );
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![crate::cxxqt_object::tests::pixel_layer(
            "layer",
            20,
            20,
            (0, 0, 0),
        )]);
    // The layer is 20×20 at the origin; a fill box at (80, 80)–(120, 140)
    // grows it to 120×140, whose tiles the fill forks.
    let bounds = PsdRect {
        top: 80,
        left: 80,
        bottom: 140,
        right: 120,
    };
    let planes = doc.layers[0]
        .channels()
        .iter()
        .filter(|c| c.id >= -1)
        .count() as u64;
    // The grown layer is one 120×140 tile.
    assert_eq!(fork_bytes(&doc, "0", bounds), planes * 140 * 120);
}

/// A `side`² RGB/8 document whose one layer covers `rect` with tiled planes
/// that hold `dense`² of varying pixels at the layer's corner and are uniform
/// elsewhere.
fn layer_with_dense_corner(side: u32, rect: PsdRect, dense: usize) -> Document {
    let (w, h) = (rect.width() as usize, rect.height() as usize);
    let block: Vec<u8> = (0..dense * dense).map(|i| (i % 251) as u8).collect();
    let channels = [0i16, 1, 2, -1]
        .into_iter()
        .map(|id| {
            let mut data = pictura_core::Plane::filled(w, h, 1, 0u8);
            data.write_rect_from(0..dense, 0..dense, w, &block);
            data.settle();
            pictura_core::Channel { id, data }
        })
        .collect();
    let layer = pictura_codec::layer_from_parts(pictura_core::LayerParts {
        name: "scribble".into(),
        rect,
        opacity: 255,
        fill: 255,
        visible: true,
        channels,
        ..Default::default()
    });
    let mut doc = Document::new(1, 1, pictura_core::ColorMode::Rgb, BitDepth::Eight);
    doc.width = side;
    doc.height = side;
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    doc
}

/// Gate a brush stroke on layer "0" of `doc` with `available` bytes free:
/// whether it was admitted and whether it was refused for memory.
fn gate_brush(doc: &Document, available: i64) -> (bool, bool) {
    set_available_memory_for_test(available);
    let admitted = admit_resident(brush_bytes(doc, "0")).admitted();
    let refused = edit_refused_for_memory();
    set_available_memory_for_test(-1);
    (admitted, refused)
}

// [bsm_gate_sparse]
/// #1603: a sparse 15,000 × 17,500 scribble layer on a 21,000² canvas with
/// about 1.9 GiB of room. The old charge (logical size + 3 × canvas) was
/// above the room.
#[test]
fn a_sparse_layer_on_a_large_canvas_admits_a_stroke() {
    let side = 21_000u32;
    let rect = PsdRect {
        top: 1_500,
        left: 3_000,
        bottom: 19_000,
        right: 18_000,
    };
    let doc = layer_with_dense_corner(side, rect, 2_048);
    let room = (2_200 - 256) * MIB as u64;
    let logical = 4 * rect.width() as u64 * rect.height() as u64;
    let canvas = u64::from(side) * u64::from(side);
    assert!(logical + 3 * canvas > room, "the old charge fit the room");
    let estimate = brush_bytes(&doc, "0");
    assert!(
        estimate < room,
        "estimate {estimate} B is above the room {room} B"
    );
    assert!(
        estimate >= 4 * 2_048 * 2_048 + canvas,
        "the estimate dropped the layer's held tiles"
    );
    assert_eq!(gate_brush(&doc, 2_200 * MIB), (true, false));
}

// [bsm_gate_dense]
/// A dense layer still pays its full size: with room for the canvas term but
/// not the layer, the stroke is refused, while an empty layer on the same
/// canvas paints.
#[test]
fn a_dense_layer_beyond_the_room_refuses_a_stroke() {
    let side = 2_048u32;
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: side as i32,
        right: side as i32,
    };
    let dense = layer_with_dense_corner(side, rect, side as usize);
    let empty = layer_with_dense_corner(side, rect, 0);
    // 16 MiB of room: above 4 MiB of canvas term, below 16 MiB of planes.
    let available = (256 + 16) * MIB;
    assert_eq!(gate_brush(&empty, available), (true, false));
    assert_eq!(gate_brush(&dense, available), (false, true));
}
