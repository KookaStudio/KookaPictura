# Design

## One config, per-sample scaling

`StrokeConfig::pressure_scaled(pressure)` returns a copy with the diameter
scaled when `pressure_size` is on and the flow plus opacity cap scaled when
`pressure_opacity` is on. `Stroke::sample` stores the sample's pressure and
rasterizes that sample with the scaled config; the next sample starts from the
stroke's own config again, so a light touch does not permanently shrink the
brush. `Stroke::hold` reuses the stored pressure, so an Airbrush build-up keeps
the pressure of the press.

The exact CPU raster uses the scaled diameter for its tip profile and the scaled
flow/opacity in the stencil. The preview path calls the same helper, so its
reduced-level coverage follows the sample pressure. The GPU dab shader has one
static parameter set per stroke, so a pressure-override stroke declines the GPU
and falls back to the exact CPU stroke or the preview.

## Tablet input

`ImageView` overrides `tabletEvent` and emits `pointerPressure` before Qt's
synthesized mouse event reaches the tool routing. `ToolController` stores the
pressure and exposes it through `ToolServices::pressure`; the Brush and stamp
handlers pass it to `paint_api::Stroke::dab`. A non-synthesized mouse press
resets the pressure to 1.0, while a mouse event synthesized from a tablet keeps
the tablet's value.

## UI

The two buttons are checkable `QToolButton`s with the `options.pressureOpacity`
and `options.pressureSize` icons. They sit after Opacity and after Airbrush, as
CS6 places them. `PaintOptions` owns the flags and emits `paintSettingsChanged`,
so a preset or a restore updates the bar. The flags are part of the brush JSON.
