# Design

## Context

See `proposal.md` for motivation and the confirmed scope of #1199.

Observed at the current checkout:

- `pictura-core::LayerMask` already contains a signed `PsdRect`, exterior
  `default_color`, and one `Plane<u8>`. It has no alpha plane to remove.
- `document_ops/layer_ops/layer_masks.rs::add_layer_mask` uses the layer rect
  for ordinary masks and sets Reveal All/From Transparency exterior to 255.
- `document_ops/layer_ops/mask_edit.rs` builds mask-sized editing documents;
  write-back assumes the edited dimensions and mask dimensions agree.
- `impl_paint/mask.rs` and `paint_tools/fills/imp.rs::apply_mask` use those
  adapters. `impl_filters.rs` also builds a mask-sized editing view. Thus
  changing only a default color would not enable canvas-wide editing.
- Properties Invert follows a separate path: `properties_panel.cpp` calls
  `impl_layers/mask_refine/imp.rs::layer_mask_invert`, which calls
  `pictura_render::invert_layer_mask`. That function currently inverts every
  stored sample and the exterior default, independently of the pixel selection.
  It must join the canvas-domain mutation path rather than remain a global
  stored-plane operation.
- The current compositor and solo/Channels sampler treat a mask with no
  decoded data as coverage 255 regardless of its stored default. The codec can
  load a nonempty mask header without a `-2` data channel into this state.
  Capturing the stored nonwhite default as visible coverage would change
  unedited canvas pixels at the first local edit.
- Whole-layer translation in `document_ops/crop.rs::offset_layer` offsets
  the attached mask rect without touching its exterior. Selected mask movement
  in `mask_edit.rs` clips destinations to the old mask rect. The bridge in
  `impl_selection.rs` supplies the background swatch's luma as the clear value.
- `helpers_masks.rs::mask_thumbnail_positioned` leaves transparent margins;
  `mask_thumbnail_image` uses stored-mask aspect. `impl_layers.rs` chooses
  between them using the layer-thumbnail contents option. Existing snapshot and
  panel tests assert transparent margins, so their expectations must change.
- `Plane` has contiguous storage with optional copy-on-write bands, not a
  sparse uniform-plane representation. Its `build` entry point can initialize
  one allocation directly. `pictura_core::memory` supplies reservations and
  allocation refusal. The codec reads/writes mask channels using mask bounds,
  independently of layer bounds, which already supports off-canvas rectangles.
- Retained high-depth `source_channels` may include native `-2` samples used
  by the compositor and writer. An 8-bit mask change cannot leave stale native
  data silently overriding the edit.

The structural graph does not cover this worktree; these observations were
verified against the current source rather than inferred from older worktrees.

Existing spec conflicts are explicit: layer-mask creation requires
layer-sized storage and white exterior; adapters require mask-sized views;
selected coverage movement requires unchanged bounds and swatch-colored holes.
The Layers-panel aspect requirement also requires transparent raster-mask
margins and mask-own-bounds images under Layer Bounds; the mask-refinement
requirement requires Properties Invert to invert every sample and the default.
The deltas replace those clauses, preserving the UI requirement's other image,
card, type, fill, folder, and vector behavior. The separate thumbnail-layout
requirement describes cards and fitting previews inside square cards, so it
does not need a change to make raster-mask image samples document-aspect.
Link-flag interpretation belongs to #1200.

## Goals / Non-Goals

**Goals:** separate the document editing window from retained mask storage;
reuse existing paint/fill/filter engines; make mutation publication atomic;
preserve exact samples and existing mask properties during integer movement.

**Non-Goals:** new sparse-plane infrastructure, mask-channel depth migration,
free-transform/resampling redesign, vector/filter masks, redesigning the pixel
selection into an off-canvas selection, or correcting PSD link bits. No claim
of Photoshop parity is added without an independent oracle.

## Decisions

### 1. Keep a finite retained domain and a canvas-sized editing window

Use the current mask model. The editing window is always the document canvas;
the stored rect may be smaller on import, canvas-sized, translated beyond the
canvas, or larger than the canvas. Ordinary canvas edits retain the union of
the old stored domain and the canvas; selected-coverage moves additionally
retain their translated destination. Do not intersect stored coverage with the
canvas and do not trim using a nonzero threshold greater than zero.

```text
retained mask storage, possibly at negative coordinates
+-----------------------------------------------------+
| off-canvas samples | canvas editing window | samples |
+-----------------------------------------------------+
                     paint / fill / filter
                     write only their dirty region
```

