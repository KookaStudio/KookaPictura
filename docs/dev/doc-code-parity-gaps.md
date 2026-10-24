# Docs ↔ code parity gaps

- **Status:** living register
- **Owner:** see `#486`
- **Source audit:** the 30-agent docs↔code parity audit summarised here; per-gap issues filed in `#488` (`#489`–`#651`)
- **Re-checked:** `#785` swept the rows this register's readers depend on and
  corrected five that the tree had moved past; the corrections are listed in
  §G and the rows that were wrong have been removed.
- **Rows:** 161 gaps — 5 systemic in §B, 156 per-domain in §C.

This register maps the behavioural contract in `docs/` against the implementation
in `crates/`. It is the canonical place to see **what is specified but missing,
partial, or divergent**, and which GitHub issue tracks each gap.

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
| SYS-3 | `Edit > Fade` (post-filter opacity + blend mode) is an inert leaf | `command_tree.cpp:96`, no handler | #491 |
| SYS-4 | Brush dynamics option sets (texture, dual brush, colour dynamics, transfer, pose, padlocks) | `brush_panel.cpp:115-152` all disabled | #492 |
| SYS-5 | Sampled / bristle / erodible / airbrush tips + Define Brush Preset | `tip.rs` round/elliptical only | #493 |

**Provenance note.** All five were landed as behavioural `photorust` ports, not
CS6-faithful; the ceilings are marked in-source with `ponytail:` comments.

## C. Per-domain register

### 00-overview & 01-architecture

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| ARCH-1 | `01-architecture/build-and-packaging.md` | Spec file never authored (`ARCH-014`); dangling refs | MISSING | `TRACEABILITY.md:37` + 8 refs | #494 |
| ARCH-2 | `01-architecture/*` | Proposed crate map diverges from the real workspace | DIVERGENT | `Cargo.toml`, `crates/` | #495 |
| OVR-2 | `11-cross-cutting/cs6-editions-and-constraints.md` | No `Edition` (Standard/Extended) gating | MISSING | no `Edition::` in `crates/` | #650 |

### 02-ui-ux — application frame and menus

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| UI-1 | `application-frame.md` | Status bar readouts + status-bar options popup | MISSING | `frame.cpp:514` | #497 |
| UI-2 | `application-frame.md` | Active layer name in the title bar | MISSING | `frame.cpp:1006` | #498 |
| UI-3 | `application-frame.md` | `Ctrl+Tab` / `Shift+Ctrl+Tab` document cycling | MISSING | `frame.cpp:224` | #499 |
| UI-4 | `application-frame.md` | Window ▸ Arrange (Float/Tile/Cascade/Match/New Window) | MISSING | `command_tree.cpp:891-900` | #500 |
| UI-5 | `menus.md` | Menu customization (Edit ▸ Menus, Show All Menu Items, enablement matrix) | MISSING | `command_tree.cpp:189` | #501 |
| UI-6 | `menus.md` | Type menu (work path, convert to shape, spellcheck); warp text landed in #684, convert point/paragraph in #682 | MISSING | `command_tree.cpp:588-618` | #502 |
| UI-7 | `menus.md` | View menu stubs (Proof Setup/Colors, Gamut Warning, Snap To, Lock Slices, Extras, Pixel Aspect Ratio, 32-bit Preview Options) | MISSING | `command_tree.cpp:807-885` | #503 |
| UI-8 | `menus.md` | File menu stubs (Open As, Import, Export, Automate, Scripts, Browse in Bridge) | MISSING | `command_tree.cpp:23-75` | #504 |
| UI-9 | `preferences.md` | 5 missing panes (Cursors, Transparency & Gamut, Plug-ins, Type, 3D) | MISSING | `preferences_dialog.cpp:35-47` | #505 |
| UI-10 | `preferences.md` | Sparse existing panes (General/Interface/File Handling/Performance/Units/Guides) | PARTIAL | `preferences_dialog.cpp:113-177` | #506 |
| UI-11 | `accessibility.md` | AT-SPI2 / `QAccessible`, high-contrast, colour-blind, reduced-motion | MISSING | no `QAccessible` in `crates/` | #507 |
| UI-12 | `keyboard-shortcuts.md` | Shortcut sets (save/load, workspace-assigned, Summarize HTML, Legacy Channel Shortcuts) | PARTIAL | `frame_shortcuts.cpp:20`; `keyboard_shortcuts_dialog.cpp:249-282` | #508 |
| UI-13 | `color-sweeps-and-modes.md` | Quick fill with Alt+Backspace / Ctrl+Backspace | MISSING | `frame_menus_edit.cpp:307-310` | #524 |

