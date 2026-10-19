# Test plan — Transform preview appearance and action status

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A drag does not recomposite | Begin a transform, drag, sample the canvas | The preview redraws from the cached base/layer without a full recomposite | `tst_transform_skew::previewKeepsTheLayerColor` |
| Previewing a channel-less target mutates nothing | Begin a session on a channel-less smart object | The document is unchanged until commit | manual: the free-transform self-test covers the channel-less smart-object preview; no Qt Test fixture creates one |
| `ft_preview_appearance` | Begin Skew on a styled / masked / vector / fill / clipping / type layer and sample the canvas | The live preview keeps the rendered appearance | `tst_transform_skew::styleKeepsTheLayerColor`, `tst_transform_skew::clippedFillKeepsTheLayerColor`, `tst_transform_skew::thinLineKeepsTheLayerColor`, `tst_transform_skew::typeKeepsTheLayerColor`, `tst_transform_skew::previewKeepsTheLayerColor`, `pictura_app::a_shape_preview_ignores_empty_channels` |
| Magnification and size shown | Open a document | The status bar shows magnification and size | `tst_smoke::statusBarChromeAndZoom` |
| View options popup | Activate the status-bar options control | The popup selects the readout | `tst_smoke::statusBarChromeAndZoom` |
| `las_status_empty` | Show the status bar with no document | No readouts or tool hints | `tst_smoke::frameOpensOffscreen` |
| `las_status_no_grip` | Show the status bar | No size grip; small left padding | `tst_smoke::statusBarChromeAndZoom` |
| `las_status_action` | Begin Free Transform, switch to Skew, cancel | `Free Transform in progress`, then `Skew in progress`, then hidden | `tst_transform_skew::statusShowsTheSkewAction` |
