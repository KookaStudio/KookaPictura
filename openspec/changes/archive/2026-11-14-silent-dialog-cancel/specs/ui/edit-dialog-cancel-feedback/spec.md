# Spec Delta

## ADDED Requirements

### Requirement: Cancelling an edit dialog reports no refusal

Cancelling a filter, effect, or adjustment dialog SHALL discard its preview,
leave the document and history unchanged, and SHALL NOT report a refusal or
show the status bar's error banner. This applies to every `Filter` menu dialog,
every `Image ▸ Adjustments` dialog including HDR Toning, and any other
dialog-driven effect. A command that is refused before its dialog opens (for
example an empty blur target) or a commit the engine refuses SHALL still report
its refusal.

#### Scenario: Cancelling an adjustment dialog is silent [edc_adjustment_cancel]

- **WHEN** the Levels dialog is opened from `Image ▸ Adjustments` and cancelled
- **THEN** the status bar shows no message, the pixels and history are unchanged, and no refusal is reported

#### Scenario: Cancelling HDR Toning is silent [edc_hdr_cancel]

- **WHEN** the HDR Toning dialog is opened and cancelled
- **THEN** the status bar shows no message and the pixels and history are unchanged

#### Scenario: Cancelling a filter dialog is silent [edc_filter_cancel]

- **WHEN** a filter dialog such as Gaussian Blur is opened from the Filter menu and cancelled
- **THEN** the status bar shows no message and the pixels and history are unchanged

#### Scenario: A genuine refusal still reports [edc_refusal_reported]

- **WHEN** a command is refused before its dialog opens, such as a Blur on an empty layer
- **THEN** its refusal is still reported
