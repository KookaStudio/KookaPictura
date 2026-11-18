## ADDED Requirements

### Requirement: Extension-last rule

When a scheme contains an extension field, that field SHALL be the last
non-empty field. A Batch whose Folder destination uses a scheme with an
extension field before another non-empty field SHALL be refused with a
diagnostic naming the rule, and no input SHALL be processed.

#### Scenario: An extension before a serial is rejected [bfn_extension_last]
- **WHEN** the scheme is `Document Name`, `extension`, `3 Digit Serial Number` for a Folder destination
- **THEN** validation fails with the extension-last diagnostic and no file is opened
