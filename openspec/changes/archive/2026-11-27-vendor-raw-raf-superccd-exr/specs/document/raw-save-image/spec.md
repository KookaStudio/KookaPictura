## MODIFIED Requirements

### Requirement: Save writes a DNG with the dialog's settings and keeps the dialog open

Accepting Save Options SHALL write the DNG on a worker under a cancellable
progress window, then show the result in the Pictura Raw dialog, which stays
open; no document SHALL open and the Camera Raw settings stores SHALL NOT be
written. The worker SHALL re-read the raw file and refuse it when its content
hash differs from the open's. The DNG SHALL hold the mosaic plane read from
the file (or, with Convert to Linear Image, its demosaiced camera RGB), the
file's colour, crop, orientation, make and model, the dialog's settings as an
XMP packet over the seed record with the white-balance label of
`document/raw-settings-storage`, previews developed with those settings, and,
with Embed Original Raw File, the raw file's bytes. A raw file whose plane
is a SuperCCD grid SHALL be saved as its demosaiced, upright camera RGB
whether or not Convert to Linear Image is checked, since a mosaic DNG cannot
carry the turned lattice. The file SHALL be written
under a temporary name in the target folder and renamed into place.

#### Scenario: Save from the dialog [raw_save_writes_dng]

- **WHEN** a DNG open's dialog clicks Save Image, Save Options is accepted with a new name and Convert to Linear Image, and the dialog is then cancelled
- **THEN** the new file exists and starts with `II*\0`, the dialog's status names it, no document opened, and the Camera Raw database is empty

#### Scenario: Saved contents [raw_save_dng_contents]

- **WHEN** a pending raw from a synthetic DNG is saved as a mosaic with exposure +0.50, a Medium preview and the original embedded
- **THEN** `read_dng` of the output equals `read_raw` of the source, `raw_xmp` of the output reads exposure 0.5, the preview IFD exists, and the embedded original equals the source bytes

#### Scenario: Linear contents [raw_save_dng_linear]

- **WHEN** the same pending raw is saved with Convert to Linear Image and no preview
- **THEN** `read_linear_dng` of the output equals the source's demosaiced camera RGB and there is no preview IFD

#### Scenario: A SuperCCD raw saves linear [raw_save_superccd_linear]

- **WHEN** a pending raw from a synthetic SuperCCD RAF is saved without Convert to Linear Image
- **THEN** `read_linear_dng` of the output equals the source's demosaiced, upright camera RGB
