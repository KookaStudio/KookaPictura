## MODIFIED Requirements

### Requirement: A camera raw file is routed by its signature

Opening a document by path SHALL detect a camera raw container (DNG, CR2 or
NEF) from the file's signature, whatever its extension, and route it to the
raw open before the raster route. A TIFF without `DNGVersion`, a Photoshop
Raw `.raw` file and an unreadable path SHALL NOT be routed to it. The Open
dialog SHALL offer a "Camera Raw (*.DNG *.CR2 *.NEF)" filter, and its All
Formats filter SHALL include all three extensions.

#### Scenario: Raw files are detected by signature [raw_open_sniff]

- **WHEN** a synthetic DNG saved as `shot.tif`, a plain TIFF, and a missing path are tested
- **THEN** only the DNG is detected as camera raw

#### Scenario: The Open dialog lists Camera Raw [raw_open_filter]

- **WHEN** the Open dialog's filters are read
- **THEN** one is "Camera Raw (*.DNG *.CR2 *.NEF)" and All Formats contains `*.DNG`, `*.CR2` and `*.NEF`
