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

The quantified comparison against Photoshop CS6 and CC 28.0 is in
[`PARITY.md`](PARITY.md).

## At a glance

| Area | Status |
|---|---|
| Files, formats and metadata | Shipped — PSD/PSB round-trip, import, export, print, EXIF/IPTC/XMP |
| Layers | Shipped — groups, masks, clipping, 27 blend modes, styles, fill and adjustment layers, smart objects |
| Tools | 61 of 71 catalogue tools; the ten gaps are the CS6 Extended 3D object and camera tools |
| Selections | Shipped — marquee, lasso, magnetic lasso, wand, quick selection, Refine Edge, Colour Range, saved selections |
| Adjustments | Shipped — the classic families, with live preview and adjustment-layer variants |
| Filters | Shipped — 110 filters across the classic families; the roadmap names the remaining gaps |
| Colour and painting | Shipped — ICC profiles, Assign Profile, Convert to Profile, colour settings, the brush engine |
| Camera raw files | DNG/CR2/NEF/ARW route ships with fitted per-camera matrices (approximate); smart-object open and the other vendor containers remain (#1466, #399) |
| Panels | 19 of 22 populated; Gradients, Patterns and Styles are placeholders |
| Automation | The control server, Rust command catalogue, Actions engine, command bus and Batch ship; droplets, scripting and plug-ins are open |
| HDR tone mapping | Shipped — all four conversion methods; the Local Adaptation Toning Curve/Corner is open |
| Preset manager, Blend-If UI | Open |

## Files, formats and metadata

- [x] **PSD/PSB round-trip.** Gray, RGB, CMYK, Lab, Indexed and Bitmap at
  8/16/32-bit, with raw, RLE and ZIP compression, preserving blocks the engine
  does not model. `crates/pictura-codec/src/read.rs`,
  `crates/pictura-codec/src/write.rs`.
- [x] **Import.** PSD, PSB, PNG, JPEG, TIFF, WebP, GIF, BMP, PICT, and PDF
  and EPS through Ghostscript. `crates/pictura-app/cpp/frame_import.cpp`.
- [x] **Multi-file selection in File > Place**, with one Free Transform session
  per file and a summary for files that could not be placed (#755).
- [x] **Non-destructive canvas fit for oversized Place sources**, keeping the
  native embedded source and its alpha through scale/translation (#755).
- [x] **Export.** PSD, PSB, PNG, JPEG, TIFF, WebP, BMP, GIF, SVG, PICT and
  Photoshop PDF, plus Save for Web with palette reduction, dithering and
  transparency. `crates/pictura-app/cpp/export_as_dialog.cpp`,
  `crates/pictura-app/cpp/save_for_web_dialog.cpp`.
- [x] **Print.** A print dialog over `QPrinter`, with printer-managed colour and
  a "Save as PDF" option. `crates/pictura-app/cpp/print_dialog.cpp`.
- [x] **Metadata.** EXIF, IPTC and XMP reading, fourteen editable IPTC and
  IPTC Extension fields, File Info with Description, IPTC, Origin, IPTC
  Extension, Photoshop and editable raw XMP tabs, and XMP metadata templates.
  `crates/pictura-codec/src/exif.rs`, `crates/pictura-codec/src/xmp/mod.rs`.
- [x] **Save As format options dialogs** for JPEG, PNG, BMP and TIFF, with the
  JPEG quality/format/matte, PNG compression, and TIFF compression/transparency
  options honored and the last-used settings remembered. `crates/pictura-app/cpp/jpeg_options_dialog.cpp`,
  `png_options_dialog.cpp`, `bmp_options_dialog.cpp`, `tiff_options_dialog.cpp`.
- [x] The Save As options the Qt writer cannot honor (BMP format/depth/RLE/flip,
      PNG interlace, TIFF pixel/byte order, pyramid, layer compression) and the
      GIF, EPS, DCS, Raw, TGA, Cineon and PICT dialogs (#625, #1503).
      `crates/pictura-codec/src/save/`, `crates/pictura-app/cpp/gif_options_dialog.cpp`,
      `eps_options_dialog.cpp`, `dcs_options_dialog.cpp`, `raw_options_dialog.cpp`,
      `tga_options_dialog.cpp`, `cineon_options_dialog.cpp`,
      `pict_options_dialog.cpp`.
- [x] JPEG 2000 compression, Spot Colors, and Use Proof Setup in the PDF writer
      (#626)
- [x] Native CMYK and 16-bit PDF and EPS rendering, Images mode on encrypted
      PDFs, and inline images (#627)
- [x] File ▸ Open As lets the user choose the decoder for supported formats (#627)
- [x] GIF-via-Indexed on save (#628).
      `crates/pictura-app/cpp/gif_options_dialog.cpp`;
      `crates/pictura-codec/src/save/mod.rs`.
- [x] Save a Copy (#628)
- [x] The Save for Web slice HTML and CSS emitter, 2-Up and 4-Up, the
      colour-table editor, and the metadata menu (#629)
- [x] **Zoomify and Paths to Illustrator** (#630)
- [x] Data Sets As Files (#622)
- [ ] Render Video (deferred Extended-only, codec backend needed) (#630)
- [x] The File Info tabs for IPTC Extension, GPS, Origin, Video, Audio,
      Photoshop and DICOM, and the editing they imply (#631)
- [ ] Print colour management, proofing, output marks, and persisted defaults
      (#634). Implementation is ready for CI and manual Photoshop CS6, Photoshop
      CC, and physical CUPS acceptance.
- [x] Measurement scale and presets, measurement TSV and histogram CSV writers,
      and PSD/PSB resource 1074 (#635)
- [x] Background Save, a durable crash journal, and PSD 1086/1087 (#637)
- [x] Vector Smart Object placement and Edit Contents, PDF Paste As, and
      repair of supported external Smart Object links (#638)
- [ ] Editable Illustrator clipboard paste as Paths or Shape Layer (#638)
- [x] The Maximize PSD/PSB Compatibility preference (#639)
- [x] The document-format version policy: `opDocumentVersion`,
      `minReaderVersion` and Version Info 1057 (#640)
- [x] **Preset Manager** for eight preset types, with library loading,
      discovery, current lists in settings backup, and native Save Set. Imports
      supported `.abr`, `.aco`, `.ase`, `.asl`, `.csh`, `.grd`, `.pat`, `.shc`,
      and `.tpl` variants. Exports ACO and ASE swatches and provides a standalone
      `.csf` codec (#489).
- [x] PICT read and write (#1503). Bitmap-only PICT v1/v2 (PackBits, DirectBits
  and QuickTime JPEG read; 16/32-bit PackBits and JPEG write); QuickDraw vector
  records are skipped and a vector-only picture is refused.
- [ ] IFF, PCX and SGI read and write (#1504, #1505, #1506)
- [ ] OpenEXR, JPEG 2000, HEIC/HEIF and AVIF read and write (#1507, #1508, #1509, #1510)
- [ ] SVG import (#1511)
- [ ] Content Credentials (#1512)
- [ ] Adobe interoperability for editable Photoshop PDFs (#1513)
- [ ] The remaining Save for Web output settings (#1514)

## Layers

- [x] **Groups and nesting**, with background-layer semantics.
  `crates/pictura-core/src/layer_stack.rs`.
- [x] **The layer compositor**, which resolves groups, masks, and blend modes.
  `crates/pictura-render/src/composite.rs`.
- [x] **Raster and vector masks, and clipping masks.**
  `crates/pictura-render/src/document_ops/layer_ops/`.
- [x] **PSD mask linkage** uses the Photoshop flag polarity, and unlinked
  raster/vector mask controls remain visible at 50% opacity (#1200).
- [x] **Canvas-wide raster mask editing**, with black movement exterior and
  retained off-canvas coverage (#1199).
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
- [x] Layer comps — create, apply, update and delete (#265)
  `crates/pictura-app/cpp/panels/layer_comps_panel.cpp`; `crates/pictura-codec/src/layer_comps.rs`.
- [x] Advanced blending engine: Blend Clipped Layers as Group, Transparency
      Shapes Layers, Mask Hides Effects, channel Include and Blend Interior
      Effects as Group (#579)
- [ ] Layer Style dialog controls for the advanced-blending toggles (#695)
- [x] Layer-style Global Light, Create Layers, and Rasterize Layer Style (#580)
- [x] Bevel Contour, the style dialog's gradient editor, and channel include
      (#581)
- [x] Auto-Align Layers (#582)
- [x] Auto-Blend Layers (#583)
- [x] Fill With (Mode)-Neutral Color for new layers (#584).
  `crates/pictura-app/cpp/layer_new_dialog.cpp`
- [x] 32-bit and Lab blend-mode menu gating (#589)
- [x] `Ctrl+J` duplicate-group semantics (#590)
- [x] Smart-object Stack Modes (#591)
- [x] Pattern fill layers using the open document's embedded patterns (#262)
- [ ] Pattern preset library and `Edit > Define Pattern` (#262, #599)
- [x] Vector-mask path authoring: Current Path from the selected path, the Paths
  panel vector-mask row, and the outlined mask thumbnail (#261)
- [ ] A Blend-If editor UI (#264)
- [ ] Relink and convert-to-linked for smart objects (#113)
- [x] The Layers panel thumbnail-size option (#586).
  `crates/pictura-app/cpp/panels/layers_panel.cpp`
- [ ] The `00` / `Shift+00` layer opacity shortcut (#523)
- [x] Shape layer naming `<Tool> N` (#588).
  `crates/pictura-render/src/document_ops/layer_ops/shape_layer.rs`
- [x] **Export Layers To Files — core** (#587)
- [ ] Export Layers' shared Save As option controls and Targa (#587, #625)
- [ ] Merge Down's result appearance when the target's blend mode or opacity is
      non-default (#813)

## Tools

The catalogue holds 71 tool slots in 23 toolbox groups; 61 are built. The ten
gaps are the CS6 Extended 3D object and camera tools.
`crates/pictura-app/cpp/tool_catalog.cpp` is the list.

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
- [x] Airbrush on the stamp tools and History Brush; Art History Brush paints
      while the button is held (#799)
- [x] The Brush's Airbrush option and Shift+Alt+P (#1120)
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
- [x] Transform ▸ Scale, Rotate and Again, the exact rotations, and flips (#529).
  `crates/pictura-script/src/command_catalogue/entries.rs`
- [ ] The Transform Path and Transform Points submenus (#530)
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
- [ ] Bristle brush preview, tilt projection, and preset keys (#1520)
- [x] Type on path and area type (#683)
- [x] Warp text (#684)
- [x] Gamma blend and missing-font handling (#685)
- [x] Missing Glyph Protection (#887)
- [ ] Variable-font axis controls (#1518)
- [ ] Dynamic text on shapes and paths (#1519)
- [ ] Path operations, align and arrange, constrained path dragging, the Target
      Path toggle, and segment and marquee multi-select (#542)
- [ ] Slice style and modifiers, Divide, Combine, Duplicate, the Slice Options
      dialog, and slices from guides (#543)
- [x] Crop resampling by W×H×resolution, non-destructive crop and Reveal All,
      and the crop shield settings (#544)
- [ ] `.CSH` libraries, Define Custom Shape, geometry modes, and gradient and
      pattern fill and stroke (#545)
- [ ] Shape gradient and pattern fill and stroke, dashed stroke, path
      operations, fill-pixels options, snap-to-pixels, and proportional
      geometry (#546)
- [x] The full Color Replacement mode list, and disabling it on Bitmap, Indexed
      and Multichannel (#547)
- [x] Grayscale Sponge, the airbrush toggle, quick keys, and 32-bpc
      availability (#548)
- [x] 16-bit Dodge, Burn and Sponge operation (#1421)
- [ ] Magic-eraser background conversion, background-eraser edge halo, and pen
      pressure (#549)
- [ ] Gradient and Paint Bucket availability gates, Gradient Editor
      reachability, and the Gradient Fill dialog (#550)
- [ ] 16-bit and 32-bit operation for Smudge, Blur and Sharpen (#551)
- [ ] The cross-document colour-mode rule, and Pattern Stamp Impressionist
      (#552)
- [x] Quick Mask (#172)

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
- [x] Select ▸ Transform Selection (#526)
- [x] Apply Image (#609)
- [x] Calculations (#610)
- [x] Channel Options — Masked and Selected Areas polarity (#611)
- [x] The interactive Refine Radius and Erase brushes (#612)
- [x] Color Range Skin Tones, Localized Clusters, Out Of Gamut, previews, and
      save and load (#613)
- [x] Anti-aliased selection rasterisation for the Elliptical Marquee, the
      lasso tools, and the Magic Wand boundary ramp, and the 56-channel
      document cap (#614); control-server selection ops stay binary.
- [x] Combine operations, the layer-mask target, modifier-click combine on
      saved selections, and channel names (#615)
- [ ] Native 16/32-bit selection and alpha channels for Save and Load Selection (#1331)
- [x] Duplicate Channel, channel drags between documents, selection-outline
      drags between documents, and the alpha-channel eye view (#1332)
- [x] Quick Mask (#172)

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
- [ ] Duotone conversion and its Options dialog (#553)
- [x] Multichannel conversion (#554)
- [x] The Color Table, and Indexed transparency and Forced palette (#555)
- [x] Bitmap Halftone Screen and Custom Pattern (#556)
- [x] The HDR Conversion tone methods (#557)
- [x] Merge To HDR Pro: exposure brackets, translation alignment, ghost base,
      full-screen preview, source inclusion, editable tone curves, presets,
      and 32-bit floating-point output (#558)
- [ ] Merge native-depth exposure sources and persist the HDR preview white
      point across PSD/PSB/TIFF reopen (#558)
- [ ] The Local Adaptation Toning Curve and Corner option (follow-up to #557)
- [x] The Auto Color Correction Options dialog (#559)
- [x] Levels eyedroppers and the Alt-drag clipping preview for RGB and Grayscale (#560)
- [ ] The Curves secondary UI: on-image, pencil and smooth, sliders and
      eyedroppers, display options, and the CMYK and Lab graphs (#561)
- [x] The Shadows/Highlights local-surround operator and its eight controls
      (#562)
- [x] The Threshold histogram under the slider (#563)
- [x] Vibrance Auto and presets (#564)
- [x] Smoother, Sharper and Automatic image-size kernels, Scale Styles, Auto
      resolution, picas and columns, and float resample (#565)
- [x] Picas and columns in Canvas Size, and extension-fill availability gating
      (#566)
- [x] The arbitrary rotation dialog, the interpolation preference, and the
      ruler-straighten prefill (#567)
- [x] Convert to Profile options, mismatch and missing-profile warnings, and
      the embed toggle (#568)
- [x] CMYK conversions use the document or working ICC profile (#809)
- [x] Color Settings: working CMYK space, RGB/CMYK policies, conversion options,
      and profile warnings (#569)
- [ ] Color Settings `.csf` save/load, Gray/Spot working spaces, and installed
      profile discovery (#569)
- [x] 32-bit Preview Options, the HDR colour picker, and the 32-bit Info
      readout (#570)
- [x] The Channel Mixer Monochrome UI, and the CMYK and Gray output channels
      (#571)
- [x] Color Balance numeric readouts, and Lab and CMYK mapping (#572)
- [ ] Hue/Saturation Colorize, range-band editing, eyedroppers, and the
      on-image tool (#573)
- [ ] Gradient Map dither, and the CS6 print-toning presets (#574)
- [ ] The Match Color on-canvas eyedropper and live Info feedback (#575)
- [x] Adjustment presets for Levels, Curves, Photo Filter, Selective Color and
      Vibrance (#576)
- [x] The Selective Color composite-channel-only gate and preset whitelist (#577)
- [x] Brightness/Contrast Auto, and the shared adjustment Auto buttons (#578)

## Filters

- [x] **110 filters** across the Artistic, Blur, Blur Gallery, Brush Strokes,
      Distort, Noise, Pixelate, Render, Sharpen, Sketch, Stylize, Texture and
      Other families. `crates/pictura-filters/src/`,
      `crates/pictura-filters/src/catalogue/`.
- [x] **Liquify, Oil Paint, the Blur Gallery, and the Filter Gallery.**
      `crates/pictura-app/cpp/filter_gallery_dialog.cpp`,
      `crates/pictura-app/cpp/blur_gallery_dialog.cpp`.
- [x] Maximum/Minimum Preserve (Squareness/Roundness), a 500 px radius, and the
      above-100 warning (#364)
- [x] Reduce Noise, including the Advanced per-channel view (#593).
  `crates/pictura-app/cpp/reduce_noise_dialog.cpp`,
  `crates/pictura-filters/src/noise/reduce.rs`
- [x] The Radial Blur Blur Center, honored by the kernel (#594)
- [ ] The Smart Sharpen Basic and Advanced split, and the 32-bpc disable (#595)
- [x] The eight Stylize filters missing from the Filter Gallery category (#596)
- [ ] Colored Pencil document-background paper wiring (#597)
- [x] Adaptive Wide Angle (#346): Fisheye, Perspective and Full Spherical,
      line and polygon constraints, EXIF focal-length prefill
- [ ] Adaptive Wide Angle Auto and lens profiles, Panorama, Smart Filter
      re-edit, and rotation handles (#891)
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
- [ ] Lens Correction Save/Load Settings, Set Lens Default, and Alt-drag fringe isolation (#933)
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
- [ ] Filter Gallery canvas and Shear below-input previews over an existing Smart
      Filter chain (#1003)
- [x] Fade disabled on a Smart Object (#1004): Smart Filters create no Fade record
- [x] Vanishing Point planes (#349): Create and Edit Plane, validity colours,
      Grid Size, related planes torn off at 90° with Angle, Render Grids
- [x] Vanishing Point Marquee, Stamp, Brush, Transform, Eyedropper and Paste
      (#349): floating selections in perspective with Feather, Opacity and
      Heal, cloning and painting that foreshorten with the planes
- [x] Vanishing Point planes saved in PSD, PSB, TIFF and JPEG (#918), in a
      project-owned image resource; reopening restores them
- [x] NTSC Colors (#345)
- [x] De-Interlace (#344)
- [x] Embed and Read Watermark (#350, #351): an in-tree approximate
      spread-spectrum carrier (not Digimarc-compatible), with the copyright
      title indicator on detection
- [x] Edit ▸ Fade, the post-filter opacity and blend mode (#491)
- [ ] Quick Compare (#1515)
- [ ] A Clarity and Dehaze adjustment layer (#1516)
- [ ] A Light adjustment layer (#1517)

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
- [x] The Gradient Editor reachable from the Gradient tool, plus opacity stops
      and a midpoint (#598)
- [ ] Define Pattern, the `.pat` library, and Scripted Patterns (#599)
- [x] Color-picker gamut and web alerts, and the picker preference (#600)
- [ ] Color Libraries (PANTONE) (#600)
- [ ] The HUD colour picker (#601)
- [x] The HDR colour picker at 32 bpc (#602)
- [x] The out-of-gamut triangle and CMYK to working space (#604)
- [ ] The 16/32-bit picker fields (#604)
- [ ] `.aco` and `.ase` swatch libraries, rename, and display modes (#605)
- [ ] Pattern Overlay Snap To Origin and the Bevel Texture controls (#606)
- [x] Bristle brushes (#607)
- [ ] `.abr` libraries, the Brush Presets panel, and ABR v10 reading (#608)
- [ ] Brush Texture and Dual Brush; the Control model, Shape Dynamics,
      Scattering, Color Dynamics, Transfer, Brush Pose, and per-set padlocks
      ship (#492)
- [ ] Sampled, erodible and airbrush tips, and Define Brush Preset
      (#493)

## Camera raw

- [x] **Pictura Raw**, reached through `Filter > Pictura Raw…`, with eight tabs:
      Basic, Tone Curve, Detail, HSL/Grayscale, Split Toning, Lens Corrections,
      Effects and Camera Calibration, all wired to a Rust renderer. It applies
      to an already-decoded image — it does not read a camera raw file.
      `crates/pictura-app/cpp/pictura_raw_dialog.cpp`,
      `crates/pictura-adjust/src/pictura_raw/`.
- [x] **DNG sensor reader** — `pictura_codec::read_dng` reads a DNG's CFA plane
      (TIFF or BigTIFF, uncompressed or lossless JPEG) normalized by its levels
      and active area, with its colour tags (#396).
      `crates/pictura-codec/src/dng/`.
- [x] **Raw demosaic** — `pictura_codec::demosaic` turns the CFA plane into
      16-bit linear camera RGB, bilinear for any RGB pattern or AHD for Bayer
      (approximate parity) (#397).
      `crates/pictura-codec/src/dng/demosaic/`.
- [x] **Camera colour** — `pictura_codec::as_shot_transform` builds the
      camera-to-linear-ProPhoto transform from the DNG's own colour matrices
      (no Adobe profiles) and reports the as-shot white balance in kelvin and
      tint; `to_pro_photo` applies it (approximate parity) (#398).
      `crates/pictura-codec/src/dng/camera_color.rs`.
- [x] **Canon CR2** — `pictura_codec::read_raw` names a raw container by its
      signature and reads a DNG or a CR2 (sliced lossless JPEG, active area
      from the maker note, black from the masked border) into the same CFA
      plane (#399).
      `crates/pictura-codec/src/dng/vendor.rs`, `crates/pictura-codec/src/dng/cr2.rs`.
- [x] **Nikon NEF** — `read_raw` reads a NEF: the CFA SubIFD, uncompressed
      (MSB-packed, LSB-packed or 16-bit containers) or Nikon Huffman-coded
      (lossless and lossy, with the maker note's linearization curve), black
      from the maker note, as-shot white balance from `WB_RBLevels`; 11 Nikon
      bodies take fitted matrices, and the D100, D7100 and Coolpix P7100 a
      same-sensor sibling's (approximate parity) (#1469).
      `crates/pictura-codec/src/dng/nef.rs`, `crates/pictura-codec/src/dng/nef_decode.rs`.
- [x] **Sony ARW** — `read_raw` names a TIFF with make `SONY` as ARW and reads
      uncompressed, ARW2 block-compressed (through the tone curve) and
      lossless-JPEG-tiled sensor planes, with the levels and as-shot white
      balance from the raw IFD or the deciphered SR2 block; ARW 1.0 and the
      YCbCr small/medium raws are refused (#1470).
      `crates/pictura-codec/src/dng/arw.rs`, `crates/pictura-codec/src/dng/arw_decode.rs`.
- [x] **Olympus ORF, uncompressed** — `read_raw` names the `IIRO`, `IIRS` and
      `MMOR` headers as ORF and reads 16-bit and packed 12-bit sensor planes,
      with levels, crop and white balance from the maker note and colour from
      the camera's embedded matrix (approximate parity) (#1471).
      Olympus-compressed ORFs are below (#1535).
      `crates/pictura-codec/src/dng/orf.rs`, `crates/pictura-codec/src/dng/orf_meta.rs`.
- [x] **Panasonic RW2, plain packings, decode only** — `read_raw` names the `IIU\0` header
      as RW2 and reads 16-bit words and the two 16-byte packings (RawFormat 2
      and 7), cropped to the sensor borders, with levels and as-shot white
      balance from the Panasonic tags; other codings are refused by name, and
      File ▸ Open refuses these bodies until they have colour (#1472).
      `crates/pictura-codec/src/dng/rw2.rs`.
- [x] **Panasonic RW2, compressed codings, decode only** — RawFormat 3 and 4
      (predicted packets, zero photosites filled), 5, 6 and 8 (Huffman-coded
      strips) are ported openly from darktable's rawspeed with attribution,
      bit-exact against LibRaw on samples of each (#1541).
      `crates/pictura-codec/src/dng/rw2_decode.rs`.
- [x] **Per-camera colour matrices** — vendor raw files without a
      `ColorMatrix` take A and D65 matrices fitted from openly licensed camera
      spectral sensitivities (14 Canon CR2 and 11 Nikon NEF bodies, and 11
      Sony ARW bodies with 3 same-sensor aliases, #1470; other cameras are
      refused),
      and a CR2 reports its as-shot white balance from the maker note
      (approximate parity) (#1465). `scripts/gen-camera-matrices.py`,
      `crates/pictura-codec/src/dng/camera_matrices.rs`.
- [x] **Open camera raw files** — File ▸ Open recognises a DNG, CR2, NEF, ARW, ORF, RW2 or RAF by its
      signature, decodes it, shows Pictura Raw first (absolute white balance,
      sidecar or embedded `crs:` settings), and opens a 16-bit RGB document
      (approximate parity) (#400).
      `crates/pictura-codec/src/dng/develop.rs`,
      `crates/pictura-app/src/cxxqt_object/open_jobs/raw_open.rs`.
- [ ] Open a raw file as a smart object (Shift+Open Image) (#1466)
- [ ] CR2 CFA phase for the EOS 600D, so it takes its colour matrices (#1476)
- [ ] Colour for the Sony bodies the matrix table does not seed yet (#1525)
- [x] **Fujifilm RAF and X-Trans demosaic** — `read_raw` names the
      `FUJIFILMCCD-RAW ` header as RAF and reads uncompressed 16-bit, 12-bit
      and 14-bit packed sensor planes of X-Trans and Bayer bodies, with CFA,
      levels, crop and white balance from the RAF directories; `Ahd` takes a
      new edge-directed X-Trans demosaic of Pictura's own design (approximate
      parity); 10 bodies open with camera-JPEG-fitted colour; the Open dialog
      lists RAF (#1473). `crates/pictura-codec/src/dng/raf.rs`,
      `crates/pictura-codec/src/dng/demosaic/xtrans.rs`.
- [x] **Fujifilm-compressed RAF sensor data** — lossless and lossy
      compressed RAFs of X-Trans and Bayer bodies at 12, 14 and 16 bits
      decode sample for sample as LibRaw does, ported from LibRaw (#1620).
      `crates/pictura-codec/src/dng/raf_compressed.rs`.
- [x] **Fujifilm SuperCCD and EXR RAF sensors** — word sensor sections,
      second directories (primary frame), the EXR layout and SuperCCD's 45°
      lattice, repacked and turned upright as LibRaw does (ported from
      LibRaw); 14 more bodies open with camera-JPEG-fitted colour; Save Image
      writes SuperCCD as linear (#1621). `crates/pictura-codec/src/dng/raf_superccd.rs`,
      `crates/pictura-codec/src/dng/demosaic/superccd.rs`.
- [x] **SuperCCD SR highlight range** — the S3 Pro, S5 Pro and F700 merge
      their low-sensitivity R frame into extended highlight range, the S/R
      ratio fitted per image and colour, the headroom carried by
      `BaselineExposure` (approximate: the reference merges are closed); the
      raw Exposure now applies in linear light before the develop's clip, so
      negative Exposure recovers highlights in every raw (#1648).
      `crates/pictura-codec/src/dng/raf_sr.rs`.
- [ ] Colour for the remaining Fujifilm RAF bodies (#1622)
- [ ] Deflate, PackBits and lossy-JPEG DNGs and floating-point samples (#396)
- [x] **Olympus-compressed ORF sensor data** — the coding of most bodies
      since 2007, 12-bit and the 14-bit high-resolution shots, decodes sample
      for sample as LibRaw does, ported from LibRaw (#1535).
      `crates/pictura-codec/src/dng/orf_decode.rs`.
- [x] **Panasonic RW2 colour** — 10 Panasonic bodies open with colour: the
      DC-GX9 from spectral data, nine others from matrices fitted against each
      body's own camera JPEG (approximate, the maker's rendering); the Open
      dialog lists RW2, RWL and RAW; RawFormat 2 whites at 3967 (#1542).
      `scripts/fit-camera-jpeg-matrices.py`,
      `crates/pictura-codec/src/dng/camera_matrices.rs`.
- [ ] Colour for the remaining Panasonic RW2 bodies (#1558)
- [x] **Canon CR3** — CR3 files (every Canon body since 2018) open: lossless
      and C-RAW CRX frames, versions 1 and 2, the main image of the largest
      raw track (bursts and Dual Pixel images keep their first frame), ported
      openly from LibRaw 0.22.1 and bit-exact with it on 52 sample files; the
      EOS R, RP, R5, R5 Mark II, R6, R6 Mark II, R10 and 250D (and the R50,
      R8, Rebel SL3, Kiss X10) open with colour; the Open dialog lists CR3
      (#1474). `crates/pictura-codec/src/dng/cr3.rs`, `crx*.rs`.
- [x] **Canon CR3 colour** — 15 more CR3 bodies open with colour: the 850D,
      90D, M200, M50, M50 Mark II, R100, R50 V, R6 Mark III, R7, 1D X Mark
      III, PowerShot G5 X Mark II and SX70 HS from matrices fitted against
      each body's own camera JPEG (approximate, Canon's rendering), the M6
      Mark II and PowerShot G7 X Mark III by same-sensor proxy, and Canon's
      regional names (Kiss M, Kiss M2, Rebel T8i, Kiss X10i) (#1574).
      `scripts/fit-camera-jpeg-matrices.py`,
      `crates/pictura-codec/src/dng/camera_matrices.rs`.
- [x] **Canon CR3 colour, more bodies** — the EOS R1, R3 and PowerShot V1
      open with colour, fitted against their own camera JPEGs from
      owner-supplied samples, and the R6 Mark III regains its D55 + Tungsten
      pair; the RP and R50 spectral rows stay, checked against Canon's own
      white-balance presets (#1579). `scripts/fit-camera-jpeg-matrices.py`,
      `crates/pictura-codec/src/dng/camera_jpeg_matrices.rs`.
- [x] **Canon colour, remaining bodies** — the EOS R8 Mark II opens with
      colour, and the EOS M6 Mark II and PowerShot G7 X Mark III trade their
      same-sensor proxies for their own fits, all against their own camera
      JPEGs from owner-supplied samples; the fit script reads CR2, so the
      PowerShot G1 X Mark III (a CR2 body) opens with a D55 matrix, and CR2
      colour data stored as bytes now gives the as-shot white balance
      (#1583). `scripts/fit-camera-jpeg-matrices.py`,
      `crates/pictura-codec/src/dng/camera_jpeg_matrices.rs`, `cr2.rs`.
- [ ] Every Nikon camera opens with colour: verify each layout, colour sources for ~70 bodies, recent models first (#1498)
- [x] **DNG output** — Save Image… in the raw open's Pictura Raw dialog writes
      a mosaic DNG (lossless-JPEG tiles) or, with Convert to Linear Image, a
      linear DNG, with the colour tags, crop, orientation, the dialog's
      settings as `crs:` XMP, a thumbnail and optional JPEG preview developed
      with them, the original raw file on request, and an ACR compatibility
      level (approximate version mapping); linear DNGs open again (#402).
      `crates/pictura-codec/src/dng/write/`,
      `crates/pictura-app/src/cxxqt_object/open_jobs/raw_open/save_dng.rs`.
- [x] **Camera Raw settings stores** — OK and Done in the raw open's Pictura
      Raw dialog save every tab's settings as `crs:` XMP in a per-user Camera
      Raw database keyed by the file's content hash or in a `<name>.xmp`
      sidecar (Preferences ▸ File Handling), falling back to the database on a
      read-only folder; unknown keys are kept and the raw file is never
      written (#401, #632).
      `crates/pictura-codec/src/pictura_raw/xmp.rs`,
      `crates/pictura-app/src/cxxqt_object/open_jobs/raw_open/store.rs`.
- [ ] Export Settings To XMP, settings inside existing DNGs and Update DNG
      Previews (#1484)
- [ ] JPEG, TIFF and PSD from Save Options, file-naming templates (#633)
- [ ] ACR Workflow Options, ACR preferences, batch raw processing, and Purge
      Camera Raw (#633)
- [ ] A Camera Raw settings and preset menu (#633)

## Panels

- [x] **Nineteen populated panels** — Layers, History, Channels, Paths, Color,
      Swatches, Info, Histogram, Navigator, Notes, Properties, Adjustments,
      Brush, Clone Source, Character, Paragraph, Glyphs, Paragraph Styles and
      Actions. `crates/pictura-app/cpp/frame_build.cpp`.
- [x] **Dockable panels**, a dark theme, a remembered layout, and the
      Essentials, Painting, Photography and Typography workspaces.
      `crates/pictura-app/cpp/panel_host_workspaces.cpp`.
- [x] **The toolbox, the options bar, and the keyboard-shortcuts editor.**
      `crates/pictura-app/cpp/toolbox.cpp`.
- [x] The Adjustments panel icon grid (#509)
- [ ] Refine the Adjustments panel icon artwork (#819)
- [ ] Fill-layer presets in the Adjustments panel (#1521)
- [ ] The Measurement Log panel, Record Measurements, and its export workflow (#510)
- [x] **The Tool Presets panel and `.tpl` libraries (#511)** — captures every tool and its options-bar snapshot, applies through the shared picker, filters by Current Tool Only, sorts by tool, supports the three display modes, and loads/replaces/saves `.tpl` and native `.kpset` libraries. `crates/pictura-app/cpp/panels/tool_presets_panel.cpp`.
- [ ] The Styles panel and `.asl` presets (#512)
- [ ] Channel panel spot channels, Split and Merge, and Panel Options (#513)
- [x] History Options, Clear History and purge, and New Document from state
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
- [x] **A local control server** on a Unix socket, with JSON methods for documents,
      layers, selections, filters with parameters, adjustments, commands, history,
      PNG export, and pointer and key input.
      `crates/pictura-app/cpp/control_server.cpp`.
- [x] **An MCP server:** `pictura --mcp` speaks MCP over stdio to a running window
      or a private headless instance. `crates/pictura-mcp/`.
- [x] **Agent Control in Preferences:** enable agents, set their permissions, and
      copy a client configuration. `crates/pictura-app/cpp/preferences_agent_page.cpp`.
- [x] **One-click MCP client registration** from Preferences.
      `crates/pictura-app/cpp/client_registration.cpp`.
- [x] **Agent Control in Flatpak and Snap builds**, with no extra sandbox
      permission. `crates/pictura-app/cpp/agent_control.cpp`.
- [ ] MCP protocol revision 2026-07-28
      ([#1080](https://github.com/KookaStudio/KookaPictura/issues/1080)).
- [x] **Live agent sessions with permissions:** a control server in the running
      app, off by default, with permission tiers, allowed folders, and approval
      prompts. `crates/pictura-app/cpp/agent_control.cpp`.
- [x] **A Rust command catalogue and the shared command-recording bus** — the
      catalogue drives the menu tree and the checked-in `command_ids.h`; the
      C++ command bus gains executors, context handlers, a recording sink, and
      `executeRecorded` (#273, #616).
      `crates/pictura-script/src/command_catalogue.rs`,
      `crates/pictura-app/cpp/commands.cpp`.
- [ ] Create Droplet, the `.opd` format, and the `.desktop` launcher (#617)
- [x] **File ▸ Automate ▸ Batch** — runs an action over a folder (optionally
      with subfolders) or the open documents, with the Open and Save As
      overrides, None / Save and Close / Folder destinations, CS6 File Naming,
      and Stop or Log error handling; File ▸ Open is now a recordable step and
      Save As steps carry their format options (#618).
      `crates/pictura-script/src/batch/`, `crates/pictura-app/cpp/batch_runner.cpp`.
- [ ] The Batch Import and Bridge sources (#1406)
- [ ] The remaining Automate commands: Image Processor, Conditional Mode
      Change, PDF Presentation, Contact Sheet II, Picture Package, Photomerge
      and Web Photo Gallery (#619)
- [ ] A scripting host, DOM, ActionManager and notifiers on QuickJS (#620)
- [ ] A script console, sandbox limits, a security prompt, and the progress
      bridge (#621)
- [x] Variables and data-driven graphics (#622)
- [ ] A Rust C-ABI plug-in loader and the Plug-in manager dialog (#623)
- [ ] The Mini-Bridge replacement Files panel (#624)

## Application frame, menus and preferences

- [x] **The application frame** and its ten top-level menus, with 413 live
      commands. `crates/pictura-app/cpp/commands.cpp`.
- [x] **Preferences** — General, Interface, Performance, Units and Guides.
      `crates/pictura-app/cpp/preferences_dialog.cpp`.
- [x] **Settings backup and restore** — Preferences ▸ General exports every user
      store to one `pictura.json` and stages an import that applies before the
      next launch (#1155). `crates/pictura-app/cpp/settings_backup.cpp`.
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
- [x] **Help ▸ Developer ▸ Pictura Metrics (#1589)** — a light-yellow floating
      tool window (off at startup, never saved; drag it off the app, resize it,
      close it to stop collecting) refreshed every 2 s, laid out as tables in two columns (titled Pictura Metrics):
      Pictura Resource Usage (RAM, swap, CPU, threads, GPU time, CPU by thread
      group, RAM, CPU and per-group history charts, GPU busy and memory from DRM
      fdinfo), Pictura Memory Budget (governor, scratch disk, tiles), Pictura
      Activity (open files, disk I/O, page faults, context switches, jobs
      running, edit and paint p50/p95), Platform (OS, desktop, memory, swap, a
      per-CPU heat matrix, system GPU) and the build placeholder. Linux only; not a CS6 feature,
      added by owner decision.
      `crates/pictura-app/cpp/debug_toolset_panel.cpp`.
- [ ] Status bar readouts and the status-bar options popup (#497)
- [ ] The active layer name in the title bar (#498)
- [x] `Ctrl+Tab` and `Shift+Ctrl+Tab` document cycling (#499)
- [x] Window ▸ Arrange: Float, Tile, Cascade, Match and New Window (#500)
- [x] Document tab tear-out into floating windows, docking a floating window
      back by its header (drop, Ctrl+drag, or double-click), top-level
      floating windows on Wayland, and the tab and header context menus (#317)
- [x] The full document label in the tab tooltip (#917)
- [x] The open-document list at the bottom of the Window menu (#923)
- [ ] Menu customization: Edit ▸ Menus, Show All Menu Items, and the
      enablement matrix (#501)
- [x] The Type menu: work path, convert to shape, find and replace, and the
      remaining option rows (#502)
- [ ] Check Spelling with a bundled dictionary (#1443)
- [ ] Browse and install Google Fonts from the Type menu (#1442)
- [x] View ▸ Show/Extras, Show Extra Options, Grid, Pixel Grid, Layer Edges,
      Target Path, Lock/Clear Slices, 200% and Print Size (#503)
- [ ] The remaining View menu items: Gamut Warning and Pixel Aspect Ratio
      (#503); Snap To (#305)
- [x] File ▸ Scripts ▸ Load Files into Stack: the Load Layers dialog, one
      native-size layer per file on a new canvas, Auto-Align, and the embedded
      smart-object option (#1277)
- [x] File ▸ Browse… (the Bridge replacement) and File ▸ Scripts ▸ Statistics…;
      the remaining File-menu leaves are tracked by #617/#619/#620/#630 (#504)
- [ ] The Cursors, Transparency and Gamut, Plug-ins, Type and 3D preferences
      panes (#505)
- [ ] The sparse existing panes: General, Interface, File Handling, Performance,
      Units and Guides (#506)
- [x] **Memory governor** — one process-wide ceiling (Memory Usage % of RAM,
      default 60 %, capped by the cgroup's `memory.max` and `memory.high`),
      memory-pressure tracking, and fallible reservations. New Document, Add
      Layer, brushing, history, snapshots, Save, Open, the fills, Move, Free
      Transform and Image Size are refused with "not enough memory" instead of
      being OOM-killed. A Memory Usage control on the Performance page (#1211).
      `crates/pictura-core/src/memory/`.
- [x] **Content-sized pixel layers** — a new pixel layer stores no pixels; the
      paint, fill and filter engines grow a layer to their damage, Move and Free
      Transform trim it to its content, and PSD keeps the stored rect (#1208).
      `crates/pictura-core/src/layer_bounds.rs`.
- [x] **Tiled pixel planes** — document planes are 256² tiles per channel that
      are empty, uniform or shared copy-on-write, so a white Background costs
      nothing, an edit forks only the tiles it touches, history shares untouched
      tiles, and PSD read and write stream through them (#1212, #1213).
      `crates/pictura-core/src/tiled_plane.rs`.
- [x] **Tiled history** — undo states and named snapshots hold references to
      tiles, so a snapshot and its restore copy no pixels, and history's budget
      counts each tile it keeps beyond the live document once (#1215).
      `crates/pictura-app/src/history/`, `crates/pictura-core/src/memory/census.rs`.
- [x] The Performance preferences pane: Memory Usage slider and ceiling, History
      States (applied at once), Cache Levels capping the view pyramid with the
      History & Cache presets, the fixed tile size, the Scratch Disks list and
      the graphics processor, plus the Scratch Sizes and Efficiency status-bar
      readouts (#636). `crates/pictura-app/cpp/preferences_performance.cpp`.
- [ ] AT-SPI2 and `QAccessible`, high contrast, colour-blind and reduced-motion
      support (#507)
- [ ] Shortcut sets: save and load, workspace-assigned, Summarize HTML, and
      Legacy Channel Shortcuts (#508)
- [ ] Quick fill with Alt+Backspace and Ctrl+Backspace (#524)

## Engine and performance

- [x] Authoritative layer content for pixels, adjustments, fills, text, shapes,
  smart objects, and groups, with PSD preservation and shared render routing (#1344).

- [x] **A Rust engine under a Qt 6 shell** — ten engine crates, with no Qt type
      in any of them. `crates/`.
- [x] **A wgpu/Vulkan compute path** for compositing, the heavy filters, and the
      live paint stroke, gated behind a one-time comparison against the CPU
      reference. `crates/pictura-render/src/gpu/`.
- [x] **Parallel PSD decode** — layer channels, merged RLE rows, and
      ZIP-prediction rows decode on rayon, byte-identical to the serial decode
      with the same first error. `crates/pictura-codec/src/read/`. (#1019)
- [x] **Lazy, file-backed PSD open** — 8-bit raw and RLE layer channels are
      indexed at open and decoded a 256-row band at a time on first read; an
      unedited tile drops back to the file instead of scratch, and the file's
      merged image is the first picture until the first edit (#1218, #1318).
      `crates/pictura-codec/src/lazy.rs`.
- [x] Fit-to-screen first paint without rebuilding every level from level 0
      (#1321)
- [x] **Streamed saves** — the PSD/PSB and row-based raster writers stream a
      band of rows at a time into the temp file, and the save gate reserves
      what a save holds, so a large tiled document saves under a memory limit
      (#1320). `crates/pictura-codec/src/write_stream.rs`.
- [x] A typed `prefs.toml` store, the config and state split, migration,
      quarantine, and the reset gesture (#641)
- [x] Bundled German and Finnish, community translation import, and locale
      number formatting (#642)
- [ ] Right-to-left UI layout and languages (#1488)
- [ ] Investigate switching the interface language without restarting (#1501)
- [x] Structured Qt and Rust diagnostics with a rotating file sink, plus the
      opt-in History Log in text files and PSD, PSB, JPEG, and TIFF XMP (#643)
- [ ] Opt-in telemetry and `KOOKA_PICTURA_NO_TELEMETRY` (#644)
- [x] A stable error-code registry, and the FFI `OpStatus` and `catch_unwind`
      boundaries (#645)
- [x] Safe paths and decoder decompression-bomb guards (#646)
- [ ] Flatpak portal confinement (#647)
- [x] An aarch64 (ARM64) AppImage, built natively and self-updating (#1419)
- [x] aarch64 `.deb` and `.rpm` packages (#1433)
- [ ] DSSIM, PSNR and ΔE2000, property tests, fuzzing, and Criterion benches
      (#648)
- [x] `--version` and About diagnostics for the build, Qt runtime, renderer,
      GPU adapter, and driver (#649). The stable update checker shipped in
      #1146.
- [x] Edition gating for Standard and Extended (#650): `PICTURA_EDITION`
      selects the surface (default Extended); Standard hides the Count tool,
      the 3D commands, the Measurement Log and the Count overlay, while the
      Timeline stays in both editions.
- [x] Opt-in performance metrics: phase timings, edit records and GPU counters
      (#651)
- [x] Parallel CPU kernels for adjustments, effect mattes, clipping and the
      display colour transform, byte-identical to the serial path (#1011)
- [x] Parallel preparation of layer-effect passes before banded CPU compositing,
      with ordered replay and exact output (#1179)
- [x] Cold styled composites within 1000 ms on the acceptance workstation, with
      exact SIMD Gaussian fields and cache-local u8 convolution (#1178, #1179)
- [x] Reuse warm whole-layer effect passes for short sub-region composites
      (#1180)
- [x] Visibility toggles patch the Layers row in place instead of rebuilding
      the panel snapshot and thumbnails (#1197)
- [x] Frame-budget instrumentation (#941)
- [x] A tile cache with hit-rate accounting (#942): the viewport presents a
      256² display tile pyramid, with no stored full-resolution composite for
      layered documents (#1216); export, selections, previews, conversions and
      dialog backdrops read bands, tiles or reduced levels instead of a whole
      composite (#1274); the whole-level rebuild remains only as an independent
      test oracle, excluded from ordinary app builds (#1349)
- [x] Scratch paging and Efficiency accounting (#943): cold tiles pack (LZ4)
      and spill to checksummed scratch volumes under the memory ceiling, with
      Scratch Sizes and Efficiency in `status` (#1217).
      `crates/pictura-core/src/scratch.rs`, `crates/pictura-core/src/memory/eviction.rs`.
- [ ] A tile job scheduler with cancellation (#944)
- [ ] Per-tile filter cancellation (#945)
- [ ] Global bit-depth and colour-mode availability gating (#490)

## Development workflow

- [x] **Opt-in shared agent CPU quota** across worktrees and coding harnesses,
  with a stable per-user verification lock (#1490).
- [x] **Managed agent resource guards** with memory and swap limits, a shared
  verification queue, and canonical OpenCode policy enforcement (#1529).

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
- Video, the timeline, and video layers — exact timeline parity; Render Video
  is deferred Extended-only until a codec backend exists
- DICOM (Extended) — a separate medical-imaging domain; the Count tool and the
  measurement scale ship
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
