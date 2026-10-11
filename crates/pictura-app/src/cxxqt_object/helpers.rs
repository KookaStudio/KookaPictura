use cxx_qt_lib::{QString, QStringList};
use pictura_core::{
    AdjustmentData, BlendMode, ColorLabel, Document, Layer, LayerMask, LockFlags, PsdRect,
};
use pictura_paint::{PaintMode, Rgba};
use pictura_select::CombineMode;
/// Build an adjustment layer for `kind`, or `None` for an unknown kind.
///
/// Most defaults are chosen so a freshly added layer visibly changes the
/// composite; a few kinds (e.g. Color Balance) use the reference's neutral default.
/// Editing parameters is out of scope for M4-C. `mask` confines the effect to a
/// selection when one is active.
pub(super) fn adjustment_layer(kind: &str, mask: Option<LayerMask>) -> Option<Layer> {
    use pictura_render::{
        encode_brightness_contrast, encode_channel_mixer, encode_color_balance,
        encode_color_lookup, encode_gradient_map, encode_hue_saturation, encode_invert,
        encode_photo_filter, encode_posterize, encode_selective_color, encode_threshold,
        identity_cube, GradientMapParams, GradientStop, SelectiveColorMethod, SelectiveRange,
    };

    let (name, data): (&str, AdjustmentData) = match kind {
        "invert" => ("Invert", encode_invert()),
        "posterize" => ("Posterize", encode_posterize(4)),
        "threshold" => ("Threshold", encode_threshold(128)),
        "brightness-contrast" => ("Brightness/Contrast", encode_brightness_contrast(20, 0)),
        "hue-saturation" => ("Hue/Saturation", encode_hue_saturation(30, 0, 0)),
        "photo-filter" => (
            "Photo Filter",
            encode_photo_filter([255, 180, 80], 25.0, true),
        ),
        "gradient-map" => (
            "Gradient Map",
            encode_gradient_map(
                &GradientMapParams {
                    stops: vec![
                        GradientStop {
                            location: 0,
                            color: [0, 0, 0],
                        },
                        GradientStop {
                            location: 4096,
                            color: [255, 255, 255],
                        },
                    ],
                    reverse: false,
                    transparency: Vec::new(),
                },
                false,
            ),
        ),
        "color-balance" => (
            "Color Balance",
            encode_color_balance([0.0; 3], [0.0; 3], [0.0; 3], true),
        ),
        "channel-mixer" => (
            "Channel Mixer",
            encode_channel_mixer(
                false,
                [100.0, 0.0, 0.0],
                [0.0, 100.0, 0.0],
                [0.0, 0.0, 100.0],
                [0.0; 3],
            ),
        ),
        "selective-color" => (
            "Selective Color",
            encode_selective_color(
                SelectiveColorMethod::Relative,
                &[SelectiveRange::default(); 9],
            ),
        ),
        "color-lookup" => (
            "Color Lookup",
            encode_color_lookup(&identity_cube(), "Identity.CUBE"),
        ),
        "levels" => (
            "Levels",
            pictura_render::default_adjustment_block("levels", [0, 0, 0], [255, 255, 255])?,
        ),
        "curves" => (
            "Curves",
            pictura_render::default_adjustment_block("curves", [0, 0, 0], [255, 255, 255])?,
        ),
        "exposure" => (
            "Exposure",
            pictura_render::default_adjustment_block("exposure", [0, 0, 0], [255, 255, 255])?,
        ),
        "vibrance" => (
            "Vibrance",
            pictura_render::default_adjustment_block("vibrance", [0, 0, 0], [255, 255, 255])?,
        ),
        "black-white" => (
            "Black & White",
            pictura_render::default_adjustment_block("black-white", [0, 0, 0], [255, 255, 255])?,
        ),
        _ => return None,
    };

    Some(Layer {
        name: name.into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask,
        adjustment: Some(data),
        channels: Vec::new(),
        children: Vec::new(),
        is_group: false,
        background: false,
        ..Default::default()
    })
}
/// Delegates to the shared M36 Background heuristic in `pictura-render`, so the
/// panel core and the bridge agree on one definition.
pub(super) fn is_background_layer(doc: &Document, i: i32) -> bool {
    pictura_render::is_background(doc, &i.to_string())
}
/// Whether `layer` is a placed smart object: a linked/external (`liFE`) or
/// alias (`liFA`) source, the rows the panel underlines.
pub(super) fn is_placed_smart_object(layer: &Layer) -> bool {
    matches!(
        layer.smart_object.as_ref().map(|object| object.kind),
        Some(pictura_core::SmartObjectKind::External) | Some(pictura_core::SmartObjectKind::Alias)
    )
}
/// Map a lock-strip flag name to its [`LockFlags`] bit. `"all"` is the derived
/// four-bit set; anything else is `None`.
pub(super) fn lock_bit(flag: &str) -> Option<u8> {
    match flag {
        "transparency" => Some(LockFlags::TRANSPARENCY),
        "pixels" => Some(LockFlags::PIXELS),
        "position" => Some(LockFlags::POSITION),
        "nesting" => Some(LockFlags::NESTING),
        "all" => Some(LockFlags::all().bits()),
        _ => None,
    }
}
/// The shared row-kind label, so [`layer_kind`] and the tree projection agree.
pub(super) fn layer_kind_str(doc: &Document, path: &str, layer: &Layer) -> QString {
    if layer.is_group {
        QString::from("group")
    } else if layer.adjustment.is_some() {
        QString::from("adjustment")
    } else if pictura_render::is_background(doc, path) {
        QString::from("background")
    } else if layer.is_type() {
        QString::from("type")
    } else {
        QString::from("pixel")
    }
}
/// Owned copy of a `QStringList`'s entries.
pub(super) fn list_of_strings(paths: &QStringList) -> Vec<String> {
    paths.iter().map(|path| path.to_string()).collect()
}
/// Borrow a `Vec<String>` as `&[&str]` for the path core.
pub(super) fn as_str_slice(owned: &[String]) -> Vec<&str> {
    owned.iter().map(String::as_str).collect()
}
/// Map a filter `kind` to its [`pictura_filters::Filter`] with the documented default parameters, or `None` for an unknown kind.
pub(super) fn filter_from_kind(kind: &str) -> Option<pictura_filters::Filter> {
    super::filter_map::filter_from_kind_params(kind, &[])
}
/// Map an image-size resample `kind` to [`pictura_render::Resample`], or
/// `None` unknown.
pub(super) fn parse_resample(kind: &str) -> Option<pictura_render::Resample> {
    use pictura_render::Resample;

    Some(match kind {
        "nearest" => Resample::Nearest,
        "bilinear" => Resample::Bilinear,
        "bicubic" => Resample::Bicubic,
        _ => return None,
    })
}
/// Map a canvas-size `anchor` to [`pictura_render::Anchor`], or `None` unknown.
pub(super) fn parse_anchor(anchor: &str) -> Option<pictura_render::Anchor> {
    use pictura_render::Anchor;

    Some(match anchor {
        "top-left" => Anchor::TopLeft,
        "top-center" => Anchor::TopCenter,
        "top-right" => Anchor::TopRight,
        "center-left" => Anchor::MiddleLeft,
        "center" => Anchor::Center,
        "center-right" => Anchor::MiddleRight,
        "bottom-left" => Anchor::BottomLeft,
        "bottom-center" => Anchor::BottomCenter,
        "bottom-right" => Anchor::BottomRight,
        _ => return None,
    })
}
/// Map a selection `mode` string to [`CombineMode`]; unknown means `New`.
pub(super) fn combine_mode_from(mode: &str) -> CombineMode {
    match mode {
        "add" => CombineMode::Add,
        "subtract" => CombineMode::Subtract,
        "intersect" => CombineMode::Intersect,
        _ => CombineMode::New,
    }
}
/// Unpack a `0xAARRGGBB` colour.
pub(super) fn rgba_from_argb(argb: u32) -> Rgba {
    Rgba {
        r: (argb >> 16) as u8,
        g: (argb >> 8) as u8,
        b: argb as u8,
        a: (argb >> 24) as u8,
    }
}
/// Map a paint-mode string to [`PaintMode`]; unknown means `Normal`.
pub(super) fn paint_mode_from(mode: &str) -> PaintMode {
    match mode {
        "dissolve" => PaintMode::Dissolve,
        "behind" => PaintMode::Behind,
        "clear" => PaintMode::Clear,
        _ => PaintMode::Normal,
    }
}
/// The active layer's path when its raster mask is the mask edit target, so the
/// brush, fill, and filters write coverage instead of the layer's pixels.
pub(super) fn active_mask_target(rust: &super::state::PictureViewRust) -> Option<String> {
    rust.mask_edit_target
        .clone()
        .filter(|path| rust.active_layer.as_deref() == Some(path.as_str()))
}
/// The Rec.601 luma a mask paints for a colour; Adobe's mask conversion is
/// closed, so this is the approximation `pictura-adjust` uses.
pub(super) fn luma_u8(color: Rgba) -> u8 {
    (0.299 * color.r as f32 + 0.587 * color.g as f32 + 0.114 * color.b as f32)
        .round()
        .clamp(0.0, 255.0) as u8
}
/// Index of the topmost pixel layer: the last layer (bottom-first order) that is
/// neither a group nor an adjustment.
pub(super) fn topmost_pixel_layer_index(doc: &Document) -> Option<usize> {
    doc.layers
        .iter()
        .rposition(|l| l.adjustment.is_none() && !l.is_group)
}
/// The single active layer a tool edit may target, or `None` when no layer is
/// active or the path does not name a raster leaf layer.
///
/// `active` is the panel path of the selected layer (`"0/1"` = child 1 of node
/// 0); the panel pushes `None` (an empty path) for a zero- or multi-layer
/// selection, so neither can edit. A group or an adjustment resolves to `None`,
/// while a nested path resolves to its exact leaf.
pub(super) fn active_pixel_layer<'a>(doc: &'a Document, active: Option<&str>) -> Option<&'a Layer> {
    let layer = pictura_render::resolve_path(doc, active?)?;
    (!layer.is_group && layer.adjustment.is_none()).then_some(layer)
}
/// Mutable [`active_pixel_layer`].
pub(super) fn active_pixel_layer_mut<'a>(
    doc: &'a mut Document,
    active: Option<&str>,
) -> Option<&'a mut Layer> {
    let layer = pictura_render::resolve_path_mut(doc, active?)?;
    (!layer.is_group && layer.adjustment.is_none()).then_some(layer)
}
/// Whether the single active layer a tool edit may target is visible.
///
/// A path that does not resolve to one editable pixel layer (none, a group, an
/// adjustment, a nested/unknown path) is reported visible: paint and filters
/// already refuse it through [`active_pixel_layer`], and the cursor must not
/// show the invisible-layer refusal for it.
pub(super) fn active_layer_visible(doc: &Document, active: Option<&str>) -> bool {
    active_pixel_layer(doc, active).is_none_or(|layer| layer.visible)
}
/// Whether the layer (or a descendant) carries a layer-effects block.
///
/// An effect (drop shadow, glow, stroke, …) spills outside the layer's `rect`,
/// so a bounded region refresh cannot repair it. Mirrors the region compositor's
/// own conservative check (`pictura_render::composite`): a disabled effect still
/// counts.
pub(super) fn layer_has_effects(layer: &Layer) -> bool {
    layer.extra_block(b"lfx2").is_some()
        || layer.extra_block(b"lrFX").is_some()
        || layer.children.iter().any(layer_has_effects)
}

