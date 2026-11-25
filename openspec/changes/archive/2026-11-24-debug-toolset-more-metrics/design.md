# Design

## Sources (all read only while the toolset is on)

| Row | Source |
|---|---|
| RAM anon / file | `/proc/self/status` `RssAnon`, `RssFile` |
| Open files | entries of `/proc/self/fd` less the directory handle |
| Disk read / write | `/proc/self/io` `read_bytes`, `write_bytes` delta |
| Page faults | `/proc/self/stat` minflt (field 10), majflt (field 12) delta |
| Context switches | sum over `/proc/self/task/*/status` voluntary / nonvoluntary delta |
| Jobs in flight | tasks named `pictura-open/save/filter/doc`; elapsed = uptime - starttime |
| Process GPU | DRM fdinfo of `/dev/dri/*` fds, deduped by `drm-client-id`: `drm-memory-vram` KiB, `drm-engine-*` ns delta over wall |
| System GPU | first `/sys/class/drm/card*/device` with `mem_info_vram_total`: `gpu_busy_percent`, `mem_info_vram_used` |
| Governor, scratch, tiles | `memory_status_json()` (existing bridge) |
| Edit / paint p50, p95 | `metrics_snapshot` `edits[].wall_ms`, `frames[].paint_ms` |

Rate rows need two samples and read `…` until then.

## Layout

Three new tables join the panel in order: Resource Usage (extended), Memory Budget,
Activity, then Platform and Version. The group chart draws one line per thread group
(GUI, rayon workers, jobs, other) in distinct colours with a coloured legend, newest
right, sharing the 60-sample window.

## Honesty

ponytail: engine busy % is the sum over all DRM engines of the process, so it can
exceed 100 on a GPU with several engines. NVIDIA proprietary exposes no fdinfo or
sysfs here; those rows read `n/a`, never a guess.
