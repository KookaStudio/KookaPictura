# Proposal

## Why

Issue #1197: clicking a layer's eye in the Layers panel stalls on large
documents. The visibility edit itself is cheap — one boolean and the affected
layer's influence — but it presents through the region path without a `changed`
notification. The frame therefore treats the edit as an ordinary region present
and its 120 ms panel timer calls `refreshDock(includeSelfRefreshing=true)`, which
rebuilds the whole Layers snapshot and every thumbnail. The panel already
patched the toggled rows in place, so that rebuild is redundant.

## What Changes

- A visibility edit emits a dedicated `layerVisibilityChanged(paths, visible)`
  signal after its region present.
- The Layers panel connects to that signal and patches the affected rows in
  place instead of rebuilding its snapshot.
- The debounced dock pass skips the Layers panel when the pending pass was
  triggered by a visibility edit, while History and the other panels still
  refresh. A following non-visibility edit rebuilds the snapshot normally.
- Solo visibility keeps the full rebuild because it changes every row.

## Capabilities

### Modified Capabilities

- `ui/panel-refresh-lifecycle`: a visibility-only edit patches the Layers rows
  in place and does not rebuild the snapshot or its thumbnails.

## Impact

- `crates/pictura-app/src/cxxqt_object.rs` and `impl_layers.rs`: the new signal
  and its emission from `set_layer_visible` / `set_layers_visible`.
- `crates/pictura-app/cpp/panels/layers_panel.*`: row patching, the
  consume seam, and the event handler no longer patching twice.
- `crates/pictura-app/cpp/frame_session.cpp` / `frame.h`: the dock pass skips
  the Layers panel for that one pass.
- No new dependencies and no engine-model change. The document, history, and
  canvas paths are untouched.
