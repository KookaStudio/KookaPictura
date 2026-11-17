# Design

## Context

See `proposal.md` for motivation and the four spec deltas for the behaviour
contract.

The existing seams that shape the approach:

- The Layer Style dialog binds one `"<effect>.<field>"` scalar at a time
  through `layer_style_value` / `set_layer_style_value`. The engine's field
  table maps a number onto an `lfx2` value and refuses unknown keys.
- `default_effect` already writes a linear `ShpC` contour (`Nm` + `Crv` of
  `CrPt`) into every contour-bearing effect, so the descriptor shape is known.
- The bevel decoder exposes typed scalars but ignores `TrnS`, `MpgS`,
  `useShape`, `Inpr`, `AntA` and the texture keys. The renderer builds a
  chamfer height, derives normals, and splits signed shading into separate
  highlight and shadow blurs.
- The gradient-overlay decoder and renderer already consume every `Clrs` stop;
  the dialog writes only the first and last colour. The shared
  `GradientEditorDialog` exists and speaks a `"loc:rrggbb[aa] …"` text format.
- `knko`/`clbl`/`infx` are typed layer fields; `brst` is an unmodelled tagged
  block preserved in `extra_blocks`. The compositor resolves per-layer state
  once in `BlendCtx` and has fast row kernels plus a general per-pixel kernel.
- The GPU planner declines layers whose attributes it cannot model with
  `UnsupportedAdvancedBlending`, and the caller falls back to CPU.

## Goals / Non-Goals

**Goals:**

- Author and render bevel contours end to end, with Linear remaining
  byte-identical.
- Reuse the existing gradient editor and gradient renderer instead of building
  a second gradient UI.
- Make Channels Include a real model → codec → compositor → dialog slice.
- Keep the new code inside the file-size targets; split the authoring module
  rather than growing `layer_style.rs` past 800 LOC.

**Non-Goals:**

