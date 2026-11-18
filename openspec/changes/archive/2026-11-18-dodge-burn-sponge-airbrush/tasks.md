# Tasks: dodge-burn-sponge-airbrush

## 1. Tests first

- [x] 1.1 Engine tests in `pictura_paint::tone`: Airbrush held build-up, Grayscale Sponge contrast, Grayscale Dodge/Burn, 32-bpc refusal.
- [x] 1.2 Qt Test cases in `tst_retouch_tools`: Airbrush off/on, `Shift+Alt+P`, Range/Mode quick keys, ignored key, Grayscale contrast, 32-bpc disable/fallback.
- [x] 1.3 Update the tool-options persistence rows for the new `airbrush` field.

## 2. Engine

- [x] 2.1 `ToneOptions` gains `airbrush` and `grayscale`; `Stroke::begin_tone` sets `grayscale` from the document mode.
- [x] 2.2 `toned` applies every Airbrush dab and keeps the once-per-stroke cap otherwise.
- [x] 2.3 Dodge/Burn and Sponge use channel 0 in a Grayscale document; Sponge moves grey levels around 0.5.

## 3. Bridge and options

- [x] 3.1 `begin_tone` carries the Airbrush flag through `paint_tools`.
- [x] 3.2 `ToneOptions` (C++), JSON fields, and the persistence rows include `airbrush`.
- [x] 3.3 `tool_retouch.cpp` holds dabs for Dodge/Burn Airbrush and stops them on release, switch, and cancel.
- [x] 3.4 `options_bar_retouch.cpp` adds the Dodge/Burn Airbrush toggle and follows `toneOptionsChanged`.

## 4. Quick keys and availability

- [x] 4.1 `Shift+Alt+P` toggles Dodge/Burn Airbrush; `applyToneQuickKey` sets Range/Mode.
- [x] 4.2 Register the four window shortcuts and reserve them in `workspaceKeys()`.
- [x] 4.3 `toolAllowedForDocument` refuses 32-bpc tone tools; `setActiveTool` and `bindCanvas` enforce it.
- [x] 4.4 `Toolbox::available`/`refreshToolAvailability` disable the tone rows and `refresh()` calls it.

## 5. Docs and verification

- [x] 5.1 Parity register TOOL-24 -> SHIPPED; tick the ROADMAP item; update `docs/dev/STATE.md`.
- [x] 5.2 `openspec validate --all --strict`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, targeted `cargo test`/`ctest`; the full `verify-fast` gate runs in CI.
