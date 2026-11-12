//! The canvas display as a tile pyramid (#1216).
//!
//! The view keeps no full-resolution frame and no whole pyramid level: the
//! canvas, the panels and the previews ask for a rectangle of a level, and the
//! tiles under it come from [`TilePyramid`]. A missing tile is built from its
//! level-0 source: the composite in the display space, or the soloed mask.
//!
//! - **Small documents** (at most [`SYNC_PIXELS`]) and live strokes build
//!   missing tiles at once, so every presentation is exact.
//! - **Large documents** answer at once from stale or coarser tiles and build
//!   the exact ones on a worker; landing them bumps the canvas revision and
//!   emits `pyramid_settled` so the canvas repaints.
//!
//! An edit that already composited its region repairs the tiles from it
//! ([`PictureViewRust::apply_refreshed_region`]). A region refresh that would
//! have to composite more than [`REPAIR_PIXELS`] on a large document marks the
//! tiles stale instead, and only the visible ones are rebuilt.

use super::helpers::paint_timing;
use super::helpers_composite::{crop_planar, into_rgba_frame, level_display_image};
use super::impl_core::mask_region_plane;
use super::qobject;
use super::state::PictureViewRust;
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QImage;
use pictura_core::{Document, LayerMask, PixelBuffer, PsdRect};
use pictura_render::{
    build_tile, patch_buffer_region, CompositeScope, DocumentComposite, PyramidLevel, Ticket,
    TileKey, TilePyramid, TileSource, TILE_SIDE,
};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

/// Documents of at most this many pixels build missing tiles synchronously.
pub(super) const SYNC_PIXELS: u64 = 16_000_000;

/// The most level-0 pixels a region refresh composites at once on a large
/// document; above it the tiles go stale and only the visible ones rebuild.
pub(super) const REPAIR_PIXELS: u64 = 4_000_000;

/// The view's tile cache and its worker queue. The cache sits in a `RefCell`
/// because the canvas and the panels read crops through `&self`; everything
/// runs on the GUI thread.
pub(in crate::cxxqt_object) struct DisplayTiles {
    pub(in crate::cxxqt_object) cache: RefCell<TilePyramid>,
    /// Keys asked for and not yet handed to a worker.
    wanted: RefCell<HashSet<TileKey>>,
    /// Whether a worker build is running; one runs at a time.
    in_flight: Cell<bool>,
    /// Test seam: build large-document tiles on workers even when small.
    pub(in crate::cxxqt_object) force_async: Cell<bool>,
    /// The eviction pass the cache last dropped for (#1217): a newer one
    /// drops the unpinned tiles, the first memory R2 frees.
    drop_epoch: Cell<u64>,
    /// Keeps the layer tiles read for the presented view resident until the
    /// view changes (#1217).
    view_hold: RefCell<Option<pictura_core::memory::eviction::HotHold>>,
    /// The view the hold was taken for.
    view_key: Cell<(i32, PsdRect)>,
}

impl Default for DisplayTiles {
    fn default() -> Self {
        Self {
            cache: RefCell::new(TilePyramid::new(0, 0, 0)),
            wanted: RefCell::new(HashSet::new()),
            in_flight: Cell::new(false),
            force_async: Cell::new(false),
            drop_epoch: Cell::new(pictura_core::memory::eviction::display_drop_epoch()),
            view_hold: RefCell::new(None),
            view_key: Cell::new((-1, PsdRect::default())),
        }
    }
}

/// Straight-alpha sRGB RGBA, planar, for `rect` of `doc` as the canvas shows
/// it. A document whose stored composite is authoritative crops it (unless a
/// stroke is writing the layers, which the stored composite lags); any other
/// document composites the region.
pub(in crate::cxxqt_object) fn display_region(
    doc: &Document,
    rect: PsdRect,
    painting: bool,
) -> PixelBuffer {
    let stored = !painting && doc.composite_is_authoritative() && DocumentComposite::is_stored(doc);
    let raw = if stored {
        crop_planar(
            &doc.composite,
            rect.left,
            rect.top,
            rect.width().max(0) as u32,
            rect.height().max(0) as u32,
        )
    } else {
        pictura_render::composite_region(doc, rect)
    };
    into_rgba_frame(pictura_codec::buffer_to_srgb(doc, &raw).into_owned())
}

