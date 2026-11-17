# Spec Delta

## MODIFIED Requirements

### Requirement: A document carries a working color profile

The document SHALL expose borrowed read access to the working ICC bytes through
its encapsulated source value, with absence meaning the sRGB working space.
Profile changes SHALL use operations that preserve that value's invariants
rather than writing a public `document_icc` field. A document opened with the
Convert incoming policy (`read_psd`'s behaviour), constructed, or assigned/converted
to sRGB SHALL have no working ICC bytes; a document opened with the Preserve
policy for a non-sRGB embedded profile SHALL expose the embedded bytes as its
working profile because its stored pixels remain in that profile. The working
profile SHALL be distinct from the original profile a read normalised away.
Both records SHALL retain their existing shared byte storage.

#### Scenario: A freshly opened document is in the sRGB working space

- **WHEN** a PSD with no embedded profile (or with an sRGB profile) is read
- **THEN** borrowed access reports no working profile

#### Scenario: A read-normalised document records the source separately

- **WHEN** a PSD with a non-sRGB embedded profile is read with the Convert policy
- **THEN** borrowed access exposes the original embedded bytes and reports no working profile

#### Scenario: A preserved document carries the embedded profile

- **WHEN** a PSD with a non-sRGB embedded profile is read with the Preserve policy
- **THEN** borrowed access exposes the embedded bytes as the working profile, the pixels are unchanged, and no converted-away original profile is recorded