### 02-ui-ux — panels

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| PAN-1 | `panels/adjustments-panel.md` | Adjustments panel icon grid (creation shipped; mode/depth gating and fill-layer presets remain) | PARTIAL | `panels/adjustments_panel.cpp` | #509 |
| PAN-2 | `panels/measurement-log-panel.md` | Measurement Log panel + measurement engine | MISSING | `command_tree.cpp:962` | #510 |
| PAN-3 | `panels/tool-presets-panel.md` | Tool Presets panel + `.tpl` libraries | MISSING | `command_tree.cpp:982` | #511 |
| PAN-4 | `layer-styles.md` | Styles panel + `.asl` presets | MISSING | `frame_build.cpp:79` | #512 |
| PAN-5 | `panels/channels-panel.md` | Spot channels, Split/Merge, Channel Options, Panel Options | MISSING | `panel_group_menu.cpp:90-98` | #513 |
| PAN-6 | `panels/history-panel.md` | History Options, Clear History/purge, New Document from state | MISSING | `panel_group_menu.cpp:195-198` | #514 |
| PAN-7 | `panels/histogram-panel.md` | View modes, statistics, source menu, cache/refresh, 16/32-bit bins | PARTIAL | `histogram_panel.cpp:79-142` | #515 |
| PAN-8 | `panels/info-panel.md` | Sampler persistence, sample-size readout, marquee/transform readouts, before/after, gamut exclamation | PARTIAL | `info_panel.cpp:623-685` | #516 |
| PAN-9 | `panels/character-and-paragraph.md` | Character panel missing controls (faux bold/italic, language, tsume, underline, fractional widths) | PARTIAL | `character_panel.cpp:65-132` | #517 |
| PAN-10 | `panels/character-and-paragraph.md` | Paragraph Justification dialog + RTL direction | MISSING | `paragraph_panel.h:33-40` | #518 |
| PAN-11 | `panels/character-and-paragraph.md` | Character Styles panel | MISSING | no `character_styles_panel.*`; engine API at `type_tools.rs:632` | #519 |
| PAN-12 | `panels/navigator-panel.md` | Proxy colour, Ctrl-drag proxy positioning, zoom buttons, typed zoom | MISSING | `navigator_panel.cpp:138-159` | #520 |
| PAN-13 | `panels/paths-panel.md` | Make Work Path, clipping-path designation, Panel Options thumbnail size | MISSING | `paths_panel.h:24-25` | #521 |
| PAN-14 | `panels/note-panel.md` | Notes Author/Colour options + PSD/PDF/TIFF annotation round-trip | MISSING | `options_bar.cpp:593`; `annotations.rs:10` | #522 |
| PAN-15 | `panels/layers-panel.md` | Panel Options thumbnail size + `00`/`Shift+00` opacity shortcut | MISSING | `session.h:25`; `layers_panel_events.cpp` | #523 |

