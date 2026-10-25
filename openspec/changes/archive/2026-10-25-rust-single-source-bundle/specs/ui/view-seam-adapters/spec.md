# Spec Delta

## ADDED Requirements

### Requirement: The type adapter is a session façade

The type adapter SHALL stay the only C++ surface over the type bridge, and its
interface SHALL NOT mirror the bridge one to one. Specifically:

- A type layer's text, font, setting, character and paragraph state SHALL cross
  the adapter as one value, both when read and when passed to preview, commit
  and update.
- Re-editing an existing type layer SHALL be one edit session that begins on the
  hit and ends in exactly one commit or cancel. A session dropped unfinished
  SHALL cancel.
- Warp Text preview SHALL be one warp session that ends in exactly one commit or
  cancel. A session dropped unfinished SHALL cancel.
- The character and paragraph style libraries SHALL be read as one list per
  refresh, not by index loops over count, name and setting getters.

Sessions SHALL hold a guarded reference to the document view. A session whose
view was destroyed SHALL end without touching it. A failed commit SHALL also end
the session.

The generated type bridge headers (`type_tools.cxxqt.h` and those under
`type_tools/`) SHALL be included only by the type adapter's own header and
translation unit. No other production header or source, including other
adapters, SHALL include them.

#### Scenario: Re-edit commits once [type_session_commit]

- **WHEN** a type layer is re-edited through an edit session and committed with
  new text
- **THEN** the layer shows the new text, one history state is recorded, and the
  layer is visible again

#### Scenario: Dropped session cancels [type_session_cancel]

- **WHEN** an edit session begins on a type layer and is destroyed without a commit
- **THEN** the layer is restored unchanged and visible, and no history state is
  recorded

#### Scenario: Warp session previews then cancels [type_warp_session]

- **WHEN** a warp session previews a warp on a type layer and is then cancelled or
  dropped
- **THEN** the layer's warp is restored and no history state is recorded. Committing
  instead records one "Warp Text" state.

#### Scenario: Style library reads in one call [type_style_library]

- **WHEN** the Character Styles or Paragraph Styles panel refreshes
- **THEN** it receives every style's name, font and settings from one adapter
  read in library order

#### Scenario: Generated type header stays inside the adapter [type_header_confined]

- **WHEN** the production C++ tree, other than `cpp/api/type_api.{h,cpp}` and
  `cpp/tests/`, is searched for an include of a generated type bridge header
- **THEN** there is none
