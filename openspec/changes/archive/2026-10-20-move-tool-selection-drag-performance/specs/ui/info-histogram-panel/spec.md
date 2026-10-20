# Spec Delta

## ADDED Requirements

### Requirement: Cursor moves refresh only cursor readouts

A pointer move over the canvas SHALL refresh only the readouts that depend on
the pointer position: the position block, the colour blocks, and the selection
size block. The document-size line and the sampler list SHALL update on document
or selection changes instead, and a pointer move MUST NOT stat the document file.

#### Scenario: Moving the cursor does not restat the document [mdp_info_no_stat]

- **WHEN** a document opened from disk has its file removed and the pointer is then moved across the canvas
- **THEN** the document-size line keeps the size it showed on open and the panel does not read the file again
