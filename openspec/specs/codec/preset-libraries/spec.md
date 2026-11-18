# codec/preset-libraries Specification

## Purpose

Exchange preset libraries and color settings with supported Photoshop files, and preserve reusable Pictura preset sets in a versioned native format.

## Requirements

### Requirement: Photoshop preset library import

The codec SHALL decode at least one evidenced Photoshop-compatible variant of each listed format into ordered, named entries with usable typed payloads. It SHALL document the supported version, record and field coverage, and available export for each format. Unsupported records or settings SHALL be reported with the available record name, section, or offset. Duplicate names SHALL preserve separate records in file order.

| Format | Required usable payload |
|---|---|
| `.aco` and `.ase` | Named swatches with source color model and components for supported RGB, CMYK, Lab, and Gray encodings |
| `.abr` | CS6 v10 sampled brush tips with their linked names and supported tip settings |
| `.grd` | Named solid gradients with color and opacity stops, locations and midpoints, including supported foreground/background references |
| `.asl` | Named layer styles with supported effects and their values |
| `.pat` | Named RGB and Grayscale tiles with a pattern identifier, dimensions and pixels |
| `.shc` | Named contours with control points and supported corner flags |
| `.csh` | Named vector paths with original bounds, subpaths and Bézier controls |
| `.tpl` | Named presets for supported existing tools with a tool identifier and mapped option values |

#### Scenario: Each format decodes usable preset entries [preset_codec_import]

- **WHEN** independently generated libraries using the supported Photoshop encodings are decoded
- **THEN** each format returns entries in file order with intact names and the typed values in its required payload

#### Scenario: Partial import reports skipped or partially mapped entries [preset_codec_partial]

- **WHEN** a library contains both independently decodable entries and unsupported records or fields
- **THEN** the result contains the usable entries and reports the unsupported records or fields, identifying their location when available

#### Scenario: Invalid files fail safely [preset_codec_invalid]

- **WHEN** a probe or import encounters a wrong type, unknown version, invalid length or count, excessive nesting, or a truncated payload
- **THEN** it returns a diagnostic and does not read past a record boundary, panic, or allocate in proportion to an unchecked value

### Requirement: Native sets and swatch interchange export

The native set SHALL record its version and type, preserve entry order, names, typed values, and imported opaque data with known record boundaries. It SHALL distinguish an empty set from an absent set. `.aco` and `.ase` export SHALL preserve the supported swatches within the target format's numeric precision. Export of a variant or payload the writer cannot represent SHALL report the limitation before replacing the destination.

#### Scenario: Native sets round-trip for every type [preset_codec_native]

- **WHEN** each type is saved with a reordered set containing Unicode names, duplicate display names and any retained opaque data, and an empty set is also saved
- **THEN** reading back yields the same type, order, names, payloads and retained opaque data, and the empty set reads as a valid empty set

#### Scenario: Swatches are portable through exchange files [preset_codec_swatch_export]

- **WHEN** a set of representable named swatches in supported color models is exported as `.aco` and `.ase` and read back
- **THEN** names, supported models, and values agree within the precision of each format

### Requirement: Standalone color settings interchange

The `.csf` codec SHALL read and write an evidenced CS6-compatible Color Settings variant. It SHALL expose its supported working-space references, policies and conversion options as typed values, preserve unresolved profile references as data, and report unsupported fields or variants. Encoding SHALL report values that cannot round-trip through the chosen variant. Codec operations SHALL run independently of the active settings and installed profiles.

#### Scenario: Color settings are exchanged as values [preset_codec_csf]

- **WHEN** a supported `.csf` is decoded and its representable values are written and read back while a referenced profile is unavailable
- **THEN** the supported values and unresolved profile reference survive and the codec does not change active settings

#### Scenario: Claimed Adobe interoperability is checked in CS6 [preset_codec_cs6_oracle]

- **WHEN** user-authored CS6 files are imported for each claimed variant and Pictura's advertised Adobe exports are opened in CS6
- **THEN** names and supported values transfer in the advertised directions, and documented limitations match what the two applications expose
