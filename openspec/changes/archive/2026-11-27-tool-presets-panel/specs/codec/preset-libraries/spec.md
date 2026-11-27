# codec/preset-libraries Specification

## ADDED Requirements

### Requirement: Native tool preset entries retain the full option snapshot

A native Tools preset SHALL preserve the owning tool identifier, the mapped
`.tpl` values, and the complete shell option snapshot as preserved opaque data.
Reading the native set SHALL expose the snapshot without interpreting unknown
fields, and writing the set again SHALL retain those fields unchanged.

#### Scenario: Native tool entries round-trip their snapshot [preset_codec_tool_native]

- **WHEN** a native Tools set containing a tool identifier and an opaque option snapshot is decoded and encoded again
- **THEN** the tool identifier, mapped values, and opaque snapshot are unchanged

#### Scenario: Unknown option fields survive [preset_codec_tool_opaque]

- **WHEN** a native Tools entry carries opaque bytes the codec does not interpret
- **THEN** the bytes are preserved verbatim and a malformed snapshot does not invalidate the entry
