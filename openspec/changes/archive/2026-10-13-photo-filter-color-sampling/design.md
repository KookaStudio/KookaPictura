# Design

## Reaching the canvas through the blockers

`runDialog` installs an application event filter that swallows input
addressed to the parent's window. Photo Filter runs under one blocker (main
window) and its Color Picker under a second (the Photo Filter dialog). The
canvas lives in the main window, so only the outer blocker matters; it now
passes a left press / release, or a move holding at most the left button,
over an `ImageView` when the hook installed by `setDialogCanvasSampling`
reports an armed sampler. The frame installs
`tools_->canvasSamplerActive()`, mirroring `setDialogNavigationKeys`. While a
sampler is armed, `ToolController` routes a press to it and never to the
active tool, so letting the click through cannot paint.

## What is sampled

The dialog previews the adjustment on the live layer, so the composite under
the cursor is already tinted. `AdjustmentDialog::originalColorAt` lifts the
preview, reads `sample_argb`, and re-applies it. ponytail: one click costs two
section refreshes; a cached pre-preview composite would avoid that but costs a
full-image copy for every dialog.

## Scope

Only Photo Filter wires the sampler. The other `ColorPickerDialog::getColor`
callers keep the canvas-less picker; each can opt in by passing a tool
controller and a colour source.