/// What a worker needs to build tiles off the GUI thread.
struct TileJob {
    doc: Document,
    mask: Option<LayerMask>,
    painting: bool,
    ticket: Ticket,
    keys: Vec<(TileKey, PsdRect)>,
}

impl TileJob {
    /// Build the tiles one after another: each tile's composite already runs
    /// on every core, and building them side by side would hold one level-0
    /// chunk per tile at once (over a gigabyte at 17000² fit, #1216).
    fn build(&self) -> Vec<(TileKey, Arc<[u8]>)> {
        let size = (self.doc.width, self.doc.height);
        self.keys
            .iter()
            .map(|&(key, rect)| {
                let data = build_one(
                    &self.doc,
                    self.mask.as_ref(),
                    self.painting,
                    key,
                    rect,
                    size,
                    None,
                );
                (key, data)
            })
            .collect()
    }
}

fn build_one(
    doc: &Document,
    mask: Option<&LayerMask>,
    painting: bool,
    key: TileKey,
    rect: PsdRect,
    size: (u32, u32),
    cache: Option<&TilePyramid>,
) -> Arc<[u8]> {
    match (key.source, mask) {
        (TileSource::Mask, Some(mask)) => {
            let source = |r: PsdRect| mask_region_plane(mask, r);
            build_tile(cache, key, rect, size, &source)
        }
        _ => {
            let source = |r: PsdRect| display_region(doc, r, painting);
            build_tile(cache, key, rect, size, &source)
        }
    }
}

impl PictureViewRust {
    /// The source the canvas presents: the soloed mask, else the composite.
    pub(crate) fn canvas_source(&self) -> TileSource {
        if self.mask_solo_active() {
            TileSource::Mask
        } else {
            TileSource::Composite
        }
    }

    /// Whether the canvas is currently presenting the solo mask.
    pub(crate) fn mask_solo_active(&self) -> bool {
        self.solo_mask().is_some()
    }

    /// The soloed raster mask, while its layer still carries one.
    pub(in crate::cxxqt_object) fn solo_mask(&self) -> Option<&LayerMask> {
        let path = self.mask_solo.as_ref()?;
        let doc = self.session.doc.as_ref()?;
        pictura_render::resolve_path(doc, path)?.mask.as_ref()
    }

    fn painting(&self) -> bool {
        self.session.live.stroke().is_some() || self.session.live.mask_stroke().is_some()
    }

    /// Whether missing tiles are built at once on the GUI thread.
    fn tiles_sync(&self) -> bool {
        let pixels = self
            .session
            .doc
            .as_ref()
            .map_or(0, |doc| u64::from(doc.width) * u64::from(doc.height));
        (pixels <= SYNC_PIXELS && !self.tiles.force_async.get()) || self.painting()
    }

    /// Make the cache the document's size, starting it empty when it was not.
    pub(in crate::cxxqt_object) fn fit_tiles(&self) {
        let (w, h) = self
            .session
            .doc
            .as_ref()
            .map_or((0, 0), |doc| (doc.width, doc.height));
        let mut cache = self.tiles.cache.borrow_mut();
        if !cache.fits(w, h) {
            let budget = pictura_core::memory::governor().display_budget();
            *cache = TilePyramid::new(w, h, budget);
            self.tiles.wanted.borrow_mut().clear();
        }
    }

    /// The whole display changed: every tile goes stale (kept as a stand-in).
    pub(in crate::cxxqt_object) fn invalidate_tiles(&mut self) {
        self.fit_tiles();
        self.tiles.cache.get_mut().invalidate();
        self.tiles.wanted.get_mut().clear();
    }

