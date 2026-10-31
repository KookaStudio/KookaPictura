# GPU Rendering Pipeline

- **Spec ID:** `ARCH-006` (provisional; see `INDEX.md`)
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Yes` (the Mercury Graphics Engine is the CS6 accelerator; see below)
- **Depends on:** `ARCH-003` qt6-ui-design, `ARCH-004` rust-qt-interop, `ARCH-005` threading-and-concurrency, `05-layers/blend-modes.md`, `07-color-painting/color-models.md`

> All module, crate, and shader names below are **design proposals**. The
> repository contains a working implementation; this file stays the
> behavioural contract for the feature, and
> `docs/dev/doc-code-parity-gaps.md` records what shipped and what did not.
> Interop mechanisms marked "target" are not yet verified end to end; see `##
> Open questions`.

## CS6 behavior

CS6 introduced the **Mercury Graphics Engine (MGE)**. Adobe's own statements,
quoted by third parties, describe MGE as new to CS6 and as using **both the
OpenGL and OpenCL frameworks** rather than NVIDIA CUDA. MGE "delivers near-instant
results when editing with key tools such as Liquify, Warp, Lighting Effects and
the Oil Paint filter." With an unsupported card, "in most cases the acceleration
is lost and the feature runs in the normal CPU mode," although some features do
not work at all without a supported GPU.

Acceleration is not universal. Independent testing found that the new blur
filters (Field Blur, Iris Blur, Tilt-Shift) require **OpenCL 1.1**, and that
Reduce Noise is primarily a CPU function. The same tests show Liquify render
times drop substantially from CS5 to CS6 when a supported GPU is present.

Interpretation for this project: MGE is two cooperating mechanisms rather than
one. OpenGL is used for GPU display and compositing; OpenCL is used for
compute-style filters. The modern equivalents are a GPU rendering API (via
wgpu or Qt's QRhi) for compositing and a GPU compute path for filters, with a
CPU fallback.

**Behavioral parity only:** Adobe's exact tiling, precision, and blend
implementation is closed. This document specifies a pipeline that reproduces
observable behavior; it does not claim to reproduce Adobe's internals.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Canvas | GPU widget | n/a | Composited document, overlays, checkerboard |
| Navigator panel | Thumbnail | n/a | Downscaled composite of the same pipeline |
| Layers panel | Thumbnails | n/a | Per-layer reduced composites |
| Histogram / Info | Analysis views | `F8`? | Consume composited or per-channel data |
| Progress / status | CPU fallback indicator | n/a | Show when running without GPU |
| Filter dialogs | Preview | n/a | Preview through the same pipeline |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Graphics backend | enum | Vulkan | Vulkan / OpenGL / Null (CPU) | Platform-dependent default |
| Tile size | int (px) | 256 | 64 to 512 | Power of two; no seams |
| Working precision | enum | RGBA16F | RGBA8 / RGBA16F / RGBA32F | 8/16-bit integer docs map to 16F or 32F |
| Layer cache | MB | TBD | memory budget | LRU; tied to `ARCH-007` cap |
| MSAA | int | 1 | 1, 4, 8 | Canvas overlays only; not document pixels |
| Color space | enum | document | document space / display | Composite in linear document space |
| CPU fallback | bool | auto | auto / on / off | User preference and safety valve |
| Dirty rect | bool | true | on / off | Recompose only changed tiles |

## Algorithms & pipeline

### Tile-based rendering

The document is divided into fixed tiles (proposed 256x256, shared with
`ARCH-005`). A tile is the unit of caching, dirty tracking, and GPU upload. A
tile's content depends on the layer stack below it, so a change to a layer marks
the tiles it overlaps as dirty unless the layer is fully opaque and unblended.
Dirty rectangles drive incremental recomposition; a full recomposite is used for
global changes (color mode, document size, mode switch).

Tiles are cached in an LRU keyed by `(node, generation, resolution)`. Cache
entries are evicted under the memory cap. On GPU, tiles live in a texture atlas
or an array texture; on CPU, in row-major buffers. A generation counter is
incremented on undo/redo so stale tiles are never displayed.

### Industry architecture

Mature editors converge on the same shape, and it is worth stating because this
project's current path is its inverse. The document owns pixels; the screen
shows a derived view; a change produces a **damage region** (rect or tile set);
only that region is re-composited; and the result is **presented as a GPU
texture without reading the whole document back**.

- **GIMP/GEGL** — ~128² tiles in a global LRU (budget ≈ `min(RAM, available)`,
  512 MB floor, swap backends); `GimpProjection` holds a mipmap pyramid; dirty
  tracking via `cairo_region` aligned to 32² chunks; `GimpTileHandlerValidate`
  blits one tile-sized region per tile through the graph (`gegl_node_blit`);
  viewport-priority rendering on an idle callback; the Move tool changes a layer
  offset, not pixels. Known scar: a projectable offset change re-renders the
  whole projection (FIXME), and the mipmap canvas is opt-in.
- **Krita** — 64² CPU tiles in a hash table with copy-on-write; per-node
  `KisProjectionPlane` composited by `KisAsyncMerger`; `setDirty` propagation
  and a rects-walker for `changeRect`/`needRect`/`accessRect`;
  `KisUpdateScheduler` with two queues and workers; Move = node offset; a
  non-destructive Transform Mask applied at composite time; the GPU canvas uses
  256² textures with a `1 << levels` border, per-tile mipmaps, PBO uploads, and
  no host readback. Known scars: freezes at the tile RAM+swap ceiling and hidden
  layers still recomputing.
- **Photoshop** — the GPU (Mercury) is a display/effects layer over a CPU tiled
  working set plus scratch disk, not a fully GPU-resident document. Adobe patent
  US20150062182A1 describes progressively rendered scale-level tiles with the best
  cached tile drawn immediately.
- **Chromium `cc`** — sparse per-scale 256² tiles, a three-tree pipeline,
  viewport-priority raster, and an explicit damage taxonomy in `DamageTracker`
  (paint invalidation vs raster invalidation vs draw/expose damage); missing
  tiles checkerboard rather than stall.
- **Graphite (Rust + wgpu)** — 256² GPU tiles, ~512 MiB LRU, viewport-driven
  cache; the closest reference implementation.

Anti-patterns from the same sources: main-thread-blocking VRAM eviction, tile
seams without a border/gutter, unbounded tile caches (Paint.NET crashed zoomed
out), and `device.poll(Wait)` per frame. The full survey and the staged response
are in `docs/dev/canvas-compositing-plan.md`.

### Layer compositing

Composition runs bottom to top. For each tile, the renderer evaluates the layer
sub-stack that overlaps it, applying in order: layer mask (luminance or
vector-derived), clipping mask constraint, opacity, blend mode, and group
isolation semantics. Adjustment layers and fill layers are evaluated as
operations inserted at their stack position. Groups that are isolated cause a
separate compositing pass before being blended as a unit.

Alpha is premultiplied for compositing. The core source-over step is the
standard Porter-Duff "over" on premultiplied values, with the blend function
applied to color before the over step, per the standard PDF/CSS compositing
model. Straight versus premultiplied storage is an internal representation
choice; the boundary and file I/O use the documented PSD conventions.

### Blend modes on GPU

CS6 has 27 blend modes. They divide into **separable** modes that operate
independently per channel (Normal, Multiply, Screen, Overlay, Darken, Lighten,
Color Dodge, Color Burn, Hard Light, Soft Light, Difference, Exclusion, Linear
Burn, Linear Dodge, Vivid Light, Linear Light, Pin Light, Hard Mix, Subtract,
Divide) and **non-separable** modes that operate on the RGB triplets (Hue,
Saturation, Color, Luminosity). Gamma-correct separability and blend formulas
are a known interoperability hazard and must be pinned to one specification
(the PDF or CSS compositing spec) and tested.

Implementation proposal: a single compositing shader with a mode selector, or a
small family of shaders generated from one source. Non-separable modes are
branch-free functions of the RGB triplets. The exact formula set and its
precision are specified in `05-layers/blend-modes.md`.

### Backend choice: wgpu versus Qt QRhi

Both APIs can access Vulkan on Linux, and both are viable. The decision is
really about texture and device ownership between Qt and Rust.

- **wgpu** is a safe, portable Rust graphics and compute library based on the
  WebGPU standard. It runs natively on Vulkan, Metal, D3D12, and OpenGL, accepts
  WGSL, SPIR-V, and GLSL shaders (SPIR-V and GLSL behind features), and exposes
  compute pipelines. It is the natural home for the Rust core and for
  compute-style filters.
- **Qt QRhi** is Qt's own GPU abstraction used by the Qt Quick scene graph and
  Qt Quick 3D. It has a Vulkan backend and is what backs `QRhiWidget` and
  `QQuickRhiItem`. It is a semi-public API with limited compatibility
  guarantees: source/binary compatibility is only promised for the Qt version
  the application was built against. A QRhi instance and all its resources are
  bound to one thread, and resources are not shareable between QRhi instances.

Interop primitives that make sharing possible, per the fetched Qt docs:

- `QRhi::create()` accepts an `importDevice` handle, so a QRhi can be created on
  top of an externally created device (for example a Vulkan device created by
  Rust).
- `QRhiTexture::createFrom(NativeTexture)` wraps an existing native texture as a
  **non-owning** `QRhiTexture`; `QRhiTexture::nativeTexture()` exposes a
  QRhi-created texture to a foreign engine. Sizes, formats, sample counts, and
  flags must still be set correctly; ownership is not transferred.
- `QRhiTexture::setNativeLayout()` communicates the image layout after native
  Vulkan commands, and `QRhiCommandBuffer::beginExternal()`/`endExternal()`
  allow recording native API commands inside an RHI render pass.
- `QRhiWidget` states it can incorporate native rendering through
  `beginExternal` and wrap native textures with `createFrom`, while cautioning
  that device/context configurability is limited because its primary goal is
  QRhi-based rendering.

Constraints to respect: one widget window uses one graphics API; two widgets
requesting different APIs cannot both work; a QRhi is single-threaded; QRhi
resources are not shareable between instances.

### Recommendation (staged)

The research pass and the M29 timing evidence reorder this. At 4000² the GPU
composite is **readback-bound**: the 64 MB readback plus the `QImage` conversion
dominates, and the CPU/GPU composite work does not. The first win is therefore
not a new device arrangement but **not doing whole-document work per update**.
Stages, in order (M31–M33 in `docs/dev/canvas-compositing-plan.md`):

**Stage 1 — dirty-region compositing (correct, lazy).** Composite only the
changed document rect into a cached full-document canvas and read back only that
rect. A sub-rect composite is byte-identical to the corresponding sub-rect of the
full composite, so it is testable directly against the CPU oracle. This stays on
the ordinary QWidget/QImage canvas and removes the full composite and full
readback from every move and paint.

**Stage 2 — zero-readback present.** Keep the composite in a persistent GPU
texture and present it directly. **Qt Quick is required**: `QRhiWidget` owns its
`QRhi` and cannot adopt our wgpu device (M0.5 finding), so the host is
`QQuickRhiItem` sharing the window's `QRhi`,
`QQuickWindow::createTextureFromRhiTexture()` (6.6), or one Vulkan device shared
via `QQuickGraphicsDevice::fromDeviceObjects(...)`. The existing same-device
image-sharing path (`QRhiTexture::createFrom`) already works; presentation was
the gap. This removes the readback and the `QImage` conversion entirely.

**Stage 3 — 256² GPU tiles, LRU, seam gutters, mipmaps.** The Graphite-style
consensus architecture, adopted only if pan/zoom over documents larger than VRAM
demands it. Includes display-time LoD so a zoomed-out view composites a proxy.

**Rejected for now:** presenting with wgpu directly to a native window outside Qt,
and migrating the canvas host before Stage 1 has shown the readback still
dominates. The former bypasses widget composition, docking, and HiDPI handling
and conflicts with `ARCH-003`; the latter is a large architectural change with no
measured justification yet.

### Shader strategy

One shader source, two consumers. Author in **WGSL** for wgpu and transpile to
**SPIR-V** for Qt (`QShader`/`.qsb` via the Qt Shader Tools, `qsb` or
`qt_add_shaders`). If transpilation proves lossy, author in Vulkan-style GLSL
(as the Qt examples do) and compile to SPIR-V, which wgpu accepts with its
`spirv` feature. The pipeline fixes Vulkan as the Linux backend so SPIR-V is the
common currency. Shader compilation and packaging belong in the build spec
(`ARCH-013`).

### Color management in the pipeline

- Composite in **linear light** in the document's working space at the chosen
  precision (default RGBA16F; 32F for HDR/32-bit documents).
- Keep integer 8/16-bit documents and floating 32-bit documents distinct; do not
  silently clamp 32-bit data to 16F when the document is 32-bit (see
  `04-image-ops/32-bit-hdr.md`).
- Apply ICC transforms through 3D LUT textures on the GPU, or via lcms2 on the
  CPU for non-RGB spaces. CMYK and Lab blends are not trivially separable and
  may require a CPU or LUT-backed path.
- Encode to the display transfer function only at presentation. wgpu applies the
  sRGB OETF automatically **only** when rendering to an `*Srgb` view format;
  for every other color space the shader must apply the transfer function and
  gamut conversion itself. QRhi has an analogous `sRGB` texture flag. The
  present pass is therefore the single point that encodes for the display.
- Soft proofing inserts a proofing transform before display encoding.

### CPU fallback

When no usable GPU is present, or a device is lost, the same tile interface is
served by a CPU raster compositor implemented in Rust (workers from `ARCH-005`).
Output is a CPU buffer displayed through a raster viewport (`QGraphicsView`
background or a plain widget blit). The CPU path must reproduce the blend
formulas and color handling within tolerance, at lower performance. The pipeline
must expose backend availability and a user-visible indicator.

### Windows backend selection

On Windows the compositor defaults to Direct3D 12, then Vulkan, then the CPU.
A launch pins exactly one backend: `wgpu::Instance::new` initializes every
backend named in its bitmask, so a multi-backend instance would still load the
Vulkan ICD that crashes on Intel Gen9 hardware. The launcher probes each
candidate in a child process (`pictura --gpu-probe dx12|vulkan`), records the
outcome in `kooka-pictura/gpu-backend.json` beside `state.json`, and pins the
winner in the parent. A `gpu-init.marker` left by a crash during the parent's
own init marks that backend `crashed` on the next launch. `WGPU_BACKEND`
replaces the candidate order but still probes and still honors a `crashed`
entry; `WGPU_BACKEND=none` forces the CPU. Linux keeps the direct Vulkan path,
and the zero-copy interop probe stays Vulkan-only.

## Rust module mapping

- `pictura_render::tile` — tile grid, dirty rectangles, cache keys, generation.
- `pictura_render::cache` — LRU tile/atlas cache with a memory budget.
- `pictura_render::graph` — composition graph derived from the layer tree.
- `pictura_render::blend` — blend-mode functions, separable and non-separable.
- `pictura_render::wgpu_backend` — device, pipelines, compute passes.
- `pictura_render::cpu_backend` — raster compositor fallback.
- `pictura_render::color` — working space, LUT upload, present encoding.
- `pictura_render::bridge` — texture handoff to/from the Qt side.

Data crossing: tile handles, `u32`/`u64` generation stamps, GPU texture ids,
small present-state structs. No Qt types in `pictura_render`.

## Qt6 component mapping

- `QRhiWidget` — proposed canvas host in a Widgets window; manages the backing
  color texture, depth/stencil buffer, and render target; `initialize()`/
  `render()`; `setApi()` called early to force Vulkan.
- `QQuickRhiItem` — alternative host if the canvas moves to Qt Quick; enforces
  the item/renderer split on the scene-graph thread.
- `QRhi` / `QRhiTexture` / `QRhiCommandBuffer` — texture sharing
  (`createFrom`, `nativeTexture`, `setNativeLayout`, `beginExternal`).
- `QRhiShaderResourceBindings`, `QRhiGraphicsPipeline`, `QShader` — shader and
  pipeline plumbing.
- `QGraphicsView` / custom overlay widget — checkboard, rulers, guides, handles
  above the composited texture.

Widgets-vs-QML rationale is in `ARCH-003`; this document only requires that the
host expose a GPU color buffer.

## Data-model impact

- The tile cache, atlas, and pipelines are runtime state and are never
  serialized.
- The composition graph is derived from the layer tree; no new document fields.
- Blend-mode identity maps to PSD blend-mode keys in `05-layers/blend-modes.md`
  and to layer comps and styles.
- Precision and working-space fields live on the document model and round-trip
  through PSD/PSB and XMP; they are inputs to this pipeline (`ARCH-010`).
- Undo/redo bumps the tile generation counter and invalidates caches; no undo
  record stores pixels produced by the pipeline.

## Edge cases

- **No GPU / GPU blocked.** CPU fallback, with the indicator; never fail to open.
- **Device lost or driver reset.** Cancel GPU jobs, release device resources,
  rebuild on the same backend or fall back to CPU; see `ARCH-005`.
- **Huge (PSB) documents.** Respect `textureSizeMax`, `MaxColorAttachments`,
  VRAM and cache budgets; use tiling and, if needed, downscaled proxy tiles for
  the viewport.
- **1-pixel documents and empty documents.** Tile math must handle sub-tile
  sizes; zero-size layers are valid.
- **8/16/32-bit.** 8/16-bit integer documents composite at 16F or 32F without
  banding; 32-bit documents keep float precision end to end and are not clamped.
- **CMYK / Lab / Duotone / Indexed.** Non-RGB working spaces need LUT or CPU
  paths; indexing is converted on entry to the composite.
- **Non-separable blend precision.** Hue/Saturation/Color/Luminosity and Hard
  Mix/Linear Light are sensitive to precision and clipping; test against a
  reference.
- **Masked, clipped, and isolated groups.** Evaluate in the correct order; a
  group's mask and blend apply to the group result, not per child.
- **Adjustment and fill layers, smart filters.** They are graph nodes, not
  pixels; they must not be baked by the cache keyed only on pixels.
- **Undo/redo invalidation.** Generation stamps must invalidate both GPU and CPU
  caches; a stale tile must never win.
- **Tile seams.** Bilinear sampling, mask feathering, and nonseparable blends can
  seam across tile borders; use overlapping compute or a 1-tile apron where
  needed.
- **HiDPI.** The canvas color buffer is sized to widget size times
  `devicePixelRatio`; zoom is a separate transform and must not be conflated with
  the device ratio.
- **MSAA.** Applies to overlay geometry, not to document pixel data.
- **Color banding.** 8-bit intermediate buffers cause banding on gradients;
  default to 16F in the composite.
- **Presentation transfer function.** Encoding twice (shader plus `*Srgb`
  format) produces washed or crushed output; the present pass is the only
  encoding point.
- **Readback cost.** CPU readback for progress or thumbnails can stall the GPU;
  throttle and use tiles.

## Parity acceptance criteria

- Given a reference composite of a test document, the GPU pipeline output matches
  the CPU pipeline output within a per-channel tolerance T (TBD) in 8/16/32-bit
  modes.
- Given each of the 27 blend modes, the result matches the pinned PDF/CSS
  formula reference within tolerance, including the non-separable modes.
- Given a document with masked, clipped, and isolated groups, the composite
  matches a layer-by-layer reference.
- Given the same document at 8-bit, 16-bit, and 32-bit, no precision is lost
  beyond the source bit depth and color management is applied once.
- Given a filter result produced offscreen, it appears on the canvas without a
  visible stall beyond the defined budget.
- Given the GPU is unavailable, the CPU fallback renders the same document
  within tolerance and the UI indicates CPU mode.
- Given undo/redo, no stale tile is displayed and caches are invalidated by
  generation.
- Given a document near the maximum supported size, the pipeline stays within
  the configured memory cap and reports when a proxy resolution is in use.

## Sources

Fetched for this document:

- `https://www.geeks3d.com/20120425/adobe-creative-suite-6-opencl-accelerated-mercury-graphics-engine-opengl/`
  — Adobe statement that MGE is new to CS6, uses OpenGL and OpenCL, not CUDA;
  accelerates Liquify, Warp, Lighting Effects, Oil Paint; CPU fallback and
  features that require a supported card.
