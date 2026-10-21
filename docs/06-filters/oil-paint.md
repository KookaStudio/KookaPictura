# Oil Paint

- **Spec ID:** `FILT-091`
- **Status:** `Draft`
- **Parity tier:** `Core` — `Filter > Oil Paint` is a CS6 top-level filter in both Standard and Extended. It is the only CS6 filter that is **gated on a supported GPU**; on a machine without one it is unusable (see `## CS6 behavior`).
- **New in CS6:** `Yes` — the Oil Paint filter is new in CS6. Later releases moved it to `Filter > Stylize > Oil Paint` and changed the GPU stack (OpenCL 1.1+, macOS Metal); CS6 exposes it directly under `Filter > Oil Paint`.
- **Depends on:** `FILT-001` filters-overview, `ARCH-006` gpu-rendering-pipeline, `ARCH-007` color-management, `ARCH-009` undo-history, `LAY-021` smart-filters, `LAY-020` smart-objects, `IMG-005` bit-depth-and-conversion.

> All module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Adobe's Oil Paint is a GPU (OpenCL) effect whose
> shader is closed; the algorithm description is **behavioral parity only,
> algorithm TBD** and any kernel family named below is *(inferred)* from the
> documented control behaviour, not from Adobe source.

## CS6 behavior

`Filter > Oil Paint` produces the look of a classic painting. The dialog offers **Brush** and **Lighting** controls (the CS6 Help suggests experimenting with the Brush and Lighting options and documents no further detail in the fetched PDF). The CS6 Help's only operational caveat is explicit:

> If the Oil Paint filter is not working, your graphics card may be unsupported or its driver may be out of date.

Adobe's GPU FAQ lists Oil Paint under **"GPU enhancements added in Photoshop CS6"** as **"requires a compatible graphics card"** — i.e. unlike Liquify (merely accelerated) and Blur Gallery (OpenCL-accelerated), Oil Paint **does not fall back to a CPU renderer**; without a supported card/driver the filter is **disabled or errors out**. A later Adobe Help page states the requirement precisely: **OpenCL v1.1 or higher**; on macOS 10.11+ with AMD GPUs, Apple's **Metal** framework can be used instead via `Preferences > Performance > Use Graphics Processor > Advanced > Use Native Operating System GPU Acceleration`.

The effect applies to the **active, visible layer or selection**. The CS6 Help's 16-bpc and 32-bpc filter lists **do not include Oil Paint**, which indicates **8-bpc-only** in CS6 (the lists are stated as exhaustive for those depths; see `## Open questions`).

Menu path history: CS6 and the CS6-era Help use the top-level **`Filter > Oil Paint`**; the modern (CC) Help uses **`Filter > Stylize > Oil Paint`**. The spec targets the CS6 path.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Oil Paint` | menu, dialog | — | CS6 top-level; later `Filter > Stylize > Oil Paint` |
| Dialog — Brush sliders | sliders | — | Stylization, Cleanliness, Scale, Bristle Detail |
| Dialog — Lighting sliders | sliders | — | Angular Direction (angle), Shine |
| Dialog — Preview | canvas | — | Full preview of the stylized result |
| Dialog — OK / Cancel | buttons | — | Commits one history state |
| `Edit > Fade Oil Paint` | menu, dialog | `Shift+Ctrl/Cmd+F` *(inferred)* | Opacity + mode after commit |
| Preferences — Graphics Processor | pane | — | `Use Graphics Processor`; `Advanced` → OpenCL / Metal (post-CS6) |

## Parameters & ranges

The CS6 Help PDF names only "Brush and Lighting options." The control **names, meanings, and 0–10 ranges** below come from the fetched Adobe Help page (CC-era, but describing the same Oil Paint parameters) plus community tutorials; defaults are **unverified**.

| Group | Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|---|
| Brush | Stylization | float | *(unverified)* | 0–10 | Stroke style: daubed at 0, smooth at 10. *(Adobe Help)* |
| Brush | Cleanliness | float | *(unverified)* | 0–10 | Stroke length: shortest and choppiest at 0, longest and most fluid at 10. *(Adobe Help)* |
| Brush | Scale | float | *(unverified)* | 0–10 | Paint relief / apparent thickness: a thin coat at 0, thick Van Gogh-style globs at 10. *(Adobe Help)* |
| Brush | Bristle Detail | float | *(unverified)* | 0–10 | Visibility of paintbrush-hair indentation: soft at 0, strong grooves at 10. *(Adobe Help)* |
| Lighting | Angular Direction | float ° | *(unverified)* | 0–360 (angle) | Light incidence angle (independent of the brushstroke). *(Adobe Help)* |
| Lighting | Shine | float | *(unverified)* | 0–10 | Light-source brightness and how much light bounces off the paint surface. *(Adobe Help)* |
| GPU | Graphics processor | bool | on where supported | on / off | Filter **requires** a supported card; OpenCL 1.1+ (CC doc) |

Community CS6 examples use mid-range values (e.g. Stylization 3.5, Cleanliness 4.5, Scale 0.75, Bristle Detail 3, Angular Direction 85, Shine 0.55) and Adobe's own tutorial recommendation uses Stylization 8.96, Cleanliness 3.5, Scale 8.96, Bristle Detail 2.2, Angular Direction 244.8, Shine 0 — both are *examples*, not defaults.

## Algorithms & pipeline

Behavioral parity only; the OpenCL kernel is closed. The documented control semantics constrain a plausible family:

- **Stylization** controls stroke *shape/regularity* (daubed → smooth): a painterly edge-following smoothing where stroke orientation follows image structure ("respecting the image's edges", per a community description), with higher values giving longer, smoother, more coherent strokes.
- **Cleanliness** controls stroke *length*: short/choppy → long/fluid. This is the scale over which a stroke's color is aggregated before deposition.
- **Scale** controls the *apparent paint thickness/relief*: the size of the brush footprint and the magnitude of the lighting relief applied to it.
- **Bristle Detail** controls *bristle-hair indentation*: high-frequency groove structure added to the stroke normal.
- **Angular Direction** sets the light azimuth (independent of stroke direction).
- **Shine** sets the specular strength / light-source brightness bounced off the paint.

A kernel family consistent with all of the above is an **edge-aware directional smoothing plus a local shading pass** *(inferred)*:

```text
# per pixel, in linear working space
1. estimate local structure tensor / gradient orientation
2. aggregate colours along the stroke direction over a stylization-scaled,
   cleanliness-scaled kernel (edge-stopping so strokes do not cross strong edges)
