# The Edit menu

The Edit menu changes pixels and the document's settings. Undo and the clipboard
commands are at the top, the transform commands below them, and the colour,
shortcut and preference commands at the bottom.

## Undo, clipboard and editing

Undo, clipboard, fill and stroke, and the commands that open the transform,
colour and preference dialogs.

| Command | Shortcut | What it does | More |
|---|---|---|---|
| Undo | `Ctrl+Z` | Undoes the last change. |  |
| Redo | `—` | Redoes the last undone change. |  |
| Step Forward | `Ctrl+Shift+Z` | Moves forward one step in the history. |  |
| Step Backward | `Ctrl+Alt+Z` | Moves back one step in the history. |  |
| Fade… | `Shift+Ctrl+F` | Mixes the last change with the picture as it was before it. |  |
| Cut | `Ctrl+X` | Copies the selection to the clipboard and clears it. |  |
| Copy | `Ctrl+C` | Copies the selected pixels to the clipboard. |  |
| Copy Merged | `Shift+Ctrl+C` | Copies the visible pixels from all layers in the selection. |  |
| Paste | `Ctrl+V` | Pastes the clipboard as a new layer. |  |
| Clear | `—` | Clears the selected pixels. |  |
| Find And Replace Text… | `—` | Finds and replaces text in type layers. | [Guide](text-and-typography.md) |
| Fill… | `Shift+F5` | Fills the selection or layer with a colour, pattern or history state. |  |
| Stroke… | `—` | Draws a line around the selection or layer. |  |
| Puppet Warp | `—` | Moves and anchors parts of a raster layer or mask with pins. |  |
| Free Transform | `Ctrl+T` | Starts the full transform box on the selection or layer. |  |
| Auto-Align Layers… | `—` | Lines up several layers that show the same scene. |  |
| Auto-Blend Layers… | `—` | Blends several aligned layers, keeping the sharp parts of each. |  |
| Adobe PDF Presets… | `—` | Manages the PDF save presets. |  |
| Color Settings… | `Shift+Ctrl+K` | Chooses the working colour spaces and conversion policies. |  |
| Assign Profile… | `—` | Gives the document a profile without converting its numbers. |  |
| Convert to Profile… | `—` | Converts the document's numbers to another profile. |  |
| Keyboard Shortcuts… | `Alt+Shift+Ctrl+K` | Edits the keyboard shortcuts. |  |

## Paste Special

**Paste Special** pastes the clipboard in a particular place or into a
selection.

| Command | Shortcut | What it does | More |
|---|---|---|---|
| Paste in Place | `—` | Pastes at the same position the copy came from. |  |
| Paste Into | `Shift+Ctrl+V` | Pastes into the current selection as a mask. |  |
| Paste Outside | `—` | Pastes outside the current selection. |  |

## Puppet Warp

Select an editable 8-bit RGB or Grayscale raster layer or its user-mask
thumbnail, then choose **Edit ▸ Puppet Warp**. Click the preview to place
pins. Leave one pin in place to anchor the mesh and drag another to move
that part of the image. Use **Show Mesh** to hide the triangles, or hold
`H` to hide the pins temporarily. Use **Apply** to commit the deformation
as one undo step, or `Esc` to discard it.

## Transform

**Transform** changes the selected layer or selection. The **Free Transform**
command above gives you the full box; these commands start in one mode. See
[Layers](layers.md) for the transform controls.

| Command | Shortcut | What it does | More |
|---|---|---|---|
| Again | `Shift+Ctrl+T` | Repeats the last transform. | [Guide](layers.md) |
| Scale | `—` | Starts a transform in scale mode. | [Guide](layers.md) |
| Rotate | `—` | Starts a transform in rotate mode. | [Guide](layers.md) |
| Skew | `—` | Starts a transform in skew mode. | [Guide](layers.md) |
| Distort | `—` | Starts a transform in distort mode. | [Guide](layers.md) |
| Perspective | `—` | Starts a transform in perspective mode. | [Guide](layers.md) |
| Warp | `—` | Starts a transform in warp mode. | [Guide](layers.md) |
| Rotate 180° | `—` | Turns the selection or layer by 180 degrees. | [Guide](layers.md) |
| Rotate 90° Clockwise | `—` | Turns the selection or layer 90 degrees clockwise. | [Guide](layers.md) |
| Rotate 90° Counter Clockwise | `—` | Turns the selection or layer 90 degrees counter-clockwise. | [Guide](layers.md) |
| Flip Horizontal | `—` | Mirrors the selection or layer left to right. | [Guide](layers.md) |
| Flip Vertical | `—` | Mirrors the selection or layer top to bottom. | [Guide](layers.md) |

