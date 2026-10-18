# Proposal: encrypted-pdf-layers

## Why

Owner feedback on #627: a layered document saved as Photoshop PDF with
"Require a password to open the document" reopens as a single flattened
layer. The editing data is in the file, but the reader refused any encrypted
PDF and fell back to the Import PDF dialog's rasterized page. Photoshop asks
for the password and reopens such a file with its layers.

## What Changes

- The PDF object reader decrypts a file through the standard security handler
  (ISO 32000-1 §7.6.3 algorithms 1, 6, and 7; ISO 32000-2 algorithm 2.A):
  RC4 40/128-bit, AES-128, and AES-256 (revisions 2–6). Either the user or the
  owner (permissions) password opens it.
- `pictura_codec::pdf::editing_psd` returns the editing data's PSD (Pictura's
  payload, else Photoshop's), decrypting with a password, or reports that
  there is none or that the password is wrong.
- `pdf_open` takes a password and returns `ok`, `none`, `password`, or
  `wrongpassword`. Opening a protected PDF asks for the password before
  anything else; a file with editing data opens with its layers, one without
  goes to the Import PDF dialog with the password already given. A file with
  only a permissions password opens without a prompt. Revert reuses the
  password, which is kept in memory only.

## Capabilities

### Modified Capabilities

- `interop/pdf-eps-import`: a new requirement for password-protected
  Photoshop PDFs.

## Impact

- No new dependency (MD5, AES, and SHA-2 are already used by the writer).
- Copying a type layer with Edit > Copy and pasting it still gives a pixel
  layer: CS6's clipboard carries pixels only (`docs/05-layers/layer-management-ui.md`).