    /// The soloed mask changed as a whole (entered, restored): its tiles go
    /// stale and the composite's stay.
    pub(crate) fn invalidate_mask_tiles(&mut self) {
        self.fit_tiles();
        let (w, h) = self.tiles.cache.get_mut().size();
        let whole = PsdRect {
            top: 0,
            left: 0,
            bottom: h as i32,
            right: w as i32,
        };
        self.tiles.cache.get_mut().damage(TileSource::Mask, whole);
    }

    /// The level count and sizes the canvas presents from.
    pub(crate) fn display_level_count(&self) -> i32 {
        self.fit_tiles();
        match self.session.doc.as_ref() {
            Some(_) => self.tiles.cache.borrow().level_count() as i32,
            None => 0,
        }
    }

    pub(crate) fn display_level_size(&self, level: i32) -> Option<(u32, u32)> {
        if level < 0 || level >= self.display_level_count() {
            return None;
        }
        Some(self.tiles.cache.borrow().level_size(level as usize))
    }

    /// Whether `level` exists and `rect` (in its pixels) overlaps it: a crop
    /// that misses the level is a null image, not a transparent one.
    pub(crate) fn level_meets(&self, level: i32, rect: PsdRect) -> bool {
        let Some((lw, lh)) = self.display_level_size(level) else {
            return false;
        };
        rect.right > 0 && rect.bottom > 0 && rect.left < lw as i32 && rect.top < lh as i32
    }

    /// `rect` of `level` of `source`, premultiplied, plus the keys still to
    /// build. Missing tiles are built here when [`Self::tiles_sync`]; otherwise
    /// they are stood in for and returned for a worker.
    pub(crate) fn tile_crop(
        &self,
        source: TileSource,
        level: i32,
        rect: PsdRect,
    ) -> (PyramidLevel, Vec<TileKey>) {
        self.fit_tiles();
        let level = level.max(0) as usize;
        let (crop, missing) = self.tiles.cache.borrow_mut().crop(source, level, rect);
        if missing.is_empty() || !self.tiles_sync() {
            return (crop, missing);
        }
        self.build_now(&missing);
        let (crop, missing) = self.tiles.cache.borrow_mut().crop(source, level, rect);
        (crop, missing)
    }

    /// Build `keys` on this thread and cache them.
    fn build_now(&self, keys: &[TileKey]) {
        let Some(doc) = self.session.doc.as_ref() else {
            return;
        };
        let t = Instant::now();
        let mask = self.solo_mask();
        let painting = self.painting();
        let size = (doc.width, doc.height);
        let mut keys = keys.to_vec();
        // Finer levels first, so a coarser tile can fold its cached children.
        keys.sort_by_key(|key| key.level);
        for key in keys {
            let built = {
                let cache = self.tiles.cache.borrow();
                let rect = cache.tile_rect(key);
                build_one(doc, mask, painting, key, rect, size, Some(&cache))
            };
            self.tiles.cache.borrow_mut().insert(key, built);
        }
        paint_timing::record("tiles_build_sync", t.elapsed());
    }

    /// A premultiplied display image of `rect` of `level` of the composite,
    /// for the panels and dialogs, and the keys left to a worker.
    pub(crate) fn display_crop_keys(&self, level: i32, rect: PsdRect) -> (QImage, Vec<TileKey>) {
        if !self.level_meets(level, rect) {
            return (QImage::default(), Vec::new());
        }
        let (crop, missing) = self.tile_crop(TileSource::Composite, level, rect);
        if crop.width() == 0 || crop.height() == 0 {
            return (QImage::default(), missing);
        }
        (level_display_image(&crop), missing)
    }

