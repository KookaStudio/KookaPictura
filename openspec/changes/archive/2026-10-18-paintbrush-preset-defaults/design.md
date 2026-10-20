# Design: paintbrush-preset-defaults

## Context

`Preset` carries the tip and the pen-tip preview flag; `applyPreset` writes
those fields into the shared `PaintOptions`. The previous change deliberately
kept Mode, Opacity, Flow, and the foreground colour out of a preset. The owner
now wants the four added brushes to reproduce the screenshots, which needs
those fields.

## Decisions

Extend `Preset` with optional `color`, `mode`, `opacity`, and `flow`
(`nullptr` / `-1` means "leave unchanged"). The existing 44 entries keep the
defaults, so they remain tip-only.

| Preset | Grain | Colour | Mode | Opacity | Flow |
|---|---|---|---|---|---|
| `2B Pencil` | 30 | `#47484a` | Normal | 100 % | 100 % |
| `Airbrush Soft Round 50% flow` | 0 | `#47484a` | Normal | 100 % | 50 % |
| `Non-Photo Blue Pencil` | 30 | `#73d6ff` | Normal | 100 % | 100 % |
| `Paintbrush Oval 45 pixels Multiply` | 0 | `#47484a` | Multiply | 35 % | 50 % |

`applyPreset` writes the tip first, then the foreground through
`ToolController::setForeground`, then Mode, Opacity, and Flow through
`PaintOptions`. The Non-Photo Blue Pencil stays a 9 px hard anti-aliased tip;
the close-up's colour, 9 px width, and paper grain are what the preset
reproduces. The Oval's Multiply/35/50 produces the light grey stroke in the
screenshot. The pencils' grain is 30: the earlier 75 was tuned for a binary
drop, which scattered the stroke into isolated dots; the alpha modulation at 30
keeps the line continuous while showing the paper texture.

The airbrush keeps Flow 50 but uses 25 % spacing: at the previous 10 % the
overlapping dabs built a nearly solid core, while the owner's Photoshop
reference is airy and light. `PaintOptions::paintSettingsChanged` lets the
options bar re-read Opacity, Flow, and Mode after a preset applies, so the bar
cannot show stale values (the reference screenshot showed Flow 100 while the
brush was named 50 % flow).

`ponytail:` the values are approximated from the owner's screenshots; no
`.abr` file exists.

## Risks

- Selecting an added preset overwrites a custom foreground colour and paint
  settings. This is the requested "default" behaviour.
- Existing CS6 presets are unaffected.
