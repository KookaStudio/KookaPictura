# Video, Other and Digimarc filters

## Video

![The Video submenu](images/video-menu.png)

- **De-Interlace…** removes the lines of one video field. Choose **Odd Fields** or **Even Fields** to say which field is thrown away, and **Interpolation** (a blend of the lines above and below) or **Duplication** (a copy of the line above) to say how the missing lines are rebuilt.
- **NTSC Colors** limits the colours to those a television can show. It runs at once, without a dialog.

## Other

![The Other submenu](images/other-menu.png)

| Filter | What it does |
|---|---|
| Custom… | Your own filter matrix |
| High Pass… | Keeps edges and removes smooth areas |
| Maximum… | Spreads light areas; choose a square or round footprint |
| Minimum… | Spreads dark areas; choose a square or round footprint |
| Offset… | Shifts the picture |

### High Pass

Choose **Filter ▸ Other ▸ High Pass…** and set a **Radius** in pixels. The picture turns grey with only the
edges showing.

![High Pass at a radius of 4 pixels](images/high-pass.jpg)

A common use is sharpening: apply High Pass to a copy of the layer, cancel the grey with the
[Overlay or Soft Light blend mode](blend-modes.md), and the edges become crisper.

### Maximum and Minimum

**Filter ▸ Other ▸ Maximum…** spreads light areas and **Minimum…** spreads dark areas. Set a
**Radius** from 1 to 500 pixels and choose **Preserve**:

- **Squareness** keeps square corners (the CS6 behaviour, and the default).
- **Roundness** rounds the corners with a circular footprint.

A radius above 100 pixels can use a lot of CPU and run slowly, so the dialog shows a warning.
`Roundness` is slower than `Squareness`, which is why the larger radii are best used on small
selections or masks.

## Digimarc

![The Digimarc submenu](images/digimarc-menu.png)

Digimarc places an invisible watermark in the picture, and reads it back later.

**Embed Watermark…** has:

![The Embed Watermark dialog](images/digimarc-embed.png)

- **Watermark Type**, such as Image ID.
- **Creator ID** and **Image ID**.
- **Watermark Durability:** Low, Medium or High. Higher durability survives more editing but is more visible.

**Read Watermark** looks for a mark in the open picture.

The watermark is an approximation made for Pictura and is not compatible with Digimarc's own service.
