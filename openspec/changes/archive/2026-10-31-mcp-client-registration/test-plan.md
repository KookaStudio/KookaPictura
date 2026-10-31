# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `reg_new_file` | Register with no file | Entry written | `tst_client_registration::newFileIsCreated` |
| `reg_merge` | Config with other keys | Survive; backup holds original | `tst_client_registration::otherServersSurvive` |
| `reg_invalid` | JSONC; array `mcpServers` | Problem; bytes unchanged | `tst_client_registration::invalidFilesAreRefused` |
| `reg_decline` | Decline in the box | No file | `tst_client_registration::decliningChangesNothing` |
| `reg_replace` | Entry with another command | Question names it | `tst_client_registration::differentEntryNeedsItsOwnAcceptance` |
| `reg_unregister` | Unregister | Only the entry gone | `tst_client_registration::unregisteringRemovesOnlyTheEntry` |
| `reg_symlink` | Config is a link | Target changed; link kept | `tst_client_registration::symbolicLinkIsKept` |
| `reg_cli` | Fake `claude` script | Arguments logged | `tst_client_registration::cliIsUsed` |
| `reg_cli_missing` | No CLI | Row says so; disabled | `tst_client_registration::missingCliIsReported` |
