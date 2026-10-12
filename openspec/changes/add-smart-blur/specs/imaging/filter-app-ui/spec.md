## ADDED Requirements

### Requirement: Smart Blur dialog

`Filter ▸ Blur ▸ Smart Blur…` SHALL be enabled whenever filters are. It SHALL open the filter dialog with `Radius:` and `Threshold:` (`0.1..=100`, one decimal, defaults 3.0 and 25.0), `Quality:` (Low, Medium, High; default Low) and `Mode:` (Normal, Edge Only, Overlay Edge; default Normal). The `smart-blur` kind SHALL take 4 slots in that order; an empty list SHALL be the defaults, a radius or threshold outside `0.1..=100` SHALL clamp into it, and any other length SHALL be refused. Changing a control SHALL preview on the canvas as other filter dialogs do, OK SHALL commit one history state labelled `Filter: Smart Blur`, and `Filter ▸ Last Filter` SHALL re-apply the same values.

#### Scenario: The dialog opens on CS6's controls

- **WHEN** the Smart Blur dialog opens on a document
- **THEN** its values are `3, 25, 0, 0`, both value boxes range `0.1..=100`, and the menus list Low, Medium, High and Normal, Edge Only, Overlay Edge

#### Scenario: Edge Only previews, commits, and repeats

- **WHEN** Mode is set to Edge Only on a picture of two flat halves and OK is pressed
- **THEN** the canvas turns black-and-white before OK, one `Filter: Smart Blur` state is added with white along the split, and `Filter ▸ Last Filter`, labelled `Smart Blur`, applies Edge Only again