- `https://barefeats.com/pscs6.html` — Field/Iris/Tilt-Shift blurs require
  OpenCL 1.1; Reduce Noise is primarily a CPU function; Liquify CS5 vs CS6
  speedup; GPU-disabled CPU rendering.
- `https://www.pugetsystems.com/labs/articles/Adobe-Photoshop-CS6-GPU-Acceleration-161/`
  — MGE uses the video card via OpenCL and OpenGL for "certain features";
  benchmark methodology for GPU-accelerated effects; enabling "Use Graphics
  Processor" in Performance preferences.
- `https://doc.qt.io/qt-6/qrhi.html` — QRhi backends, shaders via SPIR-V/QShader,
  single-thread instance affinity, resources not shareable between instances,
  ownership rules, `nativeHandles`.
- `https://doc.qt.io/qt-6/qrhitexture.html` — `createFrom(NativeTexture)`
  wrapping of external textures, `nativeTexture()` export, non-ownership,
  `setNativeLayout()` for Vulkan/D3D12, sRGB and format flags.
- `https://doc.qt.io/qt-6/qrhiwidget.html` — backing texture and render target
  management, `initialize`/`render`, `setApi` early, one API per window,
  `beginExternal`/`createFrom` for native rendering.
- `https://doc.qt.io/qt-6/qquickrhiitem.html` — Quick-hosted RHI item, renderer
  on the scene-graph thread, same RHI instance as the window, SPIR-V via `.qsb`.
