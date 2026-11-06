# Design

## Context

A fractured `Plane` holds one base allocation and a list of 64-row bands. Each
band is either a range of the base (`Shared`) or a forked copy (`Owned`).
Whole-plane slice access goes through `as_slice()`, `as_mut_slice()`, `Deref`,
`DerefMut`, `AsRef<[T]>` or `IntoIterator for &Plane`. On a plane with an
`Owned` band it has to concatenate the bands into a new allocation, and it
bumps `MATERIALIZATIONS`. The tiled plane (#1213) will have no base at all, so
every whole-plane caller on a document plane has to move to row, rectangle,
range or band access first.

## API (`crates/pictura-core/src/plane/access.rs`)

The API lives in a child module of `plane.rs`, so it can reach the private
band representation without making `plane.rs` (already over 1000 lines)
longer.

| Method | Contract |
|---|---|
| `rows(ys, width)` | `ys.map_while(row)`: borrowed rows; stops at the first row past the end |
| `rows_mut(ys, width)` | Mutable rows; a banded plane forks only the bands under `ys` (a uniquely held base is written in place); a flat plane forks as `as_mut_slice` always has |
| `read_rect_into(xs, ys, width, out)` | Copies the in-plane part of the rect; `out` has stride `xs.len()`; the outside part is untouched |
| `write_rect_from(xs, ys, width, src)` | The inverse; outside samples are skipped |
| `read_into(start, out)` | A logical range, band by band; for `PixelBuffer`s that stack channels in one plane |
| `for_each_band(f)` / `for_each_band_mut(f)` | `f(first_row, band)`; the mutable form forks each shared band |
| `assert_no_materialization(f)` | Panics when `f` bumped the counter; exact only alone in a process (nextest) |

`width` follows the existing `row(y, width)`: a banded plane uses the width it
was fractured at, and a flat plane uses the argument.

The private helper `band_slices_mut(range)` calls `write_band` on each band in
the range, then splits a unique base into disjoint band slices. It does not use
`write_bands`, because that method flattens a plane with a `Shared` band
through `as_mut_slice`, which materializes once another band has diverged.

## Migration rules

- **Per-pixel reads** move to `sample(i)` when the access is sparse (mask
  gates, clipboard), and to a per-row `row()` when a loop walks a row
  (layer-effect matte, Move's lift and merge).
- **Row copies** move to `row()`/`row_mut()`/`rows()` (`grow_to`,
  `trim_layer`, `clear_layer`, core `copy_rows`, the transform's padded source
  copy).
- **Whole-buffer copies** of a stacked `PixelBuffer` move to `read_into`
  (`rgba_frame`, `with_merged_alpha`).
- **Free Transform** already copies its source into a padded scratch buffer.
  `resample_plane` now takes `&Plane<u8>` and fills that scratch from `row()`,
  so no extra scratch is needed. The scratch is the layer's own rect plus
  2 px, not the canvas.
- **Planes that are never fractured** stay on slices: filter and adjust
  buffers, metadata blobs, `Samples`, `RawChannel`, and fresh `Vec`→`Plane`
  outputs. They cannot materialize. Under the tiled plane they become plain
  `Vec`s or keep a contiguous form.

## Inventory

Only document-owned pixel planes are fractured: `PixelBuffer.data` (the
composite), `Channel.data` (layer and document channels), `LayerMask.data`,
and their clones (a `Selection` loaded from a channel, the borrowed
`picture()`, the level-0 frame). The counts below are of non-test sites that
can reach such a plane:
- render ~83
- app ~39
- codec ~26
- paint ~17
- select ~13
- core 3

Never-banded sites (filters ~214, adjust ~44, ops ~20 and the metadata blobs)
are excluded.

### Migrated in this change

| Crate | Site | Path | Access → API |
|---|---|---|---|
| core | `layer_bounds.rs` `grow_layer_to`/`pad_u8`/`copy_rows` | paint/move grow (#1208) | row copy → `rows` |
| core | `layer_bounds.rs` `shrink_layer_to` | trim | row copy → `read_rect_into` |
| render | `mask_refine.rs` `plain_mask_at` | compositing (mask gate) | per pixel → `sample` |
| render | `mask_refine.rs` `feather_raster` | compositing (feathered mask) | row slice → `row` |
| render | `composite.rs` `mask_value` | compositing | per pixel → `sample` |
| render | `composite_route.rs` content probe | compositing | `channel().is_none()` → `channel_plane` |
| render | `layer_effects/planes.rs` alpha matte | compositing (effects) | per pixel → `row` |
| render | `layer_source/mod.rs` `with_merged_alpha` | smart-object source | whole → `read_into` |
| render | `document_composite.rs` `rgba_frame` | layerless composite | whole → `read_into` |
| render | `layer_ops/neutral.rs` `layer_is_inert` | Move (neutral check) | `iter()` → `bands().flatten()` |
| render | `layer_ops/clipboard.rs` `copy_layer`, `copy_merged` | copy | per pixel → `sample` |
| render | `layer_ops/clipboard.rs` `clear_layer` | cut / clear | row slice → `row_mut` |
| render | `layer_ops/move_content.rs` `lifted_copy` | Move selection (M9 family) | per pixel → `row` |
| render | `layer_ops/move_content.rs` `composite_over` | Move merge-back | per pixel r/w → `row`/`row_mut` |
| render | `layer_ops/move_content.rs` `grow_to`, `trim_layer` | Move/Transform trim (M9, M10) | row copy → `row` |
| render | `layer_ops/transform.rs` `resample_plane` | Free Transform (M10) | row copy → `row` |
| render | `layer_ops/via.rs` `copy_source`, `clear_source` | Layer via Copy/Cut, Move selection cut | per pixel → `sample` / `row_mut` |
| app | `history/planes.rs` `copy_plane` (via `detached_copy`) | snapshot (M8), level 0 | whole → parallel band copy into `Plane::build` |

### Deferred to later changes (one per crate, same capability)

| Crate | Site | Path | Plan |
|---|---|---|---|
| paint | `stroke.rs` `SavedTiles::restore` | brush (M6) | `row_mut` |
| paint | `stroke.rs` `read_idx`/`read_planes` | brush seed, GPU stroke | `sample` / `rows` |
| paint | `stroke/grow.rs` `grow` | brush grow | `row` |
| paint | `fill.rs` `fill_eight_bit`, `fill_native.rs` `fill_at` | Edit › Fill, bucket (M3–M5) | `fill_bands` / `rows` |
| paint | `eraser.rs` `magic_erase`, `healing/layer.rs` | eraser, healing | `sample` + `row_mut` |
| select | `lib.rs` `rgb_at` (magic wand, grow, similar on the composite) | selection | row cache |
| select | `selection_from_layer_alpha`, `selection_from_mask`, `translate.rs`, `contour.rs` | selection | `row` |
| select | `refine.rs` `smart_band_radius`/`estimate_coverage`/`decontaminate` | Refine Edge | `rows` |
| app | `helpers_composite.rs` `planes_of`, `buffer_to_rgba_bytes`, `display_rgba_bytes` | viewport pyramid (M2–M10 refresh, M13) | row source for `ViewPyramid` |
| app | `impl_transform/session.rs` `lift_coverage`; `align/imp.rs` | Transform / align | `plane_coverage_bounds` |
| app | `paint_tools/imp.rs` `white_preview`; `impl_paint/mask_gpu.rs` `mask_rgba_seed`; `fills/imp.rs` | paint previews | `rows` |
| app | `impl_selection*.rs`, `selection_move.rs`, `refine/imp.rs`, `color_range/imp.rs` | selection UI | `rows` / `row_mut` |
| codec | `write.rs` `write_container`, `write_layers.rs` `layer_plans` / mask | PSD write (M11) | `native_plane` / `encode_scanlines` take a row source |
| codec | `write_cmyk.rs`, `write_indexed.rs`, `write_duotone.rs`, `write_bitmap.rs`, `color_mode.rs`, `icc_convert.rs`, `pdf/*` | non-RGB and PDF writes | row source, or `ponytail:` whole-document |
| render | `filter.rs` `apply_op_region`/`channel_mut`/`coverage`; `apply_image.rs` | Filter and Apply Image on a layer | neighbourhood kernels need a padded rect; `read_rect_into` scratch |
| render | `warp.rs`, `perspective_crop.rs`, `smart_object.rs` resamples | Warp, perspective crop, smart-object scale | `read_rect_into` scratch, as for Free Transform |
| render | `resize.rs`, `crop.rs`, `canvas.rs`, `orient.rs`, `rotate_in.rs`, `mode/*`, `depth.rs`, `pictura_raw.rs`, `matting.rs`, `layer_masks.rs`, `mask_edit.rs`, `create.rs`, `auto_blend/sources.rs`, `text_render.rs` | whole-document ops | `ponytail:` (a whole-document op that rewrites every plane; not on an M2–M10 path) |

## Scenario guard

`tests_profiles/memory.rs::measure` now also returns the materialization delta
across the measured step, and `guard` prints it. `STREAMED` lists the
scenarios that must report 0, which is all of M2–M10. M1, M11 and M12 build or
write whole documents and are not in R3's scope. M13 has no Rust guard
(Qt-side view only), so its row in the test plan is manual.

Measured at 4250² on this change against master (65d3e023):
- **M2–M10:** every scenario reports 0 materializations, and its heap peak is
  byte-identical to master's. Their measured steps did not materialize on
  master either, so this change makes that a guarantee rather than lowering
  the peaks.
- **M8 (snapshot):** it did start materializing in its measured step once its
  setup stopped materializing. `copy_plane` now copies band by band, so M8 is
  back to 0 and to master's 327.4 MiB.
- **M11 (save_psd):** the peak rises from 207.7 to 281.7 MiB, within its
  guard's limit of 305 MiB. On master, an `as_slice` read during setup
  materialized the composite, and the next band write adopted that copy as
  the base, so the plane was no longer diverged by the time the PSD was
  written. Now setup reads in place, the plane stays diverged, and the codec's
  `as_slice` builds the copy inside the measured step. The codec change
  removes it.

## Risks

- `sample(i)` costs a division per call on a banded plane. On the composite
  mask gate this replaces an indexed read of a cached mirror. The time budget
  (≤ 5 % per scenario) is checked with
  `profile-tools.py --scenarios memory --compare`.
- On a malformed plane whose length is not `width × height`, `row()` returns
  `None` for a partial final row where the old slice code read the partial
  row. Documents the engine builds are always whole rows.
