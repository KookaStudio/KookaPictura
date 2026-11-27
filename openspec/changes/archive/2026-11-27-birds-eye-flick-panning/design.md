## Context

ImageView already owns per-window zoom, rotation, pan limits and tiled painting. The frame handles tool shortcuts and wires Hand movement to the visible peers. The existing GPU backend and preferences store supply availability and persistence.

## Goals / Non-Goals

Deliver both Hand navigation gestures from #534. Preserve ordinary Hand, Space, middle-button, and modal tool routing. Exact CS6 friction and the #533 Zoom gestures are outside this change.

## Decisions

- Arm the overview only when the existing shortcut routing accepts a non-repeated plain H press as the Hand tool shortcut. A command, active tool or text input that owns H keeps precedence. If Hand is remapped away from H, H does not arm the overview; a remapped shortcut still selects Hand normally but does not arm this H-specific gesture. Ignore key autorepeat for arming and release. Releasing H before a pointer press disarms it without changing ordinary Hand selection.
- During an overview use a scoped effective zoom and offset for the rotated fit, without changing the live viewport or emitting its signals. The effective viewport must drive pyramid level selection, visible crop, clipping, painting and document overlays so the entire document is rendered, not just the original viewport crop. Draw the previous viewport footprint as a destination outline in widget space, and map pointer positions through the inverse preview transform to pick document locations. Commit through the existing pan clamp at the original zoom and rotation.
- A primary-button release commits the destination. Escape, early H release, focus/window loss and interrupted input discard the preview. Clearing preview state and consuming the captured pointer's matching release are separate steps so cancellation cannot trigger a tool action. A release after cancellation cannot commit.
- For real left-button Hand or Space-Hand drags, sample recent screen-space motion with a monotonic clock. Decay a capped velocity on a canvas-owned timer, using the same rotation and pan bounds as ordinary Hand. Slow/paused releases and middle-button panning skip inertia. Each tick propagates the displacement actually applied to the initiating view, in the same coordinate system as `handPanned`, to visible peers through the existing Scroll All Windows routing. The option or Shift state at release determines whether the flick continues to scroll peers; each peer still applies its own pan clamp. A clamped source therefore sends no further displacement.
- Persist Flick Panning as Automatic, On or Off. Automatic is the default and enables flick only when GPU navigation is available and reduced motion is not requested. On is an explicit user override of reduced motion, still subject to GPU availability; Off disables flick. Use the Qt widget animation duration as the available desktop reduced-motion hint, treating a zero duration as reduced motion; when no hint is exposed, treat reduced motion as not requested. Re-evaluate the hint when settings change. Disable the control where GPU navigation is unavailable without overwriting the stored choice.
- Put gesture state and timer logic in a separate ImageView translation unit. Keep common event hooks small so #533 can rebase its preference additions independently.

## Risks / Trade-offs

- [Input ownership] Clearing overview state and consuming the matching release are separate steps so a canceled gesture cannot commit a tool action.
- [GPU availability] Offscreen Qt will cover the disabled path and deterministically exercise gesture integration with an explicit availability injection. Native GPU availability and tactile feel need smoke testing.
- [Fit and rotation] Use transformed document bounds for the fit and the inverse preview transform for destination picking.
- [Desktop hint] Qt's widget animation duration is the available hint, not a guarantee that every desktop exposes its reduced-motion setting through Qt.
