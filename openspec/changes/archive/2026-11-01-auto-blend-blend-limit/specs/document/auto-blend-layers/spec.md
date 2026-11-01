## ADDED Requirements

### Requirement: Merged layer size limit

When the processed layers' content spans more than 100 megapixels, Auto-Blend SHALL
add its masks and no merged layer, and SHALL tell the user that only masks were
added. This takes precedence over the requirement that every run adds a merged
layer.

#### Scenario: A very large span adds masks only [abl_blend_limit]

- **WHEN** two layers span more than 100 megapixels
- **THEN** each gains a mask and no merged layer is added
