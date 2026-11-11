//! The memory scenarios' steps (#1210), each the engine and session work of
//! the bridge call it names, in the order that call runs it. Geometry is given
//! at 17000² and scaled with the document, as `scripts/memory_scenarios.py`
//! does.

use crate::cxxqt_object::document_session::{DocumentSession, TransformMode};
use crate::cxxqt_object::edit::{insertion_edit, Edit};
use crate::cxxqt_object::helpers_composite::composite_regions;
use crate::cxxqt_object::impl_core::save_jobs::{save_finish_rust, save_start_rust};
use crate::cxxqt_object::impl_transform::translate_move_group;
use crate::cxxqt_object::open_jobs::{apply_opened, decode_psd, make_background};
use crate::cxxqt_object::state::PictureViewRust;
use crate::history::{History, Restored};
use crate::layer_target::LayerTarget;
use pictura_core::mask_ops::TiledMask;
use pictura_core::{BitDepth, Channel, ColorMode, Document, Layer, PsdRect};
use pictura_paint::bucket::{self, BucketPaint, BucketSource};
use pictura_paint::stroke::Stroke;
use pictura_paint::{Alpha, PaintMode, Rgba, StrokeConfig, StrokeSample};
use pictura_render::DocumentComposite;
use pictura_select::Selection;

