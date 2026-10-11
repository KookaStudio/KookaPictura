# Spec Delta

## ADDED Requirements

### Requirement: Base tool-arrow fill

Every cursor built on the workspace default arrow SHALL fill that arrow black
(`#202020`) beneath its white outline, matching `cursor.workspace.svg`. Any other
art compounded in the same cursor SHALL keep the standard white body fill, and
the arrow SHALL keep the white halo used for legibility on dark canvases. The
arrow geometry SHALL be the same path in every cursor that uses it.

#### Scenario: A tool cursor's arrow is black [svg_cursor_arrow_black]

- **WHEN** a cursor that compounds the workspace arrow (the Move, lasso, patch,
  or content-aware move cursors and `cursor.moveSelection`) is created
- **THEN** its arrow is filled black with the white halo, and the remaining art
  keeps the standard white fill
