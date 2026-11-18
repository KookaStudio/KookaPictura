# Spec Delta

## MODIFIED Requirements

### Requirement: Adjustment data is carried opaquely in the document model

The document model SHALL retain an adjustment's original four-byte key and
payload without normalization or re-encoding. The retained data SHALL be
accessible independently of the active typed content, through read-only
preservation access. An ordinary pixel layer SHALL have no active adjustment.
The layer's authoritative content SHALL represent either a supported typed
adjustment or an opaque undecodable adjustment, never an independently writable
optional adjustment competing with other content kinds. Core SHALL own the
plain content data and SHALL NOT depend on adjustment execution or PSD decoding.

#### Scenario: AdjustmentData preserves the key and payload verbatim

- **WHEN** an adjustment is imported with a four-byte key and a payload
- **THEN** preservation access returns the exact key and every original payload byte without re-encoding

#### Scenario: Core stays free of an adjustment dependency

- **WHEN** the core crate manifest is inspected
- **THEN** it declares no dependency on adjustment execution or PSD decoding

#### Scenario: A pixel layer has no adjustment

- **WHEN** an ordinary layer without an adjustment is loaded or constructed
- **THEN** its active content is Pixel and its typed adjustment accessor returns no adjustment

### Requirement: Codec preserves adjustment blocks through PSD read/write

`pictura-codec` SHALL recognise the adjustment additional-layer-info keys
(`levl`, `curv`, `brit`, `expA`, `vibA`, `hue2`, `hue `, `blwh`, `phfl`, `mixr`,
`grdm`, `invr`, `nvrt`, `post`, `thrs`, `selc`, `clrL`) and SHALL store each
block's key and payload bytes verbatim in `AdjustmentData`. On write it MUST emit
the same key and payload bytes it read, so a document with adjustment layers
round-trips byte-for-byte and unknown payload fields are not lost.

#### Scenario: Reader captures an adjustment key and payload

- **WHEN** a PSD layer record contains an additional-layer-info block whose key
  is one of the recognised adjustment keys
- **THEN** the resulting layer retains that key and the block's exact payload bytes
  separately from its normalized active content

#### Scenario: Read/write round-trip preserves key and bytes

- **WHEN** a document containing adjustment layers is written with `write_psd`
  and read back with `read_psd`
- **THEN** every adjustment layer's key and payload bytes are identical to the
  originals, including bytes the decoder does not understand
