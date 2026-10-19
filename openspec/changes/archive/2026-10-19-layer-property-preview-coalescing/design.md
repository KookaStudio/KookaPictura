# Design

## Context

See `proposal.md` - Why.

The current Layers-panel path is:

- `NumericField` emits `valueChanged` per user step and `valueCommitted` once
  when the gesture ends (`crates/pictura-app/cpp/panels/numeric_field.cpp`).
  That contract is correct and is not changing.
- The panel's `valueChanged` lambdas call `preview_layers_opacity` /
  `preview_layers_fill`; each mutates the layer and calls full `recomposite()`
  (`crates/pictura-app/src/cxxqt_object/impl_layers.rs`).
- `recomposite()` composites the whole document, resets the pyramid, and emits
  `changed` (`impl_core.rs`). The frame's `changed` handler runs an immediate
  `refresh()` and starts a 120 ms single-shot panel timer.
- The panel also queues its own `refresh()` on every `changed`
  (`layers_panel.cpp`). `refresh()` rebuilds the model and ends in
  `syncControls()`, so a model sync can run while a drag is still in progress.
- The frame's `regionBlitted` handler blits the region and restarts the same
  120 ms single-shot panel timer; `retargetDock()` then calls
  `LayersPanel::setView()` and `refresh()` (`frame.cpp`, `frame_session.cpp`).
- `set_layers_opacity` / `set_layers_fill` already use `batch_property`, which
  computes the changed layers' bounded union with `layer_visibility_region` and
  calls `refresh_region`. The panel's `commit_layers_*` functions do not; they
  always call full `recomposite()`.
- `refresh_region` patches `doc.composite` and the level-0 frame before
  emitting, so a history `record()` after a region refresh captures a current
  composite.

Constraints: the `compositing/described-damage` contract requires bounded
opacity/fill changes to avoid the full recomposite path; the `NumericField`
public preview/commit contract stays as specified; no new dependencies.

## Goals / Non-Goals

**Goals:**

- Bound the number of preview composites during a drag and keep the slider
  responsive.
- Update the preview while the pointer is held still without releasing.
- Preserve a pending preview through the panel's own model syncs.
- Cancel a pending gesture when its view or target selection changes.
- Use the existing bounded-region path for both the preview and the commit.
- Show `100%` in a field that has no eligible target.
- Keep exactly one commit and one undo state per finished gesture.
- Measure the change with a counting test and an ignored profile.

**Non-Goals:**

- Moving compositing off the GUI thread.
- Lower-resolution or GPU-resident previews.
- Per-path originals to suppress the one-state `127 -> 128` round-trip ceiling.
- Changing the options-bar or dialog sliders; only the Layers panel coalesces.

## Decisions

### 1. Start-if-idle throttle in the panel, not a debounce in the control

On the first `valueChanged` of a gesture, the panel stores
`{paths, field, value}` and starts a single-shot 120 ms timer. Later ticks
replace only the stored value; they do not restart the timer. When it fires, the
latest value is previewed. On commit the timer is stopped and the pending value
is discarded because the commit carries the final value.

This gives:

- a held-still pointer a preview within the interval (the timer was already
  running);
- continuous motion at most one preview per interval;
- a fast round trip shorter than the interval zero previews. When the final
  stored byte equals the starting stored byte, the commit is a no-op; when the
  byte differs (`127` displays `50%` but `round(50 * 255 / 100)` is `128`), the
  release records exactly one state. `100%` is a fixed point and is the zero-work
  case.

Alternatives considered:

- **Restart-per-tick debounce.** It would starve previews during continuous
  motion and contradicts the existing "canvas updates live" contract.
- **Leading-edge plus trailing flush.** The first tick would preview
  immediately, so a fast right-left gesture would composite twice and record a
  redundant state on release.
- **A 0 ms single-shot coalescer.** It still blocks the GUI thread on every
  event-loop turn during a drag.
- **Timer in `NumericField` configured per site.** It would change every numeric
  field's behavior and duplicate a policy the panel owns. The control keeps
  emitting per change; only the bridge calls are coalesced.

The interval is a local named constant matching the existing 120 ms panel
timers. It is a tuning knob, not a claim of verified CS6 parity.

### 2. Preserve a gesture through a sync; cancel on target change

There is no flush in `refresh()`. The panel's `changed` connection and the
frame's 120 ms panel timer both call `refresh()`/`syncControls()` during a drag.
Flushing there would defeat the throttle: an unbounded preview emits `changed`
and can queue an immediate refresh, and any unrelated `changed` can do the same.
A flush would apply the latest pending value in the next event-loop turn instead
of waiting for the interval.

Instead:

- The panel keeps a persistent gesture record `{paths, namePaths, fill,
  generation}` independent of the timer's pending slot. It is captured on the
  first tick and cleared only by commit or cancel. The model's occurrence-tagged
  name path is the stable layer identity across index-path shifts; the document
  generation catches a same-view replacement.
- `syncControls()` skips `setValue()` for the field with an active gesture. The
  field keeps the user's latest value; the model is rebuilt from the last
  applied document state. The timer remains the only preview trigger.
- Outside a refresh, an empty real-target set or a different name-path chain
  cancels the gesture and the field's pending commit.
- `refresh()` reconciles after the selection is restored: a transiently empty
  selection is kept, a restored non-empty selection with different name paths
  (or no real target) cancels, and the restored paths replace the stale captured
  paths. This is what catches a delete, duplicate, reorder, or undo that changes
  the selection during the model reset.
- `setView()` cancels on a different view or a changed document generation. The
  timer and the commit check the generation too, so a Revert/open that replaces
  the document in the same view cannot apply the old gesture to the new layers.
