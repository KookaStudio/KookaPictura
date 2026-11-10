# Design: auto-align-layers

## Context

`LAY-030` defines the behavior but leaves the matcher unpublished. The document
model stores raster layers as channel planes in document-space rects; there is
no live per-layer transform. Existing transform ops (`transform_layer_with`,
`transform_layer_quad`) resample destructively and the app records one history
snapshot per `edit`. Auto-Blend already establishes the worker + plan + apply
pattern this change follows.

## Engine

### Module layout

`crates/pictura-render/src/document_ops/layer_ops/auto_align/`

| File | Responsibility |
|---|---|
| `mod.rs` | Public types, eligibility/reference selection, `can_auto_align`, `plan_auto_align`, `apply_auto_align`, progress/cancel. |
| `image.rs` | `WorkImage`: bounded-scale luma/alpha view of one layer, optionally sampled through a `LensMap`. |
| `features.rs` | Harris corners, dominant orientation, SIFT-lite 4×4×8 descriptors, ratio-test matching. |
| `estimate.rs` | RANSAC translation/similarity/homography, normalised DLT refinement, overlap scoring. |
| `projection.rs` | Cylindrical/spherical forward and inverse maps and the `PlaneMap` used to apply them. |
| `lens.rs` | Bake a `LensCorrectionParams` correction into a layer through `LensSource`/`LensMap`. |
| `tests.rs` | Unit tests. |

### Public API

```rust
pub enum Projection {
    Auto, Perspective, Cylindrical, Spherical, SceneCollage,
    RepositionOnly, LensCorrection,
}

pub struct AutoAlignOptions {
    pub projection: Projection,
    pub vignette_removal: bool,
    pub geometric_distortion: bool,
    /// Resolved by the app from EXIF + installed profiles; `None` skips it.
    pub lens: Option<pictura_filters::LensCorrectionParams>,
}

pub struct AutoAlignPlan {
    pub reference: String,
    pub warps: Vec<LayerWarp>,
    pub notice: Option<String>,
}

pub enum LayerWarp {
    Identity { path: String },
    Translate { path: String, dx: f64, dy: f64 },
    Similarity { path: String, xx: f64, xy: f64, yx: f64, yy: f64, tx: f64, ty: f64 },
    Homography { path: String, h: [f64; 9] },
    Projected { path: String, kind: Projection, h: [f64; 9], focal: f64, centre: [f64; 2] },
}
```

`can_auto_align(doc, paths)` is true with at least two eligible layers with
content. `plan_auto_align` only reads, so it runs on a worker over a clone.
`apply_auto_align` bakes the optional lens correction and then each warp inside
the caller's single `edit`.

### Eligibility and reference

Eligible: a visible, non-group, non-adjustment, non-fill, non-type, non-shape,
non-Smart-Object, non-clipped raster layer with content and not the Background.
`layer_content_bounds` supplies the box round the non-transparent pixels.

Reference: among eligible layers, a layer with `LockFlags::POSITION` wins;
otherwise the content centre nearest the union centre. Ties break by stack
order. The reference gets `Identity` and is never moved. Any other
position-locked layer is also left still (it cannot be the reference if a
locked layer already is).

### Matching pipeline

1. Build one `WorkImage` per eligible layer. `scale = ceil(max(w,h)/640)`, so a
   large document matches on a bounded grid. Luma is Rec.601 from channels 0–2
   (channel 0 alone for Grayscale, mean of the first three otherwise); alpha is
   channel −1 when present, else opaque.
2. Harris response on the luma: central-difference gradients, a 3×3 box
   structure tensor, `R = det − 0.04·trace²`, a relative threshold, 3×3
   non-maximum suppression, and the strongest 800 corners at least 12 px from
   the work-image edge.
3. Descriptor: the dominant gradient orientation in a 16×16 window, then a
   4×4 grid of 8-bin orientation histograms sampled in the rotated frame,
   normalised and clamped (SIFT-lite). Rotation invariance keeps Perspective and
   Scene Collage usable when the camera rotates.
4. Match moving descriptors to the reference with an L2 nearest/second-nearest
   ratio of 0.75. Fewer than four matches fall back to a translation vote; a
   still-empty match set yields no warp and a notice.
