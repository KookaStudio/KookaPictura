# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cpn_new_text` | Select a type layer, click elsewhere with the Type tool, toggle Faux Bold and Underline in the Character panel, commit | Each toggle changes the preview at once; the new layer has both; the old layer has neither | `tst_character_paragraph_panels::characterPanelEditsNewText` |
| Live update on screen | Drag out new type and toggle Bold, Underline, and size in the Character panel | The placeholder on the canvas changes with each click, before committing | manual: confirms the reported screen behaviour on a real desktop |
