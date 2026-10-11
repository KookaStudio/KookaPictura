# Spec Delta

## ADDED Requirements

### Requirement: Marquee Alt two-stage behavior

During a marquee drag, the first Alt press SHALL only subtract from the existing
selection. A second Alt press during the same drag SHALL switch to pivot-mirror,
so the rectangle mirrors about the anchor. The two behaviours SHALL NOT apply
from a single Alt press.

#### Scenario: First Alt subtracts [uistalt_first_subtract]

- **WHEN** an Alt press begins a marquee drag with an existing selection
- **THEN** the drag subtracts from the selection and does not mirror the rectangle

#### Scenario: Second Alt mirrors [uistalt_second_mirror]

- **WHEN** Alt is pressed a second time during the same drag
- **THEN** the rectangle mirrors about the anchor point
