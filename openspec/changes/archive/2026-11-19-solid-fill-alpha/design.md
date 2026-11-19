# Design

## Context

See `proposal.md` — Why. The current state that shapes the approach:

- `pictura-codec`'s `stage_content::encoded_fill` already implements the desired
  rule: `FillContent::Solid([r, g, b, a])` with `a != 255` becomes a 4-byte
  `SoCo` payload, and an opaque solid becomes `encode_solid_color_fill`'s
  version-16 descriptor. It is `pub(crate)` and reached only through
  `set_fill_content` (the edit path).
- `layer_from_parts` decodes both `SoCo` forms (`decode_fill_within`), so a
  created layer exposes `FillContent::Solid([r, g, b, a])` with its alpha.
- `composite_solid_fill` uses `rgba[3]` as source alpha, and `write_layers`
  already writes a 4-byte `SoCo` as the primary tag when the fill carries alpha.
- Both create paths bypass that rule: `create.rs::add_solid_fill` and the app's
  `fill_layer` call `encode_solid_color_fill([r, g, b])` directly.

## Goals / Non-Goals

**Goals:**

- Make the requested alpha survive the create paths with no new wire form and
  no change to opaque output.

**Non-Goals:**

- An alpha-capable colour source in the UI (the picker has no alpha field).
- An 8-digit app payload; the panel bridge keeps its 6-digit colour.
- A descriptor alpha key; see D2.
- Translucent shape layers: the shape fill model is `[u8; 3]`, so
  `add_shape_layer` keeps shapes opaque.

## Decisions

### D1. One public encode seam, reused by both create paths

Expose the existing `encoded_fill` rule from `pictura-codec` (re-exported as a
public fill encoder) and call it from `add_solid_fill` and the app's
`fill_layer`. Opaque fills then take the same descriptor path as today, and a
non-opaque fill takes the 4-byte form. This removes the duplicated rule rather
than adding a second one: the edit path, both create paths, and the writer all
agree.

Alternative considered: repeating the `a != 255` branch in each create path.
Rejected — it is the divergence that produced the bug.

### D2. Non-opaque fills use the native 4-byte `SoCo`, not a descriptor key

The standard `SoCo` descriptor has no alpha: psd-tools 1.19 exposes
`SolidColorFill.data` as the RGB-only `Clr ` object, ag-psd's colour serializer
writes only `Rd  `/`Grn `/`Bl  ` and its parser reads the same three, and the
archived `2026-09-19-solid-color-fill-descriptor` change concluded the same. The
4-byte form already decodes, composites, rasterizes, and saves, so it is the
correct carrier for a value Photoshop cannot represent.

Alternative considered: adding a `Trns` key inside `Clr `. Rejected — no
reference implementation reads it, so it would be an invented format with no
oracle, and no better than the 4-byte form.

Alternative considered: mapping the colour alpha to the layer's `Fill` field.
Rejected — it is standard, but it discards the alpha from the colour model:
`layer_fill_color` would return opaque and reopening the editor would show an
opaque swatch.

### D3. The app keeps its 6-digit payload

`fill_layer` still parses six hex digits and passes alpha 255; no user surface
can request anything else today. Routing it through the shared encoder keeps the
rule uniform and leaves the payload format unchanged.

## Risks / Trade-offs

- **A translucent fill is not fully portable.** Photoshop cannot represent a
  translucent solid fill, so there is no fidelity to lose; opaque fills stay
  standard. psd-tools 1.19 still opens such a file and classifies the layer as
  a solid-color fill, but cannot decode the 4-byte `SoCo` payload (it reports a
  failed tagged block). The spec records this as an in-house limitation.
- **A corrupt exactly-4-byte payload decodes as a colour.** → Pre-existing and
  accepted by the archived design; length is the discriminator, and the reader
  is bounded.
