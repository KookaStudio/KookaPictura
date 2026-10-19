# Proposal

## Why

Dragging the Layers panel Opacity or Fill slider runs a full synchronous
document recomposite on every value tick, and the resulting `changed` signal
also rebuilds the whole layer tree with thumbnails. On a non-trivial document the
slider stutters and the canvas lags behind the pointer (#448). The history
commit is already one per gesture; the per-tick cost is the live preview.

Photoshop's Layers panel is reported to coalesce opacity/fill updates and let
the canvas catch up with a short settle rather than on every tick. Treat that as
a design reference, not verified CS6 parity.

## What Changes

- **Coalesce previews.** The Layers panel SHALL apply at most one opacity/fill
  preview per short interval (~120 ms), using the latest pending value,
  including while the pointer is held stationary mid-drag. The timer is started
  once and is not restarted by later ticks.
- **Preserve through a panel sync.** A pending preview SHALL NOT be overwritten
  or discarded when the panel refreshes its model; the field keeps the user's
  latest value and the pending preview fires on its interval. It SHALL NOT be
  applied early by a panel refresh.
- **Cancel on target change.** A real view change, a same-view document
  replacement, or a change to the selected targets SHALL cancel the gesture and
  the field's pending commit, before or after the preview fires; no value is
  applied to the new target. A model reset preserves the gesture only while its
  selection is transiently empty, and the commit is a no-op when the captured
  target no longer resolves. A canceled gesture that already previewed leaves
  the document modified and dirty with no history state.
- **One bounded commit.** Releasing the edit SHALL stop and discard the pending
  preview and apply the final value once, adding exactly one undo state. The
  commit SHALL use the same bounded-region path as the preview (full-document
  fallback for unbounded layers). When the preview already applied the final
  value, only the history state is recorded. A gesture that returns to its
  starting value before the interval elapses SHALL do no work when the final
  stored byte equals the starting stored byte (`100%` is a fixed point;
  `127 -> 128` is a known one-state ceiling).
- **Region-scope preview and commit.** Reuse `layer_visibility_region` +
  `refresh_region` for opacity and fill, falling back to the full recomposite
  for any layer that helper cannot bound (groups, effect-bearing layers,
  unbounded/disabled adjustment masks, channel-less or shape layers). The
  result MUST be byte-identical to a full recomposite. A canvas-sized
  rectangle uses the region path; the constraint is not to run the full
  recomposite path, not to avoid a canvas-sized region.
- **Default disabled fields.** When no layer is selected, or no selected layer
  is eligible for a field, the field SHALL display `100%` and be disabled. This
  covers the Background, a fully locked layer, and a group's Fill. An enabled
  field shows the current row's value.
- **Tests and a profile.** Add Qt Test cases for coalescing, continuous ticks,
  mid-drag pause, fast round trips (including a non-fixed start byte), commit,
  preservation through a panel refresh, view/selection/same-view replacement
  cancellation, disabled defaults, synthetic-row eligibility, the bounded
  preview and commit regions, the canvas-sized region, and the full fallback;
  add a Rust helper test and an ignored profile for N synthetic steps.

No bridge signature, file-format, or dependency changes.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `ui/layers-panel`: Opacity and Fill previews are coalesced and preserved
  through panel syncs, are canceled when their target changes, and the commit
  uses the bounded region path. The percent controls show `100%` when disabled
  for lack of an eligible target.
- `compositing/described-damage`: live previews and their commits are covered by
  the bounded-operation rule; a canvas-sized region is clarified as a region
  path, not the full recomposite path.

## Impact

- `crates/pictura-app/cpp/panels/layers_panel.{h,cpp}`: preview timer, pending
  state, capture/cancel points, `syncControls` preservation and defaults.
- `crates/pictura-app/cpp/panels/numeric_field.{h,cpp}`: a `cancelPending()`
  seam for target changes.
- `crates/pictura-app/src/cxxqt_object/impl_layers.rs`: shared bounded-region
  helper used by the preview, commit, and `set_layers_*` paths.
- `crates/pictura-app/cpp/tests/tst_layers_preview_coalescing.cpp`: new Qt Test cases.
- `crates/pictura-app/src/cxxqt_object/tests_profiles.rs`: an ignored preview
  profile.
- `crates/pictura-app/src/cxxqt_object/state.rs` and `impl_core.rs`: a document
  generation exposed to the panel so a same-view replacement ends the gesture.
- Issue #448; related to #404 and #379 (other recomposite costs, out of scope).
