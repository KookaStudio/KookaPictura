# Design

## Context

Freeform Pen currently collects a drag and commits a fitted path on release. Magnetic Lasso has a lazy image edge field and a live-wire trace, but its gesture state and selection commit are specific to selection. Pen output already routes paths to Shape, Path, and Pixels.

## Goals / Non-Goals

**Goals:** Preserve a transient trace until completion, reuse image edge detection and existing path output, and keep cancellation and history coherent.

**Non-Goals:** Claiming algorithmic parity with Adobe or changing path transforms or shape-component boolean operations.

## Decisions

Use the existing magnetic edge field and wire tracing for consistency with Magnetic Lasso. Keep Magnetic Pen session state in the Pen handler: it needs open-path completion and path output rather than selection closure. Add a path conversion that preserves manual and automatic fastening points while fitting each intervening wire separately, committed as one logical edit. The existing freeform bridge simplifies the whole trace and cannot guarantee retained fastening anchors. Store Magnetic Pen controls in the Pen options rather than coupling them to Lasso settings. Alt gestures bypass the wire. Carry tablet-device availability alongside pressure through the tool services: mouse pressure is currently reported as 1.0, indistinguishable from full tablet pressure. Forward pressure to the trace only for tablet input. The edge-field session must be released before path editing takes its slot. Permit Magnetic Pen on 32-bit documents without relaxing the Magnetic Lasso restriction. Shape and Pixels need an atomic path-and-output operation: the current separate path commit and output commit are two history entries.

## Risks / Trade-offs

- The edge cost and fit are approximations → exercise a synthetic high-contrast edge without claiming Adobe equivalence.
- Session teardown on tool/document switches can leak the edge field or commit stale data → explicitly release it on every finish or cancel.
- Existing output modes have different history behavior → cover each mode and undo.
