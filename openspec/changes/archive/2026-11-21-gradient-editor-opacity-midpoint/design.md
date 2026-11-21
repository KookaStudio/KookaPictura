# Design

## Gradient model

`pictura_paint::gradient::Gradient` keeps its colour-stop list and gains an
optional opacity-stop list:

- `GradientStop { position, color, midpoint }`
- `GradientOpacityStop { position, opacity, midpoint }`

`midpoint` is the fraction of the segment (0.0–1.0, clamped to 0.01–0.99) at
which the interpolation is exactly half way. Sampling maps the segment fraction
`t` through

```
f(t) = t <= m ? 0.5 * t / m : 0.5 + 0.5 * (t - m) / (1 - m)
```

so `m = 0.5` is the existing straight blend. A zero-width segment stays a hard
step. Opacity is sampled by the same rule. When the opacity list is empty the
colour stops' alpha is used, which keeps every existing preset and call site
unchanged. Reverse negates each position and each midpoint.

The C++ editor model mirrors this: `GradientColorStop` gains a percent midpoint
and `GradientSpec` gains `GradientOpacityStops`. Midpoints and opacity stops are
serialised for the Gradient tool's persisted options; the other consumers keep
the existing `"location:RRGGBB[AA] …"` ramp and receive a baked RGBA list.

## Baking for stored-ramp consumers

`bakedSolidStops(spec)` evaluates the colour and opacity lists at every stop
location. When any midpoint is off 50 % it also samples every 32 units across
the ramp, because a midpoint has no visible effect at the endpoints alone. The
result is an ordinary RGBA stop list, so Gradient Map, Gradient Fill, and the
layer-style overlay need no PSD-format change. `resolvedStops` then applies
Smoothness exactly as before.

## Editor interaction

`GradientStopBar` gains a second stop row:

- colour stops below the bar (existing behaviour),
- opacity stops above the bar, dragged off upward to delete,
- midpoint diamonds inside the bar, one per adjacent pair in each row; the
  colour row uses the lower half and the opacity row the upper half.

Selection is `(kind, index)`. `selected()` keeps returning the colour index for
existing call sites; `selectedOpacity()` and `selectedKind()` serve the new row.
The Stops group enables the matching row and a shared Midpoint field. The
layer-style call site keeps strict colour locations.

## Gradient tool reachability

The options bar's sample becomes a `MenuButtonPopup` tool button: its main part
opens the editor, its arrow opens the existing preset menu. The tool's options
gain a serialised `gradient` spec. When it is non-empty the bridge parses it
into the engine's midpoint/opacity model; otherwise the bridge falls back to the
built-in preset index. Accepting the editor stores the spec and unchecks the
preset actions; choosing a preset clears the spec.

## Files and size

`gradient_editor_dialog.cpp` is already near the 800-line target. The new
opacity/midpoint logic belongs in the same class, so the file may cross the
target; it stays under the hard cap, and a split along the stop-bar seam is the
follow-up if it grows further.