## Purge

**Purge** frees memory the document no longer needs. It cannot be undone.

| Command | Shortcut | What it does | More |
|---|---|---|---|
| Undo | `—` | Frees the undo history. |  |
| Histories | `—` | Frees the stored history states. |  |
| Clipboard | `—` | Frees the clipboard image. |  |
| All | `—` | Frees every purgeable cache. |  |

## Presets

**Preset Manager** edits the brush, swatch and other preset lists. The CS6
migrate/export/import entries are not available yet.

| Command | Shortcut | What it does | More |
|---|---|---|---|
| Preset Manager… | `—` | Edits the brush, swatch, gradient and other preset lists. |  |

## Preferences

**Preferences** opens the settings dialog. The pages are described on the
[Preferences reference](preferences.md); Performance has its own detailed page
at [Performance settings](performance.md).

| Command | Shortcut | What it does | More |
|---|---|---|---|
| General | `Ctrl+K` | Opens Preferences on the General page. |  |
| Interface | `—` | Opens Preferences on the Interface page. |  |
| Performance | `—` | Opens Preferences on the Performance page. |  |
| Units & Rulers | `—` | Opens Preferences on the Units & Rulers page. |  |
| Guides, Grid, & Slices | `—` | Opens Preferences on the Guides, Grid, & Slices page. |  |
| Type | `—` | Opens Preferences on the Type page. | [Guide](text-and-typography.md) |
| Agent Control | `—` | Opens Preferences on the Agent Control page. |  |

## Not yet available

These commands appear in the menu but are greyed out for now. They are listed here so the menu is not a surprise.

- **Edit ▸ Check Spelling…**
- **Edit ▸ Seam-Aware Scale**
- **Edit ▸ Transform Path ▸ Again**
- **Edit ▸ Transform Path ▸ Scale**
- **Edit ▸ Transform Path ▸ Rotate**
- **Edit ▸ Transform Path ▸ Skew**
- **Edit ▸ Transform Path ▸ Distort**
- **Edit ▸ Transform Path ▸ Perspective**
- **Edit ▸ Transform Path ▸ Warp**
- **Edit ▸ Transform Points ▸ Again**
- **Edit ▸ Define Brush Preset…**
- **Edit ▸ Define Pattern…**
- **Edit ▸ Define Custom Shape…**
- **Edit ▸ Purge ▸ Video Cache**
- **Edit ▸ Presets ▸ Migrate Presets…**
- **Edit ▸ Presets ▸ Export Presets…**
- **Edit ▸ Presets ▸ Import Presets…**
- **Edit ▸ Menus…**
- **Edit ▸ Preferences ▸ File Handling**
- **Edit ▸ Preferences ▸ Cursors**
- **Edit ▸ Preferences ▸ Transparency & Gamut**
- **Edit ▸ Preferences ▸ Plug-ins**
- **Edit ▸ Preferences ▸ 3D** (Extended edition)

<!-- pictura-commands
edit.undo
edit.redo
edit.stepForward
edit.stepBackward
edit.fade
edit.cut
edit.copy
edit.copyMerged
edit.paste
edit.pasteSpecial.pasteInPlace
edit.pasteSpecial.pasteInto
edit.pasteSpecial.pasteOutside
edit.clear
edit.check.spelling
edit.find.and.replace.text
edit.fill
edit.stroke
edit.seam.aware.scale
edit.puppet.warp
edit.freeTransform
edit.transform.again
edit.transform.scale
edit.transform.rotate
edit.transform.skew
edit.transform.distort
edit.transform.perspective
edit.transform.warp
edit.transform.rotate180
edit.transform.rotate90cw
edit.transform.rotate90ccw
edit.transform.flipHorizontal
edit.transform.flipVertical
edit.transform.path.again
edit.transform.path.scale
edit.transform.path.rotate
edit.transform.path.skew
edit.transform.path.distort
edit.transform.path.perspective
edit.transform.path.warp
edit.transform.points.again
edit.auto.align.layers
edit.auto.blend.layers
edit.define.brush.preset
edit.define.pattern
edit.define.custom.shape
edit.purge.undo
edit.purge.histories
edit.purge.clipboard
edit.purge.video.cache
edit.purge.all
edit.pdfPresets
edit.presets.preset.manager
edit.presets.migrate.presets
edit.presets.export.presets
edit.presets.import.presets
edit.colorSettings
edit.assignProfile
edit.convertProfile
edit.keyboardShortcuts
edit.menus
-->
