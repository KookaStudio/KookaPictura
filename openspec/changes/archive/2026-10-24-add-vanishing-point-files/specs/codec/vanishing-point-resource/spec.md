## ADDED Requirements

### Requirement: Vanishing Point plane encoding

The system SHALL encode a Vanishing Point plane set as a versioned,
big-endian record of the planes as drawn: each root plane's corners,
anchor, unit and grid size, and each related plane's parent, parent edge,
angle, side, span and last outline. Decoding SHALL re-solve the planes and
SHALL yield a set equal to the one encoded. Decoding SHALL refuse the whole
record for another version, a truncated or overlong record, an unknown plane
kind, a number that is not finite, a related plane whose parent does not
come before it, or an edge outside 0–3.

#### Scenario: Planes round-trip through bytes [vpr_format]

- **WHEN** `pictura-filters::planes_round_trip_through_bytes` encodes a
  tilted root plane with Grid Size 40, a related plane turned to 60° and
  extended, a crossed (red) root plane, and an empty set, then decodes them
- **THEN** each decoded set equals its original, including the red
  plane's validity

#### Scenario: Malformed records are refused [vpr_refuse]

- **WHEN** `pictura-filters::malformed_plane_bytes_are_refused` decodes a
  record cut short, with a trailing byte, version 2, an unknown kind, a NaN
  corner, a parent after its child, or edge 4
- **THEN** every one decodes to nothing

### Requirement: Vanishing Point image resource

The system SHALL store the planes in an `8BIM` image resource with id 4900
and the Pascal name `Kooka Vanishing Point`; a block with that id and any
other name SHALL NOT be read or replaced. Writing the planes SHALL replace
the block where it stands or append one, and removing them SHALL drop it;
every other block and any unparsed trailing bytes SHALL be kept
byte-for-byte. The block is project-owned: CS6's own plane block is
undocumented, and Photoshop is not required to read this one.

#### Scenario: The resource is replaced in place and others are kept [vpr_resource]

- **WHEN**
  `pictura-codec::the_planes_resource_is_replaced_in_place_and_others_are_kept`
  writes, rewrites and removes the planes in a section holding another
  plug-in's id-4900 block, an XMP block and unparsed trailing bytes
- **THEN** the other plug-in's block is never read as planes, the rewrite
  lands where the first write put it, and removing it restores the
  original section byte-for-byte

#### Scenario: Photoshop opens a file with the block [vpr_photoshop]

- **WHEN** a reviewer opens a PSD, TIFF and JPEG saved with planes in
  Photoshop CS6
- **THEN** each opens without a warning and with its pixels intact

### Requirement: Photoshop resource block in JPEG and TIFF

Saving a JPEG or TIFF SHALL write one Photoshop image-resource block holding
the document's IPTC-IIM and the planes, when it has either: in a JPEG as
APP13 `Photoshop 3.0` segments, continuing in further segments when one
cannot hold it, and in a TIFF as IFD0 tag 34377, replacing any earlier copy.
Opening a JPEG or TIFF SHALL read that block into the document's image
resources, joining a JPEG's APP13 segments in file order, and SHALL leave
out any ICC profile block, since the opened pixels are not converted from
it.

#### Scenario: A JPEG carries the block both ways [vpr_jpeg]

- **WHEN**
  `pictura-codec::the_photoshop_block_round_trips_and_splits_across_segments`
  saves IPTC and 100 001 bytes of planes into a JPEG, and reads a JPEG
  with no APP13
- **THEN** the block spans two APP13 segments and reads back identical;
  the plain JPEG has none

#### Scenario: A TIFF carries the block both ways [vpr_tiff]

- **WHEN** `pictura-codec::the_photoshop_block_round_trips` saves the
  planes into a TIFF, then saves that file again
- **THEN** the block reads back identical, and the second save leaves one
  tag 34377

#### Scenario: An open reads the block without its profile [vpr_open]

- **WHEN** `pictura-codec::raster_open_reads_the_block_without_its_profile`
  opens a JPEG whose block holds an ICC profile, the planes and IPTC, a
  TIFF saved with IPTC and planes, and a PNG
- **THEN** the JPEG yields the planes and IPTC without the profile, the
  TIFF yields IPTC and the planes, and the PNG yields nothing
