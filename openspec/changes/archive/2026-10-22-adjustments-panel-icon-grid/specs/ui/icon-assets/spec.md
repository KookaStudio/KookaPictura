# Spec Delta

## ADDED Requirements

### Requirement: Panel item glyph ids

Panel content glyphs SHALL use `<panel>.<item>` icon ids, where `<panel>` is the owning panel and `<item>` is the item the glyph represents (e.g. `adjustments.levels`). Such ids SHALL resolve through `icon(id)` like every other bundled icon and SHALL obey the same provenance and style rules (24×24 Lucide-style, mapped in `assets/icons/lucide-map.json`, present in `assets/pictura.qrc`).

#### Scenario: Adjustments panel item glyphs resolve

- **WHEN** `icon(id)` is called for each of `adjustments.solid-color`, `adjustments.gradient`, `adjustments.pattern`, `adjustments.brightness-contrast`, `adjustments.levels`, `adjustments.curves`, `adjustments.exposure`, `adjustments.vibrance`, `adjustments.hue-saturation`, `adjustments.color-balance`, `adjustments.black-white`, `adjustments.photo-filter`, `adjustments.channel-mixer`, `adjustments.color-lookup`, `adjustments.invert`, `adjustments.posterize`, `adjustments.threshold`, `adjustments.gradient-map`, and `adjustments.selective-color`
- **THEN** each call returns a non-null icon whose pixmap is non-null at 16 px

#### Scenario: Panel item glyphs are mapped

- **WHEN** the provenance guard inspects `assets/icons/` and `assets/icons/lucide-map.json`
- **THEN** every `adjustments.*` file has a map entry and every entry has a file