    /// Pin the presented view's tiles (and the coarsest level, which the
    /// Navigator and Histogram read) and shed unpinned tiles under pressure.
    pub(in crate::cxxqt_object) fn pin_presented_view(&mut self) {
        self.fit_tiles();
        let source = self.canvas_source();
        let level = self.presented_level.max(0) as usize;
        let cache = self.tiles.cache.get_mut();
        if level >= cache.level_count() {
            return;
        }
        let step = 1i32 << level;
        let margin = TILE_SIDE as i32;
        let r = self.presented_rect;
        let view = PsdRect {
            top: r.top.div_euclid(step) - margin,
            left: r.left.div_euclid(step) - margin,
            bottom: (r.bottom + step - 1).div_euclid(step) + margin,
            right: (r.right + step - 1).div_euclid(step) + margin,
        };
        let coarsest = cache.level_count() - 1;
        let (cw, ch) = cache.level_size(coarsest);
        let mut keys = cache.keys_in(source, level, view);
        keys.extend(cache.keys_in(
            TileSource::Composite,
            coarsest,
            PsdRect {
                top: 0,
                left: 0,
                bottom: ch as i32,
                right: cw as i32,
            },
        ));
        cache.pin(keys);
        let epoch = pictura_core::memory::eviction::display_drop_epoch();
        let asked = epoch != self.tiles.drop_epoch.replace(epoch);
        if asked
            || pictura_core::memory::governor().status().pressure
                == pictura_core::memory::Pressure::Critical
        {
            cache.drop_unpinned();
        }
        if self.tiles.view_key.replace((self.presented_level, r)) != (self.presented_level, r) {
            let mut hold = self.tiles.view_hold.borrow_mut();
            let next = pictura_core::memory::eviction::hot();
            *hold = Some(next);
        }
    }

    /// Fold an already composited straight-sRGB `region` of document rect
    /// `clipped` into the tiles, widening it to the repair alignment from the
    /// display source where it falls short.
    pub(in crate::cxxqt_object) fn repair_display(
        &mut self,
        clipped: PsdRect,
        region: PixelBuffer,
    ) {
        self.fit_tiles();
        let painting = self.painting();
        let cache = self.tiles.cache.get_mut();
        if !cache.any_current_under(TileSource::Composite, clipped) {
            cache.damage(TileSource::Composite, clipped);
            return;
        }
        let aligned = cache.repair_rect(clipped);
        let buffer = match self.session.doc.as_ref() {
            Some(doc) if aligned != clipped => widen(doc, painting, clipped, region, aligned),
            _ => region,
        };
        self.tiles
            .cache
            .get_mut()
            .repair(TileSource::Composite, aligned, &buffer);
    }

    /// Repair a soloed mask's tiles over `clipped` from the mask plane, or mark
    /// them stale when the region is large on a large document.
    pub(crate) fn sync_mask_solo_region(&mut self, clipped: PsdRect) {
        if self.mask_solo.is_none() {
            return;
        }
        let Some(mask) = self.solo_mask().cloned() else {
            self.mask_solo = None;
            self.drop_mask_solo_source();
            return;
        };
        self.fit_tiles();
        let sync = self.tiles_sync();
        let cache = self.tiles.cache.get_mut();
        let aligned = cache.repair_rect(clipped);
        let area = aligned.width().max(0) as u64 * aligned.height().max(0) as u64;
        if !cache.any_current_under(TileSource::Mask, aligned) || (!sync && area > REPAIR_PIXELS) {
            cache.damage(TileSource::Mask, aligned);
            return;
        }
        cache.repair(
            TileSource::Mask,
            aligned,
            &mask_region_plane(&mask, aligned),
        );
    }

    /// End mask solo's display: drop the mask tiles and any still queued.
    pub(crate) fn drop_mask_solo_source(&mut self) {
        self.tiles.cache.get_mut().drop_source(TileSource::Mask);
        self.tiles
            .wanted
            .get_mut()
            .retain(|key| key.source != TileSource::Mask);
    }

    /// The solo target changed or a restore replaced masks: present the mask
    /// afresh, or end the mode when its mask is gone.
    pub(crate) fn rebuild_mask_solo_source(&mut self) {
        if self.mask_solo.is_some() && self.solo_mask().is_none() {
            self.mask_solo = None;
        }
        self.invalidate_mask_tiles();
    }

    /// Bring a soloed mask up to a restore that changed masks.
    pub(in crate::cxxqt_object) fn refresh_mask_solo(&mut self) {
        if self.mask_solo.is_some() {
            self.rebuild_mask_solo_source();
            self.frame_revision = self.frame_revision.wrapping_add(1);
        }
    }

