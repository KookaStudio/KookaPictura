# Design: encrypted-pdf-layers

## Decisions

### D1 Decrypt the whole object table, then read as before

`PdfObjects::decrypt(password)` derives the file key once, decrypts every
string and stream in place with each object's own key (object and generation
numbers, now recorded by the scanner), drops `/Encrypt` from the trailer, and
expands the object streams the encryption hid. The Pictura payload and the
Photoshop rebuild then read the decrypted table through the same code as an
unencrypted file. Not decrypted: the `/Encrypt` dictionary itself,
cross-reference streams, and the metadata stream when `/EncryptMetadata` is
false.

### D2 One key schedule for both directions

The writer's revision 2–4 steps (owner RC4 key, file key, user entry, object
key) become shared functions, so the reader checks a password against the
exact bytes the writer produced. Revisions 5 and 6 unwrap the file key from
`/UE` or `/OE` with AES-256-CBC (zero IV, no padding).

### D3 The prompt moves ahead of the editing-data check

The window asks for the password when `pdf_open` reports `password`, and asks
again on `wrongpassword`; Cancel opens nothing. `none` (no editing data, or a
security handler other than the standard one) goes to the Import PDF flow,
handing over the accepted password so Ghostscript is not asked twice.

### D4 Revert

The accepted password is kept on the view (memory only, never in the session
file) so Revert reopens the file without asking.
