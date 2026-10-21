# Index

Full file listing with status. Statuses: `planned` (not written), `stub`,
`draft`, `spec'd`, `verified`. See [`README.md`](README.md) for the legend.

> **Status note (2026-09):** the corpus is written. Every spec row below that
> reads `planned` is now `draft` — the per-row column is retained for layout
> only and is superseded by this note. Root docs are `spec'd`. A file is
> `draft`, not `spec'd`, until its area has had a cross-review pass.
>
> **Version corrections:** artboards (CC 2015) and linked smart objects (CC 2014)
> are **not CS6**; see `11-cross-cutting/open-questions.md` §A for the full
> corrected-attribution list.
>
> **One file was never written:** `01-architecture/build-and-packaging.md`
> (`ARCH-014`). The `**Depends on:**` lines that name it point at a spec that
> does not exist; the gap and its issue are tracked in
> [`dev/doc-code-parity-gaps.md`](dev/doc-code-parity-gaps.md) (`ARCH-1`, #494).

## Root

| File | Purpose | Status |
|---|---|---|
| `README.md` | Project overview, source policy, how to read | spec'd |
| `INDEX.md` | This file | spec'd |
| `SPEC_TEMPLATE.md` | Canonical per-feature spec structure | spec'd |
| `GLOSSARY.md` | Terms and CS6 vocabulary | draft |
| `TRACEABILITY.md` | Feature → spec → Rust → Qt matrix | draft |

## 00-overview

| File | Purpose | Status |
|---|---|---|
| `product-overview.md` | What Kooka Pictura is, target parity, audiences | draft |
| `cs6-editions-and-constraints.md` | Standard vs Extended, version-13 feature set | draft |
| `feasibility-and-non-goals.md` | What is realistic, what is explicitly dropped | draft |
| `licensing-and-provenance.md` | Trademark, independent-creation method, asset policy | draft |

## 01-architecture

| File | Purpose | Status |
|---|---|---|
| `system-architecture.md` | Layering: Rust core, Qt6 shell, GPU, IPC boundaries | planned |
| `rust-core-design.md` | Crate layout, types, error model | planned |
| `qt6-ui-design.md` | Widgets vs QML, model/view, styling/dark theme | planned |
| `rust-qt-interop.md` | cxx-qt vs qmetaobject-rs vs manual FFI | planned |
| `threading-and-concurrency.md` | Threading, cancellation, progress, rayon/tokio | planned |
| `gpu-rendering-pipeline.md` | Compositing, wgpu↔QRhi, tile cache | planned |
| `color-management.md` | ICC, working spaces, lcms2, soft proofing | draft |
| `document-model.md` | Layer/channel/path/mask tree, serialization | draft |
| `undo-history.md` | History states, snapshots, memory strategy | draft |
| `plugin-and-scripting-abi.md` | Plugin ABI + scripting host design | planned |
| `file-formats.md` | PSD/PSB/TIFF/PNG/JPEG/RAW read-write matrix | draft |
| `performance-targets.md` | Latency/throughput budgets | planned |
| `build-and-packaging.md` | Linux packaging: Flatpak/AppImage/native | planned |

## 02-ui-ux

| File | Purpose | Status |
|---|---|---|
| `application-frame.md` | Window, workspace, dark UI, full-screen modes | planned |
| `menus.md` | Complete CS6 menu tree | planned |
| `workspace-and-docks.md` | Docking, tabbing, saved workspaces | planned |
| `toolbox-and-options-bar.md` | Tools panel, options bar behavior | planned |
| `preferences.md` | All preference pages and settings | planned |
| `keyboard-shortcuts.md` | Full CS6 shortcut map | planned |
| `accessibility.md` | Keyboard nav, screen reader, high contrast | planned |
| `panels/layers-panel.md` | | planned |
| `panels/channels-panel.md` | | planned |
| `panels/paths-panel.md` | | planned |
| `panels/history-panel.md` | | planned |
| `panels/adjustments-panel.md` | | planned |
| `panels/color-panel.md` | | planned |
| `panels/swatches-panel.md` | | planned |
| `panels/brushes-panel.md` | | planned |
| `panels/properties-panel.md` | | planned |
| `panels/info-panel.md` | | planned |
| `panels/navigator-panel.md` | | planned |
| `panels/histogram-panel.md` | | planned |
| `panels/tool-presets-panel.md` | | planned |
| `panels/actions-panel.md` | | planned |
| `panels/character-and-paragraph.md` | | planned |
| `panels/3d-panel.md` | Extended-only | planned |
| `panels/timeline-panel.md` | Video/animation | planned |
| `panels/measurement-log-panel.md` | | planned |
| `panels/note-panel.md` | | planned |
| `panels/clone-source-panel.md` | | planned |
| `panels/smart-object-and-mask-panels.md` | | planned |

## 03-tools

| File | Status |
|---|---|
| `move-and-transform.md` | planned |
| `marquee-selection.md` | planned |
| `lasso-selection.md` | planned |
| `quick-selection-and-magic-wand.md` | planned |
| `crop-tool.md` | planned |
| `perspective-crop.md` | planned |
| `slice-tools.md` | planned |
| `eyedropper-color-sampler-ruler.md` | planned |
| `note-and-count.md` | planned |
| `brush-and-pencil.md` | planned |
| `color-replacement.md` | planned |
| `mixer-brush.md` | planned |
| `eraser-tools.md` | planned |
| `gradient-and-paint-bucket.md` | planned |
| `clone-stamp-and-pattern-stamp.md` | planned |
| `healing-brushes.md` | planned |
| `content-aware-move-and-patch.md` | planned |
| `history-brush.md` | planned |
| `art-history-brush.md` | planned |
| `smudge-blur-sharpen.md` | planned |
| `dodge-burn-sponge.md` | planned |
| `type-tools.md` | planned |
| `pen-and-path-tools.md` | planned |
| `path-selection-tools.md` | planned |
| `shape-tools.md` | planned |
| `custom-shape.md` | planned |
| `hand-and-zoom.md` | planned |
| `rotate-view.md` | planned |
| `quick-mask-tool.md` | planned |
| `3d-tools.md` | planned |

## 04-image-ops

| File | Status |
|---|---|
| `image-size.md` | planned |
| `canvas-size.md` | planned |
| `image-rotation-and-flip.md` | planned |
| `adjustments-overview.md` | planned |
| `adjustments/levels.md` | planned |
| `adjustments/curves.md` | planned |
| `adjustments/brightness-contrast.md` | planned |
| `adjustments/exposure.md` | planned |
| `adjustments/vibrance.md` | planned |
| `adjustments/hue-saturation.md` | draft |
| `adjustments/color-balance.md` | draft |
| `adjustments/black-white.md` | draft |
| `adjustments/photo-filter.md` | draft |
| `adjustments/channel-mixer.md` | draft |
| `adjustments/invert.md` | planned |
| `adjustments/posterize.md` | planned |
| `adjustments/threshold.md` | planned |
| `adjustments/gradient-map.md` | draft |
| `adjustments/selective-color.md` | planned |
| `adjustments/shadow-highlight.md` | planned |
| `adjustments/hdr-toning.md` | planned |
| `adjustments/desaturate.md` | planned |
| `adjustments/match-color.md` | planned |
| `adjustments/replace-color.md` | planned |
| `adjustments/auto-adjustments.md` | planned |
| `image-modes.md` | planned |
| `bit-depth-and-conversion.md` | planned |
| `color-profiles-and-assignment.md` | planned |
| `duotone.md` | planned |
| `indexed-color.md` | planned |
| `32-bit-hdr.md` | planned |

## 05-layers

| File | Status |
|---|---|
| `layers-overview.md` | planned |
| `blend-modes.md` | planned |
| `layer-styles.md` | planned |
| `adjustment-layers.md` | planned |
| `fill-layers.md` | planned |
| `layer-groups.md` | planned |
| `layer-masks.md` | planned |
| `vector-masks-and-clipping-masks.md` | planned |
| `smart-objects.md` | draft |
| `smart-filters.md` | draft |
| `layer-comps.md` | draft |
| `artboards.md` | draft — **non-goal: not CS6 (CC 2015)** |
| `linked-and-embedded-objects.md` | draft — linked SO are **not CS6 (CC 2014)** |
| `layer-management-ui.md` | planned |
| `align-and-distribute.md` | planned |
| `merge-and-flatten.md` | planned |
| `layer-filtering-and-search.md` | planned |

## 06-filters

| File | Status |
|---|---|
| `filters-overview.md` | planned |
| `blur-filters.md` | planned |
| `sharpen-filters.md` | planned |
| `distort-filters.md` | planned |
| `noise-filters.md` | planned |
| `stylize-filters.md` | planned |
| `render-filters.md` | planned |
| `artistic-filters.md` | planned |
| `brush-strokes-filters.md` | planned |
| `sketch-filters.md` | planned |
| `texture-filters.md` | planned |
| `pixelate-filters.md` | planned |
| `other-filters.md` | planned |
| `liquify.md` | planned |
| `oil-paint.md` | planned |
| `blur-gallery.md` | planned |
| `sharpening-tools.md` | planned |
| `adaptive-wide-angle.md` | planned |
| `camera-raw-filter.md` | planned |
| `lens-correction.md` | planned |
| `vanishing-point.md` | planned |
| `lighting-effects.md` | planned |
| `extract-and-pattern-maker.md` | planned |

## 07-color-painting

| File | Status |
|---|---|
| `color-models.md` | planned |
| `color-picker.md` | planned |
| `swatches-and-libraries.md` | planned |
| `brush-engine.md` | planned |
| `brush-dynamics.md` | planned |
| `bristle-brushes.md` | planned |
| `mixer-brush-engine.md` | planned |
| `airbrush-and-flow.md` | planned |
| `brush-presets.md` | planned |
| `pattern-presets.md` | planned |
| `color-sweeps-and-modes.md` | planned |
| `gradient-presets.md` | planned |
| `histogram-and-info.md` | planned |

## 08-selection

| File | Status |
|---|---|
| `selection-model.md` | planned |
| `selection-tools-overview.md` | planned |
| `refine-edge.md` | planned |
| `quick-mask.md` | planned |
| `channel-based-masking.md` | planned |
| `paths-and-vector-selection.md` | planned |
| `color-range.md` | planned |
| `grow-similar-and-modify.md` | planned |
| `transform-selection.md` | planned |
| `save-and-load-selections.md` | planned |

## 09-automation

| File | Status |
|---|---|
| `actions.md` | planned |
| `droplets.md` | planned |
| `batch-processing.md` | planned |
| `script-events-and-jsx.md` | planned |
| `extendscript-api-surface.md` | planned |
| `rust-scripting-replacement.md` | planned |
| `variables-and-data-driven-graphics.md` | planned |
| `plugin-sdk.md` | planned |
| `automation-plugins.md` | planned |
| `mini-bridge.md` | planned |

## 10-workflow-io

| File | Status |
|---|---|
| `document-lifecycle.md` | planned |
| `open-and-new.md` | planned |
| `save-and-save-as.md` | planned |
| `export-formats.md` | planned |
| `web-export-and-slices.md` | planned |
| `file-info-and-metadata.md` | planned |
| `color-settings.md` | planned |
| `camera-raw-workflow.md` | planned |
| `printing.md` | planned |
| `measurement-and-count.md` | planned |
| `presets-manager.md` | planned |
| `workspace-management.md` | planned |
| `scratch-disks-and-memory.md` | planned |
| `bridge-and-interop.md` | planned |

## 11-cross-cutting

| File | Status |
|---|---|
| `localization.md` | planned |
| `preference-storage.md` | planned |
| `logging-and-telemetry.md` | planned |
| `testing-strategy.md` | draft |
| `error-handling.md` | planned |
| `security-and-sandboxing.md` | planned |
| `update-and-versioning.md` | draft |
| `crash-recovery-and-autosave.md` | draft |
| `open-questions.md` | draft |
| `gap-analysis.md` | draft |
