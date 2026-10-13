# Icon provenance and rename map

Generated from `assets/icons/lucide-map.json` by the vendoring tool. Do
not edit by hand; change the JSON and regenerate.

## Upstream

- **Lucide** 1.50.0 — https://github.com/lucide-icons/lucide
- Icons: `https://github.com/lucide-icons/lucide/tree/v1.50.0/icons`
- License: **ISC**, with a subset derived from Feather under **MIT**;
  full text in `LICENSES/Lucide.txt`.
- Style contract: `viewBox="0 0 24 24"`, `fill="none"`,
  `stroke="currentColor"`, `stroke-width="2"`, round caps/joins.
  Colour is applied at render time (the loader tints by alpha), so the
  vendored files carry no baked colour. Internal shading uses alpha
  gradients / `stroke-opacity`.

## Summary (190 icons)

- Lucide-derived: **115**
- Custom (Lucide-style): **62**
- Note / deferred: **3**
- Out of scope (3D tools, file unchanged): **10**

## Rename map

`source` is `lucide:<slug>` for vendored icons, `custom` for
hand-authored assets, or the note for deferred entries.

| Pictura id | source | match | notes |
|---|---|---|---|
| `count.newGroup` | lucide:folder-plus | good |  |
| `crop.apply` | lucide:check | good |  |
| `crop.cancel` | lucide:ban | good |  |
| `crop.cog` | lucide:settings | good |  |
| `crop.grid` | lucide:grid-3x3 | good |  |
| `crop.swap` | lucide:arrow-left-right | good |  |
| `edit.redo` | lucide:redo-2 | good |  |
| `edit.stepBackward` | lucide:step-back | good |  |
| `edit.stepForward` | lucide:step-forward | good |  |
| `edit.undo` | lucide:undo-2 | good |  |
| `ellipsis` | lucide:ellipsis | exact | the Move options bar's three-dots menu |
| `file.close` | lucide:file-x | approximate |  |
| `file.closeAll` | lucide:files | approximate |  |
| `file.exit` | lucide:log-out | good |  |
| `file.new` | lucide:file-plus | good |  |
| `file.open` | lucide:folder-open | good |  |
| `file.revert` | lucide:file-clock | good |  |
| `file.save` | lucide:save | exact |  |
| `file.saveAs` | lucide:save-all | approximate |  |
| `help.about` | lucide:circle-question-mark | good |  |
| `history.snapshot` | lucide:camera | good |  |
| `image.crop` | lucide:crop | exact |  |
| `image.flipHorizontal` | lucide:triangles-centerline-dashed-vertical | good | shared with flipVertical |
| `image.flipVertical` | lucide:triangles-centerline-dashed-vertical | good | shared with flipHorizontal |
| `image.rotate180` | custom (base: rotate-cw-square) | custom | rotate-cw-square with an added arrow in the bottom-right corner |
| `image.rotate90ccw` | lucide:rotate-ccw-square | good |  |
| `image.rotate90cw` | lucide:rotate-cw-square | good |  |
| `info.bounds` | lucide:scan | good |  |
| `info.crosshair` | custom | custom | same as the existing crosshair readout glyph |
| `info.protractor` | lucide:angle | good |  |
| `layer.alignBottom` | lucide:align-end-horizontal | good |  |
| `layer.alignHorizontalCenter` | lucide:align-center-vertical | good |  |
| `layer.alignLeft` | lucide:align-start-vertical | good |  |
| `layer.alignRight` | lucide:align-end-vertical | good |  |
| `layer.alignTop` | lucide:align-start-horizontal | good |  |
| `layer.alignVerticalCenter` | lucide:align-center-horizontal | good |  |
| `layer.distributeBottom` | lucide:align-vertical-distribute-end | good |  |
| `layer.distributeHorizontalCenter` | lucide:align-horizontal-distribute-center | good |  |
| `layer.distributeLeft` | lucide:align-horizontal-distribute-start | good |  |
| `layer.distributeRight` | lucide:align-horizontal-distribute-end | good |  |
| `layer.distributeTop` | lucide:align-vertical-distribute-start | good |  |
| `layer.distributeVerticalCenter` | lucide:align-vertical-distribute-center | good |  |
| `layers.clipMask` | lucide:corner-left-down | approximate |  |
| `layers.delete` | lucide:trash | exact |  |
| `layers.disclosureDown` | lucide:chevron-down | exact |  |
| `layers.disclosureRight` | lucide:chevron-right | exact |  |
| `layers.eyeOff` | lucide:eye-off | exact |  |
| `layers.eyeOn` | lucide:eye | exact |  |
| `layers.fillAdjustment` | custom | custom | mirror of layers.kindAdjustment, rotated 45 degrees |
| `layers.filterOff` | lucide:toggle-left | good |  |
| `layers.filterOn` | custom (base: toggle-right) | custom | toggle with the knob filled (on state) |
| `layers.fx` | custom | custom | lowercase letters "fx" |
| `layers.group` | lucide:folder | good |  |
| `layers.kindAdjustment` | custom | custom | Lucide-style contrast glyph |
| `layers.kindBackground` | note | — | mis-spec/bug: this is not actually a background layer; follow-up needed |
| `layers.kindGroup` | note | — | there is no kindGroup concept; file kept as-is |
| `layers.kindPixel` | lucide:image | good |  |
| `layers.kindShape` | lucide:vector-square | good |  |
| `layers.kindSmartObject` | custom (base: file) | custom | Lucide file with a filled square in the bottom-left |
| `layers.kindType` | lucide:type | exact |  |
| `layers.link` | lucide:link | exact |  |
| `layers.link2` | lucide:link-2 | exact |  |
| `layers.lockAll` | lucide:lock-keyhole | good |  |
| `layers.lockAlpha` | custom | custom | Lucide-style checkerboard |
| `layers.lockNesting` | note | — | not used; file kept as-is |
| `layers.lockPaint` | custom (base: brush) | custom | brush with the top and bottom sections filled |
| `layers.lockPosition` | lucide:move | approximate |  |
| `layers.mask` | custom | custom | 4:3 rectangle with a dashed circle inside |
| `layers.newLayer` | lucide:square-plus | good |  |
| `layers.reset` | lucide:rotate-ccw | good |  |
| `layers.search` | lucide:search | exact |  |
| `layers.unlink2` | lucide:unlink-2 | exact |  |
| `options.warp` | custom | custom | Free Transform warp-mode toggle; a warped 3x3 grid, drawn on the 24x24 Lucide grid |
| `panel.close` | lucide:x | exact |  |
| `panel.columnsOne` | lucide:chevrons-left | good |  |
| `panel.columnsTwo` | lucide:chevrons-right | good |  |
| `panel.menu` | lucide:menu | exact |  |
| `path.delete` | lucide:trash | exact |  |
| `path.fill` | lucide:paint-bucket | good |  |
| `path.loadSelection` | custom | custom | dashed 4:3 rectangle |
| `path.makeWorkPath` | lucide:workflow | approximate |  |
| `path.newPath` | lucide:pen-tool | good |  |
| `path.stroke` | lucide:pen-line | good |  |
| `path.thumbnail` | lucide:frame | good |  |
| `select.all` | lucide:square-dashed | good |  |
| `select.deselect` | lucide:square-dashed-x | good |  |
| `select.mode.add` | lucide:squares-unite | good |  |
| `select.mode.intersect` | lucide:squares-intersect | good |  |
| `select.mode.new` | lucide:square | good |  |
| `select.mode.subtract` | lucide:squares-subtract | good |  |
| `tool.addanchorpoint` | custom (base: pen-tool) | custom | pen-tool rotated 90 ccw with a plus symbol in the top-left |
| `tool.arthistorybrush` | custom (base: brush) | custom | brush with a squiggle line behind it |
| `tool.backgrounderaser` | custom (base: eraser) | custom | eraser with a scissor symbol in the top-left |
| `tool.blur` | custom (base: droplet) | custom | filled droplet |
| `tool.brush` | custom (base: brush) | custom | brush with all except the middle section filled |
| `tool.burn` | custom (base: hand-helping) | custom | pinching hand pointing left |
| `tool.camerapan` | out-of-scope | — | 3D camera tool, non-scope; file unchanged |
| `tool.cameraroll` | out-of-scope | — | 3D camera tool, non-scope; file unchanged |
| `tool.camerarotate` | out-of-scope | — | 3D camera tool, non-scope; file unchanged |
| `tool.camerawalk` | out-of-scope | — | 3D camera tool, non-scope; file unchanged |
| `tool.camerazoom` | out-of-scope | — | 3D camera tool, non-scope; file unchanged |
| `tool.clonestamp` | lucide:stamp | good |  |
| `tool.colorreplacement` | custom (base: brush) | custom | brush with a square and a bidirectional corner arrow in the top-left |
| `tool.colorsampler` | custom (base: pipette) | custom | pipette base, top bulb filled, crosshair in the top-left |
| `tool.contentawaremove` | lucide:shuffle | good |  |
| `tool.convertpoint` | custom | custom | an angle from the top-left opening 30 degrees to the right |
| `tool.count` | custom | custom | the numerals "123", playfully placed |
| `tool.crop` | lucide:crop | exact |  |
| `tool.customshape` | custom | custom | soft 5-pointed star |
| `tool.deleteanchorpoint` | custom (base: pen-tool) | custom | pen-tool rotated 90 ccw with a minus symbol |
| `tool.directselection` | custom | custom | filled mouse pointer, left edge vertical, right edge 45 degrees |
| `tool.dodge` | custom | custom | circle with a line leaving it, pointing to the bottom-left at 45 degrees |
| `tool.ellipse` | custom | custom | ellipse with a 50% alpha fill |
| `tool.ellipticalmarquee` | lucide:circle-dashed | good |  |
| `tool.eraser` | lucide:eraser | exact |  |
| `tool.eyedropper` | custom (base: pipette) | custom | pipette base with the top bulb fully filled |
| `tool.freeformpen` | custom (base: pen-tool) | custom | pen-tool rotated 90 ccw with a short dashed line to the top-right |
| `tool.gradient` | custom | custom | 4:3 rectangle with an alpha-gradient fill |
| `tool.hand` | lucide:hand | exact |  |
| `tool.healingbrush` | custom (base: bandage) | custom | bandage rotated 45 ccw |
| `tool.historybrush` | custom (base: brush) | custom | brush with an undo glyph behind it |
| `tool.horizontaltype` | lucide:type | exact |  |
| `tool.horizontaltypemask` | custom (base: type) | custom | dashed type |
| `tool.lasso` | lucide:lasso | exact |  |
| `tool.line` | custom | custom | 45-degree line from bottom-left to top-right |
| `tool.magiceraser` | custom | custom | eraser with small plus symbols around it |
| `tool.magicwand` | lucide:wand-sparkles | good |  |
| `tool.magneticlasso` | custom (base: lasso) | custom | sharp-angled lasso with a magnet on the right |
| `tool.marquee` | lucide:square-dashed | good |  |
| `tool.mixerbrush` | custom (base: brush) | custom | brush with a drop in the top-left |
| `tool.move` | lucide:move | exact |  |
| `tool.note` | custom | custom | sticky-note with horizontal lines inside |
| `tool.objectpan` | out-of-scope | — | 3D object tool, non-scope; file unchanged |
| `tool.objectroll` | out-of-scope | — | 3D object tool, non-scope; file unchanged |
| `tool.objectrotate` | out-of-scope | — | 3D object tool, non-scope; file unchanged |
| `tool.objectscale` | out-of-scope | — | 3D object tool, non-scope; file unchanged |
| `tool.objectslide` | out-of-scope | — | 3D object tool, non-scope; file unchanged |
| `tool.paintbucket` | lucide:paint-bucket | exact |  |
| `tool.patch` | custom | custom | rectangle with three lines leaving it on each edge |
| `tool.pathselection` | custom | custom | like direct selection but without the fill |
| `tool.patternstamp` | custom (base: stamp) | custom | stamp with a 3x3 checker icon in the top-left |
| `tool.pen` | custom (base: pen-tool) | custom | pen-tool rotated 90 ccw |
| `tool.pencil` | lucide:pencil | exact |  |
| `tool.perspectivecrop` | lucide:vector-polygon | good |  |
| `tool.polygon` | lucide:pentagon | good |  |
| `tool.polygonallasso` | custom (base: lasso) | custom | sharp pacman lasso (bottom-left corner kept) |
| `tool.quickselection` | custom (base: brush) | custom | zoomed-in brush with a dashed area on the left |
| `tool.rectangle` | custom | custom | 4:3 rectangle with a 50% fill |
| `tool.redeye` | custom | custom | eye with a plus in the top-left |
| `tool.rotateview` | lucide:repeat-2 | good |  |
| `tool.roundedrectangle` | lucide:squircle | good |  |
| `tool.ruler` | lucide:ruler | exact |  |
| `tool.sharpen` | lucide:triangle | approximate |  |
| `tool.slice` | lucide:slice | exact |  |
| `tool.sliceselect` | custom | custom | slice with a plus in the bottom-right |
| `tool.smudge` | custom (base: pointer) | custom | finger pointing to the bottom-left at about 90 degrees |
| `tool.sponge` | custom | custom | sponge glyph (no Lucide equivalent) |
| `tool.spothealingbrush` | custom (base: bandage) | custom | bandage with a dashed circle behind it |
| `tool.verticaltype` | custom (base: type) | custom | type glyph with a thin down arrow on its left |
| `tool.verticaltypemask` | custom (base: type) | custom | dashed type glyph with a thin down arrow on its left |
| `tool.zoom` | custom (base: search) | custom | search glyph with a 50% fill |
| `view.actualPixels` | lucide:scan | approximate | shared with info.bounds |
| `view.fitOnScreen` | lucide:fullscreen | good |  |
| `view.options` | lucide:sliders-horizontal | good |  |
| `view.screenMode.full` | custom | custom | 4:3 rectangle with small corner arrows |
| `view.screenMode.fullWithMenuBar` | lucide:panel-top | approximate |  |
| `view.screenMode.standard` | custom (base: panel-top) | custom | two stacked panel-tops |
| `view.zoomIn` | lucide:zoom-in | exact |  |
| `view.zoomOut` | lucide:zoom-out | exact |  |
| `window.panels.actions` | lucide:play | good |  |
| `window.panels.adjustments` | custom | custom | typical contrast glyph rotated 45 degrees |
| `window.panels.brushes` | custom (base: brush) | custom | brush |
| `window.panels.channels` | custom | custom | three evenly overlapping circles with varying fill |
| `window.panels.character` | lucide:a-large-small | approximate |  |
| `window.panels.cloneSource` | custom (base: stamp) | custom | stamp with a small list in the top-left |
| `window.panels.color` | lucide:palette | good |  |
| `window.panels.glyphs` | lucide:omega | approximate |  |
| `window.panels.gradients` | custom | custom | 4:3 rectangle with an alpha-gradient fill |
| `window.panels.histogram` | custom | custom | audio-lines, bottom-aligned |
| `window.panels.history` | lucide:rotate-ccw-clock | good |  |
| `window.panels.info` | lucide:info | exact |  |
| `window.panels.layers` | lucide:layers | exact |  |
| `window.panels.navigator` | lucide:ship-wheel | good |  |
| `window.panels.notes` | custom | custom | same custom sticky-note as tool.note |
| `window.panels.paragraph` | lucide:pilcrow | exact |  |
| `window.panels.paths` | lucide:spline | good |  |
| `window.panels.patterns` | lucide:grid-3x3 | approximate |  |
| `window.panels.properties` | lucide:settings-2 | approximate |  |
| `window.panels.swatches` | lucide:swatch-book | exact |  |
| `window.panels.tools` | lucide:wrench | good |  |

## Cursors

`assets/cursors/` (77 SVGs) is unchanged by this change; a follow-up
issue covers moving it to a Lucide-style or CC0 cursor set.
