//! The canvas display as a tile pyramid (#1216): tiles against a whole
//! rebuild at test sizes (R5), visibility, repair, history, the worker path,
//! the solo mask, Sample All Layers, and the merged image a save writes.

use super::*;
use crate::cxxqt_object::document_session::DocumentSession;
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::helpers_composite::planes_of;
use crate::cxxqt_object::impl_core::mask_display_plane;
use crate::cxxqt_object::presentation::present_core;
use crate::history::History;
use pictura_core::{BitDepth, Channel, ColorMode, Layer, LayerStack};
use pictura_render::ViewPyramid;

/// Odd on both sides and four levels deep (2101 → 1051 → 526 → 263).
const W: u32 = 2101;
const H: u32 = 1301;

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom,
        right,
    }
}

fn whole(w: u32, h: u32) -> PsdRect {
    rect(0, 0, w as i32, h as i32)
}

/// A pixel layer over `r` whose every channel varies with position.
fn varied(name: &str, r: PsdRect, seed: u32) -> Layer {
    let (w, h) = (r.width() as u32, r.height() as u32);
    let mut layer = super::super::tests::pixel_layer(name, w, h, (0, 0, 0));
    layer.rect = r;
    for (c, channel) in layer.channels.iter_mut().enumerate() {
        let data: Vec<u8> = (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                let v = x * 7 + y * 13 + seed + c as u32 * 61;
                if channel.id == -1 {
                    (60 + v % 196) as u8
                } else {
                    (v % 251) as u8
                }
            })
            .collect();
        *channel = Channel {
            id: channel.id,
            data: data.into(),
        };
    }
    layer
}

/// An opaque base under a half-covering multiply layer: a layered RGB
/// document, which keeps no stored composite.
fn layered(w: u32, h: u32) -> Document {
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    let mut base = varied("base", whole(w, h), 3);
    for channel in &mut base.channels {
        if channel.id == -1 {
            channel.data = vec![255u8; (w * h) as usize].into();
        }
    }
    let mut top = varied(
        "top",
        rect(w as i32 / 5, h as i32 / 7, w as i32 - 9, h as i32 / 2 + 31),
        41,
    );
    top.blend = pictura_core::BlendMode::Multiply;
    doc.layers = LayerStack::from_layers(vec![base, top]);
    doc
}

fn view_of(doc: Document) -> PictureViewRust {
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    };
    view.composite_into_doc();
    view.reset_pyramid();
    view
}

/// The pyramid a whole rebuild of the display composite gives: the R5
/// reference, at test sizes only.
fn reference(view: &PictureViewRust) -> (PixelBuffer, ViewPyramid) {
    let doc = view.session.doc.as_ref().expect("document");
    let source = display_region(doc, whole(doc.width, doc.height), view.painting());
    let pyramid = ViewPyramid::rebuild(planes_of(&source).expect("RGBA"));
    (source, pyramid)
}

fn assert_crop_matches(view: &PictureViewRust, level: usize, r: PsdRect) {
    let (source, pyramid) = reference(view);
    let planes = planes_of(&source).expect("RGBA");
    let (crop, missing) = view.tile_crop(TileSource::Composite, level as i32, r);
    assert!(
        missing.is_empty(),
        "a small document builds its tiles at once"
    );
    assert_eq!(
        crop.data(),
        pyramid.crop(planes, level, r).data(),
        "level {level} rect {r:?}"
    );
}

fn assert_presented_levels_match(view: &PictureViewRust) {
    for level in 0..view.display_level_count() as usize {
        let (w, h) = view.tiles.cache.borrow().level_size(level);
        assert_crop_matches(view, level, whole(w, h));
    }
}

// [tp_crop]
#[test]
fn tile_crops_match_a_whole_rebuild() {
    let view = view_of(layered(W, H));
    assert_eq!(view.display_level_count(), 4);
    let rects = [
        rect(0, 0, 40, 40),
        rect(250, 120, 777, 400),
        rect(-30, -10, 300, 280),
        rect(1900, 1100, 2200, 1400),
    ];
    for level in 0..4usize {
        for r in rects {
            let s = 1i32 << level;
            let scaled = rect(r.left / s, r.top / s, r.right / s, r.bottom / s);
            assert_crop_matches(&view, level, scaled);
        }
    }
    let doc = view.session.doc.as_ref().expect("document");
    assert!(
        !pictura_render::DocumentComposite::is_stored(doc),
        "a layered RGB document keeps no composite"
    );
}

