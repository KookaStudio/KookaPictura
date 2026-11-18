# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `bfn_extension_last` | Validate a Folder destination with `Document Name`, `extension`, `3 Digit Serial Number`; then with the extension moved last | Extension-last diagnostic; then accepted | `pictura-script::validate_rejects_an_extension_before_other_fields` |
