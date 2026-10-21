# Test Plan

The scenario is exercised by the automated `--headless --self-test`;
`check-test-plan.py` cannot name a self-test check, so it is recorded as a
manual row with the self-test command as the evidence. The row is listed in the
PR body as the reviewer checklist.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `vh_float_screen_bounds` | Build the app, then run `./build/pictura --headless --self-test`. The check places the window at the screen origin itself, so neither the window's starting position nor its size is an input, and the float is moved past the screen's bottom-right corner. | The run prints `pictura self-test: PASS core float_screen_bounds` with `on_screen=1` and `on_screen_edge=1`, the window's previous position is restored, and the process exits 0. | manual: the check itself is the automated self-test, whose coverage cannot be referenced by `check-test-plan.py`; the same binary is what the Fedora rpm smoke-check runs. |
