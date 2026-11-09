# Spec Delta

## MODIFIED Requirements

### Requirement: Marquee Alt press and mid-drag behavior

Alt held at the press that begins a marquee drag SHALL subtract from an existing
selection and SHALL NOT mirror. With no existing selection, Alt at the press
SHALL select the from-centre (alt-pivot) geometry and commit New. The first Alt
press after the drag has begun SHALL switch to pivot-mirror immediately and SHALL
NOT switch to subtract; the rectangle mirrors about the anchor. After an
Alt-at-press subtract, releasing and pressing Alt again during the same drag
SHALL also switch to pivot-mirror. The drag cursor SHALL keep the combine mode
captured at press; a mid-drag mirror SHALL NOT change it. The two behaviours
SHALL NOT apply from a single Alt press.

#### Scenario: First Alt subtracts [uistalt_first_subtract]

- **WHEN** Alt is held at the press that begins a marquee drag with an existing selection
- **THEN** the drag subtracts from the selection and does not mirror the rectangle

#### Scenario: Alt during the drag mirrors [uistalt_drag_mirror]

- **WHEN** Alt is first pressed after a marquee drag has begun
- **THEN** the rectangle mirrors about the anchor, the drag does not switch to subtract, and the captured drag cursor is unchanged

#### Scenario: Second Alt mirrors [uistalt_second_mirror]

- **WHEN** Alt is released and pressed again during the same drag after an Alt-at-press subtract
- **THEN** the rectangle mirrors about the anchor point

#### Scenario: Alt with no selection draws from the centre [uistalt_no_selection_centre]

- **WHEN** there is no existing selection and Alt is held at the press that begins a marquee drag
- **THEN** the rectangle is centred on the press point and the committed selection is created as New
