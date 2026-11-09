# Design — Edit > Fade

## Context

`docs/06-filters/filters-overview.md` defines Fade as
`result = lerp(original, filtered, opacity)` under `blend(mode)`, equivalent to
placing the filtered result on its own layer above the original with that
opacity and mode. The command must not re-run the filter: it changes only the
last effect's composite. Photoshop exposes one most-recent effect; another
operation invalidates it.

The app already has a per-document `last_filter` record for Filter ▸ Last Filter,
but it stores only a kind and parameter list. Fade needs the actual pre-effect
and post-effect pixels, so it gets a separate record.

## Goals

- Fade any committed destructive filter, Image adjustment, brush/pencil stroke,
  eraser stroke, or Magic Eraser on a normal pixel layer.
- Opacity 0 % is the original pixels; 100 % Normal is the effect output; a
  repeated Fade recomputes from the original effect output.
- One atomic history state per Fade commit.
- Lab mode restrictions are enforced in the dialog.
- The command is disabled with no retained effect, during another session, and
  after any later edit.

## Non-goals

- Fade after paint on a layer mask or in Quick Mask mode; those targets do not
  create a Fade record.
- Fade on a Smart Object (Smart Filters have their own blending options).
- Fade after Fill, Stroke, transform, or layer-style edits; the specification's
  list is filter, painting tool, erasing tool, and color adjustment.
- A GPU fade path.

## Decisions

### The compositor lives in `pictura-filters::fade`

```rust
pub struct Fade {
    pub opacity: u8,
    pub mode: BlendMode,
}

pub fn fade_layer(original: &Layer, effected: &Layer, fade: &Fade) -> Layer;
```

The result uses the union of the two layer rectangles and keeps the effected
layer's metadata and non-color channels. Each color channel and the alpha plane
follow the W3C source-over composite:

```text
αs' = αs · opacity
αo  = αs' + αb · (1 - αs')
Co  = (1 - αs')·αb·Cb + αs'·((1 - αb)·Cs + αb·B(Cb, Cs))
```

with straight-alpha output `Co / αo`. `B` is the existing 27-mode W3C blend
function. For an effect that preserves alpha this reduces to
`lerp(Cb, B(Cb, Cs), opacity)`. Opacity 0 copies the original into the union
rectangle (transparent outside it) bit-exactly; opacity 100 Normal returns the
effected layer bit-exactly, so both extremes avoid rounding.

### The blend function moves to `pictura-core`

`pictura-render` owns the W3C blend function today. `pictura-filters` cannot
depend on `pictura-render` (render already depends on filters). The function is
pure spec math over `pictura_core::BlendMode`, so it moves to
`pictura_core::blend`; `pictura-render::blend` re-exports it and every existing
call site keeps compiling.

### The session keeps original, effected, and current layers

```rust
struct FadeState {
    path: String,
    label: String,
    original: Layer,
    effected: Layer,
    current: Layer,
    opacity: u8,
    mode: BlendMode,
}
```

`original` is the layer before the effect, `effected` the unfaded result,
`current` the last committed fade (or `effected` before the first Fade).
Plane clones are copy-on-write, so the record costs refcounts until a layer is
written. `current` lets Cancel restore a previously faded result when the dialog
is reopened.

Capture points:

- **Filter and adjustment worker commits** — `filter_tools::jobs::commit`
  clones the target layer before applying the op and returns both layers in
  `Committed`; `commit_finish` installs the record. This covers single filters,
  Filter Gallery stacks, and Image-adjustment dialogs.
- **Synchronous filter paths** (`apply_filter`, `apply_filter_params`,
  `apply_filter_stack`) clone before and record after their `edit` call.
- **Image-adjustment direct commands** (`Invert`, `Desaturate`, `Equalize`,
  `Auto *`) clone before `run` applies and record after its `edit` call.
- **Paint and erase** — `begin_paint` clones the target layer into the stroke
  session; `end_paint` records after a stroke that committed a state. Background
  Eraser captures the layered Background, so Fade does not undo the structural
  conversion.
- **Magic Eraser** clones before the one-shot edit and records after.

Every capture skips Smart Objects, layer-mask targets, Quick Mask, and a
no-op result (`original == effected`).

### Invalidation is central

`PictureViewRust::apply_edit_with` clears the record for every edit except
`Meta` and `SelectionPreview`; `impl_history::restore` clears it for undo, redo,
and history jumps. A Fade commit therefore clears and then reinstalls the record
with the new opacity/mode and `current`; an effect commit clears the old record
and installs its own after its edit. Selection changes and layer metadata do not
invalidate it.

### Preview and commit

The bridge exposes `fade_available`, `fade_name`, `fade_opacity`, `fade_mode`,
`fade_mode_allowed`, `fade_preview`, `fade_apply`, and `fade_cancel`.
`fade_preview` swaps the computed layer into the document and presents a
non-history region refresh; `fade_cancel` swaps `current` back; `fade_apply`
swaps the computed layer in and calls `edit("Fade <label>", …)`, then
reinstalls the record with the new values and `current`.

The C++ `FadeDialog` owns an opacity spin box, a mode combo built from
`blendModes()`, a Preview checkbox (on), and OK/Cancel. It reads the prefill
from the bridge, calls `fade_preview` on changes while Preview is on, calls
`fade_cancel` on reject, and calls `fade_apply` on accept. The mode combo uses a
`QStandardItemModel` so the six Lab-restricted modes are visible but disabled.

### Command wiring

`edit.fade` becomes `implemented: true` in the catalogue and gains a generated
`command_ids::EditFade` constant. `registerEditHandlers` sets the handler, an
enabled provider (`fade_available`), and a label provider
(`Fade <effect>…`). The enabled provider also refuses while another session is
live.

## Risks

- Storing two extra layer references per effect can look like memory growth.
  Plane clones are copy-on-write refcounts; the record is dropped by the next
  edit, and the three layers share almost all pixels.
- The fade composite runs on the UI thread. It is a pointwise row-parallel
  operation, unlike a filter kernel, so it is proportional to the layer area but
  has no neighbourhood cost.
- Blend math on a CMYK document runs on the RGB working planes, as the rest of
  the compositor does. The spec's Lab restrictions are enforced; CMYK Fade uses
  the same per-channel blend and is not claimed as exact Adobe parity.