- Bevel Texture (#606), Pattern Overlay Snap To Origin (#606).
- Gradient opacity stops, stop midpoints, noise gradients (#598).
- Blend If UI (#264), Transparency Shapes / mask-hides toggles (#695).
- Contours on shadows, glows and satin.
- CMYK/Lab channel-include semantics beyond ignoring indices above 2.

## Decisions

### Contour is a point list with a 256-step piecewise-linear LUT

Store `Contour { points: Vec<(f32, f32)> }` and derive a `[f32; 256]` lookup
table. The points are the PSD `Crv` data; the LUT is a render detail.

Alternative: store the LUT in the layer. Rejected - it would not round-trip the
descriptor and would make the editor the source of truth.

`is_linear()` is defined on the LUT, so any identity point list (including a
hand-edited `(0,0) (128,128) (255,255)`) takes the fast path. A linear contour
must not merely approximate identity: skipping the mapping entirely is what
keeps an existing style's composite byte-identical (the golden/oracle tests
compare exact bytes).

Writing a contour updates only `Hrzn`/`Vrtc` on each `CrPt` and preserves any
other items on points whose input is unchanged, so a corner/smooth flag that a
PSD carries is not dropped. The bevel sub-panel switches are ordinary scalar
fields added to the `BEVEL` table (`edgeContourOn` → `useShape`,
`edgeRange` → `Inpr`, `edgeAntiAlias` → `AntA`); the gloss `antiAlias`
(`antialiasGloss`) default is corrected to 1 to match the docs' "on".

The editor's piecewise-linear mode and the engine LUT must agree. Photoshop's
exact contour interpolation is unpublished, so piecewise linear is the
defensible shared interpretation; this is recorded in the module's `ponytail:`
note.

### Gloss applies after shading; edge applies after rounding

The gloss contour remaps the signed Lambertian shading:
`t = clamp((s + 1) / 2, 0, 1)`, `s' = 2·lut(t) − 1`. The clamp matters because
the shipped shading is `dot(n, l) − sin(altitude)` and leaves `[-1, 1]` at the
default altitude. Applying the remap before the highlight/shadow split keeps
both sides consistent and makes a linear curve an exact identity.

The edge contour remaps the chamfer height after the technique's rounding blur
and before the normal is derived, so the normal sees the contoured profile.
`Range` scales the input (`height · 100 / range`, clamped to `1..=100`) and
defaults to 50. An identity contour skips both the LUT and the range scaling, so
a linear contour is byte-identical even with the default range.

The anti-alias flags are decoded and authored but do not change the render;
they join the existing `ponytail:` ceiling list.

### Structured values travel as text, not as new scalar kinds

The scalar `Kind` enum stays for numeric fields. Contours and gradients get
dedicated getter/setter pairs:

- contours: `"x,y x,y …"` (0–255, strictly rising), matching
  `CurveWidget::pointsText`
- gradient: `"loc:rrggbb …"` (0–4096, strictly rising, six hex digits). The
  shared editor's own format also accepts `aa`; this key rejects it because a
  PSD `Clrs` colour has no alpha.

Both parse in Rust and refuse malformed input without mutation. The gradient
key is six hex digits only: a PSD `Clrs` colour has no alpha, so accepting
`rrggbbaa` would silently drop it. A gradient write keeps `Trns`, `GrdF`,
`Intr`, and the `Mdpn` of every stop whose location is unchanged; new stops take
midpoint 50. Both setters refuse a group and the Background, like the scalar
setter. This mirrors the existing `layer_style_set_blend_if` precedent (a typed
multi-argument call beside the scalar setter) and avoids inventing a JSON
encoding or a generic structured-value kind for two shapes.

Alternative: widen `Kind` with `Contour`/`Gradient` variants. Rejected - the
field table would then need text parameters, and every existing field would
carry the new machinery for no benefit.

### Contour authoring lives in its own module

`layer_style.rs` was at 798 lines (target 800, hard cap 1200). Contour
read/write moves to `layer_style_contour.rs`, the gradient helpers go with it,
and the `LayerStyle` copy/paste block moves to `layer_style_commands.rs`, so the
parent stays under the target.
The bridge extracts the existing before/after paint-bounds refresh into one
helper so both scalar and structured setters share the live-preview path.

### Gradient editing reuses the shared editor and preserves opacity

The Gradient Overlay page keeps its existing Reverse/Style/Align/Angle/Scale/
Dither controls, replaces the two swatches with one `GradientSwatch`, and opens
`GradientEditorDialog` on click. The nested editor applies on accept, not
live; the outer dialog's controls remain live. This keeps Cancel trivial and
avoids writing on every drag in a modal sub-dialog.

The renderer ignores opacity stops today, so the write path replaces only
`Clrs`, leaving `Trns`, `GrdF`, `Intr` and the effect's other fields untouched.
The editor's alpha is forced opaque and its disabled opacity row stays
disabled. Implementing opacity stops here would duplicate #598.

The shared editor gets a solid-only mode for this call site: the Noise type and
Smoothness control are unavailable, and the call site validates stop locations
as strictly rising (the bar itself permits equal locations, and the engine
setter refuses them). Without the guard, a valid drag could become a silent
no-op.

### Channels Include is a restriction list, not three booleans

Model the PSD data directly: `Layer.channel_exclude: Vec<u32>` of restricted
channel indices, backed by `brst`. The dialog's "Include" checkboxes are the
inverse. This keeps the file format as the model and lets the UI derive its
labels and count from the document's colour channels, capped at the three
working planes. Unlike the `Blend If` view, a parsed `brst` is removed from
`extra_blocks` and re-encoded from the typed list.

Alternatives:

- `channel_include: [bool; 3]` - simpler in memory but maps poorly onto a
  variable-channel document and loses the PSD ordering.
- A new `lfx2` key - wrong: Photoshop stores this as a tagged block.

### The CPU gate lives at the layer boundary

`composite_layer_inner` snapshots the excluded channel planes over the layer's
paint reach (the content rect union the effect-pass rects, or the whole canvas
for a group or a destructive adjustment, clipped to the canvas) before its
below-content effects, and restores them after its above-content effects. For a
pass-through group the same snapshot/restore wraps the child recursion. The
restored value is the entry backdrop's straight channel at the normal output
alpha. A knockout composites into a temp seeded from its stopping-point base,
so `composite_knockout` also snapshots the running canvas and restores the
excluded channels there after copying the covered pixels. A clipping group's
canvas is the clipped member's entry backdrop. This covers pixels, fills,
groups, clipping members and effect passes from one place; a per-pixel
`BlendCtx` mask is unnecessary. The fast row kernels are bypassed when the
layer carries restrictions.

The GPU planner adds a non-empty restriction list to `own_attribute_decline`
and returns `UnsupportedAdvancedBlending`; the existing fallback handles the
rest. This is the same pattern the planner uses for Blend If and knockout.

Malformed `brst` data is ignored and dropped on save: `write_tag` frames an
even declared length, so preserving a three-byte payload would turn it into a
valid four-byte restriction on reopen. `LayerStyle` copy/paste carries the
restriction list, so Paste Layer Style does not lose it. The UI row exists only on an RGB
document; other modes show none, and indices above 2 are ignored by the engine.

### The contour editor reuses `CurveWidget`

`CurveWidget` already provides the interaction a contour editor needs: click to
add, drag, delete, endpoints, a 256-entry LUT, and a points text format. It gains
a `setLinearInterpolation(bool)` flag (default off) so the contour path matches
the engine LUT while Curves keeps its spline. A thin `ContourSwatch` +
`ContourEditorDialog` pair supplies the swatch, preset list, and OK/Cancel.

Alternative: a dedicated contour widget. Rejected - it would duplicate the
drag/delete/add logic for a visual difference that is not verifiable against
CS6.

## Risks / Trade-offs

- **Contour interpolation and gloss mapping are unverified against CS6** →
  piecewise linear and the `(s+1)/2` remap are documented as model choices;
  tests assert direction and bounds, not CS6 pixels.
- **`useShape` as the contour checkbox is inferred from the descriptor** →
  default off, and the UI writes it explicitly, so an unreadable value degrades
  to today's render.
- **`brst` indices are assumed to be 0-based R/G/B** → indices above 2 are
  ignored; a CS6 fixture can confirm or correct the mapping without changing
  the public keys.
- **Gradient opacity stops and midpoints stay stale when colour locations
  change** → `Trns` is preserved verbatim and opacity is ignored by the
  renderer; `Mdpn` is preserved only for unchanged locations; #598 owns the
  rest.
- **Files that already carry a non-linear contour or `brst` now render it** →
  this is the intended parity change; fixture tests pin the old-ignored and
  new-rendered behaviour.
- **A malformed `brst` is dropped rather than preserved** → documented in the
  spec; the alternative would fabricate a restriction on the next save.
- **The channel row is RGB-only** → documents in other colour modes show no
  channel row; the model still uses the first three indices.
- **`CurveWidget` is shared with Curves** → the new flag defaults off and the
  existing Curves tests remain the regression guard.
- **File-size pressure** → contour/gradient authoring is split out; the new C++
  pair is a separate translation unit registered in `cmake/sources/dialogs.cmake`.

## Migration Plan

No migration. Newly authored styles default to today's behaviour: the edge
contour is off, both contours are linear, `brst` is absent, and gradient
opacity stops and midpoints are preserved. A file that already stores a
non-linear contour or a `brst` block now renders it instead of ignoring it; that
is the intended parity change and is covered by fixture tests. A malformed
`brst` is ignored and dropped on save. Styles saved by this change remain
readable by the shipped decoder.
