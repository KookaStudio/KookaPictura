# Channels panel delta

## ADDED Requirements

### Requirement: Temporary Quick Mask channel row

While the active document is in Quick Mask, the Channels panel SHALL append one
temporary row named `Quick Mask` after its durable channel rows. Its thumbnail
SHALL show the current grayscale mask and update after mask edits, while the
composite and color rows SHALL continue to show document pixels. The row SHALL
be removed on exit, SHALL NOT consume a durable alpha channel slot, and SHALL
NOT be renamed, deleted, reordered, or passed to durable alpha-channel commands.
Selecting the Quick Mask row SHALL show its grayscale view. Selecting an
ordinary channel SHALL leave Quick Mask through the normal selection-conversion
path before applying that row's ordinary behavior.

#### Scenario: The temporary row follows mode and edits [cp_quick_mask_row]
- **WHEN** Quick Mask is entered, painted, and exited
- **THEN** exactly one temporary row appears, its thumbnail follows the edited mask, durable channel rows and their thumbnails remain intact, and the temporary row disappears on exit

#### Scenario: Temporary row rejects durable-channel operations [cp_quick_mask_protected]
- **WHEN** the Quick Mask row is current and rename, delete, reorder, or a durable alpha-channel action is attempted
- **THEN** the operation is unavailable or refused and neither coverage nor durable channels change

#### Scenario: Channel navigation leaves Quick Mask cleanly [cp_quick_mask_navigation]
- **WHEN** the Quick Mask row is selected and then an ordinary color or alpha row is selected
- **THEN** the first selection shows grayscale mask coverage and the second exits Quick Mask with its edited coverage preserved as the pixel selection before normal row behavior occurs
