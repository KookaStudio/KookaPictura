## MODIFIED Requirements

### Requirement: The memory scenario catalogue

The profiler SHALL provide scenarios M1–M14 as listed in the designs of this
capability's changes (M14, an arc warp of a full-layer gradient, in
`warp-commit-out-of-core`). They default to 17000×17000 RGB 8-bit with a white
Background, and `--size` scales every geometry with the document. Each SHALL
run in a fresh app process, with its prerequisites run before the peak reset and
not measured.

#### Scenario: Every scenario runs at a small size [mb_catalogue]

- **WHEN** `profile-tools.py --self-check` runs with a built app
- **THEN** every scenario M1–M14 runs at 512×512 with `outcome: ok` and a
  valid row
