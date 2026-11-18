# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ce_export_webp` | Call `export_webp` with a path and no options. | The file is RIFF/WEBP with a `VP8 ` chunk and no `VP8L`. | `tst_control_editing::exportWebpModes` |
| `ce_export_webp_lossless` | Call `export_webp` with `lossless: true`. | The file is RIFF/WEBP with a `VP8L` chunk and no `VP8 `. | `tst_control_editing::exportWebpModes` |
| `ce_export_webp_state` | Capture the path, dirty state, and history count, export WebP, and compare. | All three are unchanged. | `tst_control_editing::exportWebpModes` |
| `ce_export_webp_gate` | Run a restricted session without the Save grant and call `export_webp`. | The request is `refused` and the destination does not exist. | `tst_control_editing::exportIsGatedLikeASave` |
| Live control recipe | Run `scripts/verify-control.sh` against `./build/pictura --headless --control`. | The socket exports a lossy VP8 file and a lossless VP8L file. | manual: the shell recipe is exercised locally; its assertions are not a `tst_*` or Python test reference |