/// A rectangle that provably bounds a visibility toggle's effect, or `None` when
/// the toggle can change a pixel outside any such rectangle.
///
/// A pixel layer's channels cover its `rect`, so hiding it changes the composite
/// only there. An adjustment layer transforms the whole backdrop, but a mask
/// that is enabled, carries data, and has a zero `default_color` confines it to
/// its mask `rect` (outside, `mask_alpha` returns the default 0). Everything
/// else — groups, effect-bearing layers (whose effects spill past the rect),
/// unmasked/disabled/data-less/non-zero-default adjustments — is unbounded and
/// returns `None`. `refresh_region` clamps the returned rect.
pub(super) fn layer_visibility_region(layer: &Layer) -> Option<PsdRect> {
    if layer.is_group || layer_has_effects(layer) {
        return None;
    }
    if layer.adjustment.is_some() {
        let mask = layer.mask.as_ref()?;
        if mask.disabled || mask.data.is_none() || mask.default_color != 0 {
            return None;
        }
        return Some(mask.rect);
    }
    // A type layer without a rasterized proxy renders its text inside its
    // rect, so it reaches no further than a pixel layer does.
    if layer.type_tool.is_none()
        && !layer
            .channels
            .iter()
            .any(|c| c.id >= 0 && !c.data.is_empty())
    {
        return None;
    }
    match &layer.mask {
        None => Some(layer.rect),
        Some(mask) if mask.disabled || mask.default_color == 0 => Some(layer.rect),
        Some(_) => None,
    }
}
/// The bounding box of two document-space rects.
pub(super) fn union_rect(a: PsdRect, b: PsdRect) -> PsdRect {
    PsdRect {
        top: a.top.min(b.top),
        left: a.left.min(b.left),
        bottom: a.bottom.max(b.bottom),
        right: a.right.max(b.right),
    }
}

