## Purpose

Preserve an independent whole-rebuild reference for reduced display pixels while
excluding the retired stored-level API from ordinary application builds.

## ADDED Requirements

### Requirement: The whole-rebuild oracle is test-only

Renderer unit tests SHALL have access to the whole-rebuild oracle without
explicitly enabling a feature. Cross-crate application tests and profiles SHALL
have access through a non-default `test-oracle` feature enabled only for their
development dependency. Ordinary release and dev-app library builds SHALL NOT
compile or publicly export the whole-rebuild oracle, its borrowed planar source
adapter, or its private sampling and reduction helpers. Shared level-result
values and existing production constants SHALL remain available without the
feature.

#### Scenario: Renderer tests use the oracle without a feature [vo_renderer_tests]

- **WHEN** renderer unit tests run alone without the oracle feature
- **THEN** they can construct the independent reference and compare level counts, sizes and pixels

#### Scenario: Application tests retain their independent reference [vo_app_tests]

- **WHEN** application tests build through the development dependency
- **THEN** the whole-rebuild reference is available to existing crop, damage and solo-restore comparisons

#### Scenario: A normal release build excludes the oracle [vo_release]

- **WHEN** an ordinary application library is built in release with no oracle feature or test targets
- **THEN** the renderer artifact has no oracle feature, an external consumer cannot import the oracle or its planar source adapter, and the shared level result and constants remain usable

#### Scenario: A normal dev-app build excludes the oracle [vo_dev_app]

- **WHEN** an ordinary application library is built with the dev-app profile and no oracle feature or test targets
- **THEN** the renderer artifact has no oracle feature, an external consumer cannot import the oracle or its planar source adapter, and the shared level result and constants remain usable

### Requirement: The reference stays independent and minimal

The oracle SHALL independently rebuild, report level count and dimensions, and
crop exact premultiplied bytes from a supplied level-0 source. It SHALL NOT offer
coarse approximation, size-fit checks, incremental updates or stored-level
patching. Its level-count and reduction math MUST NOT call or share production
region or tile-pyramid math. Its retained row-based source reads SHALL support
contiguous and tiled planes without materializing a whole tiled source. Exact
crop, region, damage and solo-restore comparisons SHALL keep using this
independent reference.

#### Scenario: Numerical comparisons keep two implementations [vo_independent]

- **WHEN** regions, cached crops, repaired tiles and restored solo-mask tiles are compared against whole rebuilds
- **THEN** expected pixels come from the independent minimal reference and all comparisons remain byte-identical

#### Scenario: Tiled source reads preserve the retained behavior [vo_tiled_source]

- **WHEN** the oracle rebuilds and crops a tiled planar source and the same pixels supplied as contiguous slices
- **THEN** the bytes match and the tiled source is not materialized
