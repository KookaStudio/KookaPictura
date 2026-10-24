# Spec Delta

## ADDED Requirements

### Requirement: Auto Erase is Pencil-only

The Pencil's Auto Erase option SHALL affect Pencil strokes only. The Brush,
Eraser, stamps, and retouch tools SHALL ignore the Auto Erase state even when
the Pencil left it enabled. A Brush stroke that starts over the foreground
colour SHALL paint the foreground colour and record one "Brush" history state.

#### Scenario: Auto Erase does not leak into the Brush [lbt_auto_erase_pencil_only]

- **WHEN** Auto Erase is enabled on the Pencil and the Brush then paints over a
  pixel matching the foreground colour
- **THEN** the Brush paints the foreground colour and records exactly one
  "Brush" history state