- `https://doc.qt.io/qt-6/topics-graphics.html` — RHI as the layer translating
  Qt graphics calls to target APIs; `QRhiWidget`/`QQuickRhiItem` as hosts.
- `https://wgpu.rs/` — wgpu is a safe, portable Rust graphics and compute
  library based on WebGPU; runs on Vulkan, Metal, D3D12, GLES.
- `https://docs.rs/wgpu/latest/wgpu/` — wgpu 30.0.1; WGSL/SPIR-V/GLSL shader
  input; HDR surface color spaces and the rule that wgpu applies the transfer
  function automatically only for `*Srgb` view formats; MSRV 1.87.

Industry survey for the `## Industry architecture` subsection (full list and
per-editor notes in `docs/dev/canvas-compositing-plan.md`):

- `https://gegl.org/features.html` and `https://gegl.org/environment.html` —
  GEGL tile size and cache budget.
- `https://raw.githubusercontent.com/GNOME/gimp/master/app/core/gimpprojection.c`
  — GIMP projection, mipmap pyramid, dirty regions, per-tile blit.
- `https://raw.githubusercontent.com/Krita/krita/master/libs/image/tiles3/kis_tiled_data_manager.h`
  — Krita 64² copy-on-write tiles.
- `https://docs.krita.org/en/reference_manual/layers_and_masks/transformation_masks.html`
  — Krita composite-time transform mask.
