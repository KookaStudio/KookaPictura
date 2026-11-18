# Spec Delta

## Purpose

Place and render PDF and PDF-compatible AI artwork while preserving its editable source.

## ADDED Requirements

### Requirement: Place an embedded vector source

File > Place SHALL accept PDF and PDF-compatible AI. For a multipage source it SHALL let the user select one page, and SHALL use the selected page and crop box for the artwork bounds. On success it SHALL insert one embedded Smart Object through the normal fit and transform flow. The source SHALL retain its original bytes, filename, format, page, and crop. Cancelled and unsupported placements SHALL create no layer or history state.

#### Scenario: Place a selected PDF or AI page [vector_place]
- **WHEN** the user places a valid PDF or PDF-compatible AI file and accepts a page and crop
- **THEN** one fitted Smart Object shows the chosen artwork and enters the normal Place transform flow

#### Scenario: Cancel or fail vector placement [vector_place_refused]
- **WHEN** the user cancels page selection or placement, or a supported-looking file cannot be decoded or rendered
- **THEN** no layer or placement history state remains for that file, and failures are reported through the Place flow

### Requirement: Vector placement preserves source fidelity

The application SHALL use the vector source at the committed output size when it prepares the placed appearance and when scaling commits. It SHALL preserve transparency and apply the layer's composition properties. Save and reopen SHALL preserve the appearance and the original source, including the page, crop, and format. Export Contents SHALL export the current source bytes with their original format. Each authored vector source SHALL have a persistent identity; ordinary duplication shares it and New Smart Object via Copy assigns a different identity. The PSD/PSB writer SHALL group authored vector records by that identity, not by filename or payload equality, and reopening SHALL preserve shared and independent identities.

#### Scenario: Enlarge placed artwork [vector_place_scale]
- **WHEN** a scale commit enlarges the artwork
- **THEN** the new display pixels are rendered from the selected vector page at the new size, with transparency, and undo restores the prior appearance

#### Scenario: Preserve shared and independent source identities [vector_place_identity]
- **WHEN** shared and independent copies with identical filenames and bytes are saved and reopened as PSD and PSB, then one shared source is edited
- **THEN** only the shared instances update and the independent copy retains its source and appearance

#### Scenario: Save and export an embedded vector source [vector_place_roundtrip]
- **WHEN** a placed PDF or PDF-compatible AI is saved and reopened as PSD or PSB and Export Contents runs
- **THEN** the selected page and crop render and the original filename, format, and exact source bytes survive