### 03-tools

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| TOOL-1 | `marquee-selection.md` | Single Row / Single Column marquee tools | MISSING | no `ToolId`/registration | #525 |
| TOOL-2 | `transform-selection.md` | Select ▸ Transform Selection | MISSING | `command_tree.cpp:658` | #526 |
| TOOL-3 | `move-and-transform.md` | Puppet Warp | MISSING | `command_tree.cpp:129` | #527 |
| TOOL-4 | `move-and-transform.md` | Content-Aware Scale | MISSING | `command_tree.cpp:125-127` | #528 |
| TOOL-5 | `move-and-transform.md` | Transform Scale/Rotate/Again + exact rotations + flip | MISSING | `command_tree.cpp:132-148` | #529 |
| TOOL-6 | `move-and-transform.md` | Transform Path / Transform Points submenus | MISSING | `command_tree.cpp:149-159` | #530 |
| TOOL-7 | `pen-and-path-tools.md` | Magnetic Pen | MISSING | `options_bar_pen.cpp:19` | #531 |
| TOOL-8 | `merge-and-flatten.md` | Merge Shape Components (path boolean ops) | MISSING | no path-boolean in `pictura-core/src/path*` | #532 |
| TOOL-9 | `hand-and-zoom.md` | Hand/Zoom options bar (Scrubby/Animated Zoom, Resize Windows, Zoom All/Scroll All/Match) | MISSING | `options_bar.cpp` no case | #533 |
| TOOL-10 | `hand-and-zoom.md` | Bird's Eye View + Flick Panning | MISSING | none found | #534 |
| TOOL-11 | `eyedropper-color-sampler-ruler.md` | Ruler protractor + Straighten from the ruler | MISSING | `tool_ruler.cpp:37` | #535 |
| TOOL-12 | `eyedropper-color-sampler-ruler.md` | Eyedropper modifiers (Alt→background, temporary) + Shift-click sampler | MISSING | `tool_eyedropper.cpp:23-36` | #536 |
| TOOL-13 | `clone-stamp-and-pattern-stamp.md` | Clone Source overlay controls | MISSING | `clone_source_panel.h` | #537 |
| TOOL-14 | `healing-brushes.md` | Healing Mode/Source/Sample, Sample All Layers, depth/mode handling | PARTIAL | `options_bar.cpp:694-741`; `tool_healing.cpp:165` | #538 |
| TOOL-15 | `history-brush.md` | Merged/composite source + Edit ▸ Fill ▸ History | PARTIAL | `paint_tools.rs:524-527` | #539 |
| TOOL-16 | `mixer-brush.md` | Sample All Layers, Load Solid Colors Only, keyboard shortcuts | MISSING | `options_bar_paint.cpp:310`; `tool_mixerbrush.cpp:39-41` | #540 |
| TOOL-17 | `type-tools.md` | Missing Glyph Protection (#887); gamma blend and missing-font handling landed in #685 (an approximated colour-independent curve, Replace All substitutes Liberation Sans); warp text landed in #684 (outline-warped presets, the caret stays unwarped while editing), type-on-path and area type landed in #683 (area type clips a bounding-box layout), paragraph type in #682 (greedy wrap, resize-only box handles) | PARTIAL | `type_path.rs`; `text_gamma.rs`; `missing_fonts.rs`; `command_tree.cpp:595-630` | #541 |
| TOOL-18 | `path-selection-tools.md` | Path operations/align/arrange, constrain path dragging, Target Path toggle, segment/marquee multi-select | MISSING | `tool_path_selection.cpp:13-14` | #542 |
| TOOL-19 | `slice-tools.md` | Slice options bar/style/modifiers, Divide/Combine/Duplicate, Slice Options dialog, slices-from-guides | MISSING | `tool_slice.cpp:65-102` | #543 |
| TOOL-20 | `crop-tool.md` | Crop resampling by W×H×resolution, non-destructive crop/Reveal All, crop shield settings | PARTIAL | `options_bar_crop.cpp:244-375` | #544 |
| TOOL-21 | `custom-shape.md` | `.CSH` libraries, Define Custom Shape, geometry modes, gradient/pattern fill-stroke | MISSING | `options_bar_shape.cpp:506-510,420-431` | #545 |
| TOOL-22 | `shape-tools.md` | Gradient/pattern fill+stroke, dashed stroke, path ops, fill-pixels options, snap-to-pixels, proportional geometry | MISSING | `options_bar_shape.cpp:281-431`; `shapes.rs:316-333` | #546 |
| TOOL-23 | `color-replacement.md` | Full mode list + disable on Bitmap/Indexed/Multichannel | DIVERGENT | `replace.rs:25`; `paint_tools.rs:343` | #547 |
| TOOL-24 | `dodge-burn-sponge.md` | Grayscale Sponge, airbrush toggle, quick keys, 32-bpc availability | PARTIAL | `tone.rs:223-243`; `options_bar_retouch.cpp:93` | #548 |
| TOOL-25 | `eraser-tools.md` | Magic-eraser background convert, background-eraser edge halo, pen pressure | PARTIAL | `eraser.rs:166-332` | #549 |
| TOOL-26 | `gradient-and-paint-bucket.md` | Gradient/Bucket availability gates, Gradient Editor reachability, Gradient Fill dialog | PARTIAL | `tool_fills.cpp`; `options_bar_fill.cpp:129-131`; `frame_menus.cpp:719-725` | #550 |
| TOOL-27 | `smudge-blur-sharpen.md` | HDR (16/32-bpc) operation | DIVERGENT | `stroke.rs:395` rejects non-8-bit | #551 |
| TOOL-28 | `clone-stamp-and-pattern-stamp.md` | Cross-document colour-mode rule + Pattern Stamp Impressionist | MISSING | `paint_tools.rs:451`; `tool_stamps.cpp` | #552 |

### 04-image-ops and adjustments

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| IMG-1 | `image-modes.md`, `duotone.md` | Duotone conversion + Options dialog | MISSING | `command_tree.cpp:216`; `mode/mod.rs:72` | #553 |
| IMG-2 | `image-modes.md` | Multichannel conversion | MISSING | `mode/mod.rs:72` | #554 |
| IMG-3 | `image-modes.md`, `indexed-color.md` | Color Table + Indexed transparency/Forced palette | MISSING | `command_tree.cpp:227`; `indexed_color_dialog.h:16` | #555 |
| IMG-4 | `image-modes.md` | Bitmap Halftone Screen + Custom Pattern | MISSING | `mode/flat.rs:20,185` | #556 |
| IMG-5 | `bit-depth-and-conversion.md`, `hdr-toning.md` | HDR Conversion tone methods implemented; the Local Adaptation Toning Curve/Corner remains | PARTIAL | `hdr_conversion_dialog.cpp`; `pictura-adjust/src/hdr_toning/` | #557 |
| IMG-6 | `32-bit-hdr.md` | Merge To HDR Pro | MISSING | `command_tree.cpp:65` | #558 |
| IMG-7 | `auto-adjustments.md` | Auto Color Correction Options dialog | MISSING | `panel_group_menu.cpp:249` | #559 |
| IMG-8 | `adjustments/levels.md` | Levels eyedroppers + Alt-drag clipping preview | MISSING | `levels_dialog.cpp` | #560 |
| IMG-9 | `adjustments/curves.md` | Curves secondary UI (on-image, pencil/smooth, sliders+eyedroppers, display options, Auto, CMYK/Lab graphs) | MISSING | `curves_dialog.h:17`; `panel_group_menu.cpp:250-251` | #561 |
| IMG-10 | `adjustments/shadow-highlight.md` | Shadows/Highlights local-surround operator (8 controls) | MISSING | `tonal.rs:155-159` | #562 |
| IMG-11 | `adjustments/threshold.md` | Threshold histogram under the slider | MISSING | `adjustment_params.rs:448-454` | #563 |
| IMG-12 | `adjustments/vibrance.md` | Vibrance Auto + preset | MISSING | `adjustment_dialog.cpp:446-452` | #564 |
| IMG-13 | `image-size.md` | Smoother/Sharper/Automatic kernels, Scale Styles, Auto resolution, picas/columns, float resample | DIVERGENT | `resize.rs:13`; `image_size_dialog.cpp:70` | #565 |
| IMG-14 | `canvas-size.md` | picas/columns units + extension-fill availability gating | PARTIAL | `canvas_size_dialog.cpp:256,274` | #566 |
| IMG-15 | `image-rotation-and-flip.md` | Arbitrary rotation dialog, interpolation preference, ruler-straighten prefill | DIVERGENT | `frame_menus_image.cpp:115` | #567 |
| IMG-16 | `color-profiles-and-assignment.md` | Convert to Profile options + mismatch/missing warnings + embed toggle | PARTIAL | `profile_dialog.cpp:19`; `icc.rs:259` | #568 |
| IMG-17 | `color-settings.md` | full Color Settings dialog (spaces, policies, conversion options, `.csf`) | PARTIAL | `color_settings_dialog.cpp:23-43` | #569 |
| IMG-18 | `32-bit-hdr.md` | 32-bit Preview Options, HDR colour picker, 32-bit Info readout | MISSING | `info_panel.cpp:377` | #570 |
| ADJ-1 | `adjustments/channel-mixer.md` | Monochrome UI + CMYK/Gray outputs | MISSING | `channel_mixer_dialog.h:15` | #571 |
| ADJ-2 | `adjustments/color-balance.md` | Numeric readouts + Lab/CMYK mapping | MISSING | `color_balance_dialog.cpp:33-53`; `color.rs:165-192` | #572 |
| ADJ-3 | `adjustments/hue-saturation.md` | Colorize, range-band editing, eyedroppers, on-image tool | MISSING | `hue_saturation_dialog.h:17-21` | #573 |
| ADJ-4 | `adjustments/gradient-map.md` | Dither + CS6 print-toning presets | MISSING | `gradient_map_dialog.cpp:82-91`; `gradient.rs:295-310` | #574 |
| ADJ-5 | `adjustments/match-color.md` | On-canvas eyedropper + live Info feedback | MISSING | `match_color_dialog.cpp` | #575 |
| ADJ-6 | `adjustments-overview.md` | Adjustment presets (Levels/Curves/Photo Filter/Selective Color/Vibrance) | MISSING | `adjustment_params.rs:639` (Color Lookup only) | #576 |
| ADJ-7 | `adjustments/selective-color.md` | Composite-channel-only gate + preset whitelist | MISSING | no channel gate | #577 |
| ADJ-8 | `adjustments/brightness-contrast.md` | Brightness/Contrast Auto + shared adjustment Auto buttons | MISSING | `adjustment_dialog.cpp:445` | #578 |

### 05-layers

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| LAY-1 | `blend-modes.md` | Advanced Blending toggles (Blend Clipped, Transparency Shapes, mask hides effects, channel Include; Blend Interior inert) | MISSING | `core/lib.rs:884` stored but never read | #579 |
| LAY-2 | `layer-styles.md` | Global Light dialog, Create Layers, Rasterize Layer Style | MISSING | `frame_menus_layer_style.cpp:3`; `command_tree.cpp:346-347` | #580 |
| LAY-3 | `layer-styles.md` | Bevel Contour/Texture UI, gradient editor, channel include in the authoring dialog | MISSING | `layer_style_dialog.h:9` | #581 |
| LAY-4 | `align-and-distribute.md` | Auto-Align Layers | MISSING | none found | #582 |
| LAY-5 | `align-and-distribute.md` | Auto-Blend Layers | MISSING | none found | #583 |
| LAY-6 | `blend-modes.md` | `Fill With (Mode)-Neutral Color` for new layers | MISSING | none found | #584 |
| LAY-8 | `layer-management-ui.md` | Panel Options thumbnail size (Entire Document/Bounds) | PARTIAL | `session.h:25` | #586 |
| LAY-9 | `export-formats.md` | File ▸ Scripts ▸ Export Layers To Files | MISSING | `command_tree.cpp:69` | #587 |
| LAY-10 | `layer-management-ui.md` | Shape layer naming `<Tool> N` | DIVERGENT | `shapes.rs:238` | #588 |
| LAY-11 | `blend-modes.md` | 32-bit/Lab blend-mode menu gating | MISSING | `blend.rs` offers all | #589 |
| LAY-12 | `layer-management-ui.md` | `Ctrl+J` duplicate-group semantics | DIVERGENT | `command_tree.cpp:312` | #590 |
| LAY-13 | `smart-objects.md` | Extended Stack Modes | MISSING | no `StackMode`/`reduce_stack` | #591 |
| LAY-14 | `merge-and-flatten.md` | Merge Down result appearance: the merged node inherits the target's blend mode and opacity, so a target with non-default settings renders the merged result differently from the pre-merge canvas; the pair also composites against a transparent backdrop, so where the target is non-`Normal` or not opaque the source's blend sees the target's unblended colour | UNRESOLVED | `merge.rs:195`; `merge.rs:202-203`; `merge.rs:422-424` | #813 |

Layer Comps (`LAY-022`) is already tracked by `#265`.

### 06-filters

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| FILT-2 | `noise-filters.md` | Reduce Noise (incl. Advanced per-channel) | MISSING | `command_tree.cpp:743` | #593 |
| FILT-4 | `sharpen-filters.md` | Smart Sharpen Basic/Advanced split + 32-bpc disable | DIVERGENT | `filter_map/descriptor/render.rs` (Smart Sharpen row) | #595 |
| FILT-5 | `stylize-filters.md` | 8 Stylize filters missing from the Filter Gallery category | DIVERGENT | `filter_gallery_dialog.cpp:118` | #596 |
| FILT-6 | `artistic-filters.md` | Colored Pencil document-background paper wiring | PARTIAL | `filter_map/mapping.rs` (Colored Pencil arm) | #597 |
| FILT-7 | `adaptive-wide-angle.md` | Adaptive Wide Angle Auto / lens profiles, Panorama, Smart Filter re-edit, rotation handles | PARTIAL | `pictura-filters/src/distort/adaptive_wide_angle/`, `awa_dialog.cpp` | #891 |
| FILT-8 | `vanishing-point.md` | Vanishing Point Marquee (Feather / Opacity / Heal / Move Mode), Stamp, Brush, Transform, Eyedropper and Paste | MISSING | `vp_dialog.cpp` (tool column) | #349 |
| FILT-9 | `vanishing-point.md` | Vanishing Point planes saved in PSD / TIFF / JPEG | MISSING | `document_session/session.rs` (`vanishing_planes`) | #918 |

Lens Correction (`#347`),
De-Interlace (`#344`), NTSC Colors (`#345`), Embed/Read Watermark (`#350`/`#351`)
are already tracked.

### 07-color-painting

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| CLR-1 | `gradient-presets.md` | Gradient Editor reachable from the Gradient tool + opacity stops + midpoint | PARTIAL | `options_bar_fill.cpp:129-131`; `gradient_editor_dialog.cpp:481-484` | #598 |
| CLR-2 | `pattern-presets.md` | Define Pattern, `.pat` library, Scripted Patterns | MISSING | `command_tree.cpp:159`; `codec/patterns.rs` | #599 |
| CLR-3 | `color-picker.md` | Gamut/web alerts, Color Libraries (PANTONE), picker preference | MISSING | `color_picker_dialog.cpp:1083-1085` | #600 |
| CLR-4 | `color-picker.md` | HUD colour picker | MISSING | none found | #601 |
| CLR-5 | `color-picker.md` | HDR colour picker (32-bpc) | MISSING | `color_picker_dialog.cpp` 8-bit only | #602 |
| CLR-7 | `color-models.md` | Out-of-gamut triangle, CMYK→working space, 16/32-bit picker fields | MISSING | `color_picker_dialog.cpp:777-780` | #604 |
| CLR-8 | `swatches-and-libraries.md` | `.aco`/`.ase` libraries, rename, display modes | MISSING | `swatches_panel.h:74` | #605 |
| CLR-9 | `pattern-presets.md` | Pattern Overlay Snap To Origin + style Texture sub-options | PARTIAL | `layer_style_pages.cpp:297-315` | #606 |
| CLR-10 | `bristle-brushes.md` | Bristle brushes | MISSING | `paint/tip.rs` | #607 |
| CLR-11 | `brush-presets.md` | `.abr` libraries, Brush Presets panel, ABR v10 read | MISSING | `brush_panel.cpp:137-139` | #608 |

### 08-selection

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| SEL-1 | `channel-based-masking.md` | Apply Image | MISSING | `command_tree.cpp:298` | #609 |
| SEL-2 | `channel-based-masking.md` | Calculations | MISSING | `command_tree.cpp:299` | #610 |
| SEL-3 | `channel-based-masking.md` | Channel Options (Masked/Selected Areas) | MISSING | no polarity model | #611 |
| SEL-4 | `refine-edge.md` | Interactive Refine Radius/Erase brushes | MISSING | `changes/refine-edge` out of scope | #612 |
| SEL-5 | `color-range.md` | Skin Tones, Localized Clusters, Out Of Gamut, previews, save/load | MISSING | `color_range.rs:7` | #613 |
| SEL-6 | `selection-model.md` | Engine-wide anti-aliased rasterisation + 56-channel cap | MISSING | binary pixel-centre rasteriser | #614 |
| SEL-7 | `save-and-load-selections.md` | Combine ops, layer-mask target, channel modifier-click combine | PARTIAL | `impl_selection.rs:298-332` | #615 |

Quick Mask is tracked by `#172`.

### 09-automation

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| AUTO-1 | `actions.md` + `script-events-and-jsx.md` | Shared ActionDescriptor / command-recording bus (foundation) | MISSING | `commands.cpp:146` dispatcher only | #616 |
| AUTO-2 | `droplets.md` | Create Droplet, `.opd`, `.desktop` launcher | MISSING | `command_tree.cpp:62` | #617 |
| AUTO-3 | `batch-processing.md` | File ▸ Automate ▸ Batch engine + dialog | MISSING | `command_tree.cpp:58` | #618 |
| AUTO-4 | `batch-processing.md` | Remaining Automate commands (Image Processor, Conditional Mode Change, PDF Presentation, Contact Sheet II, Picture Package, Photomerge, Web Photo Gallery, File Naming) | MISSING | `command_tree.cpp:59-68` | #619 |
| AUTO-5 | `rust-scripting-replacement.md` | Scripting host, DOM, ActionManager, notifiers (QuickJS) | MISSING | no scripting crate/dep | #620 |
| AUTO-6 | `rust-scripting-replacement.md` | Script console, sandbox limits, security prompt, progress bridge | MISSING | none found | #621 |
| AUTO-7 | `variables-and-data-driven-graphics.md` | Variables / data-driven graphics | MISSING | `command_tree.cpp:293-295` | #622 |
| AUTO-8 | `plugin-sdk.md` | Rust C-ABI plugin loader + Plug-in manager dialog | MISSING | no `pictura-plugin` | #623 |
| AUTO-9 | `mini-bridge.md` | Mini-Bridge replacement Files panel | MISSING | none found | #624 |

The Actions panel/engine is tracked by `#301`.

### 10-workflow-io

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| WF-1 | `save-and-save-as.md` | Save As per-format option dialogs (TIFF/JPEG/PNG/GIF/EPS/DCS/Raw/BMP/TGA/Cineon) | MISSING | `encode_image.cpp:23-64` | #625 |
| WF-2 | `save-and-save-as.md` | Photoshop PDF writer shipped (#626); still missing: JPEG2000 compression (no encoder), Spot Colors (no spot-channel model), Use Proof Setup (no Proof Setup); Preserve Photoshop Editing Capabilities embeds Pictura's own PSD, not Adobe's layout | PARTIAL | `pictura-codec/src/pdf/options.rs:661`; `pdf_save_dialog.cpp:220,278` | #626 |
| WF-3 | `open-and-new.md` | Generic PDF/EPS import (Import PDF and Rasterize Generic EPS Format dialogs) and Open As shipped (#627) through Ghostscript; still missing: native CMYK/16-bit rendering (converted from an 8-bit RGB render), Images mode on encrypted PDFs, inline images, the 3D option | PARTIAL | `pictura-codec/src/pdf/import/mod.rs`; `import_pdf_dialog.cpp`; `frame_import.cpp` | #627 |
| WF-4 | `save-and-save-as.md` | GIF-via-Indexed on save + Save a Copy | MISSING | `encode_image.cpp` | #628 |
| WF-5 | `web-export-and-slices.md` | Save for Web slice HTML/CSS emitter, 2-Up/4-Up, colour-table editor, metadata menu | MISSING | `save_for_web_dialog.cpp:176-292` | #629 |
| WF-6 | `export-formats.md` | Export Layers To Files, Data Sets As Files, Zoomify, Render Video, Paths to Illustrator | MISSING | `command_tree.cpp:53,55-57,69` | #630 |
| WF-7 | `file-info-and-metadata.md` | Extra tabs (IPTC Extension/GPS/Origin/Video/Audio/Photoshop/DICOM) + editing | PARTIAL | `file_info_dialog.cpp:73` | #631 |
| WF-8 | `camera-raw-workflow.md` | Camera Raw settings stores (XMP sidecar / ACR database) | MISSING | `crs_xmp.rs:1-9` | #632 |
| WF-9 | `camera-raw-workflow.md` | ACR Workflow Options, ACR preferences, batch, Purge Camera Raw | MISSING | `pictura_raw_dialog.cpp:47-60` | #633 |
| WF-10 | `printing.md` | Print colour management, proofing, output marks, persist defaults | PARTIAL | `print_dialog.cpp:181-199` | #634 |
| WF-11 | `measurement-and-count.md` | Measurement scale/presets, exporters, PSD resource 1074 | MISSING | no measurement engine | #635 |
| WF-12 | `scratch-disks-and-memory.md` | Performance pane (memory, history states, cache level/tile, scratch disks, GPU) + Efficiency/Scratch readouts | MISSING | `preferences_dialog.cpp:172-177` | #636 |
| WF-13 | `document-lifecycle.md`, `crash-recovery-and-autosave.md` | Background Save, durable crash journal, PSD 1086/1087 | MISSING | `STATE.md:215`; `recovery_store.h:26-29` | #637 |
| WF-14 | `bridge-and-interop.md` | Edit Contents, relink, Paste As, Files panel | MISSING | `helpers.rs:128` | #638 |
| WF-15 | `document-lifecycle.md` | Maximize PSD/PSB Compatibility preference | MISSING | writer always emits composite | #639 |
| WF-16 | `update-and-versioning.md` | Document-format version policy (`opDocumentVersion`/`minReaderVersion`, Version Info 1057) | MISSING | none found | #640 |

Raw file open is tracked by `#400`; DNG output by `#402`.

### 11-cross-cutting

| ID | Spec | Gap | Status | Evidence | Issue |
|---|---|---|---|---|---|
| XC-1 | `preference-storage.md` | Typed `prefs.toml` store, config/state split, migration, quarantine, reset gesture | DIVERGENT | `session.cpp:74-83` | #641 |
| XC-2 | `localization.md` | `QTranslator`/`.qm`, RTL, locale number formatting | MISSING | none found | #642 |
| XC-3 | `logging-and-telemetry.md` | `tracing` + file sink + Qt→Rust bridge + History Log | MISSING | `main.cpp:104-107` | #643 |
| XC-4 | `logging-and-telemetry.md` | Opt-in telemetry + `KOOKA_PICTURA_NO_TELEMETRY` | MISSING | none found | #644 |
| XC-5 | `error-handling.md` | Stable error-code registry + FFI `OpStatus`/`catch_unwind` boundaries | MISSING | no `pictura-error` | #645 |
| XC-6 | `security-and-sandboxing.md` | Safe paths, decoder decompression-bomb guards | PARTIAL | `png.rs` only | #646 |
| XC-7 | `security-and-sandboxing.md` | Flatpak packaging + portal confinement | PARTIAL | `flatpak/io.github.KookaStudio.KookaPictura.yml`; bundle/smoke ship, portal dialogs unproven | #647 |
| XC-8 | `testing-strategy.md` | DSSIM/PSNR/ΔE2000, property tests, fuzzing, Criterion benches | MISSING | `testkit` abs-diff only | #648 |
| XC-9 | `update-and-versioning.md` | `--version`, About dialog with Qt/GPU info, update checker | MISSING | `main.cpp` | #649 |
| XC-10 | `cs6-editions-and-constraints.md` | Edition gating (Standard/Extended) | MISSING | no `Edition::` | #650 |
| XC-11 | `performance-targets.md` | Performance instrumentation (cache/scratch/frame/metrics) | MISSING | none found | #651 |

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

- **Linked/embedded objects** (`LAY-024`) — the *app* has no relink or
  convert-to-linked surface, but the codec is not blind to the feature: it
  parses and writes the `lnkD`/`lnk2`/`lnk3` linked records
  (`crates/pictura-codec/src/smart_object.rs:199`). The relink UI is tracked by
  `#113`; the records are read, not offered.
- **Snap and AppImage packaging** — not a non-goal. Both ship alongside
  Flatpak and the native packages (`snap/snapcraft.yaml`,
  `.github/workflows/snap.yml`, `.github/workflows/appimage.yml`).

## F. Existing issues already tracking gaps

`#102`/`#103`/`#104`/`#309` crop · `#113` linked smart objects · `#166` smart
filters · `#167`/`#183` filter bundles · `#172` Quick Mask · `#261` vector-mask
path authoring · `#262` pattern fill layers · `#263` Edit Adjustment row ·
`#264` Blend-If UI · `#265` Layer Comps · `#301` Actions · `#305` View Snap ·
`#306` canvas right-click menus · `#312` Alt-drag duplicate · `#313` picker
eyedropper · `#319` Color panel · `#344`/`#345`/`#346`/`#347`/`#349`/`#350`/`#351`
filters & watermark · `#364` Maximum filter · `#367` Help menu · `#392`/`#396`–`#402`
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
- **Corrections from the `#785` sweep.** Five rows in this register were wrong,
  and have been fixed or removed:
  - `FILT-1` (Lighting Effects engine, `#592`) — the engine ships
    (`crates/pictura-filters/src/render/lighting.rs`, mapped at
    `filter_map/mapping.rs` (Lighting Effects arm), descriptor at
    `filter_map/descriptor/render.rs`). The row is removed and
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