- `https://deepwiki.com/KDE/krita/3.2-opengl-rendering` — Krita GPU canvas
  (256² textures, border, mipmaps, PBO, no readback).
- `https://patents.google.com/patent/US20150062182A1/en` — Adobe tile-based
  caching for complex artwork.
- `https://www.nvidia.com/content/adobe/pdf/adobe-hardware-performance-white-paper.pdf`
  — Mercury / OpenCL over a CPU tiled working set.
- `https://chromium.googlesource.com/chromium/src.git/+/HEAD/docs/how_cc_works.md`
  — Chromium `cc` tiles and three-tree pipeline.
- `https://chromium.googlesource.com/chromium/src/+/f830f3d4ae0191c9095949096e9c3ca1f6dc8d12/cc/damage_tracker.h`
  — `DamageTracker` damage taxonomy.
- `https://docs.rs/wgpu/latest/wgpu/struct.Queue.html` — readback/submit cost
  model as used in the anti-patterns.
- `https://doc.qt.io/qt-6/qrhiwidget.html`,
  `https://doc.qt.io/qt-6/qquickrhiitem.html`,
  `https://doc.qt.io/qt-6/qquickwindow.html`,
  `https://doc.qt.io/qt-6/qquickgraphicsdevice.html` — Qt host constraints and
  the Qt Quick zero-readback present path.
