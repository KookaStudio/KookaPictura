feat: implement Quick Mask mode (#172)

Quick Mask converts the starting selection into a temporary editable mask. Q and the toolbox control enter/leave the mode; black protects and white selects. It includes display options, a temporary Channels row, red active-layer highlighting, and a blocked painting cursor when no layer is selected.

Selections created inside Quick Mask constrain edits without replacing mask coverage. Undo/redo restores both states and repaints the canvas. Brush updates preserve existing coverage and use bounded presentation, including at reduced zoom. Unsupported operations, including Vanishing Point, refuse safely.

Both OpenSpec changes are synced and archived. No dependencies were added. The command catalogue and frozen-ID golden tables each intentionally gain one Quick Mask entry; existing rows are preserved.

Validation: an earlier full run passed 8,142 checks with 124 skipped and zero failures. The final Vanishing Point guard and regression extension have formatting/static checks only; they have not been built or executed. No additional local build or verification was run for publication, per request; CI must validate the final revision. Local CPU brush reproduction improved from 105.0 ms to 1.68 ms median at 2048×2048.

Reviewer checklist:
- [ ] Where a lawful CS6 runtime is available, compare entry/exit, history steps, native-depth temporary-channel appearance, and filter availability. Record observations separately; project-contract tests do not establish independent CS6 parity.

Closes #172.
