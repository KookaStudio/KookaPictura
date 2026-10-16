## ADDED Requirements

### Requirement: Displace dialog and map file

`Filter ▸ Distort ▸ Displace…` SHALL be enabled whenever filters are, and SHALL open CS6's Displace dialog: `Horizontal Scale:` and `Vertical Scale:` (percent, `-999..=999`, default 10), a `Displacement Map:` radio pair (`Stretch To Fit` checked, `Tile`), and an `Undefined Areas:` radio pair (`Wrap Around`, `Repeat Edge Pixels` checked). The dialog SHALL have no preview pane and no Preview checkbox, and SHALL leave the canvas untouched while open. After OK the system SHALL ask for the displacement map in a file dialog filtered to Photoshop files; Cancel there, an unreadable file, or a Bitmap-mode file SHALL abandon the filter with no history state, and an unusable file SHALL say so. The map SHALL be the file's flattened composite with its embedded profile ignored: the first channel drives horizontal and the second vertical, and a one-channel file drives both. The `displace` kind SHALL take 5 slots in order: Horizontal Scale, Vertical Scale, Displacement Map (`0` Stretch To Fit, `1` Tile), Undefined Areas (`0` Wrap Around, `1` Repeat Edge Pixels), and the map id. The dialog SHALL carry the id slot through without a widget. Map id `0` SHALL be the neutral map, and an id the store no longer holds SHALL refuse the commit without touching pixels. The store SHALL keep at least the most recent map, so `Filter ▸ Last Filter` re-applies Displace with the same map without asking for a file, and `Last Filter Settings` SHALL reopen the dialog with the last settings and ask for a map again.

#### Scenario: The dialog follows CS6's layout and defaults

- **WHEN** the Displace dialog opens on a document
- **THEN** its values are `10, 10, 0, 1, 0`, the radios read `Stretch To Fit`, `Tile`, `Wrap Around`, `Repeat Edge Pixels` in that order, and no preview pane or Preview checkbox is present

#### Scenario: A loaded map displaces the picture and Last Filter reuses it

- **WHEN** a flattened RGB PSD whose red channel is 255 and green channel 128 is loaded as a map and Displace is committed with its id at 10%
- **THEN** each pixel shows the colour from about 12.7 px to its right with its row unchanged, and `Filter ▸ Last Filter`, labelled `Displace`, adds one more history state that displaces again

#### Scenario: Unusable maps are refused

- **WHEN** a missing file, a non-PSD file, or a Bitmap-mode PSD is loaded as a map, or Displace is committed with an id the store never issued
- **THEN** the load returns no id, and the commit fails with the pixels unchanged
