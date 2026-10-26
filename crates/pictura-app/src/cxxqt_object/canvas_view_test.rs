use super::helpers_composite::*;
use super::PictureViewRust;
use pictura_core::{BitDepth, ColorMode, Document, PixelBuffer, PsdRect};
use pictura_render::{CanvasDamage, ViewPyramid};

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

fn assert_pyramids_equal(a: &ViewPyramid, b: &ViewPyramid, source: &PixelBuffer) {
    assert_eq!(a.level_count(), b.level_count(), "level count");
    let planes = planes_of(source).expect("RGBA source");
    for level in 0..a.level_count() {
        let rect = whole_level(a, level);
        assert_eq!(
            a.crop(planes, level, rect).data(),
            b.crop(planes, level, rect).data(),
            "level {level}"
        );
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
fn level0_region_update_matches_a_rebuild() {
    let mut doc = Document::new(600, 400, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(600, 400, 0);
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc.clone()),
        ..Default::default()
    };
    rust.reset_pyramid();

    let reference = {
        let level0 = rust.level0.as_ref().expect("sRGB level-0 frame");
        ViewPyramid::rebuild(planes_of(level0).unwrap())
    };
    let level0 = rust.level0.as_ref().expect("sRGB level-0 frame");
    assert_pyramids_equal(&rust.pyramid, &reference, level0);

    // A described region change, applied to the app's own composite and folded
    // into the cached level-0 frame the way `refresh_region` does.
    let rect = PsdRect {
        top: 100,
        left: 120,
        bottom: 180,
        right: 260,
    };
    let changed = varied_rgba(600, 400, 77);
    blit_region(&mut doc.composite, &changed, rect);
    rust.session.doc = Some(doc.clone());
    rust.refresh_level0_region(crop_buffer(&changed, rect), rect.left, rect.top);
    rust.update_pyramid(rect);

    let level0 = rust.level0.as_ref().expect("sRGB level-0 frame");
    let reference = ViewPyramid::rebuild(planes_of(level0).unwrap());
    assert_pyramids_equal(&rust.pyramid, &reference, level0);
}

#[test]
fn level0_carries_the_document_profile_conversion() {
    let mut doc = Document::new(16, 16, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(16, 16, 4);
    pictura_codec::assign_document_profile(&mut doc, &pictura_codec::Profile::adobe_rgb());

    let expected = level0_buffer(&doc);
    let raw = rgba_frame(&doc.composite);
    assert_ne!(
        expected.data, raw.data,
        "a non-identity profile must convert the frame"
    );

    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        ..Default::default()
    };
    rust.reset_pyramid();
    let level0 = rust.level0.as_ref().expect("sRGB level-0 frame");
    assert_eq!(
        level0.data, expected.data,
        "the crop source is color managed"
    );
}

#[test]
fn region_refresh_reuses_the_level0_frame_when_it_matches() {
    let mut doc = Document::new(400, 300, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(400, 300, 0);
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc.clone()),
        ..Default::default()
    };
    rust.reset_pyramid();
    let before = rust
        .level0
        .as_ref()
        .expect("sRGB level-0 frame")
        .data
        .as_ptr();

    let rect = PsdRect {
        top: 20,
        left: 30,
        bottom: 80,
        right: 120,
    };
    let changed = varied_rgba(400, 300, 9);
    blit_region(&mut doc.composite, &changed, rect);
    rust.session.doc = Some(doc.clone());
    rust.refresh_level0_region(crop_buffer(&changed, rect), rect.left, rect.top);

    let level0 = rust.level0.as_ref().expect("sRGB level-0 frame");
    assert_eq!(
        level0.data.as_ptr(),
        before,
        "the frame is patched in place"
    );
    assert_eq!(level0.data, level0_buffer(&doc).data, "and stays in sync");
}

