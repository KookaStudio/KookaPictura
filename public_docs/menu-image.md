# The Image menu

The Image menu works on the whole picture rather than one layer: its colour mode,
adjustments, size, rotation and analysis commands.

## Mode

**Mode** changes the document's colour model and bit depth. Some conversions
flatten or discard data, so Pictura asks before a lossy change.

| Command | Shortcut | What it does | More |
|---|---|---|---|
| Bitmap | `—` | Converts to one-bit black and white. |  |
| Grayscale | `—` | Converts to a single grey channel. |  |
| Indexed Color | `—` | Converts to an indexed palette with optional forced colours and transparency. |  |
| RGB Color | `—` | Converts to RGB colour. |  |
| CMYK Color | `—` | Converts to CMYK for print. |  |
| Lab Color | `—` | Converts to Lab colour. |  |
| 8 Bits/Channel | `—` | Sets 8 bits per channel. |  |
| 16 Bits/Channel | `—` | Sets 16 bits per channel. |  |
| 32 Bits/Channel | `—` | Sets 32-bit floating-point channels. |  |
| Color Table | `—` | Edits the palette of an indexed image, including its transparent entry. Loads and saves ACT palettes. |  |

## Adjustments

**Adjustments** changes colour and tone on the active layer. The same
adjustments are available as adjustment layers, which stay editable; see
[Adjustments](adjustments.md) and [Adjustment layers](adjustment-layers.md).

| Command | Shortcut | What it does | More |
|---|---|---|---|
| Brightness/Contrast | `—` | Makes the whole image lighter or darker and changes its contrast. | [Guide](adjustments.md) |
| Levels | `Ctrl+L` | Sets the black, white and midtone points with a histogram. | [Guide](adjustments-levels.md) |
| Curves | `Ctrl+M` | Draws a tone curve for precise brightness and contrast. | [Guide](adjustments-curves.md) |
| Exposure | `—` | Changes exposure, offset and gamma like a camera exposure. | [Guide](adjustments.md) |
| Vibrance | `—` | Boosts muted colours more than saturated ones. | [Guide](adjustments.md) |
| Hue/Saturation | `Ctrl+U` | Changes hue, saturation and lightness. | [Guide](adjustments-hue-saturation.md) |
| Color Balance | `Ctrl+B` | Shifts the colour balance between shadows, midtones and highlights. | [Guide](adjustments.md) |
| Black & White | `Alt+Shift+Ctrl+B` | Converts to grey with per-colour mixing. | [Guide](adjustments-black-white.md) |
| Photo Filter | `—` | Adds a coloured filter like a camera filter. | [Guide](adjustments.md) |
| Channel Mixer | `—` | Builds each output channel from a mix of the input channels. | [Guide](adjustments.md) |
| Color Lookup | `—` | Applies a lookup table (LUT) for a film or grade look. | [Guide](adjustments.md) |
| Invert | `Ctrl+I` | Inverts every channel. | [Guide](adjustments.md) |
| Posterize | `—` | Reduces the number of tones per channel. | [Guide](adjustments.md) |
| Threshold | `—` | Turns the image into black and white at a chosen level. | [Guide](adjustments.md) |
| Gradient Map | `—` | Maps the tones to a gradient. | [Guide](adjustments.md) |
| Selective Color | `—` | Changes the amount of ink in a colour range. | [Guide](adjustments.md) |
| Shadows/Highlights | `—` | Lifts shadows or recovers highlights. | [Guide](adjustments-shadows-highlights.md) |
| HDR Toning | `—` | Maps a high dynamic range image into a normal range. | [Guide](adjustments.md) |
| Desaturate | `Shift+Ctrl+U` | Removes colour without changing the colour mode. | [Guide](adjustments.md) |
| Match Color | `—` | Matches the colour of another image or selection. | [Guide](adjustments.md) |
| Replace Color | `—` | Replaces one colour range with another. | [Guide](adjustments.md) |
| Equalize | `—` | Redistributes the tones evenly across the histogram. | [Guide](adjustments.md) |

## Image-wide commands

These commands act on the whole image: size, canvas, rotation, crop and the
document-level tools.

