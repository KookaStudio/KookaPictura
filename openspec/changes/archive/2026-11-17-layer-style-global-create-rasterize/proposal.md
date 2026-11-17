# Proposal

## Why

Layer effects render, but the three commands that complete the CS6 layer-style
workflow — Global Light, Create Layers and Rasterize Layer Style — are still
disabled stubs (`menu_layer_style.cpp:3`, `layers_row_actions.cpp`, the catalog
table). Every effect with **Use Global Light** therefore renders its stored
local angle instead of one document-wide light, and a styled layer can only be
cleared or copied, never converted to editable pixels or merged into the layer.
The docs-to-code parity register tracks this as `LAY-2` (#580).

## What Changes

- **Global Light** becomes a real document setting: a dialog edits the master
  angle and altitude, the values round-trip through PSD image resources 1037
  (angle) and 1049 (altitude), and every enabled effect with `Use Global Light`
  (Drop Shadow, Inner Shadow, Bevel & Emboss, including its altitude) renders
  with it. The effect pages disable their own angle/altitude control while
  global light is on and show the global values.
- **Rasterize Layer Style** bakes a layer's live effects into its pixels as it
  shows on the canvas, removes the style, keeps the layer's name, blend mode,
  opacity, fill and mask, and records one undoable state. It is wired to
  `Layer ▸ Rasterize ▸ Layer Style` and to the Layers panel row menu, where the
  entry stops being a disabled stub.
- **Create Layers** explodes a layer style into ordinary pixel layers: below
  effects beneath the base, surface/interior effects above it, each carrying
  its effect's blend mode and the base layer's opacity. The base keeps its
  pixels and loses the style; the produced stack composes to the same
  appearance within tolerance.
- The panel's layer-style submenu stops showing **Global Light…** and **Create
  Layers** as disabled, and all surfaces dispatch the same registry commands so
  their enabled states agree.

No API or file-format break. The stored `lagl`/`Lald` values remain the effect's
local angles; the global setting only overrides them at render time when `uglg`
is on. A document that sets `Use Global Light` without stored resources now
renders at Photoshop's 30° default, and the long-form docs are corrected to
record that and the non-clipping Create Layers shape.

## Capabilities

### New Capabilities

- `codec/psd-global-light`: read and write the document global-light resources
  (1037 angle, 1049 altitude) with Photoshop defaults and byte-preserving
  round-trip.

### Modified Capabilities

- `compositing/layer-effects`: an enabled effect with `Use Global Light` takes
  its angle (and the bevel's altitude) from the document global light instead
  of the stored local value, and the effect-pass cache follows the document
  light.
- `compositing/layer-style-authoring`: three new commands — set the document
  global light, create pixel layers from a style, and rasterize a style into
  its layer — with their enablement and undo behaviour.
- `ui/layer-style-dialog`: `Global Light…` and `Create Layers` are enabled
  commands, and a Global Light dialog edits the document's angle and altitude.
- `ui/layers-panel`: the layer-style menu's `Global Light…` / `Create Layers`
  entries are enabled and registry-dispatched, and `Rasterize Layer Style` is an
  enabled row command when the row has a renderable style.

## Impact

- `pictura-core`: a typed `GlobalLight` on `Document`.
- `pictura-codec`: new global-light resource module, resolved on both full and
  lazy opens and emitted on every PSD/PSB save.
- `pictura-render`: effective-angle resolution in the effect pass build, pass
  cache key, paint reach, a style rasterizer, and per-effect pass
  materialisation for Create Layers.
- `pictura-app`: bridge functions, a `GlobalLightDialog`, menu, panel submenu
  and row-action wiring, and one new catalogue constant
  (`LayerRasterizeLayerStyle`) with regenerated `command_ids.h`.
- Tests: codec resource unit tests, render effect/command tests, and Qt Test
  coverage for the dialog, commands, panel submenu and row menu.
- No new dependencies.
