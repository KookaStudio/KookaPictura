# Design

## Reference and method

The authoritative reference is the CS6 contract:
`docs/02-ui-ux/keyboard-shortcuts.md` (`UI-011`) and
`docs/02-ui-ux/menus.md` (`UI-002`), both transcribed from the CS6 Help. The
PDF attached to #465 is a newer Photoshop sheet (Artboard, Frame, Object
Selection, Curvature Pen) and is used only where it agrees with CS6.

The audit compared, row by row:

- every implemented command in `command_catalogue()` that carries a shortcut,
  and every implemented command the contract binds;
- the tool letters in `tool_catalogue()` against the CS6 tool table;
- the window's `workspaceKeys()` and key handlers against the CS6 workspace
  and function-key tables.

## Results

### Already correct

- **Tool letters.** V, M, L, W, C, I, J, B, S, Y, E, G, O, P, T, A, U, K, N, H,
  R, Z all match CS6, including the shared-letter groups. Blur, Sharpen, and
  Smudge correctly have no letter.
- **Workspace keys.** `[`/`]` size, `Shift+[`/`Shift+]` hardness, `D`, `X`, `Q`,
  `F`/`Shift+F`, `Space+F`, `Tab`/`Shift+Tab`, `Ctrl+Tab`/`Shift+Ctrl+Tab`,
  Delete/Backspace, and `Shift+F1`/`Shift+F2` all match.
- **Menu bindings.** The remaining 68 shortcut-bearing implemented rows match
  the CS6 menu contract.

### Defects fixed

| Defect | CS6 owner | Change |
|---|---|---|
| `Ctrl+Y` on both Redo and Proof Colors | Proof Colors | Redo loses its default |
| `Ctrl+Alt+Shift+E` on both Export As and Stamp Visible | Stamp Visible | Export As loses its default |
| Color/Layers/Info panels unbound | F6/F7/F8 | Bind F6/F7/F8 |

The Redo command stays in the catalogue and menu, but CS6 has no separate Redo
chord: the Undo control already toggles to redo when `can_undo()` is false, and
Step Forward owns `Ctrl+Shift+Z`. Removing Redo's `Ctrl+Y` therefore loses no
CS6 behaviour. Export As is a CC-era extension with no CS6 binding, so it
becomes menu-only; Stamp Visible keeps the CS6 chord.

### Out of scope

These are feature gaps, not binding mismatches, and belong to other issues:

- Brush-preset cycling `,` `.` `<` `>` (`docs/02-ui-ux/keyboard-shortcuts.md`);
  related to #540.
- `F2`/`F3`/`F4` alternates for Cut/Copy/Paste (the catalogue stores one default
  chord per command; the multi-binding store can hold them later).
- `00`/`Shift+00` layer opacity (#523) and shortcut sets (#508).

## Test strategy

- **Rust catalogue tests** (`pictura-script`): collect the default shortcut of
  every row and assert no chord repeats; pin the CS6 function keys and the two
  chord owners. These are cheap and fail on any future data drift.
- **Golden table** (`tests/golden/catalogue.tsv`): the existing drift test pins
  the five edited rows; no generator changes.
- **Qt tests** (`pictura-app`): press F6/F7/F8 on an active window and assert
  the panel toggle action's checked state flips; a new Edit test proves `Ctrl+Z`
  redoes once no undo state remains. The quick-mask regression switches its
  redo press from `Ctrl+Y` to `Ctrl+Shift+Z`.
