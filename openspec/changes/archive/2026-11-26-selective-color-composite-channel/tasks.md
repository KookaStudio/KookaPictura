## 1. Coverage

- [x] 1.1 Add Qt tests for launcher availability and refusal, component and mask visibility, external mask-solo entry and exit, and rebinding the Channels panel on document switch.
- [x] 1.2 Add Qt coverage for Selective Color preset Save/Load through Properties and selection in the destructive dialog.

## 2. Implementation

- [x] 2.1 Expose the composite channel predicate and change notifications from Channels, including external mask-solo changes, and re-evaluate it when switching documents.
- [x] 2.2 Apply the predicate at the supported launchers and their dispatch paths.
- [x] 2.3 Update ROADMAP.md and PARITY.md for #577.

## 3. Follow-up

- [ ] 3.1 Run the targeted Qt suites, manual Multichannel scenario, and repository gates when authorized.
