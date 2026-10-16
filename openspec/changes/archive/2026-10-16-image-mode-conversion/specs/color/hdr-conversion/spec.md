## MODIFIED Requirements

### Requirement: HDR Conversion dialog and Image Mode bit-depth commands

The application SHALL route `Image > Mode > 16 Bits/Channel` and
`Image > Mode > 8 Bits/Channel` on a 32-bit document through an HDR Conversion
dialog and, on accept, invoke the conversion bridge with the dialog's Exposure
(EV) and Gamma values. The dialog SHALL expose only those two fields,
defaulting to Exposure `0` and Gamma `1.0` (no Method control, since only
Exposure & Gamma is implemented). On an 8- or 16-bit document the same
commands SHALL convert without the dialog, as `ui/image-mode-menu` specifies.

The `PictureView` bridge SHALL expose `convert_depth(bits, exposure_ev, gamma)`;
it SHALL call the engine conversion for `bits` 16 or 8, then recomposite and
record one history state labelled `HDR Conversion` when it returns true, and
SHALL return false without mutating the document when it does not.

#### Scenario: The bit-depth commands require a 32-bit document

- **WHEN** the active document is 8- or 16-bit
- **THEN** the HDR Conversion dialog is not opened: choosing another depth
  converts directly and records one `Convert Mode` state

#### Scenario: The dialog converts a 32-bit document

- **WHEN** the dialog is accepted on a 32-bit document after the bridge reports
  success
- **THEN** `document_depth_bits()` reflects the requested depth and one
  `HDR Conversion` history state exists

#### Scenario: A failed conversion does not record history

- **WHEN** the bridge returns false (no document, or a refused depth)
- **THEN** no history state is recorded and the document is unchanged
