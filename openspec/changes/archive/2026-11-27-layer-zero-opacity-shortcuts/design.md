# Design

## Context

The Layers panel already resolves the selection and exposes undoable opacity and fill transactions. The tree and canvas own distinct Qt key paths.

## Decisions

- Route numeric keys through the existing Layers tree and active Move canvas filters. Keep Qt command shortcuts, text editing, and painting canvas input in their normal paths.
- Use the Layers panel's pending shortcut session and a precise single-shot timer. The session captures the focus owner, document generation, selected layer paths, Shift state, and first digit. A second matching digit resolves immediately within 500 ms. At timeout, a still-valid single digit resolves to its tens percentage, with 0 meaning 100%. Cancel on context changes and recheck the stored target before mutating.
- Recognize direct digit keys and mapped native Shift+digit candidates using the existing Qt key mapper. A translated punctuation character alone cannot identify a digit across layouts.
- A compatible Qt shortcut override can precede either a key press or an intercepted shortcut. Leave it unclaimed and clear the pending digit if no corresponding press follows. While that decision is outstanding, timeout resolution SHALL wait for it; if the shortcut intercepts the key, discard the pending digit even when its action enters a nested event loop across the deadline. Skip repeat key events without restarting the timer.
- Resolve via the existing batch opacity and fill setters, using the same conversion to the stored byte as the panel fields. The first press creates no preview or history entry, so the two-digit result is atomic.

## Risks / Trade-offs

Single-digit feedback follows the existing 500 ms pairing window. Native keyboard smoke on the target layouts is needed for translated key events.
