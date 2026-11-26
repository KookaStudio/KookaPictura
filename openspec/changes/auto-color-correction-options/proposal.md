## Why

Levels and Curves lack a way to choose the automatic correction and save its settings. The Properties menu advertises Auto Options as unavailable.

## What Changes

- Open Auto Color Correction Options from the Levels or Curves Properties menu or by Alt-clicking its Auto button.
- Configure one correction with separate shadow and highlight clipping, RGB target colors, and optional neutral midtone correction. Apply it as an ordinary editable Levels or Curves adjustment.
- Save defaults in the application preference store. The three Image Auto commands retain their named algorithms and use the saved targets and clipping. Auto Color also uses the saved midtone setting.
- Preserve the factory results of all existing Image Auto commands for fresh and legacy profiles until defaults are explicitly saved; opening or cancelling Options does not create an override.

## Capabilities

### New Capabilities

- `ui/auto-color-correction-options` adds the shared dialog, the Properties Auto workflow, and its application defaults.

### Modified Capabilities

None.

## Impact

Properties, the profile store, and the existing destructive adjustment pipeline share automatic correction parameters. PSD/PSB continues to carry ordinary Levels and Curves parameters. The existing destructive adjustment dialogs' selected-channel Auto buttons and Brightness/Contrast stay under the adjustment-dialog-auto contract.
