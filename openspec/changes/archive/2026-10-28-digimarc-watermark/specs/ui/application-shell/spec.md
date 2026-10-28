# Spec Delta

## ADDED Requirements

### Requirement: Detected watermark indicator

A document whose watermark has been read SHALL show a copyright symbol in its
title, as `docs/02-ui-ux/application-frame.md` specifies. The indicator SHALL be
per document and transient: it is set when Read Watermark decodes a payload on
that document and is absent while no watermark has been detected.

#### Scenario: A detected watermark marks the document title [dw_title]

- **WHEN** Read Watermark decodes a payload on a document
- **THEN** that document's title (its tab, float-window, or tile label) carries a
  copyright symbol

#### Scenario: An undetected watermark leaves the title plain [dw_title_plain]

- **WHEN** Read Watermark finds no watermark, or a document has never been read
- **THEN** its title carries no copyright symbol
