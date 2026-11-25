# meta/crate-test-dependency-boundaries Specification

## Purpose
Defines how the codec and renderer crates depend on each other in tests and where cross-crate behavior checks live, so test builds do not reverse the production dependency direction.

## Requirements

### Requirement: Codec tests exclude the renderer

The `pictura-codec` development dependency graph SHALL exclude `pictura-render` and SHALL NOT pull the renderer's optional GPU dependencies into codec test builds.

#### Scenario: Codec development graph stays renderer-free [ctdb_codec_graph]
- **WHEN** the codec development dependencies are resolved
- **THEN** the graph contains no `pictura-render` package and no `wgpu` package

### Requirement: Renderer behavior tests live with the renderer

Assertions about compositor output and renderer-owned document mutations SHALL live in `pictura-render` tests. Codec tests SHALL use codec or core fixtures for codec-only setup.

#### Scenario: Output assertions move to render [ctdb_output_ownership]
- **WHEN** a test asserts `composite_rgba` or `composite_tiled` behavior
- **THEN** the test is owned by `pictura-render`

#### Scenario: Mutation assertions move or use local fixtures [ctdb_mutation_ownership]
- **WHEN** a codec round-trip test needs a layer, shape, mask, flatten, rasterize, or depth conversion
- **THEN** it either lives in `pictura-render` or builds the fixture through codec and core APIs

### Requirement: Document operations stay in render

The refactor SHALL NOT move document mutation helpers into `pictura-core`. Render SHALL remain the owner of document and layer operations.

#### Scenario: Core gains no document mutation helpers [ctdb_core_ownership]
- **WHEN** the dependency edge is removed
- **THEN** no render document-operation helper is added to `pictura-core`

### Requirement: Test coverage is preserved

The refactor SHALL preserve the existing test count, assertions, and ignore reasons. Tests may move between crates but SHALL NOT be deleted or weakened.

#### Scenario: Moved tests keep their checks [ctdb_coverage_preserved]
- **WHEN** render-using codec tests are relocated or rebuilt with local fixtures
- **THEN** each original test remains present with its assertions and ignore reason unchanged
