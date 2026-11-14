# Design

## Context

See `proposal.md` for motivation and the spec deltas for the behaviour contract.

Current state that shapes the approach:

- `pictura-select` rasterises the ellipse analytically and the lasso polygon with a
  scanline fill, but always at the pixel centre, producing binary coverage
  (`Selection::ellipse`, `polygon::fill`).
- `pictura-core::shape` already has an anti-aliased scanline rasteriser:
  `coverage` / `path_coverage` / `coverage_box` sample four sub-rows per pixel and
  accumulate fractional horizontal spans. Shape layers and Paths > Make
  Selection already use it; vector masks sample the separate row table in
  `crates/pictura-core/src/vector.rs:402`.
- The app's selection commit path is split between the cxx-qt bridge
  (`cxxqt_object.rs`, at its 1204-line allowlist ceiling) and
  `impl_selection.rs` (1189 lines), which is close to the 1200 hard cap.
- The Magic Wand flood is tiled and boolean (`TileMatch`, `FloodSource`); the
  image is read tile by tile.
- `pictura-codec::common::MAX_CHANNELS` is a private 56; the app's
  `channels_new_alpha` and `save_selection_channel` append without a budget.
- The options bar already renders the Anti-alias checkbox but disables it in
  two places; `SelectionOptions` has an unused `antiAlias_` defaulting to true.

## Goals / Non-Goals

**Goals:**

- One anti-aliased geometric rasteriser shared by the Elliptical Marquee and the
  three lassos, with no pixel-centre-only path left for those tools.
- An inferred but deterministic Magic Wand boundary ramp that reuses the
  existing tiled flood.
- A single 56-channel budget in the document model, used by the app and the PSD
  codec.
- Keep the bridge and core files inside their file-size allowlists.

**Non-Goals:**

- Exact byte parity with Photoshop's unpublished AA filters.
- Spot-channel kinds, Duplicate Channel, Split/Merge Channels.
- The Paths panel's missing Anti-aliased checkbox; its coverage is already AA.
- Per-tool AA state among the three lassos; they share the existing selection
  options group like Feather and combine mode.

## Decisions

### Polygon and lasso: reuse `pictura_core::shape`

Build a closed `Subpath` from the integer points and call
`shape::coverage_box`. The function rasterises with even-odd winding for one
subpath, four sub-rows per row, and fractional horizontal spans — exactly the
contract the lasso spec needs. Write the returned bbox mask into an empty tiled
selection and settle it.

Why not add another scanline rasteriser to `pictura-select`: the core one is
already the AA source of truth for shape layers and path selections, and its
tests cover clipping and off-canvas polygons. Reusing it keeps one AA behaviour
instead of two subtly different ones. The cost is O(bbox area) for the row
accumulator, which matches the document-sized mask output.

### Ellipse: analytic edge supersampling

Keep the analytic ellipse equation, start from the existing binary mask, and
grade the per-row boundary band: the widest analytic span at the y in the row
closest to `cy` and the narrowest span at the y farthest from `cy` bound the
pixels whose square straddles the ellipse. Pixels inside the narrowest span stay
255, pixels outside the widest stay 0, and the band is sampled 4×4 (17 levels
from 0/16 through 16/16). This grades the wide partial bands on flat ellipses'
top and bottom rows, not only the two mid-row edge pixels. Rows clipped at
the canvas edge keep the clipped value: only the shape boundary is softened.

Why not reuse `shape::coverage` for the ellipse: its ellipse outline is a
four-cubic Bézier approximation whose radial error grows with radius, while the
current analytic equation is exact. Boundary-band sampling is O(perimeter) on
ordinary rows and only widens on the caps, so it avoids that drift. The 4×4
grid is a documented implementation choice and the code carries a `ponytail:`
marker; upgrading to 8×8 is a local change if the 17 levels band visibly.

### Magic Wand: a boundary pass over the binary flood

Keep `flood_tiled` / `match_mask` unchanged. After the binary result, scan the
selected bounds expanded by one pixel. For each unselected pixel with a selected
four-neighbour, compute the Chebyshev distance `d` to the seed colour. Let
`width = max(0.5 * tolerance, 1.5)`; when `tolerance < d < tolerance + width`,
write `round(clamp(1 - (d - tolerance) / width, 0, 1) * 255)`. The
`max(..., 1.5)` floor keeps Tolerance 1 and 2 able to produce a fringe. All
other pixels keep 0 or 255. Tolerance 0 or Anti-alias off skips the pass. The
engine entry point becomes `magic_wand(..., anti_alias)`; every caller and the
dense oracle are updated for the extra argument.

Why not a soft flood: `TileMatch` and `FloodSource` are boolean, and a coverage
flood would ripple through the tiled flood, tile-uniform decisions, and the
image-reading closures. The boundary pass matches the observed Photoshop
behaviour ("only edge pixels change") and the GIMP ramp, and it is deterministic.
The width window is inferred; the spec labels it as such.

### Channel budget: one constant, one predicate

Add `pictura-core/src/channels.rs` with `MAX_CHANNELS: u16 = 56` and an inherent
`Document::can_add_channel()` implementation (a separate module keeps
`pictura-core/src/lib.rs`, already 1187 lines, below the cap). The predicate
counts the larger of the stored composite channel count and the colour-channel
count of `source_mode.unwrap_or(self.mode)`, using the composite count for
Multichannel. This is exact for RGB, Grayscale, CMYK, and Lab, and conservative
for a converted Indexed/Bitmap/Duotone document that may save back at the
working RGB count. In particular a CMYK source converted to the RGB working mode
still counts four colour channels, so the writer's four plus the extras cannot
exceed 56.

