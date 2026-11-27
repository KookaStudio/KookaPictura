# Test plan

| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| eye_background | Alt-drag the Eyedropper across distinct colours, changing modifier during drag. | Background follows; foreground and history stay unchanged. | `tst_eyedropper::altSamplesBackground` |
| eye_foreground | Drag Eyedropper without modifiers. | Foreground follows and background stays unchanged. | `tst_eyedropper::plainSamplesForeground` |
| eye_temporary | Alt-drag with each eligible tool and non-default sample size and scope. | Foreground follows scoped samples; no stroke/history or tool/options change. | `tst_eyedropper::temporarySampling` |
| eye_restore | Release Alt during a held temporary sampling drag, continue moving, release the mouse, then paint. | Sampling and temporary cursor persist through mouse release without a stroke; normal cursor and tool gesture return. | `tst_eyedropper::temporaryRestores` |
| eye_cancel | Cancel temporary sampling, then paint. | Cursor and normal tool gesture return. | `tst_eyedropper::temporaryCancel` |
| eye_quick_mask | Alt-drag with each eligible tool in Quick Mask over distinct document colours, using non-default sampling settings. | Foreground follows scoped document samples; mask coverage, history and selected tool remain unchanged. | `tst_eyedropper::temporaryQuickMask` |
| eye_reserved | Exercise Alt on Clone Stamp, Healing Brush, Eraser and Mixer Brush. | Their established source/history/reservoir actions remain. | `tst_eyedropper::reservedAltGestures` |
| eye_stroke | Begin a stroke then press Alt before release. | Stroke continues; sampling does not start. | `tst_eyedropper::altDuringStroke` |
| eye_sampler | Shift-click and drag with Eyedropper. | One sampler at press; colours unchanged. | `tst_eyedropper::shiftPlacesSampler` |
| eye_sampler_limit | Shift-click after placing four samplers. | Fifth refused with a refusal signal/message; history and count unchanged. | `tst_eyedropper::shiftSamplerLimit` |
| eye_sampler_priority | Shift+Alt-click the Eyedropper. | Sampler placed; colours unchanged. | `tst_eyedropper::shiftAltPlacesSampler` |
