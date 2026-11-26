## Why

The public user guide in `public_docs/` explains tasks, tools, layers and
filters, but has no reference for the menu bar, the Preferences pages or the
status-bar readouts. A user who meets a menu command, a preference or a canvas
overlay has nowhere to look it up. Testing #636 exposed two concrete cases: the
blue dashed **View ▸ Show ▸ Layer Edges** box looked like a bug, and the new
Performance preferences and Scratch Sizes / Efficiency readouts were
undocumented.

## What Changes

- Add one reference page per top-level menu: File, Edit, Image, Layer, Type,
  Select, Filter, View, Window and Help.
- Add a Preferences reference page and a Status bar reference page.
- Add a **Menus** section to `public_docs/README.md` linking every new page.
- Capture real Kooka Pictura screenshots for each View ▸ Show extra and add
  them to `public_docs/images/`.
- Add `scripts/check-public-docs-commands.py`, an allowlist and a guards
  workflow step that fail when a command id is neither documented nor
  allowlisted as internal.

## Capabilities

### Modified Capabilities

- `meta/documentation-corpus`: the public user guide becomes a complete menu,
  Preferences and status-bar reference, and a guard keeps it current.

## Impact

`public_docs/`, `public_docs/images/`, `scripts/check-public-docs-commands.py`,
`scripts/public-docs-command-allowlist.txt`, `.github/workflows/guards.yml`,
`scripts/verify-fast.sh`, and the OpenSpec corpus.
