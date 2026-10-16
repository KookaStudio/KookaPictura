# Spec Delta

## MODIFIED Requirements

### Requirement: New document creation
The system SHALL create a new document from a New Document dialog that specifies
name, width, height, color mode, bit depth, and background. When the system
clipboard holds an image, the dialog SHALL open with its Document Type
preselected to Clipboard and its width and height preloaded from that image's
pixel dimensions; with no image on the clipboard the dialog SHALL open on its
default type and leave the Clipboard type unavailable. Creation SHALL be
supported for 8-bit Grayscale and 8-bit RGB with a white or transparent
background; requests outside that set SHALL be rejected rather than produce a
document the codec cannot write. A new document SHALL be untitled, unmodified,
with empty history and no selection. A document SHALL be created only by an
explicit New, Open, or Place action; launch SHALL NOT fabricate a document, so a
normal launch starts with no document open and the document-requiring commands
disabled.

#### Scenario: Create an RGB document
- **WHEN** the user confirms New with an 8-bit RGB document, white background, and dimensions
- **THEN** a new untitled tab opens showing a document of those dimensions filled white

#### Scenario: Create a transparent document
- **WHEN** the user confirms New with a transparent background
- **THEN** the new document's initial composite is fully transparent

#### Scenario: Unsupported mode or depth is rejected
- **WHEN** a document is requested with a mode or depth the codec cannot write
- **THEN** creation fails and no document is added

#### Scenario: Launch fabricates no document [ldl_no_launch_doc]
- **WHEN** the application is launched with no document argument
- **THEN** no document exists until the user runs New, Open, or Place

#### Scenario: An explicit New creates a document after launch [ldl_new_after_launch]
- **WHEN** the user runs New after a launch with no document
- **THEN** a document is created and the document-requiring commands become
  enabled

#### Scenario: An image on the clipboard preloads the New size
- **WHEN** the system clipboard holds an image and the user opens File > New
- **THEN** the Document Type is Clipboard and the width and height fields show the clipboard image's pixel dimensions

#### Scenario: An empty clipboard keeps the default size
- **WHEN** the system clipboard holds no image and the user opens File > New
- **THEN** the Document Type is the default size and the Clipboard type is disabled
