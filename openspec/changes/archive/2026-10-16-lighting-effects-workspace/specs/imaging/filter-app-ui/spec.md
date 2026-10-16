## MODIFIED Requirements

### Requirement: New filter-kind mapping

The filter mapping SHALL additionally recognise `lighting-effects`, `diffuse` (one mode slot, default Normal), and `glowing-edges` (width, brightness, smoothness; defaults 2, 6, 1), each with an empty slot list producing its defaults. `lighting-effects` SHALL take 9 rig slots (colorize r, g, b, exposure, gloss, metallic, ambience, texture, height) followed by 13 slots per light (type, visible, colour r, g, b, intensity, hotspot, centre x, centre y, angle, size, width, elevation). It SHALL accept any whole number of lights from 1 to 16, report the one-light total (22) as its arity, and default to CS6's Default style. `diffuse` and `glowing-edges` SHALL refuse a non-empty list of the wrong length, and `lighting-effects` SHALL refuse a list that is not a whole rig of 1 to 16 lights. These kinds SHALL drive `Filter ▸ Render ▸ Lighting Effects`, `Filter ▸ Stylize ▸ Diffuse`, and `Filter ▸ Stylize ▸ Glowing Edges`.

#### Scenario: The new kinds resolve and default

- **WHEN** each new kind is resolved with an empty slot list
- **THEN** the corresponding `Filter` with the listed defaults is produced

#### Scenario: Wrong arity is refused

- **WHEN** a new kind is resolved with a slot list of the wrong length, or `lighting-effects` with a partial light or 17 lights
- **THEN** no filter is produced and the command returns false without changing the document

#### Scenario: A multi-light rig maps

- **WHEN** `lighting-effects` is resolved with the rig slots and two lights, a red Spot and a hidden Infinite light
- **THEN** the `Lighting` carries the rig's colorize, texture, and height and both lights in order, with their type, colour, centre, width, angle, elevation, and visibility

## ADDED Requirements

### Requirement: Lighting Effects workspace

`Filter ▸ Render ▸ Lighting Effects…` and `Filter ▸ Last Filter Settings` (when the last filter was Lighting Effects) SHALL open a dedicated Lighting Effects workspace, not the generic slot dialog. Its options bar SHALL hold a Presets menu (CS6's 17 styles, from `2 o'clock Spotlight` to `Triple Spotlight`, then `Custom`; `Default` selected), buttons that add a Spot, Point, or Infinite light, Reset, a Preview checkbox (on), Cancel, and OK. Below the bar, the picture SHALL fill the workspace with the rig rendered on it and on-canvas controls over it. The selected Spot SHALL show its ellipse, hotspot ellipse, and four axis handles. The selected Point SHALL show its radius ring. The selected Infinite light SHALL show its disc and end handle. Every light SHALL show its centre handle inside an Intensity ring. Dragging SHALL move, turn, stretch, re-hotspot, resize, aim, or change the intensity of the selected light. Pressing another light's centre SHALL select it, and Alt-dragging SHALL duplicate the selected light. To the right, a Properties panel SHALL hold the selected light's type, Color, Intensity, and Hotspot (disabled unless a Spot), then the rig's Colorize, Exposure, Gloss, Metallic, Ambience, Texture, and Height (disabled while Texture is None), each number with a slider in step. Below it, a Lights panel SHALL list each light (`Spot Light 1`, `Point Light 1`, …) with an eye that hides it, and a trash button that deletes the selected light. The rig SHALL keep 1 to 16 lights, with the add buttons disabled at 16 and the trash disabled at one. Choosing a preset SHALL load its rig, and any edit SHALL switch Presets to Custom. The workspace SHALL produce the `lighting-effects` slots for the whole rig. OK SHALL commit one history state. Cancel SHALL leave the document and history untouched.

#### Scenario: The workspace follows CS6's layout and defaults

- **WHEN** the workspace opens on a document
- **THEN** its values are the Default style's 22 slots, Presets shows 18 entries with Default selected, the type list reads Spot / Point / Infinite, Hotspot is enabled and Height disabled, Lights lists `Spot Light 1` with the trash disabled, the options bar with OK and Cancel sits above the picture, and Properties sits above Lights to the right of it

#### Scenario: Presets, lights, and properties edit the rig

- **WHEN** `RGB Lights` is chosen, Gloss is changed, Reset is pressed, a Point and an Infinite light are added, lights are added until the cap, and lights are deleted down to one
- **THEN** the rig holds three coloured Spots, Presets reads Custom after the edit and Default after Reset, the new lights are named and selected with the slot count following, the add buttons disable at 16, and the last light cannot be deleted

#### Scenario: On-canvas controls edit the selected light

- **WHEN** a Spot is dragged by its body, its long-axis handle, its short-axis handle, its Intensity ring, and beyond its ellipse, another light's centre is pressed, the selected light is Alt-dragged, and an Infinite light's end handle is dragged
- **THEN** the centre moves, size and width grow, the intensity follows the ring and the Properties field, the angle turns, the pressed light becomes selected, a copy is added and selected, and the Infinite light's angle and elevation follow the handle

#### Scenario: Commit, cancel, and reopen

- **WHEN** a Point is added and OK pressed from the Filter menu, a separate session is cancelled after choosing a preset, and Last Filter Settings is opened after a two-light commit
- **THEN** one history state is added with last params equal to the workspace's slots, the cancelled session leaves pixels and history unchanged, and the reopened workspace reads the committed rig with Presets at Custom
