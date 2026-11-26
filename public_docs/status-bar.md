# The status bar

The status bar runs along the bottom of the window. It shows the zoom, a
document readout, the graphics backend and the active tool's hints. The small
arrow beside the readout opens the readout menu.

## The readout menu

Click the arrow beside the readout to choose what it shows:

| Readout | What it shows |
|---|---|
| **Document Sizes** | The document's pixel size and resolution, for example `1650 px x 940 px (72 ppi)`. |
| **Document Dimensions** | The same size with labels: `W 1650  H 940`. |
| **Measurement Scale** | The current Measure-tool scale. It is only in the Extended edition. |
| **Scratch Sizes** | How much the memory governor is using and its ceiling, for example `312.5M/2.0G`. The first number is used; the second is the limit. |
| **Efficiency** | The share of the latest two-second interval spent computing rather than reading or writing the scratch disk, as a percentage. Higher is better; a low value means the scratch disk is the bottleneck. |

**Scratch Sizes** and **Efficiency** refresh every two seconds while they are
shown. The others update when the document or the active tool changes. See
[Performance settings](performance.md) for the memory limit they report.

## The other readouts

- The **zoom** field shows the current zoom and lets you type a new one.
- The **backend** readout shows whether the canvas is drawing with the GPU or
  the CPU.
- The **action** readout appears while a long operation runs.
- The **tool hints** show the active tool's shortcuts and modifier keys.

The status bar also shows a temporary message when a command finishes or is
refused. The message replaces the readouts until it clears.
