# ui/command-refresh-responsiveness Specification

## Purpose
What the shell may do around an edit or a tool switch, so the menu and
options-bar enabled checks, session-cancel reports and region refreshes it runs
stay cheap on a large document.

## Requirements

### Requirement: Menu enabled checks do not measure content

A menu or options-bar enabled check that asks whether layers or the selection
have content SHALL decide it without computing content bounds or copying a
plane. Its answer MUST equal the bounds-based answer. A plane whose bytes did not
change SHALL be read at most once across the checks of any number of refreshes,
and a plane with diverged bands MUST NOT be materialized.

#### Scenario: The content test agrees with the bounds [crr_content_agrees]

- **WHEN** layers that are opaque, empty, covered in one pixel, without alpha,
  zero-width, or missing are tested
- **THEN** each answer equals whether the layer has content bounds, and Align
  and Distribute enable exactly as before

#### Scenario: The coverage test agrees with the bounds [crr_coverage_agrees]

- **WHEN** empty, one-pixel, full, short and banded planes are tested
- **THEN** each answer equals whether the plane has coverage bounds

#### Scenario: An unchanged plane is read once [crr_scan_once]

- **WHEN** the same plane, and a copy of it, are tested three times, then written
  and tested again
- **THEN** the first three tests read it once and the write is seen

#### Scenario: A diverged plane is not materialized [crr_no_materialize]

- **WHEN** a plane with a diverged band is tested
- **THEN** no plane is materialized

#### Scenario: The selection test agrees with the lift [crr_selection_agrees]

- **WHEN** no selection, an empty, a one-pixel, a full and a wrongly sized
  selection are tested for a Transform lift
- **THEN** each answer equals whether the lift finds coverage

### Requirement: A tool switch with nothing live reports no cancel

Cancelling the active session SHALL report `sessionCancelled` only when a
session was live, so a tool switch with nothing live does not refresh every
panel at once.

#### Scenario: Cancelling nothing reports nothing [crr_switch_no_cancel]

- **WHEN** the active session is cancelled with nothing live
- **THEN** `sessionCancelled` is not reported

#### Scenario: Cancelling a stroke reports the cancel [crr_switch_cancels_live]

- **WHEN** the active session is cancelled during a stroke
- **THEN** `sessionCancelled` is reported once

### Requirement: A whole-document region refresh installs its frames

A region refresh whose region is the whole document SHALL install the new
composite and level-0 frame instead of copying them row by row. The result MUST
equal a full recomposite, level 0 MUST NOT share the composite's plane, and the
frame revision SHALL be unchanged.

#### Scenario: A whole-document refresh matches a recomposite [crr_whole_region]

- **WHEN** a layer changes everywhere and the whole document is refreshed as a region
- **THEN** the composite, level 0 and pyramid equal a full rebuild, and level 0
  has a plane of its own

#### Scenario: Actions stay responsive on a large document [crr_large_manual]

- **WHEN** on a real display, with a 16507×16196 document and a new empty layer,
  the user switches tools, drags a Marquee and fills with the Paint Bucket
- **THEN** tool switches and the Marquee respond without a visible stall
