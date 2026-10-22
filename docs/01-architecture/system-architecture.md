# System Architecture

- **Spec ID:** `ARCH-001`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Yes` — the Mercury Graphics Engine (MGE), CS6's unified GPU-accelerated pipeline built on OpenGL + OpenCL, is new in CS6.
- **Depends on:** none (foundational). Referenced by `ARCH-002`–`ARCH-004` and by every feature spec.

This document defines the layer boundaries of Kooka Pictura: the Rust core, the Qt6
shell, the GPU compositor, and where the process, FFI, plugin, and scripting
boundaries sit. It maps what is documented about Photoshop CS6's real
architecture (Mercury Graphics Engine, 64-bit addressing, scratch disks,
tile-based processing) onto a proposed Linux implementation. Proposed crate and
component names are design proposals, not commitments.

## CS6 behavior

Photoshop CS6 is a **single documented process** on both Windows and macOS. On
macOS CS6 is 64-bit only; on Windows Adobe shipped 32-bit and 64-bit builds, and
the 32-bit build can directly address only about 3 GB of RAM while the 64-bit
build can address as much RAM as the OS and hardware allow. Adobe's own
performance paper states that the 64-bit versions complete day-to-day imaging
tasks at least 10% faster than the 32-bit equivalents. ([CS6 Help PDF](#sources);
[Adobe performance paper](#sources).)

Rendering and interactive editing are driven by the **Mercury Graphics Engine
(MGE)**, new in CS6. MGE "uses both the OpenGL and OpenCL frameworks. It does not
use the proprietary CUDA framework from nVidia." MGE accelerates Liquify, Warp
and Puppet Warp, Lighting Effects, Oil Paint, Adaptive Wide Angle, the CS6 Blur
Gallery (Field Blur, Iris Blur, Tilt-Shift), and the CS6 3D enhancements.
Minimums documented by Adobe: 256 MB VRAM for GPU features, 512 MB VRAM for 3D,
OpenGL 2.0 + Shader Model 3.0 for OpenGL acceleration, and OpenCL 1.1 for the
OpenCL-accelerated blur filters. ([GPU FAQ](#sources); [Geeks3D](#sources).)

Internally, Photoshop processes a document as a set of **tiles**. Adobe's tuning
paper: "When Photoshop processes a photo, it splits the picture into smaller
image sections called tiles, and it works on each in turn. By default, the size
of each tile is 128Kb." A multi-resolution cache (default 4 levels) is maintained
for fast screen redraws, and the cache is reused by operations such as the
Healing Brush. ([Adobe performance paper](#sources).)

Memory is a two-tier system: RAM (allocation defaulting to 60% of system memory)
backed by **scratch disks** (virtual memory on disk). Adobe documents support for
up to 64 EB of scratch space spread across a maximum of four volumes; the status
bar exposes SCRATCH SIZES and EFFICIENCY, where Efficiency is the percentage of
time spent computing rather than paging, and sustained values below ~95% indicate
the scratch disk is being hit. ([Adobe performance paper](#sources).)

Plugin and scripting surfaces in CS6 are in-process: native 8BF/plug-in modules
run inside the Photoshop address space, and automation runs through ExtendScript
(JSX) plus OS bridges (AppleScript / Windows COM). Adobe's GPU Sniffer ("a small
program") probes the GPU at every launch and disables GPU features if the probe
fails. ([GPU FAQ](#sources).)

| CS6 architectural fact | Documented value | Confidence |
|---|---|---|
| Process model | Single process, 64-bit (Mac 64-bit only) | Sourced |
| 32-bit address ceiling | ~3 GB | Sourced |
| 64-bit speed change | ≥10% faster for daily tasks | Sourced |
| GPU API | OpenGL + OpenCL (not CUDA) | Sourced |
| GPU feature VRAM floor | 256 MB (3D: 512 MB) | Sourced |
| OpenGL floor | OpenGL 2.0 + Shader Model 3.0 | Sourced |
| OpenCL floor | OpenCL 1.1 (blur gallery) | Sourced |
| Working-unit model | Tiles, default 128 KB tile size | Sourced |
| Cache levels | Default 4 (1–8) | Sourced |
| RAM allocation | Default 60% of system RAM | Sourced |
| Scratch | ≤64 EB across ≤4 volumes | Sourced |
| GPU qualification | GPU Sniffer probe at launch | Sourced |
| Internal threading model | Not publicly documented | **Inferred / unknown** |
| Exact tile pixel dimensions | Not publicly documented (only byte size) | **Inferred / unknown** |

## UI surface

The architecture is exposed to the user only through preferences, diagnostics,
and failure dialogs. There is no user-facing "architecture" panel.

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Edit > Preferences > Performance` | Preferences page | — | RAM allocation, scratch disks, history states, cache level/tile, GPU settings. |
| `Edit > Preferences > Performance > Advanced Settings` | Dialog | — | GPU mode Basic / Normal / Advanced; OpenCL compute toggle; 30-bit display (Windows only in CS6). |
| `Edit > Preferences > 3D` | Preferences page | — | VRAM allocation slider for the 3D engine. |
| `Edit > Purge` | Menu | — | Frees Undo / Clipboard / Histories memory on demand. |
| Status bar | Status readout | — | SCRATCH SIZES and EFFICIENCY. |
| `Help > System Info` | Dialog | — | Reports the environment and detected GPU; VRAM is `Unavailable` because wgpu does not expose it. |
| Launch-time GPU warning | Modal dialog | — | Shown once when the GPU Sniffer fails. |

