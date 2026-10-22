# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `sysinfo_menu` | Construct the main window with no document and dispatch `help.system.info` | The command is enabled, a `SystemInfoDialog` is visible, and its report is non-empty | `tst_system_info::helpMenuWiresSystemInfo` |
| `sysinfo_submit_menu` | Inspect the Help menu after startup | `Submit Issue` is enabled, follows `System Info`, and has a handler | `tst_system_info::helpMenuWiresSubmitIssue` |
| `sysinfo_sections` | Format a fixture with every field populated | Every section heading and fixture value appears | `tst_system_info::reportHasAllSections` |
| `sysinfo_gpu` | Ask the renderer for its adapter summary, then format a fixture with adapter fields | The summary is `None` or has a non-empty name; the report shows name, type, backend, and driver | `pictura-render::gpu_adapter_info_never_panics`, `tst_system_info::reportHasGpuFields` |
| `sysinfo_unavailable` | Format a fixture with empty optional fields | Each missing row reads `Unavailable`; no section disappears | `tst_system_info::reportUsesUnavailable` |
| `sysinfo_proc_cpu` | Parse representative `/proc/cpuinfo` text | The model, two physical cores, and the clock are reported | `tst_system_info::parsesCpuInfo` |
| `sysinfo_proc_memory` | Parse representative `/proc/meminfo` text | Total/available RAM and swap convert to bytes | `tst_system_info::parsesMemInfo` |
| `sysinfo_bytes` | Format byte counts in the KB, MB, and GB ranges | Each result has one decimal place and the correct unit | `tst_system_info::formatsBytes` |
| `sysinfo_snapshot` | Set a different fixture on an open dialog | The text and `reportForTest()` change to the new report | `tst_system_info::reopenRefreshesReport` |
| `sysinfo_copy` | Click the dialog's `Copy` button | The clipboard text equals the dialog report | `tst_system_info::dialogCopiesReport` |
| `sysinfo_submit` | Build the issue URL for a report | The URL is `issues/new` on the repository and the decoded body contains the report in a fenced block | `tst_system_info::submitUrlEncodesReport` |
| `sysinfo_submit_cap` | Build the issue URL for a report over 6000 characters | The decoded body is at most 6000 characters and ends with the truncation marker | `tst_system_info::submitUrlCapsLongReport` |
| `sysinfo_privacy` | Collect a live report | The hostname and `USER`/`USERNAME` values do not appear | `tst_system_info::collectorOmitsHostAndUser` |
| Browser launch | Choose `Help > Submit Issue` on a desktop session | The default browser opens the repository's new-issue page with the report prefilled | manual: offscreen Qt Test cannot observe the desktop URL handler |
| Live machine values | Open `Help > System Info` on a real desktop | The CPU, GPU, memory, and display values match the machine | manual: the automated collector test checks privacy and fallbacks, not a particular machine's readings |
