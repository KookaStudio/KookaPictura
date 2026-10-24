# Design: add-vanishing-point-files

## Where the planes live

- **The block.** CS6 stores its planes in a block whose layout Adobe has
  not published, and no Photoshop-authored fixtures exist here. So the
  project owns its own block instead of guessing Adobe's: an `8BIM` image
  resource, id 4900, named `Kooka Vanishing Point`.
  - The id is in Photoshop's plug-in range (4000–4999). The name is part of
    the key, so another plug-in's block that happens to use id 4900 is
    neither read nor replaced.
  - ponytail: Photoshop neither reads these planes nor shows them; parity
    with CS6's block needs a Photoshop-authored fixture.
- **Live copy.** The session's `vanishing_planes` stays the live copy. It
  is not written into the live document's image resources, because history
  states snapshot the document's metadata. An undo would then put back
  whatever planes that state saw, although no state records the planes.
  - Installing a document (open, revert, new, duplicate) reads the block.
  - Each save writes the live planes into the bytes it writes: the PSD/PSB
    save copy, the JPEG/TIFF Photoshop block, and the recovery snapshot.
  - Planes drawn on a canvas of another size (the image was resized since)
    are neither saved nor restored, as the dialog already drops them.

## The encoding

- **Format.** Big-endian. A `u16` version (1), `u32` width and height,
  `u32` plane count, then each plane:
  - **Root.** `0u8`, then the four corners, the anchor, the unit and the
    grid size, all `f64`.
  - **Related plane.** `1u8`, its parent's index as a `u32` (it must come
    earlier), the parent edge as a `u8` (0–3), then angle, side, the four
    span fractions and the four corners last shown, all `f64`.
- **What is stored.** The inputs, not the solved frames. Decoding re-solves
  them, so a reopened file yields identical planes (`PartialEq` on the
  whole set).
- **Refusals.** Another version, a truncated or overlong record, an unknown
  plane kind, a number that is not finite, a non-positive unit, a grid
  under 1, a parent that is not earlier, or a fifth edge. A refused block
  leaves the document with no planes.

## JPEG and TIFF

- **Saving.** The block holds IPTC-IIM (1028) when the document has it,
  then the planes. It goes in:
  - a JPEG's APP13 `Photoshop 3.0` segment. Past one segment's capacity
    the block continues in further APP13 segments, as Photoshop splits
    long blocks.
  - a TIFF's IFD0 tag 34377. The existing IPTC tag 33723 is still written.
- **Opening.**
  - A JPEG's APP13 payloads before the scan are joined in file order.
  - A TIFF's IFD0 tag 34377 (BYTE or UNDEFINED) is read.
  - The section becomes the document's image resources, less any ICC
    profile block. A raster open does not convert pixels from that profile,
    so keeping it would mis-tag them. Blocks the parser cannot frame stay
    at the end byte-for-byte.
- **Scope.** EXIF and XMP live in APP1 and their own TIFF tags; reading
  them back is out of scope.

## The modified mark

- **When.** OK sets it when the planes differ from those the session
  opened on, counting a deleted last plane.
- **How.** No history state records the planes, so OK uses `mark_unsaved`:
  the document stays modified until it is saved, even if later undos
  return to the state that was saved.