    /// Composite `rect` of the current source and fold it into the tiles and
    /// the damage account, returning the clipped document rectangle, or `None`
    /// for an empty clamp or no document.
    ///
    /// A document whose stored composite is authoritative keeps it current
    /// (except under a live mask stroke, which renders without writing). On
    /// any other large document a region above [`REPAIR_PIXELS`] is left
    /// stale instead of composited: only the visible tiles rebuild.
    pub(in crate::cxxqt_object) fn refresh_display_rect(
        &mut self,
        rect: PsdRect,
    ) -> Option<PsdRect> {
        self.fit_tiles();
        let painting_mask = self.session.live.mask_stroke().is_some();
        let gpu_compute = self.gpu_compute;
        let sync = self.tiles_sync();
        let (width, height, authoritative) = {
            let doc = self.session.doc.as_ref()?;
            (doc.width, doc.height, doc.composite_is_authoritative())
        };
        let clipped = clip(rect, width, height)?;
        let aligned = self.tiles.cache.get_mut().repair_rect(clipped);
        let area = aligned.width() as u64 * aligned.height() as u64;
        let has_current = self
            .tiles
            .cache
            .get_mut()
            .any_current_under(TileSource::Composite, aligned);
        if !authoritative && (!has_current || (!sync && area > REPAIR_PIXELS)) {
            #[cfg(test)]
            if !painting_mask {
                super::impl_core::note_refresh();
            }
            self.tiles
                .cache
                .get_mut()
                .damage(TileSource::Composite, aligned);
            self.damage.mark(clipped);
            self.canvas_revision = self.canvas_revision.wrapping_add(1);
            self.sync_mask_solo_region(clipped);
            return Some(clipped);
        }
        let t = Instant::now();
        let refreshed = {
            let doc = self.session.doc.as_mut()?;
            let mut composite = DocumentComposite::new(doc);
            let scope = CompositeScope::Regions(vec![aligned]);
            if painting_mask {
                composite.render(scope, gpu_compute)
            } else {
                composite.refresh(scope, gpu_compute)
            }
        };
        paint_timing::record("rr_composite_region", t.elapsed());
        #[cfg(test)]
        if !painting_mask {
            super::impl_core::note_refresh();
        }
        let doc = self.session.doc.as_ref()?;
        let region = match refreshed.into_iter().next() {
            Some(refreshed) => {
                into_rgba_frame(pictura_codec::buffer_to_srgb(doc, &refreshed.buffer).into_owned())
            }
            // A layerless document has nothing to composite: its stored
            // composite is the picture.
            None => display_region(doc, aligned, false),
        };
        let t = Instant::now();
        self.apply_refreshed_region(aligned.left, aligned.top, aligned, region);
        paint_timing::record("rr_tiles_repair", t.elapsed());
        Some(clipped)
    }

    /// Fold a refreshed straight-sRGB region into the tiles and the damage
    /// account, and keep a soloed mask in step.
    pub(in crate::cxxqt_object) fn apply_refreshed_region(
        &mut self,
        _x0: i32,
        _y0: i32,
        clipped: PsdRect,
        srgb: PixelBuffer,
    ) {
        if srgb.width == 0 || srgb.height == 0 {
            return;
        }
        self.repair_display(clipped, srgb);
        self.damage.mark(clipped);
        self.canvas_revision = self.canvas_revision.wrapping_add(1);
        self.sync_mask_solo_region(clipped);
    }

    /// Refresh several rectangles and return their union, or `None` when
    /// nothing landed.
    pub(super) fn refresh_regions(&mut self, rects: &[PsdRect]) -> Option<PsdRect> {
        let mut union: Option<PsdRect> = None;
        for &rect in rects {
            if let Some(clipped) = self.refresh_display_rect(rect) {
                union = Some(union.map_or(clipped, |u| u.union(clipped)));
            }
        }
        union
    }

