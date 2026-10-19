# document-lifecycle Specification (delta)

## ADDED Requirements

### Requirement: The loading tab shows phase progress

While an open is in flight, the loading tab's empty workspace SHALL show a
centered progress card containing the file name, the current phase, a
determinate progress bar from 0 to 100, the percentage, and a Cancel button.
The application SHALL report the phases `Reading file`, `Decoding`,
`Compositing`, and `Preparing layers`, and for PDF/EPS imports
`Rendering page`, `Rendering image`, or `Rendering EPS`, at the boundaries of
the work they name. The percentages SHALL be phase weights rather than measured
work; the application SHALL NOT claim sub-progress inside a phase. The card
SHALL be hidden when the tab is filled or removed. Its Cancel button SHALL
cancel the open exactly as the tab spinner does, restoring the previous tab and
showing no error. A generic PDF's probe and the Import PDF dialog SHALL show no
card; the card SHALL appear when a chosen page or image render starts.

#### Scenario: The card shows the current phase and percentage [lt_phase_progress]

- **WHEN** a document open is held in flight after its decode
- **THEN** the loading tab shows a centered card with the file name, a phase such as `Preparing layers…`, a bar between 0 and 100, a percentage, and a Cancel button

#### Scenario: The card's Cancel cancels the open [lt_phase_cancel]

- **WHEN** the card's Cancel button is clicked
- **THEN** the loading tab is removed, the worker result is discarded, the previous tab or empty workspace is restored, and no error is shown

#### Scenario: A generic PDF probe shows no card [lt_phase_no_probe]

- **WHEN** a generic PDF is opened
- **THEN** no progress card exists during the probe or the Import PDF dialog, and a card appears when a chosen page or image render starts
