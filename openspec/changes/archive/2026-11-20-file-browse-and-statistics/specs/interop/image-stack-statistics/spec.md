# Spec Delta

## Purpose

`File > Scripts > Statistics…` automates creating an image stack from selected
files and rendering it with one stack mode, without the user assembling layers
by hand.

## ADDED Requirements

### Requirement: Statistics command and edition

`File > Scripts > Statistics…` (`file.scripts.statistics`) SHALL be enabled with
or without an open document and SHALL open one modal Image Statistics dialog.
The command SHALL be available in the Extended edition only; the Standard
edition SHALL omit it.

#### Scenario: Extended offers the command [iss_command]
- **WHEN** the application runs in the Extended edition
- **THEN** `file.scripts.statistics` exists in `File > Scripts`, is labelled
  `Statistics…`, and is enabled with no document

#### Scenario: Standard omits the command [iss_edition]
- **WHEN** the application runs in the Standard edition
- **THEN** `file.scripts.statistics` is absent from the menu

### Requirement: Statistics merge

Accepting the dialog SHALL decode every selected source, create one new untitled
document whose canvas is the largest source, place each decodable source on its
own layer at native size, combine the layers into one embedded Smart Object, and
apply the selected stack mode. The result SHALL be unsaved and SHALL carry the
stack mode as a Smart Object property. Sources need not share dimensions.

#### Scenario: Mean merges two sources [iss_mean]
- **WHEN** two opaque sources, one solid red and one solid blue, are accepted
  with Mean and align off
- **THEN** the result is one untitled, unsaved document whose single Smart Object
  layer renders the per-channel mean (purple)

#### Scenario: The selected mode is applied [iss_mode]
- **WHEN** the same two sources are accepted with Maximum
- **THEN** the rendered pixel is the per-channel maximum (magenta)

#### Scenario: Align registers the sources [iss_align]
- **WHEN** two sources containing the same feature at translated positions are
  accepted with align on
- **THEN** the stack is registered before reduction, and with align off both
  feature positions remain

### Requirement: Source failures and the two-source minimum

A source that cannot be read or decoded SHALL be skipped and reported. When at
least two sources decode, the document SHALL be created from those sources. When
fewer than two sources decode, no document SHALL be created.

#### Scenario: A failed source is skipped [iss_partial_failure]
- **WHEN** three sources are selected and one cannot be read
- **THEN** the document is created from the two readable sources and the skipped
  file is reported

#### Scenario: Fewer than two decodable sources create nothing [iss_all_fail]
- **WHEN** two sources are selected and only one can be read
- **THEN** no document is created and the skipped file is reported

### Requirement: Remembered statistics choices

On accept, the chosen stack mode and align flag SHALL be recorded in the session,
and the next Statistics invocation SHALL pass them to the dialog.

#### Scenario: The last accepted choice is recorded [iss_remember]
- **WHEN** the dialog is accepted with Median and align on
- **THEN** the session records Median and align on for the next Statistics dialog

### Requirement: Statistics without a document

The command SHALL operate with no open document and SHALL NOT depend on the
active document.

#### Scenario: No document is needed [iss_no_document]
- **WHEN** no document is open and `file.scripts.statistics` is dispatched
- **THEN** the Image Statistics dialog opens and an accepted merge creates a new
  document
