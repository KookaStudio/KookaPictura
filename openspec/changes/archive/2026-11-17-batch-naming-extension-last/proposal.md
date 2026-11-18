# Proposal

## Why

Saving picks the file format from the file suffix. A Batch File Naming scheme
with the extension before another field (for example `Document Name`,
`extension`, `3 Digit Serial Number`) renders names such as `a.png001`, and
every overridden Save As step then fails at run time with an unhelpful replay
error (#618). CS6 would save in the step's recorded format regardless of the
name; that needs format selection decoupled from the suffix.

## What Changes

- Batch validation refuses a Folder destination whose File Naming scheme has
  an extension field that is not its last non-empty field, with a diagnostic
  naming the rule. The dialog's OK runs the same validation, so the scheme is
  rejected before any input is processed.

## Capabilities

### Modified Capabilities

- `document/batch-file-naming`: adds the extension-last rule.
