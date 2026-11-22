# Docs ↔ code parity gaps

- **Status:** living register
- **Owner:** see `#486`
- **Source audit:** the 30-agent docs↔code parity audit summarised here; per-gap issues filed in `#488` (`#489`–`#651`)
- **Re-checked:** `#785` swept the rows this register's readers depend on and
  corrected five that the tree had moved past; the corrections are listed in
  §G and the rows that were wrong have been removed.
- **Re-checked:** `#1358` corrected five rows whose tracking issues had shipped
  since that sweep, narrowed `PAN-15`, and added the remaining Smart-Filter
  preview gap as `FILT-11`; the corrections are listed in §G.
- **Re-checked:** `#1522` reconciled `ROADMAP.md` and `PARITY.md` with the
  tree and updated stale register rows. The corrections are listed in §G.
- **Re-checked:** `#591` shipped `LAY-13` (Extended Stack Modes); the row keeps
  the implementation evidence.
- **Rows:** 159 gaps — 4 systemic in §B, 155 per-domain in §C.

This register maps the behavioural contract in `docs/` against the implementation
in `crates/`. It is the canonical place to see **what is specified but missing,
partial, or divergent**, and which GitHub issue tracks each gap. Aggregate
feature coverage is in [`PARITY.md`](../../PARITY.md).

`docs/TRACEABILITY.md` is *not* this document: every one of its rows is `draft`
and it answers "does a spec exist?", not "is it implemented?".

---

## How to read this

- **Status**
  - `MISSING` — no implementation.
  - `PARTIAL` — a stub, a subset, or a disabled control.
  - `DIVERGENT` — code contradicts the spec.
  - `UNRESOLVED` — spec too vague to confirm either way.
  - `STALE-DOC` — the doc is out of date; code is ahead.
  - `NON-GOAL` — declared absence, correct by design.
  - `SHIPPED` — implemented and evidenced; kept so a shipped roadmap item
    resolves here.
- **Evidence** is `path:line` in the current tree, or `none found`.
- **Issue** is the GitHub issue tracking the gap (`#489`–`#651` were filed from
  this register; existing issues are linked where they already covered a gap).

---

## A. Repo and process findings

1. **OpenSpec archive backlog.** Of 119 non-archived changes under
   `openspec/changes/`, 116 had all tasks `[x]` (complete but unarchived) and 3
   were in progress; ~53 of the complete changes modify a capability that already
   exists in `openspec/specs/`, so those specs are stale. Tracked in `#487`.
2. **Specs that are not an implementation matrix.** `docs/TRACEABILITY.md`
   (`draft` everywhere), the phantom `01-architecture/build-and-packaging.md`
   (`ARCH-014`, referenced by TRACEABILITY and 8 other docs, never written),
   and the skipped `ARCH-012` id. Tracked in `#494`.
3. **Architecture drift.** The docs propose `pictura-qt`, `pictura-script`,
   `pictura-plugin`, `pictura-history`, `pictura-io`, `pictura-ui`, `pictura-ffi`
   (never created), omit the real `pictura-adjust`/`ops`/`paint`/`select`/`testkit`
   crates, and still tag shipped subsystems "(planned)". Tracked in `#495`.
4. **Stale "no source code" claims.** `00-overview/product-overview.md` and
   `07-color-painting/color-models.md` still assert the repository contains no
   source code. Tracked in `#496`; **fixed by `#785`**, which corrected both
   files and swept the same claim out of the rest of the corpus.

## B. Systemic gaps (cross-cutting)

| ID | Gap | Evidence | Issue |
|---|---|---|---|
| SYS-1 | Preset/library codecs + Preset Manager (`.aco/.ase/.grd/.abr/.pat/.csh/.asl/.tpl/.csf`) — no `pictura-presets` crate exists | no `crates/pictura-presets`; `swatches_panel.h:71` | #489 |
| SYS-2 | Global bit-depth / colour-mode availability gating — every "unavailable at 16/32-bpc, Bitmap, Indexed, CMYK/Lab" rule is unenforced | `filter_tools.rs` (`filter_kind_supported`) checks arity only; no `Edition`/depth gate | #490 |
| SYS-4 | Brush dynamics: Texture and Dual Brush remain; the Control model, Shape Dynamics, Scattering, Color Dynamics, Transfer, Brush Pose, and per-set padlocks ship | `brush_panel.cpp` leaves Texture and Dual Brush disabled | #492 |
| SYS-5 | Sampled / erodible / airbrush tips + Define Brush Preset (bristle tips now ship) | `tip.rs` round/elliptical only; `bristle.rs` | #493 |

**Provenance note.** All five were landed as behavioural `photorust` ports, not
CS6-faithful; the ceilings are marked in-source with `ponytail:` comments.

## C. Per-domain register

### 00-overview & 01-architecture

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| ARCH-1 | `01-architecture/build-and-packaging.md` | Spec file never authored (`ARCH-014`); dangling refs | MISSING | `TRACEABILITY.md:37` + 8 refs | #494 |
| ARCH-2 | `01-architecture/*` | Proposed crate map diverges from the real workspace | DIVERGENT | `Cargo.toml`, `crates/` | #495 |
| OVR-2 | `11-cross-cutting/cs6-editions-and-constraints.md` | `Edition` (Standard/Extended) gating | SHIPPED | `pictura_core::Edition`, `PICTURA_EDITION`; Count/3D tool and command gate | #650 |

