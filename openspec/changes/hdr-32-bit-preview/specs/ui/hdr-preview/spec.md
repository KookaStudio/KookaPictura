# Spec Delta

## Purpose

The 32-bit display preview: adjust how an HDR document is shown without changing its stored pixels.

## ADDED Requirements

### Requirement: 32-bit preview is display-only

For a document whose reported depth is 32 bits, the application SHALL apply a per-view preview transform to the current 32-bit source when it builds the canvas frame. The transform SHALL offer `Exposure and Gamma` and `Highlight Compression` and SHALL NOT modify the document composite, the retained source samples, the file, or the history. Values above `1.0` SHALL be tone-mapped for display before the frame is narrowed to 8 bits and passed through the document's display colour transform.

The current source SHALL be the retained `f32` store while it is current and the current composite after an edit, so an edited document never displays its pre-edit pixels. The preview state SHALL reset when a new document is opened.

`Exposure and Gamma` SHALL apply `sign(x*gain)*|x*gain|^(1/gamma)` with `gain = 2^exposure_ev`; `Highlight Compression` SHALL use the same knee roll-off as the conversion method. A dialog Reset SHALL restore the frame produced by the current source with no transform.

The CS6 file-persisted preview white point (PSD/PSB/TIFF) is a recorded ceiling in this change; the preview is per-view.

#### Scenario: Exposure and Gamma changes only the display

- **WHEN** a 32-bit document is displayed and the preview exposure changes
- **THEN** the canvas frame changes, while the document composite, retained samples, and history index stay identical

#### Scenario: Highlight Compression changes only the display

- **WHEN** the preview method changes to Highlight Compression on a 32-bit document
- **THEN** the canvas frame is rebuilt with the roll-off and the document remains unmodified

#### Scenario: Reset restores the stored frame

- **WHEN** the preview is reset after an exposure change
- **THEN** the canvas frame matches the frame built from the current source with the identity transform

#### Scenario: An edited document previews its current source

- **WHEN** a 32-bit document is edited and the preview is changed without an intervening native-store refresh
- **THEN** the display is built from the edited composite, not the stale open-time store

#### Scenario: A non-32-bit document cannot preview

- **WHEN** the active document is 8- or 16-bit
- **THEN** the preview command is disabled and no preview transform runs

### Requirement: 32-Bit Preview Options dialog and status-bar exposure

`View > 32-Bit Preview Options…` SHALL open a modal dialog with a Method control (`Exposure and Gamma`, `Highlight Compression`), Exposure (EV), Gamma, and Reset. The Exposure and Gamma fields SHALL be enabled only for that method; Highlight Compression SHALL expose no numeric fields. The Method SHALL default to Exposure and Gamma with Exposure `0` and Gamma `1.0`. Accepting SHALL apply the values to the view; Cancel SHALL leave the current preview unchanged; Reset SHALL return the dialog and view to the default transform. The command SHALL be enabled only when the active document reports 32-bit depth.

The document status bar SHALL offer a per-view `32-Bit Exposure` slider for 32-bit documents. Moving it SHALL update the same per-view preview exposure, and double-clicking it SHALL reset to `0`.

#### Scenario: The method gates the numeric fields

- **WHEN** the dialog's Method changes between Exposure and Gamma and Highlight Compression
- **THEN** the Exposure and Gamma fields enable only for Exposure and Gamma

#### Scenario: The defaults are neutral

- **WHEN** the dialog opens on a 32-bit document
- **THEN** the Method is Exposure and Gamma, Exposure is `0`, and Gamma is `1.0`

#### Scenario: Accept and cancel

- **WHEN** the dialog is accepted after changing the fields, or cancelled
- **THEN** accepting rebuilds the canvas with the new preview, and cancelling leaves the previous preview in place

#### Scenario: The status-bar slider is per-view

- **WHEN** the status-bar 32-Bit Exposure slider moves and the document is reopened
- **THEN** the canvas exposure changes for that view and the stored file is unaffected
