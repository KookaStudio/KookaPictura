## ADDED Requirements

### Requirement: The panel refresh does not scan the installed fonts without type

The debounced panel refresh SHALL NOT scan the installed font families for
Missing Glyph Protection fallback faces while the active document holds no type
layer. The fallback faces MUST be registered before text is laid out, whether
by the refresh of a document that holds type or by a type operation.

#### Scenario: A document without type skips the scan [pr_no_type_no_scan]

- **WHEN** a document without type is created and edited, and the debounced
  refresh runs
- **THEN** the fallback faces are not registered

#### Scenario: A document with type registers the fallbacks [pr_type_doc_registers]

- **WHEN** the active document holds a type layer and the debounced refresh runs
- **THEN** the fallback faces are registered

#### Scenario: Laying out type registers the fallbacks [pr_layout_registers]

- **WHEN** text is laid out for a preview in a document without type, or a
  type layer is rasterized
- **THEN** the fallback faces are registered before the layout

#### Scenario: Type is found at any depth [pr_has_type_depth]

- **WHEN** a document is blank, holds a top-level type layer, holds an empty
  group, or holds a type layer inside a group
- **THEN** the type check reports no type, type, no type, and type

### Requirement: Command refresh does not walk the widget tree for Agent Control

The Agent Control command's enabled state SHALL be decided without searching
the window's whole object tree, and MUST still be enabled when the window has
an Agent Control.

#### Scenario: The Agent Control command stays enabled [pr_agent_provider]

- **WHEN** a window with an Agent Control refreshes its commands
- **THEN** Edit > Preferences > Agent Control is enabled and opens its page

### Requirement: The debounced refresh is bounded on a large document

The debounced panel refresh, including a dock retarget, SHALL complete within
30 ms after an edit on a 16507×16196 document on a typical desktop.

#### Scenario: Refresh after edits on a large document [pr_large_manual]

- **WHEN** a 16507×16196 document is opened and a brush stroke, a marquee, a
  new layer and a duplicate are applied, with PICTURA_PAINT_TIMING=1
- **THEN** every `cxx_refreshPanels (debounced timer)` line, the first one with
  its dock retarget included, reads 30 ms or less
