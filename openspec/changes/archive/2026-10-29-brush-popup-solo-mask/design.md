# Design: brush-popup-solo-mask

## Measuring

A temporary Qt probe reproduced the reporter's flow on a 4000×3000 document with
a mask on the active layer (offscreen, RelWithDebInfo): solo on, paint a stroke,
right-click to open the picker, hover. `PICTURA_PAINT_TIMING=1` and a
`presentCacheRebuildCount` counter attribute the cost.

| path | cost |
| --- | ---: |
| popup open (`BrushPresetPicker::popUpAt` → `show`) | 1–8 ms, emits no `changed` |
| canvas hover repaint (`cxx_paintEvent`) | 0.19 ms, no present-crop rebuild, no revision bump |
| debounced `refreshPanels` after an edit or solo toggle | 168–178 ms |
| `channels_active_mask_thumbnail(24)` (the bulk of it) | 165–172 ms |

Hover schedules no `refreshPanels`, so the stall is the deferred refresh after
each edit, not the hover path itself. The `presentCache` never rebuilds and
`canvas_revision` never moves across a hover, so the solo present path is
already correct.

## Why it was slow

`ChannelsPanel::refreshThumbnails` calls `channels_active_mask_thumbnail` on
every panel refresh. The Rust side, `mask_channel_thumbnail`, built a full
`width * height` plane and then downsampled it:

    plane[y * width + x] = mask_sample(mask, x, y);   // O(document)
    ...
    out.push(plane[sy * width + sx]);                 // nearest-neighbour

A 12 M-pixel scan produced 432 thumbnail pixels. Because the Channels panel
lists the mask row whenever the active layer carries a mask — the reporter's
"painting on a mask" case — this ran on every edit.

## Decision

**D1 — Sample the grid directly.** `mask_channel_thumbnail` keeps the same
`mask_sample` semantics (off-rect `default_color`, data-less white) and the same
output grid; the extracted `mask_thumbnail_samples` calls `mask_sample` once per
output pixel, so the cost is O(thumbnail). The removed full-plane step's only
reader was the same nearest-neighbour sample, so the output is bit-identical
(`mask_thumbnail_samples_match_the_full_plane_downsample`).

**D2 — Assert the bound without a timing gate.** The repository keeps perf
measurements in `#[ignore]`d profiling tests and forbids timing gates in the
normal suite. A `#[cfg(test)]` thread-local counter in `mask_sample` lets
`mask_thumbnail_samples_touch_only_the_grid` assert the exact sample count
(one per thumbnail pixel), so a rewrite back to a document scan fails
deterministically.

## Ceilings and follow-ups

- `mask_display_plane` (the solo canvas plane) is still O(document), but it is
  built only when solo is entered and on a genuine full recomposite, not per
  edit. A stroke patches it by region (`sync_mask_solo_region`).
- #665 (GPU stroke for layer-mask painting) remains the separate, larger
  mask-paint performance item.
