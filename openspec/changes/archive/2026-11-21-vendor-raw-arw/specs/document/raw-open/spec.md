## MODIFIED Requirements

### Requirement: A camera raw file is routed by its signature

Opening a document by path SHALL detect a camera raw container (DNG, CR2 or
ARW) from the file's signature, whatever its extension, and route it to the
raw open before the raster route. A TIFF without `DNGVersion` and without a
Sony make, a Photoshop Raw `.raw` file and an unreadable path SHALL NOT be
routed to it. The Open dialog SHALL offer a "Camera Raw (*.DNG *.CR2 *.ARW)"
filter, and its All Formats filter SHALL include all three extensions.

#### Scenario: Raw files are detected by signature [raw_open_sniff]

- **WHEN** a synthetic DNG saved as `shot.tif`, a TIFF with make `SONY` saved as `sony.tif`, a TIFF with another make, a plain TIFF, and a missing path are tested
- **THEN** only the DNG and the Sony TIFF are detected as camera raw

#### Scenario: The Open dialog lists Camera Raw [raw_open_filter]

- **WHEN** the Open dialog's filters are read
- **THEN** one is "Camera Raw (*.DNG *.CR2 *.ARW)" and All Formats contains `*.DNG`, `*.CR2` and `*.ARW`
