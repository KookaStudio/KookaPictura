# Spec Delta

## MODIFIED Requirements

### Requirement: Alpha channels and the footer

The panel SHALL create a black alpha channel (one "New Channel" state), delete
the selected alpha channel (one "Delete Channel" state), save the selection as
a new alpha channel, and load the selected alpha channel as the selection;
alpha rows SHALL continue the Ctrl+number labels. When the document holds 56
channels, the New Channel button SHALL be disabled with a reason. Save
Selection SHALL refuse with a clear non-modal status-bar message rather
than silently doing nothing when the channel budget is full.

#### Scenario: New, delete, save, load [cp_alpha_footer]

- **WHEN** the `tst_channels_panel` test creates and deletes a channel, saves a rectangle selection as a channel, deselects, selects that channel, and loads it
- **THEN** "Alpha 1" appears on Ctrl+6 and is removed with the matching states, the saved channel appears, and loading restores the rectangle as the selection

#### Scenario: The cap disables and refuses [cp_channel_cap]

- **WHEN** the document holds 56 channels
- **THEN** New Channel is disabled with a reason, and Save Selection shows the limit in the status bar without adding a channel
