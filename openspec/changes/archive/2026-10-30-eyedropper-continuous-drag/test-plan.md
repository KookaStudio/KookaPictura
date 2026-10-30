# Test Plan

One row per scenario in the change's spec deltas. The new GUI checks live in a
new `tst_eyedropper` suite (a full `PicturaMainWindow` plus synthetic canvas
mouse events); one drag case extends `tst_image_adjustments`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ct_eyedropper_drag` | Open a banded image, activate the Eyedropper, press over the red band, then send left-button moves over the green and blue bands, release, and send a button-up move back over red | The foreground colour is red, then green, then blue after the moves; the button-up move leaves it blue | `tst_eyedropper::eyedropperDragSamplesContinuously` |
| `cpe_arm` | With a document open, click the Color panel foreground swatch to open the picker, then read the tool controller's sampler state and the canvas cursor | The canvas sampler is armed and the canvas cursor is not the arrow | `tst_eyedropper::pickerSwatchEyedropperSamplesTheCanvas` |
| `cpe_click` | In the picker opened from the swatch, send a left-button press over the red band | The picker's colour is red and the history count is unchanged | `tst_eyedropper::pickerSwatchEyedropperSamplesTheCanvas` |
| `cpe_drag` | From the press over red, send left-button moves over the green and blue bands | The picker's colour is green then blue after the moves | `tst_eyedropper::pickerSwatchEyedropperSamplesTheCanvas` |
| `cpe_release` | Close the picker (accept) and read the sampler state | The canvas sampler is disarmed | `tst_eyedropper::pickerSwatchEyedropperSamplesTheCanvas` |

## Manual checks

- Reproduce #1040 on a real display: select the Eyedropper, press on the image,
  and drag across a gradient. The foreground swatch and the Info panel readout
  follow the pointer smoothly for the whole drag. Visual smoothness on a real
  display cannot be asserted offscreen; the automated drag checks cover the
  per-move sampling and the O(1) composite read.
- Open the picker from the toolbox foreground swatch and drag over the image;
  the pointer is the eyedropper and the picker's field and readouts follow it.
