# icon-assets Specification

## Purpose
A Lucide-derived, render-time-tinted SVG icon set covering the app, tools, and
menus, bundled through Qt resources and resolved by id.

## Requirements

### Requirement: Icon set coverage
The system SHALL provide an SVG icon for the application, for each implemented
tool, and for each command that has a registered handler. Every shipped icon
SHALL be either a Lucide-derived asset (vendored from a pinned Lucide release)
or a Lucide-style custom asset, and every custom asset SHALL use the 24×24
viewBox, `fill="none"`, `stroke="currentColor"`, `stroke-width="2"`, and round
caps and joins, expressing any internal shading as an alpha gradient. Icon files
SHALL live under `assets/icons/` and SHALL be named exactly by their icon id
(the command id verbatim, or `tool.<tool>` for tools; the application icon is
`app`).

#### Scenario: Every implemented command has an icon
- **WHEN** the implemented-command id set is enumerated
- **THEN** an SVG file exists for each id

#### Scenario: Every tool has an icon
- **WHEN** the tool set is enumerated
- **THEN** an SVG file exists for each `tool.<tool>` id

#### Scenario: Every icon is Lucide or Lucide-style
- **WHEN** each bundled SVG is inspected
- **THEN** it declares `viewBox="0 0 24 24"`, `fill="none"`, and
  `stroke="currentColor"` with `stroke-width="2"`

### Requirement: Resource-bundled icon loading
The system SHALL bundle the SVG icons into the executable through a Qt resource
and SHALL expose `QIcon icon(const QString& id)` that resolves the id to a
non-null icon, plus `QIcon icon(const QString& id, const QColor& color)` that
resolves it in an explicit colour. The system SHALL NOT alter the icon id set
when changing icon artwork, so the bundled resource list is unaffected.

#### Scenario: A known id resolves
- **WHEN** `icon(id)` is called for a bundled id
- **THEN** it returns a non-null `QIcon`

#### Scenario: An explicit colour is honoured
- **WHEN** `icon(id, color)` is called and its pixmap is sampled
- **THEN** the rendered opaque pixels use `color`, not the theme foreground

#### Scenario: An unknown id is null
- **WHEN** `icon(id)` is called for an id with no asset
- **THEN** it returns a null `QIcon` without crashing

### Requirement: Application icon
The system SHALL set the application/window icon from the `app` asset at
startup.

#### Scenario: Window icon is set
- **WHEN** the application starts
- **THEN** the main window's icon is the application icon

### Requirement: Tool and menu icons are displayed
The system SHALL display the tool icon on each Tools-panel action and the command
icon on each implemented menu action.

#### Scenario: Tools panel shows icons
- **WHEN** the Tools panel is shown
- **THEN** every tool button carries its tool icon

#### Scenario: Menu actions show icons
- **WHEN** a menu containing an implemented command is opened
- **THEN** that action carries its command icon

### Requirement: Icons are scalable
The system SHALL load icons from vector SVG so they render crisply at the
application's device pixel ratio and at the sizes the widgets request.

#### Scenario: HiDPI request
- **WHEN** an icon is requested at 2x device pixel ratio
- **THEN** the returned icon provides a suitably sized pixmap rather than a blurry upscale

### Requirement: Panel and path glyph set

The system SHALL provide standalone SVG assets for the Layers-panel glyphs
`layers.search`, `layers.kindShape`, `layers.kindSmartObject`, and
`layers.reset`, for the Paths-panel glyphs `path.thumbnail`, `path.fill`,
`path.stroke`, `path.loadSelection`, `path.makeWorkPath`, `path.newPath`, and
`path.delete`, and for the panel-group corner glyph `panel.menu`. Each asset SHALL
use the 24×24 grid, SHALL use `stroke="currentColor"` so it is tinted at render
time and needs no baked colour, SHALL be registered in the Qt resource
`assets/pictura.qrc`, and SHALL resolve through `icon(id)` like every other
bundled icon.

#### Scenario: The new Layers glyphs resolve [plg_layers_resolve]

- **WHEN** `icon(id)` is called for each of `layers.search`,
  `layers.kindShape`, `layers.kindSmartObject`, and `layers.reset`
- **THEN** each call returns a non-null icon whose pixmap is non-null at 16 px