// [tp_visible_only]
#[test]
fn only_visible_tiles_are_built() {
    let mut view = view_of(layered(4096, 2048));
    view.presented_level = 0;
    view.presented_rect = rect(1000, 600, 1800, 1200);
    view.pin_presented_view();
    let (_, missing) = view.tile_crop(TileSource::Composite, 0, view.presented_rect);
    assert!(missing.is_empty());
    let visible = view
        .tiles
        .cache
        .borrow()
        .keys_in(TileSource::Composite, 0, view.presented_rect)
        .len() as u64;
    let stats = view.tile_stats();
    assert_eq!(stats.tiles, visible, "only the tiles under the view");
    assert!(visible <= 16);
}

// [te_display_drop]
#[test]
fn the_display_cache_drops_on_request() {
    let mut view = view_of(layered(4096, 2048));
    view.presented_level = 0;
    view.presented_rect = rect(0, 0, 4096, 2048);
    view.pin_presented_view();
    let _ = view.tile_crop(TileSource::Composite, 0, view.presented_rect);
    let all = view.tile_stats().tiles;
    view.presented_rect = rect(1000, 600, 1200, 800);
    view.pin_presented_view();
    assert_eq!(view.tile_stats().tiles, all, "no request, nothing dropped");
    // An eviction pass asks first; it frees nothing else here.
    pictura_core::memory::eviction::run_pass(0);
    view.pin_presented_view();
    assert!(
        view.tile_stats().tiles < all,
        "the unpinned tiles were dropped on request"
    );
}

// [tp_damage]
#[test]
fn an_edit_repairs_the_visible_tiles() {
    let mut view = view_of(layered(W, H));
    assert_presented_levels_match(&view);
    let far = TileKey {
        source: TileSource::Composite,
        level: 0,
        tx: 7,
        ty: 4,
    };
    let far_before = view.tiles.cache.borrow_mut().get(far).expect("cached");

    let dirty = rect(130, 90, 410, 333);
    {
        let doc = view.session.doc.as_mut().expect("document");
        let layer = &mut doc.layers[0];
        let w = layer.rect.width() as usize;
        let red = layer.channels[0].data.as_mut_slice();
        for y in dirty.top..dirty.bottom {
            for x in dirty.left..dirty.right {
                red[y as usize * w + x as usize] ^= 0x5a;
            }
        }
    }
    assert_eq!(view.refresh_display_rect(dirty), Some(dirty));
    assert_presented_levels_match(&view);
    let far_after = view
        .tiles
        .cache
        .borrow_mut()
        .get(far)
        .expect("still cached");
    assert!(
        Arc::ptr_eq(&far_before, &far_after),
        "a tile the damage does not reach is not rebuilt"
    );
}

fn restore_and_present(
    view: &mut PictureViewRust,
    step: impl FnOnce(
        &mut History,
        &mut Document,
        &mut Option<pictura_select::Selection>,
        &mut crate::layer_target::LayerTarget,
    ) -> Option<crate::history::Restored>,
    legacy: impl FnOnce(&mut History) -> Option<crate::history::Snapshot>,
) {
    let restored = view
        .session
        .restore(&mut view.layer_target, step, legacy)
        .expect("a state to restore");
    let presentation = view.restore_presentation(restored, false);
    present_core(view, &presentation);
}

// [tp_history]
#[test]
fn history_presents_current_tiles() {
    let mut view = view_of(layered(900, 700));
    let target = view.layer_target.clone();
    view.session.record(&target, "Open");
    assert_presented_levels_match(&view);
    let snapshot = view.named_snapshot().expect("a snapshot");

    {
        let doc = view.session.doc.as_mut().expect("document");
        assert!(pictura_render::set_opacity_paths(doc, &["1"], 90) > 0);
    }
    let edit = view.apply_edit("Opacity", Edit::regions(vec![whole(900, 700)], true));
    present_core(&mut view, &edit);
    assert_presented_levels_match(&view);

    restore_and_present(&mut view, History::undo_live, History::undo);
    assert_presented_levels_match(&view);
    restore_and_present(&mut view, History::redo_live, History::redo);
    assert_presented_levels_match(&view);

    // A named-snapshot restore replaces the document as a whole.
    view.session.doc = Some(snapshot.doc);
    view.reset_pyramid();
    assert_presented_levels_match(&view);
}

