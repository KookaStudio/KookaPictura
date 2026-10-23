# Design

`ToolController::setActiveTool` calls `resolvePendingCrop` before the transform
prompt, with the same shape: a prompt callback the frame wires to
`askCropImage`. Unset, or on the offscreen platform, it answers Don't Crop,
which is what a switch did before, so headless runs and existing tests are
unchanged. A pending Crop is one whose box differs from the canvas or whose
angle is set (`hasPendingCrop`); a pending Perspective Crop is a complete quad.
Crop commits through `cropCommitSession`; a refused commit stays on the tool.

Only a tool switch asks. The far-click apply, Enter and the options-bar
buttons keep their meaning.
