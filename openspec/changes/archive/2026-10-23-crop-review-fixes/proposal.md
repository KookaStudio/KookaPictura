# Proposal

## Why

Review of PR #910 (#104) found that:

- `P` and `H` lost to the Pen and Hand tool shortcuts on a real key press.
- The grid menu's show modes and cycle commands did nothing.
- The Crop defaults contradicted CS6.
- A runtime options patch left the menu and canvas stale.
- The turned Classic commit had no pixel check.

## What Changes

- **Keys:** the Crop tool's keys (`P`, `H`, `O`, `Shift+O`) go through the tool-letter shortcut path first, so they win over Pen, Hand and Dodge while Crop is active. `H` in Classic is taken as a no-op.
- **Grid menu:** Auto / Always / Never Show Overlay decide when the guides are drawn. Auto, the default, shows them only while the box is adjusted. Cycle Overlay (`O`) steps the overlay. Cycle Orientation (`Shift+O`) mirrors Triangle and Golden Spiral, and is enabled only for them. Both settings persist with the other Crop settings.
- **Defaults:** they are CS6's: Classic Off, Show Cropped Area On, Auto Center Preview On. The docs table is corrected to match.
- **Runtime patch:** a `crop` options patch notifies, so the menu and canvas follow it.
- **BREAKING**: a fresh install now starts in Modern mode, where Classic was the default. A stored session keeps its stored value.

## Capabilities

### Modified Capabilities
- `tools/crop-settings`: Settings keys, Settings persist, Classic mode turns the box; adds Crop defaults.
- `tools/crop-overlay-feedback`: adds Overlay show mode and cycle commands.
