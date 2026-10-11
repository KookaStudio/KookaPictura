# Tasks

## 1. Output-size seam (Rust + bridge)

- [x] 1.1 Add `perspective_crop_sized(doc, quad, w, h)` to `pictura-render` (0/negative falls back to `perspective_crop_size`) and make `perspective_crop` delegate with 0; verify a Rust test that an explicit size wins and 0 matches `perspective_crop_size`
- [x] 1.2 Add the `perspective_crop_commit_sized(view, quad, w, h)` declaration in `crop_group.rs` and its impl (pass-through to `perspective_crop_sized`, commit/record unchanged); verify the app builds

## 2. Four-click staging + snapping + rotate (`tool_perspectivecrop.cpp`)

- [x] 2.1 Replace drag-draw with click placement: each press adds the next corner (TL,TR,BR,BL), closing on the fourth; verify with a `tst_perspective_crop` case that four clicks stage a quad and no commit happens
- [x] 2.2 Snap each placed/dragged corner to the canvas edges/corners and composite bbox within 6 screen px; verify a corner clicked near a canvas corner lands exactly on it
- [x] 2.3 Add the rotate gesture and cursor outside the completed quad (rotate the quad about its centre, reuse `crop_grip.h`); verify the cursor kind and that a rotate drag changes the quad
- [x] 2.4 Commit with the options-bar W/H override (blank → 0/auto); verify Enter/Apply still records one state

## 3. Overlay (`image_view_overlays.cpp`)

- [x] 3.1 Draw the placed points for an incomplete quad and gate the shield/edges on a complete quad; verify a partial quad draws points and no shield
- [x] 3.2 Shade only the canvas rect (not `viewRect()`), matching the rectangular crop; verify the workspace is not dimmed
- [x] 3.3 Draw the grid at a 1-inch spacing from the document resolution, clipped to the quad; verify a larger quad yields more lines

## 4. Options bar

- [x] 4.1 Add a Perspective Crop options page (W, H, swap, Resolution, unit, Front Image, Clear, Show Grid) and route `ToolId::PerspectiveCrop` to it; verify the controls exist via `tst_perspective_crop`
- [x] 4.2 Show Cancel/Apply icon buttons only while the quad is complete; verify their visibility follows the quad state
- [x] 4.3 `Clear` empties W/H only; `Front Image` sets the resolution; verify with a test

## 5. Docs + verification

- [x] 5.1 Update `docs/03-tools/perspective-crop.md` for four-click staging, rotate, snapping, grid, shield scope, and the options bar (carries `TASK-ALLOWS-DOCS`)
- [x] 5.2 Run `openspec validate --all --strict`
- [x] 5.3 Run `bash scripts/verify-fast.sh`

## 6. Follow-ups

- [x] 6.1 Paint a staged corner from the first click (not only once the quad completes), keeping the canvas fade at four; cover in `tst_perspective_crop`
