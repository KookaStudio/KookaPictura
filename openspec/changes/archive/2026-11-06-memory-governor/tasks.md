# Tasks

## Tests first

- [x] `pictura-core::memory` tests: cgroup limit and `max`, nested cgroup, headroom, unreadable host, room, PSI and fallback pressure, reservation drop, injected roots, cached reading, `try_plane`
- [x] `pictura-app` tests: every new gate refuses without room and leaves the document and history unchanged; the 512 MiB cgroup fill; the failpoint edits; history's share of the ceiling; Image Size's room bound
- [x] Heap guard: each estimate covers its measured peak at 4250²
- [x] Qt Test: `tst_fill_tools::refusedForMemoryShowsStatus`; `tst_memory_governor` (preference default and change, status-bar messages, control `refused` answers, `status.memory`)

## Implementation

- [x] `pictura-core::memory::{system, governor}`; move the host memory readers from `pictura-codec` (re-exported there)
- [x] `try_plane` with the failpoint; New Document's planes and the bucket's match plane use it
- [x] `memory_gate` reads the governor and the refusal record; history's cap is half the ceiling; Image Size's bound is the room
- [x] Gates: New Document, Add Layer, brush begin, Undo/Redo/jump, snapshot add and restore, Save (held reservation), Open (bounded decode budget)
- [x] Bridge: Memory Usage get/set, the status JSON; control server `status.memory` and `refused` answers
- [x] Preferences: Memory Usage spin box, session field, applied at start and on change; status-bar messages
- [x] `docs/02-ui-ux/preferences.md`: Memory Usage default 60 % (TASK-ALLOWS-DOCS, separate commit)

## Verification

- [x] `openspec validate --all --strict`
- [x] `scripts/profile-tools.py --scenarios memory --memory-max 4G` at 17000²: no `oom`
- [x] `bash scripts/verify-fast.sh`
