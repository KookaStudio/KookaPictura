## MODIFIED Requirements

### Requirement: The memory scenario catalogue

The profiler SHALL provide scenarios M1–M23 as listed in the designs of this
capability's changes (M14, an arc warp of a full-layer gradient, in
`warp-commit-out-of-core`; M15, Image Size of a full-layer gradient to 150 %, in
`image-size-out-of-core`; M16, Canvas Size to 150 %, M17, Rotate 90° CW, and
M18, Rotate 15°, each of a full-layer gradient, in
`canvas-geometry-out-of-core`; M19–M21, Free Transform of a full-layer gradient
to 50 %, 100 % with 15°, and 150 %, and M22–M23, Perspective and Distort of it
through a corner drag, in `transform-acceptance-scenarios`). They default to
17000×17000 RGB 8-bit with a white Background, and `--size` scales every
geometry with the document. Each SHALL run in a fresh app process, with its
prerequisites run before the peak reset and not measured.

#### Scenario: Every scenario runs at a small size [mb_catalogue]

- **WHEN** `profile-tools.py --self-check` runs with a built app
- **THEN** every scenario M1–M23 runs at 512×512 with `outcome: ok` and a
  valid row

## ADDED Requirements

### Requirement: Pixel digests for out-of-core equivalence

With `--digest`, each memory row SHALL carry every layer's `layer_digest`,
read after the peak is recorded. `--compare` SHALL report, per scenario,
whether the two runs' digests are the same, so a run under a hard limit can be
shown to match the same run in RAM pixel for pixel.

#### Scenario: Digests are recorded and compared [mb_digest]

- **WHEN** two saved runs carry digests that agree in one scenario and differ
  in another, and a self-check scenario runs with `--digest`
- **THEN** the compare table says `same` for the first and `differs` for the
  second, and the self-check row carries a digest for every layer

#### Scenario: A 17000² transform under a hard limit matches RAM [mb_transform_cgroup]

- **WHEN** M19–M23 run at 17000² with `--digest`, once without a limit and
  once each under `--memory-max 2G` and `4G` with scratch on disk
- **THEN** every capped row is `ok`, lands one history state, and its digests
  are the same as the uncapped run's
