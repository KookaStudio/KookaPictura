# Design — built-in pattern library

## Context

The pattern fill surface currently resolves a `PtFl` pattern id only through the
document's `Patt`/`Pat2`/`Pat3` blocks. The app has no pattern library of its
own, so a blank document has no pattern ids, the Pattern fill command is
disabled, and the editor shows an empty state.

`pictura-codec` already has `encode_rgb_pattern` and `add_document_pattern`,
which layer effects use to write a pattern into the document. The same
machinery can embed a built-in pattern when a fill starts referencing it.

## Goals

- Pattern fill is available on every open document.
- Built-in patterns are project-authored, deterministic, and small.
- A fill that references a built-in renders, gets a thumbnail, and survives a
  save/reopen because the pattern is in the document.
- Document patterns keep their current order and take precedence over a
  built-in with the same id.

## Non-goals

- `Edit > Define Pattern`, importing `.pat` files, or a user pattern library.
- Alpha in built-in patterns; the existing `encode_rgb_pattern` writes opaque
  RGB patterns.
- Changing how Photoshop-authored patterns decode.

## Decisions

### Built-ins live in the codec

`pictura_codec::builtin_patterns()` returns a `OnceLock`-backed slice of
`PatternPixels`. Two 8x8 opaque patterns are enough to make the picker useful
and keep the library tiny: `Pictura Checkerboard` and `Pictura Diagonal`. The
ids double as the picker labels, matching the existing dialog which shows
document pattern ids.

### Available ids are document-first

The app bridge returns the document's decoded patterns first, then built-ins
whose ids are not already in the document. A document that embeds a pattern with
a built-in's id therefore keeps its own pixels.

### Embed on create and edit

`add_fill_layer` and `set_layer_fill_pattern` call an app helper that, for a
built-in id not already in the document, encodes the pattern and appends it to
the `Patt` block before setting the layer's `PtFl`. The embed happens inside the
same `edit` closure as the layer change, so it is one history state and undo
removes both. A document pattern is already present and needs no write.

### Gating becomes document-only

The Layer menu, Layers-panel menu, Adjustments-panel icon, and grid use the same
rule: a document is open. The "No patterns are loaded" reason is retired; the
Pattern Fill editor keeps its empty state only as a defensive fallback for a
future no-library configuration.