#[test]
fn image_after_region_refresh_matches_a_full_recomposite() {
    let mut doc = Document::new(64, 48, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(64, 48, 1);
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc.clone()),
        ..Default::default()
    };
    rust.reset_pyramid();
    let revision = rust.canvas_revision;

    // Replay `refresh_region`'s non-painting body, with no paint or `take`
    // between the refresh and the dirty rebuild `image()` performs.
    let rect = PsdRect {
        top: 8,
        left: 8,
        bottom: 24,
        right: 24,
    };
    let changed = varied_rgba(64, 48, 21);
    blit_region(&mut doc.composite, &changed, rect);
    rust.session.doc = Some(doc.clone());
    rust.refresh_level0_region(crop_buffer(&changed, rect), rect.left, rect.top);
    rust.damage.mark(rect);
    rust.update_pyramid(rect);

    assert!(
        rust.canvas_revision > revision,
        "refresh bumps the revision"
    );
    assert_eq!(
        rust.display_rect_damage(),
        Some(rect),
        "damage survives for image()"
    );

    let rebuilt =
        rebuild_display(&rust.session.doc, rust.session.live.stroke(), false).expect("document");
    let full = premultiplied_display_image(&level0_buffer(&doc));
    assert_eq!(rebuilt, full, "image() equals a full recomposite");
}

#[test]
fn histogram_source_level_is_bounded_and_size_independent() {
    // The Histogram panel reads the coarsest pyramid level. Its long side is
    // always at most 512, whatever the document size, so the binning cost does
    // not grow with the document.
    for (w, h) in [(513u32, 513u32), (4000, 3000), (8000, 100)] {
        let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
        doc.composite = varied_rgba(w, h, 3);
        let mut rust = PictureViewRust {
            session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
            ..Default::default()
        };
        rust.reset_pyramid();

        let levels = rust.display_level_count();
        assert!(levels >= 1, "{w}x{h} has at least one level");
        let (lw, lh) = rust
            .display_level_size(levels - 1)
            .expect("coarsest level size");
        assert!(
            lw <= 512 && lh <= 512,
            "coarsest level {lw}x{lh} for a {w}x{h} document"
        );

        let crop = rust.display_crop(
            levels - 1,
            PsdRect {
                top: 0,
                left: 0,
                bottom: lh as i32,
                right: lw as i32,
            },
        );
        assert!(!crop.is_null(), "the level crop is servable");
        assert_eq!((crop.width(), crop.height()), (lw as i32, lh as i32));
    }
}

#[test]
fn mid_stroke_pyramid_matches_a_full_recomposite_of_the_working_document() {
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
        ..Default::default()
    };
    rust.session.live.set_stroke_session(session);
    // A full rebuild while a stroke is live must use the painted document's
    // composited layers, not its stale cached `composite`.
    rust.reset_pyramid();
    {
        let working = rust.session.doc.as_ref().unwrap();
        let reference_level0 = level0_composited(working, false);
        let reference = ViewPyramid::rebuild(planes_of(&reference_level0).unwrap());
        assert_pyramids_equal(&rust.pyramid, &reference, &reference_level0);
    }

    // A mid-stroke region refresh keeps the pyramid equal to a full rebuild, so
    // the present path can crop it instead of resampling the full image.
    let working = rust.session.doc.as_ref().unwrap();
    let region = pictura_render::composite_region_active(working, rect, false).0;
    let level0 = level0_from_buffer(working, &region);
    rust.refresh_level0_region(level0, rect.left, rect.top);
    rust.update_pyramid(rect);
    let working = rust.session.doc.as_ref().unwrap();
    let reference_level0 = level0_composited(working, false);
    let reference = ViewPyramid::rebuild(planes_of(&reference_level0).unwrap());
    assert_pyramids_equal(&rust.pyramid, &reference, &reference_level0);
}