## Parameters & ranges

These are the CS6-visible knobs that this architecture must be able to honor.
They are also inputs to the budgets in `ARCH-003`.

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Memory Usage (RAM) | Slider, % of system RAM | 60% | 5–100% in practice; ideal range shown by app | Changing more than ~5% at a time and restarting is Adobe's guidance. |
| Scratch Disks | Ordered volume list | Startup volume | Up to 4 volumes; ≤64 EB total | Order = usage priority; startup drive last. |
| History States | Integer | 20 | 1–1000 | Larger = more scratch, more undo. |
| Cache Levels | Integer | 4 | 1–8 | 1 disables caching (Adobe warns against). |
| Cache Tile Size | Enum / bytes | 128 KB | Adobe preset steps | Larger tiles help large images on >1 GB RAM. |
| GPU: Use Graphics Processor | Checkbox | On if qualified | On / Off | Disables all GL features when off. |
| GPU mode | Enum | Normal | Basic / Normal / Advanced | Restart required. |
| GPU: Use OpenCL | Checkbox | On if supported | On / Off | Required for Blur Gallery acceleration. |
| 3D VRAM allocation | Slider, % of VRAM | — | Up to 100% | Windows CS6; reserves OS share at 100%. |

Defaults for `Cache Levels = 4` and `History States = 20` are documented in the
GPU FAQ and the Adobe performance paper respectively. The tile size is documented
as 128 KB. All other ranges marked "preset steps" are **inferred** from the UI
description and need empirical confirmation.

## Algorithms & pipeline

### Proposed layering

```text
┌───────────────────────────────────────────────────────────────────┐
│ Qt6 shell  (C++/QML, GUI thread)                                   │
│   application frame · panels · dialogs · tool controllers          │
│   CanvasView (QWidget or QQuickItem)                               │
└──────────────▲──────────────────────────────────┬──────────────────┘
               │ cxx-qt bridge (handles, commands) │ present
               │                                   ▼
┌──────────────┴───────────────────────────────────────────────────┐
│ Rust core  (one OS process by default)                            │
│   pictura-core    Document · Layer · Channel · TileCache          │
│   pictura-color   ICC / working spaces (lcms2)                    │
│   pictura-codec   PSD/PSB/TIFF/PNG/JPEG/… (image, zune-jpeg, …)   │
│   pictura-filters CPU filter kernels (rayon)                      │
│   pictura-render  GPU compositor (wgpu) + tile scheduler          │
│   pictura-script  scripting host                                  │
└───────┬───────────────────────────────┬───────────────────────────┘
        │ optional core subprocess       │ plugin host boundary
        ▼                                ▼
┌───────────────────────┐      ┌────────────────────────────────────┐
│ pictura-core worker    │      │ out-of-process plugin host         │
│ (crash isolation /     │      │ (sandboxed helper process; proposed)│
│  scratch manager)      │      └────────────────────────────────────┘
└───────────────────────┘
```

