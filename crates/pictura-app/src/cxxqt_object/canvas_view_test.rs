use super::helpers_composite::*;
use super::tiles::display_region;
use super::PictureViewRust;
use pictura_core::{BitDepth, ColorMode, Document, PixelBuffer, PsdRect};
use pictura_render::{patch_buffer_region, CanvasDamage, TileKey, TileSource, ViewPyramid};

/// A 4-plane RGBA buffer with variation in every channel, so a level mismatch
/// cannot hide behind uniform pixels.
fn varied_rgba(width: u32, height: u32, seed: u32) -> PixelBuffer {
    let n = (width * height) as usize;
    let mut data = vec![0u8; n * 4];
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize;
            let v = ((x * 7 + y * 13 + seed) % 251) as u8;
            data[i] = v;
            data[n + i] = v.wrapping_mul(3);
            data[2 * n + i] = 255 - v;
            data[3 * n + i] = (60 + (x * 3 + y) % 196) as u8;
        }
    }
    PixelBuffer {
        width,
        height,
        channels: 4,
        data: data.into(),
    }
}

/// Copy `rect` of `src` into `dst`, plane by plane.
fn blit_region(dst: &mut PixelBuffer, src: &PixelBuffer, rect: PsdRect) {
    let (dplane, splane) = (dst.pixel_count(), src.pixel_count());
    for y in rect.top..rect.bottom {
        for x in rect.left..rect.right {
            let di = y as usize * dst.width as usize + x as usize;
            let si = y as usize * src.width as usize + x as usize;
            for c in 0..4 {
                dst.data[c * dplane + di] = src.data[c * splane + si];
            }
        }
    }
}

/// Copy `rect` of `src` into a new 4-plane buffer.
fn crop_buffer(src: &PixelBuffer, rect: PsdRect) -> PixelBuffer {
    let (w, h) = (rect.width() as usize, rect.height() as usize);
    let splane = src.pixel_count();
    let dplane = w * h;
    let mut data = vec![0u8; dplane * 4];
    for y in 0..h {
        for x in 0..w {
            let si = (rect.top as usize + y) * src.width as usize + rect.left as usize + x;
            let di = y * w + x;
            for c in 0..4 {
                data[c * dplane + di] = src.data[c * splane + si];
            }
        }
    }
    PixelBuffer {
        width: w as u32,
        height: h as u32,
        channels: 4,
        data: data.into(),
    }
}

fn whole_level(pyramid: &ViewPyramid, level: usize) -> PsdRect {
    let (w, h) = pyramid.level_size(level);
    PsdRect {
        top: 0,
        left: 0,
        bottom: h as i32,
        right: w as i32,
    }
}

fn whole(w: u32, h: u32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: h as i32,
        right: w as i32,
    }
}

fn view_of(doc: Document) -> PictureViewRust {
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    };
    // As an open or a New Document does: a layered RGB document drops the
    // composite it was constructed with.
    rust.composite_into_doc();
    rust.reset_pyramid();
    rust
}

/// The whole of `level` as the canvas shows it, built (and so cached) from
/// tiles; a small document builds every missing tile at once.
fn shown(rust: &PictureViewRust, level: i32) -> Vec<u8> {
    let (w, h) = rust.display_level_size(level).expect("the level exists");
    let (crop, missing) = rust.tile_crop(TileSource::Composite, level, whole(w, h));
    assert!(
        missing.is_empty(),
        "a small document builds its tiles at once"
    );
    crop.data().to_vec()
}

/// The whole display composite of the view's document, the R5 reference:
/// whole buffers only at test sizes.
fn reference(rust: &PictureViewRust) -> PixelBuffer {
    let doc = rust.session.doc.as_ref().expect("document");
    display_region(
        doc,
        whole(doc.width, doc.height),
        rust.session.live.stroke().is_some(),
    )
}

/// Every level the view presents equals the same level of a pyramid rebuilt
/// from the whole display composite.
fn assert_tiles_match_a_rebuild(rust: &PictureViewRust) {
    let source = reference(rust);
    let planes = planes_of(&source).expect("RGBA source");
    let pyramid = ViewPyramid::rebuild(planes);
    assert_eq!(
        rust.display_level_count() as usize,
        pyramid.level_count(),
        "level count"
    );
    for level in 0..pyramid.level_count() {
        assert_eq!(
            shown(rust, level as i32),
            pyramid
                .crop(planes, level, whole_level(&pyramid, level))
                .data(),
            "level {level}"
        );
    }
}