/// A stroke's dirty area as a document-space bitmap of [`pictura_render::TILE`]
/// cells. The commit decomposes it into disjoint rectangles so a diagonal stroke
/// composites its tiles, not its whole bounding box.
#[derive(Default)]
pub(super) struct TileSet {
    cols: usize,
    rows: usize,
    set: Vec<bool>,
}

impl TileSet {
    pub(super) fn reset(&mut self, width: u32, height: u32) {
        let tile = pictura_render::TILE as usize;
        self.cols = (width as usize).div_ceil(tile);
        self.rows = (height as usize).div_ceil(tile);
        self.set.clear();
        self.set.resize(self.cols * self.rows, false);
    }

    pub(super) fn mark(&mut self, rect: PsdRect) {
        if self.cols == 0 || self.rows == 0 || rect.right <= rect.left || rect.bottom <= rect.top {
            return;
        }
        let tile = pictura_render::TILE as usize;
        let x0 = rect.left.max(0) as usize / tile;
        let y0 = rect.top.max(0) as usize / tile;
        let x1 = (rect.right.max(0) as usize).div_ceil(tile).min(self.cols);
        let y1 = (rect.bottom.max(0) as usize).div_ceil(tile).min(self.rows);
        for y in y0..y1 {
            for x in x0..x1 {
                self.set[y * self.cols + x] = true;
            }
        }
    }

