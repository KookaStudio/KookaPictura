# Proposal

## Why

The toolset shows process and machine totals but not what is spending Pictura's
memory budget, why it is slow, or what the GPU is doing for Pictura. Owner-approved
suggestions fill that in.

## What Changes

New rows and groups, all Linux-only and all read only while the toolset is on:

- **Pictura Resource Usage:** RAM split anonymous/file-backed; GPU engine busy % and
  GPU memory for this process (DRM fdinfo); a thread-group CPU line chart.
- **Pictura Memory Budget (new group):** the memory governor's used/ceiling and
  pressure, scratch-disk use, and the packed/spilled/backed tile counts, from the
  existing `memory_status_json` bridge call.
- **Pictura Activity (new group):** open files, disk read/write rate, page faults,
  context switches, background jobs in flight with elapsed time, and p50/p95 edit
  and paint times.
- **Platform:** a system GPU row (busy %, VRAM used of total) from sysfs.

Edit and paint timings need `PICTURA_METRICS` (read once at startup) and say
`not measured` without it. GPU rows say `n/a` where the driver exposes no source
(NVIDIA proprietary). Open-documents count is not included: it needs a frame hook.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

<!-- None: requirements are added. -->

## Impact

- `debug_toolset.{h,cpp}`: more sources, parsers and snapshot fields.
- `debug_toolset_panel.{h,cpp}`: rows, two groups, one multi-line chart.
- `frame_system_info.cpp`: supplies the memory-status provider.
- `tst_debug_toolset.cpp` extended. No Rust, engine or dependency change.