### Process boundary (recommended: single process)

The default recommendation is a **single process** with two threads of control:
the Qt GUI thread and a Rust worker pool. Rationale: CS6 is a single process,
and a single process avoids copying tile data across an IPC boundary for every
brush stroke. Tile data is the hot path; moving it through a socket or pipe would
put pan/zoom and brush latency at the mercy of IPC scheduling.

A **core subprocess** is retained as a proposal for specific cases:

- Crash isolation for untrusted decoders (PSD/TIFF/RAW parsers) and filters.
- Scratch-disk management as a dedicated service that can survive a UI restart.
- Enforcing a hard memory ceiling without destabilizing the UI.

If adopted, the boundary is tile-granular and uses shared memory (memfd /
`/dev/shm`) for pixel payloads; commands and handles travel over a small
control channel. This is a design proposal, not a CS6-derived requirement.

### FFI boundary (Rust ↔ Qt)

Proposed mechanism: **cxx-qt** (KDAB), which generates C++ `QObject` types from
Rust `#[cxx_qt::bridge]` modules over the `cxx` bridge. The Rust side exports
opaque handles and value types; the Qt side owns presentation objects. The
boundary must not expose raw pointers to tile memory that the Rust side can free.
Pixel uploads go through a buffer that is either copied once per visible tile or
wrapped as a QRhi/`wgpu` shared texture (see `ARCH-003` for the throughput
budget). A manual `extern "C"` surface is the fallback if cxx-qt cannot express a
required Qt type.

### GPU compositor boundary

Two GPU stacks meet here: Qt's **QRhi** (Qt Rendering Hardware Interface, which
selects Vulkan/OpenGL/other backends at runtime) and the Rust `wgpu` device.
The proposed approach is that the Rust compositor owns the `wgpu` device and
renders composite results into a texture that the Qt scene graph presents, or,
where a shared texture path is unavailable, `wgpu` renders to a CPU-readable
buffer and Qt uploads it. QRhi is documented as not sharing resources between
instances, so the design must pick one owner for each texture and keep the
handoff explicit.

### Plugin host boundary

CS6 plugins are in-process native modules, which is a stability and security
liability on Linux. Proposal (not parity-required): a stable C ABI for the Rust
core, with first-party effects compiled in, and third-party plugins hosted
**out of process** behind a capability-restricted RPC surface. The plugin ABI is
specified in `ARCH-001` companion `plugin-and-scripting-abi.md` (planned).

### Scripting host boundary

CS6 automation is ExtendScript/JSX plus OS script bridges. Proposal: an embedded
scripting host (language TBD) that drives the same public command API the Qt UI
drives, so UI actions, scripts, actions-panel steps, and batch jobs all funnel
through one command layer. This keeps the command surface testable and avoids a
second, drifting code path.

## Rust module mapping

Proposed workspace crates (names provisional). The workspace follows standard
Cargo workspace conventions: one root virtual manifest, shared `Cargo.lock` and
`target/`, and shared `workspace.dependencies` / `workspace.lints`.

| Crate (proposed) | Responsibility | Key types |
|---|---|---|
| `pictura-core` | Document model, layers/channels, selection, undo | `Document`, `Layer`, `Channel`, `Selection`, `TileId` |
| `pictura-color` | Color spaces, ICC transforms, soft proofing | `ColorSpace`, `Profile`, `ColorEngine` |
| `pictura-codec` | File read/write: PSD, PSB, TIFF, PNG, JPEG, EXIF/XMP | `Decoder`, `Encoder`, `PsdHeader` |
| `pictura-render` | Tile cache, scheduler, GPU compositor | `TileCache`, `Compositor`, `GpuDevice` |
| `pictura-filters` | Filter kernels, separable convolutions, gallery | `Filter`, `FilterRegistry`, `Kernel` |
| `pictura-script` | Scripting host and command API | `Command`, `ScriptHost` |
| `pictura-qt` (bridge) | cxx-qt bridge objects; the only Qt-dependent crate | `DocumentView`, `LayerModel` |
| `pictura-app` | Binary: Qt startup, wiring, main | `main` |