#[test]
fn paint_commit_region_patch_equals_a_full_recomposite() {
    use pictura_paint::{Stroke, StrokeConfig, StrokeSample};

    let rgba = vec![30u8; 128 * 128 * 4];
    let mut doc = Document::from_rgba("paint", 128, 128, &rgba);
    let cfg = StrokeConfig {
        diameter: 24,
        ..StrokeConfig::default()
    };
    let mut stroke = Stroke::begin_at(&doc, "0", cfg).expect("begin stroke");
    for i in 0..8 {
        assert!(stroke.sample(
            &mut doc,
            StrokeSample {
                x: 20.0 + i as f32 * 8.0,
                y: 64.0,
                pressure: 1.0,
            }
        ));
        stroke.take_dirty();
    }
    let rect = stroke.finish().expect("the stroke painted pixels");

    // Replay `end_paint`'s region commit against the committed bytes.
    let mut committed = doc.clone();
    let buffer = pictura_render::composite_region_active(&committed, rect, false).0;
    let x0 = rect.left.max(0);
    let y0 = rect.top.max(0);
    patch_composite_region(&mut committed, &buffer, x0, y0);

    let full = current_buffer(&doc, false);
    assert_eq!(
        committed.composite.data, full.data,
        "the stroke's committed region equals a full recomposite"
    );
}

/// Patching a damage rectangle into a cached level-0 crop only works if the
/// damage-rect crop is the same pixels as that rectangle of the cached crop.
/// The Qt test proves the in-place patch; this proves its positional premise.
#[test]
fn a_level_zero_damage_crop_matches_the_cached_crop_rect() {
    let rgba = vec![30u8; 200 * 160 * 4];
    let mut doc = Document::from_rgba("paint", 200, 160, &rgba);
    doc.composite = varied_rgba(200, 160, 9);
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        ..Default::default()
    };
    rust.reset_pyramid();
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

    let cache_crop = rust.display_crop(0, cached);
    let damage_crop = rust.display_crop(0, damage);
    let fresh = rust.display_crop(0, cached);
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
fn sample_argb_after_a_region_refresh_reads_the_composite() {
    let mut layer = vec![0u8; 64 * 32 * 4];
    for px in layer.chunks_exact_mut(4) {
        px.copy_from_slice(&[255, 0, 0, 255]);
    }
    let mut doc = Document::from_rgba("paint", 64, 32, &layer);
    doc.composite = varied_rgba(64, 32, 3);
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        ..Default::default()
    };
    rust.reset_pyramid();
    let outside_before = sample_planar_argb(&rust.session.doc.as_ref().unwrap().composite, 1, 1);
    let rect = PsdRect {
        top: 8,
        left: 10,
        bottom: 24,
        right: 40,
    };

    let (x0, y0, clipped, srgb) = rust
        .refresh_region_buffer(rect)
        .expect("the region composites pixels");
    rust.apply_refreshed_region(x0, y0, clipped, srgb);

    let doc = rust.session.doc.as_ref().expect("the document survives");
    let fresh = pictura_render::composite_rgba(doc);
    for &(x, y) in &[(20, 12), (39, 23)] {
        assert_eq!(
            sample_planar_argb(&doc.composite, x, y),
            sample_planar_argb(&fresh, x, y),
            "sample at {x},{y} must come from the refreshed composite"
        );
    }
    assert_eq!(
        sample_planar_argb(&doc.composite, 1, 1),
        outside_before,
        "a pixel outside the region keeps its value"
    );
}

