## Why

Issue #555 tracks the disabled Image > Mode > Color Table command and the missing Forced and Transparency controls in Indexed Color conversion. Indexed images need a way to edit their palette directly and to keep a transparent index when saved.

## What Changes

- Enable Color Table on indexed documents whose retained index planes match the working image. Edit entries and interpolate ranges, mark or clear a transparent entry, and load or save ACT files.
- Add None, Black And White, Primaries, Web, and Custom forced colors to Indexed Color conversion. Preserve binary transparency when selected and use white matte otherwise.
- Reject forced-color sets that cannot coexist with a required clear entry, and make Exact availability account for all required entries.
- Carry the active color count and transparent index through native PSD/PSB save and reopen.

## Capabilities

### Modified Capabilities

- `ui/image-mode-menu`: Add indexed palette editing and conversion options.

## Impact

Affects the Qt mode menu and dialog, the engine's indexed conversion, PSD indexed normalization and write-back, and the command catalogue. The shared GIF options dialog keeps its existing controls. System palette presets and additional matte choices are separate parity work.