3. quantize/relief: build a height field from luminance + "scale" thickness +
   "bristle" high-frequency ridged noise
4. shade the height field with a Lambert + Blinn-Phong model using
   (angular_direction, shine) -> normal-perturbed lighting
5. composite back
```

This is unrelated to Photoshop's older Filter Gallery "Dry Brush"/"Palette Knife" filters; Oil Paint is a separate GPU effect.

**GPU requirement (hard).** No supported card ⇒ no Oil Paint. A reimplementation should decide deliberately: either match CS6 and **disable the filter**, or provide a `wgpu` compute path and, unlike CS6, a CPU fallback. Matching CS6 means the parity build has no CPU fallback; a portability fallback must be clearly labelled a Linux/non-parity extension.

## Rust module mapping

Proposals. Oil Paint is a whole-layer GPU filter with a small parameter set, so it reuses the generic GPU filter dispatch and adds one compute pass graph.

- `pictura_filters::oil_paint` — `OilPaintParams { stylization, cleanliness, scale, bristle_detail, angular_direction, shine }` (serde, for smart-filter blobs); `fn apply(&Document, Rect, &OilPaintParams, &mut Backend) -> FilterResult`.
- `pictura_filters::oil_paint::structure` — structure-tensor / edge-orientation estimation.
- `pictura_filters::oil_paint::shade` — relief + Blinn-Phong lighting from the parameter set.
- `pictura_render::oil_paint` — `wgpu`/compute pipeline (or a Qt RHI shader) for the filter; if CS6 strictness is required, gate dispatch on `GpuCapabilities::opencl_compute` and return `FilterError::UnsupportedGpu` otherwise.
- `pictura_filters::backend` — shared `enum Backend { Cpu, Gpu }`, capability probe, and the CS6-style "requires supported GPU" gate.

Crossing types: `OilPaintParams`, `Rect` (region), `GpuCapabilities`, `FilterResult`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `OilPaintDialog` | `QDialog` | Sliders for the four Brush and two Lighting controls; live preview; OK/Cancel |
| `OilPaintPreview` | `QLabel`/`QRhiWidget`/`QImage` view | Shows the stylized result; falls back to a static preview if the GPU pass is unavailable |
| `GpuUnsupportedNotice` | `QMessageBox` / inline banner | CS6-style message when no supported graphics processor is available |
| `FilterPreviewScheduler` | `QObject` | Debounces slider changes and submits the GPU pass off the GUI thread |

Widgets, not QML: a modal slider dialog with a live preview matches the existing filter dialogs (`ARCH-003`).

## Data-model impact

- **Destructive by default** in CS6 (one history state on OK, holding the pre-filter tile snapshot).
- **Smart filter:** CS6 does **not** support Oil Paint as a smart filter (only Blur Gallery/Liquify gained that in CC). Store `OilPaintParams` in the filter record so a future smart-filter extension can re-edit it (`LAY-021`).
- **GPU capability** is app/session state, not document state; do not serialize it.
- **No new document nodes.**

## Edge cases

- **No supported GPU / out-of-date driver** — CS6 shows an error and the filter does not run. Decide and document the Linux policy (disable vs CPU fallback). This is the single most important parity edge case.
- **8-bit only (per the CS6 lists)** — on 16-/32-bpc documents, grey out the filter or convert; confirm in `## Open questions`.
- **Mode** — filters cannot run on Bitmap/Indexed; whether Oil Paint is RGB-only is not stated in the fetched CS6 source.
- **Empty/1-px documents** — no meaningful effect; avoid creating history states on no-ops.
- **Huge (PSB) documents** — the GPU pass must be tiled / memory-bounded; watch VRAM.
- **Selection** — confine the pass to the selection (with the shared filter apron for any neighborhood reads).
- **Undo/redo** — one atomic state per OK; Cancel touches nothing.
- **Color management** — decide whether stylization/relief run in linear light or gamma space; Adobe's behaviour is not documented (see `## Open questions`).

