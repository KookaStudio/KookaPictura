# Design

## One family-aware step

`Theme::shade(color, steps)` is direction-agnostic: positive steps always
lighten. That is right for the dark ramp and wrong for the light one, where a
raised row, a gutter, or a selection clamps to white. `Theme::contrastShade`
wraps it with the active family:

```cpp
QColor Theme::contrastShade(QColor color, int steps)
{
    return shade(color, currentFamily() == ThemeFamily::Light ? -steps : steps);
}
```

The helper reads the applied family, the same state `rampFor` and `paint` use.
The fixed-palette test helper in `LayersPanel::rowImageForTest` keeps its
128-grey base and runs under the default dark family, so existing surface
assertions are unchanged.

## Layers rows

The delegate replaces the three `Theme::shade(Window, N)` calls and the Blend If
chip brush with `Theme::contrastShade(Window, N)`. In the light family the row
steps darker than the list, the gutter darker than the row, and the selection
darker than the gutter; `HighlightedText` stays white and now sits on a medium
grey instead of white. The dark family is byte-for-byte unchanged.

## Channels rows

`ChannelsPanel::styleRows` uses the same helper. A `changeEvent` override calls
`styleRows` on `PaletteChange` and `StyleChange` so a live family switch
re-applies the row palettes (the panel captured the base colour at style time).
The row labels keep using `WindowText`, which is dark in the light family and
light in the dark family; both are legible on the stepped surface.

## Brush tip presets

`tipIcon` takes the ink colour instead of `Qt::white`; the gradient's
transparent end reuses the same colour with alpha 0, and the size label uses it
too. The
panel stores its preset buttons and `updateTipIcons` re-builds their icons from
`Theme::tokenColor("icon")`. A `changeEvent` override calls it on `PaletteChange`
and `StyleChange`, matching `CharacterPanel`, so a live switch follows into the
cached icons. The frame applies the restored theme before the panels are built,
so the initial icons are already correct.

## Not changed

The brush-panel stroke preview and the Brush Preset Picker render the engine's
dab, which already uses the foreground colour. On-canvas white overlays (crop,
annotations, selections) are image data and stay white.
