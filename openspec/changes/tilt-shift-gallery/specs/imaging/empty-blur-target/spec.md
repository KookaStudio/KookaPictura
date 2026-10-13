## ADDED Requirements

### Requirement: Blurs refuse an empty target

Every `Filter ▸ Blur` and `Filter ▸ Blur Gallery` command SHALL refuse to
run when the active pixel layer has no non-transparent pixel inside the
selection, or none at all when there is no selection (a layer without a
transparency channel is never empty; a layer mask does not count). The
refusal SHALL happen before any dialog or workspace opens, on the menu,
Last Filter, and Last Filter Settings paths, SHALL add no history state,
and SHALL show a critical message box reading "Could not complete the
`<command>` command because the selected area is empty." for the Blur
family and "Could not complete the `<command>` command because the initial
bounding rectangle is empty." for the Blur Gallery, where `<command>` is the
menu label without its ellipsis.

#### Scenario: An empty layer refuses every blur

- **WHEN** a new empty layer is active and each Blur and Blur Gallery
  command is chosen, then Last Filter after a Tilt-Shift commit
- **THEN** each shows its error once, no dialog opens, and the history is
  unchanged

#### Scenario: A selection over transparency is empty

- **WHEN** a layer has pixels only outside the selection and Gaussian Blur
  is chosen
- **THEN** the "selected area is empty" error is shown, and with a
  selection over the pixels the Gaussian Blur dialog opens instead
