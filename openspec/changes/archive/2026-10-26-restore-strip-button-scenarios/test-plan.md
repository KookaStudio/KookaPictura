# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| The implemented strip buttons carry icons | Read the seven action-strip buttons' icons | Each matches its `layers.*` asset | `tst_linked_layers::stripButtonsCarryIcons` |
| lpl_strip_link | Select two layers, click the Link strip button, then click again | Both rows report linked after the first click, unlinked after the second | `tst_linked_layers::stripLinkButtonToggles` |
| lpc_strip_no_document | Clear the panel's view and read the strip buttons | All seven are disabled | `tst_layers_panel_fixes::bottomStripDisabledWithoutDocument` |
| lmk_strip | Click the mask button with no selection | A reveal-all mask is added in one undoable step | `tst_layers_mask_commands::maskButtonFollowsTheMasks` |
| lmk_strip_sel | Select a region and click the mask button | A reveal-selection mask is added to the active layer in one undoable step | `tst_layers_mask_commands::maskButtonRespectsSelection` |
| lmk_strip_alt | Alt-click the mask button | A hide-all mask is added in one undoable step | `tst_layers_mask_commands::maskButtonAltHidesAll` |
| The fx button opens Blending Options | Click the fx button on a styleable layer | The Layer Style dialog opens | `tst_layers_panel_fixes::fxButtonOpensBlendingOptions` |
| Alt-clicking the fx button toggles all effects | Alt-click the fx button twice | Effects are hidden then shown, one undoable step each | `tst_layers_panel_fixes::fxButtonAltTogglesAll` |
