# Tasks

## 1. Tests named by the plan

- [x] 1.1 `pictura-codec`: `streamed_channels_match_the_dense_decode`, `streamed_channels_report_the_dense_errors`, `decoded_layer_channels_are_tiled`, `tiled_channels_charge_the_tile_budget`, `a_mapped_file_reads_as_its_bytes`.
- [x] 1.2 `pictura-app`: `open_budget_adds_scratch_to_the_tile_allowance`, `an_open_larger_than_the_room_is_admitted_with_scratch`.
- [x] 1.3 Qt: `tst_recovery::memoryRefusedRecoveryIsOfferedAgain`; dialog assertions in `recoverReopensAnOrphanedDocument` and `partialRecoveryQuarantinesFailedDocument`.

## 2. Codec

- [x] 2.1 `read/stream.rs`: row sources (raw, RLE, ZIP with deflate fallback, prediction per row) and `TiledSink`.
- [x] 2.2 `read/decode.rs` + `read.rs`: Phase B returns planes; Phase A charges tiles and strips; Phase C takes planes.
- [x] 2.3 `ImageBudget::max_tiled_bytes`, `DecodeBudget::charge_tiled`; literal constructors updated.
- [x] 2.4 `mapped.rs`: `MappedFile`.

## 3. App

- [x] 3.1 `open_budget` / `bounded_budget` with scratch; `decode_psd_with` maps the file.
- [x] 3.2 `recoverOrphans`: refused / unreadable / recovered; `retained` sessions; `recoveryFailures` dialog.

## 4. Measurement

- [x] 4.1 Lower the `open_psd` heap guard baseline to the new 4250² peak.
- [x] 4.2 M12 at 12000² and 17000² under `MemoryMax=2G`; record in `docs/dev/STATE.md`.

## 5. Close-out

- [x] 5.1 `docs/dev/STATE.md`, `ROADMAP.md` if it lists the item.
- [x] 5.2 `openspec validate --all --strict`, `check-test-plan.py`, one build, `verify-fast`.
- [x] 5.3 Archive the change.
