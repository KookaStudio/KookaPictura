# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mcp_initialize` | `initialize` with a known, then unknown version | Echoed; newest | `pictura-mcp::initialize_negotiates_the_version` |
| `mcp_notification` | `notifications/initialized` | No reply | `pictura-mcp::notifications_get_no_reply` |
| `mcp_bad_input` | Bad JSON; unknown method | -32700; -32601 | `pictura-mcp::bad_input_is_reported` |
| `mcp_oversized_request` | Line over 1 MiB; `ping` | -32600; answered | `pictura-mcp::an_oversized_request_is_refused_and_the_bridge_continues` |
| `mcp_tools_list` | `tools/list` | Prefix, schema, exclusions | `pictura-mcp::tools_are_listed_safely` |
| `mcp_call_forward` | `pictura_filter` on a fake backend | Forwarded; text result | `pictura-mcp::a_call_forwards_its_arguments` |
| `mcp_call_error` | Backend answers `refused` | `isError`; code named | `pictura-mcp::a_control_error_is_a_tool_error` |
| `mcp_unknown_tool` | Unknown name | -32602 | `pictura-mcp::an_unknown_tool_is_a_protocol_error` |
| `mcp_image` | Result with `base64` | Image item | `pictura-mcp::images_are_image_content` |
| `mcp_not_running` | Missing socket | `isError` naming Preferences | `pictura-mcp::nothing_listening_names_preferences` |
| `mcp_token_mode` | Token mode 0644 on a fake server | `isError`; nothing sent | `pictura-mcp::an_exposed_token_is_refused` |
| `mcp_too_large` | Oversized line, then normal | `too_large`; ok | `pictura-mcp::a_huge_reply_does_not_break_the_stream` |
| `mcp_stale_id` | Old id first | Matching reply returned | `pictura-mcp::a_late_reply_is_skipped` |
| `mcp_spawn_args` | Parse options | Child args present | `pictura-mcp::spawn_arguments_come_from_the_options` |
| `mcp_spawn_default` | `--spawn` alone | `--control-tier read` | `pictura-mcp::the_default_tier_is_read` |
| `mcp_bad_args` | Unknown option; bad tier | Parse error | `pictura-mcp::bad_arguments_are_refused` |
| `mcp_last_client` | Two clients; each leaves | Signal after the last | `tst_control_serial::lastClientLeavingIsReported` |
| `mcp_end_to_end` | Drive attach and spawn | Tools answer; child exits | manual: a process-level run; `scripts/verify-control.sh` runs it in CI |