    /// The dirty tiles as disjoint rectangles, or `[union]` when the area is not
    /// worth decomposing: one rectangle, more than the cap, or the rectangles
    /// filling most of their bounding box.
    pub(super) fn regions(self, union: PsdRect) -> Vec<PsdRect> {
        if self.cols == 0 || self.rows == 0 {
            return vec![union];
        }
        let tile = pictura_render::TILE;
        let mut rects: Vec<PsdRect> = Vec::new();
        // Greedy vertical merge: bands of an identical tile run extend downward.
        let mut active: Vec<(usize, usize, usize)> = Vec::new();
        for y in 0..self.rows {
            let mut next: Vec<(usize, usize, usize)> = Vec::new();
            for (rx0, rx1) in row_runs(&self.set, y * self.cols, self.cols) {
                match active
                    .iter()
                    .position(|&(ax0, ax1, _)| ax0 == rx0 && ax1 == rx1)
                {
                    Some(pos) => next.push(active.remove(pos)),
                    None => next.push((rx0, rx1, y)),
                }
            }
            for &(x0, x1, sy) in &active {
                rects.push(tile_rect(x0, x1, sy, y, tile));
            }
            active = next;
        }
        for &(x0, x1, sy) in &active {
            rects.push(tile_rect(x0, x1, sy, self.rows, tile));
        }
        collapse(rects, union)
    }
}

const REGION_CAP: usize = 64;

