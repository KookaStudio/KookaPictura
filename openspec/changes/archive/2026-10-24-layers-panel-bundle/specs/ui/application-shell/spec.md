# Spec Delta — ui/application-shell

## MODIFIED Requirements

### Requirement: Canvas transparency checkerboard

The document canvas SHALL draw a two-tone checkerboard behind the document image,
clipped to the document rect, so that any pixel with alpha less than 255 reveals
it. A fully transparent document SHALL show the checkerboard, and the
checkerboard SHALL NOT be drawn outside the document rect. The canvas SHALL NOT
substitute an opaque backdrop for a document whose bottom layer is a Background:
a visible Background covers the checkerboard through its opaque pixels, and
hiding the Background SHALL reveal the checkerboard.

#### Scenario: A transparent document reveals both checker tones

- **WHEN** a fully transparent document is displayed on the canvas
- **THEN** pixels inside the document rect show the two checkerboard tones and
  pixels outside the document rect show the canvas colour

#### Scenario: Opaque content covers the checkerboard

- **WHEN** a document region is fully opaque
- **THEN** that region shows the document pixels and the checkerboard is not
  visible through it

#### Scenario: Hiding the Background reveals the checkerboard [app_shell_hidden_background]

- **WHEN** a document with a visible Background layer has that Background's visibility toggled off
- **THEN** the document rect shows the transparency checkerboard instead of an opaque canvas backdrop