const REFERENCE_SIDE: u32 = 17000;
const BLACK: Rgba = Rgba {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

/// `reference` pixels at 17000², scaled to `side` (at least one).
fn px(side: u32, reference: u32) -> i32 {
    (u64::from(reference) * u64::from(side) / u64::from(REFERENCE_SIDE)).max(1) as i32
}

pub(super) fn square(side: u32) -> PsdRect {
    let half = px(side, 1000) / 2;
    let c = side as i32 / 2;
    PsdRect {
        top: c - half,
        left: c - half,
        bottom: c + half,
        right: c + half,
    }
}

/// A file under the temp directory, removed on drop.
pub(super) struct TempFile(pub(super) String);

impl TempFile {
    pub(super) fn new(tag: &str) -> Self {
        let name = format!("pictura-heap-{}-{tag}.psd", std::process::id());
        Self(
            std::env::temp_dir()
                .join(name)
                .to_string_lossy()
                .into_owned(),
        )
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// M1, as `new_document` + `create_document`: a white locked Background, the
/// view and the "New" state. No composite is rendered or kept (#1216).
pub(super) fn new_document(side: u32) -> PictureViewRust {
    // As the bridge builds it: uniform white tiles that hold nothing (#1213).
    let mut doc =
        Document::with_uniform_composite(side, side, ColorMode::Rgb, BitDepth::Eight, 255);
    let n = side as usize;
    let channels = (0..4)
        .map(|id| Channel {
            id: if id == 3 { -1 } else { id },
            data: pictura_core::Plane::filled(n, n, 1, 255),
        })
        .collect();
    doc.layers
        .push(Layer {
            name: "Layer 0".to_string(),
            rect: canvas(side),
            opacity: 255,
            fill: 255,
            visible: true,
            channels,
            ..Default::default()
        })
        .expect("the layer is added");
    make_background(doc.layers.first_mut().expect("the layer"));
    DocumentComposite::new(&mut doc).refresh(pictura_render::CompositeScope::Full, false);
    let mut view = PictureViewRust {
        gpu_compute: false,
        ..Default::default()
    };
    view.replace_document(Some(doc), History::default(), LayerTarget::default(), None);
    view.select_only_layer("0");
    record(&mut view, "New");
    view
}

fn canvas(side: u32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: side as i32,
        right: side as i32,
    }
}

fn record(view: &mut PictureViewRust, label: &str) {
    let target = view.layer_target.clone();
    assert!(view.session.record(&target, label), "{label} records");
}

/// M2, as `add_layer(-1)`: Layer 1 on top of the Background (path "1"),
/// made active.
pub(super) fn add_layer(view: &mut PictureViewRust) {
    let doc = view.session.doc.as_mut().expect("document");
    let name = pictura_render::next_layer_name(doc, "Layer");
    let created = pictura_render::add_layer(doc, -1, &name);
    assert_eq!(created, 1, "Layer 1 lands above the Background");
    let path = created.to_string();
    let edit = insertion_edit(doc, &path);
    view.layer_target.select_only(&path);
    view.apply_edit("New Layer", edit);
}

/// The 1000² marquee at the centre, as the Marquee tool's mask.
pub(super) fn marquee(view: &mut PictureViewRust, side: u32) {
    let rect = square(side);
    view.session.set_selection(Some(Selection::rect(
        side,
        side,
        rect.left,
        rect.top,
        rect.width(),
        rect.height(),
    )));
}

/// `fills::apply`: run `edit` on a copy of the document, adopt it, refresh
/// the changed rectangle and record `label`.
fn apply_fill(
    view: &mut PictureViewRust,
    path: &str,
    label: &str,
    edit: impl FnOnce(&mut Document, PsdRect, Option<&TiledMask>) -> Option<PsdRect>,
) {
    view.layer_target = LayerTarget::only(path);
    let doc = view.session.doc.as_ref().expect("document");
    let canvas = canvas(doc.width);
    let selection = view.session.selection.as_ref().map(|s| &s.data);
    let bounds = match selection {
        Some(_) => view
            .selection_mask_bounds()
            .and_then(|b| b.intersect(canvas))
            .expect("a selection with pixels"),
        None => canvas,
    };
    let mut edited = doc.clone();
    let rect = edit(&mut edited, bounds, selection).expect("the fill changes pixels");
    view.session.doc = Some(edited);
    view.refresh_regions(&[rect]);
    record(view, label);
}

/// M3, as Edit ▸ Fill with the foreground on the Background.
pub(super) fn fill_background(view: &mut PictureViewRust) {
    apply_fill(view, "0", "Fill", |doc, bounds, selection| {
        bucket::fill_within(
            doc,
            "0",
            bounds,
            None,
            BucketPaint::Foreground(BLACK),
            PaintMode::Normal,
            1.0,
            selection,
            Alpha::Blend,
        )
    });
}

/// M4 and M5, as a Paint Bucket click at the centre of Layer 1 (tolerance
/// 32, contiguous, anti-aliased).
pub(super) fn bucket(view: &mut PictureViewRust) {
    bucket_at(view, 2);
}

/// The click at `1/quarter` of the document's side on both axes: M7's
/// whole-layer click lands outside the square (quarter 4), since a black
/// click on the black square changes nothing.
pub(super) fn bucket_at(view: &mut PictureViewRust, quarter: i32) {
    apply_fill(view, "1", "Paint Bucket", |doc, bounds, selection| {
        let seed = (doc.width as i32 / quarter, doc.height as i32 / quarter);
        let source = BucketSource::layer(doc, "1")?;
        let size = (doc.width, doc.height);
        let flood = bucket::flood(source, size, bounds, selection, seed, 32, true)?;
        let mask = bucket::antialias(flood, size, bounds, selection);
        bucket::fill_tiled(
            doc,
            "1",
            bounds,
            &mask,
            BucketPaint::Foreground(BLACK),
            PaintMode::Normal,
            1.0,
            selection,
            Alpha::Blend,
        )
    });
}

/// M6, as `begin_paint` / `paint_dab` / `end_paint`: a 300 px hard round
/// brush across 4000 px of Layer 1, in the 60 motion events the script sends.
pub(super) fn brush_stroke(view: &mut PictureViewRust, side: u32) {
    view.layer_target = LayerTarget::only("1");
    let cfg = StrokeConfig {
        color: BLACK,
        diameter: px(side, 300) as u32,
        hardness: 100,
        opacity: 100,
        flow: 100,
        ..StrokeConfig::default()
    }
    .sanitized();
    let doc = view.session.doc.as_mut().expect("document");
    let mut stroke = Stroke::begin_at(doc, "1", cfg).expect("a stroke");
    let (cx, cy) = (side as f32 / 2.0, side as f32 / 2.0);
    let half = px(side, 4000) as f32 / 2.0;
    for i in 0..=60 {
        let x = cx - half + 2.0 * half * i as f32 / 60.0;
        stroke.sample(
            doc,
            StrokeSample {
                x,
                y: cy,
                pressure: 1.0,
            },
        );
    }
    let rect = stroke.finish().expect("the stroke paints");
    view.apply_edit("Brush", Edit::regions(vec![rect], false));
}

/// M7, as three Undo and three Redo commands with their display refresh.
pub(super) fn undo_redo(view: &mut PictureViewRust) {
    for undo in [true, true, true, false, false, false] {
        let PictureViewRust {
            session,
            layer_target,
            ..
        } = view;
        let DocumentSession {
            doc,
            history,
            selection,
            ..
        } = session;
        let doc = doc.as_mut().expect("document");
        let restored = if undo {
            history.undo_live(doc, selection, layer_target)
        } else {
            history.redo_live(doc, selection, layer_target)
        }
        .expect("a state to step to");
        match restored {
            Restored::Region(rect) | Restored::Composite(rect) => {
                view.refresh_from_composite(rect);
            }
            Restored::Everywhere => view.reset_pyramid(),
        }
    }
}

/// M8, as `history_add_snapshot` then `history_restore_snapshot`.
pub(super) fn snapshot_restore(view: &mut PictureViewRust) {
    snapshot_restore_edit(view);
    view.reset_pyramid();
}

/// M8 without the display rebuild (#1216): the snapshot, the restore and its
/// recorded state, which share tiles instead of copying them (#1215).
pub(super) fn snapshot_restore_edit(view: &mut PictureViewRust) {
    let snapshot = view.named_snapshot().expect("a snapshot");
    let target = view.layer_target.clone();
    view.session
        .history
        .add_snapshot("Benchmark", snapshot, target);
    let snapshot = view.session.history.snapshot(0).expect("the snapshot");
    view.cancel_active_sessions_report_only();
    view.session.doc = Some(snapshot.doc);
    view.session.set_selection(snapshot.selection);
    record(view, "Benchmark");
}

/// M9, as a Move tool drag of Layer 1 by 500 px: the translate, the deferred
/// edit and its worker composite landing.
pub(super) fn move_layer(view: &mut PictureViewRust, side: u32) {
    view.layer_target = LayerTarget::only("1");
    let paths = vec!["1".to_string()];
    let doc = view.session.doc.as_mut().expect("document");
    let dirty = translate_move_group(doc, &paths, px(side, 500), 0).expect("moved");
    pictura_render::trim_to_content(doc, "1");
    view.apply_edit_move("Move", Edit::regions(vec![dirty], false));
    settle_move(view);
}

fn settle_move(view: &mut PictureViewRust) {
    if let Some(job) = view.pending_move_job.take() {
        let regions = composite_regions(&job.doc, &job.rects, false);
        assert!(view.land_move_composite(job.generation, regions).is_some());
    } else {
        view.settle_move_composite();
    }
}

/// M10, as Free Transform of Layer 1 at 150 % and 15° about its centre,
/// committed: the session's trim, the resample and the edit.
pub(super) fn free_transform(view: &mut PictureViewRust) {
    view.layer_target = LayerTarget::only("1");
    let session = view
        .prepare_transform("1", TransformMode::Free)
        .expect("a transform session");
    let (orig, interpolation) = (session.orig_rect, session.interpolation);
    drop(session);
    let doc = view.session.doc.as_mut().expect("document");
    let transform = pictura_render::LayerTransform {
        scale_x: 1.5,
        scale_y: 1.5,
        angle_radians: 15f64.to_radians(),
        dx: 0.0,
        dy: 0.0,
    };
    assert!(pictura_render::transform_layer_with(
        doc,
        "1",
        transform,
        interpolation
    ));
    let after = pictura_render::resolve_path(doc, "1").expect("layer").rect;
    view.apply_edit_move(
        "Free Transform",
        Edit::regions(vec![orig.union(after)], false),
    );
    settle_move(view);
}

/// M11, as Save As PSD: the save job's snapshot, its worker's encode and write.
pub(super) fn save_psd(view: &mut PictureViewRust, file: &TempFile) {
    assert!(save_start_rust(view, &file.0), "the save starts");
    assert!(save_finish_rust(view), "the save lands");
}

/// M12's file: a white Background under four layers of seeded opaque discs,
/// the shape `scripts/gen-large-fixtures.py` writes.
pub(super) fn write_fixture(side: u32, file: &TempFile) -> String {
    let mut doc = new_document(side).session.doc.take().expect("document");
    let mut state = 1210u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 33) as f64 / (1u64 << 31) as f64
    };
    let n = side as usize;
    for index in 0..4 {
        let mut planes = vec![vec![0u8; n * n]; 4];
        for _ in 0..60 {
            let r = side as f64 * (1.0 / 40.0 + next() * (1.0 / 12.0 - 1.0 / 40.0));
            let (cx, cy) = (next() * side as f64, next() * side as f64);
            let rgb = [next(), next(), next()].map(|v| (v * 255.0) as u8);
            let (y0, y1) = ((cy - r).max(0.0) as usize, ((cy + r) as usize).min(n));
            for y in y0..y1 {
                let dy = y as f64 + 0.5 - cy;
                let half = (r * r - dy * dy).max(0.0).sqrt();
                let x0 = (cx - half + 0.5).max(0.0) as usize;
                let x1 = ((cx + half + 0.5) as usize).min(n);
                for (c, plane) in planes.iter_mut().enumerate() {
                    let value = if c == 3 { 255 } else { rgb[c] };
                    plane[y * n + x0.min(x1)..y * n + x1].fill(value);
                }
            }
        }
        let channels = planes
            .into_iter()
            .enumerate()
            .map(|(c, data)| Channel {
                id: if c == 3 { -1 } else { c as i16 },
                data: data.into(),
            })
            .collect();
        doc.layers
            .push(Layer {
                name: format!("Paint {}", index + 1),
                rect: canvas(side),
                opacity: 255,
                fill: 255,
                visible: true,
                channels,
                ..Default::default()
            })
            .expect("the layer is added");
    }
    doc.composite = pictura_render::composite_rgba(&doc);
    let bytes = pictura_codec::write_psd(&doc).expect("the fixture encodes");
    std::fs::write(&file.0, bytes).expect("the fixture is written");
    file.0.clone()
}