fn collapse(rects: Vec<PsdRect>, union: PsdRect) -> Vec<PsdRect> {
    if rects.is_empty() {
        return vec![union];
    }
    let area = |r: &PsdRect| (r.width().max(0) as i64) * (r.height().max(0) as i64);
    let sum: i64 = rects.iter().map(area).sum();
    let union_area = area(&union);
    if rects.len() > REGION_CAP || union_area <= 0 || sum * 4 >= union_area * 3 {
        return vec![union];
    }
    rects
}

fn row_runs(set: &[bool], base: usize, cols: usize) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut x = 0;
    while x < cols {
        if !set[base + x] {
            x += 1;
            continue;
        }
        let start = x;
        while x < cols && set[base + x] {
            x += 1;
        }
        runs.push((start, x));
    }
    runs
}

fn tile_rect(x0: usize, x1: usize, y0: usize, y1: usize, tile: i32) -> PsdRect {
    PsdRect {
        top: y0 as i32 * tile,
        left: x0 as i32 * tile,
        bottom: y1 as i32 * tile,
        right: x1 as i32 * tile,
    }
}
/// Apply `set_visible_paths` and return the changed count plus the union of the
/// changed layers' bounded [`layer_visibility_region`]s.
///
/// The second value is `None` when any changed layer is unbounded (a group, or
/// a channel-less/unmasked adjustment), which forces the caller's full
/// `recomposite` fallback. A union of `None` (nothing changed) is distinct: the
/// caller sees `changed == 0` and records nothing.
pub(super) fn set_visible_paths_union(
    doc: &mut Document,
    paths: &[&str],
    visible: bool,
) -> (usize, Option<PsdRect>) {
    let mut union: Option<PsdRect> = None;
    let mut unbounded = false;
    for path in paths {
        if let Some(layer) = pictura_render::resolve_path(doc, path) {
            if layer.visible != visible {
                match layer_visibility_region(layer) {
                    Some(rect) => union = Some(union.map_or(rect, |u| union_rect(u, rect))),
                    None => unbounded = true,
                }
            }
        }
    }
    let changed = pictura_render::set_visible_paths(doc, paths, visible);
    (changed, if unbounded { None } else { union })
}
/// Clamp `rect` to `width`×`height`.
///
/// Returns `(x0, y0, w, h)` in document pixels, or `None` when the intersection
/// is empty (zero or negative area).
pub(super) fn clamp_region(rect: PsdRect, width: u32, height: u32) -> Option<(i32, i32, u32, u32)> {
    let x0 = rect.left.max(0);
    let y0 = rect.top.max(0);
    let x1 = rect.right.min(width as i32);
    let y1 = rect.bottom.min(height as i32);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some((x0, y0, (x1 - x0) as u32, (y1 - y0) as u32))
}
/// The region to patch for the move-preview base, or `None` to fall back to a
/// full composite.
///
/// The authoritative composite must match the document (`cached_ok`); an empty
/// clamped rect also falls back. There is no area budget: the region is blitted
/// in C++ regardless of size.
pub(super) fn move_preview_region(
    rect: PsdRect,
    width: u32,
    height: u32,
    cached_ok: bool,
) -> Option<(i32, i32, u32, u32)> {
    if !cached_ok {
        return None;
    }
    clamp_region(rect, width, height)
}

/// Probe `bytes`, decode them with Qt to packed RGBA8888, and enforce the
/// budget against the actual decoded allocation.
///
/// Returns `(rgba, width, height)`, or `None` when a recognized header is over
/// budget, Qt cannot decode the bytes, or the decoded allocation is over budget.
///
/// Qt's runtime decoders are the authoritative allow-list: the probe is only a
/// fast-path guard that pre-empts a recognized header, so a container the probe
/// does not recognize is still handed to Qt and capped afterwards.
pub(super) fn decode_import(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let budget = pictura_codec::ImageBudget::default();
    match pictura_codec::probe_image(bytes, budget) {
        Ok(_) | Err(pictura_codec::ImportError::UnknownContainer { .. }) => {}
        Err(_) => return None,
    }
    let mut width = 0i32;
    let mut height = 0i32;
    let rgba = super::qobject::decode_image_rgba(bytes, &mut width, &mut height);
    if width <= 0 || height <= 0 {
        return None;
    }
    let (w, h) = (width as u32, height as u32);
    if w > budget.max_dimension || h > budget.max_dimension {
        return None;
    }
    if (w as u64) * (h as u64) * 4 > budget.max_alloc_bytes {
        return None;
    }
    if rgba.len() != (w as usize) * (h as usize) * 4 {
        return None;
    }
    Some((rgba, w, h))
}

