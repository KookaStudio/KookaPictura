# ui/dialog-chrome Specification

## Purpose
Every Qt-drawn dialog wears Kooka's own frameless chrome, matching CS6's
self-drawn dialog windows, instead of the window manager's frame.

## Requirements

### Requirement: Dialogs are frameless with a drawn title bar

The system SHALL show every top-level Qt-drawn `QDialog` (including message
boxes and input dialogs) as a frameless window
(`Qt::FramelessWindowHint`). The dialog SHALL draw a title bar showing its
window title and a close button that rejects it, plus a 1 px outline. The
dialog's own content SHALL sit below the title bar and SHALL NOT be covered
by it.

#### Scenario: A dialog is frameless [dc_frameless]

- **WHEN** a dialog such as New Document is shown
- **THEN** its window flags include `Qt::FramelessWindowHint` and it has a
  visible title bar whose label is the dialog's window title

#### Scenario: Content stays clear of the title bar [dc_content_below]

- **WHEN** a dialog is shown with the chrome
- **THEN** every child widget of the dialog's layout lies below the title bar

#### Scenario: The close button rejects [dc_close_rejects]

- **WHEN** the title bar's close button is clicked
- **THEN** the dialog finishes with `QDialog::Rejected`

#### Scenario: The title follows the dialog [dc_title_tracks]

- **WHEN** the dialog's window title changes while it is open
- **THEN** the title bar shows the new title

### Requirement: Frameless dialogs move and resize

The system SHALL move a chromed dialog when its title bar is dragged and
SHALL let a resizable dialog be resized from its edges. Where the platform
supports it, the system SHALL use the window system's interactive move and
resize so it works on Wayland.

#### Scenario: Dragging the title bar moves the dialog [dc_drag_moves]

- **WHEN** the title bar is pressed and dragged on a platform without
  system move
- **THEN** the dialog moves by the drag offset

#### Scenario: A fixed-size dialog has no resize edges [dc_fixed_no_resize]

- **WHEN** the pointer hovers the edge of a dialog whose minimum and maximum
  sizes are equal
- **THEN** no resize cursor is shown

### Requirement: Native pickers keep their system frame

The system SHALL leave a dialog the platform draws (a native or portal
`QFileDialog` or `QColorDialog`) and an already-frameless or popup window
untouched.

#### Scenario: A Qt-drawn file dialog gets the chrome [dc_picker_drawn]

- **WHEN** a `QFileDialog` is shown with no native helper available, with or
  without `DontUseNativeDialog`
- **THEN** it is frameless and has a title bar

#### Scenario: Popups and frameless windows are exempt [dc_exempt]

- **WHEN** a `Qt::Popup` dialog or one already flagged frameless is polished
- **THEN** it gets no title bar

### Requirement: Inline warning and info text is dark on yellow

The system SHALL draw inline warning and information text inside dialogs
(any label flagged `notice`) as dark text on a yellow background that covers
only the text, constant across interface brightness levels. The icon beside
the text and any buttons near it SHALL keep their own colours on the
dialog's surface.

#### Scenario: A conflict notice is dark on yellow [dc_notice_yellow]

- **WHEN** the Keyboard Shortcuts dialog reports a conflict under the
  default theme
- **THEN** the notice text sits on yellow in near-black, while the area
  around it keeps the dialog's dark surface
