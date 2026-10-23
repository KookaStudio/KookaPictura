## ADDED Requirements

### Requirement: Crop defaults

The Crop tool SHALL start as CS6 does: Classic mode off, Show Cropped Area on,
Auto Center Preview on, Enable Crop Shield on at 75 % opacity, and the overlay
shown automatically.

#### Scenario: A fresh start uses the CS6 defaults [cs_defaults]

- **WHEN** the Crop options and session state are fresh
- **THEN** Classic mode is off, Show Cropped Area and Auto Center Preview are on, and the overlay show mode is Auto

## MODIFIED Requirements

### Requirement: Settings keys

While the Crop tool is active, `P` SHALL toggle Classic mode and `H` SHALL
toggle Show Cropped Area when Classic mode is off (in Classic mode `H` SHALL do
nothing). A real key press SHALL reach these settings ahead of the tool-letter
shortcuts, so the keys SHALL NOT switch to the Pen or Hand tool.

#### Scenario: P and H toggle their settings [cs_keys]

- **WHEN** the Crop tool is active and `P` is typed into the window, then `H`
- **THEN** Classic mode flips, and Show Cropped Area flips only while Classic mode is off, and the active tool stays Crop

### Requirement: Settings persist

The Settings menu's values and the grid menu's overlay show mode and
orientation SHALL persist across restarts. A runtime options patch SHALL update
the open menus and the canvas.

#### Scenario: Settings survive a restart [cs_persist]

- **WHEN** the shield opacity, colour and toggles are changed
- **THEN** the session store holds them and a fresh tool controller restores them

#### Scenario: A runtime patch refreshes the menu [cs_runtime_patch]

- **WHEN** a `crop` options patch sets the shield opacity to 40 %
- **THEN** the Settings menu shows 40 % and the shield paints with 40 % alpha

### Requirement: Classic mode turns the box

In Classic mode a rotation SHALL turn the crop box over the fixed image, with
its handles, guides and shield following it, and pointer hit-testing SHALL use
the turned box. Dragging around the box clockwise SHALL turn it clockwise. The
committed crop SHALL equal the content under the turned box. In Modern mode
the content SHALL keep turning behind a fixed box.

#### Scenario: A turned box is dragged on screen [cs_classic_turn_drag]

- **WHEN** Classic mode is on, a box is turned a quarter turn, and its body is dragged down on screen
- **THEN** the box moves down on screen and the image does not move

#### Scenario: The rotate gesture follows the pointer [cs_classic_turn_gesture]

- **WHEN** Classic mode is on and the pointer sweeps a quarter turn clockwise around the box
- **THEN** the box turns a quarter turn clockwise, and in Modern mode the content turns the same way

#### Scenario: A turned box commits its content [cs_classic_turn_commit]

- **WHEN** Classic mode is on and a 20 x 80 box turned a quarter turn across the edge between two differently filled halves is committed
- **THEN** the document is 20 x 80, with one half at each end and no other content
