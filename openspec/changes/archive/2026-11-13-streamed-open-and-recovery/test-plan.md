# Test plan

This plan covers `streamed-open-and-recovery` (#1299, #1273):

- the modified `document/crash-recovery` requirement "Recover after an unclean
  exit";
- the new capabilities `codec/streamed-layer-decode` and
  `document/out-of-core-open`.

**Fixtures.**

- **Codec tests.** They build channel payloads with the writer's encoders
  (raw, PackBits, zlib with and without prediction) at 600 × 300 and 257 × 513.
  So a plane spans several strips and a partial tile column, and its rows are
  seeded noise. The dense `decode_channel_data` and `decode_depth_channel` are
  the oracle.
- **Recovery memory test.** It pins the host's available memory with
  `set_available_memory_for_test`, which leaves only a few MiB of room, and
  pins no scratch space with `set_scratch_available_for_test(0)`. That way only
  the 2048² snapshot is refused, on any host.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A crashed document comes back | Leave a 64×48 Untitled-1 snapshot with a Work Path in a lock-less session; recover in a new window | `Untitled-1-Recovered`, 64×48, dirty, no path, Work Path; orphan deleted; new session holds a snapshot; no `recoveryFailures` dialog | `tst_recovery::recoverReopensAnOrphanedDocument` |
| A partial recovery quarantines the failed document | One readable and one corrupted snapshot in an orphan; Recover | Readable opens; corrupt snapshot and manifest under `quarantine/<session>/`; orphan deleted; rescan empty; dialog names the document and the quarantine dir | `tst_recovery::partialRecoveryQuarantinesFailedDocument` |
| `cr_memory_refused` | Orphan with a 64×48 and a 2048² snapshot; pin room small, no scratch; Recover; unpin; read the next offer; Recover | Small opens, its files gone; large snapshot and manifest stay; session kept; nothing quarantined; dialog says memory and gives the path; next offer lists only the large one; it then opens | `tst_recovery::memoryRefusedRecoveryIsOfferedAgain` |
| `sd_identical` | Stream raw/RLE/ZIP/ZIP-prediction channels at depth 8, 16, 32 and two geometries | Every 8-bit and native plane equals the dense decode | `pictura-codec::streamed_channels_match_the_dense_decode` |
| `sd_errors` | Stream a short raw channel, an overrunning RLE row, and short, long, raw-deflate and garbage ZIP payloads | Same `PsdError` variant and message as dense; raw deflate decodes to the same bytes | `pictura-codec::streamed_channels_report_the_dense_errors` |
| `sd_tiled` | Read an 8-bit PSD whose layer has a solid 1024² channel and a user mask | Channels and mask tiled; solid channel allocates < ¼ dense | `pictura-codec::decoded_layer_channels_are_tiled` |
| `sd_tile_budget` | Read a layered PSD with `max_document_bytes` below its layer planes and `max_tiled_bytes` above; then with `max_tiled_bytes` below | First opens equal to the default read; second fails with "exceeds the decode budget" | `pictura-codec::tiled_channels_charge_the_tile_budget` |
| `sd_mapped` | Map a written PSD, an empty file, a missing path | Bytes equal; decodes like `read_psd`; empty slice; error | `pictura-codec::a_mapped_file_reads_as_its_bytes` |
| `oo_budget` | `open_budget` with 3 GiB room + 10 GiB scratch, 3 GiB + none, no reading | Contiguous 2 GiB, tiles 12 GiB / 2 GiB; default when unread | `pictura-app::open_budget_adds_scratch_to_the_tile_allowance` |
| `oo_admits` | Open a PSD whose tiles exceed ⅔ of the pinned room, with ample scratch, then with none | First opens; second refused, `open_refused_for_memory` | `pictura-app::an_open_larger_than_the_room_is_admitted_with_scratch` |
| `oo_m12` | `python3 scripts/profile-tools.py --scenarios M12 --size 12000 --peak`, then `--size 17000 --memory-max 2G` | 12000² peak ≤ layer tiles + 256 MiB; 17000² `outcome: ok` | manual: needs generated multi-GB fixtures, a systemd cgroup memory limit and minutes of a real app; run for this change and recorded in `docs/dev/STATE.md` |
