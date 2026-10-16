# Proposal: pen-tool

## Why

The Pen tool (issue #32) was catalogued but disabled. photorust's
`core/src/path.rs` models a vector path of anchors with direction handles; the
port follows `docs/03-tools/pen-and-path-tools.md`.

## What Changes

- `pictura_core::path` (new module): `VectorPath` of `Subpath`s of
  `PathPoint`s (anchor, optional in/out handles, smooth flag) in `f64` document
  pixels. Appending corners, pulling symmetric handles on a drag (Alt: the
  outgoing handle only), closing, finishing open, resuming from an open
  endpoint (CS6's extend), and anchor / handle / segment hit tests.
  `Document::work_path` holds the Work Path; it rides the history snapshot.
- `cxxqt_object/paths.rs` (new bridge): the Work Path's reads and edits. A Pen
  click or drag records one "New Work Path" (the first anchor) or "Add Anchor
  Point" state on release; closing records "Close Path".
- `tool_pen.cpp`: the Pen handler (click corner, drag smooth, Shift 45°, click
  the first anchor to close, Ctrl-click / Enter / Esc / a tool switch end the
  path open, Auto Add/Delete over the path while not drawing). The Work Path
  overlay (`image_view_paths.cpp`) shows while a Pen-group tool is active.
- `options_bar_pen.cpp`: Auto Add/Delete (on) and Rubber Band (off).
- `panels/paths_panel.cpp` replaces the Paths placeholder: one italic "Work
  Path" row with a thumbnail once the path exists, "No Paths" otherwise.
- Keys: P and Shift+P cycle only the slot's lettered members (Pen, Freeform
  Pen); P from an anchor tool selects the Pen.
- Qt Test `tst_pen_tools::penTool`, `penAutoAddDelete`,
  `pathsPanelListsTheWorkPath`. The guard (98) now
  probes Horizontal Type and `shift_plain` (117) presses T.

## Capabilities

### New Capabilities

- `tools/pen-tool`: the Pen tool.

## Impact

- `pictura-core` (`path.rs`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/path.rs`
(<https://github.com/perfecto25/photorust>). Behavioural parity only: no CS6 oracle exists for path geometry or
history labels. Ceiling (`ponytail:`): no Shape / Pixels mode, path operations, saved paths or
Paths panel commands, or
PSD path-resource read/write; the history labels are approximations.
