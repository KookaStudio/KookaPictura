# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pdf_encrypted_layers` | Save a two-layer document as a protected Photoshop PDF at every compatibility level (and Fast Web View), read its editing data with the password; in the window, save with open password `kooka`, open, answer `wrong` then `kooka` | Two layers at every level; `""` and `wrong` are refused; two prompts, no Import PDF dialog, two layers, the composite pixels shown | `pictura-codec::password_protected_pdf_reopens_with_its_layers`, `pictura-codec::the_user_and_owner_passwords_decrypt_what_was_encrypted`, `tst_import_pdf::protectedPicturaPdfKeepsLayers` |
| `pdf_owner_password_opens` | Read the editing data of a file with open `kooka` and permissions `owner` using `owner` | Two layers | `pictura-codec::password_protected_pdf_reopens_with_its_layers`, `pictura-codec::the_user_and_owner_passwords_decrypt_what_was_encrypted` |
| `pdf_permissions_only_layers` | Save with only a permissions password; read with the empty password; open it in the window | Two layers, no prompt, no Import PDF dialog | `pictura-codec::permissions_password_alone_reopens_without_a_password`, `tst_import_pdf::permissionsOnlyPdfOpensWithoutPrompt` |
| `pdf_encrypted_revert` | After the protected reopen, delete a layer, Revert | Two layers again, no further prompt | `tst_import_pdf::protectedPicturaPdfKeepsLayers` |
| A password-protected PDF saved by Photoshop | Save a layered PSD from Photoshop as Photoshop PDF with an open password; open it in Pictura | Password prompt, then the layers, no Import PDF dialog | manual: needs Photoshop to produce the file; Adobe-generated files are not committed as fixtures |