// [tp_async]
#[test]
fn a_missing_large_tile_does_not_block() {
    let view = view_of(layered(W, H));
    view.tiles.force_async.set(true);
    let shown = rect(300, 200, 700, 500);

    // The coarsest level is built; the level-0 tiles are not.
    let coarsest = view.display_level_count() - 1;
    let (w, h) = view.tiles.cache.borrow().level_size(coarsest as usize);
    view.tiles.force_async.set(false);
    view.tile_crop(TileSource::Composite, coarsest, whole(w, h));
    view.tiles.force_async.set(true);

    let (stand_in, missing) = view.tile_crop(TileSource::Composite, 0, shown);
    assert!(!missing.is_empty(), "the exact tiles are left to a worker");
    assert!(
        stand_in.data().iter().any(|&v| v != 0),
        "the stand-in comes from the coarser tiles"
    );

    // The worker's build, as `start_tile_job` hands it over.
    let job = {
        let cache = view.tiles.cache.borrow();
        TileJob {
            doc: view.session.doc.clone().expect("document"),
            mask: None,
            painting: false,
            ticket: cache.ticket(),
            keys: missing
                .iter()
                .map(|&key| (key, cache.tile_rect(key)))
                .collect(),
        }
    };
    for (key, data) in job.build() {
        assert!(view.tiles.cache.borrow_mut().install(key, job.ticket, data));
    }
    view.tiles.force_async.set(false);
    assert_crop_matches(&view, 0, shown);
}

// [tp_sync_small]
#[test]
fn small_documents_build_tiles_synchronously() {
    let view = view_of(layered(640, 480));
    for level in 0..view.display_level_count() {
        let (w, h) = view.tiles.cache.borrow().level_size(level as usize);
        let (_, missing) = view.tile_crop(TileSource::Composite, level, whole(w, h));
        assert!(missing.is_empty(), "level {level} is exact at once");
    }
}

/// A layered document whose top layer carries a raster mask with a gradient,
/// soloed.
fn soloed() -> PictureViewRust {
    let (w, h) = (1100u32, 900u32);
    let mut doc = layered(w, h);
    let r = doc.layers[1].rect;
    let (mw, mh) = (r.width() as usize, r.height() as usize);
    doc.layers[1].mask = Some(pictura_core::LayerMask {
        rect: r,
        default_color: 30,
        data: Some(
            (0..mw * mh)
                .map(|i| ((i % mw) * 255 / mw + (i / mw) % 7) as u8)
                .collect::<Vec<u8>>()
                .into(),
        ),
        ..Default::default()
    });
    let mut view = view_of(doc);
    view.mask_solo = Some("1".to_string());
    view.rebuild_mask_solo_source();
    view
}

fn assert_mask_matches(view: &PictureViewRust) {
    let doc = view.session.doc.as_ref().expect("document");
    let mask = view.solo_mask().expect("a soloed mask");
    let plane = mask_display_plane(mask, doc.width, doc.height);
    let planes = planes_of(&plane).expect("RGBA");
    let pyramid = ViewPyramid::rebuild(planes);
    for level in 0..pyramid.level_count() {
        let (w, h) = pyramid.level_size(level);
        let (crop, missing) = view.tile_crop(TileSource::Mask, level as i32, whole(w, h));
        assert!(missing.is_empty());
        assert_eq!(
            crop.data(),
            pyramid.crop(planes, level, whole(w, h)).data(),
            "mask level {level}"
        );
    }
}

// [tp_solo_restore]
#[test]
fn solo_mask_restore_presents_visible_tiles() {
    let mut view = soloed();
    assert_eq!(view.canvas_source(), TileSource::Mask);
    assert_mask_matches(&view);

    // A whole-document restore while soloed: the mask tiles go stale, not
    // rebuilt, and the visible ones come back exact.
    let target = view.layer_target.clone();
    view.session.record(&target, "Open");
    let snapshot = view.named_snapshot().expect("a snapshot");
    view.session.doc = Some(snapshot.doc);
    view.reset_pyramid();
    view.refresh_mask_solo();
    assert!(
        view.tile_stats().resident_bytes > 0,
        "the stale tiles stay as stand-ins"
    );
    assert_mask_matches(&view);

    // Leaving solo presents the restored document view.
    view.mask_solo = None;
    view.drop_mask_solo_source();
    assert_eq!(view.canvas_source(), TileSource::Composite);
    assert_presented_levels_match(&view);
}

