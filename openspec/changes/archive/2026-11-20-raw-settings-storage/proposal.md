# Proposal: store raw develop settings (#401)

## Why

A camera raw open (#400) reads its seed settings from a `<stem>.xmp` sidecar or
the DNG's embedded `crs:` packet, but nothing is ever written back: the
settings chosen in the Pictura Raw dialog are lost once the document opens, and
only the 11 Basic controls are read at all. WF-012 ("Where ACR settings are
stored") has Camera Raw keep each file's settings in a per-user database keyed
by the file's content, or in a sidecar `.xmp` beside it, chosen by a
preference, with the database as the fallback on a read-only volume. #632
reports the same gap from the parity audit.

## What changes

- **`crs:` for every tab.** `pictura_raw_from_xmp` reads, and
  `pictura_raw_to_xmp` writes, every control the Pictura Raw settings model
  holds (Basic, process version, grayscale, white-balance preset, tone curve
  parametric and point curves, Detail, HSL / grayscale mix, Split Toning, Lens,
  Effects, Camera Calibration) under their Camera Raw `crs:` names. Writing
  patches an existing packet in place: every byte outside the edited
  properties is kept, so unknown `crs:` keys, other namespaces, and the packet
  wrapper survive. One table maps each `crs:` name to its `Fltr` key, so the
  `crs:` and `Fltr` vocabularies cannot drift.
- **Two stores.** A per-user **Camera Raw database** (one `<SHA-256 of the
  file>.xmp` record under `$XDG_DATA_HOME/kooka-pictura/Camera Raw/`, so the
  settings follow the file's content across moves and renames) and the
  **sidecar** `<stem>.xmp` beside the raw file. Edit ▸ Preferences ▸ File
  Handling gains **Save Camera Raw Settings In** (Camera Raw Database, the CS6
  default, or Sidecar ".XMP" Files), kept in the session.
- **Read order.** The seed comes from the preferred store, then the other
  store, then the DNG's embedded packet, and every control the record sets
  overrides the as-shot seed. A record whose `crs:WhiteBalance` is "As Shot"
  keeps the as-shot temperature and tint.
- **Write.** OK in the Pictura Raw dialog writes the settings to the preferred
  store, then opens the document. A new **Done** button writes them and opens
  nothing. Cancel and non-interactive opens write nothing. When the sidecar
  cannot be written (a read-only folder), the record goes to the database and
  the status bar says so; when neither can be written, the status bar warns
  and the open goes on.
- **DNG files are never modified.** A DNG uses the same stores as any raw file;
  its embedded packet is read but not rewritten.

## Out of scope (follow-ups)

- Writing settings inside a DNG and `Export Settings To XMP` (#402).
- The rest of the Camera Raw preferences: `Ignore Sidecar ".XMP" Files`,
  camera defaults, cache, JPEG/TIFF handling (#633).
- Settings for the smart-object route stay the smart object's own source
  settings (#1466); nothing here changes them.

## Capabilities

- New: `codec/raw-settings-xmp`, `document/raw-settings-storage`.
- Modified: `document/raw-open` (the seed requirement).

## Provenance

The `crs:` property names are the public Camera Raw XMP namespace
(`http://ns.adobe.com/camera-raw-settings/1.0/`) as written in Lightroom and
Camera Raw sidecars and listed by ExifTool's XMP-crs tag table; the process
version strings (6.7, 5.7, 5.0) and white-balance labels come from the same
table. The database layout (one XMP file per content hash) is our own; CS6's
database format is not documented, and WF-012 leaves it open.
