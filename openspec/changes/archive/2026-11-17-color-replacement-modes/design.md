# Design: color-replacement-modes

## Mode model

`ReplaceMode` becomes the 25-entry CS6 paint-mode list in menu order:

```
Normal, Dissolve, Behind, Clear, Darken, Multiply, Color Burn, Linear Burn,
Lighten, Screen, Color Dodge, Linear Dodge (Add), Overlay, Soft Light,
Hard Light, Vivid Light, Linear Light, Pin Light, Hard Mix, Difference,
Exclusion, Hue, Saturation, Color, Luminosity
```

The list is the one `docs/03-tools/brush-and-pencil.md:121` prints; the
Color Replacement spec calls it the "CS6 paint-mode list". `Color` is still the
default (index 23).

`ReplaceMode::ALL` is the single source for `from_i32`, `name`, and the bridge
name/count functions. Every blend mode maps to the existing
`pictura_core::BlendMode` and calls `pictura_core::blend::blend(base, paint)`;
the non-separable Hue/Saturation/Color/Luminosity formulas are the same ones the
four-mode engine already used. `Dissolve` uses the core formula (paint), so it
stays deterministic.

`Behind` returns the base colour and `dab` skips it entirely: replacement never
writes a transparent pixel, so the mode is a faithful no-op. `Clear` leaves the
colour channels alone and multiplies the pixel's alpha by `1 - weight`, erasing
matched pixels while keeping partial coverage soft. This is the only mode that
does not preserve alpha.

The 32-bit subset is the intersection of the 25-mode list with the documented
32-bpc paint modes: Normal, Dissolve, Darken, Multiply, Lighten, Linear Dodge
(Add), Difference, Hue, Saturation, Color, Luminosity (11 entries). 8- and
16-bit documents keep all 25.

## Document gate

One Rust predicate is the source of truth:

```rust
fn color_replacement_available(mode: &QString) -> bool
```

It receives an `image_mode` name (`bitmap`, `indexed`, `multichannel`, …) and
returns false for the three flat modes. `begin_color_replacement` checks the
document's `document_color_mode()` through it before starting a stroke, so a
caller cannot bypass the UI. The C++ handler uses the same bridge function for
its refusal message, and `toolAllowedForDocument` uses it for the toolbox.

## Tool availability

`toolAvailable(id)` stays the edition/implementation gate.
`toolAllowedForDocument(id, view)` adds the active document's mode gate; only
Color Replacement has one today.

- `ToolController::setActiveTool` refuses a tool that the document disallows.
- `ToolController::bindCanvas` (reached on every frame refresh) switches an
  unavailable active tool to Move.
- `Toolbox` resolves each member through `toolAvailable && toolAllowedForDocument`
  and refreshes its slot icons, checked state, flyout actions, and tooltips when
  the document's `image_mode` changes. The B slot stays enabled because Brush is
  still available; only the Color Replacement flyout row is disabled.
- `OptionsBar::refreshDocumentFacts` rebuilds the Color Replacement Mode combo
  when the visible bit depth changes; `showTool` and the frame refresh both call
  it. The combo stores the engine mode index in item data, so filtering does not
  renumber the options. If the stored mode disappears, the combo falls back to
  Color (or Normal when Color is also filtered out).

## Testing

- Engine: Rust unit tests for the 25-index round trip, the default and names,
  Normal/Multiply formulas, Behind/Clear pixel behaviour, and the 32-bit subset.
- Bridge: a Rust test that the flat modes are refused and RGB/Grayscale/CMYK/
  Lab/Duotone are allowed.
- GUI: `tst_color_replacement` checks the 25-item menu, the 11-item 32-bit menu,
  the bitmap and indexed gates (including the Move fallback and the disabled
  flyout row), the Multichannel predicate, and a repaint stroke. The existing
  self-test check 540 keeps its history assertion but reads the new mode
  indices.
