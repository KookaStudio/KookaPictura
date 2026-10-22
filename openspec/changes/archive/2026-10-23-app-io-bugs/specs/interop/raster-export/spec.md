# Spec Delta: interop/raster-export

## ADDED Requirements

### Requirement: Raster Save As preserves document metadata

When a document is saved to JPEG or TIFF, the application SHALL write the
document's EXIF, IPTC-IIM, and XMP metadata into the output container,
preserving the values read by File Info, including Title, ObjectName, and Make.
The write SHALL remain atomic: encode to a temporary file, inject the metadata,
and rename only after both succeed. A failure at any step SHALL leave the
previous file untouched and report failure. PNG, BMP, and WebP are outside this
change and continue to drop metadata.

#### Scenario: JPEG and TIFF keep the File Info fields

- **WHEN** the `metadata.psd` fixture is saved as `.jpg` and `.tif` and inspected with exiftool
- **THEN** each output carries the fixture's Title, ObjectName, and Make values