5. Estimate:
   - Reposition Only: RANSAC over 1-point translation hypotheses, inlier
     threshold 3 work pixels.
   - Scene Collage: RANSAC over 2-point similarity hypotheses, then a
     least-squares similarity refit on the inliers.
   - Perspective and Lens Correction: RANSAC over 4-point homographies using
     `Homography::from_quads`, then a normalised DLT refit on all inliers.
   - Cylindrical/Spherical: project both point sets into the common cylinder or
     sphere about the union centre with `f = 1.2·max(union w, union h)`, then
     estimate a homography there. The applied map composes source→projection→H→
     reference projection→reference plane.
   - Auto: run Perspective and Cylindrical, score both by normalised
     cross-correlation over a sampled overlap grid, and keep the higher score.
6. Convert work-image coordinates back to document coordinates and store the
   warp. Translation and similarity are expressed in document space; a
   homography maps document-space source points to document-space reference
   points.

### Applying

`apply_auto_align` returns the paths it changed.

- `Identity` changes nothing (except the optional lens correction).
- `Translate`/`Similarity` go through `transform_layer_with` with a
  `LayerTransform`; an integer translation stays a pure translation.
- `Homography` computes the destination quad from the layer rect's four corners
  and calls `transform_layer_quad`.
- `Projected` calls a new `pub(super)` `transform_layer_with_map` wrapper around
  the existing private `apply_layer_map`; the wrapper accepts any `PlaneMap`, so
  the cylindrical/spherical composition reuses the same resample, mask, native
  plane, and write-back path.

Lens correction is baked first with `LensSource::render` at 1:1 through a
`LensMap` built from the resolved profile and the layer's rect size. The output
is premultiplied RGBA; it is unpremultiplied and written back to the layer's
channels before the alignment warp. This keeps one history state and matches
the destructive-resample ceiling.

### Cancellation and determinism

`plan_auto_align` takes `&AtomicBool` and `&AtomicU32` like Auto-Blend. Every
stage checks cancellation; progress reaches 1000 only on a complete plan.
Corners, matches, and RANSAC iterate in index order with no randomness beyond a
fixed-seed xorshift for hypothesis sampling, so repeated runs on the same
document produce the same plan.

## App

### Bridge

`crates/pictura-app/src/cxxqt_object/auto_align.rs` + `auto_align/imp.rs`
mirror Auto-Blend:

- `auto_align_can`, `auto_align_start(paths, projection, vignette, geometric)`,
  `auto_align_running`, `auto_align_progress`, `auto_align_cancel`,
  `auto_align_finish`, `auto_align_notice`, `auto_align_set_delay_for_test`.
- `auto_align_start` clones the document, resolves the lens parameters through a
  new `filter_tools` helper, spawns the worker, and records the session content
  revision. `auto_align_finish` drops a stale plan and otherwise calls
  `view.edit("Auto-Align Layers", …)`.
- The bridge resolves the lens parameters from `pictura_codec::read_metadata` and
  the installed profiles, gating the profile's distortion/vignette parts by the
  dialog's checkboxes. No profile means `None` and a notice.

### Dialog and menu

`AutoAlignDialog` (C++ `auto_align_dialog.{h,cpp}`) uses seven radio buttons and
two checkboxes. Object names: `autoAlignAuto`, `autoAlignPerspective`,
`autoAlignCylindrical`, `autoAlignSpherical`, `autoAlignCollage`,
`autoAlignReposition`, `autoAlignLens`, `autoAlignVignette`,
`autoAlignGeometric`. The sub-options enable only for Lens Correction.

`frame_menus_align.cpp` registers `command_ids::EditAutoAlignLayers` beside
Auto-Blend, opens the dialog, and runs `runAutoAlign` through
`runCancellableJob`, selecting the moved layers afterwards. The command id is
added to the Rust catalogue, `const_names`, `modal_ids`, and the generated
`command_ids.h`.

## Risks and mitigations

- **Matcher accuracy without an oracle.** Tests use synthetic transforms with
  known ground truth and assert a tolerance, not identity; the design marks the
  algorithm behavioral parity only.
- **Projection focal length is unknown.** The inferred `1.2·max` keeps wide
  panoramas stable; the result is still approximate and documented.
- **Lens profiles may be absent.** The app reports a skip; no pixels change.
- **Large documents.** Matching is bounded to a 640 px work grid; applying the
  transform resamples at document resolution as the existing transform ops do.
- **History size.** Destructive resampling stores one snapshot per run, the same
  as Free Transform and Auto-Blend.
