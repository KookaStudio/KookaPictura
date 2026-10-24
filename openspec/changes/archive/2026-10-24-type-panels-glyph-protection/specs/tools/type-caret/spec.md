# Spec Delta

## ADDED Requirements

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
