# Tasks

- [x] Add the failing tests: `pictura-paint::sample_all_layers_picks_up_from_the_composite`, `pictura-paint::preset_strokes_hold_their_documented_ordering`, `tst_options_bars_parity::mixerBarMatchesCc`
- [x] Add `Stroke::begin_mixer` and the composite pickup in `MixerBrush::dab`
- [x] Wire `sample_all` through `begin_mixer_brush`, `MixerOptions` and its JSON
- [x] Remove Mode and Opacity from the Mixer bar; enable Sample All Layers
- [x] Build, run the affected suites, `scripts/verify-fast.sh`
- [x] Update `ROADMAP.md` and `docs/dev/doc-code-parity-gaps.md`; archive the change
