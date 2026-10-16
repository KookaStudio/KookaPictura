## ADDED Requirements

### Requirement: Picture Frame dialog

`Filter ▸ Render ▸ Picture Frame…` and `Filter ▸ Last Filter Settings` (when the last filter was Picture Frame) SHALL open a dedicated Picture Frame dialog, not the generic slot dialog. The dialog SHALL show a frame picker listing the 47 frames with a thumbnail each, a preview of the picture with the frame drawn at the current settings, a `Basic` page (vines colour, Margin, Size, Arrangement, Flower, flower colour and size, Leaf, leaf colour and size) and an `Advanced` page (Number of Lines, Thickness, Angle, Fade, Invert), and OK, Cancel, and a Preview checkbox (on by default). The dialog SHALL produce the `picture-frame` slots in order: frame, vine r/g/b, margin, size, arrangement, flower, flower r/g/b, flower size, leaf, leaf r/g/b, leaf size, lines, thickness, angle, fade, invert. With Preview on, the canvas SHALL show a non-committing whole-layer preview, deferred during a slider drag until release. OK SHALL commit one history state and Cancel SHALL restore the canvas bit-identically. A Picture Frame canvas preview SHALL always render against the whole layer, never the visible crop.

#### Scenario: The dialog follows CS6's layout and defaults

- **WHEN** the Picture Frame dialog opens on a document
- **THEN** its values are the CS6 defaults (frame 34, vine `60 84 34`, margin 14, size 20, arrangement 1, flower 2, flower colour `0 88 218`, flower size 20, leaf 4, leaf colour `104 170 60`, leaf size 20, lines 15, thickness 29, angle 10, fade 0, invert off), the picker lists 47 frames with frame 34 selected, and there is a `Basic` page and an `Advanced` page with Preview checked

#### Scenario: The picker and controls drive the frame

- **WHEN** a different frame is picked, and then the flower is set to None
- **THEN** the frame slot carries the picked frame's 0-based index (index 0 is Happy Vine), the preview shows that frame, and setting the flower to None leaves the vine but draws no flowers

#### Scenario: Commit, reopen, and cancel

- **WHEN** the dialog is opened from the Filter menu, a frame and margin are set, and OK is pressed, and Last Filter Settings is then opened after a separate commit, and a separate dialog is cancelled after a preview
- **THEN** one history state is added with the matching last params and the frame drawn at those settings, the reopened dialog reads the committed params, and the cancelled canvas equals its prior pixels

#### Scenario: The section preview matches the commit

- **WHEN** Picture Frame is previewed with a visible viewport that excludes part of the border
- **THEN** the preview is rendered against the whole layer and equals the full-layer commit inside the viewport
