# Proposal

## Why

Issue #1277 records that `File ▸ Scripts ▸ Load Files into Stack` is missing.
The catalogue already carries the row as a disabled stub
(`file.scripts.load.files.into.stack`), and the documentation names it as the
bulk way to get several images into one layered document before Auto-Align or
Auto-Blend (`docs/05-layers/align-and-distribute.md`,
`docs/05-layers/smart-objects.md`). CS6 presents a **Load Layers** dialog with
a source list, Browse/Remove/Add Open Files, and the two stack options.

## What Changes

- **Command wiring.** The existing disabled catalogue row becomes implemented
  and opens the Load Layers dialog. The generated `command_ids.h` gains its
  constant and the catalogue goldens/counts are updated.
- **Load Layers dialog.** `File ▸ Scripts ▸ Load Files into Stack…` opens one
  modal dialog: a Source Files group with a `Use` menu (`Files` / `Folder`), a
  file list, `Browse…`, `Remove`, `Add Open Files`, and the two CS6 options
  **Attempt to Automatically Align Source Images** and **Create Smart Object
  after Loading Layers**, both off by default. `OK` is refused with an empty
  list; `Cancel` changes nothing.
- **Stack construction.** On `OK` the listed files are decoded and a new
  untitled document is created on a canvas sized to the largest source, with
  one native-size raster layer per decodable file named after the file. The
  first listed file is the topmost layer. Undecodable files are skipped and
  reported; if none decode, no document is created.
- **Auto-Align option.** When checked, the loaded layers are aligned with the
  Auto projection before the document is shown. When unchecked, every layer
  keeps its source position.
- **Smart Object option.** When checked, the loaded (and aligned) layers are
  combined into one embedded smart object layer on the same canvas, with the
  stack preserved inside the embedded source. When unchecked, the layers stay
  separate.
- **No document required.** The command works with no document open; it always
  creates a new one.
- **Roadmap.** `ROADMAP.md` records the shipped command.

No **BREAKING** changes: this is a new surface and it leaves every open
document's pixels, path, modified state, and history untouched.

## Capabilities

### New Capabilities

- `interop/load-files-into-stack`: the Load Layers dialog, source selection
  (Files/Folder, Add Open Files), stack construction and sizing, layer naming
  and order, failure reporting, Auto-Align, and the embedded smart-object
  combination.

### Modified Capabilities

None.

## Impact

- `pictura-script`: the catalogue row flips to implemented, the command gains a
  frozen constant, and `tests/golden/catalogue.tsv` / `const_names.tsv` and the
  shape counts are regenerated.
- `pictura-app` (Rust): a `stack_files` bridge that decodes the sources,
  builds the document, applies Auto-Align, and optionally combines the stack.
- `pictura-render`: `stack_into_smart_object` builds one embedded source from
  the whole layer stack.
- `pictura-app` (C++): `LoadLayersDialog`, the command handler, the
  `document_api` wrapper, CMake source registration, and a Qt Test suite.
- `ROADMAP.md`: the shipped command is ticked.
- No new dependencies. The open/composite/auto-align/smart-object paths are
  reused as-is.
- Out of scope: Photoshop's `.jsx` script engine and the other Scripts rows
  (`Image Processor`, `Layer Comps to Files`, `Script Events Manager`,
  `Statistics`), which stay tracked separately.