/// Brush size/hardness step for a `[`/`]` key event, or `0` when the key is not
/// a brush bracket or `paint_tool` is false.
///
/// `key` is the Qt key value (`[` = 0x5B, `]` = 0x5D, `{` = 0x7B, `}` = 0x7D);
/// `native_scan_code` is the Linux evdev code (`[` = 34, `]` = 35), so the
/// binding follows the physical key on EU/Scandinavian layouts. A magnitude of
/// `1` is a diameter step and `5` a hardness step; the caller picks the target.
pub(super) fn brush_shortcut_delta(
    key: i32,
    native_scan_code: u32,
    shift: bool,
    paint_tool: bool,
) -> i32 {
    const KEY_BRACKET_LEFT: i32 = 0x5B;
    const KEY_BRACKET_RIGHT: i32 = 0x5D;
    const KEY_BRACE_LEFT: i32 = 0x7B;
    const KEY_BRACE_RIGHT: i32 = 0x7D;
    const EVDEV_LEFT: u32 = 34;
    const EVDEV_RIGHT: u32 = 35;
    if !paint_tool {
        return 0;
    }
    let left = key == KEY_BRACKET_LEFT || key == KEY_BRACE_LEFT || native_scan_code == EVDEV_LEFT;
    let right =
        key == KEY_BRACKET_RIGHT || key == KEY_BRACE_RIGHT || native_scan_code == EVDEV_RIGHT;
    match (shift, left, right) {
        (false, true, _) => -1,
        (false, _, true) => 1,
        (true, true, _) => -5,
        (true, _, true) => 5,
        _ => 0,
    }
}

/// Opt-in timing for the paint path, using the same `PICTURA_PAINT_TIMING`
/// switch the Qt side reads. A no-op unless the variable is set, so normal runs
/// and tests pay only an `Instant::now()` per phase.
pub(crate) mod paint_timing {
    use std::cell::RefCell;
    use std::sync::OnceLock;
    use std::time::{Duration, Instant};