- `https://deepwiki.com/GraphiteEditor/Graphite/4.3-tile-based-caching` —
  Graphite 256² GPU tiles, LRU, viewport-driven cache.

Not fetched or not usable in this pass:

- Adobe Photoshop CS6 Help GPU FAQ (linked from the above pages; not fetched).
- `https://frameandfocal.com/post-processing/...` — returned unrelated content.

## Open questions

> **M0.5 finding (2026-09, Linux, Qt 6.11.1, wgpu 30.0.1):**
> - Offscreen wgpu (Vulkan) render → readback → `QImage` **works** and is the
>   M1 default; CPU fallback verified.
> - **Device + texture sharing works**: QRhi `importDevice` adopted the
>   wgpu-created `VkDevice`, and `QRhiTexture::createFrom` imported a wgpu-owned
>   `VkImage`. So the first two questions below are answered *yes*.
> - **On-screen present via `QRhiWidget` is blocked**: `QRhiWidget` owns its
>   `QRhi` internally (no adopt/`setRhi` hook). On-screen zero-copy needs a manual
>   `QRhi` + `QWindow` swapchain and `VK_KHR_swapchain` (absent from the wgpu
>   device). Evidence: `crates/pictura-app/GPU-INTEROP-NOTES.md`.

> **Industry-survey finding (canvas-compositing-plan.md):** at 4000² the GPU
> composite is readback-bound, so the first priority is **viewport/dirty-region
> compositing** (M31), not a new device arrangement. Zero-readback present
> (M32) requires **Qt Quick** — `QQuickRhiItem` + `createTextureFromRhiTexture`,
> or a shared Vulkan device via `QQuickGraphicsDevice::fromDeviceObjects` —
> because `QRhiWidget` cannot adopt the wgpu device. **256² GPU tiles + LRU +
> seam gutters + mipmaps** (M33, Graphite-style) are the last step, gated on
> pan/zoom over documents larger than VRAM. The questions below are re-ordered
> accordingly.

