## MODIFIED Requirements

### Requirement: Pictura Raw photo information

The dialog SHALL show the photo's name, colour space, bit depth, pixel
dimensions with megapixels, and resolution in ppi for the pending raw file or
the open document, plus the file size when the file is on disk. A raw file's
dimensions SHALL be those the develop produces: its default crop, turned by
its orientation. It SHALL show the camera's f-number, exposure time, ISO, and
focal length in the histogram area when the source metadata provides them,
and SHALL omit a value that is missing; a Fujifilm RAF's metadata SHALL be
the Exif of its camera JPEG. A raw file's resolution SHALL come from its EXIF
when present, else 72 ppi; a document without a resolution resource SHALL
report 72 ppi.

#### Scenario: A raw open shows the raw file's information [rb_photo_raw]

- **WHEN** the raw open dialog is shown for a 16 by 12 DNG named `shot.dng`
- **THEN** the status bar names `shot.dng` and reports 16 by 12 and 16 bits

#### Scenario: A filter shows the document's information [rb_photo_document]

- **WHEN** the filter dialog is shown for a 16 by 16 8-bit RGB document
- **THEN** the status bar reports 16 by 16, 8 bits, sRGB, and 72 ppi

#### Scenario: Missing camera metadata is omitted [rb_photo_missing]

- **WHEN** the source carries no EXIF exposure tags
- **THEN** the histogram's exposure labels are empty

#### Scenario: A RAF's camera metadata comes from its camera JPEG [rb_photo_raf]

- **WHEN** the photo information is read for a RAF whose camera JPEG's Exif names f/8, ISO 100 and 240 ppi
- **THEN** it reports f/8, ISO 100 and 240 ppi

#### Scenario: The size is the developed size [rb_photo_size]

- **WHEN** a 40 by 30 raw has a 32 by 24 default crop, upright and then with orientation 6
- **THEN** the reported size is 32 by 24, then 24 by 32
