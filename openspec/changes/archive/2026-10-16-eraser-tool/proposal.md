# Proposal: eraser-tool

## Why

The Eraser (issue #21) was catalogued but disabled. photorust's plain Eraser
is its Brush painting the background colour (`setEraseMode`); its
`core/src/erase.rs` holds the Background and Magic Erasers, which are separate
issues (#22, #23). The port follows `docs/03-tools/eraser-tools.md` (TOOL-023).

## What Changes

- `pictura_paint::eraser`: `EraserMode` (Brush / Pencil / Block) and
  `begin_erase`, which starts a Brush stroke that erases an ordinary layer to
  transparency (the Clear mode: coverage multiplies alpha down) and paints the
  background colour on the Background, an alpha-less layer, or a
  transparency-locked layer. Block is a hard 16 px square at full strength
  (`StrokeConfig::square`). With a history source it paints that state back
  (Erase To History).
- `cxxqt_object/paint_tools.rs`: `begin_eraser`, one `"Eraser"` state per stroke.
- `tool_eraser.cpp` handler: Erase to History, or Alt held at the press, paints
  the History Brush's source state back. The bar has the tip, Mode, Opacity,
  Flow (both greyed for Block), and Erase to History. Catalog row enabled; the
  Eraser joins the brush size ring and `[` / `]`; Alt keeps the paint cursor.
- C++ self-test `eraser_tool` (547). `shift_plain` (117) asserts `E` selects
  the Eraser and probes the still-unimplemented G group; `keys_shown` (116)
  probes the E group's disabled members; the unimplemented-tool guard (98) now
  probes the Background Eraser.

## Capabilities

### New Capabilities

- `tools/eraser-tool`: the Eraser, its modes, and Erase To History.

## Impact

- `pictura-paint` (`eraser.rs`, a square tip), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

The background-colour erase is ported from photorust's erase mode
(<https://github.com/perfecto25/photorust>); erasing to transparency, Block,
and Erase To History follow the spec. Behavioural parity only. Block is 16
document pixels (CS6: 16 screen pixels), and the Erase To History source layer
is matched by panel path — both `ponytail:` ceilings.
