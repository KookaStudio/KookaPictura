# Test plan

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| magpen_options | Enable Magnetic, edit width and press brackets at boundaries. | Controls have the specified defaults and width is clamped and stepped. | tst_pen_tools::magneticOptions |
| magpen_pressure | Trace with tablet pressure enabled at different pressures. | Higher pressure narrows the detection width; mouse uses configured width. | tst_pen_tools::magneticPressure, tst_pointer_pressure::tabletAndMouse |
| magpen_trace | Trace a high-contrast edge, fasten, delete the latest fastening, and complete the path. | Edge is followed, last fastening removed, and remaining manual fastening is an editable anchor. | tst_pen_tools::magneticTrace |
| magpen_frequency | Trace the same edge at zero and high Frequency and complete each path. | High Frequency adds automatic anchors; zero adds none. | tst_pen_tools::magneticFrequency |
| magpen_depth | Trace on a 32-bit document with Magnetic Pen and attempt Magnetic Lasso. | Pen traces; Lasso refuses. | tst_pen_tools::magneticDepth |
| magpen_switch | Start a trace, switch tools or documents, and start another trace. | Old preview and field are released. | tst_pen_tools::magneticSwitch |
| magpen_cancel | Begin a trace and cancel using Escape and Undo. | Preview clears, document path and history unchanged. | tst_pen_tools::magneticCancel |
| magpen_open | Trace in Path mode and press Enter. | Saved path remains open and follows trace. | tst_pen_tools::magneticOpen |
| magpen_close | Close comparable traces with double-click and Alt-double-click. | Paths close with magnetic and straight closing segments respectively. | tst_pen_tools::magneticClose |
| magpen_alt | Alt-drag and Alt-click during a trace. | Freehand and straight segments are retained. | tst_pen_tools::magneticAlt |
| magpen_modes | Complete traces in each output mode and undo. | Correct output is created as one logical edit; undo restores the prior Work Path and output. | tst_pen_tools::magneticModes |
| magpen_quick_mask | Trace in Pixels mode with Quick Mask active, then undo. | Mask is painted and one undo restores mask and prior path. | tst_pen_tools::magneticQuickMask |
| magpen_active_paths | Trace with a saved path and a vector mask active, in Shape and Pixels modes, then undo. | The active path gains the trace; no shape or pixel output is created, and undo restores the path. | tst_pen_tools::magneticActivePaths |
