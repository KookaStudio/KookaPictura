# smart-object-link-repair Specification

## Purpose
Repair supported external Smart Object links in imported PSD and PSB documents through an explicit file selection.

## Requirements

### Requirement: Imported external links retain their saved preview

Opening an imported PSD/PSB SHALL preserve external Smart Object records and display their stored layer previews when present. The codec SHALL NOT dereference the recorded path on open or decode. Relink to File SHALL be offered for supported external records and SHALL permit the user to select a replacement, including when the original is missing. Embedded, alias, unresolved, and unsupported external records SHALL be ineligible.

#### Scenario: Open with the link unavailable [external_link_open]
- **WHEN** a PSD/PSB with a supported external record and stored layer preview is opened without its original asset
- **THEN** the document opens with that preview and Relink to File is available

### Requirement: Link repair is a single undoable update

Relink to File SHALL validate and render only the file explicitly selected by the user. A supported replacement SHALL decode successfully, match the recorded format family and source dimensions, and retain the existing placement page and crop when applicable. PSD and PSB form one family; PDF, PDF-compatible AI, PNG, JPEG, and TIFF each form a separate family. Original dimensions SHALL come from recognized placed descriptor `Sz  ` / `Wdth` / `Hght` fields and page from `PgNm`; applicable crop SHALL come from a recognized, unambiguously decoded placed descriptor field. The old path and transformed preview SHALL NOT be used to infer them. Records with missing, ambiguous, or inconsistent required fields across shared instances SHALL be ineligible. A format-family, dimension, page, or crop mismatch SHALL be refused before mutation. A successful repair SHALL update the external record and the previews of all layers that share that record in the document, preserving layer geometry, masks, effects, and Smart Filter settings. It SHALL record one Relink to File history state. Undo and Redo SHALL restore the corresponding link and previews together. A cancelled, unreadable, unsupported, or stale repair SHALL leave both links and history unchanged.

#### Scenario: Relink shared instances [external_link_apply]
- **WHEN** the user selects a compatible supported file from the row command
- **THEN** the clicked record and its shared instances update together, independent sources retain their appearance, and Undo and Redo restore both the link and the previews

#### Scenario: Cancel or fail link repair [external_link_refused]
- **WHEN** the picker is cancelled, the replacement is unreadable or incompatible, or the target changed before commit
- **THEN** there is no preview, record, or history change, and failures are reported

#### Scenario: Reject an incompatible replacement [external_link_incompatible]
- **WHEN** a replacement has a different format family or source dimensions, or cannot satisfy the recorded page and crop
- **THEN** repair reports failure and leaves the record, previews, and history unchanged

#### Scenario: Reopen with the repaired record [external_link_reopen]
- **WHEN** a repaired document is saved and reopened without the replacement available
- **THEN** the new recorded path, external kind, shared source identity, and updated preview survive and the object can be relinked again
