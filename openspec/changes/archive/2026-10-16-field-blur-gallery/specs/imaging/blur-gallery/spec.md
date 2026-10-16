## ADDED Requirements

### Requirement: Blur Gallery workspace and pins

`Filter ▸ Blur Gallery ▸ Field Blur…` SHALL be enabled whenever filters are
and SHALL open a Blur Gallery workspace instead of the generic filter dialog:
a full-size preview with on-canvas pins beside a Blur Tools panel and an
Effects / Motion Effects / Noise tabbed panel. The `Filter ▸ Blur Gallery`
submenu SHALL list Field Blur…, Iris Blur…, Tilt-Shift…, Path Blur…, and
Spin Blur… in that order; only Field Blur SHALL be implemented and enabled,
and the rest SHALL stay disabled stubs (Iris Blur and Tilt-Shift await
#341/#342; Path Blur and Spin Blur are post-CS6). The Blur Tools panel SHALL
list Field Blur checked and expanded with its Blur amount, Iris Blur and
Tilt-Shift as unchecked selectable rows, and Path Blur and Spin Blur as
disabled rows (see the combined-effects requirement). The workspace SHALL open on one pin at the picture's centre
with blur 0 and the Effects tab's defaults (Bokeh on, Light Bokeh 14 %,
Bokeh Color 40 %, Light Range 40..118; Noise off, Amount 0, Size 50,
Roughness 50, Color 0, Highlights 0, Gaussian). A click on the picture SHALL
add
a pin carrying the selected pin's blur; dragging a pin SHALL move it
(positions outside the picture allowed); dragging a pin's blur handle ring
SHALL change its blur; each pin's ring SHALL show its blur amount as a white
arc from the top, empty at 0 and a full ring at 100 px and above; Delete or
Backspace SHALL remove the selected pin but
never the last one; the Blur Tools panel SHALL edit the selected pin's Blur
amount in pixels (`0..=500`). Pressing `H` SHALL hide and restore the pin
overlay without changing the result. The Effects tab SHALL carry the Bokeh
checkbox, Light Bokeh and Bokeh Color fields with sliders, and a Light Range
bar with a low and a high handle and their value boxes; the Noise tab SHALL
carry the Gaussian/Uniform/Grain picker, its checkbox, and the Amount, Size,
Roughness, Color, and Highlights fields with sliders; the Motion Effects tab
SHALL state that Path/Spin motion effects are not implemented. Unchecking Bokeh or Noise
SHALL switch that effect off at render and commit time without losing the
controls' values, which stay editable. The preview
SHALL re-render as the pins, the amount, or any effect control changes, and a
Preview checkbox SHALL restore the unfiltered picture when unchecked. Cancel
SHALL close without touching the document; OK SHALL commit one `"Filter"`
history state named for the menu row.

#### Scenario: The workspace opens on CS6's default pin

- **WHEN** Field Blur is chosen from the Filter menu on an open document
- **THEN** the workspace shows one pin at the picture's centre with blur 0,
  the Field Blur row checked and editable, the Iris Blur / Tilt-Shift rows
  unchecked but selectable, the Path Blur / Spin Blur rows present but
  disabled, and the Effects, Motion
  Effects, and Noise tabs

#### Scenario: The Blur Gallery submenu lists the gallery entries

- **WHEN** the Filter menu is opened
- **THEN** a Blur Gallery submenu lists Field Blur…, Iris Blur…, Tilt-Shift…,
  Path Blur…, and Spin Blur…, with only Field Blur implemented and enabled

#### Scenario: Pins are added, moved, and removed

- **WHEN** the picture is clicked away from the existing pin
- **THEN** a second pin is added and selected; dragging it moves it; Delete
  removes it; and Delete on the only remaining pin is refused

#### Scenario: A blur amount previews live

- **WHEN** the selected pin's Blur is raised
- **THEN** the workspace preview blurs around that pin, and unchecking
  Preview shows the unfiltered picture

#### Scenario: The Effects and Noise tabs edit the preview

- **WHEN** Light Bokeh is raised on the Effects tab, or Noise is enabled with
  an amount on the Noise tab
- **THEN** the preview lifts or roughens, and unchecking Bokeh or Noise turns
  the effect off while the tab's controls stay editable

#### Scenario: Cancel restores and OK commits once

- **WHEN** the workspace is cancelled after edits, or accepted
- **THEN** Cancel leaves the document bit-identical, and OK records exactly
  one history state labelled `Filter: Field Blur`

### Requirement: Field Blur slots and Last Filter

The `field-blur` kind SHALL take a variable slot list `[count, x₀, y₀, blur₀,
…, bokeh enabled, light bokeh, bokeh color, range low, range high, noise
enabled, noise amount, noise size, noise roughness, noise color, noise
highlights, noise kind, seed]`, with an arity-table canonical count of one
pin and the 13-slot effects tail (17 slots). An empty list and the canonical
all-defaults list (count 0) SHALL both map to one centred pin with blur 0;
the empty list SHALL carry the documented effect defaults and the all-defaults
list the zeroed effects; a count of 1..=16 SHALL require exactly
`1 + 3·count + 13` slots; any other list and a count above 16 SHALL be
refused without changing the document. The workspace SHALL produce the list
with pin coordinates as fractions of the picture and blur amounts in
document pixels. `Filter ▸ Last Filter` SHALL re-apply Field Blur with the
stored slots without reopening the workspace, and `Last Filter Settings`
SHALL reopen the workspace prefilled with them.

#### Scenario: The canonical one-pin list maps

- **WHEN** `field-blur` is mapped from the canonical 17 slots
- **THEN** the filter is one centred pin with blur 0 and the zeroed effects,
  and the empty list maps to the same pin with the documented defaults

#### Scenario: A malformed pin list is refused

- **WHEN** the kind is mapped from a list whose length does not match its
  count slot plus the effects tail, from the old no-tail layout, or from more
  than 16 pins
- **THEN** the mapping returns no filter and the document is unchanged

#### Scenario: Last Filter reuses the pin list

- **WHEN** `Filter ▸ Last Filter` is chosen after a Field Blur commit, or
  `Last Filter Settings` is opened
- **THEN** Last Filter applies the same pins, amounts, and effects with one
  more history state, and Last Filter Settings prefills the workspace with
  them
