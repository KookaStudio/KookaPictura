# ui/channels-panel

## ADDED Requirements

### Requirement: Channel toggle shortcuts act globally

The main window SHALL handle `Ctrl+2` through `Ctrl+5` as document-wide channel
shortcuts whenever an RGB document is open, regardless of which widget holds
keyboard focus, including the canvas. `Ctrl+2` SHALL show every working colour
channel, and `Ctrl+3`, `Ctrl+4`, and `Ctrl+5` SHALL toggle the Red, Green, and
Blue rows using the same rules as the Channels panel's eye column, so the last
visible channel stays visible. The panel SHALL resolve each digit against the
row shortcut it already publishes, and the window SHALL leave the key
unconsumed when no document is open.

#### Scenario: Canvas focus toggles a colour channel [cp_shortcut_individual]

- **WHEN** an RGB document is open and the canvas has keyboard focus
- **THEN** `Ctrl+3` hides the Red channel, pressing `Ctrl+3` again shows it, and
  `Ctrl+4` and `Ctrl+5` do the same for Green and Blue

#### Scenario: The composite key restores every channel [cp_shortcut_composite]

- **WHEN** an RGB document has one colour channel hidden
- **THEN** `Ctrl+2` shows all colour channels
