## ADDED Requirements

### Requirement: Lens Blur dialog and depth source

`Filter ▸ Blur ▸ Lens Blur…` SHALL be enabled whenever filters are and SHALL open CS6's Lens Blur dialog with these controls in order: Preview (`Faster` checked / `More Accurate`), Depth Map Source (None / Transparency / Layer Mask / Selection), Blur Focal Distance (`0..=255`), Invert, Shape (`Triangle (3)` through `Octagon (8)`), Radius (`0..=100`), Blade Curvature (`0..=100`), Rotation (`0..=360`), Specular Brightness (`0..=100`), Specular Threshold (`0..=255`), Noise Amount (`0..=100`), Distribution (Uniform / `Gaussian` checked), and Monochromatic, plus a depth-map id and a noise seed carried through without widgets. The `lens-blur` kind SHALL take 15 slots in that order; an empty slot list SHALL default to `Faster, None, 126, off, Triangle (3), 50, 33, 191, 44, 93, 47, Gaussian, off, 0, 1`, and a list of the wrong length SHALL be refused without changing the document. The dialog SHALL preview the whole layer (the depth map is positional), and the Preview radio SHALL change sampling density only: a commit SHALL always use the More Accurate path. Before every preview and commit the system SHALL materialize the source slot into a depth map — Transparency from the active layer's alpha, Layer Mask from its mask, Selection from the document selection, None as no map — cropped to the active layer's rectangle, and SHALL carry its id in the hidden slot. An unavailable source (a layer with no mask, no active selection) SHALL refuse the commit with a message and leave the pixels unchanged. `Filter ▸ Last Filter` SHALL re-apply Lens Blur with the same settings without reopening the dialog, and `Last Filter Settings` SHALL reopen the dialog prefilled.

#### Scenario: The dialog follows CS6's layout and defaults

- **WHEN** the Lens Blur dialog opens on a document
- **THEN** its 15 slots are `0, 0, 126, 0, 0, 50, 33, 191, 44, 93, 47, 1, 0, 0, 1`, the source list reads `None`, `Transparency`, `Layer Mask`, `Selection`, the shape list starts at `Triangle (3)`, and the preview covers the whole layer

#### Scenario: A transparency source blurs out of focus

- **WHEN** Lens Blur with Source `Transparency`, focal 126, and radius 50 is committed on a layer whose alpha varies
- **THEN** pixels whose alpha is far from the focal value blur more than pixels near it, and the layer's pixels change

#### Scenario: An unavailable source refuses the commit

- **WHEN** Lens Blur with Source `Layer Mask` is committed on a layer that has no mask, or Source `Selection` with no active selection
- **THEN** the commit fails with a message and the pixels are unchanged

#### Scenario: A Faster preview still commits More Accurate

- **WHEN** Lens Blur is committed with Preview `Faster`
- **THEN** the stored last-filter settings carry `More Accurate`, and the result equals the same commit made with `More Accurate`

#### Scenario: Last Filter reuses the settings

- **WHEN** `Filter ▸ Last Filter` is chosen after a Lens Blur commit, or `Last Filter Settings` is opened
- **THEN** Last Filter applies Lens Blur again with the same slots and one more history state, and Last Filter Settings prefills the dialog with them