#### Scenario: The Paths glyphs resolve [plg_path_resolve]

- **WHEN** `icon(id)` is called for each of `path.thumbnail`, `path.fill`,
  `path.stroke`, `path.loadSelection`, `path.makeWorkPath`, `path.newPath`, and
  `path.delete`
- **THEN** each call returns a non-null icon whose pixmap is non-null at 16 px

#### Scenario: The corner menu glyph resolves [plg_panel_menu_resolve]

- **WHEN** `icon("panel.menu")` is called
- **THEN** it returns a non-null icon whose pixmap is non-null at 16 px and the
  asset is recorded in `assets/icons/lucide-map.json`

#### Scenario: An unknown id stays null [plg_unknown_null]

- **WHEN** `icon(id)` is called for an id with no bundled asset
- **THEN** it returns a null icon without crashing

### Requirement: Icons follow the active theme

The system SHALL render each icon in the active theme's foreground colour at
render time, SHALL render a disabled icon in the palette's disabled colour, and
SHALL produce updated pixmaps when the theme brightness changes so no stale
colour survives.

#### Scenario: Theme foreground is applied [lia_foreground]
- **WHEN** an icon is rendered with the default call
- **THEN** its opaque pixels use the palette's foreground colour

#### Scenario: Disabled mode differs [lia_disabled]
- **WHEN** `icon(id).pixmap(size, QIcon::Disabled)` is rendered
- **THEN** it differs from the `QIcon::Normal` pixmap

#### Scenario: Brightness change updates icons [lia_theme_change]
- **WHEN** the brightness level is changed and the same icon rendered again
- **THEN** the second pixmap differs from the first

### Requirement: Icon provenance and licensing

The system SHALL ship the Lucide license text under `LICENSES/Lucide.txt`
(covering the ISC license and the Feather-derived MIT subset), SHALL reference
the icon assets from `NOTICE.md` and the generated `THIRD-PARTY-LICENSES`, and
SHALL record every bundled icon's provenance (Lucide slug or custom) in
`assets/icons/lucide-map.json` and `docs/dev/icon-provenance.md`. A mechanical
guard SHALL fail when an icon file under `assets/icons/` is absent from the map.

#### Scenario: License text is present [lia_license]
- **WHEN** the repository is inspected
- **THEN** `LICENSES/Lucide.txt` exists and names the ISC license and the
  Feather-derived MIT subset

#### Scenario: Every icon is mapped [lia_mapped]
- **WHEN** the guard compares `assets/icons/*.svg` with `assets/icons/lucide-map.json`
- **THEN** every file has an entry and every entry has a file

#### Scenario: An unmapped icon fails the guard [lia_guard]
- **WHEN** a new SVG is added under `assets/icons/` without a map entry
- **THEN** the provenance guard exits non-zero

### Requirement: Panel item glyph ids

Panel content glyphs SHALL use `<panel>.<item>` icon ids, where `<panel>` is the owning panel and `<item>` is the item the glyph represents (e.g. `adjustments.levels`). Such ids SHALL resolve through `icon(id)` like every other bundled icon and SHALL obey the same provenance and style rules (24×24 Lucide-style, mapped in `assets/icons/lucide-map.json`, present in `assets/pictura.qrc`).

#### Scenario: Adjustments panel item glyphs resolve

- **WHEN** `icon(id)` is called for each of `adjustments.solid-color`, `adjustments.gradient`, `adjustments.pattern`, `adjustments.brightness-contrast`, `adjustments.levels`, `adjustments.curves`, `adjustments.exposure`, `adjustments.vibrance`, `adjustments.hue-saturation`, `adjustments.color-balance`, `adjustments.black-white`, `adjustments.photo-filter`, `adjustments.channel-mixer`, `adjustments.color-lookup`, `adjustments.invert`, `adjustments.posterize`, `adjustments.threshold`, `adjustments.gradient-map`, and `adjustments.selective-color`
- **THEN** each call returns a non-null icon whose pixmap is non-null at 16 px

#### Scenario: Panel item glyphs are mapped

- **WHEN** the provenance guard inspects `assets/icons/` and `assets/icons/lucide-map.json`
- **THEN** every `adjustments.*` file has a map entry and every entry has a file
