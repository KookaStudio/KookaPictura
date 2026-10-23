# Design

## Context

See `proposal.md` — Why. The current conversion path is a two-stage contract: `pictura-adjust::hdr_toning::exposure_gamma` maps f32 samples, and `pictura_render::document_ops::depth::convert_depth_exposure_gamma` owns the document gate, quantization, store rebuild, and layer-store clearing. The C++ dialog and the `PictureView::convert_depth` invokable carry only Exposure and Gamma.

Two constraints shape the approach. `pictura-adjust` depends only on `pictura-core`, while the nine-parameter `HdrToningParams` lives in `pictura-filters`; the two crates cannot share it without a common home. And `crates/pictura-app/src/cxxqt_object.rs` is exactly 1200 lines (the code cap), so a new 11-argument invokable there is not viable.

## Goals / Non-Goals

**Goals:**

- Four selectable conversion methods with one document gate and one quantization path.
- Local Adaptation on unbounded f32 input with the same parameter ranges as the HDR Toning command.
- One shared numeric-control widget so the HDR Toning dialog and the HDR Conversion dialog cannot drift.
- Correct conversion of an edited 32-bit document (current composite, not the stale retained store).
- Existing Exposure & Gamma output remains byte-identical.

**Non-Goals:**

- Claiming CS6-identical kernels for Equalize Histogram, Highlight Compression, or Local Adaptation. Only Exposure & Gamma has a fixed formula.
- The Local Adaptation Toning Curve and Corner option. This is a recorded ceiling; the nine numeric controls are the supported surface.
- Changing the HDR Toning command's 8-bit operator or its presets.
- Adding a live preview to the conversion dialog; the dialog is modal and applies once.
- 32-bit Lab/CMYK sources. CS6 has no 32-bit Lab/CMYK mode; the existing gate stays.

## Decisions

### D1: Share the parameter type through `pictura-core`

Move `HdrToningParams` and its validation to `pictura-core` (a small `hdr` module) and re-export it from `pictura-filters` so existing callers keep compiling. `pictura-adjust` and `pictura-filters` then use one range definition and one validation rule. This is a pure move; the 8-bit filter's behaviour is unchanged.

Alternative considered: duplicate the struct and ranges in `pictura-adjust` and add a cross-crate consistency test. Rejected because the ranges would drift the first time a preset changes.

### D2: One method enum, thin wrappers at the document layer

`HdrToneMap` carries the method and its parameters. `convert_depth(doc, out, method)` is the new entry point; `convert_depth_exposure_gamma` remains as a wrapper that constructs `HdrToneMap::ExposureGamma`, so the existing tests and any engine caller keep their contract. The gate, quantization, store rebuild, and `drop_layer_source` stay in one function. A layered document is flattened to one Background built from the tone-mapped composite: the app recomposites after the conversion, and a preserved layer tree would resurrect the pre-conversion pixels. The converted Background keeps the 16-bit store's native color planes, so a save does not widen them.

### D3: Only the nine numeric rows are shared

Extract the nine numeric slider/spin rows into `HdrToningControls` with a signal for edits and a getter/setter for the values. The HDR Toning dialog keeps its preset combo and live preview; the HDR Conversion dialog hosts only the numeric rows. The proposal's "shared control widget" refers to the numeric rows, not the preset or preview machinery.

### D4: Defaults match the HDR Toning `Default` preset

The conversion dialog opens on Local Adaptation with the same values as the HDR Toning dialog's `Default` preset (`radius 187`, `strength 4.0`, `gamma 0.99`, `exposure 0`, `detail 30`, `shadow 0`, `highlight 0`, `vibrance 0`, `saturation 20`). These values are inferred CS6 defaults; the docs' `gamma 1.0`/`saturation 0` figures are also inferred, so the change keeps the two dialogs consistent rather than inventing a second default set.

### D5: Inferred kernels are explicit in code and spec

- Equalize Histogram: luminance CDF over finite positive samples; apply the per-pixel luminance gain to RGB. A uniform input returns the input unchanged rather than dividing by a zero span.
- Highlight Compression: knee at `0.8`; `y = x` below the knee, `y = 0.8 + 0.2*(1 - exp(-(x-0.8)/0.2))` above it. Values above white approach `1.0` without a hard clamp.
- Local Adaptation: port the existing log-domain split (blurred base, gated detail, pivot compression, shadow/highlight, filmic shoulder) to f32. The parameter ranges are the shared validated ranges.

Each function carries a comment naming the approximation. The spec scenarios assert observable behaviour, not byte parity.

### D6: Free-function bridge, not a wider invokable

`cxxqt_object.rs` is at 1200 lines and the cap is hard. A new conversion bridge lives in a free-function module (the `image_hdr_toning.rs` / `image_adjust/new_document.rs` pattern) with the method and parameter scalars. The existing `convert_depth(bits, exposure_ev, gamma)` invokable stays as the three-argument Exposure & Gamma convenience wrapper, so `tst_image_mode.cpp` and the self-test keep compiling unchanged. The dialog calls the free-function bridge.

### D7: Current-source freshness

The retained store is a clean-open snapshot; ordinary edits update the composite, and the store is refreshed only on save, PDF export, or recovery. `convert_depth` therefore uses the retained `f32` store only while it is current, and the current composite otherwise. The signal is the store's 8-bit narrowing compared with `doc.composite`: a match means the store still describes the document, a mismatch means an edit recomposited it. The open path must not overwrite that merged composite with the display recomposite, or a clean 32-bit open would be misread as edited and lose its values above white; the open decode keeps the file's merged composite and lets the display path recomposite layered documents.

Alternative considered: always use the retained store (current behaviour). Rejected because an edit-then-convert would tone-map pre-edit pixels.

### D8: Toning Curve is a recorded ceiling

The docs describe a Toning Curve over an EV histogram and a Corner option for Local Adaptation. The existing HDR Toning dialog also omits it. This change does not add a second curve editor; the spec, proposal, and parity register record the omission, and a follow-up issue tracks it. The nine numeric controls are not described as "the whole" CS6 Local Adaptation surface.

## Risks / Trade-offs

- **Two Local Adaptation implementations (f32 conversion vs 8-bit HDR Toning) can diverge visually** → The shared parameter type/ranges and the shared numeric controls keep the inputs aligned; both get their own tests.
- **Histogram equalization can produce harsh results on synthetic images** → Keep the CDF monotone and finite; the spec only claims order preservation and finiteness.
- **The local adaptation f32 path may be slow on large documents** → Reuse `pictura_core::blur::approx_gaussian`; no full-document clone beyond the existing source samples.
- **The HDR Toning dialog refactor could regress presets or preview** → The existing `tst_image_adjustments` HDR Toning cases and the selftest HDR block must stay green; the extraction is a move of the numeric rows only.
- **Dialog growth pushes files over the cap** → `hdr_toning` becomes a directory module; the conversion dialog stays in its own files; the shared numeric rows get their own files and CMake entries.
- **Values above white are already lost on an edited 32-bit document** → This is the existing documented ceiling; the new freshness rule at least prevents tone-mapping the wrong (pre-edit) pixels.
- **The Toning Curve omission** → Explicit ceiling + follow-up issue; never claim full CS6 Local Adaptation parity.
