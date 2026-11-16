# Spec Delta — imaging/blur-gallery

## ADDED Requirements

### Requirement: Blur Gallery preview scrollbars and panning

The Blur Gallery preview SHALL show a horizontal and a vertical scrollbar when
the zoomed picture overflows the preview in that axis, with each scrollbar's
range covering the overflow, and SHALL hide them at the fitted view. Scrolling a
bar SHALL move the picture with it. Pressing the scroll wheel (the middle
button) and dragging over the preview SHALL pan the picture. Scrolling and
panning SHALL only place the preview; the pin fractions and blur amounts
committed by OK SHALL be unchanged.

#### Scenario: Scrollbars appear when zoomed in [blur_scrollbars_appear]

- **WHEN** the preview is zoomed in past the fitted view and then fitted again
- **THEN** both scrollbars show while it overflows and hide at the fit

#### Scenario: Scrolling moves the picture [blur_scrollbars_scroll]

- **WHEN** the horizontal and vertical scrollbars are scrolled to their ends
- **THEN** the picture moves left and up with the scrollbars

#### Scenario: The middle button pans [blur_middle_drag_pans]

- **WHEN** the scroll wheel is pressed and dragged over the zoomed preview
- **THEN** the picture pans with the pointer
