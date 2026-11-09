# ui/tool-options-persistence Specification

## Purpose
Every tool's options-bar settings saved in the session store and restored at launch.

## Requirements

### Requirement: Tool options survive a restart

Every tool's options-bar settings SHALL be saved in the session store and
restored at the next launch: the Move, Type, Shape, Pen, brush (size, hardness,
opacity, flow, mode, tip, dynamics), selection, eraser, stamp, healing, patch,
retouch and toning (each tool its own), gradient, bucket, eyedropper, mixer,
color replacement, art history, and red eye options. They SHALL be saved on
close, on quit, and on a tool switch. A missing group or field SHALL restore the
group's default, and a restored value SHALL be clamped as the options bar clamps
it. Restoring SHALL NOT notify any change listener. Clone Source points and
values that mirror the open document SHALL NOT be saved.

#### Scenario: Options survive a restart [top_round_trip]

- **WHEN** `tst_tool_options_persistence::optionsSurviveARestart` unchecks Show Transform Controls, sets the type size, brush size and opacity, the marquee feather, the Blur strength, and the Burn exposure, saves the session, and opens a new window
- **THEN** the new window's tools report every changed value, Sharpen keeps its own default strength, and Dodge its own default exposure

#### Scenario: Bad entries keep the defaults [top_bad_entries]

- **WHEN** `tst_tool_options_persistence::badEntriesKeepDefaults` restores a store with an out-of-range type size, a non-object group, and an unknown key
- **THEN** the type size is clamped, the other groups keep their defaults, and nothing fails

#### Scenario: Restore does not notify [top_silent_restore]

- **WHEN** `tst_tool_options_persistence::restoreDoesNotNotify` attaches listeners to the brush, selection, and crop groups and applies a stored object
- **THEN** every value is restored and no listener fires

#### Scenario: A missing field restores its default [top_missing_default]

- **WHEN** `tst_tool_options_persistence::missingFieldsRestoreDefaults` applies a stored group that omits fields the group held before
- **THEN** each omitted field reads its default, not the previous value

### Requirement: Marquee options persist independently

The Rectangular Marquee and Elliptical Marquee's independent Style, ratio, and
size SHALL be saved and restored separately. A missing elliptical group SHALL
restore Normal Style with the default ratio and size.

#### Scenario: The two marquees keep their own options across a restart [uistalt_marquee_options_persist]

- **WHEN** the Rectangular Marquee is set to Fixed Size 100x50 and the Elliptical Marquee is set to Fixed Ratio 2:1, the session is saved, and a new window opens
- **THEN** the Rectangular Marquee restores Fixed Size 100x50 and the Elliptical Marquee restores Fixed Ratio 2:1
