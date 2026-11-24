# Proposal

## Why

`clipSelectionShapeToCanvas` is still a Qt polygon boolean in the C++ shell. That leaves selection preview clipping untestable without Qt and blocks other engine consumers from reusing it.

## What Changes

- Move the clipping algorithm into `pictura-select` as a Qt-free function over floating-point loops and a canvas rectangle.
- Use `i_overlay` 9.2.0 with default features disabled for the even-odd intersection. The crate is licensed MIT OR Apache-2.0 and has no Qt dependency.
- Keep `pictura::clipSelectionShapeToCanvas` as the only C++ entry point. It will serialize loops through the existing selection bridge and rebuild `QPolygonF` values from the engine result.
- Preserve the current rules for short traces, wholly outside loops, closed traces that enclose nothing, self-overlap, and canvas-border closure across the six pinned cases and a deterministic Qt differential corpus. Open zero-area bands use path clipping so the magnetic-lasso preview stays inside the canvas.
- Compare preserved traces with exact ordered equality. Compare filled intersections with a `1e-6` topology tolerance after simplifying collinear runs, splitting implicit edge crossings, and matching undirected boundary segments.
- Use a local copy of the complete old clipping function as the Qt oracle. `QPainterPath::intersected` remains only the oracle's boolean branch.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/selection-outline-performance`: add an engine-owned clipping contract, pinned parity cases, a deterministic differential corpus, and a numeric tolerance requirement.

## Impact

- Adds `i_overlay` and its four small MIT-licensed support crates to `pictura-select`. The optional Rayon feature stays off.
- Changes `crates/pictura-select`, the selection cxx-qt bridge, `cpp/api/selection_api.{h,cpp}`, `selection_geometry.cpp`, `tst_selection_preview.cpp`, and the bridge verb manifest.
- Updates `Cargo.toml`, `Cargo.lock`, and the generated `THIRD-PARTY-LICENSES`. No document format or public C++ signature changes. Behavior parity is claimed for the pinned cases and differential corpus, not arbitrary untested inputs. No performance improvement is claimed.
