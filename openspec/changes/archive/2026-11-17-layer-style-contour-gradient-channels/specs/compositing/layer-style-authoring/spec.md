# Spec Delta

## ADDED Requirements

### Requirement: Bevel contours read and write through text keys

`pictura-render` SHALL expose `layer_style_contour(layer, key)` and
`set_layer_style_contour(layer, key, text)` for `bevel.glossContour` and
`bevel.edgeContour`. The text SHALL be `"x,y x,y …"` with integer input and
output in 0–255, at least two points, and strictly increasing inputs. Reading an
absent contour SHALL return the linear list `"0,0 255,255"`. Writing SHALL
replace the contour's points, update only each point's `Hrzn`/`Vrtc` entries,
preserve any other items on a `CrPt` whose input is unchanged, and preserve the
contour's stored name. Writing SHALL add an absent bevel off at its defaults.
Writing SHALL return false, changing nothing, for an unknown effect field,
malformed text, fewer than two points, an input outside 0–255, a list whose
inputs do not strictly increase, a group, or the Background.

#### Scenario: A contour round-trips [contour-author-round-trip]
- **WHEN** `bevel.glossContour` is set to `"0,0 64,128 255,255"` and read back
- **THEN** the read returns that list and the bevel's gloss contour decodes with the same points

#### Scenario: An absent contour reads linear [contour-author-default]
- **WHEN** a plain pixel layer is asked for `bevel.edgeContour`
- **THEN** it returns `"0,0 255,255"`

#### Scenario: Bad contour text is refused [contour-author-refused]
- **WHEN** a contour is written as `"0,0"`, `"0,0 0,255"`, `"0,0 256,10"`, `"0,0 nope 255,255"`, on a group, or on the Background
- **THEN** each write returns false and the layer's contour is unchanged

#### Scenario: A rewritten point keeps its other items [contour-preserves-point-items]
- **WHEN** a contour point carries an extra `CrPt` item (such as a corner flag) and the contour is rewritten with the same input for that point
- **THEN** the extra item is still present and only its `Hrzn`/`Vrtc` values changed

### Requirement: The bevel contour sub-panel writes its switches

`layer_style_value` / `set_layer_style_value` SHALL expose `bevel.edgeContourOn`
(`useShape`, 0/1, default 0), `bevel.edgeRange` (`Inpr`, 1–100, default 50) and
`bevel.edgeAntiAlias` (`AntA`, 0/1, default 0). A new bevel created by a contour
write SHALL carry `bevel.antiAlias` (`antialiasGloss`) as 1, matching CS6's
Anti-aliased default; an absent bevel SHALL read 1. Out-of-range writes SHALL be
clamped to the field's range, and the Background and groups SHALL be refused.

#### Scenario: The sub-panel switches round-trip [contour-sub-panel-fields]
- **WHEN** `bevel.edgeContourOn`, `bevel.edgeRange` and `bevel.edgeAntiAlias` are set to 1, 30 and 1 and read back
- **THEN** the reads return 1, 30 and 1, and the bevel decodes `useShape` true, `Inpr` 30 and `AntA` true

#### Scenario: The gloss anti-alias defaults on [contour-antialias-default]
- **WHEN** a bevel is created by writing `bevel.glossContour`
- **THEN** `bevel.antiAlias` reads 1 and the effect carries `antialiasGloss` true

### Requirement: A gradient overlay's stops read and write through a text key

`pictura-render` SHALL expose `layer_style_gradient(layer, key)` and
`set_layer_style_gradient(layer, key, text)` for `gradientOverlay.gradient`.
The text SHALL be `"loc:rrggbb …"` (space-separated, six hex digits, no alpha),
with locations in 0–4096 rising strictly and at least two stops. Reading SHALL
return the gradient's colour stops. Writing SHALL replace the colour stops and
their locations and SHALL preserve the gradient's opacity stops (`Trns`), format
(`GrdF`), interpolation (`Intr`) and the effect's other fields. For a stop whose
location is unchanged, its midpoint (`Mdpn`) SHALL be preserved; a new stop
SHALL take midpoint 50. Writing SHALL add an absent Gradient Overlay off at its
defaults and SHALL refuse malformed text, an eight-digit colour, a group, or the
Background, changing nothing.

#### Scenario: A gradient round-trips [gradient-author-round-trip]
- **WHEN** `gradientOverlay.gradient` is set to a three-stop list, read back, and the effect is decoded
- **THEN** the read matches the list and the decoded overlay carries the same three stops

#### Scenario: Opacity stops and midpoints survive a colour edit [gradient-author-preserves-opacity]
- **WHEN** a gradient with non-linear `Trns` stops and a non-default `Mdpn` at an unchanged location has its colour stops rewritten
- **THEN** the `Trns` list and that `Mdpn` are byte-identical to before, and the composited alpha is unchanged

#### Scenario: Bad gradient text is refused [gradient-author-refused]
- **WHEN** the text has one stop, a location above 4096, a non-rising location, a bad hex colour, an eight-digit colour, a group target, or the Background
- **THEN** the write returns false and the layer's gradient is unchanged

### Requirement: An authored contour and gradient survive save and reopen

A PSD saved with an authored contour or a multi-stop gradient SHALL reopen with
the same contour points and gradient stops, and the composite SHALL match.

#### Scenario: Save and reopen [authored-style-survives-save]
- **WHEN** a bevel's gloss contour and a Gradient Overlay's three-stop gradient are authored, the document is saved as PSD, and it is reopened
- **THEN** the reopened layer reads the same contour points and gradient stops and the composite equals the original's

### Requirement: Channels Include authors the channel restrictions

`layer_style_value(layer, "blending.includeN")` SHALL read 1 when channel `N`
is included in blending and 0 when it is excluded, for `N` 0–2.
`set_layer_style_value` SHALL add or remove `N` in the layer's channel
restrictions and SHALL accept the key on a group. An absent restriction list
SHALL read as every channel included. An index outside 0–2 SHALL be refused, and
the Background SHALL be refused.

#### Scenario: Unchecking a channel excludes it [channel-include-toggle]
- **WHEN** `blending.include1` is set to 0 and then to 1
- **THEN** the first write excludes channel 1, the second restores it, and each write reports a change

#### Scenario: Every channel starts included [channel-include-default]
- **WHEN** a plain layer is asked for `blending.include0`, `blending.include1` and `blending.include2`
- **THEN** each reads 1

#### Scenario: A group accepts channel restrictions [channel-include-group]
- **WHEN** `blending.include2` is set to 0 on a group
- **THEN** the group's channel restrictions contain 2 and its effect keys remain refused

### Requirement: Copy and paste carry the channel restrictions

`copy_layer_style` SHALL capture the layer's channel restrictions and
`paste_layer_style` SHALL replace the target's with them, alongside the effect
blocks and the other Blending Options fields.

#### Scenario: Copy and paste a restricted style [channel-include-copy-paste]
- **WHEN** a layer with channel restrictions is copied and pasted onto another layer
- **THEN** the target's restrictions match the source's and the composite reflects them
