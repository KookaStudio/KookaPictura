# Tasks

## 1. Tests before implementation

- [x] 1.1 Add the multi-selection Qt tests named in the plan for toggling, marquee, overlay, replacement, and invalidation across components and targets.
- [x] 1.2 Add the path-transform Qt tests named in the plan for every mode, controls, target, history, cancellation, session ownership, and repeat scenario.

## 2. Selection and transforms

- [x] 2.1 Implement component and anchor multi-selection across the active path, including Shift-click, marquee, overlay and invalidation.
- [x] 2.2 Integrate path transform and backup into the single live-session slot; support Work, saved and editable layer vector targets and restore full target metadata on cancel or invalidation.
- [x] 2.3 Implement all six interactive modes, mapping original selected geometry on each preview, preserving flags and unrelated geometry, and handling invalid maps, live shapes and history.
- [x] 2.4 Implement per-document repeat state and component/anchor Again with shared selection bounds, invalidation and history.
- [x] 2.5 Wire command enablement and all eight catalogue entries, update the catalogue golden intentionally, and cover routing.

## 3. Integration

- [x] 3.1 Review `ROADMAP.md` and `PARITY.md`, update coverage and issue links consistently, including the delivered #530 commands and remaining #531 and #532 work.
- [ ] 3.2 After implementation is complete, run the permitted build and relevant Qt and catalogue tests, then project gates and strict OpenSpec validation; record results and resolve failures.
- [x] 3.3 Archive the change after tracker review.
- [ ] 3.4 Include any manual plan rows in the PR checklist.