- **wgpu–QRhi device sharing feasibility.** ~~Unverified.~~ **Resolved (M0.5):**
  QRhi's Vulkan backend imports the wgpu `VkDevice` and wraps a wgpu `VkImage`;
  see the finding above.
- **`QVulkanInstance` wrapping.** ~~Must be confirmed.~~ **Resolved (M0.5):**
  `QVulkanInstance::setVkInstance` + `QRhi::create(..., importDevice)` works.
- **Queue and layout ownership.** Which side owns the queue and who performs
  layout transitions when both engines touch the same image. `setNativeLayout()`
  is the documented hook but the full contract needs testing. Applies to
  **Stage 2** (zero-readback present), not Stage 1.
- **Exact Mercury feature set.** The authoritative CS6 GPU feature list from
  Adobe is not fetched; current claims come from secondary sources. Resolve from
  the archived CS6 Help PDF/GPU FAQ.
- **27 blend-mode formulas and gamma.** The exact formula set, the separability
  convention, and gamma handling must be pinned in `05-layers/blend-modes.md`.
- **Shader authoring language.** WGSL-to-SPIR-V transpilation quality versus
  authoring Vulkan GLSL for both consumers. Resolve with a shader spike.
- **GPU color management.** Whether 3D LUTs on the GPU cover CMYK/Lab/soft-proof
  accuracy, or whether those paths stay on the CPU. Resolve in
  `01-architecture/color-management.md`.
