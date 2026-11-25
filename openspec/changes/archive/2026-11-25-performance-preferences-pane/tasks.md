# Tasks

## 1. Tests

- [x] 1.1 `pictura-app::history::tests::depth` (three tests).
- [x] 1.2 `pictura-render::tile_pyramid::tests::cache_levels_cap_the_pyramid`.
- [x] 1.3 `tst_performance_preferences` (all rows of `test-plan.md` it names).

## 2. Engine

- [x] 2.1 `history::depth`: the state limit, at-once trim, test override.
- [x] 2.2 `tile_pyramid`: the Cache Levels cap, `with_levels`, `fits` compares levels.
- [x] 2.3 Bridge: `history_states`, `set_history_states`, `cache_levels`,
      `set_cache_levels`, `apply_history_states(view)`.
- [x] 2.4 Schema: `historyStates`, `cacheLevels`.

## 3. UI

- [x] 3.1 Performance page (own file): Memory slider and ceiling, History & Cache,
      presets, tile size, scratch list, graphics processor.
- [x] 3.2 Frame wiring: apply at start, on change, save.
- [x] 3.3 Status bar: Scratch Sizes and Efficiency (`status_readouts.*`).

## 4. Trackers

- [x] 4.1 `ROADMAP.md`, `PARITY.md`, parity-gap register WF-12 / UI-10, `docs/dev/STATE.md`.
