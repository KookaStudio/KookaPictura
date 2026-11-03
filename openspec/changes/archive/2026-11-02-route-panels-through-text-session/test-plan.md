# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `tsp_character` | Reopen a layer, change size, anti-aliasing, family and Faux Bold, commit | Each edit previews; history unmoved; all land as one state | `tst_type_session_panels::reopenedCharacterEditsSession` |
| `tsp_color` | Reopen a layer, cancel one colour pick, accept another, commit | Cancel restores the session colour; the accepted colour lands as one state | `tst_type_color_preview::characterPanelColorEditsReopenedSession` |
| `tsp_cancel` | Edit both panels in a reopened session, cancel | Layer unchanged; no state | `tst_type_session_panels::reopenedCharacterCancelKeepsLayer` |
| `tsp_para_new` | Select a layer, open new text, set justification and an indent, commit | Justification previews; new layer has both; selected layer has neither | `tst_type_session_panels::paragraphEditsNewText` |
| `tsp_para_reopen` | Reopen a layer, set indent and justification, commit | Previews; history unmoved; one state carries both | `tst_type_session_panels::paragraphEditsReopenedLayer` |
| `tsp_mask` | Select a layer, edit size and justification in a mask session, commit | Larger selection; selected layer unchanged | `tst_type_session_panels::panelsEditMaskSession` |
| `tsp_menu` | Open a session with no layer, use Justification, Reset Character, Reset Paragraph | Items enabled; session changes; no state until commit | `tst_type_session_panels::menuActionsEditSession` |
| `tsp_bar` | Change size and justification in the bar, then edit both panels, commit | The bar's values survive | `tst_type_session_panels::optionsBarEditSurvivesPanelEdit` |
