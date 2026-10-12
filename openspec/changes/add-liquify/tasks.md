# Tasks: add-liquify

## 1. Engine

- [x] 1.1 Port photocraft's displacement field as `LiquifyMesh` with the dab model, Forward Warp, Reconstruct, Twirl, Pucker, Bloat, Push Left, Freeze and Thaw (`pictura-filters/src/distort/liquify.rs`).
- [x] 1.2 Add Mirror and Turbulence, Reconstruct amount, Restore All, Mask All / None / Invert All, and `scaled_to`.
- [x] 1.3 Add the commit resampler `liquify` and the preview `LiquifyProxy` (`liquify/resample.rs`); `Filter::Liquify` dispatch.
- [x] 1.4 List Liquify in `filter_preserves_opacity`.

## 2. App

- [x] 2.1 `filter_tools/liquify.rs`: the dialog session, the kept-mesh store, and the bridge.
- [x] 2.2 Map the one-slot `liquify` kind; positional preview; arity.
- [x] 2.3 `LiquifyDialog` and `LiquifyCanvas`; enable `Filter ▸ Liquify…`.

## 3. Verification

- [x] 3.1 Engine unit tests: identity, translation, each tool's direction, Alt reversals, freeze, reconstruct/restore, scale-to-fit, far-off strokes, proxy.
- [x] 3.2 Store test and `filter_map` mapping test.
- [x] 3.3 `tst_liquify_dialog`: modes, shortcuts, preview, restore/reconstruct, freeze, cancel, OK, Load Last Mesh, Last Filter Settings.
- [x] 3.4 Update `docs/06-filters/liquify.md` and `docs/dev/STATE.md` (`TASK-ALLOWS-DOCS`).
- [x] 3.5 Run `bash scripts/verify-full.sh` and `openspec validate --all --strict`.
