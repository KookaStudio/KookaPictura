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
| Panels | 19 of 22 populated; Gradients, Patterns and Styles are placeholders |
| Automation | The control server, Rust command catalogue, Actions engine and command bus ship; batch, scripting and plug-ins are open |
| HDR tone mapping | Shipped — all four conversion methods; the Local Adaptation Toning Curve/Corner is open |
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
- [x] **Save As format options dialogs** for JPEG, PNG, BMP and TIFF, with the
  JPEG quality/format/matte, PNG compression, and TIFF compression/transparency
  options honored and the last-used settings remembered. `crates/pictura-app/cpp/jpeg_options_dialog.cpp`,
  `png_options_dialog.cpp`, `bmp_options_dialog.cpp`, `tiff_options_dialog.cpp`.
- [ ] The Save As options the Qt writer cannot honor (BMP format/depth/RLE/flip,
      PNG interlace, TIFF pixel/byte order, pyramid, layer compression) and the
      GIF, EPS, DCS, Raw, TGA and Cineon dialogs (#625)
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
- [x] **Layer styles and effects** — the ten `lfx2` effects, with the Layers
  panel's nested `Effects` group and per-effect rows (#960).
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
- [x] Pattern fill layers using the open document's embedded patterns (#262)
- [ ] Pattern preset library and `Edit > Define Pattern` (#262, #599)
- [ ] Vector-mask path authoring (#261)
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
- [x] Crop Settings menu: Classic mode, shield colour and opacity, Show Cropped
      Area, Auto Center Preview (#104)
- [x] Eyedropper, colour sampler, ruler, note and count
- [x] The Eyedropper and the Color Picker's eyedropper sample continuously
      while dragging (#1040)
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
- [x] Hand and Zoom multi-window options: Scroll All Windows, Zoom All
      Windows, Resize Windows To Fit, and the Zoom Resizes Windows preference
      (#500)
- [ ] Hand and Zoom options: Scrubby and animated zoom (#533), plus Bird's Eye
      View and Flick Panning (#534)
- [ ] Ruler protractor and Straighten from the ruler (#535); eyedropper
      modifiers and Shift-click sampler (#536)
- [ ] Clone Source overlay controls (#537)
- [ ] Healing Mode, Source and Sample, Sample All Layers, and depth and mode
      handling (#538)
- [ ] Merged and composite History Brush sources, and Edit ▸ Fill ▸ History
      (#539)
- [x] Mixer Brush Sample All Layers (#911)
- [ ] Mixer Brush Load Solid Colors Only and keyboard shortcuts (#540)
- [x] Type on path and area type (#683)
- [x] Warp text (#684)
- [x] Gamma blend and missing-font handling (#685)
- [x] Missing Glyph Protection (#887)
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
- [x] The HDR Conversion tone methods (#557)
- [ ] Merge To HDR Pro (#558)
- [ ] The Local Adaptation Toning Curve and Corner option (follow-up to #557)
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
- [x] Convert to Profile options, mismatch and missing-profile warnings, and
      the embed toggle (#568)
- [x] CMYK conversions use the document or working ICC profile (#809)
- [x] Color Settings: working CMYK space, RGB/CMYK policies, conversion options,
      and profile warnings (#569)
- [ ] Color Settings `.csf` save/load, Gray/Spot working spaces, and installed
      profile discovery (#569)
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
      `crates/pictura-filters/src/catalogue/`.
- [x] **Liquify, Oil Paint, the Blur Gallery, and the Filter Gallery.**
      `crates/pictura-app/cpp/filter_gallery_dialog.cpp`,
      `crates/pictura-app/cpp/blur_gallery_dialog.cpp`.
- [ ] Reduce Noise, including the Advanced per-channel view (#593)
- [x] The Radial Blur Blur Center, honored by the kernel (#594)
- [ ] The Smart Sharpen Basic and Advanced split, and the 32-bpc disable (#595)
- [ ] The eight Stylize filters missing from the Filter Gallery category (#596)
- [ ] Colored Pencil document-background paper wiring (#597)
- [x] Adaptive Wide Angle (#346): Fisheye, Perspective and Full Spherical,
      line and polygon constraints, EXIF focal-length prefill
- [ ] Adaptive Wide Angle Auto and lens profiles, Panorama, Smart Filter
      re-edit (#891)
- [x] Lens Correction (#347): Remove Distortion, three fringe sliders,
      Vignette, Vertical/Horizontal Perspective, Angle and Scale; Edge fill
      and Auto Scale; Straighten, Remove Distortion and Move Grid tools;
      EXIF profile search over an in-tree profile format; camera (3,860
      bodies) and lens (lensfun's list) catalogues for the Search Criteria,
      lenses filtered by the photo's focal length
- [x] Lens Correction profiles for real lenses (#932): the system's lensfun
      database read at runtime (none bundled) and user profiles; sub-profiles
      by focal length, f-stop, and focus distance with the right-click menu;
      Prefer RAW Profiles; settings carry the resolved coefficients; Search
      Online and Save Online Profile Locally stay disabled
- [ ] Lens Correction Save/Load Settings and Set Lens Default (#933)
- [x] Lens Correction as a Smart Filter (#934): an embedded Smart Object
      attaches a Lens Correction smart filter instead of filtering the proxy
      destructively, re-opens the dialog prefilled, and renders through the
      composite; the Layers panel gives the Smart Filters group and each filter
      their CS6 context menus (Edit Smart Filter…, Delete Filter Mask), and
      smart filter masks are read and rendered (#956); the descriptor is
      Pictura-only
- [x] Filters on a Smart Object become Smart Filters (#984): every Filter-menu
      filter, Last Filter and a Filter Gallery stack (one "Filter Gallery"
      entry) append a Pictura-only Smart Filter; a selection becomes the
      filter mask, authored as `FEid`/`FMsk` on save; Liquify, Vanishing Point,
      Lens Blur, Displace, Adaptive Wide Angle and Image > Adjustments are
      greyed out on a Smart Object
- [ ] Shadow/Highlight and Variations as Smart Filters (#996)
- [x] Re-edit generic and Filter Gallery Smart Filters (#997): double-click
      reopens the dialog prefilled, after CS6's stacked-filters notice
- [ ] Per-Smart-Filter blend mode and opacity (#998)
- [ ] Photoshop validation of authored filter masks (#999)
- [ ] White filter mask without a selection and Add Filter Mask (#1000)
- [ ] Lens Blur, Displace and Adaptive Wide Angle as Smart Filters (#1001)
- [ ] Smart Filter chain rendered off the UI thread (#1002)
- [ ] Dialog pane preview over an existing Smart Filter chain (#1003)
- [ ] Fade disabled on a Smart Object once Fade ships (#1004)
- [x] Vanishing Point planes (#349): Create and Edit Plane, validity colours,
      Grid Size, related planes torn off at 90° with Angle, Render Grids
- [x] Vanishing Point Marquee, Stamp, Brush, Transform, Eyedropper and Paste
      (#349): floating selections in perspective with Feather, Opacity and
      Heal, cloning and painting that foreshorten with the planes
- [x] Vanishing Point planes saved in PSD, PSB, TIFF and JPEG (#918), in a
      project-owned image resource; reopening restores them
- [x] NTSC Colors (#345)
- [ ] De-Interlace (#344)
- [x] Embed and Read Watermark (#350, #351): an in-tree approximate
      spread-spectrum carrier (not Digimarc-compatible), with the copyright
      title indicator on detection
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
- [x] Color-picker gamut and web alerts, and the picker preference (#600)
- [ ] Color Libraries (PANTONE) (#600)
- [ ] The HUD colour picker (#601)
- [ ] The HDR colour picker at 32 bpc (#602)
- [x] The out-of-gamut triangle and CMYK to working space (#604)
- [ ] The 16/32-bit picker fields (#604)
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

- [x] **Nineteen populated panels** — Layers, History, Channels, Paths, Color,
      Swatches, Info, Histogram, Navigator, Notes, Properties, Adjustments,
      Brush, Clone Source, Character, Paragraph, Glyphs, Paragraph Styles and
      Actions. `crates/pictura-app/cpp/frame_build.cpp`.
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
- [x] Character panel font style, language, tsume, underline, fractional widths,
      faux bold, and italic (#517)
- [x] The Paragraph Justification dialog and RTL direction (#518)
- [x] The Character Styles panel (#519)
- [ ] Navigator proxy colour, Ctrl-drag proxy positioning, zoom buttons, and
      typed zoom (#520)
- [ ] Make Work Path, clipping-path designation, and the panel-options
      thumbnail size (#521)
- [ ] Notes Author and Colour options, and the PSD, PDF and TIFF annotation
      round-trip (#522)
- [x] **The Actions panel and its engine** — the set/action/step tree, Button
      mode, command and Brush/Pencil tool-stroke recording, playback including
      conditional steps, the panel menu, action/step dialogs, exact version-16
      `.atn` read/write, and atomic `Actions Palette.psp` persistence.
      `crates/pictura-app/cpp/panels/actions_panel.cpp`,
      `crates/pictura-script/`, `crates/pictura-codec/src/action_file.rs`
      (#301).

## Automation

- [x] **A non-interactive `--headless` mode.**
      `crates/pictura-app/cpp/main.cpp`.
- [x] **A local control server** on a Unix socket, with twenty JSON methods for
      documents, layers, selections, filters, and pointer and key input.
      `crates/pictura-app/cpp/control_server.cpp`.
- [x] **A Rust command catalogue and the shared command-recording bus** — the
      catalogue drives the menu tree and the checked-in `command_ids.h`; the
      C++ command bus gains executors, context handlers, a recording sink, and
      `executeRecorded` (#273, #616).
      `crates/pictura-script/src/command_catalogue.rs`,
      `crates/pictura-app/cpp/commands.cpp`.
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

- [x] **The application frame** and its ten top-level menus, with 413 live
      commands. `crates/pictura-app/cpp/commands.cpp`.
- [x] **Preferences** — General, Interface, Performance, Units and Guides.
      `crates/pictura-app/cpp/preferences_dialog.cpp`.
- [x] **Live UI size** — Window ▸ UI Size row (minus, readout, plus, reset) and
      an Interface ▸ UI Size control, 75–200 % in 10 % steps, applied without a
      restart and persisted. `Ctrl+Alt++`, `Ctrl+Alt+-`, and `Ctrl+Alt+0` step
      or reset it; the tools column scrolls so the frame stays resizable at
      every scale. Not CS6's four-step restart-only setting; added by owner
      decision (#798). `crates/pictura-app/cpp/theme.cpp`.
- [x] **Help ▸ Search** (`Shift+Space`) — finds any command, tool, panel,
      Preferences page or key panel control by name, shows where it lives, and
      runs or reveals it. Not a CS6 feature; added by owner decision.
      `crates/pictura-app/cpp/search_bar.cpp`.
- [x] **Help ▸ System Info and Submit Issue** — a snapshot environment report
      with Copy, and a prefilled GitHub issue carrying the same report.
      `crates/pictura-app/cpp/system_info.cpp`.
- [ ] Status bar readouts and the status-bar options popup (#497)
- [ ] The active layer name in the title bar (#498)
- [x] `Ctrl+Tab` and `Shift+Ctrl+Tab` document cycling (#499)
- [x] Window ▸ Arrange: Float, Tile, Cascade, Match and New Window (#500)
- [x] Document tab tear-out into floating windows, docking a floating window
      back by its header (drop, Ctrl+drag, or double-click), top-level
      floating windows on Wayland, and the tab and header context menus (#317)
- [x] The full document label in the tab tooltip (#917)
- [ ] The open-document list at the bottom of the Window menu (#923)
- [ ] Menu customization: Edit ▸ Menus, Show All Menu Items, and the
      enablement matrix (#501)
- [ ] The Type menu: work path, convert to shape, and spellcheck (#502)
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
- [x] Safe paths and decoder decompression-bomb guards (#646)
- [ ] Flatpak portal confinement (#647)
- [ ] DSSIM, PSNR and ΔE2000, property tests, fuzzing, and Criterion benches
      (#648)
- [ ] `--version`, an About dialog with the Qt and GPU details, and an update
      checker (#649)
- [ ] Edition gating for Standard and Extended (#650)
- [x] Opt-in performance metrics: phase timings, edit records and GPU counters
      (#651)
- [x] Parallel CPU kernels for adjustments, effect mattes, clipping and the
      display colour transform, byte-identical to the serial path (#1011)
- [ ] Frame-budget instrumentation (#941)
- [ ] A tile cache with hit-rate accounting (#942)
- [ ] Scratch paging and Efficiency accounting (#943)
- [ ] A tile job scheduler with cancellation (#944)
- [ ] Per-tile filter cancellation (#945)
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
