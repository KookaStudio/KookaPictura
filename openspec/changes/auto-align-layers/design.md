# Design

## Context

See `proposal.md` — Why. `pictura-render` already exposes `align_layers`/`distribute_layers` (content-box translation), `layer_content_bounds`, `LayerTransform` (similarity), `transform_layer_quad` (projective), and `solve_homography`. `solve_homography` is a four-point minimal solver, not a general fit; `LayerTransform` is a representation/applicator, not a similarity estimator. There is no feature detection or registration code. `crates/pictura-app/src/cxxqt_object.rs` is at the 1200-line cap.

## Goals / Non-Goals

**Goals:**

- A deterministic, testable registration engine for translation, homography, and similarity cases.
- Reference selection, eligibility, and atomic application that match the documented behavior or label the inferred parts.
- One atomic history state and a cancel path that cannot half-apply transforms.
- A dialog that is honest about the unsupported modes.

**Non-Goals:**

- Cylindrical/Spherical warping and lens-profile correction.
- Matching CS6's `Auto` candidate set (Perspective/Cylindrical). The supported `Auto` is a reduced heuristic and is recorded as a ceiling.
- Byte parity with Adobe's unpublished matcher.
- Auto-Blend, Photomerge, or seam finding (separate issues).
- GPU acceleration; the CPU path is the oracle.

## Decisions

### D1: Separate registration from application

`auto_align/registration.rs` returns `Vec<LayerPlacement>` (path, map, residual) without touching the document. `auto_align/mod.rs` validates eligibility and applies the placements. This makes the matcher unit-testable on synthetic buffers and keeps the atomicity boundary clear.

### D2: Multi-scale normalized cross-correlation for translation

Each layer's content is reduced to a grayscale plane and a 3-level box-filtered pyramid with a working long edge of about 512 pixels. `Reposition Only` searches the coarsest level over the overlap window, keeps the best normalized cross-correlation score, refines at each finer level, and rounds to an integer translation. Ties break by the smallest `(dy, dx)` in a fixed order. Translation is applied with `offset_layer`, so no resampling occurs.

### D3: Harris corners, patch descriptors, and fitting

`Perspective` and `Scene Collage` detect Harris corners on the working-resolution luminance, describe each corner with a normalized 8×8 gradient patch, and match by NCC. Perspective uses a deterministic RANSAC of four-point hypotheses (`solve_homography` is the minimal solver), sorts matches before sampling, and refits the inliers by least squares/DLT. Scene Collage fits a similarity (Procrustes/Umeyama) from the same inliers, so shapes are not stretched. Both are new helpers; `LayerTransform` is only the application representation.

Application uses `transform_layer_quad` for Perspective and `LayerTransform` for the similarity. A pair with too few inliers falls back to the translation result and reports a lower confidence.

### D4: Reference and eligibility

A locked layer is the reference. Otherwise the reference is the eligible layer whose content centre is nearest the **document/composition centre**, not the selected-content union centre; the latter is not CS6's rule. The reference is never transformed. Groups, adjustment layers, fill/content layers, vector/type layers, Smart Objects, and the Background are skipped as movable layers; the Background may be the reference. A position-locked non-reference layer is skipped. A read-only `can_auto_align(doc, paths)` applies the same rules for the menu enablement.

### D5: Auto is a reduced supported-mode heuristic

`Auto` runs the translation fit first. If the residual is already below the acceptance threshold it keeps Reposition Only. Otherwise it runs Perspective and Scene Collage, compares residuals, and keeps the better supported map. It never uses Cylindrical because that mode is unsupported. The spec and UI call this a reduced Auto; the CS6 "Perspective or Cylindrical" behavior is a recorded ceiling.

### D6: Atomic application

Registration returns placements; application runs on a cloned `Document` (cheap, ref-counted planes) and swaps it in only after every placement applies successfully. A mid-apply refusal or a cancel therefore leaves the session document untouched. This is required because `transform_layer`/`transform_layer_quad`/`offset_layer` mutate in place and `PictureView::edit` does not roll back a closure that mutated before returning `Edit::None`.

### D7: Worker, progress, cancel, and bridge placement

The app copies the selected layers' rasters into a job, runs registration on a `std::thread`, and reports progress between pyramid levels and descriptor batches. The dialog is modal with a cancel button; cancel sets an atomic flag. The main thread applies the returned placements only if the job completed and was not cancelled.

`cxxqt_object.rs` is at the cap, so the job state, progress signal, and apply bridge are free functions in a new module (the `image_hdr_toning.rs`/`image_adjust/new_document.rs` pattern). No new invokables are added to the root.

### D8: Unsupported modes are explicit

`Cylindrical`, `Spherical`, and `LensCorrection` return `OpsError::Unsupported` and are disabled in the dialog with a reason. Lens Correction is an option with Vignette Removal/Geometric Distortion sub-options in CS6, not a projection; the dialog presents it in a separate option group and disables it. The unsupported set is recorded in the spec/proposal, a parity row, and a follow-up issue.

## Risks / Trade-offs

- **Low-texture or non-overlapping layers produce weak matches** → Accept the translation fallback only when its residual is finite and above the confidence floor; otherwise return a refusal with no mutation and show a message.
- **Feature matching is slow on large layers** → Work at a bounded resolution, parallelize descriptor matching with `rayon`, and compute on a worker.
- **A homography can distort a layer unexpectedly** → Perspective is restricted to well-spread inliers and a sanity check on the mapped content box; Scene Collage uses a similarity by construction.
- **Resampling changes pixels** → Reposition Only avoids resampling; Perspective/Scene Collage resample once with the existing bilinear path. Add a save/reopen test because a raster layer's transform is baked on save.
- **Determinism vs. a random RANSAC** → Fixed-seed xorshift and sorted matches; a repeat test asserts identical output.
- **Locked and background layers** → The reference stays byte-identical; a position-locked non-reference is skipped; the Background never moves.
- **Projection accuracy has no CS6 oracle** → The spec claims synthetic translation/quad accuracy and determinism only; the unsupported modes and reduced Auto are explicit ceilings.