### 02-ui-ux — application frame and menus

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| UI-1 | `application-frame.md` | Status bar readouts + status-bar options popup | MISSING | `frame.cpp:514` | #497 |
| UI-2 | `application-frame.md` | Active layer name in the title bar | MISSING | `frame.cpp:1006` | #498 |
| UI-5 | `menus.md` | Menu customization (Edit ▸ Menus, Show All Menu Items, enablement matrix) | MISSING | `crates/pictura-script/src/command_catalogue/entries.rs` | #501 |
| UI-6 | `menus.md` | Type menu wired: work path, convert to shape, option rows, find/replace; Check Spelling split to #1443 and Google Fonts to #1442 | SHIPPED | `crates/pictura-app/cpp/menu_type.cpp` | #502 |
| UI-7 | `menus.md` | View menu stubs (Gamut Warning, Snap To, Lock Slices, Extras, Pixel Aspect Ratio, 32-bit Preview Options); Extras/Show, grid, pixel grid, layer edges, target path, Lock/Clear Slices and the zoom presets shipped | PARTIAL | `frame_view.cpp`; `image_view_grid.cpp`; `crates/pictura-script/src/command_catalogue/entries.rs` | #503 |
| UI-8 | `menus.md` | `File > Browse…` (the Linux Bridge replacement) and `File > Scripts > Statistics…` shipped; `File > Scripts > Browse…` is disabled until #620; `Check In…` and `Import ▸ WIA Support` are platform-only; the remaining Automate/Scripts leaves stay with their feature issues | PARTIAL | `crates/pictura-app/cpp/frame_commands.cpp`; `crates/pictura-app/cpp/statistics_dialog.cpp`; `crates/pictura-script/src/command_catalogue/entries.rs` | #504, #558, #617, #619, #620, #630 |
| UI-9 | `preferences.md` | 5 missing panes (Cursors, Transparency & Gamut, Plug-ins, Type, 3D) | MISSING | `preferences_dialog.cpp:35-47` | #505 |
| UI-10 | `preferences.md` | Sparse existing panes (General/Interface/File Handling/Performance/Units/Guides) | PARTIAL | `preferences_dialog.cpp:113-177` | #506 |
| UI-11 | `accessibility.md` | AT-SPI2 / `QAccessible`, high-contrast, colour-blind, reduced-motion | MISSING | no `QAccessible` in `crates/` | #507 |
| UI-12 | `keyboard-shortcuts.md` | Shortcut sets (save/load, workspace-assigned, Summarize HTML, Legacy Channel Shortcuts) | PARTIAL | `frame_shortcuts.cpp:20`; `keyboard_shortcuts_dialog.cpp:249-282` | #508 |
| UI-13 | `color-sweeps-and-modes.md` | Quick fill with Alt+Backspace / Ctrl+Backspace | MISSING | `frame_menus_edit.cpp:307-310` | #524 |
| UI-14 | `application-frame.md:241`, `:297` | On Wayland a floating document's header drag and a tab tear-out are a drag-and-drop that carries the window (`xdg_toplevel_drag_v1`), so the compositor moves it with the pointer and a drop on the strip docks it; a compositor without that protocol leaves the window in place until the drop, and drop-to-dock and a header double-click still dock | DIVERGENT | `document_float_window.cpp` (`movesByWindowDrag`); `frame_document_menus.cpp` (`startWindowDockDrag`, `windowDropFilter`) | #317 |
| UI-15 | `application-frame.md:221`, `:285`, `:337` | Floating-window geometry and float state are not saved per workspace or session; relaunch and crash recovery reopen every document as a tab | DIVERGENT | `session.h` has no float or tile state; `frame_recovery.cpp` | #317 |
| UI-16 | `preferences.md:108`, `application-frame.md:71` | The doc names the Interface option "Enable Floating Document Window"; the CS6 label, used by the code, is "Enable Floating Document Window Docking" | STALE-DOC | `preferences_dialog.cpp:149` | #317 |