Verified dependencies (crate exists; doc URL listed in Sources):
`image`, `zune-jpeg`, `qoi`, `lcms2`, `wgpu`, `rayon`, `serde`, `half`,
`kamadak-exif`, `thiserror`, `anyhow`, `cxx-qt`, `qmetaobject`. Additions must be
justified against the workspace ladder (stdlib/native first).

## Qt6 component mapping

Proposed Qt6 surface. Widgets vs QML is decided per-region; see
`qt6-ui-design.md` (planned) for the full rationale. This doc only fixes the
architecture-relevant components.

| Component | Kind | Responsibility |
|---|---|---|
| `QApplication` / `QQmlApplicationEngine` | C++ | Host; one or the other depending on the shell decision. |
| `CanvasView` | `QQuickItem` (proposed) | Presents the Rust-composited texture; forwards input. |
| `LayerModel` | `QAbstractItemModel` | Layers/channels/paths, generated by cxx-qt. |
| `QRhi` / `QRhiTexture` | Qt GUI (private) | Backend selection and texture presentation. |
| `QThreadPool` / worker thread | Qt Core | Not used for image work; Rust `rayon` owns the pool. |

Decision: the canvas is proposed as QML/QQuickItem rather than `QWidget` because
Qt Quick is built on QRhi and supports scene-graph-provided textures with less
custom paint plumbing. Panels that are dense and table-like (Layers, Channels,
History) may still be `QWidget`-hosted via `QQuickWidget`, or re-implemented with
Quick Controls. This is a proposal.

## Data-model impact

Crossing the FFI boundary, the document is represented on the Rust side as an
owned, atomically-swappable handle. The Qt side holds a generation-stamped
`DocumentHandle` plus immutable snapshots for panels; it never owns pixel memory.

- **Undo granularity:** one undo record per user-visible command, stored as
  tiles touched + prior bytes, matching CS6's "smaller changes require less
  information per state" behavior. Snapshots are full-document copies.
- **Serialization:** the model maps to PSD/PSB sections (see `ARCH-002` and
  `file-formats.md`); 64-bit lengths and version marker 2 for PSB.
- **Tile cache:** keyed by `(layer_id, channel, level, TileId)`, LRU-evicted, and
  backed by the scratch manager below the RAM watermark.

## Edge cases

