## ADDED Requirements

### Requirement: Work Path and saved paths in PSD

A PSD/PSB save SHALL write the document's Work Path as image resource 1025 and
each saved path, in Paths panel order, as resources 2000 onward named by the
path name, as path records in document-relative 8.24 fixed point. A read SHALL
restore the Work Path and saved paths from those resources. A document whose
paths still match its stored resources SHALL keep their bytes unchanged; other
image resources, including the clipping path (2999), SHALL be preserved.

#### Scenario: psd-tools reads the written paths

- **WHEN** the `path_resource_oracle` test writes a 200×100 document with a closed Work Path triangle and two saved paths (one open and one closed subpath, and an open line) and reads it with psd-tools' own path parser
- **THEN** psd-tools lists resource 1025 unnamed and 2000 `Outline` / 2001 `Path 2` with the same subpaths, closed flags, and anchors

#### Scenario: Round trip and verbatim re-emission

- **WHEN** the codec unit tests write and read paths, re-emit a section whose paths are unchanged, and delete every path
- **THEN** the paths read back within the fixed-point quantization, the unchanged section is byte-identical, and deleting the paths removes their resources while another resource survives