#[test]
fn a_multi_rect_stroke_commit_equals_a_full_recomposite() {
    use super::helpers::TileSet;
    use pictura_paint::{Stroke, StrokeConfig, StrokeSample};

    let rgba = vec![30u8; 512 * 512 * 4];
    let mut doc = Document::from_rgba("paint", 512, 512, &rgba);
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

    // Replay `end_paint`'s per-rect commit against the committed bytes.
    let mut committed = doc.clone();
    for region in &regions {
        let buffer = pictura_render::composite_region_active(&committed, *region, false).0;
        patch_composite_region(
            &mut committed,
            &buffer,
            region.left.max(0),
            region.top.max(0),
        );
    }
    let full = current_buffer(&doc, false);
    assert_eq!(
        committed.composite.data, full.data,
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

    // Change the top layer's opacity, then refresh only its bounded rect the
    // way `mutate_layer` does.
    doc.layers.last_mut().unwrap().opacity = 128;
    let rect = doc.layers.last().unwrap().rect;
    let buffer = pictura_render::composite_region_active(&doc, rect, false).0;
    patch_composite_region(&mut doc, &buffer, rect.left, rect.top);

    let full = current_buffer(&doc, false);
    assert_eq!(
        doc.composite.data, full.data,
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

/// Commit a content move through the regional path, returning the document, the
/// composite before the move, and the refreshed dirty rectangle.
fn content_move_region_fixture() -> (Document, PixelBuffer, PsdRect) {
    let base: Vec<u8> = (0..64 * 64 * 4)
        .map(|i| if i % 4 == 3 { 255 } else { (i % 251) as u8 })
        .collect();
    let mut doc = Document::from_rgba("base", 64, 64, &base);
    let top: Vec<u8> = (0..64 * 64 * 4)
        .map(|i| if i % 4 == 3 { 255 } else { (i * 7 % 251) as u8 })
        .collect();
    let path = pictura_render::add_raster_layer_from_rgba(&mut doc, "top", 64, 64, &top);
    assert!(!path.is_empty());
    doc.composite = current_buffer(&doc, false);
    let before = doc.composite.clone();

    let selection = rect_selection(64, 64, 12, 10, 40, 36);
    let mask = selection_to_mask(&selection, &doc);
    let (dx, dy) = (9, 6);
    assert!(pictura_render::move_selection_content(
        &mut doc, &path, &mask, dx, dy, false
    ));
    let dirty = super::impl_selection::move_dirty_rect(&selection, dx, dy).expect("dirty rect");
    let buffer = pictura_render::composite_region_active(&doc, dirty, false).0;
    patch_composite_region(&mut doc, &buffer, dirty.left, dirty.top);
    (doc, before, dirty)
}

#[test]
fn content_move_region_equals_a_full_recomposite() {
    let (doc, _, dirty) = content_move_region_fixture();
    let full = current_buffer(&doc, false);
    assert_eq!(
        doc.composite.data, full.data,
        "a content move region refresh equals a full recomposite"
    );
    assert!(dirty.right > dirty.left && dirty.bottom > dirty.top);
}

#[test]
fn content_move_region_keeps_outside_pixels() {
    let (doc, before, dirty) = content_move_region_fixture();
    let plane = doc.composite.pixel_count();
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
                    doc.composite.data[c * plane + i],
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

    let before = doc.layers[index].rect;
    assert!(pictura_render::translate_layer_index(
        &mut doc, index, 10, 6
    ));
    let after = doc.layers[index].rect;
    let dirty = super::helpers::union_rect(before, after);
    let buffer = pictura_render::composite_region_active(&doc, dirty, false).0;
    patch_composite_region(&mut doc, &buffer, dirty.left, dirty.top);

    let full = current_buffer(&doc, false);
    assert_eq!(
        doc.composite.data, full.data,
        "a move region refresh equals a full recomposite"
    );
}

#[test]
fn a_level0_crop_outside_the_level_is_null_and_a_partial_one_is_not() {
    let mut doc = Document::new(100, 100, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(100, 100, 2);
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        ..Default::default()
    };
    rust.reset_pyramid();

    let outside = PsdRect {
        top: 2000,
        left: 2000,
        bottom: 2010,
        right: 2010,
    };
    assert!(rust.display_crop(0, outside).is_null());
    let partial = PsdRect {
        top: 90,
        left: 90,
        bottom: 110,
        right: 110,
    };
    let crop = rust.display_crop(0, partial);
    assert!(!crop.is_null());
    assert_eq!((crop.width(), crop.height()), (20, 20));
}

#[test]
fn reset_pyramid_rebuilds_and_clears_damage() {
    // `recomposite`, `undo`, `redo`, `history_jump`, `history_restore_snapshot`,
    // and the move-preview path all route through `reset_pyramid`.
    let first = varied_rgba(600, 400, 0);
    let mut doc = Document::new(600, 400, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = first.clone();
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        ..Default::default()
    };
    rust.reset_pyramid();
    assert_eq!(rust.damage, CanvasDamage::default());
    assert_pyramids_equal(
        &rust.pyramid,
        &ViewPyramid::rebuild(planes_of(&first).unwrap()),
        &first,
    );

    rust.damage.mark(PsdRect {
        top: 1,
        left: 1,
        bottom: 5,
        right: 5,
    });
    let second = varied_rgba(600, 400, 31);
    let mut restored = Document::new(600, 400, ColorMode::Rgb, BitDepth::Eight);
    restored.composite = second.clone();
    rust.session.doc = Some(restored);
    rust.reset_pyramid();
    assert_eq!(rust.damage, CanvasDamage::default(), "reset clears damage");
    assert_pyramids_equal(
        &rust.pyramid,
        &ViewPyramid::rebuild(planes_of(&second).unwrap()),
        &second,
    );
}

#[test]
fn display_accessors_report_levels_and_damage() {
    let mut doc = Document::new(1500, 700, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(1500, 700, 5);
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        ..Default::default()
    };
    rust.reset_pyramid();

    assert_eq!(rust.display_level_count(), 3);
    assert_eq!(rust.display_level_size(0), Some((1500, 700)));
    assert_eq!(rust.display_level_size(1), Some((750, 350)));
    assert_eq!(rust.display_level_size(2), Some((375, 175)));
    assert_eq!(rust.display_level_size(3), None);
    assert_eq!(rust.display_level_size(-1), None);

    // A level crop carries that level's size; out-of-range and empty rects are
    // null, not a panic.
    let crop = rust.display_crop(
        1,
        PsdRect {
            top: 10,
            left: 10,
            bottom: 26,
            right: 42,
        },
    );
    assert_eq!((crop.width(), crop.height()), (32, 16));
    assert!(!crop.is_null());
    assert!(rust
        .display_crop(3, whole_level(&rust.pyramid, 0))
        .is_null());
    assert!(rust
        .display_crop(-1, whole_level(&rust.pyramid, 0))
        .is_null());
    let empty = PsdRect {
        top: 0,
        left: 0,
        bottom: 0,
        right: 0,
    };
    assert!(rust.display_crop(0, empty).is_null());

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

/// The preview snapshots the stored view-pyramid level rather than the
/// document: the bytes are that level's, taken before the stroke, and for the
/// document sizes the threshold targets they stay small enough to be free.
#[test]
fn the_preview_snapshots_the_stored_level_and_stays_small() {
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

    let level0 = varied_rgba(900, 600, 3);
    let planes = planes_of(&level0).expect("RGBA level 0");
    let pyramid = ViewPyramid::rebuild(planes);
    let whole = whole_level(&pyramid, 1);

    let preview = PreviewStroke::new(1, &pyramid, &level0, pictura_paint::StrokeConfig::default())
        .expect("the level exists");
    assert_eq!(
        preview.snapshot,
        pyramid.crop(planes, 1, whole).data(),
        "the snapshot is the level's bytes, taken before anything is painted"
    );
    assert_eq!(preview.level, 1);
    assert_eq!(preview.scale, 2);
    assert_eq!(
        preview_snapshot_bytes(level0.width, level0.height, 1),
        preview.snapshot.len()
    );
    assert_eq!(
        preview.coverage.len(),
        (preview.width * preview.height) as usize,
        "the coverage buffer covers the level exactly once"
    );
    assert!(preview.extent.is_none(), "nothing presented yet");
    assert!(preview.samples.is_empty(), "nothing recorded yet");

    // A level the pyramid does not store is refused rather than snapshotted
    // as zeros.
    assert!(PreviewStroke::new(
        99,
        &pyramid,
        &level0,
        pictura_paint::StrokeConfig::default()
    )
    .is_none());
}

/// A full rebuild is the one path that writes every level from `level0`; it must
/// also drop a live preview so a stale snapshot cannot resurrect over it.
#[test]
fn reset_pyramid_drops_a_live_preview() {
    use super::document_session::{PreviewStroke, StrokeSession};

    let mut doc = Document::new(600, 600, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = varied_rgba(600, 600, 1);
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        ..Default::default()
    };
    rust.reset_pyramid();
    let level0 = rust.level0.as_ref().expect("level-0 frame");
    let preview = PreviewStroke::new(
        1,
        &rust.pyramid,
        level0,
        pictura_paint::StrokeConfig::default(),
    );
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