- **GPU unavailable:** every GPU-accelerated path must have a CPU fallback. CS6
  behaves this way ("in most cases the acceleration is lost and the feature runs
  in the normal CPU mode"), except for a documented set of features that "don't
  work without a supported video card." Kooka Pictura should degrade to CPU and
  clearly mark features that cannot run, rather than silently shipping wrong
  output.
- **GPU device loss:** `wgpu`/Vulkan device loss must be recoverable: flush the
  cache, re-create the device, re-upload visible tiles.
- **Wayland vs X11:** texture presentation and window management differ; see
  `ARCH-004`. Mixed-DPI multi-monitor is a known Wayland pain point.
- **Huge documents (PSB):** 300,000 px per dimension forces tiled, out-of-core
  processing; whole-image allocations are forbidden.
- **Scratch full:** must surface an actionable error (CS6 shows a scratch-full
  error) and stop cleanly, not corrupt documents.
- **Zero/one-pixel documents and empty layers:** tile cache and compositor must
  handle degenerate extents.
- **Thread starvation:** a long CPU filter must not block the GUI thread; the GUI
  thread must never wait on `pictura-core` without a timeout.

## Parity acceptance criteria

1. Given an 8-bit RGB document of 3000×2000 px, panning and zooming while a
   GPU is present sustains the frame budget defined in `ARCH-003`; with the GPU
   disabled, the app still completes the same gestures without error.
2. Given a document larger than available RAM, opening and editing it succeeds
   using a configured scratch volume, and the status bar Efficiency-equivalent
   drops below 100% rather than the app aborting.
3. Given `History States = 20`, 21 distinct pixel edits leave exactly 20
   reversible states plus the current state, and undo/redo is exact.
4. Given a 64-bit-only build, the address ceiling documented in `ARCH-003`
   (memory ceiling) is enforced and reported, not silently exceeded.
5. Given an unsupported/failed GPU probe, startup completes with GPU features
   disabled and a one-time notice, matching CS6's sniffer behavior.
6. Given a scripted command and the same command issued from the UI, both produce
   byte-identical document state (single command layer).

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — CS6 Help PDF; Mercury Graphics Engine description, GPU preferences, 64-bit performance note, scratch/efficiency readouts, Liquify GPU acceleration.
- `https://www.pugetsystems.com/labs/articles/Adobe-Photoshop-CS6-GPU-Acceleration-161` — MGE uses OpenGL + OpenCL; GPU benchmark methodology on a 38 MP image.
- `https://www.geeks3d.com/20120425/adobe-creative-suite-6-opencl-accelerated-mercury-graphics-engine-opengl` — direct MGE quote ("uses both the OpenGL and OpenCL frameworks… does not use… CUDA"); supported card families.
- `https://www.barefeats.com/pscs6.html` — Blur Gallery requires OpenCL 1.1; Advanced Settings > Use OpenCL toggle; CPU fallback observed with GPU off.
- `https://www.pugetsystems.com/labs/articles/Adobe-Photoshop-CS6-Memory-Optimization-182` — default 60% RAM allocation; peak RAM vs. image size table (109 MB→1024 MB).
- `https://web.archive.org/web/20140204041700/http://blogs.adobe.com/crawlspace/2012/10/how-to-tune-photoshop-cs6-for-peak-performance.html` — Adobe performance paper: 64-bit, ~3 GB 32-bit ceiling, 64 EB scratch across 4 volumes, History States default 20 (1–1000), Cache Levels default 4, 128 KB tiles, 256 MB VRAM, PSD 2 GB / PSB 4 EB, Efficiency metric.
- `https://web.archive.org/web/20140401152634/http://helpx.adobe.com/photoshop/kb/photoshop-cs6-gpu-faq.html` — MGE feature list, OpenGL 2.0 + SM3.0, OpenCL 1.1, 256/512 MB VRAM floors, GPU mode options, GPU Sniffer, Cache Levels default 4.
- `https://doc.qt.io/qt-6/wayland-and-qt.html` — Wayland vs X11, QPA selection, multi-process trade-offs, `EXT_platform_wayland`.
- `https://doc.qt.io/qt-6/qrhi.html` — QRhi backends (Vulkan/OpenGL/…), no resource sharing between instances, threading model, device import.
- `https://doc.rust-lang.org/cargo/reference/workspaces.html` — Cargo workspace conventions used by the proposed layout.
- `https://kdab.github.io/cxx-qt/book/` — cxx-qt safe Rust↔Qt bridge and build model.
- `https://paulbourke.net/dataformats/psdpsb/psdpsb.html` — Adobe PSD/PSB format: version 1/2, 30,000/300,000 px limits, 56 channels, depths 1/8/16/32, compression modes.
- crates.io API (`https://crates.io/api/v1/crates/<name>`) — verified existence/versions of the proposed dependencies.

## Open questions

- **Internal threading model of CS6** is undocumented. *Resolves with:* Adobe engineering disclosures or measured concurrency behavior; otherwise parity is behavioral only.
- **Exact tile pixel dimensions.** Only the 128 KB byte size is documented. *Resolves with:* analysis of the cache or community documentation; until then Kooka Pictura chooses its own tile size and validates against `ARCH-003` budgets.
- **Whether the Rust core should run in-process or as a subprocess** is a deferred decision. *Resolves with:* a prototype measuring pan/zoom and brush latency across an IPC boundary vs. a crash-isolation requirement.
- **QML vs Widgets for the canvas** is unresolved here. *Resolves with:* `qt6-ui-design.md` and a texture-sharing spike against QRhi.
- **Plugin ABI shape** (C ABI vs. C++/Qt plugin) is deferred. *Resolves with:* `plugin-and-scripting-abi.md`.
- **Scripting language** for the host is undecided. *Resolves with:* `rust-scripting-replacement.md`.