    /// The whole level `level` of the composite, exact, for a reduced-level
    /// stroke preview's stroke-start snapshot. `None` past the last level.
    pub(in crate::cxxqt_object) fn level_snapshot(&self, level: u32) -> Option<PyramidLevel> {
        self.fit_tiles();
        let level = level as usize;
        let (w, h) = {
            let cache = self.tiles.cache.borrow();
            if level >= cache.level_count() {
                return None;
            }
            cache.level_size(level)
        };
        let whole = PsdRect {
            top: 0,
            left: 0,
            bottom: h as i32,
            right: w as i32,
        };
        let (_, missing) = self
            .tiles
            .cache
            .borrow_mut()
            .crop(TileSource::Composite, level, whole);
        if !missing.is_empty() {
            self.build_now(&missing);
        }
        let (crop, missing) =
            self.tiles
                .cache
                .borrow_mut()
                .crop(TileSource::Composite, level, whole);
        missing.is_empty().then_some(crop)
    }

    /// Write a display-only preview's rows into the cached tiles of `level`.
    pub(in crate::cxxqt_object) fn patch_display_level(
        &mut self,
        level: usize,
        rect: PsdRect,
        rows: &[u8],
    ) {
        self.fit_tiles();
        let source = self.canvas_source();
        self.tiles.cache.get_mut().patch(source, level, rect, rows);
    }

    /// The cache accounting (#942), for tests and profiles.
    #[cfg(test)]
    pub(crate) fn tile_stats(&self) -> pictura_render::TileStats {
        self.tiles.cache.borrow().stats()
    }
}

/// `region` (straight planar, covering `clipped`) grown to `aligned` by
/// compositing the border strips it lacks.
fn widen(
    doc: &Document,
    painting: bool,
    clipped: PsdRect,
    region: PixelBuffer,
    aligned: PsdRect,
) -> PixelBuffer {
    let mut out = PixelBuffer::new(aligned.width() as u32, aligned.height() as u32, 4);
    patch_buffer_region(
        &mut out,
        &region,
        clipped.left - aligned.left,
        clipped.top - aligned.top,
    );
    let strips = [
        PsdRect {
            bottom: clipped.top,
            ..aligned
        },
        PsdRect {
            top: clipped.bottom,
            ..aligned
        },
        PsdRect {
            top: clipped.top,
            bottom: clipped.bottom,
            right: clipped.left,
            ..aligned
        },
        PsdRect {
            top: clipped.top,
            bottom: clipped.bottom,
            left: clipped.right,
            ..aligned
        },
    ];
    for strip in strips {
        if strip.width() > 0 && strip.height() > 0 {
            let pixels = display_region(doc, strip, painting);
            patch_buffer_region(
                &mut out,
                &pixels,
                strip.left - aligned.left,
                strip.top - aligned.top,
            );
        }
    }
    out
}

fn clip(rect: PsdRect, width: u32, height: u32) -> Option<PsdRect> {
    let clipped = PsdRect {
        top: rect.top.max(0),
        left: rect.left.max(0),
        bottom: rect.bottom.min(height as i32),
        right: rect.right.min(width as i32),
    };
    (clipped.width() > 0 && clipped.height() > 0).then_some(clipped)
}

impl qobject::PictureView {
    /// Queue `keys` for a worker build, starting one if none is running.
    pub(in crate::cxxqt_object) fn request_tiles(&self, keys: Vec<TileKey>) {
        if keys.is_empty() {
            return;
        }
        self.rust().tiles.wanted.borrow_mut().extend(keys);
        if !self.rust().tiles.in_flight.get() {
            self.start_tile_job();
        }
    }