Add a small mask-domain helper within `pictura-render` to compute checked
unions/translations, build padded coverage, and stage replacement masks. Keep
the logic mask-specific; do not add a generic image-storage framework. Split
the already large inline mask tests into a sibling test module if necessary.

Rejected: clipping back to a document-sized plane after each edit, because
that destroys the exact samples the user requires to return. Rejected: storing
only a layer-sized plane with a black exterior, because the current adapters
would still forbid edits elsewhere. A sparse backend can follow separately.

### 2. Initialize new masks in document coordinates

Allocate the union of the canvas and any nonempty source rectangle needed to
retain off-canvas source coverage. Initialize the plane directly on black and
write Reveal All's white rectangle or From Transparency's alpha at the source
offset. For selection variants, sample selection coverage/default over the
canvas and retain explicit selection coverage outside it; invert for Hide
Selection. Set exterior default to zero for all new kinds.

Compatibility defaults: retain white Reveal All for groups and empty pixel
layers, since neither has a meaningful content rectangle; an empty layer must
remain useful for later pixel painting. Keep existing From Transparency
behavior for a zero-area target rather than inventing a new alpha conversion.
These fallback cases and missing-alpha behavior get explicit tests. Preserve
existing mask properties and refusal-to-convert-Background behavior.

### 3. Expand imports on mutation, preserving their current canvas coverage

Opening, targeting, showing solo, or taking thumbnails is read-only. At the
first edit/move that needs a finite black-backed domain, build a replacement
according to whether the import has decoded coverage:

- For decoded masks, copy existing samples and fill the captured canvas area
  outside the old rect with the existing default color before setting the new
  exterior to zero. Preserve stored off-canvas samples and unrelated metadata.
- For data-less masks, initialize the captured working domain at 255,
  independently of the old default, to preserve their current rendered and
  solo/Channels coverage. Do not change the renderer's existing missing-data
  interpretation. Keep the original missing-data state, rect, default, native
  data, and metadata in the operation baseline for exact undo/cancel/refusal.

For a feathered imported mask, include the existing feather support halo around
the canvas when capturing its visible coverage (the old default for decoded
exterior pixels, or 255 for a data-less mask). Otherwise changing a white exterior
to black exactly at the canvas edge would introduce a feather edge inside the
visible canvas before the requested edit. Use the existing feather reach rule,
not a new blur-radius constant. Density and disabled state remain metadata;
black here means stored coverage zero, without bypassing their normal effects.

Stage all allocation and copying before swapping into the live document. A
first edit's history state includes normalization; no separate migration state
is recorded. Cancel or refusal restores the original rect, exterior, samples,
and retained native/opaque data. A no-op move does not normalize an import.

### 4. Adapt canvas editing without overwriting retained storage

Make mask editing views document-sized, positioned at `(0, 0)`. Sample the
stored mask into that view by document coordinates, preserving the shared-plane
fast path when the stored rect already equals the canvas. Keep a one-channel
coverage view for paint and shared fills, and route compatible filters through
that view without adding per-dab RGBA intermediates.

Change write-back to map document-space dirty rectangles into the retained
mask plane. Never replace the whole mask with the canvas view. Keep old
off-canvas samples and all canvas samples outside the dirty region unchanged.
For unselected fills/adjustments/filters the editable region is the canvas,
not the union of retained bounds; effects may read their usual boundary support
but must not write into retained off-canvas data.

Properties Invert must explicitly use this same mutation path. Replace the
Properties bridge's direct all-storage inversion with a staged canvas-wide
inversion, normalizing imported coverage first and writing only canvas samples
back. Continue to ignore the active pixel selection, as Properties Invert does
today; Image > Adjustments Invert keeps its existing selection-aware policy.
Keep all off-canvas samples, native off-canvas values, and unrelated metadata;
retain the normalized zero exterior and record one "Invert" state. Synchronize
native data/cache keys through the same region policy as other coverage edits.
Its button test must include an active smaller selection and an off-canvas
sentinel, so it proves both the canvas domain and unchanged selection policy.

Update mask stroke sessions, direct/shared fills, Properties Invert, and filter preview snapshots
to retain enough pre-edit mask state for cancellation, including rect/default
and native data. A small dab remains region-bounded. Damage is mapped to
document coordinates, expanded by the existing refinement/effect reach, and
clipped only for presentation.

### 5. Translate retained storage; clear selected coverage toward black

Whole-mask integer moves offset the retained mask rectangle without resampling
its planes. Normalize a legacy nonblack exterior first, then retain a zero
default. Do not reanchor or crop the stored plane to the canvas after moving.
Reversing offsets therefore recovers all original samples exactly.

