# Keep the stroke band at the selection's edge, and the fill's transparency

## Why

The manual pass over #791's fill change found three behaviours that are wrong
at every depth, not only at 16 bits. None of them is new: each is a
pre-existing gap between the specs and the code, and each survived a green
test suite because the assertions happened to sit where the bug does not
reach.

**The stroke's band smears along the row.** `Edit ▸ Stroke…` with the band
2 px Outside on a rectangular selection drew the band correctly *and* filled
the whole area from the selection's right edge to the canvas's right edge.
`bucket.rs`'s `dilate` runs its horizontal max-filter in place: it reads
`out[y * w + nx]` while writing `out[y * w + x]` in the same left-to-right
loop, so the value it has just written — already dilated — is read back by the
next pixel and the dilation cascades rightward along the row. The vertical
pass uses a separate buffer, which is why only the horizontal direction
smears. `erode` has the same in-place pass, so the Inside band is wrong too.
The existing test uses an 8×1 row and the Qt case checks three pixels, none
of them the one just past the band.

**Preserve Transparency does not preserve the transparency.** `Edit ▸ Fill…`
with Preserve Transparency set filled a partly transparent pixel at 100 %
opacity. The app implements the option as a *coverage mask* — the layer's
surface, alpha > 0 becomes 255 — so a fully transparent pixel is skipped but a
half transparent one is filled at full strength, and the source-over blend
then pushes its alpha to 255. Photoshop's Preserve Transparency keeps every
pixel's existing alpha and leaves the empty pixels alone. The engine already
has that behaviour, under the name of the layer's transparency lock
(`keep_alpha` in `fill_planes`), but the dialog's flag never reaches it: the
two mechanisms are separate and only one of them is wired up.

**A transparent gradient does not cut through.** The Gradient tool's
Transparency option is on by default and the ramp's transparent end did not
make the drawn pixels transparent; they stayed opaque. The fill's Normal
blend is a source-over — `out_a = src_a + da * (1 - src_a)` — so a source
whose alpha is 0 leaves the destination's alpha exactly as it was. A brush
cannot cut a hole, but a gradient is not a brush: Photoshop's Gradient tool
*replaces* the pixels' alpha with the ramp's. `tools/gradient-tool` already
says "Transparency off SHALL ignore the ramp's own alpha", which implies that
Transparency on honours it — the code has never done so.

## What Changes

- The stroke's band stops at the selection's edge. `dilate` and `erode` run
  their horizontal pass from the source mask into a buffer of their own, as
  their vertical pass already does.
- Preserve Transparency becomes the engine's own `keep_alpha` behaviour, so
  every pixel keeps its alpha and the empty pixels are left alone. The app's
  coverage-mask workaround is deleted, not extended.
- The fill engine grows one alpha policy with three behaviours, so a caller
  can ask for the source-over (the default), for the destination's alpha to
  survive, or for the source's alpha to become the pixel's. The Gradient
  tool asks for the last one when Transparency is on and the mode is Normal.
- The layer's transparency lock outranks the caller's policy: a locked layer
  keeps its alpha whatever the caller asked for.

## Capabilities

- Added: `tools/fill-engine` — the fill engine's alpha policy.
- Modified: `tools/gradient-tool` — a Gradient's ramp alpha reaches the
  pixels it draws.
- Modified: `ui/edit-menu-commands` — `Edit > Fill`'s Preserve Transparency
  keeps every pixel's alpha, and `Edit > Stroke`'s band stays at the
  selection's edge.

## Impact

- Code: `pictura-paint` (`fill.rs` grows the `Alpha` policy and threads it
  through `fill_planes` and `fill_layer`; `fill_native.rs` passes it on;
  `bucket.rs` grows the policy parameter and its `dilate`/`erode` stop
  cascading; `gradient.rs` asks for the source's alpha), and
  `pictura-app` (`fills.rs` passes the dialog's flag to the engine instead of
  building a mask).
- Tests: `pictura-paint` unit tests for the band, the policies and the
  ramp's alpha, and `tst_fill_tools` cases for the band with room to the
  right of the selection and for Preserve Transparency on a 16-bit document.
- No new dependency, and no new engine concept beyond the alpha policy.
- `docs/` is unchanged: the corpus already says Preserve Transparency keeps
  the transparent areas, that a stroke lays a band along the selection edge,
  and that the Gradient's Transparency uses the ramp's mask.
- The 8-bit path's bytes do not move for a fill that asks for the source-over
  — the policy defaults to it, and the dialog's Preserve Transparency was
  already skipping the empty pixels.
- **Out of scope.** The Gradient's ramp alpha is not replaced in the other
  brush modes: in Multiply the blend governs the alpha, as in Photoshop. A
  feathered selection does not feather a gradient's alpha, because the
  source's coverage and its opacity arrive merged and separating them is a
  larger change than the bug warrants. The Paint Bucket's flood still matches
  its region at 8-bit precision.
