# Spec Delta

## ADDED Requirements

### Requirement: Writing a repaired external Smart Object record

When an external link is explicitly repaired, the writer SHALL update the selected supported external record's location, filename, file type, and file size to match the replacement while preserving its UUID and external kind. It SHALL also update recognized timestamps and modification times when present, and replace cached external data consistently if present and understood. A record with cached data or associated metadata that cannot be safely updated SHALL be ineligible for repair. It SHALL preserve unrelated linked records, the layers' placed descriptors, and unknown fields in the repaired record. The byte preservation contract for records that were never repaired continues to apply.

#### Scenario: A selective repair round-trips in PSD and PSB [external_record_write]
- **WHEN** the writer saves a repair in a file with multiple records and layers sharing the repaired UUID
- **THEN** the selected record points to the new location with consistent file metadata, the filename, file type, and file size match the replacement, layer references resolve to that same external record, and placed descriptors, unrelated records, and unknown fields retain their bytes

#### Scenario: An unsupported external record stays opaque [external_record_opaque]
- **WHEN** a document contains an external record whose layout cannot be safely rewritten
- **THEN** it remains ineligible for repair and an ordinary save preserves its bytes
