# Spec Delta: codec/psd-codec

## ADDED Requirements

### Requirement: Bitmap documents with extra channels write

`write_psd` SHALL write a depth-1 Bitmap document that carries extra channels.
While the document is a flat, unedited Bitmap (no layer tree and its retained
packed composite still expands to the working composite), the writer SHALL emit
header mode Bitmap (0) at depth 1 with the retained packed color plane and every
extra channel at the depth-1 packed stride (`ceil(width / 8) * height`),
preserving the channel bytes. When the composite has changed, or the document is
otherwise not a flat unchanged Bitmap, and a merged composite is present, the
writer SHALL fall back to 8-bit RGB and SHALL expand every packed extra channel
to one 8-bit sample per pixel before writing. A Bitmap document without a merged
composite SHALL keep the existing no-image-data behaviour, because the extra
channels ride with the composite. When a merged composite is present, the write
SHALL succeed in both cases and `read_psd` SHALL return the document's extra
channels.

#### Scenario: An unchanged Bitmap keeps its packed extra channel

- **WHEN** a depth-1 Bitmap document with one packed extra channel is written and read back without changing its composite
- **THEN** the output header declares mode Bitmap at depth 1 and the extra channel's selected/unselected pattern survives

#### Scenario: An edited Bitmap falls back to RGB and expands the extra channel

- **WHEN** the composite of a Bitmap document with one packed extra channel is changed and the document is written and read back
- **THEN** the output writes 8-bit RGB and the extra channel has one sample per pixel with the same selected/unselected pattern