| Command | Shortcut | What it does | More |
|---|---|---|---|
| Auto Tone | `Shift+Ctrl+L` | Sets the black and white points automatically. |  |
| Auto Contrast | `Alt+Shift+Ctrl+L` | Sets the contrast automatically without shifting colours. |  |
| Auto Color | `Shift+Ctrl+B` | Corrects the colour balance automatically. |  |
| Image Size… | `Ctrl+Alt+I` | Changes the pixel size, print size and resolution; resampling can change detail. |  |
| Canvas Size… | `Ctrl+Alt+C` | Changes the canvas size without resampling the image. |  |
| Crop | `—` | Crops the canvas to the selection. | [Guide](image-size.md) |
| Trim… | `—` | Trims the border of a chosen colour or transparency. |  |
| Reveal All | `—` | Grows the canvas to show every layer's full content. | [Guide](image-size.md) |
| Duplicate… | `—` | Makes a copy of the document. |  |
| Apply Data Set… | `—` | Applies one data set to the document. |  |
| Apply Image… | `—` | Blends another document into the active layer. |  |
| Calculations… | `—` | Combines two channels into a new document, channel or selection. |  |

## Image Rotation

**Image Rotation** turns or flips the whole canvas. It is different from
**Edit ▸ Transform**, which turns one layer or selection.

| Command | Shortcut | What it does | More |
|---|---|---|---|
| 180° | `—` | Turns the whole canvas by 180 degrees. |  |
| 90° CW | `—` | Turns the whole canvas 90 degrees clockwise. |  |
| 90° CCW | `—` | Turns the whole canvas 90 degrees counter-clockwise. |  |
| Arbitrary… | `—` | Turns the canvas by an angle you type. |  |
| Flip Canvas Horizontal | `—` | Mirrors the whole canvas left to right. |  |
| Flip Canvas Vertical | `—` | Mirrors the whole canvas top to bottom. |  |

## Variables

**Variables** defines data-driven text and visibility for a template.

| Command | Shortcut | What it does | More |
|---|---|---|---|
| Define… | `—` | Defines the text and visibility variables of a template. |  |
| Data Sets… | `—` | Edits the data sets that fill the variables. |  |

## Analysis > Set Measurement Scale

**Set Measurement Scale** tells the Measure tool what one pixel means, so its
readouts can be in a real-world unit. It is an Extended-edition feature.

| Command | Shortcut | What it does | More |
|---|---|---|---|
| Default | `—` | Uses the default measurement scale. (Extended edition) |  |
| Custom… | `—` | Sets a custom measurement scale. (Extended edition) |  |

## Not yet available

These commands appear in the menu but are greyed out for now. They are listed here so the menu is not a surprise.

- **Image ▸ Mode ▸ Duotone**
- **Image ▸ Mode ▸ Multichannel**
- **Image ▸ Trap…**

<!-- pictura-commands
image.mode.bitmap
image.mode.grayscale
image.mode.duotone
image.mode.indexed.color
image.mode.rgb.color
image.mode.cmyk.color
image.mode.lab.color
image.mode.multichannel
image.mode.8.bits
image.mode.16.bits
image.mode.32.bits
image.mode.color.table
image.adjustments.brightnesscontrast
image.adjustments.levels
image.adjustments.curves
image.adjustments.exposure
image.adjustments.vibrance
image.adjustments.huesaturation
image.adjustments.color.balance
image.adjustments.black..white
image.adjustments.photo.filter
image.adjustments.channel.mixer
image.adjustments.color.lookup
image.adjustments.invert
image.adjustments.posterize
image.adjustments.threshold
image.adjustments.gradient.map
image.adjustments.selective.color
image.adjustments.shadowshighlights
image.adjustments.hdr.toning
image.adjustments.desaturate
image.adjustments.match.color
image.adjustments.replace.color
image.adjustments.equalize
image.auto.tone
image.auto.contrast
image.auto.color
image.image.size
image.canvas.size
image.rotate180
image.rotate90cw
image.rotate90ccw
image.rotate.arbitrary
image.flipHorizontal
image.flipVertical
image.crop
image.trim
image.revealAll
image.duplicate
image.variables.define
image.variables.data.sets
image.apply.data.set
image.trap
image.apply.image
image.calculations
image.analysis.set.measurement.scale.default
image.analysis.set.measurement.scale.custom
-->
