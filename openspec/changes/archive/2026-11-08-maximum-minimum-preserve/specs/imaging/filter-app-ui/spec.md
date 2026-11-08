# Spec Delta

## ADDED Requirements

### Requirement: Maximum and Minimum dialog

`Filter ▸ Other ▸ Maximum…` and `Filter ▸ Other ▸ Minimum…` SHALL open the shared filter dialog with `Radius:` (an integer, `1..=500` pixels, default 2) and `Preserve:` (a menu with `Squareness` and `Roundness`, default `Squareness`). The `maximum` and `minimum` kinds SHALL take two slots in order: Radius, then Preserve (`0` Squareness, `1` Roundness). An empty slot list SHALL build radius 2 with `Squareness`, and a non-empty list of any other length SHALL be refused. While the dialog is open and the Radius is above 100, the dialog SHALL show a non-blocking warning that a high radius can use a lot of CPU and run slowly; at or below 100 the warning MUST be hidden. OK SHALL commit exactly one history state labelled `Filter: Maximum` or `Filter: Minimum`, and `Filter ▸ Last Filter` SHALL re-apply the same radius and Preserve choice without reopening the dialog.

#### Scenario: The dialog opens on the CS6 radius and the square default [morph_dialog_defaults]

- **WHEN** the Maximum or Minimum dialog opens on a document
- **THEN** its values are `2, 0`, the Radius box ranges `1..=500`, the Preserve menu lists `Squareness` and `Roundness` with `Squareness` selected, and the warning is hidden

#### Scenario: A radius above 100 warns without blocking [morph_dialog_warning]

- **WHEN** the Radius is raised to 150 and then lowered to 100
- **THEN** at 150 the dialog shows a visible warning naming the CPU cost, and at 100 the warning is hidden

#### Scenario: Roundness commits and Last Filter repeats it [morph_dialog_commit]

- **WHEN** `Roundness` is chosen, OK is pressed, and `Filter ▸ Last Filter` is then invoked
- **THEN** the canvas changes, one `Filter: Maximum` (or `Filter: Minimum`) history state is recorded with Preserve slot `1`, and Last Filter re-applies the same radius and `Roundness`
