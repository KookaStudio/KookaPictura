## MODIFIED Requirements

### Requirement: A camera raw file is routed by its signature

Opening a document by path SHALL detect a camera raw container (DNG, CR2,
CR3, NEF, ARW, ORF, RW2 or RAF) from the file's signature, whatever its extension, and route it to
the raw open before the raster route. A TIFF without `DNGVersion` and without
a Nikon or Sony make or an Olympus, Panasonic or Fujifilm header, an ISO media file of
another brand, a Photoshop Raw `.raw` file and an unreadable path SHALL
NOT be routed to it. The Open dialog SHALL offer a "Camera Raw (*.DNG *.CR2
*.CR3 *.NEF *.ARW *.ORF *.ORI *.RW2 *.RWL *.RAW *.RAF)" filter, and its All
Formats filter SHALL include all eleven extensions; `.RAW` also names Photoshop Raw, so
it is routed by the signature alone.

#### Scenario: Raw files are detected by signature [raw_open_sniff]

- **WHEN** a synthetic DNG saved as `shot.tif`, a TIFF with make `SONY` saved as `sony.tif`, a TIFF with another make, an `IIRO` file saved as `olympus.tif`, an `IIU\0` file saved as `panasonic.tif` and as `P1000001.RAW`, an ISO media file of brand `crx ` saved as `IMG_0001.mp4`, one of brand `isom` saved as `clip.cr3`, a `FUJIFILMCCD-RAW ` file saved as `fuji.dat`, interleaved pixels saved as `pixels.RAW`, a plain TIFF, and a missing path are tested
- **THEN** only the DNG, the Sony TIFF, the `IIRO` file, the two `IIU\0` files, the `crx ` file and the Fujifilm file are detected as camera raw

#### Scenario: The Open dialog lists Camera Raw [raw_open_filter]

- **WHEN** the Open dialog's filters are read
- **THEN** one is "Camera Raw (*.DNG *.CR2 *.CR3 *.NEF *.ARW *.ORF *.ORI *.RW2 *.RWL *.RAW *.RAF)" and All Formats contains `*.DNG`, `*.CR2`, `*.CR3`, `*.NEF`, `*.ARW`, `*.ORF`, `*.ORI`, `*.RW2`, `*.RWL`, `*.RAW` and `*.RAF`