// [rc_sample_all]
#[test]
fn sample_all_reads_regions() {
    let view = view_of(layered(700, 520));
    let doc = view.session.doc.as_ref().expect("document");
    let full = pictura_render::composite_rgba(doc);
    let size = (doc.width, doc.height);
    let fetch = crate::cxxqt_object::picture::sample_tiles(doc, &view);
    let tiles = pictura_paint::bucket::BucketSource::Tiles {
        fetch: &fetch,
        width: doc.width,
        height: doc.height,
    };
    for (seed, tolerance, contiguous) in [((40, 30), 25u8, true), ((500, 300), 60, false)] {
        let wand =
            pictura_select::magic_wand(&full, seed.0 as u32, seed.1 as u32, tolerance, contiguous)
                .expect("a wand selection");
        let flooded = pictura_paint::bucket::magic_flood(tiles, size, seed, tolerance, contiguous)
            .expect("a flood");
        assert!(
            flooded == wand.data,
            "the wand over tiles equals the wand over the composite"
        );
        let bucket_tiles = pictura_paint::bucket::flood(
            tiles,
            size,
            whole(size.0, size.1),
            None,
            seed,
            tolerance,
            contiguous,
        );
        let bucket_whole = pictura_paint::bucket::flood(
            pictura_paint::bucket::BucketSource::Composite(&full),
            size,
            whole(size.0, size.1),
            None,
            seed,
            tolerance,
            contiguous,
        );
        assert!(
            bucket_tiles == bucket_whole,
            "the bucket over tiles equals the bucket over the composite"
        );
    }
    for (x, y) in [(3, 4), (699, 519), (350, 100)] {
        let one = rect(x, y, x + 1, y + 1);
        let sampled = crate::cxxqt_object::picture::picture_region(doc, &view, one);
        let plane = (full.width * full.height) as usize;
        let i = (y as u32 * full.width + x as u32) as usize;
        let expected: Vec<u8> = (0..4).map(|c| full.data[c * plane + i]).collect();
        assert_eq!(
            sampled.data.as_ref(),
            &expected[..],
            "the eyedropper at {x},{y}"
        );
    }
}

// [rc_save]
#[test]
fn save_streams_the_merged_image() {
    let view = view_of(layered(640, 360));
    let doc = view.session.doc.as_ref().expect("document");
    assert!(!pictura_render::DocumentComposite::is_stored(doc));
    let streamed = pictura_codec::write_psd(&pictura_render::save_view(doc)).expect("writes");
    let mut stored = doc.clone();
    stored.composite = pictura_render::composite_rgba(doc);
    let expected = pictura_codec::write_psd(&stored).expect("writes");
    assert!(streamed == expected, "the merged image is byte-identical");
}

// [rc_save_peak]
#[test]
fn save_allocates_no_whole_composite() {
    let (w, h) = (2048u32, 2048u32);
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    let mut base = super::super::tests::pixel_layer("base", w, h, (255, 255, 255));
    for channel in &mut base.channels {
        let value = channel.data.sample(0).expect("a sample");
        channel.data = pictura_core::Plane::filled(w as usize, h as usize, 1, value);
    }
    doc.layers = LayerStack::from_layers(vec![base, varied("dot", rect(100, 100, 300, 260), 9)]);
    let view = view_of(doc);
    let doc = view.session.doc.as_ref().expect("document");
    let saved = pictura_render::save_view(doc);
    let merged = saved.composite.data.allocated_bytes() as u64;
    let canvas = u64::from(w) * u64::from(h) * 4;
    assert!(
        merged * 8 < canvas,
        "the merged image holds its dense tiles only: {merged} of {canvas} bytes"
    );
}

/// Ending solo drops the mask tiles and the mask keys still queued for a
/// worker, so no mask tile is built without a mask to build it from.
#[test]
fn ending_solo_drops_queued_mask_tiles() {
    let mut view = soloed();
    let key = TileKey {
        source: TileSource::Mask,
        level: 0,
        tx: 1,
        ty: 1,
    };
    let composite = TileKey {
        source: TileSource::Composite,
        ..key
    };
    view.tiles.wanted.get_mut().extend([key, composite]);
    view.mask_solo = None;
    view.drop_mask_solo_source();
    let wanted = view.tiles.wanted.get_mut();
    assert!(!wanted.contains(&key), "the mask key is dropped");
    assert!(wanted.contains(&composite), "a composite key stays queued");
}
