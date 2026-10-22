# Kooka Pictura Roadmap

Kooka Pictura is a photo editor for Linux with Photoshop-format (PSD/PSB) files
at its core. This page lists what works today, what is still open, and what is
out of scope by design.

Legend: `[x]` shipped · `[ ]` open. Every open item names the GitHub issue
that tracks it, so this page is an index of the backlog rather than a second
copy of it. Each area below holds both what ships and what does not, so
shipped work and the work still in front of it stay in one place.

The authoritative gap register is
[`docs/dev/doc-code-parity-gaps.md`](docs/dev/doc-code-parity-gaps.md): every
gap between the specification corpus and the code, each with a status, an
evidence path, and a tracking issue (#489–#651, filed under #488). The open
items below group those gaps by area; the register holds the detail.

Deeper technical status for PSD/PSB lives in
[`docs/dev/psd-support-roadmap.md`](docs/dev/psd-support-roadmap.md).

## At a glance

| Area | Status |
|---|---|
| Files, formats and metadata | Shipped — PSD/PSB round-trip, import, export, print, EXIF/IPTC/XMP |
| Layers | Shipped — groups, masks, clipping, 27 blend modes, styles, fill and adjustment layers, smart objects |
| Tools | 61 of 71 catalogue tools; the ten gaps are the CS6 Extended 3D object and camera tools |
| Selections | Shipped — marquee, lasso, magnetic lasso, wand, quick selection, Refine Edge, Colour Range, saved selections |
| Adjustments | Shipped — the classic families, with live preview and adjustment-layer variants |
| Filters | Shipped — 110 filters across the classic families; three CS6 families have no engine |
| Colour and painting | Shipped — ICC profiles, Assign Profile, Convert to Profile, colour settings, the brush engine |
| Camera raw files | Open — there is no raw container reader and no demosaic |
| Panels | 18 of 22 populated; Actions, Gradients, Patterns and Styles are placeholders |
| Automation | The local control server ships; Actions, batch, scripting and plug-ins are open |
| HDR tone mapping | One of four methods |
| Layer comps, preset manager, stack modes, Blend-If UI | Open |

## Files, formats and metadata

- [x] **PSD/PSB round-trip.** Gray, RGB, CMYK, Lab, Indexed and Bitmap at
  8/16/32-bit, with raw, RLE and ZIP compression, preserving blocks the engine
  does not model. `crates/pictura-codec/src/read.rs`,
  `crates/pictura-codec/src/write.rs`.
- [x] **Import.** PSD, PSB, PNG, JPEG, TIFF, WebP, GIF, BMP, and PDF and EPS
  through Ghostscript. `crates/pictura-app/cpp/frame_import.cpp`.
- [x] **Export.** PSD, PSB, PNG, JPEG, TIFF, WebP, BMP, GIF, SVG and Photoshop
  PDF, plus Save for Web with palette reduction, dithering and transparency.
  `crates/pictura-app/cpp/export_as_dialog.cpp`,
  `crates/pictura-app/cpp/save_for_web_dialog.cpp`.
- [x] **Print.** A print dialog over `QPrinter`, with printer-managed colour and
  a "Save as PDF" option. `crates/pictura-app/cpp/print_dialog.cpp`.
- [x] **Metadata.** EXIF, IPTC and XMP reading, six editable IPTC fields, File
  Info, and XMP metadata templates. `crates/pictura-codec/src/exif.rs`,
  `crates/pictura-codec/src/xmp.rs`.
- [ ] Save As per-format option dialogs: TIFF, JPEG, PNG, GIF, EPS, DCS, Raw,
      BMP, TGA and Cineon (#625)
- [ ] JPEG 2000 compression, Spot Colors, and Use Proof Setup in the PDF writer
      (#626)
- [ ] Native CMYK and 16-bit PDF and EPS rendering, Images mode on encrypted
      PDFs, inline images, and the 3D option (#627)
- [ ] GIF-via-Indexed on save, and Save a Copy (#628)
- [ ] The Save for Web slice HTML and CSS emitter, 2-Up and 4-Up, the
      colour-table editor, and the metadata menu (#629)
- [ ] Data Sets As Files, Zoomify, Render Video, and Paths to Illustrator
      (#630)
- [ ] The File Info tabs for IPTC Extension, GPS, Origin, Video, Audio,
      Photoshop and DICOM, and the editing they imply (#631)
- [ ] Print colour management, proofing, output marks, and persisted defaults
      (#634)
- [ ] The measurement scale and presets, the exporters, and PSD resource 1074
      (#635)
- [ ] Background Save, a durable crash journal, and PSD 1086/1087 (#637)
- [ ] Edit Contents and relink for vector smart objects, Paste As, and the
      Files panel (#638)
- [ ] The Maximize PSD/PSB Compatibility preference (#639)
- [ ] The document-format version policy: `opDocumentVersion`,
      `minReaderVersion` and Version Info 1057 (#640)
- [ ] The Preset Manager: eight preset types, load, append, replace, reset and
      save-set, and the `.abr`, `.aco`, `.ase`, `.asl`, `.grd`, `.pat`, `.csh`
      and `.tpl` library formats (#489)

## Layers

- [x] **Groups and nesting**, with background-layer semantics.
  `crates/pictura-core/src/layer_stack.rs`.
- [x] **The layer compositor**, which resolves groups, masks, and blend modes.
  `crates/pictura-render/src/composite.rs`.
- [x] **Raster and vector masks, and clipping masks.**
  `crates/pictura-render/src/document_ops/layer_ops/`.
- [x] **27 blend modes**, plus Pass Through for groups.
  `crates/pictura-core/src/lib.rs`.
- [x] **Layer styles and effects** — the ten `lfx2` effects.
  `crates/pictura-render/src/layer_effects/`.
- [x] **Fill and adjustment layers.** `crates/pictura-render/src/fill.rs`.
- [x] **Shape and type layers.** `crates/pictura-render/src/type_layer.rs`.
- [x] **Smart objects and smart filters.**
  `crates/pictura-render/src/smart_filter.rs`.
- [x] **Merge, flatten, and Merge Clipping Mask.**
  `crates/pictura-render/src/document_ops/layer_ops/merge.rs`.
- [x] **Align and distribute**, by content bounds and against the selection.
  `crates/pictura-render/src/document_ops/layer_ops/align.rs`.
- [x] **Locks, colour labels, and layer filtering.**
  `crates/pictura-app/cpp/panels/layers_panel.cpp`.
- [ ] Layer comps — create, apply, update and delete (#265)
- [ ] Advanced blending toggles: Blend Clipped Layers as Group, Transparency
      Shapes Layers, Mask Hides Effects and channel Include. Blend Interior
      Effects as Group is stored but never read (#579)
- [ ] Layer-style Global Light, Create Layers, and Rasterize Layer Style (#580)
- [ ] Bevel Contour and Texture, a gradient editor in the style dialog, and
      channel include (#581)
- [ ] Auto-Align Layers (#582) and Auto-Blend Layers (#583)
- [ ] Fill With (Mode)-Neutral Color for new layers (#584)
- [ ] 32-bit and Lab blend-mode menu gating (#589)
- [ ] `Ctrl+J` duplicate-group semantics (#590)
- [ ] Smart-object Stack Modes (#591)
- [ ] Pattern fill layers (#262) and vector-mask path authoring (#261)
- [ ] A Blend-If editor UI (#264)
- [ ] Relink and convert-to-linked for smart objects (#113)
- [ ] The Layers panel thumbnail-size option and the `00` / `Shift+00` opacity
      shortcut (#523, #586)
- [ ] Shape layer naming `<Tool> N` (#588)
- [ ] Export Layers To Files (#587)
- [ ] Merge Down's result appearance when the target's blend mode or opacity is
      non-default (#813)

## Tools

The catalogue holds 71 tool slots in 23 toolbox groups, of which 21 are
built. `crates/pictura-app/cpp/tool_catalog.cpp` is the list.

- [x] Move
- [x] Rectangular and elliptical marquee
- [x] Lasso, polygonal lasso and magnetic lasso
- [x] Magic wand and quick selection
- [x] Crop, perspective crop, slice and slice select
- [x] Eyedropper, colour sampler, ruler, note and count
- [x] Spot healing, healing brush, patch, content-aware move and red eye
- [x] Brush, pencil, colour replacement and mixer brush
- [x] Clone stamp and pattern stamp
- [x] History brush and art history brush
- [x] Eraser, background eraser and magic eraser
- [x] Gradient and paint bucket
- [x] Blur, sharpen and smudge
- [x] Dodge, burn and sponge
- [x] Pen, freeform pen, and the add/delete/convert anchor point tools
- [x] Horizontal and vertical type, and type masks
- [x] Path selection and direct selection
- [x] Rectangle, rounded rectangle, ellipse, polygon, line and custom shape
- [x] Hand, rotate view and zoom
- [ ] Single Row and Single Column marquee tools (#525)
- [ ] Puppet Warp (#527) and Content-Aware Scale (#528)
- [ ] Transform ▸ Scale, Rotate and Again, the exact rotations, and flips (#529),
      and the Transform Path and Transform Points submenus (#530)
- [ ] Magnetic Pen (#531), and Merge Shape Components — path boolean
      operations (#532)
- [ ] Hand and Zoom options: Scrubby and animated zoom, Resize Windows, Zoom
      All, Scroll All and Match (#533), plus Bird's Eye View and Flick Panning
      (#534)
- [ ] Ruler protractor and Straighten from the ruler (#535); eyedropper
      modifiers and Shift-click sampler (#536)
- [ ] Clone Source overlay controls (#537)
- [ ] Healing Mode, Source and Sample, Sample All Layers, and depth and mode
      handling (#538)
- [ ] Merged and composite History Brush sources, and Edit ▸ Fill ▸ History
      (#539)
- [ ] Mixer Brush Sample All Layers, Load Solid Colors Only, and keyboard
      shortcuts (#540)
- [x] Type on path and area type (#683)
- [ ] Warp text (#684) and gamma blend and missing-font handling (#685) (#541)
- [ ] Path operations, align and arrange, constrained path dragging, the Target
      Path toggle, and segment and marquee multi-select (#542)
- [ ] Slice style and modifiers, Divide, Combine, Duplicate, the Slice Options
      dialog, and slices from guides (#543)
- [ ] Crop resampling by W×H×resolution, non-destructive crop and Reveal All,
      and the crop shield settings (#544)
- [ ] `.CSH` libraries, Define Custom Shape, geometry modes, and gradient and
      pattern fill and stroke (#545)
- [ ] Shape gradient and pattern fill and stroke, dashed stroke, path
      operations, fill-pixels options, snap-to-pixels, and proportional
      geometry (#546)
- [ ] The full Color Replacement mode list, and disabling it on Bitmap, Indexed
      and Multichannel (#547)
- [ ] Grayscale Sponge, the airbrush toggle, quick keys, and 32-bpc
      availability (#548)
- [ ] Magic-eraser background conversion, background-eraser edge halo, and pen
      pressure (#549)
- [ ] Gradient and Paint Bucket availability gates, Gradient Editor
      reachability, and the Gradient Fill dialog (#550)
- [ ] 16-bit and 32-bit operation for Smudge, Blur and Sharpen (#551)
- [ ] The cross-document colour-mode rule, and Pattern Stamp Impressionist
      (#552)
- [ ] Quick Mask (#172)

## Selections

- [x] **Marquee, elliptical, lasso, polygonal, magnetic lasso, magic wand, quick
      selection, and the path selection tools.** `crates/pictura-select/src/`.
- [x] **The Modify set** — Border, Smooth, Expand, Contract, Feather — plus
      Grow and Similar. `crates/pictura-select/src/lib.rs`.
- [x] **Refine Edge**, with its view modes, output targets, and
      decontamination. `crates/pictura-select/src/refine.rs`.
- [x] **Color Range.** `crates/pictura-select/src/color_range.rs`.
- [x] **Save and load selections to channels**, and selection-masked edits.
      `crates/pictura-app/src/cxxqt_object/impl_selection.rs`.
- [ ] Select ▸ Transform Selection (#526)
- [ ] Apply Image (#609) and Calculations (#610)
- [ ] Channel Options — Masked and Selected Areas polarity (#611)
- [ ] The interactive Refine Radius and Erase brushes (#612)
- [ ] Color Range Skin Tones, Localized Clusters, Out Of Gamut, previews, and
      save and load (#613)
- [ ] Engine-wide anti-aliased selection rasterisation, and the 56-channel cap
      (#614)
- [ ] Combine operations, the layer-mask target, and modifier-click combine on
      saved selections (#615)
- [ ] Quick Mask (#172)

## Adjustments and image operations

- [x] **The adjustment families** from Levels to Match Color, with live preview
      and adjustment-layer variants. Each one that still lacks a control names
      it below. `crates/pictura-adjust/src/`.
- [x] **Image size, canvas size, rotate and flip.**
      `crates/pictura-render/src/document_ops/`.
- [x] **Image modes and bit depth** — Gray, RGB, CMYK, Lab, Indexed and Bitmap
      at 8/16/32-bit. `crates/pictura-render/src/document_ops/mode/`.
- [x] **Crop**, with an interactive region and a non-destructive default.
      `crates/pictura-render/src/document_ops/crop.rs`.
- [x] **Transforms** — Free Transform, skew, distort, perspective, and warp.
      `crates/pictura-render/src/document_ops/layer_ops/transform.rs`.
- [ ] Duotone conversion and its Options dialog (#553); Multichannel
      conversion (#554)
- [ ] The Color Table, and Indexed transparency and Forced palette (#555)
- [ ] Bitmap Halftone Screen and Custom Pattern (#556)
- [ ] The remaining HDR Conversion tone methods (#557), and Merge To HDR Pro
      (#558)
- [ ] The Auto Color Correction Options dialog (#559)
- [ ] Levels eyedroppers and the Alt-drag clipping preview (#560)
- [ ] The Curves secondary UI: on-image, pencil and smooth, sliders and
      eyedroppers, display options, Auto, and the CMYK and Lab graphs (#561)
- [ ] The Shadows/Highlights local-surround operator and its eight controls
      (#562)
- [ ] The Threshold histogram under the slider (#563)
- [ ] Vibrance Auto and presets (#564)
- [ ] Smoother, Sharper and Automatic image-size kernels, Scale Styles, Auto
      resolution, picas and columns, and float resample (#565)
- [ ] Picas and columns in Canvas Size, and extension-fill availability gating
      (#566)
- [ ] The arbitrary rotation dialog, the interpolation preference, and the
      ruler-straighten prefill (#567)
- [ ] Convert to Profile options, mismatch and missing-profile warnings, and
      the embed toggle (#568)
- [ ] The full Color Settings dialog: spaces, policies, conversion options and
      `.csf` (#569)
- [ ] 32-bit Preview Options, the HDR colour picker, and the 32-bit Info
      readout (#570)
- [ ] The Channel Mixer Monochrome UI, and the CMYK and Gray output channels
      (#571)
- [ ] Color Balance numeric readouts, and Lab and CMYK mapping (#572)
- [ ] Hue/Saturation Colorize, range-band editing, eyedroppers, and the
      on-image tool (#573)
- [ ] Gradient Map dither, and the CS6 print-toning presets (#574)
- [ ] The Match Color on-canvas eyedropper and live Info feedback (#575)
- [ ] Adjustment presets for Levels, Curves, Photo Filter, Selective Color and
      Vibrance (#576)
- [ ] The Selective Color composite-channel-only gate, and the preset
      whitelist (#577)
- [ ] Brightness/Contrast Auto, and the shared adjustment Auto buttons (#578)

## Filters

- [x] **110 filters** across the Artistic, Blur, Blur Gallery, Brush Strokes,
      Distort, Noise, Pixelate, Render, Sharpen, Sketch, Stylize, Texture and
      Other families. `crates/pictura-filters/src/`,
      `crates/pictura-app/src/cxxqt_object/filter_map/arities.rs`.
- [x] **Liquify, Oil Paint, the Blur Gallery, and the Filter Gallery.**
      `crates/pictura-app/cpp/filter_gallery_dialog.cpp`,
      `crates/pictura-app/cpp/blur_gallery_dialog.cpp`.
- [ ] Reduce Noise, including the Advanced per-channel view (#593)
- [ ] The Radial Blur Blur Center, which the kernel ignores (#594)
- [ ] The Smart Sharpen Basic and Advanced split, and the 32-bpc disable (#595)
- [ ] The eight Stylize filters missing from the Filter Gallery category (#596)
- [ ] Colored Pencil document-background paper wiring (#597)
- [ ] Adaptive Wide Angle (#346), Lens Correction (#347) and Vanishing Point
      (#349)
- [ ] De-Interlace (#344) and NTSC Colors (#345)
- [ ] Embed and Read Watermark (#350, #351)
- [ ] Edit ▸ Fade, the post-filter opacity and blend mode (#491)

## Colour and painting

- [x] **Colour management** — ICC profiles, Assign Profile, Convert to Profile,
      and a colour settings policy. `crates/pictura-color/src/lib.rs`.
- [x] **The brush engine**, with the stroke rasteriser, tips, spacing, and
      paint modes. `crates/pictura-paint/src/`.
- [x] **The colour picker, the Color panel, and swatches.**
      `crates/pictura-app/cpp/panels/color_panel.cpp`,
      `crates/pictura-app/cpp/panels/swatches_panel.cpp`.
- [x] **Gradients and patterns.** `crates/pictura-paint/src/gradient.rs`,
      `crates/pictura-paint/src/pattern.rs`.
- [ ] The Gradient Editor reachable from the Gradient tool, plus opacity stops
      and a midpoint (#598)
- [ ] Define Pattern, the `.pat` library, and Scripted Patterns (#599)
- [ ] Color-picker gamut and web alerts, Color Libraries (PANTONE), and the
      picker preference (#600)
- [ ] The HUD colour picker (#601)
- [ ] The HDR colour picker at 32 bpc (#602)
- [ ] The out-of-gamut triangle, CMYK to working space, and the 16/32-bit
      picker fields (#604)
- [ ] `.aco` and `.ase` swatch libraries, rename, and display modes (#605)
- [ ] Pattern Overlay Snap To Origin, and the style Texture sub-options (#606)
- [ ] Bristle brushes (#607)
- [ ] `.abr` libraries, the Brush Presets panel, and ABR v10 reading (#608)
- [ ] The brush dynamics option sets: texture, dual brush, colour dynamics,
      transfer, pose and padlocks (#492)
- [ ] Sampled, bristle, erodible and airbrush tips, and Define Brush Preset
      (#493)

## Camera raw

- [x] **Pictura Raw**, reached through `Filter > Pictura Raw…`, with nine tabs:
      Basic, Tone Curve, Detail, HSL/Grayscale, Split Toning, Lens Corrections,
      Effects, Camera Calibration and Presets. Eight are wired to a Rust
      renderer; Presets is a stub. It applies to an already-decoded image — it
      does not read a camera raw file.
      `crates/pictura-app/cpp/pictura_raw_dialog.cpp`,
      `crates/pictura-adjust/src/pictura_raw/`.
- [ ] Open camera raw files — there is no raw container reader and no demosaic
      (#400)
- [ ] DNG output (#402)
- [ ] Camera Raw settings stores — an XMP sidecar or an ACR database (#632)
- [ ] ACR Workflow Options, ACR preferences, batch raw processing, and Purge
      Camera Raw (#633)
- [ ] The Pictura Raw Presets tab is a stub list with no persistence (#633)

## Panels

- [x] **Seventeen populated panels** — Layers, History, Channels, Paths, Color,
      Swatches, Info, Histogram, Navigator, Notes, Properties, Brush, Clone
      Source, Character, Paragraph, Glyphs and Paragraph Styles.
      `crates/pictura-app/cpp/frame_build.cpp`.
- [x] **Dockable panels**, a dark theme, a remembered layout, and the
      Essentials, Painting, Photography and Typography workspaces.
      `crates/pictura-app/cpp/frame_workspaces.cpp`.
- [x] **The toolbox, the options bar, and the keyboard-shortcuts editor.**
      `crates/pictura-app/cpp/toolbox.cpp`.
- [x] The Adjustments panel icon grid (#509)
- [ ] Refine the Adjustments panel icon artwork (#819)
- [ ] The Measurement Log panel and the measurement engine (#510)
- [ ] The Tool Presets panel and `.tpl` libraries (#511)
- [ ] The Styles panel and `.asl` presets (#512)
- [ ] Channel panel spot channels, Split and Merge, Channel Options, and Panel
      Options (#513)
- [ ] History Options, Clear History and purge, and New Document from state
      (#514)
- [ ] Histogram view modes, statistics, the source menu, cache and refresh, and
      the 16/32-bit bins (#515)
- [ ] Info panel sampler persistence, the sample-size readout, marquee and
      transform readouts, before/after, and the gamut exclamation (#516)
- [ ] Character panel faux bold and italic, language, tsume, underline, and
      fractional widths (#517)
- [ ] The Paragraph Justification dialog and RTL direction (#518)
- [ ] The Character Styles panel (#519)
- [ ] Navigator proxy colour, Ctrl-drag proxy positioning, zoom buttons, and
      typed zoom (#520)
- [ ] Make Work Path, clipping-path designation, and the panel-options
      thumbnail size (#521)
- [ ] Notes Author and Colour options, and the PSD, PDF and TIFF annotation
      round-trip (#522)
- [ ] The Actions panel and its engine (#301)

## Automation

- [x] **A non-interactive `--headless` mode.**
      `crates/pictura-app/cpp/main.cpp`.
- [x] **A local control server** on a Unix socket, with twenty JSON methods for
      documents, layers, selections, filters, and pointer and key input.
      `crates/pictura-app/cpp/control_server.cpp`.
- [ ] A shared ActionDescriptor and command-recording bus (#616)
- [ ] Create Droplet, the `.opd` format, and the `.desktop` launcher (#617)
- [ ] The File ▸ Automate ▸ Batch engine and dialog (#618)
- [ ] The remaining Automate commands: Image Processor, Conditional Mode
      Change, PDF Presentation, Contact Sheet II, Picture Package, Photomerge,
      Web Photo Gallery and File Naming (#619)
- [ ] A scripting host, DOM, ActionManager and notifiers on QuickJS (#620)
- [ ] A script console, sandbox limits, a security prompt, and the progress
      bridge (#621)
- [ ] Variables and data-driven graphics (#622)
- [ ] A Rust C-ABI plug-in loader and the Plug-in manager dialog (#623)
- [ ] The Mini-Bridge replacement Files panel (#624)

## Application frame, menus and preferences

- [x] **The application frame** and its ten top-level menus, with 411 live
      commands. `crates/pictura-app/cpp/commands.cpp`.
- [x] **Preferences** — General, Interface, Performance, Units and Guides.
      `crates/pictura-app/cpp/preferences_dialog.cpp`.
- [ ] Status bar readouts and the status-bar options popup (#497)
- [ ] The active layer name in the title bar (#498)
- [ ] `Ctrl+Tab` and `Shift+Ctrl+Tab` document cycling (#499)
- [ ] Window ▸ Arrange: Float, Tile, Cascade, Match and New Window (#500)
- [ ] Menu customization: Edit ▸ Menus, Show All Menu Items, and the
      enablement matrix (#501)
- [ ] The Type menu: warp text, convert point and paragraph, work path, convert
      to shape, and spellcheck (#502)
- [ ] The View menu stubs: Proof Setup and Colors, Gamut Warning, Snap To, Lock
      Slices, Extras, Pixel Aspect Ratio, and 32-bit Preview Options (#503)
- [ ] The File menu stubs: Open As, Import, Export, Automate, Scripts, and
      Browse in Bridge (#504)
- [ ] The Cursors, Transparency and Gamut, Plug-ins, Type and 3D preferences
      panes (#505)
- [ ] The sparse existing panes: General, Interface, File Handling, Performance,
      Units and Guides (#506)
- [ ] The Performance preferences pane: memory, history states, cache level and
      tile size, scratch disks and GPU, plus the Efficiency and Scratch readouts
      (#636)
- [ ] AT-SPI2 and `QAccessible`, high contrast, colour-blind and reduced-motion
      support (#507)
- [ ] Shortcut sets: save and load, workspace-assigned, Summarize HTML, and
      Legacy Channel Shortcuts (#508)
- [ ] Quick fill with Alt+Backspace and Ctrl+Backspace (#524)

## Engine and performance

- [x] **A Rust engine under a Qt 6 shell** — ten engine crates, with no Qt type
      in any of them. `crates/`.
- [x] **A wgpu/Vulkan compute path** for compositing, the heavy filters, and the
      live paint stroke, gated behind a one-time comparison against the CPU
      reference. `crates/pictura-render/src/gpu/`.
- [ ] A typed `prefs.toml` store, the config and state split, migration,
      quarantine, and the reset gesture (#641)
- [ ] `QTranslator` and `.qm`, RTL, and locale number formatting (#642)
- [ ] `tracing` with a file sink, the Qt-to-Rust bridge, and the History Log
      (#643)
- [ ] Opt-in telemetry and `KOOKA_PICTURA_NO_TELEMETRY` (#644)
- [ ] A stable error-code registry, and the FFI `OpStatus` and `catch_unwind`
      boundaries (#645)
- [ ] Safe paths and decoder decompression-bomb guards (#646)
- [ ] Flatpak portal confinement (#647)
- [ ] DSSIM, PSNR and ΔE2000, property tests, fuzzing, and Criterion benches
      (#648)
- [ ] `--version`, an About dialog with the Qt and GPU details, and an update
      checker (#649)
- [ ] Edition gating for Standard and Extended (#650)
- [ ] Performance instrumentation: cache, scratch, frame and metrics (#651)
- [ ] Global bit-depth and colour-mode availability gating (#490)

## Specification corpus

- [ ] The missing `01-architecture/build-and-packaging.md` spec, and the
      dangling references to it (#494)
- [ ] The architecture specs' crate map, which still names crates that were
      never created (#495)
- [ ] `docs/TRACEABILITY.md`, whose every row is still `draft` (#494)

## Not planned

Kooka Pictura targets photo editing. The following are out of scope by
design rather than merely unscheduled. The rationale and the revisit
condition behind each live in
[`docs/00-overview/feasibility-and-non-goals.md`](docs/00-overview/feasibility-and-non-goals.md).

- 3D tools and 3D layers — the 3D object and camera tools sit in the tool
  catalogue but are not built, and there is no 3D panel
- Video, the timeline, and video layers
- The measurement engine, its Measurement Log panel, and DICOM — the Count tool
  ships, the rest do not
- Print fidelity matching Adobe's engine
- Adobe cloud services (Creative Cloud, Cloud Documents, Behance, Stock)
- Adobe Camera Raw parity — Pictura Raw is an independent approximation
- Adobe plug-in (`.8bf`) binary compatibility
- ExtendScript and JSX compatibility — a substitute automation API is planned
  instead
- Bundled Adobe assets (brushes, patterns, gradients, profiles, fonts)
- Artboards — a post-CS6 feature
- Extract and Pattern Maker — removed from the default install in CS4 and
  optional downloads in CS6, so not a parity target. The Filter menu still
  shows the rows
- Byte-exact PSD output (preserve-and-round-trip instead)