## Parity acceptance criteria

1. Given a supported GPU, applying Oil Paint with default parameters produces a painterly result: flat regions become brush-stroke patches whose orientation follows image structure, while strong edges are broadly preserved.
2. Increasing **Stylization** (0→10) monotonically increases stroke smoothness/regularity; **Cleanliness** increases stroke length; **Scale** increases apparent paint thickness; **Bristle Detail** increases fine groove contrast.
3. Changing **Angular Direction** rotates the apparent lighting around the surface at a fixed stroke layout; increasing **Shine** increases specular highlight intensity.
4. Given no supported GPU, the CS6-parity build **disables** the filter (matching CS6) and shows the GPU notice; a fallback build is clearly labelled non-parity.
5. Given identical parameters and input, output is deterministic across runs (same GPU backend).
6. Given a selection, pixels outside it are bit-identical to the input.
7. Given a commit, exactly one new history state appears and `Ctrl+Z` restores the pre-filter pixels bit-exactly.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — official CS6 Help reference (downloaded to `/tmp`, `pdftotext`-extracted). Established: the CS6 What's-New "Oil Paint filter" entry (recommending `Filter > Oil Paint` for a classic-painting look); the Oil Paint page (the suggestion to experiment with Brush and Lighting; the "supported graphics card / out-of-date driver" caveat); the exhaustive 16-bpc and 32-bpc filter lists that **exclude** Oil Paint.
- `https://web.archive.org/web/20170913004356id_/https://helpx.adobe.com/photoshop/using/oil-paint-filter.html` — Adobe Help snapshot (wayback; live page 403). Established the six parameter names and their documented 0–10 semantics (Stylization, Cleanliness, Scale, Bristle Detail, Angle/Angular Direction, Shine), the modern `Filter > Stylize > Oil Paint` path, the **OpenCL v1.1+** requirement, and the macOS Metal option.
- `https://topic.alibabacloud.com/a/photoshop-cs6-gpu-faq_8_8_10184243.html` — mirror of the **Photoshop CS6 GPU FAQ**. Established that the Mercury Graphics Engine uses OpenGL + OpenCL (not CUDA), and that **Oil Paint requires a compatible graphics card** (alongside Adaptive Wide Angle), whereas Liquify/Warp/Blur Gallery are accelerated.
- `https://the-digital-photography-school.com/new-oil-paint-filter-in-photoshop-cs6` and `https://www.digigalaxy.net/tutorials/oilpainting.html` (surfaced via search) — community CS6-era descriptions and example values; secondary, used only for illustrative values, not asserted as defaults.
- `https://www.shutterstock.com/blog/tutorial-using-photoshops-new-oil-paint-filter-and-skin-tone-selection-tool` — community description of Angular Direction and Shine; secondary.
- SearXNG meta-search (queries: "Photoshop CS6 Oil Paint filter Stylization Cleanliness Scale Bristle Detail Lighting Angle Shine", "Photoshop CS6 GPU FAQ") — used to locate the above; no facts asserted from snippets alone.

Not parsed in this pass: `helpx.adobe.com` live pages (HTTP 403); Adobe's Oil Paint shader (proprietary).

## Open questions

- **All Oil Paint defaults.** The fetched CS6 source does not state them; Adobe's and community examples are not defaults. Resolves with: a first-run CS6 dialog capture.
- **Exact ranges/steps.** Adobe documents 0–10 for the brush/shine sliders but not the increment; Angular Direction is an angle but its range/format is unconfirmed. Resolves with: a CS6 capture.
- **8-bpc-only confirmation.** Inferred from the CS6 16-/32-bpc lists excluding Oil Paint. Resolves with: a depth test in CS6.
- **Colour model support** (RGB-only? CMYK/Lab?) and the working space used for relief/lighting. Resolves with: CS6 mode/depth tests and Adobe technical notes.
- **Kernel family.** Adobe's shader is closed. A structure-tensor + edge-stopping aggregation + relief-shading model is *(inferred)*. Resolves with: reference-image fitting against CS6 output, or an acceptance tolerance.
- **Linux fallback policy.** CS6 hard-requires a GPU; a Linux build needs a CPU path for machines without one. Decide whether this is a labelled extension or the filter is unavailable. Resolves with: architecture/GPU-pipeline decision (`ARCH-006`).
- **Determinism across GPUs.** Whether the effect should be reproducible on different vendors. Resolves with: a determinism requirement and a golden-image harness.
