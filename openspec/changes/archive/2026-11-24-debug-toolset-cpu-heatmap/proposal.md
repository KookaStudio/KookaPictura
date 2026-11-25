# Proposal

## Why

The Platform table lists per-CPU usage as numbers. With 16 or more CPUs that is a
wall of digits where an overloaded core is hard to spot. A heat matrix shows the
same data at a glance.

## What Changes

- The `CPUs` row shows one square per logical CPU, coloured cold (blue) to hot
  (red) by usage, with no numbers; hovering a square names the CPU and its usage.
- One row of squares for 8 or fewer CPUs, two rows for more.
- Until the second sample the row reads `…`.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

<!-- None: only adds a requirement. -->

## Impact

- `debug_toolset_panel.{h,cpp}`: a `CpuHeatmap` widget replaces the `cpus` text row.
- `tst_debug_toolset.cpp` extended. No bridge, engine or dependency change.