The PSD codec imports the core constant instead of keeping a private copy.
`channels_new_alpha` and `save_selection_channel` check the predicate before
appending and return their existing failure values (`-1` / `Edit::None`) without
touching channels or history.

Why not a `pictura_channels` crate: the docs propose one, but no such crate
exists; the document model already owns `Document.channels`, so the rule belongs
next to it. Why not mirror the writer's output-mode calculation in core: that
would duplicate codec policy and risk drift; the conservative count is enough at
50+ saved selections.

### App bridge: extend the existing selection bridge

`cxxqt_object.rs` is at its allowlist ceiling, and `impl_selection.rs` is near
the code cap. Reuse `crates/pictura-app/src/cxxqt_object/impl_selection/bridge.rs`,
already listed in `build.rs` and `api/selection_bridge.h`, with implementations
in `impl_selection/bridge/imp.rs`:

- `selection_select_ellipse_aa(view, x, y, w, h, mode, feather, anti_alias)`
- `selection_end_lasso_aa(view, feather, anti_alias)`
- `selection_magic_wand_aa(view, x, y, tolerance, contiguous, mode, anti_alias)`

They reuse `feathered`, `feathered_within`, `combine_mode_from`,
`apply_selection_labeled` / `apply_selection_boxed`, and the existing lasso
gesture state. `impl_selection::bridge::imp` is a descendant of
`impl_selection`, so it can call the module-private `apply_selection_boxed`
without widening its visibility. The existing member methods stay binary for the
control server, selftests, and older tests. No `cxxqt_object.rs`, `build.rs`, or
bridge C++ header change is needed. The cap refusal tests go in
`impl_selection/cap_tests.rs` so `impl_selection.rs` stays below 1200.

### Options bar and persistence

Add `SelectionOptions::setAntiAlias`. Enable the existing Anti-alias checkboxes
for the Elliptical Marquee and the three lassos, and the wand page's checkbox;
initialize each with `setChecked(group->antiAlias())`, connect it to
`setAntiAlias`, and add object names for Qt tests. Replace the old
"Not modelled: the rasteriser is binary" tooltips with the new behaviour: the
ellipse uses inferred analytic edge supersampling and the wand uses the inferred
boundary ramp, both behavioural parity rather than Adobe pixel parity.

`applySelection` calls `setAntiAlias`, and `SelectionOptions::apply` copies
`antiAlias_` so restore does not keep the default. Add `antiAlias` to
`selectionFields`; `quickSelection` shares that struct, so its persisted row
also gains the field. Update `optionRows()` in
`tst_tool_options_persistence.cpp` for `selection`, `ellipticalSelection`, and
`quickSelection`, add a `wandSelection` row, and keep
`roundTripTableIsComplete` green.

Give the Magic Wand its own `SelectionOptions` group registered for
`ToolId::MagicWand` and persisted as `wandSelection`. This avoids a visible
stale checkbox when switching between the lasso and wand pages and matches CS6's
per-tool options. The three lassos continue to share one group, as they already
do for Feather and combine mode. Update `selftest_tools_selection.cpp`'s Magic
Wand option setup (currently `ToolId::Marquee`) to `ToolId::MagicWand`, and run
the app self-test in verification.

### Cap refusal UX

Disable the Channels panel's New Channel button when `channels_can_add` is
false, with a tooltip naming the 56-channel limit. Keep Save Selection enabled
(the user may not see why it is disabled) and show a non-modal status-bar
message when it is invoked at the cap, in both the panel and
`Select > Save Selection`; the message names the 56-channel limit so
`tst_selection_aa::saveSelectionRefusesAtCap` can assert it without a modal
dialog. The engine still returns failure for automation and control-server
callers.

## Risks / Trade-offs

- **Unpublished Photoshop AA values** → The specs require observable properties
  (fractional boundary, binary off, 50 % contour within 1 px), not byte parity.
  The wand ramp is labelled inferred; the manual test row notes the missing CS6
  comparison.
- **4×4 ellipse quantization** → 17 coverage levels on the boundary
  (0/16..16/16). The `ponytail:` note names the 8×8 upgrade path; the 4×4 grid
  is a documented implementation choice, not a spec ceiling.
- **Conservative channel budget for converted modes** → A converted
  Indexed/Bitmap/Duotone document may refuse one or two channels earlier than
  its eventual PSD output could hold. The predicate uses
  `source_mode.unwrap_or(mode)`, so a converted CMYK document counts its four
  source channels and no save can exceed 56.
- **`shape::coverage_box` cost** → O(bbox area) row accumulation on a huge
  diagonal lasso. The output is already a document-sized mask; rows run in
  parallel. No extra materialization beyond the bbox mask.
- **File-size ceilings** → the free functions land in the existing
  `impl_selection/bridge.rs` and `bridge/imp.rs`, so `cxxqt_object.rs` and
  `impl_selection.rs` do not grow. `pictura-core/src/lib.rs` gains one
  `pub mod` line; the codec test goes to a new integration-test file.
- **Lasso AA shares one options group** → Toggling AA on one lasso page does not
  refresh the other lasso pages' checkboxes until they are rebuilt; this is the
  existing behaviour for Feather and combine mode and is out of scope.

## Migration Plan

No data migration. PSDs with 56 channels remain readable; adding another channel
is refused. Existing selections and channel round-trips are unchanged. Rollback
is deleting the change; no persisted format changes, and the new session key
`wandSelection` falls back to defaults when absent.

## Open Questions

- Whether 4×4 edge sampling is visually sufficient for very large ellipses. This
  is a rendering-quality knob that does not change the spec or the task
  breakdown; upgrade to 8×8 only if a review finds banding.
