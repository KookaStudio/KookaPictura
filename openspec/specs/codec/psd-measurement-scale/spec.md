# PSD Measurement Scale Specification

## Purpose

Restore document calibration through PSD and PSB image resource 1074 while preserving data from files the application cannot interpret.

## Requirements

### Requirement: Active measurement scale round-trips

The PSD and PSB codec SHALL read and write supported measurement scale
descriptors in resource 1074, using descriptor version 16. An explicitly
applied custom calibration SHALL save its pixel length, logical length, and
Unicode unit label. A new default document SHALL omit 1074. Selecting Default
after loading a custom calibration SHALL remove 1074 on the next save. A
successful dimension-changing resample of a recognized custom scale SHALL be a
calibration edit. Saving its reset SHALL omit 1074. Undo SHALL restore the
original resource and calibration edit intent.

#### Scenario: Edited calibration survives reopen [scale_psd_roundtrip]

- **WHEN** a document with a custom scale is saved and reopened as PSD or PSB through an eager or lazy open
- **THEN** all three calibration values are restored from 1074, and a subsequent explicit Default followed by save and reopen restores the default scale

#### Scenario: Photoshop exchanges a calibrated document [scale_psd_oracle]

- **WHEN** Photoshop saves a custom calibration that Kooka opens, edits, and saves back
- **THEN** Kooka reads the original values and Photoshop reads the edited values

### Requirement: Original resource preservation

Saving a document whose scale has not been edited SHALL preserve the raw 1074
block in either edition. An unsupported, invalid, or truncated descriptor in a
well-framed 1074 block SHALL fall back to the default scale and SHALL preserve
its bytes through unrelated edits. Applying a replacement scale or explicitly
choosing Default SHALL update or remove 1074 while preserving other resource
blocks. Existing transformations of other edited metadata SHALL still apply.
An implicit pixel resample SHALL preserve opaque 1074 in either edition.

#### Scenario: A guide edit preserves the scale resource [scale_psd_guides]

- **WHEN** a document with untouched 1074 has its guides edited and is saved
- **THEN** the guide edit survives reopen and the original 1074 bytes remain intact

#### Scenario: Unrelated edits preserve recognized and opaque resources [scale_psd_preserve]

- **WHEN** files with recognized, unsupported, or malformed 1074 descriptors are saved after unrelated edits in both editions
- **THEN** only the recognized scale is interpreted, every original 1074 block remains byte-identical, and neighboring resources are preserved

#### Scenario: An explicit calibration edit replaces opaque data [scale_psd_replace]

- **WHEN** an Extended user loads a file with an unreadable 1074 and explicitly applies a custom scale or chooses Default
- **THEN** the next save contains the replacement 1074 or removes it for Default, and other resources remain intact
