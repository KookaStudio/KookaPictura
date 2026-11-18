# PSD and PSB Compatibility

## Purpose

Control whether native saves include a real merged image while preserving document data and supporting file exchange with Photoshop.

## Requirements

### Requirement: File Handling remembers the native compatibility preference

File Handling SHALL offer Always, Ask, and Never for Maximize PSD and PSB File Compatibility. It SHALL persist the selected value across launches and use Always when there is no valid stored value.

#### Scenario: The preference defaults and persists [psd-compat-preference]

- **WHEN** File Handling is opened with no saved choice, and then reopened after changing the choice and restarting
- **THEN** it first shows Always and subsequently shows the saved choice

### Requirement: Interactive native saves resolve compatibility before writing

Save, Save As, and Save a Copy to PSD or PSB SHALL use the preference. Always SHALL write a real merged image. Never SHALL omit the real merged image when the layer data can reconstruct the picture and the file can retain all document-level channels. With the current channel representation, layerless files and files with document-level alpha, spot, or saved-selection channels SHALL retain the image data needed for a lossless reopen.

For a save that can omit the merged image, Ask SHALL offer include, omit, or cancel. The choice SHALL apply only to that save. Cancelling SHALL leave the destination, path, dirty state, history, and ICC save setting intact, including for Save As and Save a Copy. When the image is required to preserve data, the save SHALL retain it without offering omission. The preference SHALL apply only to user-initiated interactive Save, Save As, and Save a Copy. Control-server saves, including pathless and explicit-path calls, and Action replay SHALL retain their existing noninteractive output policy without prompting.

#### Scenario: Always and Never control the merged image [psd-compat-write]

- **WHEN** an eligible layered document is saved to PSD and PSB with Always and Never
- **THEN** Always writes the expected merged pixels, and Never writes editable layers and no real merged image; on the controlled nonuniform fixture it produces a smaller file

#### Scenario: Always restores a real merged image [psd-compat-restore]

- **WHEN** a layered PSD or PSB without a real merged image is reopened, edited, and saved with Always
- **THEN** the file includes a real merged image matching the edited composition and preserves the editable layers

#### Scenario: Required image data survives Never [psd-compat-protected]

- **WHEN** a layerless document or a layered document with document-level channels is saved with Never
- **THEN** the file reopens with its image pixels, layers when present, and all document-level channel pixels

#### Scenario: Ask applies to one save or cancels it [psd-compat-ask]

- **WHEN** an eligible PSD or PSB is saved through Save, Save As, or Save a Copy with Ask
- **THEN** the prompt offers both outcomes on each attempt, accepting either writes the chosen output, and cancelling leaves the destination and document unchanged, including the ICC save setting for Save As and Save a Copy

#### Scenario: A native copy preserves the open document [psd-compat-copy]

- **WHEN** a modified layered document is saved as a native copy with a different merged-image choice
- **THEN** the copy reflects that choice and the open document retains its path, dirty state, history, and pixels

#### Scenario: Other save paths bypass the preference [psd-compat-other]

- **WHEN** Ask or Never is selected and the user saves to another format, the app takes a recovery snapshot, an internal Smart Object edit is saved back to its parent, a control-server save runs with or without an explicit path, or an Action replays a save
- **THEN** no compatibility prompt appears and those paths preserve their existing output behavior, including their native merged-image policy

### Requirement: PSD and PSB exchange preserves the picture and editable layers

For representative layered documents using features supported by Kooka Pictura, PSD and PSB files with and without a real merged image SHALL exchange with Photoshop CS6 and the current Photoshop CC desktop release. The saved layers SHALL remain editable and the document SHALL open with the expected picture.

#### Scenario: Photoshop opens Kooka files [psd-compat-photoshop-opens]

- **WHEN** Kooka PSD and PSB files saved with Always and Never are opened in Photoshop CS6 and current Photoshop CC
- **THEN** each file opens with the expected picture and editable layers, and can be edited, saved, and reopened in Kooka

#### Scenario: Kooka opens Photoshop files [psd-compat-kooka-opens]

- **WHEN** Photoshop CS6 and current Photoshop CC PSD and PSB files saved with Always and Never are opened in Kooka
- **THEN** each file opens with the expected picture and editable layers, and an edited Kooka save can be reopened in the originating Photoshop version

#### Scenario: Adobe omitted-image representation restores a real composite [psd-compat-adobe-fixture]

- **WHEN** a fixture generated with the observed Adobe Never representation is opened, edited, and saved with Always
- **THEN** the real merged pixels match the edited layers and the merged-image metadata agrees with the output
