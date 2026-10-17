# document-lifecycle Specification (delta)

## ADDED Requirements

### Requirement: Open shows a loading banner

The application SHALL display a loading banner while a document is being opened:
a boxed indicator at the window's lower-left corner showing `Opening <file
name>…`. The banner SHALL use the theme's notice style — a rounded box on the
notice yellow with the dark notice text — so it stays legible over the canvas.
The banner SHALL be shown for every open entry point — File > Open, Open Recent,
a file drop, and Open As Smart Object — and for both PSD/PSB (native reader) and
raster (image import) sources. The application SHALL paint the banner before the
blocking read/decode begins, so it is visible while the open runs; the load
itself MAY remain synchronous on the GUI thread. When the open finishes the
application SHALL hide the banner and restore the wait cursor, whether the open
succeeded or was refused. A successful open that reports a notice (for example
`Converted from CMYK; saved as CMYK`) SHALL show that notice in the status bar.

#### Scenario: Every open entry point shows the loading banner [olb_banner]

- **WHEN** a document is opened through File > Open, Open Recent, a file drop, or Open As Smart Object, from a PSD/PSB or a raster source
- **THEN** while the read/decode is running the lower-left banner is visible showing `Opening <file name>…` for that file and the wait cursor is active

#### Scenario: The banner is a rounded notice-coloured box [olb_style]

- **WHEN** the application stylesheet is built
- **THEN** the loading banner has a rule giving it the notice background, the dark notice text, and rounded corners

#### Scenario: The banner clears when the open finishes [olb_clears]

- **WHEN** an open finishes, whether it succeeded or was refused
- **THEN** the banner is hidden and the wait cursor is restored

#### Scenario: A notice replaces the banner [olb_notice]

- **WHEN** an open succeeds and the opened document reports a notice
- **THEN** the banner is hidden and the status bar shows that notice
