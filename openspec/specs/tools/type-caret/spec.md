# tools/type-caret Specification

## Purpose
The Type tools' caret: its blink, the point-type placement point, and the baseline guides under typed lines.

## Requirements

### Requirement: The type caret blinks

While a Type tool session is open, the caret's line SHALL blink on a 1.5 second
cycle: shown for 750 ms, then hidden for 750 ms. Any edit, caret move, or click
in the session SHALL show the caret again and restart the cycle. Closing the
session SHALL stop the blink.

#### Scenario: The caret blinks and a keystroke shows it [caret_blink]

- **WHEN** `tst_type_tools::caretBlinks` opens a Horizontal Type session, waits one interval, then types a character
- **THEN** the blink interval is 750 ms, the caret is shown when the session opens, hidden after the interval, and shown again right after the keystroke

### Requirement: Point type marks its placement point and baseline

A point-type session SHALL mark its origin (the click) with a small square
placement point that stays visible while the caret blinks. Once text is typed,
the session SHALL draw a guide along every non-blank line's baseline (the centre
axis of a vertical column) from the line's start to its end. The guides SHALL
stay visible while the caret blinks at the end of the text. Empty text SHALL
show the caret and the placement point only.

#### Scenario: Placement point, then the baseline guide [caret_placement]

- **WHEN** `tst_type_tools::caretPlacementAndBaseline` clicks with the Horizontal Type tool, then types "some text"
- **THEN** before typing, the placement point is at the click and there is no guide; after typing, one guide starts at the click, runs along the click's baseline, and ends where the caret stands

#### Scenario: Baseline segments from the layout [caret_baselines_engine]

- **WHEN** `pictura-render::baselines_follow_the_layout` asks for the baselines of horizontal "ab\r\rc" and vertical "ab"
- **THEN** horizontal type gets one horizontal segment per non-blank line (none for the blank one), the first at the origin's y and the next two leadings lower, each ending at its line's last caret stop; vertical type gets one vertical segment on the column's centre axis

### Requirement: The caret contrasts with the image

The caret, the baseline guides, and the placement point SHALL draw white when
the image under the caret is dark (mean luminance below half, transparency read
as light) and black otherwise, re-evaluated on every edit and caret move.

#### Scenario: White over a dark image, black over a light one [caret_contrast]

- **WHEN** `tst_type_tools::caretContrastsWithTheImage` opens a Horizontal Type session on a black image, then on a white image
- **THEN** the caret draws white on the black image and black on the white one

### Requirement: The caret follows the paragraph direction

In a right-to-left paragraph, logical caret position 0 SHALL sit at the line's
right edge, and each increment of the logical position SHALL move the caret one
visual character to the left. Line start and line end SHALL map to the visual
right and left edges respectively. A click SHALL map to the nearest logical
boundary at the clicked visual position, and the baseline guides SHALL still run
from the line's visual start to its visual end. Left-to-right paragraphs SHALL
keep their current caret mapping.

#### Scenario: Position zero is at the right edge

- **WHEN** an empty RTL paragraph opens and gains focus
- **THEN** the caret draws at the line's right edge

#### Scenario: Advancing moves left

- **WHEN** the caret is at logical position 0 in an RTL line and moves forward one character
- **THEN** it draws at the left side of the first character

#### Scenario: A click maps to the logical boundary

- **WHEN** an RTL line is clicked between two characters
- **THEN** the caret lands at the logical boundary on the clicked side

#### Scenario: An LTR caret is unchanged

- **WHEN** the same line is laid out left-to-right
- **THEN** logical position 0 sits at the left edge and advancing moves right