Integrate mask-target movement in the Move session/bridge using the existing
link predicate: an unlinked targeted mask moves independently; a linked mask
continues to travel with its owner. Preserve the original pixel-layer rect and
pixels when only the mask moves. Use a mask-aware preview or region composite,
not the pixel-clone preview. Feed the same black-backed sampling into previews,
solo display, committed composites, and thumbnails; commit one normal move
state and cancel with no state. The #1200 flag/serialization fix is not included.

For selected coverage, retain the old rect plus the canvas and destination
bounds. Keep the existing rounded source-selection blending, but clear with
zero rather than a swatch value. Build from an immutable snapshot so overlapping
moves and Alt duplicates do not feed on their own writes. The selection itself
keeps its existing document-sized clipping; off-canvas mask samples are retained
independently and can be brought back through whole-mask translation.

### 6. Preserve precision and encoded geometry

Pure moves keep native `-2` samples unchanged. When domain expansion changes
layout, pad/rebase the native mask plane at its native depth and copy existing
samples without conversion. This preserves positive 16-bit/float samples even
when their 8-bit display proxy is zero. On coverage edits, synchronize only the
edited native region with the existing edit-depth policy; retain untouched
native samples outside it. Invalidate stale derived/encoded mask channels so
neither the native compositor nor writer can prefer an obsolete rectangle.

Keep mask density, feather, disabled state, unrelated flags, and opaque extras.
Use the existing PSD/PSB mask-rectangle/channel encoding; do not invent a new
file format. Round-trip negative/oversized bounds and native samples while the
mask is completely outside the canvas, then translate back after reopen.

### 7. Sample thumbnails over the document

Use one document-coordinate sampling rule for raster Layers mask thumbnails,
and align it with the stored-coverage sampling used by solo/Channels display.
Every output pixel is opaque `[v, v, v, 255]`, including black margins. A data-less
import samples as 255 without materializing or modifying its source state. Ignore
the mask's enabled state for display of its stored raster. The layer-color
thumbnail still follows Entire Document/Layer Bounds; the raster mask thumbnail
always shows the document domain. Square adjustment/fill cards still fit that
document-aspect raster image inside their unchanged square card; existing
outline, bracket, hit-target, group, type, fill, and vector layout rules remain
in force. Sampling is O(thumbnail pixels), not O(canvas
pixels), and does not normalize imported storage.

### 8. Budget and validate before publishing

Compute dimensions, byte counts, offsets, native-plane padding, and destination
unions with checked wide intermediates. Reserve all operation workspace through
the existing governor before allocating, including temporary canvas views and
native padding; hold reservations until the buffers are published or dropped.
Use direct plane construction to avoid an extra full-size Vec-to-Arc copy.

Present the existing memory-error path on refusal. No partial mask mutation,
target switch, selection translation, history record, or modified-state change
is allowed. A nontrivial source default or extreme offset may require a large
union rectangle; refuse safely instead of silently clipping or dropping samples.

## Risks / Trade-offs

- A canvas mask adds about 276 MiB at 17000 squared, before workspace and native
  planes. Mitigation: charge the real peak, share existing planes when possible,
  keep thumbnails output-sized, and test deterministic budget refusal.
- A far-away retained rectangle can make the union large. Mitigation: checked
  arithmetic and governor refusal; sparse storage is a separate change.
- Rebasing native data or changing a feathered default can alter rendering.
  Mitigation: native bit-pattern, refinement-halo, and rendered-coverage tests;
  preserve raw stored data separately from its display/composite result.
- Existing tests assert the superseded behavior. Mitigation: change only the
  expectations for white exterior, transparent mask margins, mask-local adapter
  bounds, and clipped selected destinations. Keep unrelated tests and goldens.
- Shared Move code overlaps #1200. Mitigation: use the existing link interface,
  keep this change's tests about movement/coverage, and reconcile concurrent
  source edits without changing PSD flag semantics here.

## Migration Plan

There is no eager file migration. Existing files open unchanged; only accepted
mask mutations introduce canvas-covering black-backed storage. Save uses the
existing mask record format. Reverting the implementation can still read these
files because their representation already exists in the codec.

Write the planned Rust/Qt tests first, finish all implementation edits, then
compile once using at most 80 percent of CPUs and run the targeted suites.
Archive the completed change with its deltas before the shipping PR. Planning
validation does not imply the as-yet-unwritten tests have passed.
