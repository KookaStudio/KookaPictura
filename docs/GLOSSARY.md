# Glossary

Vocabulary used across Kooka Pictura: Photoshop CS6 concepts, the project's own
architecture, PSD/PSB format fields, and the identifiers in `crates/`. Entries
are alphabetical. Each row cites the file and line it came from, so you can
check a definition against its source.

The `Type` column is one of:

| Type | Meaning |
|---|---|
| 32-bpc (HDR) | cs6 | 32 bits per channel, stored as floating point for high dynamic range images. Values relate to scene light and may exceed the [0,1] range. | docs/04-image-ops/bit-depth-and-conversion.md:20 |
| 3D panel | cs6 | Extended-only dock listing a 3D scene's components (Scene, Meshes, Materials, Lights). Their settings are edited in the Properties panel. | docs/02-ui-ux/panels/3d-panel.md:13 |
| 50% line | cs6 | The selection boundary drawn halfway between less-than-50% and more-than-50% selected pixels. It is the only visible evidence of a soft selection. | docs/08-selection/selection-model.md:21 |
| .8bf | format | Adobe filter plug-in binary extension. Loading Adobe binaries is a Linux non-goal. | docs/01-architecture/plugin-and-scripting-abi.md:19 |
| 8BIM | format | The four-byte Adobe signature prefixing a PSD image resource and every additional-layer-info tagged block. | crates/pictura-codec/src/lib.rs:460 |
| 8BPS | format | The four-byte PSD/PSB file signature. `SIGNATURE` holds `0x38425053`, the big-endian "8BPS" bytes. | crates/pictura-codec/src/lib.rs:32 |
| ABI (plug-in) | project | The stable C interface for native plug-ins, plus a Rust trait path (`OpPluginApi`) for in-tree plug-ins. | docs/01-architecture/plugin-and-scripting-abi.md:114 |
| .abr | format | Brush library format. CS6 writes ABR version 10 subversion 1 with `8BIM` tagged sections. | docs/07-color-painting/brush-presets.md:6 |
| ACR | cs6 | Adobe Camera Raw, the RAW-processing engine, version 7 in CS6. Also exposed as the Camera Raw Filter in the Filter menu. | docs/00-overview/cs6-editions-and-constraints.md:28 |
| ACR 7 (Adobe Camera Raw 7) | cs6 | The raw engine bundled with CS6. Its processing pipeline is closed, so exact parity is a non-goal. | docs/00-overview/cs6-editions-and-constraints.md:28 |
| Action | cs6 | A recorded series of tasks, such as menu commands and panel options, played back on one file or a batch. | docs/02-ui-ux/panels/actions-panel.md:13 ; docs/09-automation/actions.md:11 |
| Action descriptor | cs6 | The event-ID plus key/value record (`ActionDescriptor`) that each recorded command executes, shared with scripting and plug-ins. | docs/09-automation/actions.md:52 |
| Action set | cs6 | A named group of actions, saved as a `.atn` library. | docs/02-ui-ux/panels/actions-panel.md:17 |
| Actions panel | cs6 | Dock that records, plays, edits, and deletes actions, and can render them as buttons. | docs/02-ui-ux/panels/actions-panel.md:13 |
| active_backend | code | Invokable returning the current render backend label, one of "GPU", "CPU", or "CPU (no GPU)". | crates/pictura-app/src/cxxqt_object.rs:722 |
| adapter namespace | project | One of `document_api`, `layers_api`, `selection_api`, `paint_api`, `transform_api`, `history_api`, `type_api` — the typed C++ façade over the `PictureView` bridge for a landed migration unit, and the only surface its consumers include. | docs/dev/refactor-view-seam.md:21 |
| Adaptive Wide Angle | cs6 | CS6 top-level GPU filter that corrects wide-angle and fish-eye distortion from lens-profile data and user-drawn straight-line constraints. | docs/06-filters/adaptive-wide-angle.md:13 |
| Add Noise | cs6 | Noise filter that adds random pixels to simulate film grain, with Uniform or Gaussian distribution and an optional monochromatic mode. | docs/06-filters/noise-filters.md:15 |
| add_group | code | Inserts an empty group directly above `above` and returns its index. | crates/pictura-render/src/document_ops/layer_ops.rs:126 |
| add_group_in | code | Inserts an empty group for `name` (generated when empty) with the tree-aware `insert_node` rule, and returns the new path. | crates/pictura-render/src/document_ops/layer_ops.rs:718 |
| add_layer | code | Inserts a transparent raster layer directly above `above` and returns its index, or -1 for a zero-dimension document. | crates/pictura-render/src/document_ops/layer_ops.rs:115 |
| add_layer_in | code | Inserts a transparent raster layer for `name` (generated when empty) with the tree-aware `insert_node` rule, and returns the new path. | crates/pictura-render/src/document_ops/layer_ops.rs:703 |
| AdjustError | code | Error returned by adjustment operations for unsupported buffers or invalid parameters, instead of panicking. | crates/pictura-adjust/src/lib.rs:15 |
| Adjustment | code | One destructive adjustment of fifteen kinds, applied in place to a planar 8-bit buffer. | crates/pictura-adjust/src/lib.rs:115 |
| Adjustment layer | cs6 | A non-destructive layer storing adjustment parameters (`AdjustmentParams`), not pixels, that applies to layers below; it has a mask by default. | docs/01-architecture/document-model.md:21 ; docs/04-image-ops/adjustments-overview.md:21 ; docs/05-layers/adjustment-layers.md:16 |
| adjustment-layers | capability | Adjustment layers stored opaquely and preserved through PSD read and write, applied to the backdrop below with mask, opacity, and blend gating. Unknown adjustments are preserved but not applied. | openspec/specs/imaging/adjustment-layers/spec.md:6 |
| adjustment-ui | capability | Commands and dock UI to introspect the layer stack and add adjustment, toggle visibility, and remove layer; backed by render-crate adjustment encoders and a headless self-test. | openspec/specs/imaging/adjustment-ui/spec.md:6 |
| ADJUSTMENT_KEYS | code | The 17 additional-layer-info keys treated as adjustments, including the spellings Photoshop writes (`nvrt` not `invr`, legacy `hue `) and their alternates. | crates/pictura-codec/src/lib.rs:57 |
| AdjustmentData | code | Raw additional-layer-info block of an adjustment layer: the 4-byte PSD key plus its verbatim payload. `pictura-core` does not interpret the payload; `pictura-render` decodes the subset it understands. | crates/pictura-core/src/lib.rs:253 |
| Adjustments panel | cs6 | Dock whose icons create adjustment layers. In CS6 it shows icons only, and the controls live in the Properties panel. | docs/02-ui-ux/panels/adjustments-panel.md:13 ; docs/04-image-ops/adjustments-overview.md:131 |
| Adobe ACE (Adobe Color Engine) | cs6 | Adobe's closed color engine, the parity reference for color conversions. | docs/01-architecture/color-management.md:106 |
| Adobe Bridge | cs6 | Separate Creative Suite application for browsing and batch-processing assets. Photoshop integrates with it rather than embedding it, and it has no Linux build. | docs/10-workflow-io/bridge-and-interop.md:18 |
| Adobe Camera Raw (ACR 7) | cs6 | Bundled plug-in that interprets camera raw sensor data into a color image; CS6 ships ACR 7.0 with Process Version 2012. | docs/10-workflow-io/camera-raw-workflow.md:19 |
| Adobe Color Picker | cs6 | The default modal color dialog. It sets foreground, background, and text colors using four models and shows HSB, RGB, Lab, CMYK, and hex at once. | docs/07-color-painting/color-picker.md:15 |
| Adobe RGB (1998) | code/format | A wide-gamut RGB working space recommended for print, one of the standard profiles in Color Settings. | crates/pictura-color/src/lib.rs:55 ; docs/10-workflow-io/color-settings.md:29 |
| Align Layers | cs6 | Command that translates selected layers so a chosen edge or center lines up, with each other or with the active selection. | docs/05-layers/align-and-distribute.md:15 |
| Aligned | cs6 | Clone Stamp and Healing Brush option that keeps a fixed source-to-destination offset across separate strokes. When off, each stroke restarts from the original sample point. | docs/03-tools/clone-stamp-and-pattern-stamp.md:22 |
| Alpha channel | cs6 | A grayscale channel that stores a selection as a mask; a document can hold up to 56 channels including alpha and spot. | docs/02-ui-ux/panels/channels-panel.md:16 ; docs/08-selection/save-and-load-selections.md:13 |
| Anchor | code | Placement anchor for canvas resize, nine positions such as TopLeft, Center, and BottomRight. It is re-exported from `pictura-ops`. | crates/pictura-render/src/lib.rs:56 ; crates/pictura-ops/src/canvas.rs:9 |
| Anti-aliasing (selection) | cs6 | Softening of jagged selection edges by fractional coverage at boundary pixels. It must be set before the selection is made and cannot be added later. | docs/08-selection/selection-model.md:42 |
| Application bar | cs6 | The CS5 top bar removed in CS6. Its workspace switcher moved to the Options bar and its screen-mode control to the toolbar. | docs/02-ui-ux/application-frame.md:28 |
| Application frame | cs6 | The single integrated window holding the menu bar, options bar, document windows, Tools panel, and docks. | docs/02-ui-ux/application-frame.md:19 |
| application-shell | capability | The Qt6 plus Rust shell: the cxx-qt bridge exposing a Rust QObject, the CMake and qmake6 build, a headless window, theme brightness, screen modes, status readouts, CS6 chrome, checkerboard canvas clipping, and the toolbox catalogue. | openspec/specs/ui/application-shell/spec.md:6 |
| apply (adjust) | code | Applies an adjustment in place to a planar 8-bit buffer, alpha untouched. | crates/pictura-adjust/src/lib.rs:138 |
| apply (filters) | code | Applies a filter in place to a planar 8-bit buffer with 3 or 4 channels, alpha untouched. | crates/pictura-filters/src/lib.rs:616 |
| Apply Image | cs6 | Command that blends one image's layer and channel with a layer and channel of the active image, optionally through a mask. | docs/08-selection/channel-based-masking.md:30 |
| apply_filter | code | Applies a destructive filter to a pixel layer's color channels (0,1,2), gated by an optional document-coordinate coverage mask (`None` is full frame). The transparency channel (-1) is never modified. | crates/pictura-render/src/filter.rs:18 ; docs/dev/STATE.md:47 |
| apply_filter_active | code | Applies a filter in place and returns the backend that produced the buffer. Uses the GPU when `gpu_enabled` and `filter_gpu_available()` and a kernel exists, otherwise the CPU oracle. | crates/pictura-render/src/gpu_filter.rs:88 ; docs/dev/STATE.md:47 |
| apply_visibility | code | Solo visibility primitive: sets exactly the listed paths visible and hides every other node. Not additive, ancestors are not implied, and duplicates count once. | crates/pictura-render/src/document_ops/layer_ops.rs:393 |
| Apron | project | The band of source pixels read outside a filter's target region so neighborhood kernels have data at selection edges. | docs/06-filters/filters-overview.md:132 |
| Arbitrary rotation | cs6 | Image Rotation command that rotates the document by a user angle from -359.99 to 359.99 degrees. It resamples pixels and grows the canvas to the rotated bounding box. | docs/04-image-ops/image-rotation-and-flip.md:21 |
| ARCH-001 | project | Spec ID for System Architecture, the foundational layer-boundary document. | docs/01-architecture/system-architecture.md:3 |
| ARCH-002 | project | Spec ID for Rust Core Design, the proposed workspace, document types, and error model. | docs/01-architecture/rust-core-design.md:3 |
| ARCH-003 | project | Spec ID for Qt6 UI Design, the Widgets-shell and canvas-rendering decision. | docs/01-architecture/qt6-ui-design.md:3 |
| ARCH-004 | project | Spec ID for Rust-Qt Interop, which chooses cxx-qt. | docs/01-architecture/rust-qt-interop.md:3 |
| ARCH-005 | project | Spec ID for Threading and Concurrency. | docs/01-architecture/threading-and-concurrency.md:3 |
| ARCH-006 | project | Spec ID for the GPU Rendering Pipeline. | docs/01-architecture/gpu-rendering-pipeline.md:3 |
| ARCH-007 | project | Spec ID for Color Management. | docs/01-architecture/color-management.md:3 |
| ARCH-008 | project | Spec ID for the Document Model (provisional). | docs/01-architecture/document-model.md:3 |
| ARCH-009 | project | Spec ID for Undo History (provisional). | docs/01-architecture/undo-history.md:3 |
| ARCH-010 | project | Spec ID for File Formats (provisional). | docs/01-architecture/file-formats.md:3 |
| ARCH-011 | project | Spec ID for the Plugin and Scripting ABI. | docs/01-architecture/plugin-and-scripting-abi.md:3 |
| ARCH-013 | project | Spec ID for Performance Targets. | docs/01-architecture/performance-targets.md:3 |
| ARCH-014 | project | Spec ID for Build and Packaging. | docs/01-architecture/build-and-packaging.md:3 |
| archive | process | Moving a completed change under openspec/changes/archive/<date>-<name>/ and merging its requirement deltas into openspec/specs/. | openspec/changes/archive/2026-09-16-m38-icon-cursor-library/proposal.md:1 |
| Archive (OpenSpec) | process | Merging a completed change's deltas into openspec/specs/ and moving the change under openspec/changes/archive/. | docs/dev/STATE.md:845 |
| Art History Brush | cs6 | A paint tool that applies stylized strokes using a selected history state or snapshot as the color source. Options include Style, Area, and Tolerance. | docs/01-architecture/undo-history.md:37 ; docs/03-tools/art-history-brush.md:16 |
| Artboard | cs6 | A container holding its own canvas region. It is documented in the PSD format but arrived in CC 2015, so it is a post-CS6 extension and not CS6 parity. | docs/05-layers/artboards.md:1 |
| Artistic filters | code/cs6 | A Filter Gallery family of 15 painterly effects. All are 8-bit only, and several use the foreground and background colors as paint and paper. | crates/pictura-filters/src/artistic/mod.rs:1 ; docs/06-filters/artistic-filters.md:13 |
| artistic-filters | capability | The 16 CS6 Artistic filters, from Colored Pencil to Watercolor, sharing one error, alpha-preservation, parameter-validation, seeding, colour, and texture-option contract. | openspec/specs/imaging/artistic-filters/spec.md:6 |
| Assign Profile | cs6 | Document operation that replaces the profile tag without changing pixel values, so appearance may change. Removing the profile is the same command with Don't Color Manage. | docs/01-architecture/color-management.md:26 ; docs/04-image-ops/color-profiles-and-assignment.md:39 |
| Auto Color Correction Options | cs6 | The shared dialog defining what the Levels/Curves Auto button and the Image menu Auto commands do. It selects an algorithm, clip percentages, and target shadow, midtone, and highlight colors. | docs/04-image-ops/adjustments/levels.md:50 |
| Auto Tone / Auto Contrast / Auto Color | cs6 | Three one-click automatic corrections. Auto Tone stretches each channel, Auto Contrast stretches the composite without a cast, and Auto Color also neutralizes midtones. | docs/04-image-ops/adjustments/auto-adjustments.md:19 |
| Auto-Align Layers | cs6 | Command that matches overlapping content across layers and transforms them by a chosen projection; Adobe's matching algorithm is unpublished. | docs/05-layers/align-and-distribute.md:58 |
| Auto-Blend Layers | cs6 | Command that combines layers by generating per-layer masks, optionally matching tone and color. It works only on RGB or Grayscale. | docs/05-layers/align-and-distribute.md:84 |
| Auto-Recovery | cs6 | CS6 feature that writes a separate recovery snapshot at a user interval (default ten minutes) and reopens it as a Recovered document after a crash. | docs/11-cross-cutting/crash-recovery-and-autosave.md:33 |
| AutoKind | code | Auto adjustment variant: Tone, Contrast, or Color. | crates/pictura-adjust/src/lib.rs:107 |
| AUTOMOC | code | CMake option that runs Qt's moc over C++ sources so `Q_OBJECT` classes get their meta-object code. Required for every Qt class in the shell. | CMakeLists.txt:17 |
| Backend | code | The enum Gpu or Cpu returned from a composite or filter call to report which implementation produced the pixels. | crates/pictura-render/src/gpu.rs:86 ; docs/dev/STATE.md:47 |
| Background Eraser | cs6 | A brush that erases pixels to transparency while preserving foreground edges. It samples the color at the brush hotspot and supports Limits, Tolerance, Protect Foreground Color, and Sampling. | docs/03-tools/eraser-tools.md:45 |
| Background layer | cs6 | A special bottommost layer created with a white or colored canvas. It cannot be reordered, blended, or given opacity until converted to a regular layer. | docs/05-layers/layers-overview.md:62 |
| Background Save | cs6 | CS6 feature where File > Save completes out of band so the UI stays responsive. Progress shows in the document tab and status bar. | docs/10-workflow-io/document-lifecycle.md:45 |
| begin_move_preview | code | Enters move-preview mode by caching the document composited with the topmost raster layer hidden, that layer's image, its document-space top-left, and its opacity. Returns false without a document or raster layer. | crates/pictura-app/src/cxxqt_object.rs:487 |
| begin_paint | code | Starts a paint stroke from a full brush configuration (colour, diameter, hardness, roundness, angle, opacity, flow, spacing, mode, aliased, auto-erase). Returns false without a document or raster layer. | crates/pictura-app/src/cxxqt_object.rs:564 |
| Behavioral parity | project/process | Parity standard for operations whose algorithm Adobe never published, verified against captured reference renderings and never against an assumed formula. | docs/00-overview/product-overview.md:23 ; docs/11-cross-cutting/testing-strategy.md:38 ; docs/SPEC_TEMPLATE.md:47 |
| Bicubic | cs6 | Default resampling kernel, slower and smoother than Bilinear, using a 4x4 cubic convolution. Photoshop's exact coefficients are not published. | docs/04-image-ops/image-size.md:61 |
| Bicubic Automatic | cs6 | New in CS6; auto-selects the resample method from the resize direction, reported as Sharper when downsampling and Smoother when upsampling. The mapping is inferred. | docs/04-image-ops/image-size.md:64 |
| Bicubic Smoother / Bicubic Sharper | cs6 | Bicubic variants for enlarging (Smoother) and reducing with detail preservation (Sharper). Their exact kernels are closed. | docs/04-image-ops/image-size.md:62 |
| Big-endian | format | The PSD/PSB on-disk byte order on all platforms. Readers and writers byte-swap explicitly. | docs/01-architecture/document-model.md:161 |
| Bilinear | cs6 | Medium-quality resampling kernel that averages the surrounding 2x2 pixel neighborhood. | docs/04-image-ops/image-size.md:60 |
| Bit depth | cs6 | Per-channel precision of a document: 8 bpc integer, 16 bpc integer, or 32 bpc floating-point HDR. | docs/01-architecture/rust-core-design.md:26 ; docs/07-color-painting/color-models.md:56 |
| Bit depth (bpc) | cs6 | The number of color bits per channel: 1 (Bitmap), 8, 16, or 32. It sets the scalar type of pixel data and gates which tools and adjustments are available. | docs/04-image-ops/bit-depth-and-conversion.md:11 |
| BitDepth | code | Bits per channel; the four Photoshop depths One, Eight, Sixteen, and ThirtyTwo. | crates/pictura-core/src/lib.rs:37 |
| Bitmap mode | cs6 | A 1-bit color mode with only black and white values. It cannot hold layers, filters, or adjustments. | docs/04-image-ops/image-modes.md:16 |
| Black & White adjustment | cs6 | Converts color to grayscale with six per-hue sliders, preserving control over each color's contribution. A Tint option applies a color tone. | docs/04-image-ops/adjustments/black-white.md:15 |
| Black Point Compensation | code/cs6 | Conversion option that preserves shadow detail by simulating the output device's full dynamic range. It is set in Convert To Profile, soft proofing, and printing. | crates/pictura-color/src/lib.rs:158 ; docs/04-image-ops/color-profiles-and-assignment.md:105 ; docs/10-workflow-io/color-settings.md:100 |
| Black point compensation (BPC) | cs6 | Maps the source black point to the destination black point and scales the tonal range, preserving shadow separation. | docs/01-architecture/color-management.md:147 |
| BlackWhiteParams | code | Black and White mix with per-colour contributions in percent matching the CS6 -200 to +300 sliders, plus an optional tint and tint colour. | crates/pictura-adjust/src/lib.rs:60 |
| Blend Clipped Layers As Group | cs6 | An advanced-blending option (default on) that applies the clipping base layer's blend mode to every layer in the clipping mask. | docs/05-layers/blend-modes.md:75 |
| Blend If | cs6 | Per-channel tonal sliders that limit where a layer blends, based on this layer's or the underlying layer's brightness. | docs/05-layers/blend-modes.md:118 |
| Blend Interior Effects As Group | cs6 | An advanced-blending option deciding whether the layer's blend mode also applies to interior effects such as Inner Glow, Satin, and the overlays. | docs/05-layers/blend-modes.md:60 |
| blend key | domain | The 4-byte PSD on-disk identifier for a layer blend mode; the codec maps all 27 keys one-to-one. | openspec/specs/compositing/blend-modes/spec.md:6 |
| Blend mode | cs6 | The rule that combines a layer's source color with the backdrop to produce the result color. CS6 has 27 layer modes plus group Pass Through. | docs/01-architecture/document-model.md:84 ; docs/05-layers/blend-modes.md:18 |
| Blend Text Colors Using Gamma | cs6 | Global CS6 text-compositing option, default 1.45, that applies a gamma curve to the text layer's anti-aliased mask. It makes CS6 text look different from earlier versions. | docs/03-tools/type-tools.md:56 |
| blend-modes | capability | All 27 Photoshop blend functions, the separable W3C Level 1 formulas, the Photoshop-only extended modes, Dissolve as a deterministic binary threshold, and diffs against the ImageMagick oracle. | openspec/specs/compositing/blend-modes/spec.md:6 |
| BlendMode | code | A layer blend mode: the 27 modes Photoshop CS6 exposes plus the group-only PassThrough. PassThrough is not a layer mode and is excluded from `LAYER_MODES`. | crates/pictura-core/src/lib.rs:107 ; openspec/specs/document/document-model/spec.md:6 |
| BlendMode::from_psd_key | code | Parses a 4-byte PSD blend-mode key; returns None for unknown keys. | crates/pictura-core/src/lib.rs:207 |
| BlendMode::LAYER_MODES | code | The 27 layer modes in PSD-spec table order; PassThrough is deliberately excluded. | crates/pictura-core/src/lib.rs:142 |
| BlendMode::to_psd_key | code | The 4-byte PSD blend-mode key for a mode, for example Normal to `norm` and Multiply to `mul `. | crates/pictura-core/src/lib.rs:173 |
| Blur family | code | Gaussian, Box, Motion, Radial, Average, Blur/Blur More, and Surface blur. | crates/pictura-filters/src/blur.rs:1 |
| Blur filters | cs6 | The Filter Blur submenu that softens an image by averaging neighboring pixels (Average, Blur, Blur More, Box, Gaussian, Lens, Motion, Radial, Shape, Smart, Surface). | docs/06-filters/blur-filters.md:13 |
| Blur Gallery | cs6 | The CS6 workspace with Field Blur, Iris Blur, and Tilt-Shift, offering on-canvas pins and OpenCL-accelerated photographic blur. | docs/06-filters/blur-gallery.md:11 |
| blur-filters | capability | Gaussian, Box, Motion, Radial, Average, Surface, Blur, and Blur More, with alpha preservation, clamp-to-edge borders, determinism, and an ImageMagick oracle or no-equivalent classification. | openspec/specs/imaging/blur-filters/spec.md:6 |
| BrightnessContrastParams | code | Brightness and Contrast values plus the legacy-mode flag. | crates/pictura-adjust/src/lib.rs:38 |
| Bristle tip | cs6 | A brush tip category whose marks are bundles of individual bristles; used most often with the Mixer Brush and previewed with a rotatable brush head. | docs/07-color-painting/bristle-brushes.md:16 |
| Brush panel | cs6 | Dock holding the brush tip options and dynamics that determine how paint is applied. | docs/02-ui-ux/panels/brushes-panel.md:13 |
| Brush pose | cs6 | CS6 panel section that locks or overrides tilt, rotation, and pressure values from the input device. | docs/07-color-painting/brush-engine.md:92 |
| Brush preset | cs6 | A saved brush tip with defined size, shape, and hardness, plus all Brush panel dynamics. | docs/07-color-painting/brush-presets.md:16 |
| Brush Presets panel | cs6 | Browser for brush presets and `.abr` libraries; usually docked with the Brush panel. | docs/02-ui-ux/panels/brushes-panel.md:13 |
| Brush projection | cs6 | CS6 option where stylus tilt and rotation warp the brush tip shape. | docs/07-color-painting/brush-engine.md:92 |
| Brush Stroke filters | cs6 | A Filter Gallery family of 8 painterly stroke effects. All are 8-bit only, and Spatter and Sprayed Strokes are memory-intensive. | docs/06-filters/brush-strokes-filters.md:13 |
| Brush Strokes family | code | Accented Edges, Angled Strokes, Crosshatch, Dark Strokes, Ink Outlines, Spatter, Sprayed Strokes, and Sumi-e, built from edge detection plus directional stroke rendering and tonal gating. | crates/pictura-filters/src/brush_strokes.rs:1 |
| Brush tool / Pencil tool | cs6 | Brush paints soft anti-aliased strokes; Pencil paints hard-edged aliased lines. Both paint the foreground color and share Opacity, Flow, and mode controls. | docs/03-tools/brush-and-pencil.md:11 |
| brush-stroke-filters | capability | The Brush Stroke family (Accented Edges through Sumi-e) under the shared filter error, alpha-preservation, parameter, and seeding contract. | openspec/specs/imaging/brush-stroke-filters/spec.md:6 |
| brush-tools | capability | Brush and Pencil paint the foreground colour, with a paint options bar, canvas pointer routing to the stroke engine, and size and hardness shortcuts. | openspec/specs/tools/brush-tools/spec.md:6 |
| BrushType | code | Paint Daubs brush shape: Simple, LightRough, DarkRough, WideSharp, WideBlurry, or Sparkle. | crates/pictura-filters/src/lib.rs:114 |
| buffer_to_image | code | Converts a planar 8-bit `PixelBuffer` (1 gray, 2 gray+alpha, 3 RGB, 4 RGBA) to an interleaved RGBA8888 `QImage`. Gray replicates across RGB and RGB gets opaque alpha. | crates/pictura-app/src/cxxqt_object.rs:3677 |
| build_tree | code | Assembles the bottom-first layer tree from flat records: an `lsct` divider (type 3) opens a group and folder records (types 1/2) close it. Unbalanced dividers are flattened into the parent. | crates/pictura-codec/src/lib.rs:701 |
| Byte-identical | process | Producing the exact same bytes, the stronger parity claim used when a GPU path must match the CPU oracle exactly. | docs/dev/STATE.md:648 |
| Calculations | cs6 | Command that blends two individual channels from one or more source images into a new document, new channel, or selection. | docs/08-selection/channel-based-masking.md:34 |
| Camera Raw Filter | cs6 | Adobe Camera Raw's raw-processing engine. In CS6 it is reachable only through Bridge or Open As Smart Object, since Filter > Camera Raw Filter arrived in CC. | docs/06-filters/camera-raw-filter.md:13 |
| Canonical spec | process | The current merged spec under openspec/specs/, the contract that new changes ADD to or MODIFY. | docs/dev/STATE.md:17 |
| Canvas Extension Color | cs6 | The fill color for canvas added by Image > Canvas Size. It is unavailable when the document has no Background layer, where added canvas is transparent. | docs/04-image-ops/canvas-size.md:26 |
| Canvas Size | cs6 | Command that changes the full editable area without resampling. Growing adds space; shrinking crops pixels. | docs/04-image-ops/canvas-size.md:11 |
| canvas-operations | capability | Canvas resize with nine-anchor placement and grow fill, background alpha, and shrink discard semantics; checked against the ImageMagick -extent oracle. | openspec/specs/document/canvas-operations/spec.md:6 |
| canvas-tools | capability | Move translates the active layer, Crop trims it, Eyedropper samples a colour, and Hand and Zoom navigate. | openspec/specs/tools/canvas-tools/spec.md:6 |
| Capability | process | A kebab-case named behaviour contract. A change declares new or modified capabilities, and each becomes openspec/specs/{domain}/{capability}/spec.md once archived. | AGENTS.md:43 ; openspec/changes/archive/2026-09-16-m38-icon-cursor-library/proposal.md:44 |
| change proposal | process | A reviewable unit of work under openspec/changes/<name>/ holding proposal.md (why and what), design.md (how), specs/{domain}/{capability}/spec.md (deltas), and tasks.md. | openspec/changes/m39-panel-anatomy/proposal.md:1 |
| changed | code | QSignal emitted whenever the layer stack changes and the image is refreshed, driving the shell to repaint the document. | crates/pictura-app/src/cxxqt_object.rs:36 |
| Channel | code/cs6 | One image plane listed in the Channels panel: composite, colour, alpha, or spot. Alpha channels store selections as grayscale masks. | crates/pictura-core/src/lib.rs:242 ; docs/01-architecture/document-model.md:35 ; docs/08-selection/channel-based-masking.md:15 |
| Channel Mixer | cs6 | Adjustment that modifies an output channel as a linear mix of the source channels. Source weights range -200 to +200 percent, with a Monochrome mode. | docs/04-image-ops/adjustments/channel-mixer.md:23 |
| ChannelMixerParams | code | Channel Mixer output-channel mixes in percent with per-channel constants and a monochrome flag. | crates/pictura-adjust/src/lib.rs:81 |
| Channels panel | cs6 | Dock listing the composite channel, color channels, alpha channels, and spot channels. | docs/02-ui-ux/panels/channels-panel.md:13 |
| Character panel | cs6 | Dock for character formatting such as font, size, tracking, kerning, leading, and baseline shift. | docs/02-ui-ux/panels/character-and-paragraph.md:15 |
| Character Styles panel | cs6 | New in CS6; stores character-level attribute styles applied to text. | docs/02-ui-ux/panels/character-and-paragraph.md:65 |
| CIE L*a*b* (Lab) | cs6 | Device-independent color model used as the color-management reference. L is 0-100, a is green to red, and b is blue to yellow. | docs/07-color-painting/color-models.md:35 |
| Provenance | process | The documentation-first method used here: a behavioral specification produced without Adobe source code, binaries, or assets. | docs/README.md:58 |
| Independent-creation method | project | A behavioral specification produced from public, lawful observation, kept separate from implementation; a defense against copyright infringement, not patents. | docs/00-overview/licensing-and-provenance.md:45 |
| Clipping mask | cs6 | A relationship where a base layer's non-transparent content reveals (clips) the successive layers above it. Clipped layers take the base's opacity and mode. | docs/01-architecture/document-model.md:114 ; docs/05-layers/vector-masks-and-clipping-masks.md:55 |
| Clipping path | cs6 | A saved path designated so part of an image becomes transparent when placed in a page-layout or vector application. EPS supports it but not alpha channels. | docs/02-ui-ux/panels/paths-panel.md:17 ; docs/08-selection/paths-and-vector-selection.md:37 |
| Clone Source panel | cs6 | Dock that stores up to five sampling sources for the Clone Stamp and Healing Brush, with offset, scale, rotation, flip, and overlay. | docs/02-ui-ux/panels/clone-source-panel.md:13 |
| Clone Stamp | cs6 | Tool that paints pixels sampled from another part of the image or another document. The sample point is set with Alt-click and controlled by Aligned and Sample options. | docs/03-tools/clone-stamp-and-pattern-stamp.md:16 |
| Clouds | cs6 | A Render filter that replaces the layer's pixels with a fractal-noise cloud pattern interpolated between the foreground and background colors. | docs/06-filters/render-filters.md:17 |
| CLUT | cs6 | Color lookup table, the palette of up to 256 colors defining an Indexed Color image. Pixels store indices into the table rather than colors. | docs/04-image-ops/indexed-color.md:16 |
| CMYK | cs6 | Subtractive process-ink model. Each pixel holds a percentage per ink, produced by a profile transform from the RGB working space. | docs/07-color-painting/color-models.md:36 |
| CMYK Color | cs6 | A four-channel color mode using process-ink percentages, intended for print color separation. | docs/04-image-ops/image-modes.md:21 |
| Color Balance | cs6 | Adjustment with three complementary slider pairs (cyan-red, magenta-green, yellow-blue) applied separately to shadows, midtones, and highlights. Preserve Luminosity keeps the tonal balance. | docs/04-image-ops/adjustments/color-balance.md:15 |
| Color Burn | cs6 | A darkening blend mode that inverts the base divided by the source; it is unavailable in Lab documents. | docs/05-layers/blend-modes.md:170 |
| Color Dodge | cs6 | A lightening blend mode that divides the base by the inverted source; it is unavailable in Lab documents. | docs/05-layers/blend-modes.md:169 |
| Color Dynamics | cs6 | Brush dynamics section varying color over a stroke. CS6 applies it once per stroke by default, and Apply Per Tip restores per-dab behavior. | docs/07-color-painting/brush-dynamics.md:103 |
| Color Libraries | cs6 | Picker button opening the Custom Colors dialog for spot-color books such as PANTONE, TOYO, TRUMATCH, FOCOLTONE, HKS, DIC, and ANPA-COLOR. | docs/07-color-painting/color-picker.md:51 |
| Color Lookup | cs6 | A CS6-new adjustment layer type that applies a 3DLUT File, Abstract, or Device Link lookup table; its PSD key is `clrL`. | docs/02-ui-ux/panels/adjustments-panel.md:39 ; docs/05-layers/adjustment-layers.md:41 |
| Color management (CMS) | cs6 | The system that translates colors between device spaces using Lab as reference; configured by Color Settings and driven by ICC profiles. | docs/07-color-painting/color-models.md:21 |
| Color management policy | cs6 | The per-model RGB/CMYK/Gray rule deciding what happens when an opened or pasted document's profile is missing or mismatched. Options are Off, Preserve Embedded Profiles, and Convert To Working Space. | docs/04-image-ops/color-profiles-and-assignment.md:74 |
| Color mode | cs6 | The per-document setting determining which model displays and prints the image, hence channels, tools, and file formats available. | docs/01-architecture/rust-core-design.md:27 ; docs/07-color-painting/color-models.md:19 |
| Color model | cs6 | A numeric method for describing color, such as RGB, CMYK, HSB, or Lab. | docs/07-color-painting/color-models.md:17 |
| Color panel | cs6 | Dock for editing foreground and background colors using color models and a color ramp. | docs/02-ui-ux/panels/color-panel.md:13 |
| Color Range | cs6 | Command that selects a specified color or tonal range; CS6 adds Skin Tones and Detect Faces. | docs/08-selection/color-range.md:13 |
| Color Replacement tool | cs6 | Brush that paints a replacement foreground color over a targeted color. It samples continuously, once, or from the background swatch, and preserves luminosity when the mode is Color. | docs/03-tools/color-replacement.md:11 |
| Color Sampler | cs6 | Tool that places up to four persistent document-space points whose color values show in the Info panel. The samplers are saved with the image. | docs/03-tools/eyedropper-color-sampler-ruler.md:30 |
| Color Settings | cs6 | Dialog that selects working spaces, color-management policies, the conversion engine, and the default rendering intent; saved as `.csf`. | docs/01-architecture/color-management.md:35 |
| Color space | cs6 | A variant of a color model with a specific gamut, for example sRGB, Adobe RGB, or ProPhoto RGB within the RGB model. | docs/07-color-painting/color-models.md:18 |
| color-management | capability | Runtime-built working-space profiles, ICC load and serialization, assign versus convert, rendering intents and black point compensation, bit depths, malformed-input errors, and an ImageMagick oracle. | openspec/specs/color/color-management/spec.md:6 |
| color-swatches-panel | capability | Foreground and background colour state, Color panel controls, a default swatch grid, and the Color and Swatches dock and toggle. | openspec/specs/ui/color-swatches-panel/spec.md:6 |
| color_range | code | Selects pixels within `fuzziness` of `target` with a soft coverage ramp. | crates/pictura-select/src/lib.rs:510 |
| ColorBalanceParams | code | Color Balance per-band cyan-red, magenta-green, and yellow-blue shifts for shadows, midtones, and highlights, plus a preserve-luminosity flag. | crates/pictura-adjust/src/lib.rs:98 |
| ColorError | code | Error for invalid ICC profiles or unsupported channel and bit-depth combinations. | crates/pictura-color/src/lib.rs:37 |
| Colorize | cs6 | Hue/Saturation mode that replaces hue and saturation with the foreground color's hue while preserving each pixel's lightness. It is absolute, not relative. | docs/04-image-ops/adjustments/hue-saturation.md:55 |
| ColorLabel | code | PSD `lclr` sheet color. Values match psd-tools SheetColorType: None, Red, Orange, Yellow, Green, Blue, Violet, Gray. | crates/pictura-core/src/lib.rs:271 |
| ColorMode | code | PSD color modes carried in `header.color_mode`: Bitmap, Grayscale, Indexed, Rgb, Cmyk, Multichannel, Duotone, Lab. | crates/pictura-core/src/lib.rs:9 |
| ColorMode::color_channels | code | Number of color channels the composite carries in PSD. Extra alpha, spot, and selection channels are additional, and Multichannel has no color channels. | crates/pictura-core/src/lib.rs:25 |
| ColorPanel | code | Dock panel with RGB and HSB sliders, a hex field, foreground/background swatches, and a hue spectrum, all editing the shared `ColorState`. | crates/pictura-app/cpp/panels/color_panel.h:62 |
| ColorState | code | A frame-owned Qt state object holding foreground/background colors and the active swatch target, shared by the Color panel, toolbox, and Eyedropper. | docs/dev/STATE.md:50 ; crates/pictura-app/cpp/panels/color_panel.h:19 |
| CombineMode | code | Combine mode for rasterized shapes; `New` replaces and the rest are boolean. | crates/pictura-select/src/lib.rs:42 |
| command identifier | code | The stable string key in the command table by which a menu command is dispatched, unchanged when a label or shortcut changes. | openspec/specs/document/command-registry/spec.md:6 |
| command tree | code | The full documented CS6 menu tree (docs/02-ui-ux/menus.md) registered by `addDefaultCommands`; commands without a handler are added as not implemented. | crates/pictura-app/cpp/commands.h:138 |
| command-registry | capability | A declarative command table keyed by stable identifier, the ten CS6 top-level menus, dispatch by identifier, enablement recomputed on menu open, and dynamic labels. | openspec/specs/document/command-registry/spec.md:6 |
| command_ids | code | Namespace of stable command identifier constants (for example `file.open`, `edit.undo`), used as the dispatch key and the future localization and menu-set key. | crates/pictura-app/cpp/commands.h:19 |
| CommandRegistry | code | Declarative command table plus menu construction and dispatch. It owns no application behaviour; the frame supplies handlers, providers, and actions by id. | crates/pictura-app/cpp/commands.h:86 |
| CommandSpec | code | One entry of the command table: id, menu path, label, shortcut, implemented flag, and checkable flag. An entry with an empty id is a separator. | crates/pictura-app/cpp/commands.h:74 |
| commit_move | code | Applies the real move once: shifts the topmost raster layer by (dx, dy), captures one "Move Layer" history state, marks dirty, recomposites, and emits `changed` using a single composite. | crates/pictura-app/src/cxxqt_object.rs:520 |
| compare | code | Compares two byte buffers of equal length with an absolute per-sample tolerance, and returns an error when the lengths differ. | crates/pictura-testkit/src/lib.rs:37 |
| composite-view | capability | Displays the composited layer stack when a PSD has layers, falls back to the embedded PSD composite when there are none, and runs an offscreen GPU path with a headless self-test. | openspec/specs/compositing/composite-view/spec.md:6 |
| composite_active | code | The render entry point that composites a document through the active backend and returns the buffer plus the `Backend` used. | crates/pictura-render/src/gpu.rs:204 ; docs/dev/STATE.md:47 |
| composite_gpu | code | Composites `doc` on the GPU for Normal and every separable mode. Returns the same 4-channel planar straight-alpha 8-bit buffer as `composite_rgba`, within +/- 1 LSB for the supported modes. | crates/pictura-render/src/gpu.rs:99 ; openspec/specs/compositing/gpu-compositing/spec.md:6 |
| composite_gpu_or_cpu | code | Tries `composite_gpu`; on any `GpuError` falls back to the CPU oracle. | crates/pictura-render/src/gpu.rs:186 |
| composite_region_active | code | Composites only `rect` (clamped to the document) through the active backend. Returns a `rect`-sized buffer byte-identical to the matching sub-rectangle of `composite_active`. | crates/pictura-render/src/gpu.rs:134 ; docs/dev/STATE.md:543 |
| composite_rgba | code | Composites the document's layer stack. Returns a 4-channel (R,G,B,A) planar, straight-alpha, 8-bit buffer at document resolution. | crates/pictura-render/src/lib.rs:62 ; openspec/specs/compositing/gpu-compositing/spec.md:6 |
| compositing bridge | code | The Rust-side layer compositing exposed to Qt through the `PictureView` invokable API and the `changed`/`regionBlitted` signals, so the C++ shell only receives finished images. | crates/pictura-app/src/cxxqt_object.rs:1 |
| Compositor | project | The pipeline that composites tiles and layers, running on the GPU with a CPU fallback. | docs/01-architecture/gpu-rendering-pipeline.md:117 |
| COMPRESSION_RAW | format | PSD image-data compression method 0: uncompressed raw planar bytes. | crates/pictura-codec/src/lib.rs:47 |
| COMPRESSION_RLE | format | PSD image-data compression method 1: PackBits RLE scanlines. | crates/pictura-codec/src/lib.rs:48 |
| Constrain Proportions | cs6 | Image Size checkbox, on by default, that links width and height so editing one updates the other. It is grayed out when Resample Image is off. | docs/04-image-ops/image-size.md:34 |
| Content-Aware Fill | cs6 | Edit > Fill mode that synthesizes similar nearby content to fill a selection. Fills are random, so repeating one gives different results. | docs/03-tools/content-aware-move-and-patch.md:53 |
| Content-Aware Move | cs6 | New in CS6; moves a selected object and fills the hole left behind from surrounding content. Modes are Move and Extend, with a five-level Adaptation control. | docs/03-tools/content-aware-move-and-patch.md:16 |
| Content-Aware Patch | cs6 | CS6 addition to the Patch tool that synthesizes nearby content for blending. It shares the Adaptation levels with Content-Aware Move. | docs/03-tools/content-aware-move-and-patch.md:38 |
| Content-Aware Scale | cs6 | Edit command that resizes while protecting important content, exposed through Amount, Protect, and Protect Skin Tones options. Publicly understood as seam carving with forward energy. | docs/03-tools/move-and-transform.md:35 |
| convert | code | Converts interleaved 8- or 16-bit samples between two ICC profiles, copying RGBA alpha through unchanged. | crates/pictura-color/src/lib.rs:158 |
| Convert to Profile | cs6 | Document operation that shifts pixel values to preserve appearance, then tags the document with the destination profile. It is lossy, so undo retains the pre-conversion pixels. | docs/01-architecture/color-management.md:30 ; docs/04-image-ops/color-profiles-and-assignment.md:41 |
| Core | project | Parity tier for the first target, the Standard editing workflow; Extended-only capabilities are excluded. | docs/00-overview/product-overview.md:62 |
| Coverage mask | project | A document-sized, tiled, single-channel buffer giving each pixel's membership in the selection as 0..1. | docs/08-selection/selection-model.md:15 |
| CPU fallback | project | The CPU raster compositor that provides the same tile interface when no usable GPU is present. | docs/01-architecture/gpu-rendering-pipeline.md:247 |
| crate-type` staticlib/rlib | code | The crate builds as both a C++-linkable static library (`staticlib`) and a Rust library (`rlib`), so the Qt executable and Rust tests can both use it. | crates/pictura-app/Cargo.toml:11 |
| Crop tool | cs6 | Tool that removes portions of an image to strengthen composition. The CS6 version places an interactive crop box on selection and supports non-destructive cropping. | docs/03-tools/crop-tool.md:12 |
| crop_document | code | Crops the document to the clamped `width`x`height` rect at `(x, y)`. Returns false and leaves the document untouched when the canvas intersection is empty. | crates/pictura-render/src/document_ops/crop.rs:16 |
| current_buffer | code | The buffer a wand samples: the composited layer stack when layers exist, otherwise the embedded PSD composite. | crates/pictura-app/src/cxxqt_object.rs:3376 |
| Curves | cs6 | Adjustment that edits points across the tonal range on an input-versus-output graph, with up to 14 control points. A Pencil mode allows a freehand curve that Smooth can relax. | docs/04-image-ops/adjustments/curves.md:14 |
| CurvesParams | code | Curves adjustment holding monotone control points in (input, output) order, inclusive of the endpoints. | crates/pictura-adjust/src/lib.rs:32 |
| Custom (filter) | cs6 | An Other filter that applies a user-defined 5x5 convolution matrix with a scale divisor and offset; filters can be saved and loaded. | docs/06-filters/other-filters.md:17 |
| Custom Shape | cs6 | Tool that draws arbitrary preset outlines chosen from the Custom Shape picker. Shapes support Defined Proportions and Defined Size geometry modes. | docs/03-tools/custom-shape.md:11 |
| cxx | code | The Rust/C++ interop crate that cxx-qt is built on; also usable alone for a narrow FFI boundary. | docs/01-architecture/rust-qt-interop.md:50 |
| cxx-qt | code | The Rust-to-Qt bridge. First-party Qt calls surface a Rust Result as a C++ exception, while the C plug-in ABI uses status codes. | docs/11-cross-cutting/error-handling.md:94 |
| cxx-qt (CXX-Qt) | code | KDAB's bridge that generates C++ `QObject` types from Rust `#[cxx_qt::bridge]` modules over cxx. | docs/01-architecture/system-architecture.md:159 |
| cxx-qt bridge | code | The cxx-qt binding layer declared by the `#[cxx_qt::bridge]` macro; it generates matching C++ and Rust code for a Rust-defined QObject. | crates/pictura-app/src/cxxqt_object.rs:16 |
| cxx_qt_import_crate | code | CMake function from CXX-Qt-CMake that builds the Rust crate and imports it into the C++ target, passing the discovered qmake. | CMakeLists.txt:47 |
| cxxqt_object.cxxqt.h | code | Generated C++ header exposing the Rust `PictureView` QObject; C++ sources include it to call invokables and connect signals. | crates/pictura-app/cpp/main.cpp:40 |
| CxxQtBuilder | code | Build-script helper that compiles the cxx-qt bridge file and links the Qt modules; `QMAKE` is filled in for standalone cargo builds. | crates/pictura-app/build.rs:38 |
| DabPlacer | code | Walks a stroke sample by sample and emits dab centres spaced by a step policy, carrying residual distance across samples. | crates/pictura-paint/src/spacing.rs:15 |
| Darker Color / Lighter Color | cs6 | Non-separable blend modes that copy the base or the source based on summed channel values, so they never produce a mixed third color. | docs/05-layers/blend-modes.md:192 |
| Data set | cs6 | One version of a data-driven graphic: a collection of variables and values for a single rendered output. | docs/09-automation/variables-and-data-driven-graphics.md:38 |
| decode_adjustment | code | Decodes a raw PSD adjustment block into a destructive `Adjustment` for the encodings this crate supports. `None` means not understood: the caller leaves the backdrop unchanged, never errors. | crates/pictura-render/src/lib.rs:219 |
| decode_packbits | code | Decodes one PackBits scanline into a destination slice of exactly its length; control -128 is a no-op. | crates/pictura-codec/src/lib.rs:280 |
| decode_rle_channel | code | Decodes a layer channel's PackBits payload: a scanline byte-count table followed by packed rows. | crates/pictura-codec/src/lib.rs:680 |
| Decontaminate Colors | cs6 | Refine Edge option that replaces color fringes with nearby fully selected colors, proportional to edge softness. It writes color and forces output to a new layer or document. | docs/08-selection/refine-edge.md:28 |
| Delete Cropped Pixels | cs6 | Crop tool checkbox, on by default, that permanently discards pixels outside the crop box. When off, the crop is non-destructive and can be re-edited. | docs/03-tools/crop-tool.md:31 |
| delete_paths | code | Deletes every listed node. Skips the Background and fully-locked nodes; a selected node whose ancestor is also selected is deleted once, with its ancestor. | crates/pictura-render/src/document_ops/layer_ops.rs:517 |
| Density (mask) | cs6 | A mask control that scales mask opacity; 100 percent fully blocks the masked area and lower values reveal more. | docs/05-layers/layer-masks.md:102 |
| Desaturate | cs6 | Destructive command that converts color to grayscale while leaving the document in the same color mode. It equals Hue/Saturation with Saturation at -100. | docs/04-image-ops/adjustments/desaturate.md:15 |
| Detect Faces | cs6 | Color Range Skin Tones option that restricts or boosts skin-tone selection to detected face regions. | docs/08-selection/color-range.md:22 |
| determinism | process | Same input, operation, and machine must yield the same pixels; a fixed assumption for golden tests, with seeds fixed for stochastic filters. | docs/dev/m6-filters.md:112 ; openspec/specs/verification/verification-harness/spec.md:25 |
| Determinism gate | project | Golden-image rule that renders every case twice and fails non-bit-identical pairs as FLAKY before any tolerance is applied. | docs/11-cross-cutting/testing-strategy.md:133 |
| DICOM | format/cs6 | Medical imaging format; Extended-only, with Patient, Study, Series, Equipment, and read-only Image metadata categories. | docs/00-overview/cs6-editions-and-constraints.md:43 ; docs/10-workflow-io/file-info-and-metadata.md:62 |
| Diff | code | Result of comparing two same-shaped buffers: total samples, differing samples, max delta, and sum delta. | crates/pictura-testkit/src/lib.rs:10 |
| Difference | cs6 | Blend mode computing the absolute per-channel difference of base and source; unavailable in Lab documents. | docs/05-layers/blend-modes.md:173 |
| Differential oracle | process | Comparison of this implementation's output against an independent tool (psd-tools or ImageMagick), recorded as a measured delta or a no-equivalent classification. | docs/dev/m4-adjustments.md:41 |
| Diffusion Dither | cs6 | Error-diffusion dithering in the Floyd-Steinberg family that spreads quantization error to neighboring pixels, producing a grainy, film-like result. | docs/04-image-ops/image-modes.md:69 |
| Direct Selection | cs6 | Tool that selects individual path segments and anchor points and reshapes curves via anchors and direction points; two letter A tools share the shortcut. | docs/03-tools/path-selection-tools.md:14 |
| Dirty rect | project | Rectangular region a mutation changed, used to composite and blit only that region instead of the whole document. | docs/dev/m31-region-compositing.md:1 |
| Displace | cs6 | Distort filter that shifts pixels by an external displacement map, where gray 128 is no shift and 0 or 255 is maximum negative or positive shift. | docs/06-filters/distort-filters.md:18 |
| display_dirty | code | Flag set by `refresh_region` marking the cached display image stale so the next `image()` call rebuilds it. | crates/pictura-app/src/cxxqt_object.rs:745 |
| Dissolve | cs6 | Stochastic blend mode where each pixel becomes base or blend color by a per-pixel pseudo-random threshold; the noise pattern is not published. | docs/05-layers/blend-modes.md:237 |
| Distort family | code | Geometric inverse-mapping warps: Twirl, Pinch, Spherize, Ripple, Wave, Polar Coordinates, Shear, ZigZag, and Ocean Ripple. | crates/pictura-filters/src/distort/mod.rs:1 |
| Distort filters | cs6 | The Filter Distort submenu of geometric warps (Displace, Glass, Pinch, Polar Coordinates, Ripple, Shear, Spherize, Twirl, Wave, ZigZag); most are 8-bit only. | docs/06-filters/distort-filters.md:13 |
| distort-filters | capability | The Distort family (Twirl through Ocean Ripple) using bilinear inverse mapping, undefined-area edge handling, tiny-image safety, determinism, and an oracle classification. | openspec/specs/imaging/distort-filters/spec.md:6 |
| Distribute Layers | cs6 | Command requiring three or more selected layers that pins the extremes and spaces the interior anchor lines evenly. | docs/05-layers/align-and-distribute.md:42 |
| Dither | cs6 | Random noise added during quantization or gradient fill to break up banding; used by gradients, Gradient Map, and indexed-color conversion. | docs/03-tools/gradient-and-paint-bucket.md:33 ; docs/10-workflow-io/color-settings.md:102 |
| Divide | cs6 | Photoshop-only blend mode computing min(1, base / source) with a guard for a black source; the definition is inferred. | docs/05-layers/blend-modes.md:191 |
| DIVIDER_NAME | code | The name `</Layer group>` Photoshop writes for the synthetic section-divider layer record that opens a group. | crates/pictura-codec/src/lib.rs:50 |
| DNG (Digital Negative) | format | Non-proprietary raw container ACR can open without camera-specific knowledge and can write adjustments into. | docs/10-workflow-io/camera-raw-workflow.md:108 |
| DocEntry | code | Private struct in `PicturaMainWindow` pairing a document's `PictureView` with its `ImageView`, file path, and untitled number. | crates/pictura-app/cpp/frame.h:109 |
| Dock | cs6 | Vertical column that stacks one or more panel groups; the frame has four dock areas. | docs/02-ui-ux/workspace-and-docks.md:83 |
| Document | code | Minimal document with dimensions, mode, depth, one composite image, and a layer tree; layers are stored bottom-first matching PSD z-order on disk. | crates/pictura-core/src/lib.rs:75 ; openspec/specs/document/document-model/spec.md:6 |
| Document model | project/cs6 | Proposed flat, id-addressed node arena representing a document, its layers, channels, paths, and metadata; holds `Document`, `Node`, and `NodeKind`. | docs/01-architecture/document-model.md:163 |
| document-canvas | capability | Document-level canvas resize: nine-anchor translation of layer and mask bounds, channel re-extension, composite recomputation, dirty-region refresh, region-composited move preview, and C++ region blit coherence. | openspec/specs/ui/document-canvas/spec.md:6 |
| document-lifecycle | capability | New, Open, Save, Save As, modified-state tracking, the unsaved-changes prompt, Revert, and recent-files persistence. | openspec/specs/document/document-lifecycle/spec.md:6 |
| document-model | capability | The Document holds a bottom-first layer tree; layers carry the M1 property set; BlendMode maps all 27 PSD keys, with per-layer channels, raster masks, and signed geometry in big-endian form. | openspec/specs/document/document-model/spec.md:6 |
| document-orientation | capability | Document rotation and flip entry points, exact recursive orientation remap, exactness and identity properties, composite recomputation, and an oracle. | openspec/specs/document/document-orientation/spec.md:6 |
| document-resize | capability | Document resize entry point and error contract, recursive resample of pixel layers, masks, and groups, resampling of document-level channels, composite recomputation, and an oracle. | openspec/specs/document/document-resize/spec.md:6 |
| document-tabs | capability | Tabbed document area, active-document targeting, the tab title with modified marker, and behaviour on closing a tab. | openspec/specs/document/document-tabs/spec.md:6 |
| Document::channels | code | Document-level extra channels (saved selections or spot channels) stored after the color channels in the PSD image-data section. | crates/pictura-core/src/lib.rs:84 |
| Document::composite | code | The document's flattened composite image. | crates/pictura-core/src/lib.rs:80 |
| Document::layers | code | The document's layer tree, bottom-first matching PSD on-disk z-order. | crates/pictura-core/src/lib.rs:81 |
| Document::new | code | Creates an empty document whose composite is zeroed for the mode's channel count, with no layers. | crates/pictura-core/src/lib.rs:88 |
| Dodge / Burn | cs6 | Tonal retouch tools that lighten and darken through a brush; options include Range (shadows, midtones, highlights), Exposure, and Protect Tones. | docs/03-tools/dodge-burn-sponge.md:15 |
| Drop Shadow | cs6 | Layer effect adding a shadow behind the layer content, offset by Distance and Angle, with default blend mode Multiply at 75 percent. | docs/05-layers/layer-styles.md:31 |
| Drop zone | cs6 | Highlighted region shown while dragging a panel; its activation follows the pointer position. | docs/02-ui-ux/workspace-and-docks.md:85 |
| Droplet | cs6 | Small application that applies an action to images or folders dragged onto it; Kooka Pictura replaces the executable with a portable `.opd` descriptor. | docs/09-automation/droplets.md:11 |
| DSSIM | project | Perceptual difference metric used as the primary comparison for convolution-filter golden cases, with a default ceiling of 0.001. | docs/11-cross-cutting/testing-strategy.md:71 |
| Duotone | cs6 | 8-bit grayscale color mode printed with one to four inks, covering monotone, duotone, tritone, and quadtone; each ink is controlled by a curve mapping gray values to ink percentages. | docs/04-image-ops/duotone.md:15 ; docs/07-color-painting/color-models.md:50 |
| Duotone specification | format | Unpublished PSD block treated as opaque and preserved verbatim while compositing as grayscale. | docs/01-architecture/document-model.md:179 |
| duplicate_layer | code | Deep-clones layer `index` (children, channels, mask, adjustment, and all attributes) and inserts the copy directly above it, named `<name> copy`. | crates/pictura-render/src/document_ops/layer_ops.rs:135 |
| duplicate_paths | code | Deep-clones each listed node directly above itself, naming each copy `<name> copy`; eligible everywhere including Background and locked nodes, and returns the new paths. | crates/pictura-render/src/document_ops/layer_ops.rs:538 |
| Dust & Scratches | cs6 | Noise filter that replaces a pixel only when it differs from its neighborhood reference by more than Threshold, balancing Radius against Threshold. | docs/06-filters/noise-filters.md:19 |
| ΔE2000 | project | Color-difference metric used for color-management golden cases with a mean ceiling of 1.0 and a max of 3.0. | docs/11-cross-cutting/testing-strategy.md:76 |
| edit-history | capability | Snapshot capture of the document and selection before each mutating command, undo and redo restoring bit-identical state, a bounded depth of 20, reset on document open, and app controls. | openspec/specs/document/edit-history/spec.md:6 |
| Efficiency | cs6 | Status-bar metric giving the percentage of time spent computing rather than paging to scratch; below about 95% signals insufficient RAM or slow scratch. | docs/01-architecture/system-architecture.md:45 ; docs/10-workflow-io/scratch-disks-and-memory.md:87 |
| Embedded Smart Object | cs6 | Smart Object whose source bytes are copied into the host document; in CS6 every placed object is embedded, so the document is self-contained. | docs/05-layers/linked-and-embedded-objects.md:13 |
| Emboss | cs6 | Stylize filter that converts the fill to gray and traces edges along an angle, producing a raised or stamped look. | docs/06-filters/stylize-filters.md:18 |
| encode_brightness_contrast | code | `brit` payload: brightness (i16), contrast (i16), mean (i16), lab_only (u8), pad; inputs are clamped to the decoder's accepted ranges. | crates/pictura-render/src/lib.rs:344 |
| encode_hue_saturation | code | `hue2` payload: version (2), enable (1), pad, colorization (3 x i16), the master Hue/Saturation/Lightness triplet (3 x i16), then six per-band range records. | crates/pictura-render/src/lib.rs:364 |
| encode_invert | code | `nvrt` payload; Invert carries no data. | crates/pictura-render/src/lib.rs:322 |
| encode_posterize | code | `post` payload: a u16 levels value (2..=255) plus 2 pad bytes, with out-of-range input clamped. | crates/pictura-render/src/lib.rs:331 |
| encode_threshold | code | `thrs` payload: a u16 level (1..=255) plus 2 pad bytes, with out-of-range input clamped. | crates/pictura-render/src/lib.rs:336 |
| end_paint | code | Commits the active stroke as one "Brush" or "Pencil" history state, marks dirty, and recomposites; a stroke that painted nothing leaves the document unchanged and returns false. | crates/pictura-app/src/cxxqt_object.rs:590 |
| Enhance Monochromatic Contrast | cs6 | Auto Color Correction algorithm that clips all channels identically, preserving the overall color relationship without adding or removing a cast; used by Auto Contrast. | docs/04-image-ops/adjustments/levels.md:164 |
| Enhance Per Channel Contrast | cs6 | Auto Color Correction algorithm that maximizes the tonal range in each channel independently, so it may add or remove a cast; used by Auto Tone. | docs/04-image-ops/adjustments/levels.md:168 |
| EPS | format | Encapsulated PostScript save/open format; supports Lab, CMYK, RGB, Indexed, Duotone, Grayscale, and Bitmap and clipping paths, but no alpha channels. | docs/10-workflow-io/save-and-save-as.md:105 |
| Equalize Histogram | cs6 | Automatic tone-mapping method that compresses dynamic range through histogram equalization while trying to preserve some contrast. | docs/04-image-ops/32-bit-hdr.md:63 |
| EXIF | format | Camera-generated metadata (make, lens, exposure, focal length, date/time, orientation) surfaced in the File Info Camera Data tab. | docs/01-architecture/rust-core-design.md:33 ; docs/10-workflow-io/file-info-and-metadata.md:42 |
| Exit code | process | Process return code the app self-test uses to report one specific check; each new check takes the next code. | docs/dev/STATE.md:117 |
| Exposure (adjustment) | cs6 | Adjustment with Exposure, Offset, and Gamma Correction controls that works in linear color space; its eyedroppers set parameters from luminance rather than per channel. | docs/04-image-ops/adjustments/exposure.md:14 |
| Exposure and Gamma | cs6 | Manual tone-mapping method using an Exposure gain and a Gamma contrast control; also names the method used by the 32-bit display preview. | docs/04-image-ops/32-bit-hdr.md:65 |
| ExposureParams | code | Exposure adjustment in stops (EV) with offset and gamma. | crates/pictura-adjust/src/lib.rs:45 |
| Extended-only | project | Parity tier marking capabilities in CS6 Extended only: 3D, measurement and counting, and DICOM. | docs/00-overview/cs6-editions-and-constraints.md:5 |
| ExtendScript (JSX) | cs6 | Cross-platform JavaScript automation language for Photoshop; CS6 uses ECMA-262 3rd edition plus E4X. | docs/09-automation/script-events-and-jsx.md:11 |
| Eyedropper | cs6 | Tool that samples color to set the foreground color, or the background color with Alt-click; Sample Size averages an NxN area and it can sample from the screen. | docs/03-tools/eyedropper-color-sampler-ruler.md:13 |
| Fade | cs6 | Command that changes the opacity and blend mode of the most recently applied filter or adjustment without re-running it. | docs/06-filters/filters-overview.md:59 |
| Feather | cs6 | Softening of a selection's edge, measured in pixels, set before the selection is made; it can also be applied later with Select > Modify > Feather. | docs/03-tools/marquee-selection.md:51 ; docs/08-selection/selection-model.md:43 |
| Feather (mask) | cs6 | Mask control that blurs mask edges over a pixel radius for a softer transition. | docs/05-layers/layer-masks.md:105 |
| Field Blur | cs6 | Blur Gallery effect that builds a gradient of blur amounts from multiple placed pins, each with its own blur value. | docs/06-filters/blur-gallery.md:22 |
| File Info | cs6 | Dialog presenting metadata categories (Description, Camera Data, IPTC, IPTC Extension, GPS, Origin, Photoshop, Raw Data) over the document. | docs/10-workflow-io/file-info-and-metadata.md:25 |
| Fill (opacity) | cs6 | Layer percentage that scales only the layer's pixels and shape or text content, not its layer effects; unavailable for groups. | docs/05-layers/layers-overview.md:149 |
| Fill layer | cs6 | Layer whose content is generated from a solid color, gradient, or pattern rather than stored pixels; unlike an adjustment layer it does not affect layers underneath. | docs/05-layers/fill-layers.md:17 |
| Fill Pixels | cs6 | Shape-tool drawing mode that rasterizes shapes onto the current layer using the foreground color; it supports a blend mode, opacity, and anti-aliasing. | docs/03-tools/shape-tools.md:28 |
| Filter | code/cs6 | Destructive Filter-menu operation, one variant per CS6 filter across the blur, sharpen, noise, stylize, pixelate, distort, render, texture, sketch, brush-stroke, artistic, and oil-paint families. | crates/pictura-filters/src/lib.rs:194 ; docs/06-filters/filters-overview.md:15 |
| Filter Gallery | cs6 | Dialog that applies multiple effects cumulatively in list order; it is 8-bit-per-channel only and is not offered for 16- or 32-bit documents. | docs/06-filters/filters-overview.md:42 |
| Filter mask | cs6 | Single white mask attached to a Smart Object that gates all of its Smart Filters; Mask Edge is unavailable for it. | docs/05-layers/smart-filters.md:33 |
| Filter plug-in | project | Filter supplied through the host's native C ABI and registered under the OP_CAP_FILTER capability; Adobe .8bf binary compatibility is a non-goal. | docs/06-filters/filters-overview.md:79 |
| filter-app-ui | capability | The PictureView filter command, filter-kind mapping and defaults, filter dock controls, and a confinement self-test. | openspec/specs/imaging/filter-app-ui/spec.md:6 |
| filter-application | capability | Destructive filter entry point, colour-channel extraction and validation, mask-confined writes, alpha and layer-metadata preservation, and errors instead of panics on bad layer data. | openspec/specs/imaging/filter-application/spec.md:6 |
| filter_gpu_available | code | Whether a usable adapter exists for the GPU filter kernels; reuses the compositor's shared device and requires the kernel's four storage buffers. | crates/pictura-render/src/gpu_filter.rs:77 |
| FilterError | code | Error returned by filter operations for unsupported buffers or invalid parameters instead of panicking; variants include Unsupported and InvalidParams. | crates/pictura-filters/src/lib.rs:33 ; openspec/specs/imaging/filter-application/spec.md:6 |
| Find Dark & Light Colors | cs6 | Auto Color Correction algorithm that finds the average lightest and darkest pixels to maximize contrast with minimal clipping; used by Auto Color with Snap Neutral Midtones. | docs/04-image-ops/adjustments/levels.md:172 |
| Find Edges | cs6 | No-dialog Stylize filter that emphasizes significant transitions as dark lines on a white background using a gradient operator. | docs/06-filters/stylize-filters.md:20 |
| Flatpak | project | Primary sandboxed Linux distribution target; Snap and AppImage were dropped. | docs/01-architecture/build-and-packaging.md:16 |
| flatten | code | Flattens a layer tree into records in on-disk order, emitting a divider, then children, then the folder record for each group. | crates/pictura-codec/src/lib.rs:737 |
| Flatten Image | cs6 | Destructive command that merges all visible layers into one Background layer, discards hidden layers, and fills leftover transparency with white. | docs/05-layers/merge-and-flatten.md:47 |
| flatten_rows | code | Flattens the whole tree depth-first, topmost-first, as `(path, depth)` rows; each container is walked last index to 0 because `Layer.children` is bottom-first. | crates/pictura-render/src/document_ops/layer_ops.rs:235 |
| flip_document | code | Mirrors the whole document along the vertical axis (`horizontal`) or the horizontal axis. | crates/pictura-render/src/document_ops/orient.rs:58 |
| flip_horizontal | code | Mirrors along the vertical axis: (x, y) becomes (W-1-x, y). | crates/pictura-ops/src/orient.rs:53 |
| flip_vertical | code | Mirrors along the horizontal axis: (x, y) becomes (x, H-1-y). | crates/pictura-ops/src/orient.rs:58 |
| Flow | cs6 | Rate at which paint builds up as the pointer moves, per pass; unlike Opacity, which caps a stroke, Flow can accumulate toward the opacity ceiling. | docs/03-tools/brush-and-pencil.md:36 ; docs/07-color-painting/airbrush-and-flow.md:23 |
| Flyout | cs6 | Hidden tools revealed by holding the mouse on a tool that has a corner triangle. | docs/02-ui-ux/toolbox-and-options-bar.md:22 |
| ForegroundBackgroundWidget | code | CS6-style foreground/background swatches: two overlapping squares with the active one outlined plus a corner reset to the default black/white pair. | crates/pictura-app/cpp/toolbox.h:22 |
| Free Transform | cs6 | Continuous transform command combining rotate, scale, skew, distort, and perspective; it accumulates operations into one matrix and resamples once on commit. | docs/03-tools/move-and-transform.md:24 |
| Freeform Pen | cs6 | Pen variant that draws freehand paths and fits curve segments automatically; Curve Fit sets how aggressively the path is simplified. | docs/03-tools/pen-and-path-tools.md:18 |
| Frozen interface | process | API or data contract written down before parallel implementation starts, so sub-agents work on disjoint files against it. | docs/dev/m18-toolbox-tools.md:36 |
| Fuzziness | cs6 | Color Range control widening the range of colors selected and increasing the number of partially selected pixels. | docs/08-selection/color-range.md:28 |
| Gamut warning | cs6 | Alert for RGB/HSB/Lab colors with no CMYK equivalent; clicking the triangle substitutes the closest printable color. | docs/01-architecture/color-management.md:67 ; docs/07-color-painting/color-models.md:63 |
| Gaussian Blur | cs6 | Canonical separable Gaussian convolution blur parameterized by radius; Adobe's radius-to-sigma mapping is not published. | docs/06-filters/blur-filters.md:18 |
| Generation counter | project | Stamp bumped on undo/redo so stale CPU and GPU tiles are never displayed. | docs/01-architecture/gpu-rendering-pipeline.md:76 |
| GIF | format | 8-bpc indexed web format; RGB images pass through the Indexed Color dialog before saving. | docs/10-workflow-io/save-and-save-as.md:103 |
| Glass | cs6 | Distort filter that simulates viewing through glass using a built-in or loaded texture as a refraction height field. | docs/06-filters/distort-filters.md:19 |
| Global light | cs6 | Document-level Angle and Altitude pair (image resource 1037) that every effect with Use Global Light reads; the default global angle is 30 degrees. | docs/05-layers/layer-styles.md:57 |
| Golden compare and hash | process | pictura-testkit's pixel comparison and hashing helpers, used to check rendering determinism and parity against stored reference images. | docs/dev/STATE.md:48 |
| Golden image | project/process | Stored reference buffer compared against produced output within an absolute per-sample tolerance, reporting differing count, max delta, and mean delta. | docs/11-cross-cutting/testing-strategy.md:97 ; docs/dev/m0-walking-skeleton.md:28 ; openspec/specs/verification/verification-harness/spec.md:6 |
| golden image comparison | code | Deterministic comparison of a produced buffer against a stored reference within a per-operation tolerance, the project's verification model. | crates/pictura-testkit/src/lib.rs:1 |
| GPU compute | project | App preference gpuCompute (default on) that routes compositing and supported filters through the wgpu backend, with a per-call CPU fallback. | docs/dev/STATE.md:402 |
| gpu-compositing | capability | wgpu compute compositor matching the CPU oracle within ±1 LSB, rejection of CPU-only modes before dispatch, graceful CPU fallback, planar data assembly, region compositing, and throughput evidence. | openspec/specs/compositing/gpu-compositing/spec.md:6 |
| gpu-compute-backend | capability | GPU compute as the default compositing backend with a capability probe, a user disable switch, persisted preference, status-bar reporting, and a guarantee that no GPU never blocks a document. | openspec/specs/compositing/gpu-compute-backend/spec.md:6 |
| gpu-filter-acceleration | capability | GPU filter entry point matching the CPU oracle, default-on GPU filters, unsupported-kernel CPU fallback, profile-driven kernel selection, and heavy-kernel performance evidence. | openspec/specs/compositing/gpu-filter-acceleration/spec.md:6 |
| gpu_available | code | Whether a usable Vulkan adapter or device exists; cached behind a `OnceLock`, cheap to call repeatedly, and never panics. | crates/pictura-render/src/gpu.rs:194 |
| gpu_compute | code | Per-document preference for GPU-accelerated compositing, toggled from the View menu and read by the composite path. | crates/pictura-app/src/cxxqt_object.rs:710 |
| gpu_interop_prepare | code | Creates a Vulkan wgpu device and offscreen texture and exports its raw VkInstance, VkPhysicalDevice, VkDevice, queue family, and VkImage handles for Qt RHI import. | crates/pictura-app/src/cxxqt_object.rs:689 |
| gpuCompute | code | C++ bridge property reporting and toggling the active GPU compute backend, persisted through the session store. | openspec/specs/compositing/gpu-compute-backend/spec.md:6 |
| GpuError | code | Why the GPU compositor could not run; the caller falls back to CPU. Variants include Unavailable, TooLarge, Readback, UnsupportedMode, and UnsupportedAdjustment. | crates/pictura-render/src/gpu.rs:57 |
| GpuRender | code | Outcome enum of an offscreen wgpu render: `Unavailable` when no Vulkan device exists, or `Rendered` with dimensions, RGBA bytes, and a distinct-colour count proving the output is not blank. | crates/pictura-app/src/gpu.rs:11 |
| Gradient Editor | cs6 | Dialog defining a gradient from colour stops, opacity stops, and midpoints; new gradients live in the preferences file until saved to a library. | docs/07-color-painting/gradient-presets.md:41 |
| Gradient Map | cs6 | Adjustment that maps the image's grayscale range to the colors of a chosen gradient: shadows to the first stop, midtones to intermediate stops, and highlights to the last. | docs/04-image-ops/adjustments/gradient-map.md:15 |
| Gradient tool | cs6 | Tool that creates a blend between multiple colors using one of five styles: Linear, Radial, Angle, Reflected, or Diamond; options include Reverse, Dither, and Transparency. | docs/03-tools/gradient-and-paint-bucket.md:11 |
| GrainType | code | Grain pattern enum: Regular, Soft, Sprinkles, Clumped, Contrasty, Enlarged, Stippled, Horizontal, Vertical, or Speckle. | crates/pictura-filters/src/lib.rs:180 |
| Grayscale | cs6 | Single-channel color mode with 8-bit values 0 (black) to 255 (white) or black-ink percentage; range defined by the Gray working space. | docs/07-color-painting/color-models.md:47 |
| Grayscale mode | cs6 | Single-channel mode of shades of gray, available at 8, 16, and 32 bpc; required source for Bitmap and Duotone conversion. | docs/04-image-ops/image-modes.md:17 |
| grid_2d | code | 2-D compute grid for `n` single-invocation items at 64 per workgroup; `gx` covers one row and `gy` stacks the remaining rows. | crates/pictura-render/src/gpu.rs:798 |
| Group isolation | cs6 | Behavior where a group whose blend mode is not Pass Through composites its children into a private buffer first, so internal adjustments do not affect outside layers. | docs/05-layers/layer-groups.md:49 |
| group_layer | code | Wraps layer `index` in a new group at the same stack position: the group takes the layer's slot and the layer becomes its only child. | crates/pictura-render/src/document_ops/layer_ops.rs:149 |
| group_paths | code | Wraps the selection in one new group inserted at the topmost selected node's position; refuses when the paths span more than one container, or any is the Background or fully locked. | crates/pictura-render/src/document_ops/layer_ops.rs:561 |
| grow | code | Expands a selection to include adjacent similar-colored pixels. | crates/pictura-select/src/lib.rs:442 |
| Grow / Similar | cs6 | Two Select commands that expand the selection within the Magic Wand tolerance: Grow adds adjacent pixels, Similar adds matching pixels anywhere; neither works on Bitmap or 32-bpc images. | docs/08-selection/grow-similar-and-modify.md:15 |
| guard.sh | process | Repository script that enforces project rules, including that documentation changes carry TASK-ALLOWS-DOCS in the commit message. | docs/dev/STATE.md:32 |
| HalftoneType | code | Halftone Pattern shape: Dot, Line, or Circle. | crates/pictura-filters/src/lib.rs:173 |
| Hand tool | cs6 | Navigation tool that pans the image within its window; the spacebar temporarily activates it from another tool, and it supports Flick Panning and Scroll All Windows. | docs/03-tools/hand-and-zoom.md:15 |
| Hard Light | cs6 | Harsh-spotlight contrast blend mode applying Multiply or Screen depending on the source. | docs/05-layers/blend-modes.md:171 |
| Hard proofing | cs6 | Print mode that simulates a press condition on the output device using a printer profile, with optional Simulate Paper Color and Simulate Black Ink. | docs/10-workflow-io/printing.md:57 |
| hash_bytes | code | verification-harness function computing a deterministic content hash, so identical bytes hash equally and one changed byte changes the hash. | crates/pictura-testkit/src/lib.rs:61 ; openspec/specs/verification/verification-harness/spec.md:25 |
| HDR Color Picker | cs6 | Picker shown for 32-bpc documents; adds an Intensity slider and exposure preview swatches and writes 32-bit float RGB values. | docs/07-color-painting/color-picker.md:76 |
| HDR Toning | cs6 | Command that applies the full HDR contrast and exposure tone mapping to a single image; it requires flattened layers and offers the same four tone-mapping methods. | docs/04-image-ops/adjustments/hdr-toning.md:15 |
| Healing Brush | cs6 | Tool that paints sampled pixels and reconciles their color and shading with the surrounding destination, keeping the source texture; it solves a biharmonic equation over the brush region. | docs/03-tools/healing-brushes.md:22 |
| High Pass | cs6 | Other filter that keeps edge detail within the radius and suppresses low frequencies, commonly used before Threshold or Bitmap conversion. | docs/06-filters/other-filters.md:18 |
| Higher bit depth | cs6 | 8, 16, or 32 bits per channel; 32-bit is floating point (HDR). | docs/04-image-ops/bit-depth-and-conversion.md:1 |
| Highlight Compression | cs6 | Automatic tone-mapping method that compresses highlights so they fall within the 8- or 16-bit luminance range. | docs/04-image-ops/32-bit-hdr.md:66 |
| Histogram | cs6 | Panel graphing the number of pixels at each intensity level, with Compact, Expanded, and All Channels views and statistics. | docs/07-color-painting/histogram-and-info.md:17 |
| HistogramPanel | code | Custom-painted 256-bin histogram dock with a channel selector; recomputes bins from the active view's composite. | crates/pictura-app/cpp/panels/histogram_panel.h:32 |
| HistogramView | code | Custom-painted 256-bin histogram widget fed precomputed bins and a colour. | crates/pictura-app/cpp/panels/histogram_panel.h:17 |
| History | code | Bounded undo/redo over labeled `(Document, Selection)` snapshots plus up to `MAX_SNAPSHOTS` named restore points; the cursor indexes the current linear state. | crates/pictura-app/src/history.rs:24 |
| History Brush | cs6 | Tool that paints a copy of a chosen history state or snapshot into the current image at the same coordinates; it is a masked state-to-state copy. | docs/01-architecture/undo-history.md:36 ; docs/03-tools/history-brush.md:15 |
| History Log | cs6 | User-edit provenance log (off by default) written to metadata, a text file, or both at Sessions Only, Concise, or Detailed verbosity. | docs/01-architecture/undo-history.md:39 ; docs/11-cross-cutting/logging-and-telemetry.md:15 |
| History panel | cs6 | Dock that lists snapshots above the linear list of history states and jumps the document to any state. | docs/02-ui-ux/panels/history-panel.md:13 |
| History state | cs6 | One editing step in the per-document history list, named after the operation. | docs/01-architecture/undo-history.md:15 |
| History States | cs6 | Bounded undo ring stored in RAM and scratch; default 20 with a range of 1-1000 in the fetched Adobe article. | docs/10-workflow-io/scratch-disks-and-memory.md:58 |
| history-panel | capability | Labelled history states, jumping to a state, history snapshots, and history panel docking and toggle. | openspec/specs/ui/history-panel/spec.md:6 |
| HistoryPanel | code | Dock panel listing the active view's history states and snapshots, activating a state on selection and offering a snapshot button. | crates/pictura-app/cpp/panels/history_panel.h:14 |
| HSB | cs6 | Color model based on human perception with hue 0-360 degrees, saturation 0-100%, and brightness 0-100%. | docs/07-color-painting/color-models.md:33 |
| HUD color picker | cs6 | On-canvas picker invoked while painting (Shift+Alt+right-click) as a Hue Strip or Hue Wheel; requires OpenGL. | docs/07-color-painting/color-picker.md:67 |
| Hue / Saturation / Color / Luminosity | cs6 | Non-separable component blend modes that transfer one attribute of the source to the backdrop using the W3C Lum, SetLum, and SetSat helpers. | docs/05-layers/blend-modes.md:226 |
| Hue/Saturation | cs6 | Adjustment for hue, saturation, and lightness, globally or within six color ranges; ranges have a center, width, and fall-off, and Colorize mode recolors absolutely. | docs/04-image-ops/adjustments/hue-saturation.md:15 |
| HueSaturationParams | code | Hue, saturation, and lightness shifts for the Hue/Saturation adjustment. | crates/pictura-adjust/src/lib.rs:53 |
| HueSpectrum | code | Horizontal hue spectrum widget that emits the hue under the pointer through a callback. | crates/pictura-app/cpp/panels/color_panel.h:45 |
| ICC profile | format | Color-space description embedded in a document or stored in PSD image resource 1039/1041; the basis of color-managed workflow. | docs/01-architecture/color-management.md:124 ; docs/10-workflow-io/file-info-and-metadata.md:93 |
| icon | code | Asset id to `QIcon` resource lookup where ids are file names without extension, command ids verbatim, and tools or cursors as `tool.<tool>`. | crates/pictura-app/cpp/icons.h:11 |
| Icon dock | cs6 | Dock collapsed to icons; the double arrow at the top toggles all its panels. | docs/02-ui-ux/workspace-and-docks.md:118 |
| icon-assets | capability | Original SVG icon set covering app, tools, and menus, bundled through a Qt resource and resolved by id, with a scalable application icon. | openspec/specs/ui/icon-assets/spec.md:6 |
| Image Processor | cs6 | Script that converts a folder of files to JPEG, PSD, and/or TIFF without an action, optionally applying Camera Raw settings and sRGB conversion. | docs/09-automation/batch-processing.md:41 |
| Image resource | format | Tagged metadata block in the PSD resource section, such as ICC (1039), EXIF (1058/1059), or XMP (1060). | docs/01-architecture/document-model.md:133 |
| Image Size | cs6 | Command that changes pixel dimensions, print size, or resolution of the whole document; Resample Image decides whether pixel data changes. | docs/04-image-ops/image-size.md:11 |
| image-adjustments | capability | Destructive adjustment contract plus Levels, Curves, Brightness/Contrast, Exposure, Invert, Posterize, Threshold, Desaturate, Auto, Hue/Saturation, Black and White, Photo Filter, Channel Mixer, Vibrance, and Color Balance, with determinism and an oracle. | openspec/specs/imaging/image-adjustments/spec.md:6 |
| image-ops-app-ui | capability | App document-resize, canvas-resize, and orientation commands, selection cleared by document operations, the image operations dock, and headless self-test coverage. | openspec/specs/ui/image-ops-app-ui/spec.md:6 |
| image-orientation | capability | Exact right-angle and flip remaps, arbitrary rotation with an expanded bounding box, and an oracle that is exact for right angles and measured for arbitrary rotation. | openspec/specs/document/image-orientation/spec.md:6 |
| image-resize | capability | Image resize entry point and error contract, resample kernels with per-channel sampling, and an ImageMagick differential oracle. | openspec/specs/document/image-resize/spec.md:6 |
| ImageMagick | code/process | External image tool used as a differential oracle for filter, resize, and canvas behaviours; modes it cannot express are classified no-equivalent and covered by hand-computed tests. | docs/dev/STATE.md:11 ; openspec/specs/compositing/blend-modes/spec.md:74 |
| ImageView | code | Central document canvas that paints a `QImage` under a pan/zoom transform over a plain canvas colour, with a transparency checkerboard and tool-event signals. | docs/dev/STATE.md:790 ; crates/pictura-app/cpp/image_view.h:18 |
| Indexed Color | cs6 | 8-bpc mode with up to 256 colors via a color lookup table; conversion flattens visible layers and discards hidden ones. | docs/04-image-ops/indexed-color.md:15 ; docs/07-color-painting/color-models.md:49 |
| Info panel | cs6 | Dock showing a contextual readout grid, tool hints, status information, and up to four color samplers. | docs/02-ui-ux/panels/info-panel.md:13 |
| info-histogram-panel | capability | Info panel readouts, a histogram view, the Info and Histogram dock and toggle, and a rule that panels do not re-composite per refresh. | openspec/specs/ui/info-histogram-panel/spec.md:6 |
| InfoPanel | code | Dock panel showing cursor position, sampled colour, selection size, and document size for the active view. | crates/pictura-app/cpp/panels/info_panel.h:12 |
| Intent | code | ICC rendering intent: Perceptual, RelativeColorimetric, Saturation, or AbsoluteColorimetric. | crates/pictura-color/src/lib.rs:18 |
| interop-probe | code | `main` flag that creates an interop state and attempts the QRhi import, printing the result and exiting; used to test the zero-copy path in isolation. | crates/pictura-app/cpp/main.cpp:63 |
| InteropState | code | A live wgpu device plus its exported Vulkan handles and offscreen texture, kept alive because the raw handles are only valid while the device lives. | crates/pictura-app/src/gpu.rs:242 |
| Invert | cs6 | Adjustment that reverses every color channel on the 256-step scale, so 255 becomes 0; it has no editable settings. | docs/04-image-ops/adjustments/invert.md:15 |
| iOpa | format | Additional-layer-info tagged block holding the layer fill opacity as a single byte. | crates/pictura-codec/src/lib.rs:548 |
| IPTC | format | Editable creator, credit, source, and copyright metadata; IPTC Core values are synchronized between XMP and the older IPTC-IIM byte record. | docs/01-architecture/file-formats.md:140 ; docs/10-workflow-io/file-info-and-metadata.md:43 |
| Iris Blur | cs6 | Blur Gallery effect simulating shallow depth of field with an adjustable sharp core, fade annulus, and blurred exterior. | docs/06-filters/blur-gallery.md:26 |
| is_background | code | M36 Background heuristic: top-level index 0, not a group, no adjustment data, and named exactly `Background`. | crates/pictura-render/src/document_ops/layer_ops.rs:256 |
| isolated group | domain | Group whose children are composited in isolation before the group result is blended onto the running canvas. | openspec/specs/compositing/layer-compositing/spec.md:81 |
| Jitter | cs6 | Brush dynamics primitive expressing randomness as a percentage; 0% means no change over the stroke and 100% means maximum randomness. | docs/07-color-painting/brush-dynamics.md:20 |
| JPEG | format | Lossy 8-bit web and camera format; options include Matte, quality 0-12, and Baseline, Baseline Optimized, or Progressive. | docs/10-workflow-io/save-and-save-as.md:100 |
| Knockout | cs6 | Advanced-blending option (None, Shallow, Deep) that makes a layer punch through lower layers to reveal content from elsewhere in the stack. | docs/05-layers/layers-overview.md:89 |
| kToolTable | code | Frozen 71-entry CS6 tool catalogue in table order: name, label, shortcut, cursor, hint, group, implemented flag, and hotspot. | crates/pictura-app/cpp/tools.cpp:14 |
| Lab Color | cs6 | Device-independent three-channel mode with L 0 to 100 and a/b from +127 to -128; used as the color-management reference space. | docs/04-image-ops/image-modes.md:22 |
| Lasso / Polygonal Lasso | cs6 | Freehand and click-to-vertex selection tools that produce a closed polygon; the magnetic variant snaps the border to image edges. | docs/03-tools/lasso-selection.md:15 |
| Layer | code/cs6 | A pixel layer or a group (`is_group`); groups carry `children` bottom-first, `rect` is the layer bounds and may be empty for groups, and an adjustment layer carries `adjustment` and in PSD no color channels. | crates/pictura-core/src/lib.rs:343 ; docs/01-architecture/document-model.md:18 ; openspec/specs/document/document-model/spec.md:6 |
| Layer comp | cs6 | Saved snapshot of layer visibility, position, and appearance (style applied and blend mode) that can be applied to restore that state. | docs/02-ui-ux/workspace-and-docks.md:30 ; docs/05-layers/layer-comps.md:13 |
| Layer effect | cs6 | Procedural decoration attached to a layer's transparency, such as a shadow, glow, bevel, overlay, or stroke. | docs/05-layers/layer-styles.md:17 |
| Layer filter/search | cs6 | CS6 Layers-panel feature that shows only layers matching a criterion by name, kind, effect, mode, attribute, or color label; it is view-only. | docs/05-layers/layer-filtering-and-search.md:17 |
| Layer group | cs6 | Folder node holding layers and nested groups that can carry its own name, color, blend mode, opacity, and masks. | docs/05-layers/layer-groups.md:14 |
| Layer mask | cs6 | Resolution-dependent grayscale bitmap stored as an alpha channel that hides or reveals parts of a layer non-destructively. | docs/01-architecture/document-model.md:30 ; docs/05-layers/layer-masks.md:14 |
| layer path | code | Frozen layer address grammar: a slash-separated list of bottom-first child indices, positional and invalidated by structural change, refused rather than panicked when out of range. | crates/pictura-app/src/cxxqt_object.rs:231` ; openspec/changes/m39-panel-anatomy/proposal.md:15 |
| Layer style | cs6 | A set of effects on a layer (drop shadow, glows, bevel/emboss, satin, overlays, stroke), serialized in the `lrFX` block. | docs/01-architecture/document-model.md:32 ; docs/05-layers/layer-styles.md:18 |
| Layer Via Copy / Layer Via Cut | cs6 | Commands that copy or cut the active selection to a new layer; Smart Object and shape layers must be rasterized first. | docs/05-layers/layer-management-ui.md:93 |
| layer-compositing | capability | Planar 4-channel output format, bottom-to-top stack walk, W3C source-over equation, opacity, mask, and fill scaling of source alpha, isolated and pass-through group semantics, and off-canvas clipping. | openspec/specs/compositing/layer-compositing/spec.md:6 |
| Layer.fill, Layer.lock, Layer.color | code | M36 layer attributes: fill opacity, a LockFlags bitmask, and an eight-value color label, round-tripped through the lspf, lclr, and iOpa PSD blocks. | docs/dev/STATE.md:638 |
| Layer::is_group | code | Whether this layer is a group. | crates/pictura-core/src/lib.rs:361 |
| LayerMask | code | Raster layer mask; `data` is None until the channel image is decoded and `disabled` maps to mask flags bit 0x02. | crates/pictura-core/src/lib.rs:260 |
| LayerRowDelegate | code | Qt delegate painting the CS6 row anatomy: eye, thumbnail or folder glyph, name, colour swatch, clip indent and underline, mask thumbnail, and fx badge. | openspec/changes/m39-panel-anatomy/proposal.md:15 ; crates/pictura-app/cpp/panels/layers_panel.h:21 |
| Layers panel | cs6 | Primary navigation dock over the layer stack, exposing visibility, locks, blend, opacity, fill, masks, and filtering. | docs/02-ui-ux/panels/layers-panel.md:13 |
| layers-panel | capability | Layer rows, property editing, layer operations, grouping commands, thumbnails, docking and toggle, and the action strip icons. | openspec/specs/ui/layers-panel/spec.md:6 |
| LayersModel | code | Layers panel tree model, promoted from a flat table to a QAbstractItemModel with one visible column and a role per row value. | openspec/changes/m39-panel-anatomy/proposal.md:15 ; crates/pictura-app/cpp/panels/layers_panel.h:22 |
| LayersPanel | code | Dock panel with the layer tree, blend/opacity/fill controls, lock toggles, colour labels, solo mode, and inline rename. | crates/pictura-app/cpp/panels/layers_panel.h:25 |
| layers_snapshot | code | Single bridge call returning one typed `LayerRow` per layer — path, depth, name, kind, visibility, blend, opacity, fill, locks, colour, clipping and style flags, child count, style-effect names, and row/mask thumbnails — replacing the Layers panel's per-row field invokables. | docs/dev/refactor-view-seam.md:44 |
| lclr | format | Additional-layer-info tagged block holding the layer color label as a 16-bit value whose low byte is a ColorLabel. | crates/pictura-codec/src/lib.rs:544 |
| lcms2 | code | Little CMS 2, the system library used for ICC profile handling in pictura-color. | docs/dev/STATE.md:10 |
| lcms2 (Little CMS) | code | Color-management library wrapped by `pictura-color` for ICC transforms and soft proofing. | docs/01-architecture/color-management.md:193 |
| Lens Blur | cs6 | Depth-map-driven blur with an iris-shaped aperture controlling bokeh, plus specular highlight and re-injected noise controls. | docs/06-filters/blur-filters.md:19 |
| Lens Correction | cs6 | Filter that corrects barrel or pincushion distortion, chromatic aberration, vignetting, perspective, and rotation using lens profiles or manual sliders. | docs/06-filters/lens-correction.md:13 |
| Lens Flare | cs6 | Render filter that adds a bright source with ghost reflections and a starburst, with a movable center, brightness, and lens type. | docs/06-filters/render-filters.md:20 |
| LensType | code | Lens model for the Lens Flare ghost chain and starburst: Zoom, Prime35, Prime105, or MoviePrime. | crates/pictura-filters/src/render.rs:215 |
| Levels | cs6 | Adjustment that corrects tonal range and color balance with input and output shadow, gamma, and highlight controls; the histogram is the visual guide. | docs/04-image-ops/adjustments/levels.md:14 |
| LevelsParams | code | Levels adjustment inputs: input black and white points, gamma, and output black and white points. | crates/pictura-adjust/src/lib.rs:23 |
| LightDirection | code | Eight-way light direction for relief filters, running from Bottom around to BottomRight. | crates/pictura-filters/src/lib.rs:161 |
| Lighting Effects | cs6 | CS6-revamped GPU Render workspace that lights a layer with Point, Infinite, or Spot lights over an optional grayscale bump map; RGB only. | docs/06-filters/lighting-effects.md:13 |
| Linear Dodge (Add) | cs6 | Photoshop-only additive blend mode computing min(1, base + source); a black source is a no-op. | docs/05-layers/blend-modes.md:184 |
| Linked Smart Object | cs6 | Smart Object that references an external source file by path instead of embedding it; introduced in CC 2014, so it is not CS6 parity. | docs/05-layers/linked-and-embedded-objects.md:25 |
| Liquify | cs6 | Mesh-warp filter that pushes, pulls, twirls, puckers, and bloats pixels, with a freeze mask, saveable meshes, and reconstruction. | docs/06-filters/liquify.md:13 |
| Local Adaptation | cs6 | Tone-mapping method that adjusts local brightness regions using an edge-aware operator; controls include Edge Glow Radius and Strength, Tone and Detail, Color, and a Toning Curve. | docs/04-image-ops/32-bit-hdr.md:53 |
| Localized Color Clusters | cs6 | Color Range option that adds a spatial term so matching colors far from the sample points are excluded, controlled by the Range slider. | docs/08-selection/color-range.md:29 |
| lock_from_bits | code | Builds LockFlags from the low bits of the `lspf`/record flags, since the newtype has no public bit constructor. | crates/pictura-codec/src/lib.rs:605 |
| LockFlags | code | Newtype bitmask for layer locks: TRANSPARENCY, PIXELS, and POSITION, with all() equal to 0x07. | crates/pictura-core/src/lib.rs:304 ; docs/dev/STATE.md:639 |
| LockFlags::all | code | All three lockable bits, the panel's "Lock All" toggle. | crates/pictura-core/src/lib.rs:328 |
| LockFlags::contains | code | Tests whether a given lock bit is set. | crates/pictura-core/src/lib.rs:315 |
| LockFlags::PIXELS | code | Lock bit 0x02 for locking pixels. | crates/pictura-core/src/lib.rs:308 |
| LockFlags::POSITION | code | Lock bit 0x04 for locking position. | crates/pictura-core/src/lib.rs:309 |
| LockFlags::TRANSPARENCY | code | Lock bit 0x01 for locking transparency. | crates/pictura-core/src/lib.rs:307 |
| LockFlags::with | code | Returns a copy with one lock bit turned on or off. | crates/pictura-core/src/lib.rs:319 |
| LoD | project | Level of detail; a display-time proxy level so a zoomed-out view composites less than full resolution, currently deferred. | docs/dev/m31-region-compositing.md:46 |
| LSB | process/domain | Least significant bit; the GPU parity contract allows the GPU result to differ from the CPU oracle by at most one LSB per channel. | docs/dev/m28-gpu-heavy-filters.md:34 ; openspec/specs/compositing/gpu-compositing/spec.md:6 |
| lsct | format | Additional-layer-info tagged block marking a group section: type 1 open folder, 2 closed folder, 3 bounding section divider; a group's blend key may be embedded after its 8BIM signature. | crates/pictura-codec/src/lib.rs:551 |
| lspf | format | Additional-layer-info tagged block holding the layer lock flags; the low 3 bits map to LockFlags. | crates/pictura-codec/src/lib.rs:540 |
| luni | format | Additional-layer-info tagged block holding a layer's Unicode name: a u32 UTF-16 code-unit count followed by that many big-endian u16 units. | crates/pictura-codec/src/lib.rs:535 |
| LUT | cs6 | Lookup table used to apply a point operation efficiently, one-dimensional for per-channel maps and three-dimensional for vector maps; exact for integer depths and approximate for 32-bit float with interpolation. | docs/04-image-ops/adjustments-overview.md:171 |
| Magic Eraser | cs6 | Click tool that changes all similar pixels to transparency, or to the background color on a locked-transparency layer; options include Tolerance and Contiguous. | docs/03-tools/eraser-tools.md:31 |
| Magic Wand | cs6 | Selection tool that selects a consistently colored area by clicking. Options include tolerance, anti-aliasing, contiguity, and sampling all layers. | docs/03-tools/quick-selection-and-magic-wand.md:16 |
| magic_wand | code | Flood or global selection by color tolerance, comparing pixels with `chebyshev` and `rgb_at`. | crates/pictura-select/src/lib.rs:381 |
| Magnetic Lasso | cs6 | Selection tool whose border snaps to defined edges as the pointer moves. Width, contrast, and frequency control the edge following. | docs/03-tools/lasso-selection.md:17 |
| Magnetic Pen | cs6 | Freeform Pen option that draws a path snapping to image edges. It places fastening points as the pointer moves. | docs/03-tools/pen-and-path-tools.md:19 |
| Marching ants | cs6 | Animated dashed outline traced from a selection mask's 50 percent coverage contour. It is view-only and drawn in the overlay (`MarchingAntsItem`), not into pixels. | docs/03-tools/marquee-selection.md:83 ; docs/08-selection/selection-model.md:47 |
| Mask | cs6 | Stored grayscale image (alpha channel, quick mask, or per-layer mask) where white is fully selected or editable and black is protected. | docs/08-selection/selection-model.md:22 |
| Masked Areas | cs6 | Default quick mask display mode painting masked areas black and selected areas white. The alternate mode, selected areas, inverts that mapping. | docs/03-tools/quick-mask-tool.md:27 ; docs/08-selection/quick-mask.md:24 |
| Match Color | cs6 | RGB-only command matching colors between images, layers, or selections by transferring statistics. Options include neutralize, luminance, color intensity, and fade. | docs/04-image-ops/adjustments/match-color.md:15 |
| MAX_CHANNELS | format | Maximum channel count accepted (56) for the header and for layer records. | crates/pictura-codec/src/lib.rs:39 |
| MAX_DABS_PER_CALL | code | Hard ceiling on dabs emitted by a single `DabPlacer::feed` call. | crates/pictura-paint/src/spacing.rs:10 |
| MAX_DEPTH | code | Undo depth ceiling of 20 steps; the oldest state is dropped when a capture would exceed it. | crates/pictura-app/src/history.rs:30 |
| MAX_DIM_PSB | format | Maximum PSB dimension accepted (300,000). | crates/pictura-codec/src/lib.rs:41 |
| MAX_DIM_PSD | format | Maximum PSD dimension accepted (30,000). | crates/pictura-codec/src/lib.rs:40 |
| MAX_SNAPSHOTS | code | Named-snapshot ceiling of 10 restore points; the oldest is dropped when a new snapshot exceeds it. | crates/pictura-app/src/history.rs:31 |
| Maximize Compatibility | format/cs6 | PSD/PSB preference (always, ask, never) writing a merged composite alongside layer data so older and non-Photoshop applications can read the file. | docs/01-architecture/document-model.md:158 ; docs/10-workflow-io/document-lifecycle.md:72 |
| Maximum | cs6 | Other filter performing morphological dilation, replacing each pixel with the highest-brightness neighbor within the radius. Often used on masks. | docs/06-filters/other-filters.md:19 |
| Measurement Log panel | cs6 | Extended-only record surface for measurements and counts made with the ruler, count, or selection tools. | docs/02-ui-ux/panels/measurement-log-panel.md:13 |
| Measurement scale | cs6 | Mapping of a pixel length to logical units stored as PSD resource 1074; default is 1 pixel = 1 pixel. | docs/02-ui-ux/panels/measurement-log-panel.md:17 ; docs/10-workflow-io/measurement-and-count.md:34 |
| Median | cs6 | Non-linear rank filter replacing each pixel with the median of its neighborhood, preserving edges while removing impulse noise. | docs/06-filters/noise-filters.md:20 |
| Mercury Graphics Engine | cs6 | Adobe's CS6 GPU stack using OpenGL and OpenCL for Liquify, Lighting Effects, Oil Paint, and Blur Gallery, with CPU fallback for most. | docs/06-filters/filters-overview.md:88 |
| Mercury Graphics Engine (MGE) | cs6 | CS6's unified GPU acceleration layer, using OpenGL and OpenCL rather than CUDA. Its algorithms are closed. | docs/01-architecture/system-architecture.md:26 |
| Merge commands | cs6 | Destructive Layer menu commands that collapse layer nodes into one pixel layer: merge down, merge layers, merge visible, and merge clipping mask. | docs/05-layers/merge-and-flatten.md:21 |
| Merge To HDR Pro | cs6 | Command combining multiple exposures of the same scene into one HDR image. It supports automatic alignment, ghost removal, and a saved response curve. | docs/04-image-ops/32-bit-hdr.md:25 |
| Metadata | project | Document-level file resolution, physical size, pixel aspect ratio, color profile, EXIF, IPTC, and XMP. | docs/01-architecture/document-model.md:45 |
| Metadata template | cs6 | Plain XMP file exporting or importing File Info field sets, combined on apply by append or replace. | docs/10-workflow-io/file-info-and-metadata.md:100 |
| MezzotintType | code | Mezzotint pattern: fine, medium, grainy, or coarse dots, then short, medium, or long lines, then short, medium, or long strokes. | crates/pictura-filters/src/lib.rs:60 |
| migration unit | process | One independently landable slice of the view-seam refactor (document+history, selection, paint+transform, layers+snapshot, type), each leaving the build green with its consumers migrated and its superseded surface deleted or named as a deferral. | docs/dev/refactor-view-seam.md:66 |
| Milestone | process | An M-numbered development stage, for example m38-icon-cursor-library and m39-panel-anatomy of the layers-panel program. | docs/dev/STATE.md:52 ; openspec/changes/m39-panel-anatomy/proposal.md:1 |
| Milestone brief | process | Task document under docs/dev/m*-*.md specifying a milestone's scope, contract, oracle, and exit gate. | docs/dev/STATE.md:850 |
| Minimum | cs6 | Other filter performing morphological erosion, replacing each pixel with the lowest-brightness neighbor within the radius. | docs/06-filters/other-filters.md:20 |
| Mixer Brush | cs6 | Tool simulating realistic painting with a two-well model: a reservoir for deposited color and a pickup well for canvas paint (`MixerEngine`). Controls are wet, load, and mix. | docs/03-tools/mixer-brush.md:11 ; docs/07-color-painting/mixer-brush-engine.md:16 |
| Modal control | cs6 | Per-command, action, or set toggle that pauses action playback so the user can enter values in a dialog or use a modal tool (`withDialog`). | docs/02-ui-ux/panels/actions-panel.md:46 ; docs/09-automation/actions.md:34 |
| MODE_GRAYSCALE | format | PSD header color-mode code 1, mapped to `ColorMode::Grayscale`. | crates/pictura-codec/src/lib.rs:36 |
| MODE_RGB | format | PSD header color-mode code 3, mapped to `ColorMode::Rgb`. | crates/pictura-codec/src/lib.rs:37 |
| Motion Blur | cs6 | Blur convolving along a line at an angle over a distance of 1 to 999, simulating a moving subject. | docs/06-filters/blur-filters.md:20 |
| move preview | code | Live drag preview for the Move tool: caches a base image plus the moved layer so dragging never recomposites, then commits once on release (`begin_move_preview`, `commit_move`). | crates/pictura-app/src/cxxqt_object.rs:480 |
| move_path | code | Move the node `delta` places within its own container, clamped to container bounds. Refuses the background and fully locked nodes. | crates/pictura-render/src/document_ops/layer_ops.rs:649 |
| Multichannel | cs6 | Mode with 2 to 56 channels of 256 gray levels each, for specialized printing. It supports no layers and flattens on conversion. | docs/04-image-ops/image-modes.md:23 |
| Multiply | cs6 | Darkening blend mode computing base times source per channel. | docs/05-layers/blend-modes.md:165 |
| named deferral | process | A bridge operation no migration unit has claimed yet: listed by name in the change's tasks instead of being wrapped by a pass-through adapter, so the exclusive-surface claim stays honest. | docs/dev/refactor-view-seam.md:36 |
| Navigator panel | cs6 | Dock with a document thumbnail and a proxy view area for pan and zoom. | docs/02-ui-ux/panels/navigator-panel.md:13 |
| navigator-panel | capability | A navigator thumbnail and proxy, navigator zoom controls, and the navigator dock and toggle. | openspec/specs/ui/navigator-panel/spec.md:6 |
| NavigatorPanel | code | Dock panel with a document thumbnail, a proxy rectangle for the visible region, and a zoom slider, driven by the active `ImageView`. | crates/pictura-app/cpp/panels/navigator_panel.h:47 |
| NavigatorThumbnail | code | Widget painting the document thumbnail and the proxy rectangle for the region the canvas shows (`setImage`, `setView`). | crates/pictura-app/cpp/panels/navigator_panel.h:21 |
| Nearest Neighbor | cs6 | Fastest resampling kernel, replicating pixels and producing jagged edges. Intended for non-antialiased edges (`ResampleMethod::Nearest`). | docs/04-image-ops/image-size.md:59 |
| NewDocumentDialog | code | Modal New Document dialog collecting name, size, colour mode, depth, and background into a `NewDocumentSpec`. | crates/pictura-app/cpp/new_document_dialog.h:22 |
| NewDocumentSpec | code | Values collected by the New Document dialog: name, width, height, mode ("rgb" or "grayscale"), depth (only 8), and background ("white" or "transparent"). | crates/pictura-app/cpp/new_document_dialog.h:13 |
| next_layer_name | code | `Prefix N`, where N is one more than the highest existing `Prefix <number>` name in the tree, falling back to 1. | crates/pictura-render/src/document_ops/layer_ops.rs:94 |
| No faithful equivalent | process | No-equivalent classification: the comparison tool cannot reproduce the behavior, so it is documented rather than forced into a tolerance. | docs/dev/m6-filters.md:107 |
| No-equivalent | process | Parity classification for an operation with no faithful external operator; the measured delta is recorded and property or known-value tests are used instead. | docs/dev/m11-distort2.md:87 |
| NodeKind | code | Document-model enum of layer kinds: pixel, adjustment, fill, text, shape, smart object, group, video, 3D, and background. | docs/05-layers/layers-overview.md:195 |
| Noise family | code | Add Noise, Median, and Despeckle, where Add Noise is the only randomized filter and takes a seed for bit-repeatable output. | crates/pictura-filters/src/noise.rs:1 |
| Noise filters | cs6 | Filter Noise submenu (add noise, despeckle, dust and scratches, median, reduce noise); all are 16-bit capable but only Add Noise is 32-bit. | docs/06-filters/noise-filters.md:13 |
| noise-filters | capability | Add Noise, Median, and Despeckle, with alpha preservation, seeded RNG determinism, clamp-to-edge handling, and an oracle or no-equivalent classification (`seed`). | openspec/specs/imaging/noise-filters/spec.md:6 |
| NoiseDistribution | code | Add Noise distribution: uniform or Gaussian. | crates/pictura-filters/src/lib.rs:54 |
| Non-destructive | cs6 | Editing that preserves original pixel data, as with smart objects, smart filters, and adjustment layers. | docs/05-layers/layers-overview.md:34 |
| Non-destructive crop | cs6 | Crop mode with delete cropped pixels off, retaining hidden pixels so the crop can be re-edited. Unavailable for a background-only document. | docs/03-tools/crop-tool.md:44 |
| Non-goal | project/process | A behavior explicitly excluded from parity, usually because it is not a CS6 feature or is prohibitive on Linux; each exclusion is documented. | docs/00-overview/feasibility-and-non-goals.md:70 ; docs/README.md:16 |
| Non-goal (Linux) | project | Documented parity exclusion, such as Adobe Bridge, Mini Bridge, `.8bf` plug-in binaries, or native print-driver parity, replaced by a native equivalent. | docs/02-ui-ux/panels/timeline-panel.md:5 ; docs/09-automation/mini-bridge.md:5 |
| Non-linear history | cs6 | History toggle that appends new states instead of discarding states after the selected one. | docs/01-architecture/undo-history.md:29 |
| Normal | cs6 | Default blend mode copying the source color; labeled Threshold in bitmap and indexed modes. | docs/05-layers/blend-modes.md:31 |
| Notes panel | cs6 | Dock that edits and navigates non-printing note annotations attached to the image. | docs/02-ui-ux/panels/note-panel.md:13 |
| Notifier | cs6 | Event binding letting a script or action run when an application event fires; mapped through the Script Events Manager. | docs/09-automation/script-events-and-jsx.md:57 |
| objectName | code | Unique, stable Qt dock name registered before layout save or restore; used by workspace persistence and rejects duplicates (`saveState()`, `restoreState()`). | openspec/specs/document/workspace-persistence/spec.md:6 |
| Offset | cs6 | Other filter translating the selection and filling the exposed area with the background color, a wrap, or repeated edge pixels. | docs/06-filters/other-filters.md:21 |
| Oil Paint | code/cs6 | CS6-new GPU-only filter producing a painterly edge-aware stroke look with brush and lighting controls; no CS6 CPU fallback (`OilPaintParams`). | crates/pictura-filters/src/oil_paint.rs:1 ; docs/06-filters/oil-paint.md:6 |
| oil-paint-filter | capability | Oil Paint filter with its error contract, alpha preservation, parameter validation, and determinism. | openspec/specs/imaging/oil-paint-filter/spec.md:6 |
| Opacity | cs6 | Paint control capping the transparency of color applied in one stroke. No back-and-forth within a held stroke exceeds the set value. | docs/03-tools/brush-and-pencil.md:32 |
| Opacity (brush) | cs6 | Per-stroke transparency cap; repeated passes within one stroke do not exceed the set level until the mouse button is released. | docs/07-color-painting/airbrush-and-flow.md:18 |
| Kooka Pictura | project | Name of this independent Rust and Qt6 reimplementation of Photoshop CS6. | docs/00-overview/product-overview.md:9 |
| OpenSpec | process | Spec-first workflow used by the repo: work becomes a reviewable change proposal before code, and the specs remain the contract. | openspec/changes/m39-panel-anatomy/proposal.md:1 |
| OpenSpec change | process | Proposal directory under openspec/changes/<kebab-name>/ holding proposal.md, design.md, tasks.md, and spec deltas, archived once complete. | AGENTS.md:39 |
| OpenSpec validation | process | The `openspec validate --all --strict` gate checking change proposals and canonical specs before commit. | docs/dev/STATE.md:33 |
| OpsError | code | Error returned by image operations for unsupported buffers or invalid parameters instead of panicking (`Unsupported`, `InvalidParams`). | crates/pictura-ops/src/lib.rs:22 ; openspec/specs/document/canvas-operations/spec.md:6 |
| OpStatus | code | The `#[repr(C)]` status-code enum crossing the Rust to Qt/C plug-in boundary; codes are stable and append-only (`OP_ERR_PANIC`, `OP_ERR_SCRATCH`). | docs/11-cross-cutting/error-handling.md:78 |
| Options bar | cs6 | Context-sensitive toolbar below the menu bar, rebuilt for the active tool and hosting the workspace switcher. | docs/02-ui-ux/toolbox-and-options-bar.md:106 |
| OptionsBar | code | Context-sensitive toolbar with one stacked page per tool, switched by the frame when the active tool changes (`showTool`, `buildPage`). | crates/pictura-app/cpp/options_bar.h:15 |
| oracle | process | Independent reference used to check output: ImageMagick for filters, psd-tools for PSD layer I/O, W3C formulas for blends, and the CPU compositor for the GPU path. | docs/dev/m2.5-app-and-gpu.md:4 ; openspec/specs/compositing/blend-modes/spec.md:74 |
| Orchestrator | process | Agent role that writes the brief, dispatches sub-agents, integrates their work, verifies independently, and commits a milestone. | docs/dev/STATE.md:853 |
| Other family | code | Maximum, Minimum, Offset, High Pass, and Custom, where maximum and minimum are separable morphology and custom is a 5 by 5 convolution. | crates/pictura-filters/src/other.rs:1 |
| Other filters | cs6 | Filter Other submenu containing custom, high pass, maximum, minimum, offset, and the optional HSB/HSL plug-in. | docs/06-filters/other-filters.md:13 |
| other-filters | capability | Maximum, Minimum, Offset, High Pass, and Custom, with edge handling, tiny-image safety, determinism, and an oracle or no-equivalent classification. | openspec/specs/imaging/other-filters/spec.md:6 |
| Overlay | cs6 | Contrast blend mode equal to `HardLight(source, base)`; the inverse of hard light. | docs/05-layers/blend-modes.md:167 |
| OVR-001 | project | Spec ID for Product Overview, which frames the whole effort. | docs/00-overview/product-overview.md:3 |
| OVR-002 | project | Spec ID for CS6 Editions and Constraints, the edition and version baseline. | docs/00-overview/cs6-editions-and-constraints.md:3 |
| OVR-003 | project | Spec ID for Feasibility and Non-Goals. | docs/00-overview/feasibility-and-non-goals.md:3 |
| OVR-004 | project | Spec ID for Licensing and Provenance. | docs/00-overview/licensing-and-provenance.md:3 |
| Paint Bucket | cs6 | Tool filling adjacent pixels similar in color to the clicked pixel, with the foreground color or a pattern (`BucketEngine`). Tolerance is documented from 0 to 255. | docs/03-tools/gradient-and-paint-bucket.md:52 |
| paint stroke | code | Live painting session in the Rust object: `begin_paint` starts it, `paint_dab` samples the pointer, `end_paint` commits one history state, and `cancel_paint` drops it. | crates/pictura-app/src/cxxqt_object.rs:560 |
| paint-engine | capability | Standard brush tip coverage, stroke spacing, flow and opacity accumulation, the pencil aliased edge and auto erase, paint modes, and per-stroke commit and undo. | openspec/specs/tools/paint-engine/spec.md:6 |
| paint_api::Stroke | code | Tool-lifetime paint object in `paint_api`: constructed on press, `finish()`/`cancel()` on release or Esc, destructor closes exactly once; a mid-stroke present queues `flush_present` and never calls `image()`. | docs/dev/refactor-view-seam.md:53 |
| paint_stroke | code | Feed a whole sample list into a destructive stroke and commit it on success, returning the dirty rectangle. | crates/pictura-paint/src/stroke.rs:237 |
| PaintError | code | Error from starting a stroke: empty document or no raster layer to paint on (`EmptyDocument`, `NoRasterLayer`). | crates/pictura-paint/src/stroke.rs:11 |
| PaintMode | code | Per-pixel compositing mode for a stroke: normal, dissolve, behind, or clear. | crates/pictura-paint/src/lib.rs:16 |
| Palette | cs6 | Indexed-color quantization choice. Options include exact, system, web, uniform, and the local and master perceptual, selective, and adaptive families. | docs/04-image-ops/indexed-color.md:35 |
| Panel | cs6 | Dockable module with a tab, a title bar, and a panel menu at its upper right. | docs/02-ui-ux/workspace-and-docks.md:18 |
| Panel anatomy | project | The M39 work defining the Layers panel tree projection, row delegate, multi-selection, and menus; it excludes drag-reorder. | docs/dev/m39-panel-anatomy.md:1 |
| Panel group | cs6 | Several panels sharing one title bar and switched by their tabs. | docs/02-ui-ux/workspace-and-docks.md:18 |
| Panel rail | project | Vertical icon-only toolbar whose buttons toggle docks and track their visibility, sharing one path with the Window > Panels commands (`PanelRail`). | docs/dev/STATE.md:324 |
| panel-rail | capability | The panel set, the right panel icon rail, and rail and Window menu sharing panel toggles. | openspec/specs/ui/panel-rail/spec.md:6 |
| PanelRail | code | Narrow vertical icon rail whose buttons toggle panel docks; each entry carries a command id so the rail and Window menu share one toggle path (`addPanel`, `commandTriggered`). | crates/pictura-app/cpp/panels/panel_rail.h:11 |
| Paragraph panel | cs6 | Dock for column and paragraph formatting such as alignment, justification, indents, and hyphenation. | docs/02-ui-ux/panels/character-and-paragraph.md:49 |
| Paragraph Styles panel | cs6 | New in CS6; stores character and paragraph attribute styles, with a built-in Basic Paragraph style. | docs/02-ui-ux/panels/character-and-paragraph.md:65 |
| Paragraph type | cs6 | Type that wraps inside a bounding box and reflows when the box is resized, rotated, or skewed. Overflow is marked with a plus in the box handle. | docs/03-tools/type-tools.md:18 |
| parent_path | code | The path of `path`'s containing node: `2/1` becomes `Some("2")`, `0` becomes `None` (its container is the document). Malformed paths return `None`. | crates/pictura-render/src/document_ops/layer_ops.rs:206 |
| parity | process | Matching the GPU path to the CPU oracle within one LSB per channel, across blend modes, masks, opacity, and group semantics. | openspec/specs/compositing/gpu-compositing/spec.md:6 |
| Parity acceptance criteria | process | Testable statements of the form "given X, doing Y produces Z within tolerance T" that each spec must supply. | docs/SPEC_TEMPLATE.md:72 |
| Parity target | process | Stated goal of matching Photoshop CS6 Standard and Extended behavior on Linux. | docs/README.md:13 |
| Parity tier | project/process | Per-spec classification of intended fidelity, for example core, extended-only, or non-goal (Linux), declared in each spec's header. | docs/00-overview/cs6-editions-and-constraints.md:5 ; docs/07-color-painting/color-models.md:5 ; docs/SPEC_TEMPLATE.md:13 |
| parse_luni | code | Decode a `luni` tagged block into a String, trimming a trailing null and lossily decoding UTF-16. | crates/pictura-codec/src/lib.rs:614 |
| Pass Through | cs6 | Default group blend mode, meaning the group has no blending properties of its own and its children blend directly against the parent backdrop. | docs/01-architecture/document-model.md:114 ; docs/05-layers/layer-groups.md:42 |
| pass-through group | domain | Group whose children composite directly onto the backdrop and blend in place, rather than in isolation (`PassThrough`). | openspec/specs/compositing/layer-compositing/spec.md:95 |
| Patch tool | cs6 | Tool repairing a selected area with pixels from another area or pattern. In CS6 it gains a content-aware mode and shares the healing blend math. | docs/03-tools/content-aware-move-and-patch.md:40 |
| patch_composite_region | code | Copies a region of a 4-plane rendered buffer into the cached document composite at (x0, y0), keeping the composite consistent with a region blit (`patch_buffer_region`). | crates/pictura-app/src/cxxqt_object.rs:3324 |
| PatchMatch | project | Randomized approximate nearest-neighbor patch-matching algorithm, the strongest public candidate for Adobe's Content-Aware Fill. It iterates propagation and random search over a nearest-neighbor field. | docs/03-tools/content-aware-move-and-patch.md:116 |
| Path | code/cs6 | Slash-separated sequence of non-negative indices addressing a node in the bottom-first layer tree; positional, so any structural change invalidates it (`layer_row_path`). | docs/02-ui-ux/panels/paths-panel.md:17 ; docs/dev/m39-panel-anatomy.md:115 |
| Path grammar | project | Frozen syntax of a layer path: segments with no leading zeros, no root path, and malformed paths resolving to not found rather than panicking. | docs/dev/m39-panel-anatomy.md:113 |
| Path Operations | cs6 | CS6 drop-down menu of path boolean operations: combine shapes, subtract, intersect, exclude overlapping, and merge shape components (`pictura_vector::boolean`). | docs/03-tools/path-selection-tools.md:30 |
| Paths panel | cs6 | Dock listing saved paths, the work path, and the current vector mask. | docs/02-ui-ux/panels/paths-panel.md:13 |
| Pattern | cs6 | Image repeated or tiled when used to fill a layer or selection; applied via pattern stamp, paint bucket, Edit > Fill, fill layer, or overlay. | docs/07-color-painting/pattern-presets.md:14 |
| PDF | format | Generic PDF opens through the Import PDF dialog with page or image selection, crop-to boxes, and resolution, mode, and bit depth. | docs/10-workflow-io/save-and-save-as.md:122 |
| PDF/X | format | Prepress PDF standard presets (X-1a, X-3, X-4) with live transparency in X-4; saved through the Save Adobe PDF dialog. | docs/10-workflow-io/save-and-save-as.md:143 |
| Pen tool | cs6 | Tool drawing vector paths: a click creates a corner point and a drag creates a smooth point with direction lines. Paths are work paths until saved. | docs/03-tools/pen-and-path-tools.md:17 |
| Perspective Crop | cs6 | New in CS6 tool correcting keystone distortion while cropping, by matching four corners to a rectangle under a projective transform. | docs/03-tools/perspective-crop.md:11 |
| Photo Filter | cs6 | Adjustment simulating a colored filter on the lens, using warming and cooling presets or a custom color. Density sets strength and preserve luminosity prevents darkening. | docs/04-image-ops/adjustments/photo-filter.md:15 |
| PhotoFilterParams | code | Photo Filter colour, density, and preserve-luminosity flag. | crates/pictura-adjust/src/lib.rs:74 |
| Photoshop PDF | format | Photoshop-saved PDF with Preserve Photoshop Editing Capabilities; can hold a single image and preserves layers, alpha, and spot color. | docs/10-workflow-io/save-and-save-as.md:148 |
| PICA | cs6 | Plug-in Component Architecture, the string caller/selector model used by automation and legacy plug-ins (`AutoPluginMain`, `SPMessageData`). | docs/09-automation/plugin-sdk.md:63 |
| Pictura Raw | project | Kooka Pictura's built-in reimplementation of the Camera Raw Filter: the 11 PV2012 Basic controls, rendered on the CPU and baked into a smart-object proxy. Stored on disk as Adobe's camera-raw smart filter (`filterID 2683`) for Photoshop compatibility; named descriptively rather than after Adobe's mark. | docs/dev/STATE.md:105 ; NOTICE.md |
| pictura-adjust | code | Crate holding 15 destructive adjustments applied in place to planar channel data. | docs/dev/STATE.md:43 |
| pictura-app | code | Crate holding the cxx-qt PictureView QObject and the Qt C++ shell: command registry, main window, theme, session, docks, and tools. | docs/dev/STATE.md:50 |
| pictura-codec | code | Crate for PSD/PSB read and write, covering composite, layers, masks, adjustment keys, and document channels. | docs/dev/STATE.md:41 |
| pictura-color | code | Crate for ICC profiles and color conversion, assignment, intents, and black point compensation. | docs/dev/STATE.md:42 |
| pictura-core | code | Dependency-free core crate for document, layer, channel, mask, blend mode, and adjustment data. | docs/dev/STATE.md:40 |
| pictura-diff | code | Verification-harness CLI reading two raw 8-bit files, printing sample, differing, max-delta, and mean-delta metrics, and exiting non-zero on any difference above tolerance. | crates/pictura-testkit/src/bin/pictura-diff.rs:1 ; openspec/specs/verification/verification-harness/spec.md:38 |
| pictura-filters | code | Crate holding the CPU filter families and their Filter variants: blur, sharpen, noise, stylize, distort, render, artistic, and the remaining families. | docs/dev/STATE.md:44 |
| pictura-ops | code | Crate for image resize, canvas size, and orientation operations, using ImageMagick as the oracle. | docs/dev/STATE.md:46 |
| pictura-paint | code | Crate for the dab-splatting brush and pencil stroke engine, depending only on pictura-core. | docs/dev/STATE.md:49 |
| pictura-render | code | Crate for the CPU and GPU compositors, the GPU filter path, PSD adjustment encode/decode, and document operations. | docs/dev/STATE.md:47 |
| pictura-select | code | Crate for the selection coverage mask, boolean and modify operations, wand, color range, and shape rasterizers. | docs/dev/STATE.md:45 |
| pictura-testkit | code | Verification crate with golden compare and hash helpers plus the pictura-diff CLI. | docs/dev/STATE.md:48 |
| PicturaMainWindow | code | CS6-shaped application frame: menu bar, tabbed document area, status bar, and dock areas; it owns the UI and each open document's `PictureView` (`addDocument`, `activeView`). | docs/dev/STATE.md:50 ; crates/pictura-app/cpp/frame.h:38 |
| PictureView | code | Rust-defined QObject in namespace `pictura` owning one document's image, layers, selection, history, and GPU state; the shell talks to it through invokables and signals. | docs/dev/STATE.md:50 ; openspec/specs/tools/selection-masked-edits/spec.md:6 ; crates/pictura-app/src/cxxqt_object.rs:32 |
| PictureViewRust | code | Backing Rust state for `PictureView`: display image, optional document and selection, history, file path, dirty flag, interop state, pending lasso, active stroke, and move-preview fields. | crates/pictura-app/src/cxxqt_object.rs:727 |
| PiPL | format/cs6 | Plug-in property list resource identifying an Adobe plug-in's kind, name, category, and entry point before code runs (kind, ivrs, expt). | docs/01-architecture/plugin-and-scripting-abi.md:13 ; docs/09-automation/plugin-sdk.md:86 |
| Pixel layer | cs6 | Raster layer holding pixels, supporting painting, filters, masks, styles, blend mode, opacity, and fill. | docs/05-layers/layers-overview.md:29 |
| Pixelate family | code | Block and cell filters: mosaic, facet, fragment, mezzotint, crystallize, pointillize, and color halftone. | crates/pictura-filters/src/pixelate/mod.rs:1 |
| Pixelate filters | cs6 | Filter Pixelate submenu of cell-based effects; it is not a Filter Gallery category and all of its filters are 8-bit only. | docs/06-filters/pixelate-filters.md:13 |
| pixelate-filters | capability | Mosaic, Crystallize, Facet, Fragment, Mezzotint, Pointillize, and Color Halftone, with parameter validation, edge handling, determinism, and an oracle classification. | openspec/specs/imaging/pixelate-filters/spec.md:6 |
| PixelBuffer | code | Planar, row-major, 8-bit-per-channel pixel buffer where `data.len() == width * height * channels`. Planar means channel c for the whole image comes first, matching the PSD image-data layout. | crates/pictura-core/src/lib.rs:49 ; docs/dev/STATE.md:47 ; openspec/specs/compositing/gpu-compositing/spec.md:6 |
| PixelBuffer::channels | code | Number of channels stored (composite color planes, or extra alpha channels). | crates/pictura-core/src/lib.rs:52 |
| PixelBuffer::data | code | The planar sample bytes. | crates/pictura-core/src/lib.rs:53 |
| PixelBuffer::new | code | Allocate a zeroed buffer of the given width, height, and channel count. | crates/pictura-core/src/lib.rs:57 |
| PixelBuffer::pixel_count | code | Number of pixels (not samples). | crates/pictura-core/src/lib.rs:67 |
| Placed Layer Data (SoLd) | format | PSD additional-layer block (CS3 and later) carrying an embedded smart object's source bytes, transform, warp, and page selection; SoLE is the CC 2015 variant. | docs/05-layers/smart-objects.md:135 |
| Placeholder panel | project | Shared dock showing a centered empty-state label for panels whose real content is not implemented yet (`PlaceholderPanel`). | docs/dev/STATE.md:321 |
| PlaceholderPanel | code | Dockable panel with a CS6-style empty state, used for panels whose contents are not implemented yet (`gradientsPanel_`, `channelsPanel_`). | crates/pictura-app/cpp/panels/placeholder_panel.h:10 |
| Plug-in SDK | cs6 | Adobe's closed C/C++ interface for extending Photoshop with native plug-ins; it is not reusable on Linux. | docs/01-architecture/plugin-and-scripting-abi.md:13 |
| Plug-in SDK (.8bf) | cs6 | Closed in-process C/C++ plug-in ABI; Windows extensions include `.8bf`, `.8ba`, `.8be`, `.8bi`, and `.8li`. | docs/09-automation/plugin-sdk.md:11 |
| PNG | format | Lossless web format with an interlace option; PNG-8 and PNG-24 are also Save for Web choices. | docs/10-workflow-io/save-and-save-as.md:102 |
| Point operation | project | Shared model for tonal and color adjustments: a pointwise map on channel samples, optionally after a color-space transform. It can be evaluated directly or through a LUT (`apply_lut`, `apply_vector`). | docs/04-image-ops/adjustments-overview.md:158 |
| Point type | cs6 | Type entered by clicking in the image, forming independent lines that expand as edited without wrapping. The I-beam cross-line marks the baseline. | docs/03-tools/type-tools.md:17 |
| PolarKind | code | Polar Coordinates direction: rectangular to polar or polar to rectangular. | crates/pictura-filters/src/lib.rs:95 |
| ponytail comment | process | In-code marker naming a deliberate simplification and its upgrade path, used instead of building the full solution. | docs/dev/m14-undo-history.md:14 |
| Porter-Duff over | project | Standard source-over compositing step used after the blend function is applied to color. | docs/01-architecture/gpu-rendering-pipeline.md:128 |
| Posterize | cs6 | Adjustment quantizing each channel to a fixed number of tonal levels, mapping pixels to the nearest level. Levels range from 2 to 255 (`PosterizeOp`). | docs/04-image-ops/adjustments/posterize.md:15 |
| ppi | cs6 | Pixels per inch, the unit of image resolution in the Image Size dialog. Pixel dimensions equal document size times resolution (`Resolution`). | docs/04-image-ops/image-size.md:20 |
| Preference file (Prefs.psp) | format | Version-specific binary store written on quit, holding memory, GPU, cursors, units, file handling, and type engine settings. | docs/11-cross-cutting/preference-storage.md:15 |
| Premultiplied alpha | project | Alpha representation used for compositing; the file boundary uses the documented PSD conventions. | docs/01-architecture/gpu-rendering-pipeline.md:126 |
| Present and zoom cache | project | ImageView's cached scaled image, keyed on the source cache key and zoom, invalidated when either changes. | docs/dev/m32-interactive-canvas.md:113 |
| PresentCache | code | Internal `ImageView` cache of the scaled presentation image keyed by source and zoom, invalidated when the image or view changes (`cachedScaled`). | crates/pictura-app/cpp/image_view.h:107 |
| Preserve Luminosity | cs6 | Option in Color Balance and Photo Filter keeping per-pixel luminance unchanged while shifting color. It avoids the darkening a physical filter would cause. | docs/04-image-ops/adjustments/color-balance.md:37 |
| Preset Manager | cs6 | Dialog managing the current set of brushes, swatches, gradients, styles, patterns, contours, custom shapes, and tool presets. | docs/10-workflow-io/presets-manager.md:16 |
| Print dialog | cs6 | Single CS6 dialog covering printer and job, preview, position and size, color management, output marks, and PostScript options; Print One Copy skips it. | docs/10-workflow-io/printing.md:17 |
| Profile | code | ICC profile handle backed by Little CMS 2, built from primaries rather than bundled Adobe files (`srgb`, `adobe_rgb`, `pro_photo`). | crates/pictura-color/src/lib.rs:46 |
| Profile::from_icc | code | Parse an ICC profile from raw bytes, returning an error on malformed input. | crates/pictura-color/src/lib.rs:113 |
| Profile::srgb | code | sRGB IEC61966-2.1, the default working space. | crates/pictura-color/src/lib.rs:50 |
| Profile::to_icc | code | Serialize the profile back to ICC bytes. | crates/pictura-color/src/lib.rs:120 |
| Proof Setup | cs6 | View menu selection of the press condition to simulate; shared with the Print dialog's hard proofing and Print Colors toggle. | docs/10-workflow-io/printing.md:59 |
| Properties panel | cs6 | New in CS6; contextual dock showing settings for the current selection, including adjustment parameters, mask controls, and 3D element settings (`PropertiesPanel`). | docs/02-ui-ux/panels/properties-panel.md:13 ; docs/04-image-ops/adjustments-overview.md:132 |
| Property test | project | Test tier using `proptest` for round-trips, invariants, and algebra, run every PR with a fixed seed. | docs/11-cross-cutting/testing-strategy.md:94 |
| ProPhoto RGB | format | Very wide-gamut RGB working space available in Color Settings behind More Options. | docs/10-workflow-io/color-settings.md:145 |
| ProPhoto RGB (ROMM RGB) | code | ProPhoto RGB working-space profile at D50 with ROMM primaries and the ROMM tone curve. | crates/pictura-color/src/lib.rs:84 |
| Protect Tones | cs6 | Dodge and burn option minimizing clipping in highlights and shadows and trying to prevent hue shifting. Modeled as a soft limiter plus a luminance-only adjustment (`protect_tones`). | docs/03-tools/dodge-burn-sponge.md:27 |
| PSB | format | Photoshop large-document format (`.psb`), version 2, with 64-bit length fields and up to 300,000 px per dimension. | docs/01-architecture/rust-core-design.md:24 ; docs/10-workflow-io/save-and-save-as.md:59 |
| PSD | format | Photoshop's native layered format with a 2 GB ceiling; image resource blocks carry ICC, IPTC, EXIF, and XMP metadata. | docs/01-architecture/rust-core-design.md:24 ; docs/10-workflow-io/save-and-save-as.md:90 |
| PSD additional-layer blocks | format | Tagged blocks attached to a layer record: lspf (locks), lclr (color label), and iOpa (fill opacity), omitted at their defaults. | docs/dev/STATE.md:646 |
| psd-codec | capability | PSD and PSB signature and version detection, composite header validation, raw and RLE composite read, PSB wide lengths, composite write and round-trip, and typed errors (`read_psd`, `write_psd`, `PsdError`). | openspec/specs/codec/psd-codec/spec.md:6 |
| psd-layer-io | capability | Reading and writing the PSD layer-and-mask section, raw and PackBits channel data, luni Unicode names, lsct group markers, lock, colour, and fill attributes, round-tripped against the psd-tools oracle. | openspec/specs/codec/psd-layer-io/spec.md:6 |
| psd-tools | code | Python PSD library used as the oracle for layer I/O and the author of test documents such as two_layers.psd. | docs/dev/STATE.md:11 ; openspec/specs/document/document-model/spec.md:15 |
| PSD/PSB | domain | Photoshop document formats: version 1 uses 32-bit lengths and PSB version 2 uses 64-bit lengths and 4-byte RLE counts (8BPS). | openspec/specs/codec/psd-codec/spec.md:6 |
| PSD/PSB image resource | format | Tagged block in the PSD resource section; unknown blocks are preserved verbatim for lossless round-trip. | docs/01-architecture/file-formats.md:109 |
| PsdError | code | Codec error type: bad signature, unsupported, truncated, and invalid. Malformed input never panics, it returns one of these. | crates/pictura-codec/src/lib.rs:21 ; openspec/specs/codec/psd-codec/spec.md:6 |
| PsdRect | code | Signed rectangle in PSD coordinates. Bounds may fall outside the canvas, so all edges are i32 and right/bottom may be smaller than left/top. | crates/pictura-core/src/lib.rs:220 |
| PsdRect::height | code | Rectangle height as `bottom - top`, signed. | crates/pictura-core/src/lib.rs:232 |
| PsdRect::width | code | Rectangle width as `right - left`, signed. | crates/pictura-core/src/lib.rs:228 |
| PSNR | project | Peak signal-to-noise ratio, the primary metric for resampling, transform, and filter golden cases at 40 to 45 dB floors. | docs/11-cross-cutting/testing-strategy.md:74 |
| Puppet Warp | cs6 | Edit command laying a triangulated mesh over a layer and deforming it through movable pins. Options include mode, density, expansion, and rotate. | docs/03-tools/move-and-transform.md:33 |
| Q_DECLARE_METATYPE | code | Qt macro registering `pictura::ToolId` as a metatype so it can travel through queued signal/slot connections. | crates/pictura-app/cpp/tools.h:208 |
| Q_OBJECT | code | Qt macro that must appear in a class body to enable signals, slots, and meta-object features; the moc processes it via AUTOMOC. | crates/pictura-app/cpp/tools.h:124 |
| QAbstractItemModel | code | Qt model base used for Layers, Channels, Paths, and History panel views; it is not thread-safe. | docs/01-architecture/qt6-ui-design.md:126 |
| QDockWidget | code | Qt primitive chosen for each panel; a custom title bar is set with `setTitleBarWidget`. | docs/02-ui-ux/workspace-and-docks.md:226 |
| QMainWindow | code | Qt frame providing dock areas, tabified docking, and `saveState`/`restoreState` workspace persistence. | docs/01-architecture/qt6-ui-design.md:76 |
| qmetaobject-rs | code | Rust crate exposing Qt QML through QObject macros without C++. A fallback if cxx-qt does not fit. | docs/01-architecture/rust-qt-interop.md:39 |
| qobject | code | Rust module annotated with `#[cxx_qt::bridge]` declaring the generated QObject type, its signals, and its invokable methods. | crates/pictura-app/src/cxxqt_object.rs:16 |
| QQuickRhiItem | code | Qt Quick GPU canvas host required for zero-readback present; the renderer runs on the scene-graph thread. | docs/01-architecture/gpu-rendering-pipeline.md:275 |
| QRhi | code | Qt's Rendering Hardware Interface, Qt6's GPU abstraction and a candidate interop layer with Rust wgpu (`QRhiTexture`). | docs/01-architecture/system-architecture.md:172 |
| QRhiWidget | code | Widget-based GPU canvas host; it owns its own QRhi and cannot adopt a wgpu device. | docs/01-architecture/gpu-rendering-pipeline.md:272 |
| Qt Widgets shell | project | Decision to build the frame, docks, menus, and panels with Qt Widgets, using QML only for isolated surfaces. | docs/01-architecture/qt6-ui-design.md:68 |
| Qt6::Svg | code | Qt Svg module the app links so icons and cursors render through QSvgRenderer. | docs/dev/STATE.md:20 |
| Quality | code | Render quality level for Radial Blur: draft, good, or best. | crates/pictura-filters/src/lib.rs:47 |
| Quick Mask | cs6 | Mode turning the active selection into a temporary editable alpha channel shown as a rubylith overlay; exiting converts the mask back to a selection. | docs/02-ui-ux/panels/channels-panel.md:29 ; docs/03-tools/quick-mask-tool.md:15 ; docs/08-selection/quick-mask.md:13 |
| Quick Selection | cs6 | Brush-driven selection tool that grows outward and follows edges. Auto-enhance applies edge refinement similar to Refine Edge contrast and radius (`QuickSelectionEngine`). | docs/03-tools/quick-selection-and-magic-wand.md:15 |
| QuickJS | code | Embedded JavaScript engine chosen for the Kooka Pictura scripting host, via the `rquickjs` crate. | docs/01-architecture/plugin-and-scripting-abi.md:278 |
| QuickJS (rquickjs) | code | Proposed primary embedded scripting engine, chosen because existing scripts are JavaScript and it exposes memory, stack, timeout, and module controls (`set_memory_limit`, `set_interrupt_handler`). | docs/09-automation/rust-scripting-replacement.md:60 |
| Radial Blur | cs6 | Blur with spin (concentric) and zoom (radial) methods, a draft/good/best quality setting, and a movable blur center. | docs/06-filters/blur-filters.md:21 |
| RadialMethod | code | Radial Blur method: spin or zoom. | crates/pictura-filters/src/lib.rs:41 |
| Rasterize | cs6 | Command converting parametric content (type, shape, fill, vector mask, smart object) into pixels, irreversibly. | docs/05-layers/layer-management-ui.md:101 |
| RawLayer | code | One layer record before the tree is assembled. Channel id and declared data length are kept separate so channel image data can be read in a second pass (`channel_ids`, `channel_lens`). | crates/pictura-codec/src/lib.rs:320 |
| read_channel_data | code | Read one layer channel's image data. `declared_len` comes from the channel info and includes the 2-byte compression header. Raw and RLE are supported, ZIP (2/3) is unsupported. | crates/pictura-codec/src/lib.rs:643 |
| read_layer_info | code | Read layer records, then channel image data in a second pass, then build the tree. | crates/pictura-codec/src/lib.rs:375 |
| read_layer_record | code | Parse one layer record: rect, channel info, 8BIM blend signature and key, opacity, clipping, flags, then extra data with the mask block and tagged blocks. | crates/pictura-codec/src/lib.rs:436 |
| read_layer_section | code | Parse the Layer and Mask Information section in two lengths (PSD u32, PSB u64): layer info, then the global layer mask (skipped). A zero length means no layers. | crates/pictura-codec/src/lib.rs:327 |
| read_psd | code | Parse a PSD or PSB file into a Document holding the composite image and the layer tree, bottom-first matching PSD z-order. | crates/pictura-codec/src/lib.rs:131 ; openspec/specs/codec/psd-codec/spec.md:6 |
| Reader | code | Bounds-checked cursor over the file bytes. Every read yields `PsdError::Truncated` instead of an index panic (`take`, `remaining`). | crates/pictura-codec/src/lib.rs:68 |
| rebuild_display | code | Builds the full display image for the current state, sourcing the stroke's working document while a stroke is active and otherwise the authoritative planar composite. | crates/pictura-app/src/cxxqt_object.rs:3394 |
| recomposite | code | Full-document refresh path: re-renders the current document, stores the composite, converts it to the display image, and emits `changed`; the incremental alternative is `refresh_region`. | crates/pictura-app/src/cxxqt_object.rs:2576 |
| record | code | Snapshots the current document and selection, captures it under a label, and marks the document dirty; callers must recomposite first. | crates/pictura-app/src/cxxqt_object.rs:2353 |
| Reduce Noise | cs6 | Primarily CPU denoise filter with strength, preserve details, reduce color noise, sharpen details, JPEG artifact removal, and per-channel advanced mode. | docs/06-filters/noise-filters.md:21 |
| Reference point | cs6 | The fixed point of a transform, central by default and movable to any of nine positions or dragged in the canvas. Moving it changes translation without changing scale or rotation. | docs/03-tools/move-and-transform.md:29 |
| Refine Edge | cs6 | Dialog improving selection edges with edge detection and alpha matting; the CS6-recommended replacement for the old Extract plug-in. | docs/08-selection/refine-edge.md:13 |
| Refine Mask (Mask Edge) | cs6 | Edge-refinement pipeline opened from a mask's Mask Edge control, sharing the Refine Edge option set (radius, smooth, feather, contrast, shift, decontaminate). | docs/05-layers/layer-masks.md:108 |
| refresh_region | code | Incremental path that composites only a rectangle, patches the authoritative composite, and emits `region_blitted` with a rectangle-sized image without rebuilding the full image. | crates/pictura-app/src/cxxqt_object.rs:2505 |
| region_blitted | code | QSignal emitted after a region composite; the receiver blits the region image at (x, y) because the full image was not rebuilt. C++ name is `regionBlitted`. | crates/pictura-app/src/cxxqt_object.rs:42 |
| REGION_REFRESH_BUDGET | code | One megapixel cap that forced a full recomposite when a dirty union was large; removed in M35 when the C++ region blit replaced the per-pixel patch. | docs/dev/STATE.md:563 |
| Relative (Canvas Size) | cs6 | Canvas Size checkbox switching width and height from absolute values to deltas. Positive values add to the canvas, negative values subtract. | docs/04-image-ops/canvas-size.md:19 |
| Relative / Absolute | cs6 | Selective Color methods. Relative scales an existing ink amount by its percentage, while absolute adds the slider amount directly. | docs/04-image-ops/adjustments/selective-color.md:33 |
| Relative Colorimetric | cs6 | Rendering intent mapping white to white and the default for North America and Europe presets. | docs/10-workflow-io/color-settings.md:154 |
| Remove Ghosts | cs6 | Merge To HDR Pro option removing moving objects by taking their pixels from a chosen base exposure. The base is outlined in green. | docs/04-image-ops/32-bit-hdr.md:36 |
| rename_path | code | Rename the node at `path`. No refusal rule; returns false for a path that does not resolve. | crates/pictura-render/src/document_ops/layer_ops.rs:635 |
| Render family | code | Clouds, Difference Clouds, Fibers, and Lens Flare, all classified no-equivalent and verified by property tests rather than delta fitting. | crates/pictura-filters/src/render.rs:1 |
| Render filters | cs6 | Filter Render submenu (clouds, difference clouds, fibers, lens flare, lighting effects); Flame, Tree, and Picture Frame are CC 2014.2, not CS6. | docs/06-filters/render-filters.md:13 |
| render-filters | capability | Seeded deterministic render filters: Clouds, Difference Clouds, Fibers, and Lens Flare, with dispatch and a headless self-test. | openspec/specs/imaging/render-filters/spec.md:6 |
| render_gpu | code | Renders an offscreen wgpu gradient into a `QImage`; returns 0 with no Vulkan adapter, 1 when non-blank, and 2 when blank. | crates/pictura-app/src/cxxqt_object.rs:2388 |
| Rendering intent | cs6 | The ICC conversion mode: Perceptual, Saturation, Relative Colorimetric, or Absolute Colorimetric. The default follows the regional color setting. | docs/01-architecture/color-management.md:131 ; docs/04-image-ops/color-profiles-and-assignment.md:102 ; docs/10-workflow-io/color-settings.md:99 |
| Replace Color | cs6 | Command that replaces a color range using a range selection plus HSL sliders or a result color. It lacks Colorize and applies globally. | docs/04-image-ops/adjustments/replace-color.md:15 |
| Requirement | process | A normative SHALL or MUST statement in a capability spec; the format requires at least one scenario for each. | AGENTS.md:51 ; openspec/specs/tools/tool-framework/spec.md:6 |
| Resample | code | Image Size resampling kernel: Nearest, Bilinear, or Bicubic; Bicubic approximates Photoshop's unpublished Catmull-Rom coefficients. | crates/pictura-render/src/lib.rs:56 ; crates/pictura-ops/src/resize.rs:12 |
| Resample Image | cs6 | Image Size checkbox, on by default, that changes the amount of image data. When off, pixel dimensions are locked and resolution couples to document size. | docs/04-image-ops/image-size.md:37 |
| ResampleMethod | code | Rust enum of six interpolation methods: Nearest, Bilinear, Bicubic, BicubicSmoother, BicubicSharper, and BicubicAutomatic. Automatic resolves to a kernel from the resize direction. | docs/04-image-ops/image-size.md:147 |
| resize | code | Resizes an image to a new width and height with the chosen kernel, returning a new buffer and leaving the input untouched. | crates/pictura-ops/src/resize.rs:90 |
| resize_canvas | code | Grows or crops the canvas to a new size, placing the source per anchor and filling added pixels with a flat background while discarding cropped pixels. | crates/pictura-ops/src/canvas.rs:26 |
| resize_canvas_document | code | `Image > Canvas Size` at document scope. Offsets every layer rect and mask by the anchor offset, extends document channels, and recomputes the composite. | crates/pictura-render/src/document_ops/canvas.rs:8 |
| resize_document | code | Scales every layer rect, resamples its channel planes and mask from the old rect size to the new, then recomputes `doc.composite`. | crates/pictura-render/src/document_ops/resize.rs:7 |
| resolve_path | code | Resolves a path to a layer, or `None` for a malformed or out-of-range path. | crates/pictura-render/src/document_ops/layer_ops.rs:212 |
| resolve_path_mut | code | Mutable `resolve_path`. | crates/pictura-render/src/document_ops/layer_ops.rs:225 |
| Rgba | code | Straight 8-bit red, green, blue, and alpha colour used as paint and background colour. | crates/pictura-paint/src/lib.rs:8 |
| RippleSize | code | Ripple wave size: Small, Medium, or Large. | crates/pictura-filters/src/lib.rs:81 |
| rotate180 | code | Half-turn: (x, y) becomes (W-1-x, H-1-y). | crates/pictura-ops/src/orient.rs:46 |
| rotate90_ccw | code | 90 degrees counter-clockwise: (x, y) becomes (y, W-1-x), swapping the dimensions. | crates/pictura-ops/src/orient.rs:41 |
| rotate90_cw | code | 90 degrees clockwise: (x, y) becomes (H-1-y, x), swapping the dimensions. | crates/pictura-ops/src/orient.rs:36 |
| rotate_arbitrary | code | Rotates about the canvas centre by angle_deg, positive clockwise, growing the canvas to the axis-aligned bounding box and bilinearly sampling with clamp-to-edge. | crates/pictura-ops/src/orient.rs:67 |
| rotate_document | code | Rotates the whole document by 1 (90 degrees CW), 2 (180 degrees), or 3 (270 degrees CW) quarter turns. Any other value is `InvalidParams` and leaves the document untouched. | crates/pictura-render/src/document_ops/orient.rs:41 |
| Rubylith | cs6 | Red overlay used to display a mask; toggled by modifier-clicking a channel thumbnail. | docs/02-ui-ux/panels/channels-panel.md:33 |
| Ruler | cs6 | Tool that measures distances, locations, and angles, and can create a protractor. Its Straighten option feeds the correction angle into Arbitrary rotation. | docs/03-tools/eyedropper-color-sampler-ruler.md:49 |
| Sampled brush tip | cs6 | A tip created from an image selection via Define Brush Preset, up to 2500 x 2500 px, converted to grayscale with baked edge softness. | docs/07-color-painting/brush-engine.md:38 |
| sanitized | code | `StrokeConfig::sanitized` clamps every stroke field to its legal range: diameter 1 to 5000, hardness, opacity, flow, and roundness 0 to 100, angle -180 to 180. | crates/pictura-paint/src/lib.rs:50 |
| Save For Web | cs6 | Export dialog that saves slices as separate files and generates the HTML or CSS to display them. It offers per-slice optimization and linked slices sharing a palette. | docs/03-tools/slice-tools.md:100 |
| Scale Styles | cs6 | Image Size option that scales layer-style effects, such as blur radius and stroke width, by the same width/height factor. Available only with Constrain Proportions. | docs/04-image-ops/image-size.md:35 |
| Scenario | process | The required example clause under an OpenSpec requirement, introduced by exactly four hashes, that makes the requirement testable. | AGENTS.md:51 ; openspec/specs/tools/tool-framework/spec.md:14 |
| scenario tag | process | A bracketed identifier appended to a scenario name, such as [m39_tree], linking the scenario to a self-test check. | openspec/changes/m39-panel-anatomy/specs/layers-panel/spec.md:14 |
| Scratch disk | cs6 | Disk space Photoshop spills working data, undo, and history to when RAM is insufficient; up to four volumes, first-listed used first. | docs/01-architecture/system-architecture.md:43 ; docs/10-workflow-io/scratch-disks-and-memory.md:33 |
| Screen mode | cs6 | One of Standard, Full Screen With Menu Bar, or Full Screen; cycled with `F` and `Shift+F`. | docs/02-ui-ux/application-frame.md:129 |
| ScreenMode | code | Frame enum for the CS6 screen modes: `Standard`, `FullWithMenuBar`, and `Full`; cycled by the toolbox toggle. | crates/pictura-app/cpp/frame.h:42 |
| Script Events Manager | cs6 | Dialog mapping an application event to a script or action; the scripting equivalent is `app.notifiersEnabled` and `app.notifiers`. | docs/09-automation/script-events-and-jsx.md:57 |
| Scripted Patterns | cs6 | A CS6-new Edit > Fill option that fills with a pattern using one of five geometric scripts (Brick Fill, Cross Weave, Random Fill, Spiral, Symmetry Fill). | docs/06-filters/render-filters.md:25 ; docs/07-color-painting/pattern-presets.md:57 |
| ScriptUI | cs6 | The ExtendScript dialog and control toolkit; Kooka Pictura does not reproduce it in full. | docs/01-architecture/plugin-and-scripting-abi.md:68 |
| Scrubby Zoom | cs6 | Zoom tool option where a horizontal drag zooms out to the left and in to the right. It maps drag distance to a logarithmic scale delta. | docs/03-tools/hand-and-zoom.md:45 |
| SECTION_CLOSED_FOLDER | format | `lsct` section type 2: a closed folder record that closes a group. | crates/pictura-codec/src/lib.rs:45 |
| SECTION_DIVIDER | format | `lsct` section type 3: a bounding section divider that opens a group. | crates/pictura-codec/src/lib.rs:43 |
| SECTION_OPEN_FOLDER | format | `lsct` section type 1: an open folder record that closes a group. | crates/pictura-codec/src/lib.rs:44 |
| seed | domain | A fixed random seed making noise, texture, and render filters reproducible byte for byte. | openspec/specs/imaging/noise-filters/spec.md:6 |
| SelectError | code | Error returned by selection operations for size mismatches or invalid parameters. | crates/pictura-select/src/lib.rs:16 |
| Selection | code/cs6 | A document-sized coverage mask isolating part of an image so edits affect only those pixels; at most one active selection per document. | crates/pictura-select/src/lib.rs:25 ; docs/01-architecture/document-model.md:31 ; docs/08-selection/selection-model.md:13 |
| Selection mode | cs6 | The four boolean modes for combining a new selection with the existing one: New, Add To, Subtract From, and Intersect With. Some tools omit Intersect. | docs/03-tools/marquee-selection.md:19 |
| Selection operations | cs6 | The four modes New, Add, Subtract, and Intersect implemented as alpha-compositing max, subtract, and multiply on coverage masks. | docs/08-selection/selection-model.md:29 |
| selection-channels | capability | Document-level extra channels that round-trip through PSD, channel validation, selection to and from channel, and a PSD alpha channel visible to psd-tools. | openspec/specs/tools/selection-channels/spec.md:6 |
| selection-masked-edits | capability | PictureView selection state, a selection masking an adjustment layer, selection UI controls, and a masked adjustment self-test. | openspec/specs/tools/selection-masked-edits/spec.md:6 |
| selection-model | capability | The selection coverage mask representation, boolean combine algebra, invert, none and all, dimension-mismatch errors, feather, expand, contract, border, and smooth, with determinism and an oracle. | openspec/specs/tools/selection-model/spec.md:6 |
| selection-tools | capability | Magic Wand, Grow, Similar, Color Range, and saving and loading a selection to an alpha channel. | openspec/specs/tools/selection-tools/spec.md:6 |
| Selection::all | code | A selection with full 255 coverage everywhere. | crates/pictura-select/src/lib.rs:74 |
| Selection::border | code | Band centred on the current edge (roughly half in and half out). | crates/pictura-select/src/lib.rs:280 |
| Selection::combine | code | Combines `other` into `self` with `op` (same dimensions required). | crates/pictura-select/src/lib.rs:187 |
| Selection::combine_with | code | Combines `other` into `self` with `mode` in place. Same-size masks merge in place; a mismatched `other` can only replace (`New`), since the boolean ops have no shared canvas. | crates/pictura-select/src/lib.rs:226 |
| Selection::contract | code | Erodes the coverage. radius 0 is identity, clamped to 1..=100. | crates/pictura-select/src/lib.rs:271 |
| Selection::ellipse | code | The ellipse inscribed in the `w`x`h` rectangle at `(x, y)`; a pixel is inside when its centre is. Clipped to the canvas. | crates/pictura-select/src/lib.rs:105 |
| Selection::expand | code | Dilates the coverage. radius 0 is identity, clamped to 1..=100. | crates/pictura-select/src/lib.rs:262 |
| Selection::feather | code | Blurs the coverage with a Gaussian. radius 0 is identity. | crates/pictura-select/src/lib.rs:249 |
| Selection::from_channel | code | Interprets a document-level channel as a selection. `width` and `height` are the document dimensions the channel must match. | crates/pictura-select/src/lib.rs:170 |
| Selection::invert | code | The complement of the coverage mask; maps each byte to 255 minus the value. | crates/pictura-select/src/lib.rs:238 |
| Selection::none | code | An empty selection with zero coverage everywhere. | crates/pictura-select/src/lib.rs:66 |
| Selection::polygon | code | Even-odd scanline fill of `points`, sampled at pixel centres and clipped to the canvas. Fewer than three points yields an empty selection. | crates/pictura-select/src/lib.rs:128 |
| Selection::rect | code | A `w`x`h` rectangle at `(x, y)`; bounds are half-open and clipped to the canvas. `w <= 0` or `h <= 0` yields an empty selection. | crates/pictura-select/src/lib.rs:84 |
| Selection::smooth | code | Median/majority smoothing over a square window. radius 0 is identity. | crates/pictura-select/src/lib.rs:302 |
| Selection::to_channel | code | Exports this selection as a document-level channel (8-bit grayscale coverage, one byte per pixel). | crates/pictura-select/src/lib.rs:161 |
| SelectionMode | code | Selection combine mode enum: `New`, `Add`, `Subtract`, or `Intersect`, stringified for the bridge. | crates/pictura-app/cpp/tools.h:96 |
| Selective Color | cs6 | Adjustment that changes CMYK ink amounts within one of nine color families, six hues plus Whites, Neutrals, and Blacks. The method is Relative or Absolute. | docs/04-image-ops/adjustments/selective-color.md:15 |
| SelectOp | code | How a new selection combines with the existing one. | crates/pictura-select/src/lib.rs:33 |
| self-test | process | A headless check mode that runs named steps and exits non-zero on failure, such as the m38_icons and m39 C++ self-test steps. | docs/dev/STATE.md:30 ; openspec/changes/archive/2026-09-16-m38-icon-cursor-library/proposal.md:18 |
| self-test harness | code | `--self-test` mode in `main` that isolates the session store, drives the frame and bridge through assertions, prints `pictura self-test:` lines, and exits non-zero on the first failure. | crates/pictura-app/cpp/main.cpp:61 |
| Session schema | project | The version number of the persisted XDG session store; schema v2 added the gpuCompute preference with a backward-compatible load. | docs/dev/STATE.md:402 |
| SessionState | code | Opaque UI session state persisted across restarts (window layout, theme level, GPU preference, layers thumb options, recent files); not document data. | crates/pictura-app/cpp/session.h:10 |
| set_blend_paths | code | Sets the blend mode. Skips the Background and fully-locked nodes; groups are eligible. Returns the number of nodes changed. | crates/pictura-render/src/document_ops/layer_ops.rs:425 |
| set_color_paths | code | Sets the color label. Skips the Background; fully-locked nodes and groups are eligible. | crates/pictura-render/src/document_ops/layer_ops.rs:498 |
| set_fill_paths | code | Sets fill opacity. Skips the Background, fully-locked nodes, and groups (a group has no Fill). | crates/pictura-render/src/document_ops/layer_ops.rs:461 |
| set_lock_paths | code | Sets (`on`) or clears a lock bit. Skips the Background; a fully-locked node is still eligible so a lock can be released. | crates/pictura-render/src/document_ops/layer_ops.rs:479 |
| set_opacity_paths | code | Sets opacity. Skips the Background and fully-locked nodes; groups are eligible. | crates/pictura-render/src/document_ops/layer_ops.rs:443 |
| set_visible_paths | code | Sets the eye on every listed path (the Background included). Returns the number of nodes changed. | crates/pictura-render/src/document_ops/layer_ops.rs:373 |
| Shadow/Highlight | cs6 | Local tonal correction that lightens or darkens based on the surrounding neighborhood, not just the pixel value. Advanced controls include Tonal Width, Radius, Midtone Contrast, and Color Correction. | docs/04-image-ops/adjustments/shadow-highlight.md:15 |
| Shake Reduction | cs6 | A CC-only blind-deconvolution sharpener that estimates a blur trace and deconvolves; it does not exist in CS6. | docs/06-filters/sharpening-tools.md:27 |
| SHALL/MUST | process | The normative keywords in requirements; SHALL marks required system behaviour and MUST marks an invariant that must hold. | openspec/specs/tools/tool-framework/spec.md:8 |
| Shape layer | cs6 | A fill layer plus a linked vector mask that defines the outline, both editable. Multiple shapes may live on one layer. | docs/03-tools/shape-tools.md:26 ; docs/05-layers/layers-overview.md:33 |
| shape-selection-tools | capability | Rectangular and elliptical marquee, lasso, quick selection, selection combine modes, and a selection bounds overlay. | openspec/specs/tools/shape-selection-tools/spec.md:6 |
| Sharpen family | code | Sharpen, Sharpen More, Sharpen Edges, and Unsharp Mask. | crates/pictura-filters/src/sharpen.rs:1 |
| Sharpen filters | cs6 | The Filter Sharpen submenu; Sharpen, Sharpen Edges, and Sharpen More are automatic with no dialog, while Unsharp Mask and Smart Sharpen expose controls. | docs/06-filters/sharpen-filters.md:13 |
| sharpen-filters | capability | Sharpen, Sharpen More, Sharpen Edges, and Unsharp Mask, with parameter validation and clamping, edge handling, determinism, and an oracle. | openspec/specs/imaging/sharpen-filters/spec.md:6 |
| ShearFill | code | Shear fill for uncovered areas: WrapAround or RepeatEdgePixels. | crates/pictura-filters/src/lib.rs:101 |
| similar | code | Adds all similar-colored pixels in the image to the selection. | crates/pictura-select/src/lib.rs:480 |
| Single-writer | project | The design in which all document mutations serialize through one logical writer, initially the GUI thread. | docs/01-architecture/threading-and-concurrency.md:69 |
| Sketch family | code | Edge and emboss-style renders plus paper-and-ink simulations such as Note Paper, Photocopy, Plaster, Reticulation, Stamp, Torn Edges, and Water Paper. | crates/pictura-filters/src/sketch/mod.rs:1 |
| Sketch filters | cs6 | A Filter Gallery family of 14 hand-drawn and textured effects; all are 8-bit only and many use the foreground and background colors. | docs/06-filters/sketch-filters.md:13 |
| sketch-filters | capability | The 14 Sketch filters, from Bas Relief to Water Paper, with foreground and background colour inputs, texture options, and the shared contract. | openspec/specs/imaging/sketch-filters/spec.md:6 |
| Slice | cs6 | A rectangular region dividing an image for web export, with per-region URLs and optimization. Slices are user, layer-based, auto, or subslice. | docs/03-tools/slice-tools.md:11 ; docs/10-workflow-io/web-export-and-slices.md:54 |
| Smart Blur | cs6 | An edge-aware blur that blurs only non-edge regions, with Normal, Edge Only, and Overlay Edge modes. | docs/06-filters/blur-filters.md:23 |
| Smart Filter | cs6 | A filter applied non-destructively to a Smart Object and listed below it in the Layers panel, where it can be reordered, hidden, or removed. | docs/01-architecture/document-model.md:25 ; docs/05-layers/smart-filters.md:13 |
| Smart filter stack | code | An ordered list of filter entries applied bottom-up to a Smart Object's rendered source, with a single shared filter mask applied after the whole stack. | docs/05-layers/smart-filters.md:89 |
| Smart Object | cs6 | A layer containing source image data from a raster or vector file that can be transformed, replaced, and filtered without altering the original content. | docs/01-architecture/document-model.md:25 ; docs/05-layers/smart-objects.md:13 ; docs/10-workflow-io/open-and-new.md:75 |
| Smart Radius | cs6 | Refine Edge option that varies the refinement radius between hard and soft edge regions. | docs/08-selection/refine-edge.md:22 |
| Smart Sharpen | cs6 | The recommended general sharpener, adding Remove (Gaussian, Lens, or Motion Blur) and Advanced Shadow and Highlight tabs that are 8- and 16-bit only. | docs/06-filters/sharpen-filters.md:18 |
| Snapshot | code/cs6 | A retained document state the History Brush can use as a paint source. A merged snapshot paints from the flattened document. | docs/02-ui-ux/panels/history-panel.md:30 ; docs/03-tools/history-brush.md:15 |
| Soft Light | cs6 | A diffused-light contrast blend mode whose CS6 formula differs from the W3C formula near the 0.25 threshold and is not fully confirmed. | docs/05-layers/blend-modes.md:172 |
| Soft proofing | cs6 | Simulating a target device with a document-to-proof transform plus a proof-to-monitor transform. | docs/01-architecture/color-management.md:55 |
| Solarize | cs6 | A no-dialog Stylize filter blending a negative and a positive image using a fixed tonal-inversion threshold curve. | docs/06-filters/stylize-filters.md:22 |
| Solo visibility | project | Alt-click an eye to show only that layer or group, restoring every row's prior visibility on the next toggle. | docs/dev/m39-panel-anatomy.md:262 |
| Spacing (brush) | cs6 | Distance between dabs as a percentage of brush diameter; when unchecked, pointer velocity determines spacing. | docs/07-color-painting/brush-engine.md:52 |
| SpacingMode | code | Dab spacing policy: a fixed percentage of diameter, or velocity-driven by segment length. | crates/pictura-paint/src/spacing.rs:4 |
| Spec delta (ADDED, MODIFIED, REMOVED) | process | The tags marking whether a change adds, modifies, or removes a requirement in a capability spec. | AGENTS.md:41 |
| Spec ID | project/process | A stable feature identifier of the form AREA-NNN, such as ARCH-001 or TOOL-014, used in the template heading and the traceability matrix. | docs/00-overview/product-overview.md:3 ; docs/SPEC_TEMPLATE.md:11 |
| Spec status | process | Per-spec lifecycle value: Stub, Draft, Spec'd, or Verified. | docs/README.md:34 |
| Spec template | process | The canonical per-feature spec structure whose headings and order every spec keeps so specs stay diffable. | docs/SPEC_TEMPLATE.md:1 |
| Spec workflow (OpenSpec) | process | The project process that makes every unit of work a reviewable OpenSpec change before code, with docs/ as the long-form contract. | docs/dev/STATE.md:838 |
| Specification corpus | project | The docs under `docs/` are the specification, not a program; every mapping section is a design proposal. | docs/00-overview/product-overview.md:9 |
| SpherizeMode | code | Spherize axis: Normal, HorizontalOnly, or VerticalOnly. | crates/pictura-filters/src/lib.rs:74 |
| split_planes | code | Splits the planar image-data section into the mode's color planes (the composite) and the trailing extra channels (saved selections / alpha). | crates/pictura-codec/src/lib.rs:210 |
| Sponge | cs6 | Tool that subtly changes color saturation, in Saturate or Desaturate mode. Its Vibrance option limits clipping at the saturation extremes. | docs/03-tools/dodge-burn-sponge.md:21 |
| Spot channel | cs6 | An extra ink plate channel for printing with spot inks; carries a Solidity value and merges into CMYK. | docs/02-ui-ux/panels/channels-panel.md:16 |
| Spot color | cs6 | Named ink from a color library used in spot channels; Photoshop prints spot colors as CMYK except in Duotone mode. | docs/07-color-painting/color-picker.md:61 |
| Spot Healing Brush | cs6 | Healing brush that needs no sample point and samples around the clicked area. Its Type is Proximity Match, Create Texture, or Content-Aware. | docs/03-tools/healing-brushes.md:41 |
| sRGB IEC61966-2.1 | code/format | The standard RGB working space for web and consumer cameras; the default in the North America General Purpose 2 preset, as `Profile::srgb`. | crates/pictura-color/src/lib.rs:50 ; docs/10-workflow-io/color-settings.md:28 |
| Stack mode | cs6 | A Photoshop Extended per-channel reduction (Mean, Median, Maximum, and others) applied to an image stack inside a Smart Object. | docs/05-layers/smart-objects.md:51 |
| Status bar | cs6 | The bar at the bottom of a document window showing magnification, file size, tool instructions, and view-option readouts. | docs/02-ui-ux/application-frame.md:83 |
| store_composite | code | Persists a full-frame rendered composite into `doc.composite`, preserving a non-RGB document's colour-plane count while RGB takes the RGBA frame. | crates/pictura-app/src/cxxqt_object.rs:3291 |
| straight-alpha | domain | The planar 4-channel representation where colour is not premultiplied by alpha, returned by both the CPU and GPU compositors. | openspec/specs/compositing/gpu-compositing/spec.md:6 |
| Stroke | code | A live stroke that accumulates paint into a working document over an untouched base document, reporting dirty regions as it goes. | crates/pictura-paint/src/stroke.rs:22 |
| StrokeConfig | code | Complete brush configuration for a stroke: colour, background, diameter, hardness, roundness, angle, spacing, opacity, flow, blend mode, and flip flags. | crates/pictura-paint/src/lib.rs:31 |
| StrokeDirection | code | Directional stroke orientation: RightDiagonal, Horizontal, LeftDiagonal, or Vertical. | crates/pictura-filters/src/lib.rs:153 |
| StrokeOutcome | code | Result of finishing a stroke: the updated document and the dirty rectangle. | crates/pictura-paint/src/stroke.rs:16 |
| StrokeSample | code | One input point along a stroke, carrying position and pen pressure. | crates/pictura-paint/src/lib.rs:24 |
| Stylize family | code | Emboss, Find Edges, and Solarize. | crates/pictura-filters/src/stylize.rs:1 |
| Stylize filters | cs6 | The Filter Stylize submenu of painted and impressionistic effects; most are 8-bit only, with Emboss the only 32-bit entry. | docs/06-filters/stylize-filters.md:13 |
| stylize-filters | capability | Emboss, Find Edges, and Solarize, with alpha preservation, edge handling, determinism, and an oracle classification. | openspec/specs/imaging/stylize-filters/spec.md:6 |
| Sub-agent | process | A dispatched agent that owns a disjoint set of files or crates for one milestone and is integrated by the orchestrator. | docs/dev/STATE.md:854 |
| Subtract | cs6 | A Photoshop-only blend mode computing max(0, base - source) whose CS5-versus-CS6 origin is disputed. | docs/05-layers/blend-modes.md:190 |
| Surface Blur | cs6 | An edge-preserving blur implemented as a bilateral filter, with Radius as the spatial term and Threshold as the range term. | docs/06-filters/blur-filters.md:24 |
| svg-cursors | capability | An original SVG cursor per implemented tool under assets/cursors/, resolved by id, with catalogue-defined action hotspots and application to the active tool. | openspec/specs/ui/svg-cursors/spec.md:6 |
| Swatches panel | cs6 | Panel storing colors organized into loadable libraries; new colors persist in the preferences file and only become permanent in a library. | docs/02-ui-ux/panels/swatches-panel.md:13 ; docs/07-color-painting/swatches-and-libraries.md:15 |
| SwatchesPanel | code | Dock panel of preset swatches that set the shared `ColorState` when clicked. | crates/pictura-app/cpp/panels/swatches_panel.h:9 |
| Task DAG | process | The table at the top of a milestone brief listing tasks, owners, deliverables, and acceptance checks. | docs/dev/m0-walking-skeleton.md:9 |
| TASK-ALLOWS-DOCS | process | A token that must appear in a commit message (TASK-ALLOWS-DOCS or TASK_ALLOWS_DOCS=1) for guard.sh to allow documentation changes. | docs/dev/STATE.md:851 |
| Term | Type | Definition | Source |
| test_image | code | Deterministic generated gradient used as a fallback so the window always has something to show when no document loads. | crates/pictura-app/src/cxxqt_object.rs:3719 |
| Text engine | cs6 | The closed CS6 text layout, OpenType, and font-metrics engine; behavioral parity only. | docs/00-overview/feasibility-and-non-goals.md:40 |
| Texture (brush) | cs6 | Brush dynamics section applying a pattern to the tip, with Mode, Depth, Scale, Brightness, and Contrast controls. | docs/07-color-painting/brush-dynamics.md:77 |
| Texture family | code | Craquelure, Grain, Mosaic Tiles, Patchwork, Stained Glass, and Texturizer behavioural models, with seeded kernels that are bit-repeatable. | crates/pictura-filters/src/texture.rs:1 |
| Texture filters | cs6 | A Filter Gallery family of 6 surface and depth effects; all are 8-bit only. | docs/06-filters/texture-filters.md:13 |
| texture-filters | capability | Craquelure, Grain, Mosaic Tiles, Patchwork, Stained Glass, and Texturizer, with foreground, background, and texture inputs. | openspec/specs/imaging/texture-filters/spec.md:6 |
| TextureOptions | code | Texture parameters shared by Rough Pastels, Underpainting, Conte Crayon, and Texturizer: surface, scaling, relief, light direction, and invert. | crates/pictura-filters/src/lib.rs:132 |
| TextureSurface | code | Texture preset used by TextureOptions: Brick, Burlap, Canvas, or Sandstone. | crates/pictura-filters/src/lib.rs:124 |
| Theme | code | Single source of truth for the application theme: applies the Fusion style and a dark palette for one of four brightness levels, and no widget may hard-code a frame colour. | crates/pictura-app/cpp/theme.h:10 |
| thiserror | code | Error-derive crate for typed, matchable library errors; `anyhow` is confined to application-boundary entry points. | docs/11-cross-cutting/error-handling.md:65 |
| thiserror and anyhow | code | The error model: typed `thiserror` enums in library crates, `anyhow` context in the binary. | docs/01-architecture/rust-core-design.md:149 |
| Threshold | cs6 | Adjustment that binarizes an image: pixels above the threshold become white and those below become black. The result is neutral, not per-channel. | docs/04-image-ops/adjustments/threshold.md:15 ; docs/05-layers/blend-modes.md:41 |
| TIFF | format | Flexible raster format up to 4 GB with compression (None, RLE, LZW, ZIP, JPEG), pixel/byte order, image pyramid, and layered TIFF options. | docs/10-workflow-io/save-and-save-as.md:93 |
| Tile | project | A fixed-size block of document pixels used as the unit of caching, dirty tracking, and GPU upload. | docs/01-architecture/system-architecture.md:35 |
| Tile cache | project | An LRU cache keyed by `(layer_id, channel, level, TileId)`, evicted under a memory cap and backed by scratch. | docs/01-architecture/system-architecture.md:248 |
| Tilt-Shift | cs6 | A Blur Gallery effect producing a sharp band with a symmetric fade to blur outside it, used for a miniature look. | docs/06-filters/blur-gallery.md:30 |
| Timeline panel | cs6 | CS6's clip-based video and animation timeline, available in all editions; exact parity is a Linux non-goal. | docs/02-ui-ux/panels/timeline-panel.md:13 |
| tip_coverage | code | Tip alpha in 0 to 1 at a layer-local offset from a dab centre, shaped by hardness, roundness, angle, flip, and aliasing. | crates/pictura-paint/src/tip.rs:11 |
| Tolerance | cs6 | Similarity threshold for tools such as Magic Wand and Paint Bucket. Low values match only very similar colors and high values match a broader range. | docs/03-tools/quick-selection-and-magic-wand.md:53 |
| Tolerance (Magic Wand) | cs6 | The Magic Wand color-distance threshold, 0-255, read by Grow and Similar to extend the selection. | docs/08-selection/selection-tools-overview.md:37 |
| Tone mapping | cs6 | The conversion of HDR data to a lower dynamic range using Local Adaptation, Equalize Histogram, Exposure and Gamma, or Highlight Compression. Also used by the 32-bit display preview. | docs/04-image-ops/32-bit-hdr.md:47 |
| tool group / tool slot | code | CS6 tools live in numbered groups (1 to 23) that share one toolbox slot button; clicking cycles members and a flyout selects one. | crates/pictura-app/cpp/toolbox.h:46 |
| Tool preset | cs6 | A saved tool plus its options-bar settings, recallable as a unit. | docs/02-ui-ux/panels/tool-presets-panel.md:13 |
| Tool Presets panel | cs6 | Dock that manages `.tpl` tool-preset libraries, filtered by Current Tool Only. | docs/02-ui-ux/panels/tool-presets-panel.md:13 |
| tool-framework | capability | The fixed tool registry with one active tool and keyboard shortcut selection, the Tools panel, the options bar, canvas pointer routing to the active tool, and tool status and cursor hints. | openspec/specs/tools/tool-framework/spec.md:6 |
| Toolbox | code | The Tools dock: a CS6 single-column list of flyout slots (one button per group), the foreground/background swatches, and the screen-mode toggle. | crates/pictura-app/cpp/toolbox.h:48 |
| ToolCatalogueEntry | code | The frozen 71-row tool catalogue record holding id, label, shortcut, group and slot, implemented flag, cursor shape, and hotspot; the 10 existing tool ids stay unchanged. | openspec/changes/archive/2026-09-16-m38-icon-cursor-library/proposal.md:18 |
| ToolController | code | Routes canvas pointer events to the active tool through one switch rather than one class per tool; it also holds combine mode, tolerance, brush parameters, and foreground/background colours. | docs/dev/STATE.md:50 ; crates/pictura-app/cpp/tools.h:123 |
| ToolId | code | The 71-tool CS6 catalogue enum in frozen table order; the 10 implemented tools keep their M19 names and the order matches `kToolTable` in tools.cpp. | docs/dev/m38-icon-cursor-library.md:32 ; openspec/specs/tools/tool-framework/spec.md:6 ; crates/pictura-app/cpp/tools.h:23 |
| ToolInfo | code | Per-tool metadata struct: id, asset name, label, shortcut, cursor, hint, group, implemented flag, and cursor hotspot. | crates/pictura-app/cpp/tools.h:98 |
| Tools panel | cs6 | The left-docked icon grid of tools, one column by default, with a Screen Mode button at the bottom. | docs/02-ui-ux/toolbox-and-options-bar.md:16 |
| ToolSlotButton | code | A toolbox slot button that shows its group's current tool and distinguishes click (activate) from press-and-hold (open the flyout). | crates/pictura-app/cpp/toolbox.cpp:33 |
| Traceability matrix | process | The table mapping each CS6 feature to its spec file, spec ID, proposed Rust module, proposed Qt component, and status. | docs/TRACEABILITY.md:1 |
| tracing | code | Structured logging API for first-party Rust using spans and events, with the app binary installing a subscriber and one sink for Qt and Rust. | docs/11-cross-cutting/logging-and-telemetry.md:79 |
| Trademark posture | project | Adobe marks are used only nominatively to identify the compatibility target, with no implied affiliation. | docs/00-overview/licensing-and-provenance.md:27 |
| Transfer (brush) | cs6 | Brush dynamics section with Opacity and Flow jitter plus the Mixer Brush Wet, Load, and Mix jitter controls. | docs/07-color-painting/brush-dynamics.md:114 |
| Transform | cs6 | The command family that rotates, scales, skews, distorts, or warps a layer, selection, mask, path, or alpha channel. Transform Selection affects the border itself. | docs/03-tools/move-and-transform.md:23 |
| Transform Selection | cs6 | Command placing a transform box around the selection border itself, resampling the coverage mask without touching pixels. | docs/08-selection/transform-selection.md:13 |
| Transient preview | project | An interactive edit recomputed into a preview layer but not entered into history until commit. | docs/01-architecture/threading-and-concurrency.md:93 |
| translate_layer | code | Shifts the topmost pixel layer's bounds by `(dx, dy)`. Groups and adjustment layers are ignored; no pixels move, the compositor reads the shifted `rect`. | crates/pictura-render/src/document_ops/crop.rs:51 |
| translate_layer family | code | Layer translation operations: translate_layer and recompute are the CPU oracle, translate_layer_active composites through the active backend, and translate_layer_rect shifts a rect without recompute. | docs/dev/STATE.md:47 |
| translate_layer_active | code | Shifts the topmost pixel layer's bounds by `(dx, dy)` and refreshes the composite through the active backend. | crates/pictura-render/src/document_ops/crop.rs:86 |
| translate_layer_rect | code | Shifts the topmost pixel layer's bounds by `(dx, dy)` without recompositing. The caller must refresh the dirty region through the active backend. | crates/pictura-render/src/document_ops/crop.rs:69 |
| Transparency Shapes Layers | cs6 | An advanced-blending option (default on) that restricts effects and knockouts to the layer's opaque pixels. | docs/05-layers/layers-overview.md:104 |
| Tree projection | project | The Layers panel's flattened row list, emitted depth-first and topmost-first from the bottom-first model. | docs/dev/m39-panel-anatomy.md:139 |
| Type layer | cs6 | An editable text layer whose Lock Transparency and Lock Image are on by default; it can be rasterized to pixels. | docs/01-architecture/document-model.md:23 ; docs/05-layers/layers-overview.md:32 |
| Type on a path | cs6 | Type that flows along an open or closed path in the direction the anchors were added. Inside a closed path it is always horizontal. | docs/03-tools/type-tools.md:19 |
| U.S. Web Coated (SWOP) v2 | format | The standard North American CMYK working-space profile used by the prepress presets. | docs/10-workflow-io/color-settings.md:29 |
| UI-001 | project | Spec ID for Application Frame. | docs/02-ui-ux/application-frame.md:3 |
| UI-002 | project | Spec ID for Menus. | docs/02-ui-ux/menus.md:3 |
| UI-003 | project | Spec ID for Workspace and Docks. | docs/02-ui-ux/workspace-and-docks.md:3 |
| UI-004 | project | Spec ID for Toolbox and Options Bar. | docs/02-ui-ux/toolbox-and-options-bar.md:3 |
| UI-010 | project | Spec ID for Preferences. | docs/02-ui-ux/preferences.md:3 |
| UI-011 | project | Spec ID for Keyboard Shortcuts. | docs/02-ui-ux/keyboard-shortcuts.md:3 |
| UI-012 | project | Spec ID for Accessibility. | docs/02-ui-ux/accessibility.md:3 |
| Undo record | project | The stored reversible delta for a committed command, holding changed tiles or structural diffs. | docs/01-architecture/document-model.md:258 |
| ungroup_layer | code | Splices a group's children into the parent at the group's position. Returns false (state unchanged) when `index` is out of range or not a group. | crates/pictura-render/src/document_ops/layer_ops.rs:164 |
| ungroup_paths | code | Splices each listed group's children into its container. Skips non-groups, the Background, and fully-locked nodes. | crates/pictura-render/src/document_ops/layer_ops.rs:611 |
| UnsavedChoice | code | Enum for the unsaved-document prompt: `Save`, `Discard`, or `Cancel`; non-interactive overrides exist for tests. | crates/pictura-app/cpp/dialogs.h:9 |
| Unsharp Mask | cs6 | A blur-difference sharpener computing original + (original - blurred) * amount, gated by a threshold; it is not an edge detector. | docs/06-filters/sharpen-filters.md:17 |
| Use Legacy | cs6 | Brightness/Contrast mode that applies a simple additive shift instead of the nonlinear modern curve. It can clip highlights and shadows and is auto-selected for pre-CS3 layers. | docs/04-image-ops/adjustments/brightness-contrast.md:25 |
| Vanishing Point | cs6 | A dialog defining perspective planes so cloning, painting, pasting, and transforming stay perspective-correct; Measure and DXF/3DS export are Extended-only. | docs/06-filters/vanishing-point.md:13 |
| Variables | cs6 | Labels defining which template elements change: Visibility, Pixel Replacement, or Text Replacement, defined per layer. | docs/09-automation/variables-and-data-driven-graphics.md:22 |
| Vector mask | cs6 | A resolution-independent path that clips a layer's contents, created with the pen or shape tools and editable without pixel loss. | docs/01-architecture/document-model.md:30 ; docs/05-layers/vector-masks-and-clipping-masks.md:16 ; docs/08-selection/paths-and-vector-selection.md:49 |
| verification-harness | capability | Golden-image comparison, a stable content hash, the pictura-diff CLI, CI running format, lint, and tests, and a non-goal guard. | openspec/specs/verification/verification-harness/spec.md:6 |
| VERSION_PSB | format | PSB (large document) file version 2 in the header. | crates/pictura-codec/src/lib.rs:34 |
| VERSION_PSD | format | PSD file version 1 in the header. | crates/pictura-codec/src/lib.rs:33 |
| Vibrance | cs6 | Adjustment that boosts saturation less as colors approach full saturation, to limit clipping and protect skin tones. Its Saturation slider applies a uniform change. | docs/04-image-ops/adjustments/vibrance.md:14 |
| VibranceParams | code | Vibrance and saturation values for the Vibrance adjustment. | crates/pictura-adjust/src/lib.rs:92 |
| View Mode (Refine Edge) | cs6 | Pop-up previewing the refined selection, including Show Original and Show Radius plus community-documented modes such as Marching Ants, Overlay, and On Black. | docs/08-selection/refine-edge.md:20 |
| view seam | project | The Rust-to-Qt boundary where C++ reaches the `PictureView` bridge, governed by the adapter-namespace policy in `docs/dev/refactor-view-seam.md` (supersedes Track 4's single-bridge policy). | docs/dev/refactor-view-seam.md:1 |
| ViewportTransform | code | The mapping from document space to window space, holding scale, translation, and rotation. Hand, Zoom, and Rotate View operate on it instead of resampling document pixels. | docs/03-tools/hand-and-zoom.md:124 |
| VulkanHandles | code | Raw Vulkan handles exported from a live wgpu device (instance, physical device, device, queue family) for the zero-copy interop attempt. | crates/pictura-app/src/gpu.rs:233 |
| W3C Compositing and Blending Level 1 | process | The normative source for separable blend formulas and source-over compositing, including the dodge and burn guards and non-separable helpers. | openspec/specs/compositing/blend-modes/spec.md:20 |
| Walking skeleton | process | The M0 milestone that proved the whole stack end to end (Rust, cxx-qt, Qt6, GPU, PSD codec, harness) before any feature work. | docs/dev/m0-walking-skeleton.md:3 |
| Warp | cs6 | Transform mode that deforms through a control-point mesh, with preset Warp Styles plus Bend and X/Y distortion. It is a bicubic Bezier patch deformation. | docs/03-tools/move-and-transform.md:31 |
| Wave | cs6 | A Distort filter like Ripple with multiple wave generators, wavelength and amplitude ranges, a wave type, and a random seed. | docs/06-filters/distort-filters.md:27 |
| WaveType | code | Wave generator shape: Sine, Triangle, or Square. | crates/pictura-filters/src/lib.rs:88 |
| Wet Edges | cs6 | Brush toggle that builds paint along the stroke edges for a watercolor look. | docs/07-color-painting/brush-engine.md:71 |
| Wet/Load/Mix | cs6 | Mixer Brush options: Wet sets canvas pickup, Load sets reservoir paint, and Mix sets the canvas-to-reservoir ratio. | docs/07-color-painting/mixer-brush-engine.md:42 |
| wgpu | code | The safe Rust graphics and compute library based on WebGPU, used by the Rust compositor. | docs/01-architecture/gpu-rendering-pipeline.md:153 ; docs/dev/STATE.md:10 |
| widgetToImage | code | Maps a widget-space point to document/image coordinates, used by tools to convert pointer events. | crates/pictura-app/cpp/image_view.h:87 |
| Wind | cs6 | A Stylize filter that places tiny horizontal lines for a windblown effect, with Wind, Blast, and Stagger methods. | docs/06-filters/stylize-filters.md:25 |
| Work path | cs6 | A temporary, unsaved path that appears in the Paths panel. It can become a selection, vector mask, clipping path, or a raster fill or stroke. | docs/02-ui-ux/panels/paths-panel.md:17 ; docs/03-tools/pen-and-path-tools.md:11 |
| Working space | cs6 | The intermediate color space used to define and edit color per color model, chosen in Color Settings. It also defines the appearance of untagged documents. | docs/01-architecture/color-management.md:35 ; docs/04-image-ops/color-profiles-and-assignment.md:20 ; docs/10-workflow-io/color-settings.md:49 |
| Workspace | cs6 | A named panel arrangement that can be saved, switched, deleted, and reset; Essentials is the default. | docs/02-ui-ux/workspace-and-docks.md:143 |
| Workspace (.psw) | format | A named arrangement of panels, optionally capturing a shortcut set and menu set, saved per user and reset explicitly. | docs/10-workflow-io/workspace-management.md:18 |
| workspace-persistence | capability | Panels registered as named docks with stable objectName, layout saved and restored across restart, Window panels toggle and hide-all, and a versioned XDG session state store. | openspec/specs/document/workspace-persistence/spec.md:6 |
| write_extra | code | Serializes a record's layer mask block, blending ranges, Pascal name, and tagged blocks (`luni`, `lspf`, `lclr`, `iOpa`, adjustment, `lsct`). Default attributes are omitted so default documents serialize byte-identically. | crates/pictura-codec/src/lib.rs:901 |
| write_layer_info | code | Serializes layer info: the flat records, then raw-compressed channel data, padded to a 4-byte boundary. | crates/pictura-codec/src/lib.rs:772 |
| write_psd | code | Serializes a Document's composite image and layer tree into a valid PSD. Supports 8-bit Grayscale/RGB only. | crates/pictura-codec/src/lib.rs:984 ; openspec/specs/codec/psd-codec/spec.md:6 |
| write_record | code | Writes one layer record: rect, channel info table, 8BIM blend key, opacity, clipping, flags, and extra data. | crates/pictura-codec/src/lib.rs:825 |
| write_tag | code | Writes an additional-layer-info tagged block: 8BIM signature, 4-byte key, u32 length, data padded to an even length. | crates/pictura-codec/src/lib.rs:963 |
| XDG Base Directory | project | The Linux path convention for prefs, state, data, and cache, replacing the CS6 `.psp` location model. | docs/11-cross-cutting/preference-storage.md:91 |
| XDG session store | domain | The interface state file at $XDG_STATE_HOME/kooka-pictura/state.json, written atomically through a temporary file and carrying a schema version. | openspec/specs/document/workspace-persistence/spec.md:48 |
| XDG state store | project | The session store that persists the window layout and recent files under the freedesktop XDG state directory. | docs/dev/STATE.md:50 |
| XMP | format | ISO 16684-1 metadata layer and the canonical metadata store, embedded in PSD resources 1058/1059/1060 and readable/editable on the Raw Data tab. | docs/01-architecture/rust-core-design.md:33 ; docs/10-workflow-io/file-info-and-metadata.md:80 |
| ZigZag | cs6 | A Distort filter applying radial displacement with a ridge count and Around Center, Out From Center, or Pond Ripples styles. | docs/06-filters/distort-filters.md:28 |
| ZigZagStyle | code | ZigZag ridge pattern: AroundCenter, OutFromCenter, or PondRipples. | crates/pictura-filters/src/lib.rs:107 |
| ZIP-with-prediction | format | PSD channel compression code 3, a per-row predictor applied before deflate; a common interoperability hazard. | docs/01-architecture/file-formats.md:126 |
| Zoom tool | cs6 | Navigation tool that steps through preset magnifications and centers the display on the clicked point. It supports Scrubby Zoom, Animated Zoom, and Resize Windows To Fit. | docs/03-tools/hand-and-zoom.md:36 |
| Zoomify | cs6 | Export command producing JPEG tiles plus HTML for pan-and-zoom web viewing, with template, tile, and browser size options. | docs/10-workflow-io/export-formats.md:88 |
