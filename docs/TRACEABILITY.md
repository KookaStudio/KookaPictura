# Traceability Matrix

CS6 feature → spec file → proposed Rust module → proposed Qt6 component → status.

Proposed names come from `01-architecture/rust-core-design.md` (`pictura-*`
crates) and `01-architecture/qt6-ui-design.md`. They are design proposals, not
implemented APIs. Status is `draft` for every row until its area is cross-reviewed.

Rows are grouped by spec area. Feature groups with several files list the files
and the shared module.

## 00-overview

| CS6 feature | Spec file | Spec ID | Rust module (proposed) | Qt6 component (proposed) | Status |
|---|---|---|---|---|---|
| Product scope / parity goal | `00-overview/product-overview.md` | — | — | — | draft |
| Standard vs Extended editions | `00-overview/cs6-editions-and-constraints.md` | — | `pictura_core::edition` | feature gating | draft |
| Feasibility / non-goals | `00-overview/feasibility-and-non-goals.md` | — | — | — | draft |
| Licensing / independent-creation | `00-overview/licensing-and-provenance.md` | — | — | — | draft |

## 01-architecture

| CS6 feature | Spec file | Spec ID | Rust module (proposed) | Qt6 component (proposed) | Status |
|---|---|---|---|---|---|
| System layering | `system-architecture.md` | ARCH-001 | workspace root | app shell | draft |
| Rust workspace/types | `rust-core-design.md` | ARCH-002 | `pictura-core` et al. | — | draft |
| Qt6 UI design | `qt6-ui-design.md` | ARCH-003 | — | `QMainWindow`, `QDockWidget`, `QAbstractItemModel` | draft |
| Rust↔Qt interop | `rust-qt-interop.md` | ARCH-004 | `pictura-qt` | CXX-Qt bridge | draft |
| Threading | `threading-and-concurrency.md` | ARCH-005 | `pictura_core::task` | worker/cancel signals | draft |
| GPU pipeline | `gpu-rendering-pipeline.md` | ARCH-006 | `pictura-render` (wgpu) | `QRhiWidget` | draft |
| Color management | `color-management.md` | ARCH-007 | `pictura-color` (lcms2) | `QColorSpace` | draft |
| Document model | `document-model.md` | ARCH-008 | `pictura_core::document` | `LayersModel` | draft |
| Undo/history | `undo-history.md` | ARCH-009 | `pictura_core::history` | History panel model | draft |
| File formats | `file-formats.md` | ARCH-010 | `pictura-codec`, `pictura-io::psd` | open/import dialogs | draft |
| Plugin/scripting ABI | `plugin-and-scripting-abi.md` | ARCH-011 | `pictura-plugin`, `pictura-script` | plugin manager | draft |
| Performance targets | `performance-targets.md` | ARCH-013 | `pictura_core::perf` | — | draft |
| Build/packaging | `build-and-packaging.md` — never written (#494) | ARCH-014 | workspace build | Flatpak/AppImage | draft |

## 02-ui-ux

| CS6 feature | Spec file | Spec ID | Rust module (proposed) | Qt6 component (proposed) | Status |
|---|---|---|---|---|---|
| Application frame | `application-frame.md` | UI-001 | — | `QMainWindow` | draft |
| Menu tree | `menus.md` | UI-002 | `pictura_app::menu` | `QMenuBar` | draft |
| Workspaces/docks | `workspace-and-docks.md` | UI-003 | `pictura_app::workspace` | `QDockWidget`, `QSettings` | draft |
| Toolbox/options bar | `toolbox-and-options-bar.md` | UI-004 | `pictura_tools::registry` | `QToolBar` | draft |
| Preferences | `preferences.md` | UI-010 | `pictura_app::prefs` | `QDialog`, `QSettings` | draft |
| Keyboard shortcuts | `keyboard-shortcuts.md` | UI-011 | `pictura_app::keymap` | `QShortcut` | draft |
| Accessibility | `accessibility.md` | UI-012 | — | `QAccessible`, AT-SPI2 | draft |
| Panels (20 files) | `panels/*.md` | PAN-001…026 | panel models | `QDockWidget` + `QAbstractItemModel` | draft |

## 03-tools

| CS6 feature | Spec file | ID | Rust module (proposed) | Qt6 component (proposed) | Status |
|---|---|---|---|---|---|
| Move/Transform/Warp/Puppet | `move-and-transform.md` | TOOL-001 | `pictura_tools::transform` | canvas overlay | draft |
| Marquee selection | `marquee-selection.md` | TOOL-002 | `pictura_tools::select` | canvas overlay | draft |
| Lasso (free/poly/magnetic) | `lasso-selection.md` | TOOL-003 | `pictura_tools::select` | canvas overlay | draft |
| Quick Selection + Magic Wand | `quick-selection-and-magic-wand.md` | TOOL-004 | `pictura_tools::select` | canvas overlay | draft |
| Crop (redesigned) | `crop-tool.md` | TOOL-011 | `pictura_tools::crop` | canvas overlay | draft |
| Perspective Crop | `perspective-crop.md` | TOOL-012 | `pictura_tools::crop` | canvas overlay | draft |
| Slice tools | `slice-tools.md` | TOOL-013/014 | `pictura_core::slice` | canvas overlay | draft |
| Eyedropper/Sampler/Ruler | `eyedropper-color-sampler-ruler.md` | TOOL-015…017 | `pictura_tools::sample` | canvas overlay | draft |
| Note + Count | `note-and-count.md` | TOOL-018/019 | `pictura_core::annotation` | canvas overlay | draft |
| Brush/Pencil | `brush-and-pencil.md` | TOOL-020 | `pictura_tools::paint` | options bar | draft |
| Color Replacement | `color-replacement.md` | TOOL-021 | `pictura_tools::paint` | options bar | draft |
| Mixer Brush | `mixer-brush.md` | TOOL-022 | `pictura_tools::paint` | options bar | draft |
| Eraser tools | `eraser-tools.md` | TOOL-023 | `pictura_tools::paint` | options bar | draft |
| Gradient + Paint Bucket | `gradient-and-paint-bucket.md` | TOOL-024 | `pictura_tools::paint` | options bar | draft |
| Clone/Pattern Stamp | `clone-stamp-and-pattern-stamp.md` | TOOL-030 | `pictura_tools::retouch` | Clone Source panel | draft |
| Healing brushes | `healing-brushes.md` | TOOL-031 | `pictura_tools::retouch` | options bar | draft |
| Content-Aware Move/Patch | `content-aware-move-and-patch.md` | TOOL-032 | `pictura_tools::retouch` | options bar | draft |
| History/Art History Brush | `history-brush.md`, `art-history-brush.md` | TOOL-033/034 | `pictura_tools::retouch` | options bar | draft |
| Smudge/Blur/Sharpen | `smudge-blur-sharpen.md` | TOOL-040 | `pictura_tools::retouch` | options bar | draft |
| Dodge/Burn/Sponge | `dodge-burn-sponge.md` | TOOL-041 | `pictura_tools::tonal` | options bar | draft |
| Hand/Zoom | `hand-and-zoom.md` | TOOL-042 | `pictura_render::viewport` | canvas | draft |
| Rotate View | `rotate-view.md` | TOOL-043 | `pictura_render::viewport` | canvas | draft |
| Quick Mask tool | `quick-mask-tool.md` | TOOL-044 | `pictura_tools::quickmask` | canvas | draft |
| Type tools | `type-tools.md` | TOOL-050/051 | `pictura_core::text` (rustybuzz) | `QTextLayout` | draft |
| Pen/Path tools | `pen-and-path-tools.md` | TOOL-052/053 | `pictura_core::path` | `QPainterPath` | draft |
| Path Selection | `path-selection-tools.md` | TOOL-054/055 | `pictura_core::path` | `QPainterPath` | draft |
| Shape tools | `shape-tools.md` | TOOL-056/057 | `pictura_core::vector` | `QPainterPath` | draft |
| Custom Shape | `custom-shape.md` | TOOL-058/059 | `pictura_core::vector` | `QPainterPath` | draft |
| 3D tools (Extended) | `3d-tools.md` | TOOL-060 | — | non-goal | draft |

## 04-image-ops

| CS6 feature | Spec file | ID | Rust module (proposed) | Qt6 component (proposed) | Status |
|---|---|---|---|---|---|
| Image Size | `image-size.md` | IMG-001 | `pictura_core::resize` | dialog | draft |
| Canvas Size | `canvas-size.md` | IMG-002 | `pictura_core::canvas` | dialog | draft |
| Rotate/Flip image | `image-rotation-and-flip.md` | IMG-003 | `pictura_core::transform` | menu | draft |
| Image modes | `image-modes.md` | IMG-004 | `pictura-color::modes` | dialog | draft |
| Bit depth conversion | `bit-depth-and-conversion.md` | IMG-005 | `pictura_core::depth` | dialog | draft |
| Color profiles/assign | `color-profiles-and-assignment.md` | IMG-006 | `pictura-color` | dialog | draft |
| Duotone | `duotone.md` | IMG-007 | `pictura_color::duotone` | dialog | draft |
| Indexed color | `indexed-color.md` | IMG-008 | `pictura_color::quantize` | dialog | draft |
| 32-bit HDR | `32-bit-hdr.md` | IMG-009 | `pictura_core::hdr` | dialog | draft |
| Adjustments (26 files) | `adjustments/*.md`, `adjustments-overview.md` | ADJ-000…033 | `pictura_filters::adjust` / `pictura-core::adjust` | Properties panel | draft |

## 05-layers

| CS6 feature | Spec file | ID | Rust module (proposed) | Qt6 component (proposed) | Status |
|---|---|---|---|---|---|
| Layer model/overview | `layers-overview.md` | LAY-001 | `pictura_core::layer` | Layers panel | draft |
| Layers panel UI | `layer-management-ui.md` | LAY-002 | `pictura_core::layer` | `LayersModel` | draft |
| Groups | `layer-groups.md` | LAY-003 | `pictura_core::layer` | `QAbstractItemModel` | draft |
| Layer masks | `layer-masks.md` | LAY-004 | `pictura_core::mask` | Masks UI | draft |
| Vector + clipping masks | `vector-masks-and-clipping-masks.md` | LAY-005 | `pictura_core::mask` | path UI | draft |
| Blend modes (27) | `blend-modes.md` | LAY-010 | `pictura_render::blend` | — (GPU/CPU) | draft |
| Layer styles (fx) | `layer-styles.md` | LAY-011 | `pictura_filters::style` | Layer Style dialog | draft |
| Adjustment layers | `adjustment-layers.md` | LAY-012 | `pictura_core::adjust_layer` | Properties panel | draft |
| Fill layers | `fill-layers.md` | LAY-013 | `pictura_core::fill_layer` | Properties panel | draft |
| Smart objects | `smart-objects.md` | LAY-020 | `pictura_core::smart` | Properties panel | draft |
| Smart filters | `smart-filters.md` | LAY-021 | `pictura_core::smart` | Properties panel | draft |
| Layer comps | `layer-comps.md` | LAY-022 | `pictura_core::comp` | Layer Comps panel | draft |
| Align/Distribute/Auto | `align-and-distribute.md` | LAY-030 | `pictura_core::layer` | menu | draft |
| Merge/Flatten | `merge-and-flatten.md` | LAY-031 | `pictura_core::layer` | menu | draft |
| Layer filtering | `layer-filtering-and-search.md` | LAY-032 | — | `QSortFilterProxyModel` | draft |
| Artboards (not CS6) | `artboards.md` | LAY-023 | — | non-goal | draft |
| Linked SO (not CS6) | `linked-and-embedded-objects.md` | LAY-024 | — | non-goal | draft |

## 06-filters

| CS6 feature | Spec file | ID | Rust module (proposed) | Qt6 component (proposed) | Status |
|---|---|---|---|---|---|
| Filter plumbing | `filters-overview.md` | FILT-001 | `pictura_filters` | Filter menu | draft |
| Blur / Sharpen / Noise | `blur-filters.md`, `sharpen-filters.md`, `noise-filters.md` | FILT-010/020/030 | `pictura_filters::{blur,sharpen,noise}` | dialogs | draft |
| Distort / Stylize / Render / Other | `*-filters.md` | FILT-040…070 | `pictura_filters::*` | dialogs | draft |
| Filter Gallery families | `artistic…pixelate` | FILT-080…084 | `pictura_filters::gallery` | Filter Gallery | draft |
| Liquify / Oil Paint | `liquify.md`, `oil-paint.md` | FILT-090/091 | `pictura_filters::liquify` | full-screen overlay | draft |
| Blur Gallery | `blur-gallery.md` | FILT-092 | `pictura_filters::blur_gallery` | on-canvas pins | draft |
| Sharpening tools | `sharpening-tools.md` | FILT-093 | `pictura_filters::sharpen` | dialog | draft |
| Adaptive Wide Angle | `adaptive-wide-angle.md` | FILT-094 | `pictura_filters::awa` | on-canvas constraints | draft |
| Camera Raw (ACR 7) | `camera-raw-filter.md` | FILT-100 | `pictura-codec::raw` | ACR workspace | draft |
| Lens Correction | `lens-correction.md` | FILT-101 | `pictura_filters::lens` | dialog | draft |
| Vanishing Point | `vanishing-point.md` | FILT-102 | `pictura_filters::vp` | full-screen editor | draft |
| Lighting Effects | `lighting-effects.md` | FILT-103 | `pictura_filters::lighting` | dialog | draft |
| Extract/Pattern Maker (legacy) | `extract-and-pattern-maker.md` | FILT-104 | — | optional plug-in | draft |

## 07-color-painting

| CS6 feature | Spec file | ID | Rust module (proposed) | Qt6 component (proposed) | Status |
|---|---|---|---|---|---|
| Color models | `color-models.md` | CLR-001 | `pictura_color::model` | `QColorSpace` | draft |
| Color picker | `color-picker.md` | CLR-002 | `pictura_color::picker` | `QColorDialog` (custom) | draft |
| Swatches/libraries | `swatches-and-libraries.md` | CLR-003 | `pictura_color::swatch` | Swatches panel | draft |
| Histogram/Info | `histogram-and-info.md` | CLR-004 | `pictura_core::histogram` | Histogram/Info panels | draft |
| Gradient presets | `gradient-presets.md` | CLR-010 | `pictura_core::gradient` | gradient editor | draft |
| Pattern presets | `pattern-presets.md` | CLR-011 | `pictura_core::pattern` | pattern picker | draft |
| Color sweeps/modes | `color-sweeps-and-modes.md` | CLR-012 | `pictura_color` | Color panel | draft |
| Brush engine | `brush-engine.md` | BRU-001 | `pictura_tools::brush` | Brushes panel | draft |
| Brush dynamics | `brush-dynamics.md` | BRU-002 | `pictura_tools::brush` | dynamic dialogs | draft |
| Bristle brushes | `bristle-brushes.md` | BRU-003 | `pictura_tools::brush` | tip previews | draft |
| Mixer brush engine | `mixer-brush-engine.md` | BRU-004 | `pictura_tools::brush` | options bar | draft |
| Airbrush/flow | `airbrush-and-flow.md` | BRU-005 | `pictura_tools::brush` | options bar | draft |
| Brush presets | `brush-presets.md` | BRU-006 | `pictura_tools::preset` | Brush Presets panel | draft |

## 08-selection

| CS6 feature | Spec file | ID | Rust module (proposed) | Qt6 component (proposed) | Status |
|---|---|---|---|---|---|
| Selection model | `selection-model.md` | SEL-001 | `pictura_core::selection` | canvas overlay | draft |
| Tools overview | `selection-tools-overview.md` | SEL-002 | `pictura_tools::select` | options bar | draft |
| Refine Edge | `refine-edge.md` | SEL-003 | `pictura_core::refine` | Refine Edge dialog | draft |
| Quick Mask | `quick-mask.md` | SEL-004 | `pictura_core::quickmask` | canvas | draft |
| Color Range | `color-range.md` | SEL-005 | `pictura_core::color_range` | dialog | draft |
| Grow/Similar/Modify | `grow-similar-and-modify.md` | SEL-010 | `pictura_core::selection` | menu | draft |
| Transform Selection | `transform-selection.md` | SEL-011 | `pictura_core::selection` | canvas | draft |
| Save/Load Selection | `save-and-load-selections.md` | SEL-012 | `pictura_core::channel` | menu | draft |
| Channel masking/Calc | `channel-based-masking.md` | SEL-013 | `pictura_core::channel` | Calculations dialog | draft |
| Paths ↔ selection | `paths-and-vector-selection.md` | SEL-014 | `pictura_core::path` | Paths panel | draft |

## 09-automation

| CS6 feature | Spec file | ID | Rust module (proposed) | Qt6 component (proposed) | Status |
|---|---|---|---|---|---|
| Actions | `actions.md` | AUTO-001 | `pictura_script::action` | Actions panel | draft |
| Droplets | `droplets.md` | AUTO-002 | `pictura_script::droplet` | `.desktop` launcher | draft |
| Batch/Automate | `batch-processing.md` | AUTO-003 | `pictura_script::batch` | Batch dialog | draft |
| Script events/JSX | `script-events-and-jsx.md` | AUTO-004 | `pictura_script` | — | draft |
| ExtendScript API surface | `extendscript-api-surface.md` | AUTO-005 | `pictura_script::compat` | — | draft |
| Rust scripting replacement | `rust-scripting-replacement.md` | AUTO-010 | `pictura_script` (rquickjs) | console | draft |
| Variables/data-driven | `variables-and-data-driven-graphics.md` | AUTO-011 | `pictura_core::dataset` | Variables dialog | draft |
| Plugin SDK | `plugin-sdk.md` | AUTO-012 | `pictura-plugin` | plugin manager | draft |
| Automation plugins | `automation-plugins.md` | AUTO-013 | `pictura-plugin` | plugin manager | draft |
| Mini Bridge | `mini-bridge.md` | AUTO-014 | — | non-goal; Files panel | draft |

## 10-workflow-io

| CS6 feature | Spec file | ID | Rust module (proposed) | Qt6 component (proposed) | Status |
|---|---|---|---|---|---|
| Document lifecycle | `document-lifecycle.md` | WF-001 | `pictura_app::doc` | document tabs | draft |
| Open/New/Import | `open-and-new.md` | WF-002 | `pictura-codec` | open/new dialogs | draft |
| Save/Save As | `save-and-save-as.md` | WF-003 | `pictura-io` | save dialogs | draft |
| Export formats | `export-formats.md` | WF-004 | `pictura-codec` | export dialog | draft |
| Save for Web/Slices | `web-export-and-slices.md` | WF-005 | `pictura-io::web` | SFW dialog | draft |
| File Info/metadata | `file-info-and-metadata.md` | WF-010 | `pictura-io::meta` (XMP) | File Info dialog | draft |
| Color Settings | `color-settings.md` | WF-011 | `pictura-color::settings` | dialog | draft |
| Camera Raw workflow | `camera-raw-workflow.md` | WF-012 | `pictura-codec::raw` | ACR workspace | draft |
| Printing | `printing.md` | WF-013 | `pictura-io::print` | `QPrinter`/CUPS | draft |
| Measurement/Count | `measurement-and-count.md` | WF-014 | `pictura_core::measure` | Measurement Log | draft |
| Presets manager | `presets-manager.md` | WF-020 | `pictura_app::preset` | Preset Manager | draft |
| Workspace management | `workspace-management.md` | WF-021 | `pictura_app::workspace` | `QSettings` | draft |
| Scratch/memory | `scratch-disks-and-memory.md` | WF-022 | `pictura_core::scratch` | Performance prefs | draft |
| Bridge/interop | `bridge-and-interop.md` | WF-023 | `pictura-io` | non-goals | draft |

## 11-cross-cutting

| Concern | Spec file | ID | Rust module (proposed) | Qt6 component (proposed) | Status |
|---|---|---|---|---|---|
| Localization | `localization.md` | XC-001 | `fluent` | `QTranslator` | draft |
| Preference storage | `preference-storage.md` | XC-002 | `pictura_app::prefs` (TOML) | `QSettings` | draft |
| Logging/telemetry | `logging-and-telemetry.md` | XC-003 | `tracing` | `QLoggingCategory` | draft |
| Error handling | `error-handling.md` | XC-004 | `thiserror`/`anyhow` | error dialogs | draft |
| Security/sandboxing | `security-and-sandboxing.md` | XC-005 | boundaries | Flatpak | draft |
| Testing strategy | `testing-strategy.md` | XC-010 | golden-image harness | — | draft |
| Versioning/updates | `update-and-versioning.md` | XC-011 | SemVer | Flatpak/AppImage | draft |
| Crash recovery/autosave | `crash-recovery-and-autosave.md` | XC-012 | `pictura_core::journal` | recovery UI | draft |
| Open questions | `open-questions.md` | XC-020 | — | — | draft |
| Gap analysis | `gap-analysis.md` | XC-021 | — | — | draft |

## Area index

| Area | Directory | Files |
|---|---|---|
| Overview | `00-overview/` | 4 |
| Architecture | `01-architecture/` | 13 |
| UI/UX | `02-ui-ux/` | 7 + 20 panels |
| Tools | `03-tools/` | 30 |
| Image operations | `04-image-ops/` | 8 + 26 adjustments |
| Layers | `05-layers/` | 17 |
| Filters | `06-filters/` | 23 |
| Color & painting | `07-color-painting/` | 13 |
| Selection & masking | `08-selection/` | 10 |
| Automation | `09-automation/` | 10 |
| Workflow & I/O | `10-workflow-io/` | 14 |
| Cross-cutting | `11-cross-cutting/` | 10 |