- The timer and the commit resolve the captured name paths back to current
  paths. If a target no longer resolves, the preview cancels and the commit is a
  no-op rather than falling back to the current selection.
- The commit never re-reads `selectedPaths()` when a gesture is active. It uses
  the captured targets, so a preview that already fired cannot leak into a new
  selection on release.
- `NumericField::cancelPending()` latches until a fresh press or popup open, so a
  continued label/slider drag cannot re-arm the edit against the new target.
- A canceled gesture whose preview already changed pixels leaves the document
  modified and dirty, with no history state; per-path originals are out of scope.
- Synthetic Smart Filters rows are display-only and are excluded from the
  property target set and eligibility.

Alternatives considered: flushing on every refresh (rejected: applies previews
early and violates the coalescing interval); cancelling on every refresh
(rejected: the frame refreshes every 120 ms during a drag, so the pending update
would be lost); cancelling only while the timer slot is valid (rejected: the
slot is cleared when the preview fires, so a post-preview target change would
commit to the new selection).

### 3. Region-scope both the preview and the commit

Extract the before/after compositing check and the `layer_visibility_region`
union from `batch_property` into one private helper returning:

- `None`: unbounded (group, effect-bearing layer, unbounded/disabled adjustment
  mask, channel-less or shape layer) -> full `recomposite()`;
- `Some(None)`: no changed layer needs a repaint;
- `Some(Some(rect))`: the union of the changed layers' bounds ->
  `refresh_region(rect)`.

The preview path uses the helper, sets the sticky preview flag, and bumps
`content_revision` exactly as today. The commit path uses the same helper:

- `changed > 0`: region refresh (or full fallback), then `record()`;
- `changed == 0 && pending`: the preview already patched `doc.composite`, so
  only `record()` runs; emit `changed()` so the panel/model and frame sync;
- `changed == 0 && !pending`: no-op;
- after a region refresh or a record-only commit, emit `changed()` once so the
  panel model updates. `recomposite()` already emits it on the full fallback.

This satisfies the bounded-operation requirement for the commit as well as the
preview, and removes the release-time full composite when the preview already
applied the final value. `refresh_region` patches `doc.composite`, so the
history snapshot is current.

A full-canvas pixel layer's `layer_visibility_region` is its full rect: the
region path is used with a canvas-sized rectangle. The cost equals a full
composite, but the region path and signals are still used; the spec's
constraint is "do not run the full-document recomposite path", not "do not
touch every pixel". The throttle is the win for that case.

Alternatives considered: keeping the commit as one full recomposite (rejected:
it contradicts the existing bounded-operation requirement and leaves the release
hitch); weakening the requirement to permit a full commit (rejected: the
`set_layers_*` path already proves the bounded commit works).

### 4. Eligibility, not selection, drives the default display

`syncControls()` computes `opacityAny` and `fillAny` over the whole selection
first, then sets the field values:

- no selection, or no selected layer eligible for the field -> `100` and
  disabled;
- at least one eligible selected layer -> enabled, showing the current row's
  value.

This covers the empty selection, the Background, a fully locked layer, and a
group's Fill (when no selected layer is eligible). A multi-selection with one
eligible layer keeps the field enabled. The Background's "expose neither
opacity nor fill" means no editable value; the disabled control still displays
the default. If eligibility becomes false while a preview is pending, cancel the
gesture and show `100`.

The current code sets the two field values before the eligibility loop, so this
is a reorder plus the cancel rule, not a new control.

### 5. Measurement

The coalescing Qt tests count `changed` + `regionBlitted` emissions for N
synthetic ticks. A continuous-tick case asserts at least two previews while the
ticks keep arriving, so a restart-per-tick debounce fails; the existing pause
and round-trip cases bound the interval. The commit cases clear the damage
account, commit, and assert the region path left damage behind; a full rebuild
clears the account, so it would fail. A separate `#[ignore]` profile in
`tests_profiles.rs` runs N synthetic full and region composites on a 4000²
document with a bounded layer and records the per-step cost; it measures the two
render calls, not the app's preview count (the Qt tests own the coalescing
count). The profile is a development measurement, not an oracle parity claim.

## Risks / Trade-offs

- **A full-canvas layer still composites the whole canvas on each preview and
  commit.** The region path is used, but the cost is the same as a full
  composite; the throttle bounds how often. The spec is clarified accordingly.
- **Panels refresh 120 ms after the last region blit, not on a fixed cadence.**
  The frame's timer is a restarting single-shot, so continuous previews can defer
  the panel rebuild. The field is not overwritten because of the
  preserve-don't-sync rule. This is existing panel behavior, not a per-tick
  cost.
- **A `127` start that returns to `50%` maps to `128` and records one state.**
  This is a known ceiling; `100%` is the zero-work fixed point. Per-path
  originals remain out of scope.
- **Cancellation must cancel the field's own pending commit, not only the
  panel's pending preview.** The persistent gesture record, the name-path
  identity, and `NumericField::cancelPending()`'s canceled-until-next-press guard
  are all required; a commit that re-read the current selection would write to
  the new target after a preview had already fired.
- **The commit's `changed` triggers the queued panel refresh and the frame's
  coalesced panel refresh, not a single refresh.** That is intentional: the
  region preview removes the immediate `changed` fan-out, and the commit emits
  one sync.
- **The 120 ms interval is a heuristic**, matching this project's existing
  settle timers and the reported Photoshop behavior. It is not an
  oracle-verified CS6 value.
- **The shared helper is now used by the preview, the commit, and
  `batch_property`.** Keep it pure and unit-tested so the three call sites
  cannot drift.