- **Progress for GPU jobs.** Whether incremental progress is meaningful for GPU
  filters or the indicator is indeterminate. Resolve with `ARCH-005`.
- **Precision defaults per document type.** Mapping of 8/16/32-bit documents to
  composite precision and the resulting memory cost. Resolve in
  `04-image-ops/bit-depth-and-conversion.md` and `01-architecture/performance-targets.md`.
- **Linux driver matrix.** Which Vulkan drivers and versions are required for the
  zero-copy path, and what the guaranteed fallback is. Resolve in
  `ARCH-013` build-and-packaging and `11-cross-cutting/testing-strategy.md`.
- **Dirty-region scope (Stage 1).** Which operations are safely region-composable
  (per-pixel, order-independent across disjoint rects) and which must force a
  full recomposite (colour-mode change, document size, adjustment layers with
  neighbourhood effects, Dissolve's RNG). Resolve in M31 against the CPU oracle.
- **Canvas host migration (Stage 2).** Whether the docked QWidget shell can host
  a `QQuickRhiItem` canvas without breaking docking/HiDPI, and the effect on
  `ARCH-003`. Resolve only after Stage 1 profiling shows the readback still
  dominates.
- **Tile budget and eviction (Stage 3).** LRU size, eviction trigger (never on
  the GUI thread), and seam-gutter width for 256² tiles. Resolve in M33.
