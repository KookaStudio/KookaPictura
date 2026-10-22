# Proposal

## Why

`Layer > Align` and `Distribute` translate layers against each other by their content boxes, but Photoshop's `Edit > Auto-Align Layers` matches overlapping content and transforms layers into register. The parity audit records the command as missing (`LAY-4`, issue #582), and the existing align-distribute change explicitly left it out.

## What Changes

- Add `Edit > Auto-Align Layers…` with a projection dialog and a content-based registration engine.
- Support four modes: `Auto`, `Reposition Only`, `Perspective`, and `Scene Collage`. `Cylindrical`, `Spherical`, and the `Lens Correction` option are offered as disabled entries with an explicit unsupported reason; their projection math and lens-profile data are outside this change. `Auto` is a **reduced heuristic** over the supported modes, not CS6's Perspective/Cylindrical choice; this is recorded as a ceiling.
- Choose a reference layer: a locked layer if one is selected, otherwise the eligible layer nearest the centre of the final composition (the document canvas). The reference keeps its transform.
- Skip groups, adjustment layers, fill/content layers, vector/type layers, Smart Objects, and the Background as movable layers; a Background may only be the reference. A position-locked non-reference layer is skipped rather than moved.
- Apply every computed placement to a candidate document and commit once, so a refusal or cancel cannot leave a partially aligned stack.
- Record exactly one `Auto-Align Layers` history state; a cancelled run or a refused registration records none.
- Run the registration on a worker with a cancelable progress dialog; the job state and progress bridge live outside the capped `cxxqt_object.rs`.

## Capabilities

### New Capabilities

- `document/auto-align`: content-based layer registration, its supported mode set, reference selection, eligibility rules, atomic application, and history/cancel behavior.

### Modified Capabilities

None.

## Impact

- `crates/pictura-render`: new `document_ops/layer_ops/auto_align/` module (pyramid, translation search, corner matching, homography/similarity fitting and refit) and its tests. Reuses the existing transform/homography and content-bounds helpers.
- `crates/pictura-app`: `Edit > Auto-Align Layers…` command id and menu wiring, a new `AutoAlignDialog`, a free-function bridge plus auto-align job state, and one history state.
- New `tst_auto_align` suite; no change to `tst_align_distribute`.
- No new dependencies; `rayon` is already used by `pictura-render`.
- Pre-implementation oracle: synthetic translation/quad tests only. Projection accuracy is behavioral, not CS6 byte parity. The unsupported modes and the reduced Auto are documented ceilings with a follow-up issue/ROADMAP item.
