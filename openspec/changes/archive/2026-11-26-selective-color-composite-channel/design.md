# Design

## Channel state

Use the Channels panel as the source of truth for the selected row and colour/alpha visibility. The composite is available when the active document uses a mode with a composite row, the first row is selected, every working colour channel is visible, and there is no displayed alpha, soloed layer mask, or active Quick Mask. Multichannel has no composite row. Other mode availability keeps its existing prerequisites.

Listen for changes in the predicate after selection, visibility, or a rebuild, and recompute and notify when mask solo changes outside Channels, including Alt-clicking a layer mask or using Ctrl+\. A refresh must detect a solo change even when the active mask name is unchanged. The host forwards the availability to the Adjustments tile, Layers footer, and command registry. Registry handlers also consult the current predicate because direct dispatch does not use QAction enablement. The Adjustments flyout already uses the Layer command. Recheck availability when the panels bind to another document, using the Channels panel's existing reset on document switch.

The gate belongs in the frame and panels, where the channel view state lives. Engine and explicit headless operations use document targets. Existing adjustment layers continue to render independently from channel view state.

## Presets

Use the existing shared Save/Load whitelist and Selective Color preset store. Exercise a named preset with a nondefault correction and method through the Properties panel, then load it through the Properties menu and the destructive dialog. Use existing adjustment kinds outside the whitelist as negative controls.

## Trackers

Close the #577 tracker entries once the source and coverage are in place. Preserve any separate limitations recorded for Selective Color and follow the parity tracker's existing status definitions.