    fn start_tile_job(&self) {
        let rust = self.rust();
        let Some(doc) = rust.session.doc.as_ref() else {
            rust.tiles.wanted.borrow_mut().clear();
            return;
        };
        let soloed = rust.mask_solo_active();
        let (keys, ticket) = {
            let cache = rust.tiles.cache.borrow();
            // A mask key queued before solo ended has no mask to build from.
            let keys: Vec<(TileKey, PsdRect)> = rust
                .tiles
                .wanted
                .borrow_mut()
                .drain()
                .filter(|&key| key.source == TileSource::Composite || soloed)
                .filter(|&key| !cache.contains(key) && (key.level as usize) < cache.level_count())
                .map(|key| (key, cache.tile_rect(key)))
                .collect();
            (keys, cache.ticket())
        };
        if keys.is_empty() {
            return;
        }
        let job = TileJob {
            doc: doc.clone(),
            mask: rust.solo_mask().cloned(),
            painting: rust.painting(),
            ticket,
            keys,
        };
        rust.tiles.in_flight.set(true);
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let built = job.build();
            let ticket = job.ticket;
            // Before landing, so a write does not fork tiles nobody reads.
            drop(job);
            qt_thread
                .queue(move |view| view.land_tiles(ticket, built))
                .ok();
        });
    }

    /// Install a worker's tiles; a refused one (damage overtook it) is asked
    /// for again. The canvas repaints when anything landed.
    fn land_tiles(mut self: Pin<&mut Self>, ticket: Ticket, built: Vec<(TileKey, Arc<[u8]>)>) {
        let landed = {
            let mut rust = self.as_mut().rust_mut();
            rust.tiles.in_flight.set(false);
            rust.fit_tiles();
            let mut landed = false;
            let mut refused = Vec::new();
            let soloed = rust.mask_solo_active();
            {
                let mut cache = rust.tiles.cache.borrow_mut();
                // Mask tiles built for a solo that has since ended are dropped.
                for (key, data) in built
                    .into_iter()
                    .filter(|(key, _)| key.source == TileSource::Composite || soloed)
                {
                    if cache.install(key, ticket, data) {
                        landed = true;
                    } else if !cache.contains(key) {
                        refused.push(key);
                    }
                }
            }
            rust.tiles.wanted.borrow_mut().extend(refused);
            if landed {
                rust.canvas_revision = rust.canvas_revision.wrapping_add(1);
            }
            landed
        };
        if landed {
            self.as_mut().pyramid_settled();
        }
        if !self.rust().tiles.wanted.borrow().is_empty() {
            self.as_ref().get_ref().start_tile_job();
        }
    }
}

#[cfg(test)]
impl PictureViewRust {
    /// Build `keys` the way a worker does, from a copy of the document, and
    /// install them: a large document's tile job run inline for tests and
    /// profiles. Returns how many landed.
    pub(in crate::cxxqt_object) fn run_tile_job(&self, keys: &[TileKey]) -> usize {
        let Some(doc) = self.session.doc.clone() else {
            return 0;
        };
        let job = {
            let cache = self.tiles.cache.borrow();
            TileJob {
                doc,
                mask: self.solo_mask().cloned(),
                painting: self.painting(),
                ticket: cache.ticket(),
                keys: keys
                    .iter()
                    .map(|&key| (key, cache.tile_rect(key)))
                    .collect(),
            }
        };
        let built = job.build();
        let mut cache = self.tiles.cache.borrow_mut();
        built
            .into_iter()
            .filter(|(key, data)| cache.install(*key, job.ticket, Arc::clone(data)))
            .count()
    }

    /// The level and level rectangle a `vw`×`vh` viewport presents when the
    /// document is fit to it: the coarsest level still at least as fine as
    /// the screen, as `ImageView::presentLevelForZoom` picks.
    pub(in crate::cxxqt_object) fn fit_view(&self, vw: u32, vh: u32) -> (i32, PsdRect) {
        let (w, h) = self.tiles.cache.borrow().size();
        let zoom = (f64::from(vw) / f64::from(w.max(1))).min(f64::from(vh) / f64::from(h.max(1)));
        let mut level = 0;
        while level + 1 < self.display_level_count() && 0.5f64.powi(level + 1) >= zoom {
            level += 1;
        }
        let (lw, lh) = self.display_level_size(level).unwrap_or((0, 0));
        (
            level,
            PsdRect {
                top: 0,
                left: 0,
                bottom: lh as i32,
                right: lw as i32,
            },
        )
    }
}

#[cfg(test)]
#[path = "tiles_tests.rs"]
mod tests;
