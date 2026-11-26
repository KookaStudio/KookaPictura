# Proposal

## Why

The destructive Levels dialog supports numerical edits, but users cannot pick tonal points from the image or see which colors clip while setting the inputs. Issue #560 adds both interactions to the existing Levels workflow.

## What Changes

- Add black, gray, and white point pickers with configurable neutral targets and a sample-size menu.
- Show a temporary shadow or highlight clipping map when Alt is held during a corresponding input thumb drag.
- Use the existing Levels preview, undo, and commit paths.

## Capabilities

### New Capabilities

- `ui/levels-canvas-controls` defines point sampling and clipping feedback for Image > Adjustments > Levels.

### Modified Capabilities

None.

## Impact

The destructive Levels dialog uses canvas sampling, a bounded composite-region read, the document-region display API, and a visual preview overlay. The Levels block, engine, and Properties controls retain their current interfaces.