/// M12, as File ▸ Open of a PSD: the open job's decode and its landing.
pub(super) fn open_psd(path: &str) {
    assert!(opened(path).session.doc.is_some());
}

/// The document at `path`, opened as the open job lands it.
pub(super) fn opened(path: &str) -> PictureViewRust {
    let opened = decode_psd(path, Default::default(), false, &|_, _| {}).expect("the PSD opens");
    let mut view = PictureViewRust {
        gpu_compute: false,
        ..Default::default()
    };
    apply_opened(&mut view, opened);
    view
}

/// The Magic Wand on the first disc the middle row of the M12 fixture meets
/// (tolerance 32, contiguous), as Sample All Layers runs it: over the
/// composite a tile at a time (#1216).
pub(super) fn magic_wand(view: &mut PictureViewRust) {
    let doc = view.session.doc.as_ref().expect("document");
    let side = doc.width as usize;
    let y = side as i32 / 2;
    let row = crate::cxxqt_object::picture::picture_region(
        doc,
        view,
        PsdRect {
            top: y,
            left: 0,
            bottom: y + 1,
            right: side as i32,
        },
    );
    let mut red = vec![0u8; side];
    row.data.read_into(0, &mut red);
    let x = red
        .iter()
        .position(|&v| v != 255)
        .expect("a disc on the middle row");
    let selection = {
        let fetch = crate::cxxqt_object::picture::sample_tiles(doc, view);
        let source = pictura_paint::bucket::BucketSource::Tiles {
            fetch: &fetch,
            width: doc.width,
            height: doc.height,
        };
        let size = (doc.width, doc.height);
        let data = pictura_paint::bucket::magic_flood(source, size, (x as i32, y), 32, true)
            .expect("a flood");
        pictura_select::Selection {
            width: doc.width,
            height: doc.height,
            data,
        }
    };
    view.session.set_selection(Some(selection));
}
