# Spec Delta

## ADDED Requirements

### Requirement: Marquee options persist independently

The Rectangular Marquee and Elliptical Marquee's independent Style, ratio, and
size SHALL be saved and restored separately. A missing elliptical group SHALL
restore Normal Style with the default ratio and size.

#### Scenario: The two marquees keep their own options across a restart [uistalt_marquee_options_persist]

- **WHEN** the Rectangular Marquee is set to Fixed Size 100x50 and the Elliptical Marquee is set to Fixed Ratio 2:1, the session is saved, and a new window opens
- **THEN** the Rectangular Marquee restores Fixed Size 100x50 and the Elliptical Marquee restores Fixed Ratio 2:1
