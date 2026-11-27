# Design

## Context

PSD encodes each black or white handle as two ordered 8-bit endpoints in one big-endian word. The typed view retains these words; malformed raw blocks have no typed view. The current compositor receives RGB values and its prefix cache hashes raw range identity and typed-view presence, not typed endpoints.

## Goals / Non-Goals

Provide Gray and RGB ranges in RGB documents and Gray in grayscale documents. In other modes, show the controls unavailable, do not offer replacement, and leave existing bytes and rendering unchanged. Editing and replacement are also unavailable for pass-through groups, because isolating them changes child blending and knockout boundaries. Existing group ranges remain untouched. Native CMYK and Lab channel compositing and pass-through group gating are outside this change. Existing Channels Include, knockout and advanced blending controls belong to other changes. GPU already declines non-default ranges.

## Decisions

Use a compact custom Qt slider with a tonal ramp and four handles. Alt-drag moves one half; ordinary drag moves both halves together, preserving their separation. Clamp movement so `black_outer <= black_inner <= white_inner <= white_outer`, each in 0–255; do not allow handles to cross. Selecting a channel does not write it. A bridge changes one complete range at a time and preserves all other typed ranges and channel groups.

For normalized sample `x`, black outer/inner `b0,b1` and white inner/outer `w0,w1` scaled by 255, multiply the black and white factors. Black is 0 below `b0`, rises linearly to 1 at `b1`, and is 1 above; white is 1 below `w0`, falls linearly to 0 at `w1`, and is 0 above. Equal black endpoints form a step inclusive on the retained side (`x >= b0`); equal white endpoints form a step inclusive on the retained side (`x <= w1`). Thus defaults 0/0 and 255/255 retain both extremes. Apply the same rule to source and backdrop and each supported channel. Existing PSD words decode high byte first; do not normalize or rewrite untouched words. Define a safe no-op for malformed endpoint ordering on load; retain those bytes until an actual edit to that range.

When nonempty raw bytes have no typed view, disable range editing and explain that the data cannot be interpreted. Offer an explicit Replace action that states it discards the opaque block and starts with default ranges; selecting it alone is a preview transaction, so Cancel restores the exact bytes and accepting it creates one undoable change. Merely opening, selecting, saving, or declining replacement preserves the block byte-for-byte.

Invalidate cached prefix composites when any typed range changes, including successive preview edits with unchanged raw bytes; hash the full typed ranges or invalidate the affected cache explicitly. Use the existing live-preview and history transaction. Undo and cancel restore typed and raw state.

## Risks / Trade-offs

Legacy arbitrary u16 values are interpreted as PSD byte pairs; invalid ordering remains opaque to rendering until edited. Rendering is CPU-only as before. UI and renderer work will require integration with #695 in the shared dialog page.
