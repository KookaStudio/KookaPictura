# Design: port-photorust-filters

## Engine boundary

`apply` validates the buffer, then asks `photorust::dispatch::apply`. It
returns `None` for variants it does not run, which fall through to Kooka's
match. For a ported variant it:

1. Checks every parameter against the range its capability spec fixes, so
   `FilterError::InvalidParams` comes back before any sample is written.
2. Interleaves the planar buffer into a `Pixmap` (an RGB buffer reads as
   opaque).
3. Runs the photorust function inside `with_seed(seed, …)`.
4. Writes back R, G, and B only. Alpha is never written, which keeps every
   capability's alpha-preservation requirement independent of what a
   photorust filter does with alpha.

The photorust sub-types (Light, Texture, StrokeDirection, and the rest) are
Kooka's own public enums under photorust's names (`use crate::X as Y`). The
variant sets match one for one, so no conversion layer is needed and photorust's
methods on them are kept.

## Seeds

photorust seeds its patterns from pixel coordinates alone. Kooka's dialogs
expose a Seed. The five hash primitives (`artistic::noise`, `pixelate::jitter`,
`stylize::hash`, `texture::lattice`, and the pen-line hash in `brush_strokes`) XOR in `photorust::seed()`. That is an atomic set by
`with_seed` under a mutex for the duration of one filter run.

- **Why a global:** a thread-local would not reach rayon's workers, and
  threading a parameter through every call chain would touch over 100 call
  sites.
- **Seed 0:** folds to 0, so it reproduces photorust's own patterns and its
  tests stay valid. Every other seed folds to a nonzero salt.
- **Reset:** a drop guard puts the seed back to 0 even if the filter panics.
- **Ceiling:** seeded runs are serialized (ponytail).

## Determinism

Every `par_iter().sum::<f32>()` reduction becomes a sequential sum, so the
output does not depend on how rayon splits the work. That covers four in all,
including `brush_strokes::unit_spread` and `artistic::streaked_noise`, which a
test pins by comparing one thread with eight. Everything else is per-pixel or
per-row and order-independent.

## Ranges

Each parameter has one range, shared by the dispatch check, the photorust
function's clamp, and the dialog's slider, so the clamp never moves a value
the check accepts:

- **Dry Brush and Fresco:** Brush Size and Brush Detail run 0–10, the dialog's
  and photorust's CS6-tuned range; the spec's 1–50 / 1–12 is modified.
- **Poster Edges:** Posterization runs 0–10 in photorust too.
- **Ink Outlines:** Stroke Length starts at 1 in photorust, as in the spec.
- **Photocopy and Plaster:** Detail and Smoothness start at 0 in photorust, as
  in the spec. The model is a blur whose reach grows from a floor, so 0 is
  well defined.
- **Dialogs:** Cutout's Edge Fidelity runs 1–3, Colored Pencil's Pencil Width
  1–24, and Mosaic Tiles' Grout Width 1–15.
- **Bounded floats:** Emboss Height and Amount (1–100, 1–500), the Color
  Halftone angles (−360 to 360), and the Wave maxima (up to 999) are bounded,
  so no value reaches the filter as infinity.

## Colours

Colored Pencil, Rough Pastels, and Watercolor no longer show foreground or
background swatches. Their variants keep only the colours the engine uses.
Colored Pencil keeps `background`, its paper, which the app fills with white
in place of the document background (ponytail). Rough Pastels and Watercolor
take no colours, and Colored Pencil no foreground.

## Behaviour reconciled with the specs

Where a replaced Kooka scenario disagreed with photorust, CS6 Help decided:

- **Emboss:** traces edges in the original colour.
- **Mezzotint:** saturated colours on a colour image (the spec already said so).
- **Fragment:** four offset copies (the spec already said so).
- **Colored Pencil:** the background colour shows through, so the port was
  changed.

Where the Help is silent, photorust's documented, CS6-tuned model wins and the
spec is modified: Bas Relief, Accented Edges, Film Grain, Paint Daubs, and Add
Noise's scale. Scenario tests whose inputs could not show an effect (a flat
field for Plastic Wrap, a one-pixel-tall ramp for Facet) were given inputs that
can.

## Files

The ported modules are split into directory modules along filter seams to stay
under the 1200/1400-line caps. These are pure moves, with items widened to
`pub(crate)`. The `chunks_exact_to_as_chunks` and `needless_range_loop` lints
are allowed in `photorust` only, so the follow-up ports diff cleanly against
upstream.