### 02-ui-ux — panels

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| PAN-1 | `panels/adjustments-panel.md` | Adjustments panel icon grid (creation shipped; mode/depth gating and fill-layer presets remain) | PARTIAL | `panels/adjustments_panel.cpp` | #509, #1521 |
| PAN-2 | `panels/measurement-log-panel.md` | Measurement Log panel, Record Measurements, and its export workflow | MISSING | `crates/pictura-script/src/command_catalogue/entries.rs` | #510 |
| PAN-3 | `panels/tool-presets-panel.md` | Tool Presets panel + `.tpl` libraries | MISSING | `crates/pictura-script/src/command_catalogue/entries.rs` | #511 |
| PAN-4 | `layer-styles.md` | Styles panel + `.asl` presets | MISSING | `frame_build.cpp:79` | #512 |
| PAN-5 | `panels/channels-panel.md` | Spot channels, Split/Merge, and Name/Color/Panel Options (alpha Masked/Selected Areas shipped) | PARTIAL | `panel_group_menu.cpp:90-98`; `frame_selection_dialogs.cpp` (Channel Options) | #513 |
| PAN-6 | `panels/history-panel.md` | History Options, Clear History/purge, New Document from state | MISSING | `panel_group_menu.cpp:195-198` | #514 |
| PAN-7 | `panels/histogram-panel.md` | View modes, statistics, source menu, cache/refresh, 16/32-bit bins | PARTIAL | `histogram_panel.cpp:79-142` | #515 |
| PAN-8 | `panels/info-panel.md` | Sampler persistence, sample-size readout, marquee/transform readouts, before/after, gamut exclamation | PARTIAL | `info_panel.cpp:623-685` | #516 |
| PAN-12 | `panels/navigator-panel.md` | Proxy colour, Ctrl-drag proxy positioning, zoom buttons, typed zoom | MISSING | `navigator_panel.cpp:138-159` | #520 |
| PAN-13 | `panels/paths-panel.md` | Make Work Path, clipping-path designation, Panel Options thumbnail size | MISSING | `paths_panel.h:24-25` | #521 |
| PAN-14 | `panels/note-panel.md` | Notes Author/Colour options + PSD/PDF/TIFF annotation round-trip | MISSING | `options_bar.cpp:593`; `annotations.rs:10` | #522 |
| PAN-15 | `panels/layers-panel.md` | `00`/`Shift+00` layer opacity shortcut (the Panel Options thumbnail size shipped in #586) | MISSING | `crates/pictura-app/cpp/panels/layers_panel_events.cpp` | #523 |

### 03-tools

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| TOOL-1 | `marquee-selection.md` | Single Row / Single Column marquee tools | MISSING | no `ToolId`/registration | #525 |
| TOOL-2 | `transform-selection.md` | Select ▸ Transform Selection | SHIPPED | `crates/pictura-app/cpp/frame_menus_select.cpp`; `crates/pictura-render/src/document_ops/layer_ops/selection_transform.rs` | #526 |
| TOOL-3 | `move-and-transform.md` | Puppet Warp | MISSING | `crates/pictura-script/src/command_catalogue/entries.rs` | #527 |
| TOOL-4 | `move-and-transform.md` | Content-Aware Scale | MISSING | `crates/pictura-script/src/command_catalogue/entries.rs` | #528 |
| TOOL-5 | `move-and-transform.md` | Transform Scale/Rotate/Again + exact rotations + flip | SHIPPED | `crates/pictura-script/src/command_catalogue/entries.rs:65-76` | #529 |
| TOOL-6 | `move-and-transform.md` | Transform Path / Transform Points submenus | MISSING | `crates/pictura-script/src/command_catalogue/entries.rs` | #530 |
| TOOL-7 | `pen-and-path-tools.md` | Magnetic Pen | MISSING | `options_bar_pen.cpp:19` | #531 |
| TOOL-8 | `merge-and-flatten.md` | Merge Shape Components (path boolean ops) | MISSING | no path-boolean in `pictura-core/src/path*` | #532 |
| TOOL-9 | `hand-and-zoom.md` | Hand/Zoom options bar: Scrubby and Animated Zoom; the multi-window options (Scroll All, Zoom All, Resize Windows To Fit) landed in #500 | PARTIAL | `options_bar_navigate.cpp` | #533 |
| TOOL-10 | `hand-and-zoom.md` | Bird's Eye View + Flick Panning | MISSING | none found | #534 |
| TOOL-11 | `eyedropper-color-sampler-ruler.md` | Ruler protractor + Straighten from the ruler | MISSING | `tool_ruler.cpp:37` | #535 |
| TOOL-12 | `eyedropper-color-sampler-ruler.md` | Eyedropper modifiers (Alt→background, temporary) + Shift-click sampler | MISSING | `tool_eyedropper.cpp:23-36` | #536 |
| TOOL-13 | `clone-stamp-and-pattern-stamp.md` | Clone Source overlay controls | MISSING | `clone_source_panel.h` | #537 |
| TOOL-14 | `healing-brushes.md` | Healing Mode/Source/Sample, Sample All Layers, depth/mode handling | PARTIAL | `options_bar.cpp:694-741`; `tool_healing.cpp:165` | #538 |
| TOOL-15 | `history-brush.md` | Merged/composite source + Edit ▸ Fill ▸ History | PARTIAL | `paint_tools.rs:524-527` | #539 |
| TOOL-16 | `mixer-brush.md` | Load Solid Colors Only, keyboard shortcuts | MISSING | `tool_mixerbrush.cpp:39-41` | #540 |
| TOOL-18 | `path-selection-tools.md` | Path operations/align/arrange, constrain path dragging, Target Path toggle, segment/marquee multi-select | MISSING | `tool_path_selection.cpp:13-14` | #542 |
| TOOL-19 | `slice-tools.md` | Slice options bar/style/modifiers, Divide/Combine/Duplicate, Slice Options dialog, slices-from-guides | MISSING | `tool_slice.cpp:65-102` | #543 |
| TOOL-20 | `crop-tool.md` | Crop resampling by W×H×resolution, non-destructive crop/Reveal All, crop shield settings | SHIPPED | `options_bar_crop.cpp`; `tool_crop.cpp`; `crop_group/imp.rs`; `crop.rs`; `menu_image.cpp`; `tst_crop_options`; `tst_image_ops` | #544 |
| TOOL-21 | `custom-shape.md` | `.CSH` libraries, Define Custom Shape, geometry modes, gradient/pattern fill-stroke | MISSING | `options_bar_shape.cpp:506-510,420-431` | #545 |
| TOOL-22 | `shape-tools.md` | Gradient/pattern fill+stroke, dashed stroke, path ops, fill-pixels options, snap-to-pixels, proportional geometry | SHIPPED | `options_bar_shape.cpp`; `shape_style.rs`; `layer_effects/strokes.rs`; `tool_shape.cpp`; `tst_shape_tools` | #546 |
| TOOL-23 | `color-replacement.md` | Full CS6 mode list, 32-bit subset, and Bitmap/Indexed/Multichannel availability gates | SHIPPED | `crates/pictura-app/cpp/tool_colorreplacement.cpp`; `crates/pictura-app/cpp/tool_catalog.cpp`; `tst_color_replacement` | #547 |
| TOOL-24 | `dodge-burn-sponge.md` | Grayscale Sponge, airbrush toggle, quick keys, 32-bpc availability | SHIPPED | `tone.rs`; `tool_retouch.cpp`; `options_bar_retouch.cpp`; `toolbox.cpp` | #548 |
| TOOL-25 | `eraser-tools.md` | Magic-eraser background convert, background-eraser edge halo, pen pressure | PARTIAL | `eraser.rs:166-332` | #549 |
| TOOL-26 | `gradient-and-paint-bucket.md` | Gradient/Bucket availability gates and the Gradient Fill dialog | PARTIAL | `tool_fills.cpp`; `frame_menus.cpp:719-725` | #550 |
| TOOL-27 | `smudge-blur-sharpen.md` | HDR (16/32-bpc) operation | DIVERGENT | `stroke.rs:395` rejects non-8-bit | #551 |
| TOOL-28 | `clone-stamp-and-pattern-stamp.md` | Cross-document colour-mode rule + Pattern Stamp Impressionist | MISSING | `paint_tools.rs:451`; `tool_stamps.cpp` | #552 |
| TOOL-29 | `hand-and-zoom.md:76` | `Match All` + `Shift`+click/drag is approximated by the `Shift` forms of Scroll All (Hand drag, location) and Zoom All (Zoom click, zoom) rather than one combined match | DIVERGENT | `frame_document_arrange.cpp:173`; `tool_hand_zoom.cpp:59` | #500 |
| TOOL-30 | `brush-and-pencil.md`; `airbrush-and-flow.md` | Brush Airbrush option and `Shift+Alt+P`; held dabs on the exact, preview, GPU and mask backends; 20 dabs/s approximated | SHIPPED | `tool_brush.cpp`; `tool_stamps.cpp`; `impl_paint.rs`. Ceiling: 20 dabs/s approximated; held dabs are not recorded in actions | #1120 |

### 04-image-ops and adjustments

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| IMG-1 | `image-modes.md`, `duotone.md` | Duotone conversion + Options dialog | MISSING | `crates/pictura-script/src/command_catalogue/entries.rs`; `mode/mod.rs:72` | #553 |
| IMG-2 | `image-modes.md` | Multichannel conversion | MISSING | `mode/mod.rs:72` | #554 |
| IMG-3 | `image-modes.md`, `indexed-color.md` | Color Table + Indexed transparency/Forced palette | MISSING | `crates/pictura-script/src/command_catalogue/entries.rs`; `indexed_color_dialog.h:16` | #555 |
| IMG-4 | `image-modes.md` | Bitmap Halftone Screen + Custom Pattern | MISSING | `mode/flat.rs:20,185` | #556 |
| IMG-5 | `bit-depth-and-conversion.md`, `hdr-toning.md` | HDR Conversion tone methods implemented; the Local Adaptation Toning Curve/Corner remains | PARTIAL | `hdr_conversion_dialog.cpp`; `pictura-adjust/src/hdr_toning/` | #557 |
| IMG-6 | `32-bit-hdr.md` | Merge To HDR Pro | MISSING | `crates/pictura-script/src/command_catalogue/entries.rs` | #558 |
| IMG-7 | `auto-adjustments.md` | Auto Color Correction Options dialog | MISSING | `panel_group_menu.cpp:249` | #559 |
| IMG-8 | `adjustments/levels.md` | Levels eyedroppers + Alt-drag clipping preview | MISSING | `levels_dialog.cpp` | #560 |
| IMG-9 | `adjustments/curves.md` | Curves secondary UI (on-image, pencil/smooth, sliders+eyedroppers, display options, CMYK/Lab graphs) | PARTIAL | `curves_dialog.h`; `panel_group_menu.cpp` (Auto shipped in #578) | #561 |
| IMG-10 | `adjustments/shadow-highlight.md` | Shadows/Highlights local-surround operator (8 controls) | MISSING | `tonal.rs:155-159` | #562 |
| IMG-11 | `adjustments/threshold.md` | Threshold histogram under the slider | MISSING | `adjustment_params.rs:448-454` | #563 |
| IMG-12 | `adjustments/vibrance.md` | Vibrance Auto + preset | MISSING | `adjustment_dialog.cpp:446-452` | #564 |
| IMG-13 | `image-size.md` | Smoother/Sharper/Automatic kernels, Scale Styles, Auto resolution, picas/columns, float resample | DIVERGENT | `resize.rs:13`; `image_size_dialog.cpp:70` | #565 |
| IMG-14 | `canvas-size.md` | picas/columns units + extension-fill availability gating | PARTIAL | `canvas_size_dialog.cpp:256,274` | #566 |
| IMG-15 | `image-rotation-and-flip.md` | Arbitrary rotation dialog, interpolation preference, ruler-straighten prefill | DIVERGENT | `frame_menus_image.cpp:115` | #567 |
| IMG-17 | `color-settings.md` | `.csf` save/load, Gray/Spot working spaces, and installed-profile discovery | MISSING | `color_settings_dialog.cpp` | #569 |
| IMG-18 | `32-bit-hdr.md` | 32-bit Preview Options, HDR colour picker, 32-bit Info readout | MISSING | `info_panel.cpp:377` | #570 |
| ADJ-3 | `adjustments/hue-saturation.md` | Colorize, range-band editing, eyedroppers, on-image tool | MISSING | `hue_saturation_dialog.h:17-21` | #573 |
| ADJ-4 | `adjustments/gradient-map.md` | Dither + CS6 print-toning presets | MISSING | `gradient_map_dialog.cpp:82-91`; `gradient.rs:295-310` | #574 |
| ADJ-5 | `adjustments/match-color.md` | On-canvas eyedropper + live Info feedback | MISSING | `match_color_dialog.cpp` | #575 |
| ADJ-7 | `adjustments/selective-color.md` | Composite-channel-only gate + preset whitelist | MISSING | no channel gate | #577 |
| ADJ-8 | `adjustments/brightness-contrast.md` | Brightness/Contrast Auto + shared adjustment Auto buttons | SHIPPED | `adjustment_dialog.cpp` (`autoEndpoints`, `applyAuto`); `slider_dialog.cpp`; `curves_dialog.cpp`; `tst_image_adjustments` | #578 |

### 05-layers

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| LAY-1 | `blend-modes.md` | Advanced Blending toggles (Blend Clipped, Transparency Shapes, mask hides effects, channel Include; Blend Interior) | PARTIAL | engine: `crates/pictura-render/src/composite_channels.rs`, `composite_clipping.rs`, `composite.rs`, `layer_effects/*`; dialog controls pending #695/#581 | #579, #695 |
| LAY-2 | `layer-styles.md` | Global Light dialog, Create Layers, Rasterize Layer Style | SHIPPED | `crates/pictura-app/cpp/menu_layer_style.cpp`; `crates/pictura-render/src/document_ops/layer_ops/layer_style.rs` | #580 |
| LAY-3 | `layer-styles.md` | Bevel Contour and the style dialog's gradient editor; Bevel Texture remains | PARTIAL | `crates/pictura-render/src/layer_effects/contour.rs`; `crates/pictura-app/cpp/layer_style_contour.cpp`; `tst_layer_style` | #581, #606 |
| LAY-4 | `align-and-distribute.md` | Auto-Align Layers | SHIPPED | `crates/pictura-render/src/document_ops/layer_ops/auto_align/`; `crates/pictura-app/cpp/auto_align_dialog.cpp`; `tst_auto_align`. Behavioral parity only: the matcher and projection tolerances are our own | #582 |
| LAY-6 | `blend-modes.md` | `Fill With (Mode)-Neutral Color` for new layers | SHIPPED | `crates/pictura-app/cpp/layer_new_dialog.cpp:49`; `crates/pictura-render/src/document_ops/layer_ops/create.rs:317` | #584 |
| LAY-8 | `layer-management-ui.md` | Panel Options thumbnail size (Entire Document/Bounds) | SHIPPED | `crates/pictura-app/cpp/panels/layers_panel.cpp:1053` | #586 |
| LAY-9 | `export-formats.md` | File ▸ Scripts ▸ Export Layers To Files — core shipped; missing the shared Save As option controls and Targa | PARTIAL | `crates/pictura-app/cpp/export_layers_dialog.cpp`; `crates/pictura-app/src/cxxqt_object/export_tools/imp.rs` | #587, #625 |
| LAY-10 | `layer-management-ui.md` | Shape layer naming `<Tool> N` | SHIPPED | `crates/pictura-render/src/document_ops/layer_ops/shape_layer.rs:17`; `crates/pictura-app/cpp/tests/tst_shape_tools.cpp:258` | #588 |
| LAY-11 | `blend-modes.md` | 32-bit/Lab blend-mode menu gating | SHIPPED | `crates/pictura-core/src/blend.rs` (`BlendMode::allowed`); `crates/pictura-app/cpp/blend_modes.cpp`; `crates/pictura-app/cpp/tests/tst_blend_mode_list.cpp` | #589 |
| LAY-12 | `layer-management-ui.md` | `Ctrl+J` duplicate-group semantics | SHIPPED | `crates/pictura-app/cpp/menu_registrar.cpp:555`; `crates/pictura-app/cpp/tests/tst_layer_commands.cpp:212` | #590 |
| LAY-13 | `smart-objects.md` | Extended Stack Modes | SHIPPED | `crates/pictura-core/src/stack_mode.rs`; `crates/pictura-render/src/layer_source/stack.rs`; `crates/pictura-codec/src/stack_marker.rs`; `crates/pictura-app/cpp/tests/tst_smart_object_actions.cpp` | #591 |
| LAY-14 | `merge-and-flatten.md` | Merge Down result appearance: the merged node inherits the target's blend mode and opacity, so a target with non-default settings renders the merged result differently from the pre-merge canvas; the pair also composites against a transparent backdrop, so where the target is non-`Normal` or not opaque the source's blend sees the target's unblended colour | UNRESOLVED | `merge.rs:195`; `merge.rs:202-203`; `merge.rs:422-424` | #813 |
| LAY-15 | `smart-filters.md` | Filter-menu filters on a Smart Object append Smart Filters (#984); missing: Shadow/Highlight and Variations as Smart Filters, per-filter blend mode and opacity, the white mask without a selection, Smart Lens Blur/Displace/Adaptive Wide Angle, Photoshop-validated `FEid`/`FMsk` | PARTIAL | `crates/pictura-render/src/document_ops/smart_filter_apply.rs`; `crates/pictura-app/src/cxxqt_object/filter_tools/smart_apply.rs` | #996, #998, #1000, #1001, #999 |

| LAY-22 | `layer-comps.md` | Layer Comps panel, capture/apply/update/delete, Last Document State, caution warnings, resource 1065 and `shmd`/`cmls` persistence, Files/PDF scripts. Omitted: drag duplicate/delete, recordable Actions commands, WPG, full O(captured-entries) apply | SHIPPED | `crates/pictura-app/cpp/panels/layer_comps_panel.cpp`; `crates/pictura-codec/src/layer_comps.rs`; `crates/pictura-render/src/document_ops/layer_ops/layer_comps.rs` | #265 |

### 06-filters

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| FILT-2 | `noise-filters.md` | Reduce Noise (incl. Advanced per-channel) | SHIPPED | `crates/pictura-app/cpp/reduce_noise_dialog.cpp:32`; `crates/pictura-filters/src/noise/reduce.rs:36` | #593 |
| FILT-4 | `sharpen-filters.md` | Smart Sharpen Basic/Advanced split + 32-bpc disable | DIVERGENT | `crates/pictura-filters/src/catalogue/rows/render.rs` (Smart Sharpen row) | #595 |
| FILT-5 | `stylize-filters.md` | 8 Stylize filters missing from the Filter Gallery category | SHIPPED | `crates/pictura-filters/src/catalogue/rows/stylize.rs` (gallery ranks 40–48); `crates/pictura-app/cpp/tests/tst_filter_gallery.cpp` | #596 |
| FILT-6 | `artistic-filters.md` | Colored Pencil document-background paper wiring | PARTIAL | `crates/pictura-filters/src/catalogue/build/paint.rs` (Colored Pencil arm) | #597 |
| FILT-7 | `adaptive-wide-angle.md` | Adaptive Wide Angle Auto / lens profiles, Panorama, Smart Filter re-edit, rotation handles | PARTIAL | `pictura-filters/src/distort/adaptive_wide_angle/`, `awa_dialog.cpp` | #891 |
| FILT-9 | `lens-correction.md` | Lens Correction Save/Load Settings, Set Lens Default, and Alt-drag fringe isolation | MISSING | `lens_correction_panels.cpp` (Settings menu) | #933 |
| FILT-11 | `filters-overview.md` | Filter Gallery canvas and Shear below-input previews over an existing Smart Filter chain | PARTIAL | `crates/pictura-app/src/cxxqt_object/filter_tools/imp.rs:492`; `crates/pictura-app/cpp/filter_preview_dialog.cpp:566` | #1003 |

De-Interlace (`#344`) and NTSC Colors (`#345`) shipped and are no longer gaps.
Embed/Read Watermark (`#350`/`#351`) shipped as an in-tree approximation
(`06-filters/digimarc-watermark.md`), so it is no longer a gap.

### 07-color-painting

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| CLR-2 | `pattern-presets.md` | Define Pattern, `.pat` library, Scripted Patterns | MISSING | `crates/pictura-script/src/command_catalogue/entries.rs`; `codec/patterns.rs` | #599 |
| CLR-3 | `color-picker.md` | Color Libraries (PANTONE) | MISSING | `color_picker_dialog.cpp` libraries disabled | #600 |
| CLR-4 | `color-picker.md` | HUD colour picker | MISSING | none found | #601 |
| CLR-5 | `color-picker.md` | HDR colour picker (32-bpc) | MISSING | `color_picker_dialog.cpp` 8-bit only | #602 |
| CLR-7 | `color-models.md` | Out-of-gamut triangle, 16/32-bit picker fields (CMYK now goes through the working profile in `pictura_color::model`) | PARTIAL | `color_picker_dialog.cpp:714` | #604 |
| CLR-8 | `swatches-and-libraries.md` | `.aco`/`.ase` libraries, rename, display modes | MISSING | `swatches_panel.h:74` | #605 |
| CLR-9 | `pattern-presets.md` | Pattern Overlay Snap To Origin + style Texture sub-options | PARTIAL | `layer_style_pages.cpp:297-315` | #606 |
| CLR-10 | `bristle-brushes.md` | Bristle tip engine, tracks, rasterizer, per-bristle Mixer reservoir, Brush panel controls, and presets ship; OpenGL rotatable preview, `.abr` bristle keys, and tilt-driven projection remain | PARTIAL | `crates/pictura-paint/src/bristle.rs`; `panels/brush_panel.cpp` | #607, #1520 |
| CLR-11 | `brush-presets.md` | `.abr` libraries, Brush Presets panel, ABR v10 read | MISSING | `brush_panel.cpp:137-139` | #608 |

### 08-selection

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| SEL-1 | `channel-based-masking.md` | Apply Image grows a smaller destination to the document when Preserve Transparency is off (#1139) | SHIPPED | `crates/pictura-app/src/cxxqt_object/impl_filters.rs`; `crates/pictura-render/src/apply_image.rs` | #1139 |
| SEL-2 | `channel-based-masking.md` | Calculations | SHIPPED | `crates/pictura-render/src/calculations.rs`; `crates/pictura-app/cpp/calculations_dialog.cpp`; `crates/pictura-app/src/cxxqt_object/image_apply_image/calculations.rs` | #610 |
| SEL-3 | `channel-based-masking.md` | Channel Options (Masked/Selected Areas) | SHIPPED | `crates/pictura-codec/src/image_resources.rs` (DisplayInfo 1077 sense); `crates/pictura-app/src/cxxqt_object/impl_selection/save_load.rs`; `crates/pictura-app/src/cxxqt_object/channels/imp.rs`; `crates/pictura-app/cpp/frame_selection_dialogs.cpp`; `tst_channel_options`. Ceiling: DisplayInfo kind-label direction pending the Photoshop CC/CS6 check (task 5.3) | #611 |
| SEL-4 | `refine-edge.md` | Interactive Refine Edge / Restore Original brushes, Show Edge, CS6/CC semantics | SHIPPED | `crates/pictura-select/src/refine.rs` (`refine_from(..., dabs)`, `edge_band_from`); `crates/pictura-select/src/refine/refine_brush.rs` (`RefineDab`, the brush field); `refine_edge_dialog.cpp`; `tst_refine_edge`; `tst_layers_mask_commands`. Ceiling: the estimator is a gradient nudge, not Adobe matting, so strand recovery is not claimed; the CC workspace, Onion Skin, mask Brush, and Refine Hair remain deferred. | #612 |
| SEL-5 | `color-range.md` | Skin Tones, Localized Clusters, Out Of Gamut, previews, samplers, and save/load ship; ceilings: Detect Faces, Alt-click Reset, `.axt` import, Bitmap/Indexed availability, Grayscale behaviour, Properties-panel mask refinement, the no-match warning, and per-session persistence | SHIPPED | `crates/pictura-select/src/color_range.rs`; `crates/pictura-app/src/cxxqt_object/color_range/`; `crates/pictura-app/cpp/color_range_dialog.cpp`; `crates/pictura-color/src/proof.rs`; `tst_color_range` | #613 |
| SEL-6 | `selection-model.md` | Tool-path anti-aliased rasterisation (ellipse, lassos, wand ramp) + 56-channel cap | SHIPPED | `pictura-select/src/lib.rs` (`ellipse_aa`, `polygon_aa`), `wand.rs` (`soften_boundary`), `pictura-core/src/channels.rs`; control-server selection ops remain binary | #614 |
| SEL-7 | `save-and-load-selections.md` | Combine ops, layer-mask target, channel modifier-click combine, and channel names | SHIPPED | `crates/pictura-app/src/cxxqt_object/impl_selection.rs`; `crates/pictura-app/cpp/save_load_selection_dialog.cpp`; `crates/pictura-app/cpp/channels_rows.h`; `crates/pictura-app/cpp/menu_select.cpp`; `crates/pictura-codec/src/image_resources.rs` (alpha-name resources 1006/1045); `tst_selection_save_load`. Ceiling: 16/32-bit channels (#1331); the Load dialog lists colour rows (#1333), while Save-to-colour is excluded by decision (a colour row is a derived view with no destination-layer rule). | #615 |
| SEL-8 | `save-and-load-selections.md` | Cross-image channel duplication and selection-outline drag, with an explicit fit-and-centre resize policy | SHIPPED | `crates/pictura-app/cpp/duplicate_channel_dialog.cpp`; `crates/pictura-app/cpp/menu_panel_commands.cpp`; `crates/pictura-app/cpp/frame_selection_dialogs.cpp`; `crates/pictura-app/cpp/file_drop_router.cpp`; `pictura-select/src/lib.rs` (`fit_centered`); `tst_channels_panel`; `tst_selection_save_load` | #1332 |

Quick Mask is tracked by `#172`.

### 09-automation

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| AUTO-1 | `actions.md` + `script-events-and-jsx.md` | Shared ActionDescriptor / command-recording bus (foundation) | SHIPPED | `crates/pictura-script/src/command_catalogue/`; `crates/pictura-app/cpp/commands.cpp`; `crates/pictura-codec/src/action_descriptor.rs` | #616 |
| AUTO-2 | `droplets.md` | Create Droplet, `.opd`, `.desktop` launcher | MISSING | `crates/pictura-script/src/command_catalogue/entries.rs` | #617 |
| AUTO-3 | `batch-processing.md` | File ▸ Automate ▸ Batch engine + dialog (Folder and Opened Files sources; Import/Bridge sources deferred to #1406) | SHIPPED | `crates/pictura-script/src/batch/`; `crates/pictura-app/cpp/batch_runner.cpp`; `crates/pictura-app/cpp/batch_dialog.cpp` | #618 |
| AUTO-4 | `batch-processing.md` | Remaining Automate commands (Image Processor, Conditional Mode Change, Crop And Straighten Photos, PDF Presentation, Contact Sheet II, Picture Package, Photomerge, Web Photo Gallery) | MISSING | `crates/pictura-script/src/command_catalogue/entries.rs` | #619 |
| AUTO-5 | `rust-scripting-replacement.md` | Scripting host, DOM, ActionManager, notifiers (QuickJS) | MISSING | `crates/pictura-script` covers action automation, not a scripting host/DOM | #620 |
| AUTO-6 | `rust-scripting-replacement.md` | Script console, sandbox limits, security prompt, progress bridge | MISSING | none found | #621 |
| AUTO-7 | `variables-and-data-driven-graphics.md` | Variables / data-driven graphics | MISSING | `crates/pictura-script/src/command_catalogue/entries.rs` | #622 |
| AUTO-8 | `plugin-sdk.md` | Rust C-ABI plugin loader + Plug-in manager dialog | MISSING | no `pictura-plugin` | #623 |
| AUTO-9 | `mini-bridge.md` | Mini-Bridge replacement Files panel | MISSING | none found | #624 |

The Actions panel and engine shipped in `#301` (`crates/pictura-app/cpp/panels/actions_panel.*`, `crates/pictura-app/cpp/panels/actions_tree_model.*`, `crates/pictura-app/src/cxxqt_object/actions_engine.rs`, `crates/pictura-script`). The command-recording coverage matrix lives with the catalogue: every implemented id is classified **Adobe-verified**, **app-native** (`pictura.<id>`), or **non-recordable**, and the catalogue test fails on an unclassified id. Tool strokes, conditional steps, and Insert Path are app-native `TEXT` events, not Adobe-interoperable; a `.atn` produced with them preserves and replays in Kooka Pictura but is not claimed as CS6-compatible. Brush/Pencil strokes replay; native tool-specific and mask strokes are refused until replay paths land. Unknown imported event identifiers are preserved and reported at playback; an unknown descriptor type fails closed.

### 10-workflow-io

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| WF-1 | `save-and-save-as.md` | Save As dialogs and engine writers shipped for JPEG/PNG/BMP/TIFF/GIF/EPS/DCS/Raw/TGA/Cineon, including BMP format/depth/RLE/flip, PNG interlace, TIFF pixel/byte order, pyramid and layer compression; remaining ceilings are the documented DCS process-ink-only and 8-bit Raw/Cineon cases | DONE | `crates/pictura-codec/src/save/`; `crates/pictura-app/cpp/gif_options_dialog.cpp`; `eps_options_dialog.cpp`; `dcs_options_dialog.cpp`; `raw_options_dialog.cpp`; `tga_options_dialog.cpp`; `cineon_options_dialog.cpp` | #958, #625 |
| WF-2 | `save-and-save-as.md` | Photoshop PDF writer shipped (#626); JPEG 2000, Spot Colors, and Use Proof Setup landed; Preserve Photoshop Editing Capabilities still embeds Pictura's own PSD, not Adobe's layout | PARTIAL | `pictura-codec/src/pdf/options.rs`; `pdf_save_dialog.cpp` | #626, #1513 |
| WF-3 | `open-and-new.md` | Generic PDF/EPS import (Import PDF and Rasterize Generic EPS Format dialogs), Open As, native CMYK/16-bit rendering, Images mode on encrypted PDFs, and inline images shipped (#627) through Ghostscript; the 3D option remains out of scope (see ROADMAP not-planned) | SHIPPED | `pictura-codec/src/pdf/import/mod.rs`; `import_pdf_dialog.cpp`; `frame_import.cpp` | #627 |
| WF-4 | `save-and-save-as.md` | GIF-via-Indexed on save + Save a Copy | SHIPPED | `crates/pictura-app/cpp/import_save_save.cpp`; `crates/pictura-app/cpp/indexed_color_dialog.cpp`; `crates/pictura-app/src/cxxqt_object/impl_core/save.rs`; `tst_save_gif_copy` | #628 |
| WF-5 | `web-export-and-slices.md` | Save for Web slice export with HTML/CSS, 4-Up, the colour-table editor, and the metadata levels shipped (#629); still missing: slice names/URLs/alt and Slice Options, per-slice optimization, Selected Slices, link/unlink and Divide/Combine, `.aco` tables, Web Snap, and saved output settings | PARTIAL | `save_for_web_dialog.cpp`; `save_for_web_settings.cpp`; `save_for_web_color_table.cpp`; `save_optimized_dialog.cpp`; `pictura-codec/src/web/html.rs`; `pictura-render/src/document_ops/slices.rs` | #629, #543, #1514 |
| WF-6 | `export-formats.md` | Zoomify and Paths to Illustrator shipped; Export Layers core shipped (shared options/Targa missing); Data Sets As Files and Render Video remain | PARTIAL | `crates/pictura-app/cpp/zoomify_dialog.cpp`; `export_paths_dialog.cpp`; `export_layers_dialog.cpp`; `crates/pictura-codec/src/zoomify.rs`; `illustrator.rs` | #622, #587, #625, #630 |
| WF-7 | `file-info-and-metadata.md` | Extra tabs (IPTC Extension/GPS/Origin/Photoshop) + editing | SHIPPED | `file_info_dialog.{h,cpp}`; `pictura-codec/src/xmp/tree.rs`; `pictura-codec/src/metadata/iptc_extension.rs`; `pictura-codec/src/exif.rs` | #631 |
| WF-8 | `camera-raw-workflow.md` | Camera Raw settings stores shipped (#401): content-hash database and `.xmp` sidecar by preference, read-only fallback, every tab as `crs:`; DNG output (#402) embeds the settings in the DNGs it writes; Export Settings To XMP and settings inside an existing DNG remain (#1484), and `Ignore Sidecar ".XMP" Files` with the other ACR preferences (#633) | SHIPPED | `pictura-codec/src/pictura_raw/xmp.rs`; `open_jobs/raw_open/store.rs`; `tst_raw_open` | #632, #401 |
| WF-17 | `camera-raw-workflow.md` | DNG conversion shipped (#402): Save Image… in the raw open's dialog writes mosaic or linear DNGs with colour tags, `crs:` settings, thumbnail and JPEG preview, the embedded original, and a compatibility level (approximate DNG-version mapping); linear DNGs read back; JPEG/TIFF/PSD from Save Options, Workflow Options and lossy DNG remain (#633) | SHIPPED | `pictura-codec/src/dng/write/`; `pictura-codec/src/dng/linear.rs`; `open_jobs/raw_open/save_dng.rs`; `raw_save_options_dialog.cpp`; `tst_raw_open` | #402 |
| WF-9 | `camera-raw-workflow.md` | ACR Workflow Options, ACR preferences, batch, Purge Camera Raw | MISSING | `pictura_raw_dialog.cpp:47-60` | #633 |
| WF-10 | `printing.md` | Print colour management, proofing, output marks, persist defaults | PARTIAL | `print_dialog.cpp:181-199` | #634 |
| WF-11 | `measurement-and-count.md` | Measurement scale/presets, exporters, PSD resource 1074 | SHIPPED | `crates/pictura-app/cpp`; #635 | #635 |
| WF-12 | `scratch-disks-and-memory.md` | Performance pane (memory, history states, cache level/tile, scratch disks, GPU) + Efficiency/Scratch readouts | MISSING | `preferences_dialog.cpp:172-177` | #636 |
| WF-13 | `document-lifecycle.md`, `crash-recovery-and-autosave.md` | Background Save, durable crash journal, PSD 1086/1087 | SHIPPED | `crates/pictura-app/src/history/recovery.rs`; `crates/pictura-codec/src/image_resources.rs` | #637 |
| WF-14 | `bridge-and-interop.md` | Edit Contents, PDF Paste As, and repair of supported external Smart Object links ship. Editable Illustrator clipboard paste as Paths or Shape Layer and the Files panel remain. | PARTIAL | `crates/pictura-app/src/cxxqt_object/pdf_export/import/imp/vector.rs`; `crates/pictura-app/cpp/panels/layers_row_actions.cpp` | #638, #624 |
| WF-15 | `document-lifecycle.md` | Maximize PSD/PSB Compatibility preference | SHIPPED | `crates/pictura-app/cpp/import_save_save.cpp`; `tst_psd_compatibility` | #639 |
| WF-16 | `update-and-versioning.md` | Document-format version policy (`opDocumentVersion`/`minReaderVersion`, Version Info 1057) | SHIPPED | `crates/pictura-codec/src/document_version.rs`; `tst_document_version_policy` | #640 |

Raw file open shipped in `#400` (DNG files; vendor colour matrices and the
smart-object open remain); DNG output shipped in `#402` (WF-17).

### 11-cross-cutting

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| XC-1 | `preference-storage.md` | Typed `prefs.toml` store, config/state split, migration, quarantine, reset gesture | SHIPPED | `crates/pictura-app/src/prefs.rs` | #641 |
| XC-2 | `localization.md` | `QTranslator`/`.qm`, RTL, locale number formatting | MISSING | none found | #642 |
| XC-3 | `logging-and-telemetry.md` | `tracing` + file sink + Qt→Rust bridge + History Log | MISSING | `main.cpp:104-107` | #643 |
| XC-4 | `logging-and-telemetry.md` | Opt-in telemetry + `KOOKA_PICTURA_NO_TELEMETRY` | MISSING | none found | #644 |
| XC-5 | `error-handling.md` | Stable error-code registry + FFI `OpStatus`/`catch_unwind` boundaries | SHIPPED | `crates/pictura-error/src/lib.rs` | #645 |
| XC-6 | `security-and-sandboxing.md` | Safe paths, decoder decompression-bomb guards (guards shipped, sandbox proposed) | SHIPPED | `crates/pictura-codec/src/budget.rs` (#844); `profile_probe.rs` bounded `iCCP`, Ghostscript `-dMaxBitmap`, control `document` path checks in `control_server.cpp` (#646). Not covered here: the out-of-process decode helper is proposal-only; fuzzing is #648 | #646 |
| XC-7 | `security-and-sandboxing.md` | Flatpak packaging + portal confinement | PARTIAL | `flatpak/io.github.KookaStudio.KookaPictura.yml`; bundle/smoke ship, portal dialogs unproven | #647 |
| XC-8 | `testing-strategy.md` | DSSIM/PSNR/ΔE2000, property tests, fuzzing, Criterion benches | MISSING | `testkit` abs-diff only | #648 |
| XC-9 | `update-and-versioning.md` | `--version`, About dialog with Qt/GPU info, update checker | SHIPPED | `crates/pictura-app/cpp/system_info.cpp`; `frame_system_info.cpp` | #649 |
| XC-10 | `cs6-editions-and-constraints.md` | Edition gating (Standard/Extended) | SHIPPED | `pictura_core::Edition`; `PICTURA_EDITION` selects Standard/Extended | #650 |
| XC-11 | `performance-targets.md` | Performance instrumentation (cache/scratch/frame/metrics) | PARTIAL | `pictura-app::metrics`, frame-budget timing (#941), the tile cache (#942), and scratch paging (#943) ship; the tile scheduler and cancellation (#944/#945) remain | #651, #941–#945 |

## D. Code ahead of docs (stale specs)

| Doc | Claim | Reality |
|---|---|---|
| `openspec/specs/ui/channels-panel/spec.md` | the Purpose paragraph calls the panel presentational "because the model has no per-channel visibility" | live channels shipped, and the same spec now carries a "Channel visibility on the canvas" requirement (`spec.md:109`) — the doc contradicts itself |
| `docs/dev/STATE.md` known-risks list | history whole-doc clone, resident GPU buffers deferred, and eight further stale claims | **fixed by `#785`**: history is copy-on-write with tile diffs, and the resident GPU source buffers landed (`crates/pictura-render/src/gpu/resident.rs`) |

## E. Confirmed non-goals (absent by design)

3D tools/layers (`TOOL-060`), Artboards (`LAY-023`), ExtendScript/E4X and
`.8bf`/`.8li` plug-in binaries (`AUTO-005`, `ARCH-011`), Adobe
cloud/Stock/Bridge/Mini-Bridge (`WF-023`), Timeline/video. Verified absent, no
accidental implementation found.

Two entries this list used to carry are corrected here:

- **Linked/embedded objects** (`LAY-024`) — the codec parses and writes
  `lnkD`/`lnk2`/`lnk3` records (`crates/pictura-codec/src/smart_object.rs:199`).
  Relink to File repairs supported external links (#638); broader relink and
  convert-to-linked remain with #113.
- **Snap and AppImage packaging** — not a non-goal. Both ship alongside
  Flatpak and the native packages (`snap/snapcraft.yaml`,
  `.github/workflows/snap.yml`, `.github/workflows/appimage.yml`).

## F. Existing issues already tracking gaps

`#102`/`#103`/`#104`/`#309` crop · `#113` linked smart objects · `#166` smart
filters · `#167`/`#183` filter bundles · `#172` Quick Mask · `#262` pattern fill layers · `#263` Edit Adjustment row ·
`#264` Blend-If UI · `#305` View Snap ·
`#306` canvas right-click menus · `#312` Alt-drag duplicate · `#313` picker
eyedropper · `#319` Color panel · `#344`/`#345`/`#346`/`#347`/`#349`/`#350`/`#351`
filters & watermark · `#367` Help menu · `#392`/`#396`–`#402`
Pictura Raw · `#436`/`#437`/`#438` mask/channel click · `#448` slider recomposite
· `#451` floating panels · `#458` blur edge · `#459`/`#477` SVG export ·
`#465` shortcut audit · `#468` serialized builds.

## G. Coverage notes

- Method: each spec read in full → concrete features extracted → located in code
  (graph + grep) → classified → cross-checked against `openspec/changes/`.
- Areas confirmed largely complete (no significant gaps): PSD codec round-trip,
  blend-mode formulas, layer-style effect rendering, layer masks/vector
  masks/clipping/merge/flatten, align/distribute, panel-rail/workspace shell,
  filter families' kernels, the 8-bit adjustment engine, and the Qt Test harness.
- The dominant pattern is an **8-bit RGB/Gray working raster** with target
  mode/depth/palette/ICC held in `source_*`/`document_icc` stores for save
  round-trip, which is why full depth/colour-mode fidelity (SYS-2) is broad.
- Most "missing" secondary controls are already marked in-source with `ponytail:`
  comments naming the ceiling — they are known scope cuts, not regressions.
- **Corrections from the colour-management bundle.** `IMG-16` is removed:
  Convert to Profile options/preview/flatten, mismatch/missing warnings, the
  embed toggle, profile-aware CMYK conversion (#809), and raster ICC
  import/export now ship (`profile_dialog.cpp`, `profile_warning_dialog.cpp`,
  `save_options_dialog.cpp`, `color_management.rs`, `profile_probe.rs`).
  `IMG-17`, `CLR-3`, and `CLR-7` are narrowed to the remaining `.csf`/Gray/Spot
  settings, Color Libraries, and 16/32-bit picker gaps.
- **Corrections from the lens-correction smart-filter bundle.** `FILT-10` is
  removed: Lens Correction on a Smart Object now attaches a Lens Correction
  smart filter (`crates/pictura-codec/src/lens_correction/`,
  `pictura-render/src/smart_filter.rs`, `document_ops/lens_correction.rs`),
  re-opens the dialog prefilled, and renders through the composite. The
  descriptor is Pictura-only, so no cross-app interop is claimed (#934).
- **Corrections from the lens-correction profiles bundle.** `FILT-8` is
  removed: Lens Correction reads the system's lensfun database at runtime
  (`crates/pictura-filters/src/distort/lens_correction/lensfun.rs`,
  `pictura-app/src/cxxqt_object/filter_tools/lens_profiles.rs`) and user
  profiles, picks the closest sub-profile with a right-click menu, offers
  Prefer RAW Profiles, and stores the resolved coefficients in its settings.
  Search Online and Save Online Profile Locally stay disabled by decision:
  there is no online profile service (#932).
- **Corrections from the `#785` sweep.** Five rows in this register were wrong,
  and have been fixed or removed:
  - `FILT-1` (Lighting Effects engine, `#592`) — the engine ships
    (`crates/pictura-filters/src/render/lighting.rs`, mapped at
    `crates/pictura-filters/src/catalogue/build/render.rs` (Lighting Effects arm), descriptor at
    `crates/pictura-filters/src/catalogue/rows/render.rs`). The row is removed and
    `#592` can be closed.
  - `LAY-7` (Rubylith display options, `#585`) — shipped in `#672`
    (`crates/pictura-app/cpp/mask_display_options_dialog.cpp:20`). The row is
    removed and `#585` can be closed.
  - §D's `artistic-filters` row — the spec now says `brush_size 0..=10`; it
    caught up with the code.
  - §D's `export-formats.md` row — the spec now lists SVG as a first-class
    format; it caught up with the code.
  - §D's "repository contains no source code" row and §E's
    "linked/embedded objects — verified absent" — both corrected above.
  - `OVR-1` (the "no source code" claim, `#496`) — the claim is corrected and
    the issue is closed, so the row is removed.
- **Corrections from the `#1358` roadmap sweep.** Five rows the tree had moved
  past are now `SHIPPED`: `LAY-6` (#584), `LAY-8` (#586), `LAY-10` (#588),
  `TOOL-5` (#529), and `FILT-2` (#593). `PAN-15` is narrowed to the remaining
  `00`/`Shift+00` opacity shortcut, and `FILT-11` records the Filter Gallery and
  Shear previews that still ignore an existing Smart Filter chain (#1003).
- **Corrections from the `#1522` tracker sweep.** Seven rows are now `SHIPPED`
  (`TOOL-23` #547, `WF-13` #637, `WF-15` #639, `WF-16` #640, `XC-1` #641,
  `XC-5` #645, `XC-9` #649). `WF-14` records the shipped interop work and the
  remaining editable Illustrator paste (#638) and Files panel (#624).
  `XC-11` (#651) records the shipped frame budget, tile cache, and scratch
  paging, leaving the scheduler and cancellation (#944/#945).
