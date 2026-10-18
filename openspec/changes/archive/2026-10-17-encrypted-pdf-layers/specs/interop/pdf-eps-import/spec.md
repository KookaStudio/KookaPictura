## ADDED Requirements

### Requirement: Password-protected Photoshop PDFs reopen with their layers

Opening a PDF encrypted with the standard security handler (RC4 40/128-bit,
AES-128, or AES-256) SHALL ask for its password before anything else, and
either the open (user) or the permissions (owner) password SHALL open it. A
file whose editing data (Pictura's or Photoshop's) the password unlocks SHALL
open with its layers, without the Import PDF dialog; one without editing data
SHALL continue to the Import PDF dialog without asking again. A file with only
a permissions password SHALL open without a prompt. Revert SHALL reopen the
file without asking for the password again.

#### Scenario: A password-protected Photoshop PDF keeps its layers [pdf_encrypted_layers]

- **WHEN** a two-layer document is saved as Photoshop PDF with open password `kooka` at any compatibility level, and reopened by entering `wrong`, then `kooka`
- **THEN** the prompt is shown twice and the document opens with both layers, without the Import PDF dialog

#### Scenario: The permissions password also opens it [pdf_owner_password_opens]

- **WHEN** that file, also given permissions password `owner`, is opened with `owner`
- **THEN** its editing data opens with both layers

#### Scenario: A permissions password alone needs no prompt [pdf_permissions_only_layers]

- **WHEN** a two-layer document saved with only a permissions password is opened
- **THEN** it opens with both layers and no password prompt

#### Scenario: Revert keeps the password [pdf_encrypted_revert]

- **WHEN** a reopened password-protected Photoshop PDF is edited and reverted
- **THEN** it reopens with its layers without a prompt
