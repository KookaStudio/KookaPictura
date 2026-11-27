# Design

## Context

The Layers panel already resolves the selection and exposes undoable opacity and fill transactions. The tree and canvas own distinct Qt key paths.

## Decisions

- Observe keys on the Layers tree and the active canvas. On the canvas, start with the Move tool, the documented context for the gesture. The tree is explicitly a Layers context. Leave text inputs and modal editing in their normal event paths; do not start or complete a gesture while a transform is pending.
- Keep one pending press in the Layers panel with its focus owner, document generation, selection and Shift state. Use a monotonic 500 ms interval. Invalidate it when focus, document, selection or pointer context changes, even if the previous context is restored, and on intervening non-modifier keys or modified command shortcuts. Ignore auto-repeat. Accept only no modifier or Shift, with the keypad modifier also allowed; Ctrl, Alt and Meta combinations remain available to their existing commands. Accept direct `Qt::Key_0` for the number row and keypad. For shifted native events whose Qt key differs, use the already-linked Qt GuiPrivate `QKeyMapper::possibleKeys` as a fallback to identify Shift+the base zero key. Do not identify zero solely by the resulting character: shifted zero and shifted nine vary across layouts.
- Commit on the second press through the existing `set_layers_opacity` or `set_layers_fill` batch API, separate from the slider preview commits. Use the same eligibility and targets as the fields. A single press creates no edit or delayed action. Other numeric shortcuts are outside this change.

## Risks / Trade-offs

A fixed interval provides predictable grouping and is a Pictura choice. Native key identification depends on the platform and needs physical keyboard smoke coverage on different layouts.