    pub(crate) fn level() -> u8 {
        static LEVEL: OnceLock<u8> = OnceLock::new();
        *LEVEL.get_or_init(|| {
            std::env::var("PICTURA_PAINT_TIMING")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0)
        })
    }

    #[derive(Default)]
    struct Phase {
        count: u32,
        total: Duration,
        max: Duration,
    }

    #[derive(Default)]
    struct Stats {
        title: String,
        phases: Vec<(&'static str, Phase)>,
    }

    thread_local! {
        static STATS: RefCell<Option<Stats>> = const { RefCell::new(None) };
    }

    /// Open a stroke's timing session. No-op unless timing is enabled.
    pub(crate) fn start(title: &str) {
        if level() == 0 {
            return;
        }
        STATS.with(|s| {
            *s.borrow_mut() = Some(Stats {
                title: title.to_string(),
                phases: Vec::new(),
            });
        });
    }

    /// Accumulate `dt` under `label`. No-op unless timing is enabled.
    pub(crate) fn record(label: &'static str, dt: Duration) {
        if level() == 0 {
            return;
        }
        STATS.with(|s| {
            let mut slot = s.borrow_mut();
            let Some(stats) = slot.as_mut() else {
                return;
            };
            match stats.phases.iter_mut().find(|(l, _)| *l == label) {
                Some((_, p)) => {
                    p.count += 1;
                    p.total += dt;
                    p.max = p.max.max(dt);
                }
                None => stats.phases.push((
                    label,
                    Phase {
                        count: 1,
                        total: dt,
                        max: dt,
                    },
                )),
            }
        });
    }

    /// Print and close the session, if one is open. No-op unless enabled.
    pub(crate) fn report() {
        if level() == 0 {
            return;
        }
        STATS.with(|s| {
            let Some(stats) = s.borrow_mut().take() else {
                return;
            };
            let total: Duration = stats.phases.iter().map(|(_, p)| p.total).sum();
            eprintln!(
                "[paint-timing] ===== {} (sum of phases {:.2}ms) =====",
                stats.title,
                total.as_secs_f64() * 1000.0
            );
            for (label, p) in &stats.phases {
                let avg = if p.count > 0 {
                    p.total.as_secs_f64() * 1000.0 / f64::from(p.count)
                } else {
                    0.0
                };
                eprintln!(
                    "[paint-timing]   {label:<26} n={:<4} total={:>8.2}ms avg={:>7.3}ms max={:>7.3}ms",
                    p.count,
                    p.total.as_secs_f64() * 1000.0,
                    avg,
                    p.max.as_secs_f64() * 1000.0,
                );
            }
        });
    }

    /// A scope timer; records its elapsed time under `label` on drop.
    pub(crate) struct Scope(&'static str, Instant);

    impl Scope {
        pub(crate) fn new(label: &'static str) -> Self {
            Self(label, Instant::now())
        }
    }

    impl Drop for Scope {
        fn drop(&mut self) {
            record(self.0, self.1.elapsed());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::adjustment_layer;
    use super::TileSet;
    use pictura_core::PsdRect;
    use pictura_render::TILE;

    #[test]
    fn adjustment_layer_covers_every_kind_and_rejects_unknown() {
        const KINDS: [&str; 16] = [
            "invert",
            "posterize",
            "threshold",
            "brightness-contrast",
            "levels",
            "curves",
            "exposure",
            "vibrance",
            "hue-saturation",
            "color-balance",
            "black-white",
            "photo-filter",
            "channel-mixer",
            "selective-color",
            "color-lookup",
            "gradient-map",
        ];
        for kind in KINDS {
            let layer = adjustment_layer(kind, None).unwrap_or_else(|| panic!("{kind} should map"));
            assert!(layer.adjustment.is_some(), "{kind} carries adjustment data");
        }
        assert!(adjustment_layer("bogus", None).is_none());
    }

    fn r(top: i32, left: i32, w: i32, h: i32) -> PsdRect {
        PsdRect {
            top,
            left,
            bottom: top + h,
            right: left + w,
        }
    }

    #[test]
    fn a_diagonal_decomposes_into_tiles_not_its_bounding_box() {
        let mut tiles = TileSet::default();
        tiles.reset(8 * TILE as u32, 8 * TILE as u32);
        for i in 0..8 {
            tiles.mark(r(i * TILE, i * TILE, TILE, TILE));
        }
        let regions = tiles.regions(r(0, 0, 8 * TILE, 8 * TILE));
        assert!(regions.len() > 1, "a diagonal is more than one rect");
        let sum: i64 = regions
            .iter()
            .map(|rect| rect.width() as i64 * rect.height() as i64)
            .sum();
        let bbox = (8 * TILE as i64) * (8 * TILE as i64);
        assert!(sum < bbox / 2, "tiles cover less than half the bbox");
    }

    #[test]
    fn a_filled_blob_collapses_to_its_union() {
        let mut tiles = TileSet::default();
        tiles.reset(8 * TILE as u32, 8 * TILE as u32);
        for y in 0..4 {
            for x in 0..4 {
                tiles.mark(r(y * TILE, x * TILE, TILE, TILE));
            }
        }
        let union = r(0, 0, 4 * TILE, 4 * TILE);
        assert_eq!(tiles.regions(union), vec![union]);
    }

    #[test]
    fn an_empty_tile_set_falls_back_to_the_union() {
        let union = r(3, 4, 5, 6);
        assert_eq!(TileSet::default().regions(union), vec![union]);
        let mut tiles = TileSet::default();
        tiles.reset(128, 128);
        assert_eq!(tiles.regions(union), vec![union]);
    }

    #[test]
    fn a_single_tile_collapses_to_a_small_union() {
        let mut tiles = TileSet::default();
        tiles.reset(256, 256);
        let dab = r(70, 130, 8, 8);
        tiles.mark(dab);
        assert_eq!(
            tiles.regions(dab),
            vec![dab],
            "a lone tile composite would cover 64x64 for an 8x8 dab"
        );
    }
}