fn key(level: u8, tx: u32, ty: u32) -> TileKey {
    TileKey {
        source: TileSource::Composite,
        level,
        tx,
        ty,
    }
}

#[test]
fn display_conversion_premultiplies_and_straight_helpers_do_not() {
    let buffer = PixelBuffer {
        width: 2,
        height: 1,
        channels: 4,
        data: vec![200, 255, 100, 255, 50, 255, 128, 0].into(),
    };
    assert_eq!(
        buffer_to_rgba_bytes(&buffer),
        vec![200, 100, 50, 128, 255, 255, 255, 0],
        "straight helper is unchanged"
    );
    assert_eq!(
        display_rgba_bytes(&buffer),
        vec![100, 50, 25, 128, 0, 0, 0, 0],
        "display pixel is round(c*a/255) and alpha is kept"
    );
    let image = premultiplied_display_image(&buffer);
    assert_eq!(
        image.format(),
        cxx_qt_lib::QImageFormat::Format_RGBA8888_Premultiplied
    );
    assert_eq!(image.pixel_color(0, 0).alpha(), 128);
    assert_eq!(image.pixel_color(1, 0).alpha(), 0);
}

#[test]
fn a_region_repair_matches_a_rebuild() {
    let mut doc = Document::new(600, 400, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(600, 400, 0);
    let mut rust = view_of(doc.clone());
    assert_tiles_match_a_rebuild(&rust);

    // A described region change, applied to the stored composite and folded
    // into the cached tiles the way `refresh_region` does.
    let rect = PsdRect {
        top: 100,
        left: 120,
        bottom: 180,
        right: 260,
    };
    let changed = varied_rgba(600, 400, 77);
    blit_region(&mut doc.composite, &changed, rect);
    rust.session.doc = Some(doc);
    rust.apply_refreshed_region(rect.left, rect.top, rect, crop_buffer(&changed, rect));
    assert_tiles_match_a_rebuild(&rust);
}

#[test]
fn the_display_carries_the_document_profile_conversion() {
    let mut doc = Document::new(16, 16, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(16, 16, 4);
    pictura_codec::assign_document_profile(&mut doc, &pictura_codec::Profile::adobe_rgb());

    let converted = display_region(&doc, whole(16, 16), false);
    let raw = pictura_render::rgba_frame(&doc.composite);
    assert_ne!(
        converted.data, raw.data,
        "a non-identity profile must convert the frame"
    );
    let rust = view_of(doc);
    assert_eq!(
        shown(&rust, 0),
        pictura_render::premultiply_planar(&converted),
        "the tiles are color managed"
    );
}

#[test]
fn a_region_repair_writes_the_cached_tile_in_place() {
    let mut doc = Document::new(400, 300, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(400, 300, 0);
    let mut rust = view_of(doc.clone());
    shown(&rust, 0);
    let before = {
        let tile = rust
            .tiles
            .cache
            .borrow_mut()
            .get(key(0, 0, 0))
            .expect("cached");
        tile.as_ptr()
    };

    let rect = PsdRect {
        top: 20,
        left: 30,
        bottom: 80,
        right: 120,
    };
    let changed = varied_rgba(400, 300, 9);
    blit_region(&mut doc.composite, &changed, rect);
    rust.session.doc = Some(doc);
    rust.apply_refreshed_region(rect.left, rect.top, rect, crop_buffer(&changed, rect));

    let after = rust
        .tiles
        .cache
        .borrow_mut()
        .get(key(0, 0, 0))
        .expect("cached");
    assert_eq!(after.as_ptr(), before, "the tile is patched in place");
    drop(after);
    assert_tiles_match_a_rebuild(&rust);
}

#[test]
fn the_display_after_a_region_refresh_matches_a_full_recomposite() {
    let mut doc = Document::new(64, 48, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(64, 48, 1);
    let mut rust = view_of(doc.clone());
    shown(&rust, 0);
    let revision = rust.canvas_revision;

    let rect = PsdRect {
        top: 8,
        left: 8,
        bottom: 24,
        right: 24,
    };
    let changed = varied_rgba(64, 48, 21);
    blit_region(&mut doc.composite, &changed, rect);
    rust.session.doc = Some(doc);
    rust.apply_refreshed_region(rect.left, rect.top, rect, crop_buffer(&changed, rect));

    assert!(
        rust.canvas_revision > revision,
        "refresh bumps the revision"
    );
    assert_eq!(
        rust.display_rect_damage(),
        Some(rect),
        "damage survives for the canvas"
    );
    assert_eq!(
        shown(&rust, 0),
        pictura_render::premultiply_planar(&reference(&rust)),
        "the display equals a full recomposite"
    );
}

#[test]
fn histogram_source_level_is_bounded_and_size_independent() {
    // The Histogram panel reads the coarsest pyramid level. Its long side is
    // always at most 512, whatever the document size, so the binning cost does
    // not grow with the document.
    for (w, h) in [(513u32, 513u32), (4000, 3000), (8000, 100)] {
        let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
        doc.composite = varied_rgba(w, h, 3);
        let rust = view_of(doc);

        let levels = rust.display_level_count();
        assert!(levels >= 1, "{w}x{h} has at least one level");
        let (lw, lh) = rust
            .display_level_size(levels - 1)
            .expect("coarsest level size");
        assert!(
            lw <= 512 && lh <= 512,
            "coarsest level {lw}x{lh} for a {w}x{h} document"
        );

        let (crop, missing) = rust.display_crop_keys(levels - 1, whole(lw, lh));
        assert!(missing.is_empty());
        assert!(!crop.is_null(), "the level crop is servable");
        assert_eq!((crop.width(), crop.height()), (lw as i32, lh as i32));
    }
}

#[test]
fn mid_stroke_tiles_match_a_full_recomposite_of_the_working_document() {
    use pictura_paint::{Stroke, StrokeConfig, StrokeSample};

    let rgba = vec![40u8; 128 * 128 * 4];
    let mut doc = Document::from_rgba("paint", 128, 128, &rgba);
    let mut stroke = Stroke::begin_at(&doc, "0", StrokeConfig::default()).expect("begin stroke");
    assert!(stroke.sample(
        &mut doc,
        StrokeSample {
            x: 64.0,
            y: 64.0,
            pressure: 1.0,
        }
    ));
    let rect = stroke.take_dirty().expect("a dab dirties a region");

    let session = crate::cxxqt_object::document_session::StrokeSession {
        stroke: Some(stroke),
        ..Default::default()
    };
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    };
    rust.session.live.set_stroke_session(session);
    // A full rebuild while a stroke is live must show the painted document's
    // composited layers.
    rust.reset_pyramid();
    assert_tiles_match_a_rebuild(&rust);

    // A mid-stroke region refresh keeps every level equal to a full rebuild.
    assert!(rust.refresh_display_rect(rect).is_some());
    assert_tiles_match_a_rebuild(&rust);
}

#[test]
fn a_mid_stroke_named_snapshot_shares_the_painted_tiles() {
    use pictura_paint::{Stroke, StrokeConfig, StrokeSample};

    let rgba = vec![40u8; 128 * 128 * 4];
    let mut doc = Document::from_rgba("paint", 128, 128, &rgba);
    let before = pictura_render::composite_rgba(&doc);
    let mut stroke = Stroke::begin_at(&doc, "0", StrokeConfig::default()).expect("begin stroke");
    assert!(stroke.sample(
        &mut doc,
        StrokeSample {
            x: 64.0,
            y: 64.0,
            pressure: 1.0,
        }
    ));
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    };
    rust.session
        .live
        .set_stroke_session(crate::cxxqt_object::document_session::StrokeSession {
            stroke: Some(stroke),
            ..Default::default()
        });

    let snapshot = rust.named_snapshot().expect("a snapshot");
    let painted = pictura_render::composite_rgba(&snapshot.doc);
    assert!(painted != before, "the dab is in the snapshot");
    let live = rust.session.doc.as_ref().expect("document");
    assert!(
        snapshot.doc.composite.data.shares(&live.composite.data),
        "a layered RGB snapshot composites nothing (#1216)"
    );
    assert!(
        snapshot.doc.layers[0].channels[0]
            .data
            .shares(&live.layers[0].channels[0].data),
        "a snapshot shares its tiles with the document instead of copying them (#1215)"
    );
}

#[test]
fn a_level_zero_damage_crop_matches_the_cached_crop_rect() {
    let mut doc = Document::new(200, 160, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(200, 160, 9);
    let rust = view_of(doc);
    let cached = PsdRect {
        top: 18,
        left: 28,
        bottom: 122,
        right: 152,
    };
    let damage = PsdRect {
        top: 30,
        left: 40,
        bottom: 70,
        right: 110,
    };

    let cache_crop = rust.display_crop_keys(0, cached).0;
    let damage_crop = rust.display_crop_keys(0, damage).0;
    let fresh = rust.display_crop_keys(0, cached).0;
    assert!(!cache_crop.is_null() && !damage_crop.is_null());
    let ox = damage.left - cached.left;
    let oy = damage.top - cached.top;
    for y in 0..damage_crop.height() {
        for x in 0..damage_crop.width() {
            let a = damage_crop.pixel_color(x, y);
            let b = cache_crop.pixel_color(ox + x, oy + y);
            let c = fresh.pixel_color(ox + x, oy + y);
            let px = |c: &cxx_qt_lib::QColor| (c.red(), c.green(), c.blue(), c.alpha());
            assert_eq!(px(&a), px(&b), "the cached crop differs at {x},{y}");
            assert_eq!(px(&a), px(&c), "a fresh crop differs at {x},{y}");
        }
    }
}

#[test]
fn a_picture_region_after_a_refresh_reads_the_current_composite() {
    let mut layer = vec![0u8; 64 * 32 * 4];
    for px in layer.as_chunks_mut::<4>().0 {
        px.copy_from_slice(&[255, 0, 0, 255]);
    }
    let doc = Document::from_rgba("paint", 64, 32, &layer);
    let mut rust = view_of(doc);
    let rect = PsdRect {
        top: 8,
        left: 10,
        bottom: 24,
        right: 40,
    };
    let paint = rust.session.doc.as_mut().expect("document");
    paint.layers[0].channels[1].data.as_mut_slice().fill(200);
    assert!(rust.refresh_display_rect(rect).is_some());

    let doc = rust.session.doc.as_ref().expect("the document survives");
    let fresh = pictura_render::composite_rgba(doc);
    for &(x, y) in &[(20, 12), (39, 23), (1, 1)] {
        let one = PsdRect {
            top: y,
            left: x,
            bottom: y + 1,
            right: x + 1,
        };
        let sampled = super::picture::picture_region(doc, &rust, one);
        assert_eq!(
            sample_planar_argb(&sampled, 0, 0),
            sample_planar_argb(&fresh, x, y),
            "the picture at {x},{y} is the current composite"
        );
    }
}

#[test]
fn a_multi_rect_stroke_commit_equals_a_full_recomposite() {
    use super::helpers::TileSet;
    use pictura_paint::{Stroke, StrokeConfig, StrokeSample};

    let rgba = vec![30u8; 512 * 512 * 4];
    let mut doc = Document::from_rgba("paint", 512, 512, &rgba);
    let mut committed = pictura_render::composite_rgba(&doc);
    let cfg = StrokeConfig {
        diameter: 6,
        ..StrokeConfig::default()
    };
    let mut stroke = Stroke::begin_at(&doc, "0", cfg).expect("begin stroke");
    let mut tiles = TileSet::default();
    tiles.reset(512, 512);
    for i in 0..32 {
        assert!(stroke.sample(
            &mut doc,
            StrokeSample {
                x: 20.0 + i as f32 * 15.0,
                y: 20.0 + i as f32 * 15.0,
                pressure: 1.0,
            }
        ));
        if let Some(rect) = stroke.take_dirty() {
            tiles.mark(rect);
        }
    }
    let dirty = stroke.finish().expect("the stroke painted pixels");
    let regions = tiles.regions(dirty);
    assert!(regions.len() > 1, "a diagonal decomposes into tiles");

    // Replay `end_paint`'s per-rect commit over the pre-stroke picture.
    for region in &regions {
        let buffer = pictura_render::composite(pictura_render::CompositeRequest::region(
            &doc, *region, false,
        ))
        .0;
        patch_buffer_region(
            &mut committed,
            &buffer,
            region.left.max(0),
            region.top.max(0),
        );
    }
    let full = current_buffer(&doc, false);
    assert_eq!(
        committed.data, full.data,
        "the per-rect commit equals a full recomposite"
    );
}

#[test]
fn layer_opacity_region_refresh_equals_a_full_recomposite() {
    let base = vec![80u8; 64 * 64 * 4];
    let mut doc = Document::from_rgba("base", 64, 64, &base);
    let top: Vec<u8> = (0..32 * 32 * 4).map(|i| (i % 200) as u8).collect();
    let path = pictura_render::add_raster_layer_from_rgba(&mut doc, "top", 32, 32, &top);
    assert!(!path.is_empty(), "the top layer is appended");
    let mut picture = current_buffer(&doc, false);

    // Change the top layer's opacity, then refresh only its bounded rect the
    // way `mutate_layer` does.
    doc.layers.last_mut().unwrap().opacity = 128;
    let rect = doc.layers.last().unwrap().rect;
    let buffer =
        pictura_render::composite(pictura_render::CompositeRequest::region(&doc, rect, false)).0;
    patch_buffer_region(&mut picture, &buffer, rect.left, rect.top);

    let full = current_buffer(&doc, false);
    assert_eq!(
        picture.data, full.data,
        "an opacity region refresh equals a full recomposite"
    );
}

/// A rectangular selection plane on a `w × h` document.
fn rect_selection(w: u32, h: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> pictura_select::Selection {
    let mut selection = pictura_select::Selection::none(w, h);
    for y in y0..y1 {
        let row = y as usize * w as usize;
        selection.data[row + x0 as usize..row + x1 as usize].fill(255);
    }
    selection
}

/// Commit a content move through the regional path, returning the document,
/// the picture patched by the region, the picture before the move, and the
/// refreshed dirty rectangle.
fn content_move_region_fixture() -> (Document, PixelBuffer, PixelBuffer, PsdRect) {
    let base: Vec<u8> = (0..64 * 64 * 4)
        .map(|i| if i % 4 == 3 { 255 } else { (i % 251) as u8 })
        .collect();
    let mut doc = Document::from_rgba("base", 64, 64, &base);
    let top: Vec<u8> = (0..64 * 64 * 4)
        .map(|i| if i % 4 == 3 { 255 } else { (i * 7 % 251) as u8 })
        .collect();
    let path = pictura_render::add_raster_layer_from_rgba(&mut doc, "top", 64, 64, &top);
    assert!(!path.is_empty());
    let before = current_buffer(&doc, false);
    let mut picture = before.clone();

    let selection = rect_selection(64, 64, 12, 10, 40, 36);
    let mask = selection_to_mask(&selection, &doc);
    let (dx, dy) = (9, 6);
    assert!(pictura_render::move_selection_content(
        &mut doc, &path, &mask, dx, dy, false
    ));
    let dirty = super::impl_selection::move_dirty_rect(&selection, dx, dy).expect("dirty rect");
    let buffer =
        pictura_render::composite(pictura_render::CompositeRequest::region(&doc, dirty, false)).0;
    patch_buffer_region(&mut picture, &buffer, dirty.left, dirty.top);
    (doc, picture, before, dirty)
}

#[test]
fn content_move_region_equals_a_full_recomposite() {
    let (doc, picture, _, dirty) = content_move_region_fixture();
    let full = current_buffer(&doc, false);
    assert_eq!(
        picture.data, full.data,
        "a content move region refresh equals a full recomposite"
    );
    assert!(dirty.right > dirty.left && dirty.bottom > dirty.top);
}

#[test]
fn content_move_region_keeps_outside_pixels() {
    let (doc, picture, before, dirty) = content_move_region_fixture();
    let plane = picture.pixel_count();
    for y in 0..doc.height {
        for x in 0..doc.width {
            if (x as i32) >= dirty.left
                && (x as i32) < dirty.right
                && (y as i32) >= dirty.top
                && (y as i32) < dirty.bottom
            {
                continue;
            }
            let i = y as usize * doc.width as usize + x as usize;
            for c in 0..4 {
                assert_eq!(
                    picture.data[c * plane + i],
                    before.data[c * plane + i],
                    "pixel ({x}, {y}) channel {c} outside the move changed"
                );
            }
        }
    }
}

#[test]
fn layer_move_region_refresh_equals_a_full_recomposite() {
    let base = vec![90u8; 64 * 64 * 4];
    let mut doc = Document::from_rgba("base", 64, 64, &base);
    let top: Vec<u8> = (0..16 * 16 * 4).map(|i| (i % 180) as u8).collect();
    let path = pictura_render::add_raster_layer_from_rgba(&mut doc, "top", 16, 16, &top);
    assert!(!path.is_empty());
    let index = doc.layers.len() - 1;

    let mut picture = current_buffer(&doc, false);
    let before = doc.layers[index].rect;
    assert!(pictura_render::translate_layer_index(
        &mut doc, index, 10, 6
    ));
    let after = doc.layers[index].rect;
    let dirty = before.union(after);
    let buffer =
        pictura_render::composite(pictura_render::CompositeRequest::region(&doc, dirty, false)).0;
    patch_buffer_region(&mut picture, &buffer, dirty.left, dirty.top);

    let full = current_buffer(&doc, false);
    assert_eq!(
        picture.data, full.data,
        "a move region refresh equals a full recomposite"
    );
}

#[test]
fn a_level0_crop_outside_the_level_is_null_and_a_partial_one_is_not() {
    let mut doc = Document::new(100, 100, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(100, 100, 2);
    let rust = view_of(doc);

    let outside = PsdRect {
        top: 2000,
        left: 2000,
        bottom: 2010,
        right: 2010,
    };
    assert!(rust.display_crop_keys(0, outside).0.is_null());
    let partial = PsdRect {
        top: 90,
        left: 90,
        bottom: 110,
        right: 110,
    };
    let crop = rust.display_crop_keys(0, partial).0;
    assert!(!crop.is_null());
    assert_eq!((crop.width(), crop.height()), (20, 20));
}

#[test]
fn reset_pyramid_rebuilds_and_clears_damage() {
    // `recomposite`, `undo`, `redo`, `history_jump`, `history_restore_snapshot`,
    // and the move-preview path all route through `reset_pyramid`.
    let mut doc = Document::new(600, 400, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(600, 400, 0);
    let mut rust = view_of(doc);
    assert_eq!(rust.damage, CanvasDamage::default());
    assert_tiles_match_a_rebuild(&rust);

    rust.damage.mark(PsdRect {
        top: 1,
        left: 1,
        bottom: 5,
        right: 5,
    });
    let mut restored = Document::new(600, 400, ColorMode::Rgb, BitDepth::Eight);
    restored.composite = varied_rgba(600, 400, 31);
    rust.session.doc = Some(restored);
    rust.reset_pyramid();
    assert_eq!(rust.damage, CanvasDamage::default(), "reset clears damage");
    assert_tiles_match_a_rebuild(&rust);
}

#[test]
fn display_accessors_report_levels_and_damage() {
    let mut doc = Document::new(1500, 700, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(1500, 700, 5);
    let mut rust = view_of(doc);

    assert_eq!(rust.display_level_count(), 3);
    assert_eq!(rust.display_level_size(0), Some((1500, 700)));
    assert_eq!(rust.display_level_size(1), Some((750, 350)));
    assert_eq!(rust.display_level_size(2), Some((375, 175)));
    assert_eq!(rust.display_level_size(3), None);
    assert_eq!(rust.display_level_size(-1), None);

    // A level crop carries that level's size; out-of-range and empty rects are
    // null, not a panic.
    let crop = rust
        .display_crop_keys(
            1,
            PsdRect {
                top: 10,
                left: 10,
                bottom: 26,
                right: 42,
            },
        )
        .0;
    assert_eq!((crop.width(), crop.height()), (32, 16));
    assert!(!crop.is_null());
    assert!(rust.display_crop_keys(3, whole(1500, 700)).0.is_null());
    assert!(rust.display_crop_keys(-1, whole(1500, 700)).0.is_null());
    assert!(rust.display_crop_keys(0, whole(0, 0)).0.is_null());

    assert_eq!(rust.display_rect_damage(), None);
    let dirty = PsdRect {
        top: 4,
        left: 5,
        bottom: 12,
        right: 20,
    };
    rust.damage.mark(dirty);
    assert_eq!(rust.display_rect_damage(), Some(dirty));
    assert_eq!(rust.display_rect_damage(), None, "take starts afresh");
}

/// The frame-bounded present: the dab that opens a frame is presented at once,
/// the frame's later dabs accumulate into one region, the flush's take clears
/// it, and the next dab opens a new frame. Stroke start, commit and cancel all
/// supersede whatever was pending.
#[test]
fn present_accumulates_until_flush_and_the_stroke_lifecycle_supersedes_it() {
    let rect = |left: i32, right: i32| PsdRect {
        top: 8,
        left,
        bottom: 16,
        right,
    };
    let mut rust = PictureViewRust::default();
    // The present frame belongs to the live paint session, so the test opens
    // one the way a stroke begin does.
    rust.session
        .live
        .set_stroke_session(super::document_session::StrokeSession::default());

    let first = rect(0, 8);
    let second = rect(12, 20);
    let third = rect(24, 32);
    assert_eq!(rust.queue_present(first), Some(first), "opens the frame");
    assert_eq!(rust.queue_present(second), None, "waits for the flush");
    assert_eq!(rust.queue_present(third), None, "still waits");
    assert_eq!(
        rust.take_pending_present(),
        Some(rect(12, 32)),
        "one region covering the dabs since the last present"
    );
    assert_eq!(rust.take_pending_present(), None, "cleared by the take");
    assert_eq!(
        rust.queue_present(second),
        Some(second),
        "a new frame opens"
    );

    rust.clear_pending_present();
    assert_eq!(
        rust.take_pending_present(),
        None,
        "the stroke lifecycle superseded the pending region"
    );
}

#[test]
fn the_preview_snapshots_the_presented_level_and_stays_small() {
    use super::document_session::PreviewStroke;
    use super::impl_paint::preview_snapshot_bytes;

    // The two document sizes the threshold was tuned against, and the levels
    // `preview_level` picks for them.
    assert_eq!(preview_snapshot_bytes(4000, 4000, 3), 500 * 500 * 4);
    assert_eq!(preview_snapshot_bytes(16_000, 16_000, 4), 1000 * 1000 * 4);
    assert!(
        preview_snapshot_bytes(4000, 4000, 3) <= 4 * 1024 * 1024,
        "a 4000 square preview must stay under 4 MB"
    );
    assert!(
        preview_snapshot_bytes(16_000, 16_000, 4) <= 8 * 1024 * 1024,
        "a 16000 square preview must stay under 8 MB"
    );

    let mut doc = Document::new(900, 600, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(900, 600, 3);
    let rust = view_of(doc);
    let snapshot = rust.level_snapshot(1).expect("level 1 exists");
    assert_eq!(snapshot.data(), &shown(&rust, 1)[..], "the level's bytes");

    let preview = PreviewStroke::new(
        1,
        snapshot.clone(),
        (900, 600),
        pictura_paint::StrokeConfig::default(),
    )
    .expect("the level exists");
    assert_eq!(preview.snapshot, snapshot.data());
    assert_eq!(preview.level, 1);
    assert_eq!(preview.scale, 2);
    assert_eq!(preview_snapshot_bytes(900, 600, 1), preview.snapshot.len());
    assert_eq!(
        preview.coverage.len(),
        (preview.width * preview.height) as usize,
        "the coverage buffer covers the level exactly once"
    );
    assert!(preview.extent.is_none(), "nothing presented yet");
    assert!(preview.samples.is_empty(), "nothing recorded yet");

    // A level past the pyramid, or a snapshot of the wrong size, is refused
    // rather than snapshotted as zeros.
    assert!(rust.level_snapshot(99).is_none());
    assert!(PreviewStroke::new(
        2,
        snapshot,
        (900, 600),
        pictura_paint::StrokeConfig::default()
    )
    .is_none());
}

#[test]
fn reset_pyramid_drops_a_live_preview() {
    use super::document_session::{PreviewStroke, StrokeSession};

    let mut doc = Document::new(600, 600, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(600, 600, 1);
    let mut rust = view_of(doc);
    let preview = rust.level_snapshot(1).and_then(|snapshot| {
        PreviewStroke::new(
            1,
            snapshot,
            (600, 600),
            pictura_paint::StrokeConfig::default(),
        )
    });
    let session = StrokeSession {
        preview,
        ..Default::default()
    };
    rust.session.live.set_stroke_session(session);
    assert!(
        rust.session.live.preview().is_some(),
        "level 1 exists on a 600 square"
    );
    rust.reset_pyramid();
    assert!(
        rust.session.live.preview().is_none(),
        "a full rebuild drops the preview"
    );
}

#[test]
fn selection_contour_cache_reuses_a_revision() {
    let mut rust = PictureViewRust::default();
    rust.session
        .set_selection(Some(pictura_select::Selection::all(8, 8)));
    let first = rust.selection_contour_extents();
    let second = rust.selection_contour_extents();
    assert_eq!(first, second);
    assert_eq!(rust.selection_contour_computes.get(), 1);
}

#[test]
fn selection_contour_cache_recomputes_after_a_selection_change() {
    let mut rust = PictureViewRust::default();
    rust.session
        .set_selection(Some(pictura_select::Selection::rect(8, 8, 1, 1, 3, 3)));
    let first = rust.selection_contour_extents();
    rust.session
        .set_selection(Some(pictura_select::Selection::rect(8, 8, 2, 2, 6, 6)));
    let second = rust.selection_contour_extents();
    assert_ne!(first, second);
    assert_eq!(rust.selection_contour_computes.get(), 2);
}

#[test]
fn selection_contour_cache_clears_without_a_selection() {
    let mut rust = PictureViewRust::default();
    rust.session
        .set_selection(Some(pictura_select::Selection::all(8, 8)));
    assert!(!rust.selection_contour_extents().is_empty());
    rust.session.set_selection(None);
    assert!(rust.selection_contour_extents().is_empty());
    assert!(rust.selection_contour_cache.borrow().is_none());
    assert_eq!(rust.selection_contour_computes.get(), 1);
}

#[test]
fn a_whole_document_region_refresh_repairs_the_cached_tiles() {
    let (w, h) = (300u32, 200u32);
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![super::tests::pixel_layer(
        "px",
        w,
        h,
        (10, 20, 30),
    )]);
    let mut view = view_of(doc);
    assert_tiles_match_a_rebuild(&view);
    let frame = view.frame_revision;

    let doc = view.session.doc.as_mut().expect("document");
    doc.layers[0].channels[0].data.as_mut_slice().fill(200);
    assert_eq!(view.refresh_regions(&[whole(w, h)]), Some(whole(w, h)));

    let doc = view.session.doc.as_ref().expect("document");
    assert!(
        !pictura_render::DocumentComposite::is_stored(doc),
        "a layered RGB document keeps no composite (#1216)"
    );
    assert_eq!(view.frame_revision, frame, "the frame is not rebuilt");
    assert_tiles_match_a_rebuild(&view);
}

#[test]
fn no_full_resolution_frame_is_kept() {
    let (w, h) = (2048u32, 1536u32);
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![super::tests::pixel_layer(
        "px",
        w,
        h,
        (10, 20, 30),
    )]);
    let mut view = view_of(doc);
    view.presented_level = 0;
    view.presented_rect = PsdRect {
        top: 600,
        left: 800,
        bottom: 900,
        right: 1200,
    };
    view.pin_presented_view();
    let (crop, missing) = view.tile_crop(TileSource::Composite, 0, view.presented_rect);
    assert!(missing.is_empty());
    assert_eq!((crop.width(), crop.height()), (400, 300));

    let doc = view.session.doc.as_ref().expect("document");
    assert!(!pictura_render::DocumentComposite::is_stored(doc));
    let resident = view.tile_stats().resident_bytes;
    let frame = u64::from(w) * u64::from(h) * 4;
    assert!(
        resident * 8 < frame,
        "only the visible tiles are resident: {resident} of a {frame}-byte frame"
    );
}

#[test]
fn a_region_repair_writes_only_its_tiles() {
    let (w, h) = (300u32, 400u32);
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(w, h, 0);
    let mut view = view_of(doc.clone());
    shown(&view, 0);
    let below = view
        .tiles
        .cache
        .borrow_mut()
        .get(key(0, 0, 1))
        .expect("cached");

    let rect = PsdRect {
        top: 10,
        left: 30,
        bottom: 60,
        right: 60,
    };
    let patch = varied_rgba(30, 50, 5);
    blit_region(&mut doc.composite, &crop_into(&patch, rect, w, h), rect);
    view.session.doc = Some(doc);
    view.apply_refreshed_region(rect.left, rect.top, rect, patch.clone());

    let after = view
        .tiles
        .cache
        .borrow_mut()
        .get(key(0, 0, 1))
        .expect("cached");
    assert!(
        std::sync::Arc::ptr_eq(&below, &after),
        "a tile the repair does not reach keeps its allocation"
    );
    let top = view
        .tiles
        .cache
        .borrow_mut()
        .get(key(0, 0, 0))
        .expect("cached");
    let at = ((rect.top as usize) * 256 + rect.left as usize) * 4;
    assert_eq!(top[at], patch.data[0], "the tile shows the patched pixels");
}

/// `patch`, sized to `rect`, placed into a `w`×`h` buffer at `rect`.
fn crop_into(patch: &PixelBuffer, rect: PsdRect, w: u32, h: u32) -> PixelBuffer {
    let mut out = PixelBuffer::new(w, h, 4);
    patch_buffer_region(&mut out, patch, rect.left, rect.top);
    out
}
