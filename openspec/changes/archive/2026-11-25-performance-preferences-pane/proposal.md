# Proposal

## Why

The Performance page of Preferences holds only Memory Usage and Use GPU
Compute (#636). The engine already has a scratch store with Efficiency
accounting (#1217), a saved scratch-volume list with no UI, and a view
pyramid. The CS6 controls that sit on top of them are missing: History
States is a constant (20), Cache Levels and the History & Cache presets do
not exist, scratch volumes cannot be chosen, the pane does not name the
graphics processor, and the status bar cannot show Scratch Sizes or
Efficiency (`docs/10-workflow-io/scratch-disks-and-memory.md`, WF-12).

## What Changes

- **History States** (1–1000, default 20) replaces the history depth
  constant. A change applies at once: open documents drop their oldest
  states, then redo states if the oldest state is current. Named snapshots
  are kept.
- **Cache Levels** (2–8, default 8) caps the number of levels in each
  view's tile pyramid. The change applies at once, and each view rebuilds
  its pyramid on its next paint. The default keeps today's depth. CS6
  sources give 4 or 6, and that default would slow zoomed-out drawing of
  large images (owner decision).
- **History & Cache presets**: Tall and Thin sets Cache Levels to 4, while
  Default and Big and Flat set it to 8.
- **Cache Tile Size** shows the engine's fixed 256 px tile in a disabled
  menu. The tile size is not configurable.
- **Scratch Disks**: an ordered list of up to four directories, each shown
  with its free space and a tmpfs warning. It has Add, Remove, Move Up and
  Move Down. A change applies at once and is saved.
- **Graphics processor**: the pane names the detected adapter next to Use
  GPU Compute, or says none was found. There is no Basic / Normal /
  Advanced dialog (owner decision).
- **Memory Usage** gains a slider that tracks the spin box, plus a "Let
  Pictura Use" readout of the governor's ceiling.
- **Status bar**: the readout menu gains Scratch Sizes (the memory Pictura
  uses over the memory it may use) and Efficiency (compute time over compute
  plus scratch I/O time since the previous reading).

Help > System Info and Edit > Purge already ship and are unchanged.

## Capabilities

### New Capabilities

- `ui/performance-preferences`: History States, Cache Levels and presets,
  Cache Tile Size, the scratch disk list, and the graphics-processor line.
- `ui/status-scratch-efficiency`: the Scratch Sizes and Efficiency status
  bar readouts.

### Modified Capabilities

- `document/memory-governor`: Memory Usage gains a slider and a ceiling
  readout.

## Impact

`pictura-app` (history depth, the bridge, the Preferences page, the status
bar, the preference schema), `pictura-render` (the pyramid level cap).
