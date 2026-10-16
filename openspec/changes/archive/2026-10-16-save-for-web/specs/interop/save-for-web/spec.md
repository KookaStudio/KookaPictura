## ADDED Requirements

### Requirement: Web palettes and encoders

The system SHALL reduce an image to an indexed palette by a Color Reduction
method (Perceptual, Selective, Adaptive, Restrictive, Black and White,
Grayscale) within 2–256 colours, optionally dithered (Diffusion, Pattern,
Noise, with an amount), keeping transparency over a matte when asked, and SHALL
encode it as GIF89a (optionally interlaced) or WBMP that independent decoders
read exactly.

#### Scenario: ImageMagick reads our GIF and WBMP

- **WHEN** the `web_oracle` test reduces a colour field with a clear corner to 64 Selective colours with diffusion, encodes it as GIF (plain and interlaced) and WBMP, and decodes them with ImageMagick
- **THEN** every GIF pixel is its palette colour or transparent where clear, and every WBMP pixel is the black or white its bit says

### Requirement: Save for Web & Devices dialog

File > Save for Web & Devices (Ctrl+Alt+Shift+S) SHALL open a dialog on the
flattened document offering GIF, JPEG, PNG-8, PNG-24, and WBMP with their
settings, CS6's presets, Image Size, and Original / Optimized / 2-Up previews
showing the optimised file's size and download time; Save SHALL write the
optimised file and Done SHALL keep the settings for the next opening.

#### Scenario: Optimise, resize, save, remember

- **WHEN** the `tst_save_for_web` test opens the dialog, reads the default GIF, switches formats and settings, halves the size, picks JPEG Low, saves, chooses PNG-24, clicks Done, and reopens it
- **THEN** the GIF decodes with its clear corner, JPEG at quality 10 is smaller than at 100, PNG-8 keeps to its colour table, PNG-24 without transparency lays the corner over white, WBMP has its header, the image is 32 x 24, the preset sets JPEG at 10, the saved file is the optimised bytes, and the reopened dialog shows PNG-24
