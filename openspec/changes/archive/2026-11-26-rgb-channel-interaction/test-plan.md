| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| Composite selection | Click RGB | RGB eyes on and component rows highlighted | tst_channels_panel::rgbSelectionAndVisibility |
| Component selection | Click Red with all visible | Red selected and soloed | tst_channels_panel::rgbSelectionAndVisibility |
| Existing partial visibility | Hide Blue and click Red | Red selected, Red and Green remain visible | tst_channels_panel::rgbSelectionAndVisibility |
| Hidden component selection | Hide Blue and click Blue | Blue selected and soloed | tst_channels_panel::rgbSelectionAndVisibility |
| Eye click | Click an eye with an active row | Visibility changes, active row stays | tst_channels_panel::rgbSelectionAndVisibility |
| Partial RGB | Select a component or hide an eye | Specified adjustments gated | tst_channels_panel::rgbAdjustmentAvailability |
| Composite RGB | Click composite | Normal eligibility returns | tst_channels_panel::rgbAdjustmentAvailability |
| RGB rows displayed | Inspect row and thumbnail geometry | Layers-style eye column, 2px padding | tst_channels_panel::rgbSelectionAndVisibility |
