## ADDED Requirements

### Requirement: Filters take validated parameters

`filter` SHALL accept a `params` array of the filter's slot values. It SHALL answer
`invalid_param`, changing nothing, for a wrong count, a non-finite value, or a
value outside the slot's range. It SHALL answer `refused` for a filter that needs
a resource. `list_filters` SHALL report each kind with its parameters' ranges.

#### Scenario: Parameters reach the filter [ce_filter_params]

- **WHEN** a client calls `filter` with `gaussian-blur` and a valid radius
- **THEN** it applies and history gains one state

#### Scenario: Bad parameters are refused [ce_filter_invalid]

- **WHEN** a client passes too few values, then a value above the maximum
- **THEN** each is `invalid_param` and history is unchanged

#### Scenario: Filters are listed [ce_filter_list]

- **WHEN** a client calls `list_filters`
- **THEN** each entry names its kind and parameters, and Displace is marked unavailable

### Requirement: Adjustments apply with values

`adjust_apply` SHALL apply an adjustment to the active layer as one history state
from the kind's defaults plus a `values` object. An unknown kind or refused key
SHALL be `invalid_param`. `list_adjustments` SHALL report the kinds and their pages.

#### Scenario: An adjustment applies [ce_adjust_apply]

- **WHEN** a client calls `adjust_apply` with `posterize`
- **THEN** pixels change and history gains one state

#### Scenario: An unknown adjustment is refused [ce_adjust_invalid]

- **WHEN** a client names an unknown kind
- **THEN** the response is `invalid_param`

#### Scenario: Adjustments are listed [ce_adjust_list]

- **WHEN** a client calls `list_adjustments`
- **THEN** every listed kind has a non-empty page

### Requirement: Commands run with parameters

`run_command` SHALL run a listed command through its replay path, opening no
dialog. An unlisted id SHALL be `refused` and an unlisted parameter key
`invalid_param`. A resize beyond `ImageBudget` SHALL be `invalid_param` before
allocating.

#### Scenario: A resize runs without a dialog [ce_run_size]

- **WHEN** a client runs `image.image.size` with a width and height
- **THEN** the document has that size and no dialog appeared

#### Scenario: An unlisted command is refused [ce_run_unlisted]

- **WHEN** a client runs `file.save`
- **THEN** the response is `refused`

#### Scenario: An unlisted key is refused [ce_run_key]

- **WHEN** a client passes `in` to `image.image.size`
- **THEN** the response is `invalid_param`

#### Scenario: An oversized resize is refused [ce_run_budget]

- **WHEN** a client resizes to 30000 by 30000
- **THEN** the response is `invalid_param` and the size is unchanged

### Requirement: Layers are addressable by path

`layer_tree` SHALL list every layer with its path, depth, name and kind, and
SHALL report the active path. `set_active_layer` SHALL select a layer by path and
SHALL answer `invalid_param` for a path that does not resolve.

#### Scenario: A nested layer is visible [ce_layer_tree]

- **WHEN** a document has a group
- **THEN** `layer_tree` reports it with its depth, and `list_layers` reports `active`

#### Scenario: The active layer is set by path [ce_layer_active]

- **WHEN** a client sets a valid path, then an invalid one
- **THEN** the first becomes active and the second is `invalid_param`

### Requirement: Export never changes the document

`export_png` SHALL write a flattened PNG without changing the document, its path,
its dirty state, or its history, and SHALL pass the file checks a save passes.

#### Scenario: Export leaves the document alone [ce_export]

- **WHEN** a client exports a dirty document
- **THEN** the file exists, and the path, dirty state and history are unchanged

#### Scenario: Export is gated like a save [ce_export_gate]

- **WHEN** a session without the Save grant exports
- **THEN** the response is `refused` and no file is written

### Requirement: Requests can guard against stale state

`history` SHALL list states and snapshots, and SHALL jump, snapshot, and restore.
Listing SHALL be Read; the others SHALL be Edit. `edit` with `expected_index`
SHALL answer `stale_history` when history has moved. A mutator with `doc` SHALL
answer `stale_document` when another document is active.

#### Scenario: History is listed and jumped [ce_history]

- **WHEN** a client lists history and jumps back
- **THEN** the labels are reported and the document returns to that state

#### Scenario: A stale undo is refused [ce_stale_history]

- **WHEN** `edit` carries an old `expected_index`
- **THEN** the response is `stale_history` and nothing is undone

#### Scenario: A stale document is refused [ce_stale_document]

- **WHEN** a mutator carries the `doc` of an inactive document
- **THEN** the response is `stale_document`

### Requirement: Each call is one undo step

A call that changes a document SHALL add at most one history state. `selection`
op `lasso` SHALL take its points in one call, up to the lasso limit.

#### Scenario: Each mutator adds one state [ce_one_state]

- **WHEN** each mutator runs once
- **THEN** history grows by one

#### Scenario: A lasso is one call [ce_lasso]

- **WHEN** a client sends `lasso` with points, then with too many
- **THEN** the first selects and adds one state, and the second is `invalid_param`

### Requirement: An agent can finish its own crop

`commit_crop` SHALL commit a crop that this session's `pointer` request started.
A pending crop the person started SHALL stay refused with `human_busy`.

#### Scenario: A person's crop stays theirs [ce_crop_person]

- **WHEN** a person has a pending crop and a live session calls `commit_crop`
- **THEN** the response is `human_busy` and the crop stays pending
